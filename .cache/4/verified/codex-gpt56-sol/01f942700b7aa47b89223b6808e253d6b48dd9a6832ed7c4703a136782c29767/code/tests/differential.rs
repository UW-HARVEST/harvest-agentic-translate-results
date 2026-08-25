use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

type PrintIntPtrLine = unsafe extern "C" fn(*const c_int);
type VoidFunction = unsafe extern "C" fn();
type MainFunction = unsafe extern "C" fn() -> c_int;

struct Api {
    _library: Library,
    print_int_ptr_line: PrintIntPtrLine,
    bad: VoidFunction,
    good: VoidFunction,
    main: MainFunction,
}

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    exit_code: Option<c_int>,
    signal: Option<c_int>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[repr(C)]
struct CFile {
    _private: [u8; 0],
}

unsafe extern "C" {
    static mut stdin: *mut CFile;

    fn clearerr(stream: *mut CFile);
    fn close(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut CFile) -> c_int;
    fn fork() -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
    fn _exit(status: c_int) -> !;
}

struct Fixture {
    c: Api,
    rust: Api,
}

impl Fixture {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libdriver_c.so");
        let test_exe = std::env::current_exe().expect("current test executable");
        let deps_dir = test_exe.parent().expect("Cargo deps directory");
        let rust_path = deps_dir.join("libdriver.so");

        assert!(
            c_path.is_file(),
            "missing C shared object: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared object: {}",
            rust_path.display()
        );

        unsafe {
            Self {
                c: load_api(&c_path),
                rust: load_api(&rust_path),
            }
        }
    }

    fn compare_print(&self, label: &str, value: *const c_int) -> Outcome {
        let c_function = self.c.print_int_ptr_line;
        let rust_function = self.rust.print_int_ptr_line;
        let c = run_child(&[], || {
            unsafe { c_function(value) };
            0
        });
        let rust = run_child(&[], || {
            unsafe { rust_function(value) };
            0
        });
        assert_same(label, &c, &rust);
        c
    }

    fn compare_void(&self, label: &str, c_function: VoidFunction, rust_function: VoidFunction) {
        let c = run_child(&[], || {
            unsafe { c_function() };
            0
        });
        let rust = run_child(&[], || {
            unsafe { rust_function() };
            0
        });
        assert_same(label, &c, &rust);
    }

    fn compare_main(&self, label: &str, input: &[u8]) -> Outcome {
        let c_function = self.c.main;
        let rust_function = self.rust.main;
        let c = run_child(input, || unsafe { c_function() });
        let rust = run_child(input, || unsafe { rust_function() });
        assert_same(label, &c, &rust);
        c
    }
}

unsafe fn load_api(path: &Path) -> Api {
    let library = Library::from(
        UnixLibrary::open(Some(path), RTLD_NOW | RTLD_LOCAL).unwrap_or_else(|error| {
            panic!("failed to load {}: {error}", path.display());
        }),
    );
    let print_int_ptr_line = *library
        .get::<PrintIntPtrLine>(b"printIntPtrLine\0")
        .expect("printIntPtrLine export");
    let bad = *library.get::<VoidFunction>(b"bad\0").expect("bad export");
    let good = *library.get::<VoidFunction>(b"good\0").expect("good export");
    let main = *library.get::<MainFunction>(b"main\0").expect("main export");
    Api {
        _library: library,
        print_int_ptr_line,
        bad,
        good,
        main,
    }
}

fn run_child<F>(input: &[u8], call: F) -> Outcome
where
    F: FnOnce() -> c_int,
{
    unsafe {
        let input_pipe = make_pipe();
        let stdout_pipe = make_pipe();
        let stderr_pipe = make_pipe();

        write_all(input_pipe[1], input);
        assert_eq!(close(input_pipe[1]), 0);
        fflush(ptr::null_mut());

        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            close(stdout_pipe[0]);
            close(stderr_pipe[0]);
            if dup2(input_pipe[0], 0) < 0
                || dup2(stdout_pipe[1], 1) < 0
                || dup2(stderr_pipe[1], 2) < 0
            {
                _exit(250);
            }
            close(input_pipe[0]);
            close(stdout_pipe[1]);
            close(stderr_pipe[1]);
            clearerr(stdin);

            let code = call();
            fflush(ptr::null_mut());
            _exit(code & 0xff);
        }

        assert_eq!(close(input_pipe[0]), 0);
        assert_eq!(close(stdout_pipe[1]), 0);
        assert_eq!(close(stderr_pipe[1]), 0);

        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);
        let stdout = read_all(stdout_pipe[0]);
        let stderr = read_all(stderr_pipe[0]);
        assert_eq!(close(stdout_pipe[0]), 0);
        assert_eq!(close(stderr_pipe[0]), 0);

        let terminating_signal = status & 0x7f;
        if terminating_signal == 0 {
            Outcome {
                exit_code: Some((status >> 8) & 0xff),
                signal: None,
                stdout,
                stderr,
            }
        } else {
            Outcome {
                exit_code: None,
                signal: Some(terminating_signal),
                stdout,
                stderr,
            }
        }
    }
}

unsafe fn make_pipe() -> [c_int; 2] {
    let mut fds = [-1, -1];
    assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe failed");
    fds
}

unsafe fn write_all(fd: c_int, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let written = write(fd, bytes.as_ptr().cast(), bytes.len());
        assert!(written > 0, "write failed");
        bytes = &bytes[written as usize..];
    }
}

unsafe fn read_all(fd: c_int) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = read(fd, buffer.as_mut_ptr().cast(), buffer.len());
        assert!(count >= 0, "read failed");
        if count == 0 {
            return output;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
}

fn assert_same(label: &str, c: &Outcome, rust: &Outcome) {
    assert_eq!(c, rust, "{label}: C and Rust outcomes differ");
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }
}

#[test]
fn valid_print_int_ptr_line_rows_match_randomized() {
    let fixture = Fixture::load();
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);

    for index in 0..64 {
        let value = (rng.next_u32() | 0x8000_0000) as c_int;
        let outcome = fixture.compare_print(&format!("negative value #{index}: {value}"), &value);
        assert_eq!(outcome.exit_code, Some(0));
    }
    let minimum = c_int::MIN;
    fixture.compare_print("INT_MIN", &minimum);

    for index in 0..32 {
        let zero = 0;
        let outcome = fixture.compare_print(&format!("zero value #{index}"), &zero);
        assert_eq!(outcome.exit_code, Some(0));
    }

    for index in 0..64 {
        let value = ((rng.next_u32() & 0x7fff_ffff).max(1)) as c_int;
        let outcome = fixture.compare_print(&format!("positive value #{index}: {value}"), &value);
        assert_eq!(outcome.exit_code, Some(0));
    }
    let maximum = c_int::MAX;
    fixture.compare_print("INT_MAX", &maximum);
}

#[test]
fn valid_good_row_matches_repeated() {
    let fixture = Fixture::load();
    for index in 0..32 {
        fixture.compare_void(
            &format!("good call #{index}"),
            fixture.c.good,
            fixture.rust.good,
        );
    }
}

#[test]
fn valid_main_nonzero_rows_match_randomized() {
    let fixture = Fixture::load();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    for index in 0..64 {
        let value = (rng.next_u32() | 0x8000_0000) as c_int;
        let input = format!(" \n\t{value} trailing").into_bytes();
        let outcome = fixture.compare_main(&format!("negative main value #{index}"), &input);
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(outcome.stdout, b"5\n");
    }

    for index in 0..64 {
        let value = ((rng.next_u32() & 0x7fff_ffff).max(1)) as c_int;
        let input = format!("+{value}\nignored").into_bytes();
        let outcome = fixture.compare_main(&format!("positive main value #{index}"), &input);
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(outcome.stdout, b"5\n");
    }
}

#[test]
fn valid_main_zero_row_matches_input_shapes() {
    let fixture = Fixture::load();
    for index in 0..32 {
        let input = match index % 4 {
            0 => "0".to_owned(),
            1 => format!("{}+0 trailing", " ".repeat(index)),
            2 => format!("\n\t-{}0", "0".repeat(index)),
            _ => format!("{}0\n9", "0".repeat(index)),
        };
        let outcome = fixture.compare_main(&format!("zero main shape #{index}"), input.as_bytes());
        assert_eq!(outcome.signal, Some(11));
        assert!(outcome.stdout.is_empty());
        assert!(outcome.stderr.is_empty());
    }
}

#[test]
fn error_null_pointer_row_matches_exact_signal() {
    let fixture = Fixture::load();
    for index in 0..16 {
        let outcome = fixture.compare_print(&format!("null pointer #{index}"), ptr::null());
        assert_eq!(outcome.signal, Some(11));
        assert!(outcome.stdout.is_empty());
        assert!(outcome.stderr.is_empty());
    }
}

#[test]
fn error_direct_bad_row_matches_repeated() {
    let fixture = Fixture::load();
    for index in 0..32 {
        fixture.compare_void(
            &format!("bad call #{index}"),
            fixture.c.bad,
            fixture.rust.bad,
        );
    }
}

#[test]
fn error_main_conversion_failure_row_matches_randomized() {
    let fixture = Fixture::load();
    let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
    const INVALID_STARTS: &[u8] = b"abcdefghijklmnopqrstuvwxyz!@#_";

    for index in 0..32 {
        let start = INVALID_STARTS[(rng.next_u32() as usize) % INVALID_STARTS.len()];
        let input = format!(
            "{}{}{}",
            " \t".repeat((rng.next_u32() % 4) as usize),
            start as char,
            rng.next_u32()
        );
        let outcome = fixture.compare_main(
            &format!("failed scanf conversion #{index}"),
            input.as_bytes(),
        );
        assert_eq!(outcome.signal, Some(11));
        assert!(outcome.stdout.is_empty());
        assert!(outcome.stderr.is_empty());
    }
}

#[test]
fn error_main_eof_row_matches_whitespace_shapes() {
    let fixture = Fixture::load();
    for index in 0..16 {
        let input = " \t\n".repeat(index);
        let outcome = fixture.compare_main(&format!("scanf EOF #{index}"), input.as_bytes());
        assert_eq!(outcome.signal, Some(11));
        assert!(outcome.stdout.is_empty());
        assert!(outcome.stderr.is_empty());
    }
}

#[test]
fn generic_oversized_decimal_tokens_match() {
    let fixture = Fixture::load();
    for length in [11, 32, 256, 4096] {
        let positive = "9".repeat(length);
        fixture.compare_main(
            &format!("oversized positive token length {length}"),
            positive.as_bytes(),
        );

        let negative = format!("-{}", "9".repeat(length));
        fixture.compare_main(
            &format!("oversized negative token length {length}"),
            negative.as_bytes(),
        );
    }
}
