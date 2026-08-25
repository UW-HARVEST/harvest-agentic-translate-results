use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

type MainFn = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn kill(pid: c_int, signal: c_int) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
    fn _exit(status: c_int) -> !;
}

const STDIN_FILENO: c_int = 0;
const STDOUT_FILENO: c_int = 1;
const SIGKILL: c_int = 9;
const WNOHANG: c_int = 1;
const OUTPUT_LIMIT: usize = 4096;
const PREFIX_LENGTH: usize = 2048;

struct Api {
    _library: Library,
    main: MainFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let main = *unsafe { library.get::<MainFn>(b"main\0") }.unwrap_or_else(|error| {
            panic!("failed to resolve main in {}: {error}", path.display())
        });
        Self {
            _library: library,
            main,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Termination {
    Returned(c_int),
    OutputLimit,
    TimedOut,
    Signal(c_int),
    ExitWithoutReturn(c_int),
}

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    termination: Termination,
    stdout: Vec<u8>,
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deps_dir = std::env::current_exe()
        .expect("current test executable")
        .parent()
        .expect("test deps directory")
        .to_path_buf();
    (
        manifest.join("c_src/build/libdriver_c.so"),
        deps_dir.join("libdriver.so"),
    )
}

fn make_pipe() -> [c_int; 2] {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    fds
}

fn run(api: &Api, input: &[u8], stop_at_output_limit: bool) -> Outcome {
    let stdin_pipe = make_pipe();
    let stdout_pipe = make_pipe();
    let result_pipe = make_pipe();

    unsafe {
        fflush(std::ptr::null_mut());
    }
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        unsafe {
            close(stdin_pipe[1]);
            close(stdout_pipe[0]);
            close(result_pipe[0]);
            if dup2(stdin_pipe[0], STDIN_FILENO) < 0 || dup2(stdout_pipe[1], STDOUT_FILENO) < 0 {
                _exit(120);
            }
            close(stdin_pipe[0]);
            close(stdout_pipe[1]);

            let result = (api.main)();
            fflush(std::ptr::null_mut());
            let bytes = result.to_ne_bytes();
            let mut written = 0;
            while written < bytes.len() {
                let count = write(
                    result_pipe[1],
                    bytes[written..].as_ptr().cast(),
                    bytes.len() - written,
                );
                if count <= 0 {
                    _exit(121);
                }
                written += count as usize;
            }
            close(result_pipe[1]);
            _exit(0);
        }
    }

    unsafe {
        close(stdin_pipe[0]);
        close(stdout_pipe[1]);
        close(result_pipe[1]);
    }

    let mut input_writer = unsafe { File::from_raw_fd(stdin_pipe[1]) };
    input_writer.write_all(input).expect("write child stdin");
    drop(input_writer);

    let reached_limit = Arc::new(AtomicBool::new(false));
    let reader_limit = Arc::clone(&reached_limit);
    let output_reader = thread::spawn(move || {
        let mut reader = unsafe { File::from_raw_fd(stdout_pipe[0]) };
        let mut captured = Vec::new();
        let mut total = 0_usize;
        let mut buffer = [0_u8; 1024];
        loop {
            let count = reader.read(&mut buffer).expect("read child stdout");
            if count == 0 {
                break;
            }
            total += count;
            if captured.len() < OUTPUT_LIMIT {
                let keep = count.min(OUTPUT_LIMIT - captured.len());
                captured.extend_from_slice(&buffer[..keep]);
            }
            if total >= OUTPUT_LIMIT {
                reader_limit.store(true, Ordering::Release);
            }
        }
        captured
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut status = 0;
    let forced = loop {
        let waited = unsafe { waitpid(pid, &mut status, WNOHANG) };
        assert!(waited >= 0, "waitpid failed");
        if waited == pid {
            break None;
        }
        if stop_at_output_limit && reached_limit.load(Ordering::Acquire) {
            assert_eq!(unsafe { kill(pid, SIGKILL) }, 0, "kill failed");
            assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
            break Some(Termination::OutputLimit);
        }
        if Instant::now() >= deadline {
            assert_eq!(unsafe { kill(pid, SIGKILL) }, 0, "kill failed");
            assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
            break Some(Termination::TimedOut);
        }
        thread::sleep(Duration::from_millis(1));
    };

    let stdout = output_reader.join().expect("stdout reader thread");
    let mut result_bytes = Vec::new();
    unsafe { File::from_raw_fd(result_pipe[0]) }
        .read_to_end(&mut result_bytes)
        .expect("read child result");

    let termination = if let Some(forced) = forced {
        forced
    } else if result_bytes.len() == std::mem::size_of::<c_int>() {
        Termination::Returned(c_int::from_ne_bytes(
            result_bytes.try_into().expect("c_int result bytes"),
        ))
    } else if status & 0x7f != 0 {
        Termination::Signal(status & 0x7f)
    } else {
        Termination::ExitWithoutReturn((status >> 8) & 0xff)
    };

    Outcome {
        termination,
        stdout,
    }
}

fn assert_finite(c: &Api, rust: &Api, input: &[u8], case: &str) {
    let c_outcome = run(c, input, false);
    let rust_outcome = run(rust, input, false);
    assert_eq!(
        c_outcome.termination,
        Termination::Returned(0),
        "{case}: C did not return zero for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        rust_outcome,
        c_outcome,
        "{case}: differential mismatch for input {:?}",
        String::from_utf8_lossy(input)
    );
}

fn assert_nonterminating(c: &Api, rust: &Api, input: &[u8], case: &str) {
    let c_outcome = run(c, input, true);
    let rust_outcome = run(rust, input, true);
    assert_eq!(c_outcome.termination, Termination::OutputLimit, "{case}: C");
    assert_eq!(
        rust_outcome.termination,
        Termination::OutputLimit,
        "{case}: Rust"
    );
    assert!(c_outcome.stdout.len() >= PREFIX_LENGTH, "{case}: C prefix");
    assert!(
        rust_outcome.stdout.len() >= PREFIX_LENGTH,
        "{case}: Rust prefix"
    );
    assert_eq!(
        &rust_outcome.stdout[..PREFIX_LENGTH],
        &c_outcome.stdout[..PREFIX_LENGTH],
        "{case}: output prefix mismatch"
    );
}

#[derive(Clone)]
struct Lcg(u64);

impl Lcg {
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

    fn inclusive(&mut self, low: i32, high: i32) -> i32 {
        low + (self.next_u32() % (high - low + 1) as u32) as i32
    }

    fn choose<'a>(&mut self, values: &'a [&'a str]) -> &'a str {
        values[self.next_u32() as usize % values.len()]
    }
}

fn numeric_input(x: i32, y: i32) -> Vec<u8> {
    format!("{x} {y}\n").into_bytes()
}

fn valid_configurations(c: &Api, rust: &Api) {
    let mut rng = Lcg::new(0x6f4f_01d2_7c39_a511);

    for _ in 0..32 {
        let input = numeric_input(rng.inclusive(-20, 0), rng.inclusive(-20, 0));
        assert_finite(c, rust, &input, "CONFIGS row 1");
    }
    for _ in 0..32 {
        let input = numeric_input(rng.inclusive(1, 20), 0);
        assert_finite(c, rust, &input, "CONFIGS row 2");
    }
    for _ in 0..32 {
        let input = numeric_input(0, rng.inclusive(1, 20));
        assert_finite(c, rust, &input, "CONFIGS row 3");
    }
    for _ in 0..32 {
        let input = numeric_input(rng.inclusive(-20, -1), rng.inclusive(1, 20));
        assert_finite(c, rust, &input, "CONFIGS row 4");
    }
    for _ in 0..32 {
        let leading = rng.choose(&["", " ", "\t", "\n", " \t\n"]);
        let between = rng.choose(&[" ", "\t", "\n", " \n\t "]);
        let trailing = rng.choose(&["", "\n", " trailing", "\tignored"]);
        let input = format!("{leading}+1{between}+4{trailing}").into_bytes();
        assert_finite(c, rust, &input, "CONFIGS row 5");
    }
    for _ in 0..32 {
        let (x, y) = loop {
            let pair = (rng.inclusive(1, 3), rng.inclusive(1, 20));
            if pair != (1, 4) {
                break pair;
            }
        };
        assert_finite(c, rust, &numeric_input(x, y), "CONFIGS row 6");
    }
    for _ in 0..32 {
        let x = rng.inclusive(4, 20);
        let y = rng.inclusive(1, x - 3);
        assert_finite(c, rust, &numeric_input(x, y), "CONFIGS row 7");
    }
    for _ in 0..32 {
        let x = rng.inclusive(4, 20);
        let y = rng.inclusive(x - 2, x + 20);
        assert_finite(c, rust, &numeric_input(x, y), "CONFIGS row 8");
    }
    for _ in 0..16 {
        let input = numeric_input(rng.inclusive(1, 20), rng.inclusive(-20, -1));
        assert_nonterminating(c, rust, &input, "CONFIGS row 9");
    }
    for _ in 0..32 {
        let x = rng.inclusive(0, 12);
        let y = rng.inclusive(0, 12);
        let leading = rng.choose(&["", " ", "\t", "\n\n", " \t\n"]);
        let between = rng.choose(&[" ", "  ", "\t", "\n", "\n\t "]);
        let trailing = rng.choose(&["", "\n", "xyz", "  unconverted 17"]);
        let x_sign = if x > 0 && rng.next_u32() & 1 == 0 {
            "+"
        } else {
            ""
        };
        let y_sign = if y > 0 && rng.next_u32() & 1 == 0 {
            "+"
        } else {
            ""
        };
        let input = format!("{leading}{x_sign}{x}{between}{y_sign}{y}{trailing}").into_bytes();
        assert_finite(c, rust, &input, "CONFIGS row 10");
    }
}

fn error_configurations(c: &Api, rust: &Api) {
    let mut rng = Lcg::new(0x851c_4a7b_d212_9873);
    for _ in 0..24 {
        let whitespace = rng.choose(&["", " ", "\t", "\n", " \t\n  "]);
        assert_finite(c, rust, whitespace.as_bytes(), "ERRORS row 1");
    }
    for _ in 0..24 {
        let leading = rng.choose(&["", " ", "\t", "\n "]);
        let invalid = rng.choose(&["a", "z", "?", ".", "_", "x 3"]);
        let input = format!("{leading}{invalid}").into_bytes();
        assert_finite(c, rust, &input, "ERRORS row 2");
    }
    for _ in 0..24 {
        let x = rng.inclusive(-20, 20);
        let trailing = rng.choose(&["", " ", "\t", "\n", " \n\t"]);
        let input = format!("{x}{trailing}").into_bytes();
        assert_finite(c, rust, &input, "ERRORS row 3");
    }
    for _ in 0..24 {
        let x = rng.inclusive(-20, 20);
        let separator = rng.choose(&[" ", "\t", "\n", " \n\t"]);
        let invalid = rng.choose(&["a", "z", "?", ".", "_", "x17"]);
        let input = format!("{x}{separator}{invalid}").into_bytes();
        assert_finite(c, rust, &input, "ERRORS row 4");
    }
}

#[test]
fn all_configuration_and_error_rows_match_through_ffi() {
    let (c_path, rust_path) = library_paths();
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

    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    valid_configurations(&c, &rust);
    error_configurations(&c, &rust);
}
