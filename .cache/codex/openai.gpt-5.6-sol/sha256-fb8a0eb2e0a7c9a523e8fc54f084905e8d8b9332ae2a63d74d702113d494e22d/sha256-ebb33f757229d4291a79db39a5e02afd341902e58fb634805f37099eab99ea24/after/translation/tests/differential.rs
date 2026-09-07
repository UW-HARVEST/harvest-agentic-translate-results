use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

type DriverFn = unsafe extern "C" fn(c_int);
type PrintLineFn = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

struct Api {
    _library: Library,
    driver: DriverFn,
    print_line: PrintLineFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        assert!(
            path.is_file(),
            "shared library does not exist: {}",
            path.display()
        );
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = *unsafe { library.get::<DriverFn>(b"driver\0") }
            .unwrap_or_else(|error| panic!("missing driver in {}: {error}", path.display()));
        let print_line = *unsafe { library.get::<PrintLineFn>(b"printLine\0") }
            .unwrap_or_else(|error| panic!("missing printLine in {}: {error}", path.display()));
        Self {
            _library: library,
            driver,
            print_line,
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_dir.join("../c_src/build/libdriver.so");
        let rust_path = crate_dir.join("target/release/libdriver.so");
        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RunResult {
    stdout: Vec<u8>,
    wait_status: c_int,
}

#[derive(Clone, Copy)]
enum Invocation {
    Driver(c_int),
    PrintLine(*const c_char),
}

fn run_in_child(api: &Api, invocation: Invocation) -> RunResult {
    unsafe {
        // Do not let buffered output from the test process leak into the child.
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut pipe_fds = [0; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");

        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            close(pipe_fds[0]);
            if dup2(pipe_fds[1], 1) < 0 {
                _exit(126);
            }
            close(pipe_fds[1]);

            match invocation {
                Invocation::Driver(data) => (api.driver)(data),
                Invocation::PrintLine(line) => (api.print_line)(line),
            }

            fflush(ptr::null_mut());
            _exit(0);
        }

        close(pipe_fds[1]);
        let mut stdout = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = read(
                pipe_fds[0],
                chunk.as_mut_ptr().cast::<c_void>(),
                chunk.len(),
            );
            assert!(count >= 0, "read failed");
            if count == 0 {
                break;
            }
            stdout.extend_from_slice(&chunk[..count as usize]);
        }
        close(pipe_fds[0]);

        let mut wait_status = 0;
        assert_eq!(waitpid(pid, &mut wait_status, 0), pid, "waitpid failed");
        RunResult {
            stdout,
            wait_status,
        }
    }
}

fn assert_same(libraries: &Libraries, invocation: Invocation, expected_stdout: &[u8]) {
    let c_result = run_in_child(&libraries.c, invocation);
    let rust_result = run_in_child(&libraries.rust, invocation);
    assert_eq!(rust_result, c_result, "Rust and C results differ");
    assert_eq!(c_result.wait_status, 0, "C call did not exit successfully");
    assert_eq!(
        c_result.stdout, expected_stdout,
        "unexpected C ground-truth output"
    );
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, start: u32, end_inclusive: u32) -> u32 {
        start + (self.next_u64() % u64::from(end_inclusive - start + 1)) as u32
    }
}

// CFG-001
#[test]
fn print_line_empty_string_matches() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0x30c5_53d3_d1ff_9e21);
    for _ in 0..64 {
        // Random trailing storage verifies that the first NUL ends the C string.
        let mut bytes = vec![0_u8];
        bytes.extend((0..rng.range(0, 128)).map(|_| rng.range(1, 255) as u8));
        assert_same(
            &libraries,
            Invocation::PrintLine(bytes.as_ptr().cast::<c_char>()),
            b"\n",
        );
    }
}

// CFG-002
#[test]
fn print_line_nonempty_random_strings_match() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0x7902_5fb8_7a19_570d);
    for _ in 0..128 {
        let visible_len = rng.range(1, 512) as usize;
        let mut bytes: Vec<u8> = (0..visible_len).map(|_| rng.range(1, 255) as u8).collect();
        bytes.push(0);
        bytes.extend((0..rng.range(0, 64)).map(|_| rng.range(1, 255) as u8));

        let mut expected = bytes[..visible_len].to_vec();
        expected.push(b'\n');
        assert_same(
            &libraries,
            Invocation::PrintLine(bytes.as_ptr().cast::<c_char>()),
            &expected,
        );
    }
}

// CFG-003
#[test]
fn driver_zero_matches() {
    let libraries = Libraries::load();
    for _ in 0..64 {
        assert_same(&libraries, Invocation::Driver(0), b"\n");
    }
}

// CFG-004
#[test]
fn driver_interior_copy_lengths_match() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0xc2b2_ae3d_27d4_eb4f);
    for _ in 0..128 {
        let data = rng.range(1, 98) as c_int;
        let mut expected = vec![b'A'; data as usize];
        expected.push(b'\n');
        assert_same(&libraries, Invocation::Driver(data), &expected);
    }
}

// CFG-005
#[test]
fn driver_maximum_copy_length_matches() {
    let libraries = Libraries::load();
    let mut expected = vec![b'A'; 99];
    expected.push(b'\n');
    for _ in 0..64 {
        assert_same(&libraries, Invocation::Driver(99), &expected);
    }
}

// CFG-006
#[test]
fn driver_copy_cutoff_matches() {
    let libraries = Libraries::load();
    for _ in 0..64 {
        assert_same(&libraries, Invocation::Driver(100), b"\n");
    }
}

// CFG-007
#[test]
fn driver_values_above_cutoff_match() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0x9e37_79b9_7f4a_7c15);
    assert_same(&libraries, Invocation::Driver(c_int::MAX), b"\n");
    for _ in 0..128 {
        let data = rng.range(101, c_int::MAX as u32) as c_int;
        assert_same(&libraries, Invocation::Driver(data), b"\n");
    }
}

// ERR-001
#[test]
fn print_line_null_rejection_matches() {
    let libraries = Libraries::load();
    for _ in 0..64 {
        assert_same(&libraries, Invocation::PrintLine(ptr::null()), b"");
    }
}

// ERR-002
#[test]
fn driver_range_rejection_matches() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0xd6e8_feb8_6659_fd93);
    assert_same(&libraries, Invocation::Driver(100), b"\n");
    assert_same(&libraries, Invocation::Driver(c_int::MAX), b"\n");
    for _ in 0..128 {
        let data = rng.range(100, c_int::MAX as u32) as c_int;
        assert_same(&libraries, Invocation::Driver(data), b"\n");
    }
}
