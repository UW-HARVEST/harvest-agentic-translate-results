use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int, c_int);

static PROCESS_STATE: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

struct LoadedDriver {
    _library: Library,
    function: Driver,
}

impl LoadedDriver {
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let function = unsafe {
            *library.get::<Driver>(b"driver\0").unwrap_or_else(|error| {
                panic!("failed to load driver from {}: {error}", path.display())
            })
        };
        Self {
            _library: library,
            function,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libdriver.so"),
        crate_root.join("target/release/libdriver.so"),
    )
}

fn assert_syscall(result: c_int, operation: &str) {
    assert_ne!(
        result,
        -1,
        "{operation} failed: {}",
        std::io::Error::last_os_error()
    );
}

unsafe fn capture_stdout(function: Driver, x: c_int, y: c_int) -> Vec<u8> {
    let mut pipe_fds = [0; 2];
    assert_syscall(unsafe { pipe(pipe_fds.as_mut_ptr()) }, "pipe");
    let saved_stdout = unsafe { dup(1) };
    assert_syscall(saved_stdout, "dup");
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0, "fflush failed");
    assert_syscall(unsafe { dup2(pipe_fds[1], 1) }, "dup2 redirect");
    assert_syscall(unsafe { close(pipe_fds[1]) }, "close write fd");

    unsafe { function(x, y) };

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0, "fflush failed");
    assert_syscall(unsafe { dup2(saved_stdout, 1) }, "dup2 restore");
    assert_syscall(unsafe { close(saved_stdout) }, "close saved stdout");

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

fn compare_case(
    row: usize,
    case: usize,
    c_driver: Driver,
    rust_driver: Driver,
    x: c_int,
    y: c_int,
) {
    let c_output = unsafe { capture_stdout(c_driver, x, y) };
    let rust_output = unsafe { capture_stdout(rust_driver, x, y) };
    assert_eq!(
        rust_output, c_output,
        "CONFIGS.md row {row}, case {case} diverged for driver({x}, {y})"
    );
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn range(&mut self, low: u32, high_inclusive: u32) -> u32 {
        low + self.next_u32() % (high_inclusive - low + 1)
    }
}

fn randomized_cases(row: usize) -> Vec<(c_int, c_int)> {
    const CASES: usize = 128;
    let mut rng = XorShift64::new(0x9e37_79b9_7f4a_7c15 ^ row as u64);
    let mut result = Vec::with_capacity(CASES);

    for index in 0..CASES {
        let pair = match row {
            1 => {
                let mut y = rng.next_u32() as i32;
                if y == 0 {
                    y = 1;
                }
                (0, y)
            }
            2 | 4 | 6 | 8 => {
                let quotient = rng.range(1, 20_000) as i32;
                let divisor = rng.range(1, 30_000) as i32;
                let magnitude = quotient * divisor;
                match row {
                    2 => (magnitude, divisor),
                    4 => (magnitude, -divisor),
                    6 => (-magnitude, divisor),
                    8 => (-magnitude, -divisor),
                    _ => unreachable!(),
                }
            }
            3 | 5 | 7 | 9 => {
                let divisor = rng.range(2, 30_000) as i32;
                let quotient = rng.range(0, 20_000) as i32;
                let remainder = rng.range(1, divisor as u32 - 1) as i32;
                let magnitude = quotient * divisor + remainder;
                match row {
                    3 => (magnitude, divisor),
                    5 => (magnitude, -divisor),
                    7 => (-magnitude, divisor),
                    9 => (-magnitude, -divisor),
                    _ => unreachable!(),
                }
            }
            10 => {
                if index % 2 == 0 {
                    let x = if rng.next_u32() & 1 == 0 {
                        i32::MIN
                    } else {
                        i32::MAX
                    };
                    let mut y = rng.next_u32() as i32;
                    if y == 0 || (x == i32::MIN && y == -1) {
                        y = 1;
                    }
                    (x, y)
                } else {
                    let y = if rng.next_u32() & 1 == 0 {
                        i32::MIN
                    } else {
                        i32::MAX
                    };
                    (rng.next_u32() as i32, y)
                }
            }
            _ => unreachable!("unknown CONFIGS.md row"),
        };
        result.push(pair);
    }
    result
}

#[test]
fn phase_b_all_configuration_rows_match() {
    let _guard = PROCESS_STATE.lock().expect("process-state lock poisoned");
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    let c = unsafe { LoadedDriver::open(&c_path) };
    let rust = unsafe { LoadedDriver::open(&rust_path) };

    for row in 1..=10 {
        for (case, (x, y)) in randomized_cases(row).into_iter().enumerate() {
            compare_case(row, case, c.function, rust.function, x, y);
        }
    }
}

struct ChildResult {
    status: c_int,
    output: Vec<u8>,
}

unsafe fn call_in_child(function: Driver, x: c_int, y: c_int) -> ChildResult {
    let mut pipe_fds = [0; 2];
    assert_syscall(unsafe { pipe(pipe_fds.as_mut_ptr()) }, "pipe");
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0, "fflush failed");

    let child = unsafe { fork() };
    assert_syscall(child, "fork");
    if child == 0 {
        unsafe {
            close(pipe_fds[0]);
            dup2(pipe_fds[1], 1);
            close(pipe_fds[1]);
            function(x, y);
            fflush(ptr::null_mut());
            _exit(0);
        }
    }

    assert_syscall(unsafe { close(pipe_fds[1]) }, "close parent write fd");
    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut output).expect("read child stdout");

    let mut status = 0;
    assert_eq!(
        unsafe { waitpid(child, &mut status, 0) },
        child,
        "waitpid failed: {}",
        std::io::Error::last_os_error()
    );
    ChildResult { status, output }
}

fn terminating_signal(status: c_int) -> Option<c_int> {
    let signal = status & 0x7f;
    if signal == 0 || signal == 0x7f {
        None
    } else {
        Some(signal)
    }
}

fn compare_invalid_case(c_driver: Driver, rust_driver: Driver, x: c_int, y: c_int) {
    let c = unsafe { call_in_child(c_driver, x, y) };
    let rust = unsafe { call_in_child(rust_driver, x, y) };
    assert_eq!(
        terminating_signal(rust.status),
        terminating_signal(c.status),
        "termination diverged for driver({x}, {y}): C status {}, Rust status {}",
        c.status,
        rust.status
    );
    assert_eq!(
        terminating_signal(c.status),
        Some(8),
        "C did not terminate with SIGFPE for driver({x}, {y})"
    );
    assert_eq!(
        rust.output, c.output,
        "pre-termination output diverged for driver({x}, {y})"
    );
    assert!(
        c.output.is_empty(),
        "C unexpectedly emitted output before rejecting driver({x}, {y})"
    );
}

#[test]
fn phase_c_all_error_rows_match() {
    let _guard = PROCESS_STATE.lock().expect("process-state lock poisoned");
    let (c_path, rust_path) = library_paths();
    let c = unsafe { LoadedDriver::open(&c_path) };
    let rust = unsafe { LoadedDriver::open(&rust_path) };

    let mut rng = XorShift64::new(0xd1ff_e2e5_7bad_cafe);
    for _ in 0..8 {
        compare_invalid_case(c.function, rust.function, rng.next_u32() as i32, 0);
    }
    compare_invalid_case(c.function, rust.function, i32::MIN, -1);
}
