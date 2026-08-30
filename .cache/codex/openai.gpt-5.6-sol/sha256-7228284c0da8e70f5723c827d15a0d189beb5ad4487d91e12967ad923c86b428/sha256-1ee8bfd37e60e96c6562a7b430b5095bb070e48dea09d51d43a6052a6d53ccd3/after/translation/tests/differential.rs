use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type DriverFn = unsafe extern "C" fn(c_int, c_int);

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

const STDOUT_FILENO: c_int = 1;
const SIGFPE: c_int = 8;
const RANDOM_CASES_PER_ROW: usize = 64;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct DriverLibrary {
    _library: Library,
    driver: DriverFn,
}

impl DriverLibrary {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = {
            let symbol = unsafe { library.get::<DriverFn>(b"driver\0") }
                .unwrap_or_else(|error| panic!("missing driver in {}: {error}", path.display()));
            *symbol
        };
        Self {
            _library: library,
            driver,
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library must be built before running tests")
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
        })
        .canonicalize()
        .expect("Rust release shared library must be built before running tests")
}

unsafe fn capture_stdout(driver: DriverFn, x: c_int, y: c_int) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let mut pipe_fds = [-1; 2];

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    unsafe { driver(x, y) };

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

fn child_signal(driver: DriverFn, x: c_int, y: c_int) -> Option<c_int> {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        unsafe {
            driver(x, y);
            _exit(0);
        }
    }

    let mut status = 0;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
    let signal = status & 0x7f;
    (signal != 0 && signal != 0x7f).then_some(signal)
}

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

    fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn positive(&mut self) -> i32 {
        (self.next_u32() % i32::MAX as u32 + 1) as i32
    }

    fn negative(&mut self) -> i32 {
        -self.positive()
    }

    fn small_positive(&mut self) -> i32 {
        (self.next_u32() % 46_340 + 1) as i32
    }
}

fn matching_random_pairs(
    rng: &mut XorShift64,
    predicate: impl Fn(i32, i32) -> bool,
) -> Vec<(i32, i32)> {
    let mut cases = Vec::with_capacity(RANDOM_CASES_PER_ROW);
    while cases.len() < RANDOM_CASES_PER_ROW {
        let pair = (rng.i32(), rng.i32());
        if predicate(pair.0, pair.1) {
            cases.push(pair);
        }
    }
    cases
}

fn configuration_rows() -> Vec<(usize, Vec<(i32, i32)>)> {
    let mut rng = XorShift64::new(0x5eed_d1ff_2025_0828);
    let exact = |rng: &mut XorShift64, x_sign: i32, y_sign: i32| {
        (0..RANDOM_CASES_PER_ROW)
            .map(|_| {
                let factor = rng.small_positive();
                let divisor = rng.small_positive();
                (x_sign * factor * divisor, y_sign * divisor)
            })
            .collect()
    };

    vec![
        (
            1,
            (0..RANDOM_CASES_PER_ROW)
                .map(|_| (0, rng.positive()))
                .collect(),
        ),
        (
            2,
            (0..RANDOM_CASES_PER_ROW)
                .map(|_| (0, rng.negative()))
                .collect(),
        ),
        (3, exact(&mut rng, 1, 1)),
        (
            4,
            matching_random_pairs(&mut rng, |x, y| x > 0 && y > 0 && x % y != 0),
        ),
        (5, exact(&mut rng, 1, -1)),
        (
            6,
            matching_random_pairs(&mut rng, |x, y| x > 0 && y < 0 && x % y != 0),
        ),
        (7, exact(&mut rng, -1, 1)),
        (
            8,
            matching_random_pairs(&mut rng, |x, y| x < 0 && y > 0 && x % y != 0),
        ),
        (9, exact(&mut rng, -1, -1)),
        (
            10,
            matching_random_pairs(&mut rng, |x, y| x < 0 && y < 0 && x % y != 0),
        ),
        (
            11,
            matching_random_pairs(&mut rng, |_, y| y != 0 && y != -1)
                .into_iter()
                .map(|(_, y)| (i32::MIN, y))
                .collect(),
        ),
        (
            12,
            matching_random_pairs(&mut rng, |_, y| y != 0)
                .into_iter()
                .map(|(_, y)| (i32::MAX, y))
                .collect(),
        ),
        (
            13,
            (0..RANDOM_CASES_PER_ROW)
                .map(|_| (rng.i32(), i32::MIN))
                .collect(),
        ),
        (
            14,
            (0..RANDOM_CASES_PER_ROW)
                .map(|_| (rng.i32(), i32::MAX))
                .collect(),
        ),
    ]
}

#[test]
fn every_valid_configuration_matches_byte_for_byte() {
    let c = unsafe { DriverLibrary::load(&c_library_path()) };
    let rust = unsafe { DriverLibrary::load(&rust_library_path()) };

    for (row, cases) in configuration_rows() {
        assert_eq!(cases.len(), RANDOM_CASES_PER_ROW);
        for (x, y) in cases {
            let c_output = unsafe { capture_stdout(c.driver, x, y) };
            let rust_output = unsafe { capture_stdout(rust.driver, x, y) };
            assert_eq!(
                rust_output, c_output,
                "CONFIGS.md row {row} diverged for driver({x}, {y})"
            );
        }
    }
}

#[test]
fn every_error_condition_matches_exact_signal() {
    let c = unsafe { DriverLibrary::load(&c_library_path()) };
    let rust = unsafe { DriverLibrary::load(&rust_library_path()) };
    let mut rng = XorShift64::new(0xe220_a839_7b1d_cdaf);

    let mut zero_divisor_cases = vec![0, 1, -1, i32::MIN, i32::MAX];
    zero_divisor_cases.extend((0..16).map(|_| rng.i32()));
    for x in zero_divisor_cases {
        let c_signal = child_signal(c.driver, x, 0);
        let rust_signal = child_signal(rust.driver, x, 0);
        assert_eq!(rust_signal, c_signal, "ERRORS.md row 1 diverged for x={x}");
        assert_eq!(c_signal, Some(SIGFPE), "unexpected C result for x={x}");
    }

    let c_signal = child_signal(c.driver, i32::MIN, -1);
    let rust_signal = child_signal(rust.driver, i32::MIN, -1);
    assert_eq!(rust_signal, c_signal, "ERRORS.md row 2 diverged");
    assert_eq!(c_signal, Some(SIGFPE), "unexpected C overflow result");
}
