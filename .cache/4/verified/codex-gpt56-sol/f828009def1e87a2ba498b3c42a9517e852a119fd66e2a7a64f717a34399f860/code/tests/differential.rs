use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

const STDOUT_FD: c_int = 1;
const RANDOM_CASES: usize = 256;

struct LoadedDriver {
    _library: Library,
    driver: DriverFn,
}

impl LoadedDriver {
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = unsafe {
            *library
                .get::<DriverFn>(b"driver\0")
                .unwrap_or_else(|error| {
                    panic!("failed to load driver from {}: {error}", path.display())
                })
        };
        Self {
            _library: library,
            driver,
        }
    }

    fn call(&self, x: c_int, y: c_int, z: c_int) -> Vec<u8> {
        capture_stdout(|| unsafe { (self.driver)(x, y, z) })
    }
}

struct DriverPair {
    c: LoadedDriver,
    rust: LoadedDriver,
}

impl DriverPair {
    fn load() -> Self {
        let c_path = manifest_dir().join("c_src/build/libdriver.so");
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "C reference library is missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust library is missing: {}",
            rust_path.display()
        );

        unsafe {
            Self {
                c: LoadedDriver::open(&c_path),
                rust: LoadedDriver::open(&rust_path),
            }
        }
    }

    fn assert_match(&self, inputs: (c_int, c_int, c_int)) -> Vec<u8> {
        let (x, y, z) = inputs;
        let c_output = self.c.call(x, y, z);
        let rust_output = self.rust.call(x, y, z);
        assert_eq!(
            rust_output, c_output,
            "C/Rust output mismatch for driver({x}, {y}, {z})"
        );
        c_output
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("DRIVER_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target/release/libdriver.so"))
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    static STDOUT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = STDOUT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("stdout capture lock was poisoned");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush before call failed");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");

        let saved_stdout = dup(STDOUT_FD);
        assert!(saved_stdout >= 0, "dup stdout failed");
        assert_eq!(dup2(pipe_fds[1], STDOUT_FD), STDOUT_FD, "redirect failed");
        assert_eq!(close(pipe_fds[1]), 0, "close write fd failed");

        call();
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush after call failed");
        assert_eq!(dup2(saved_stdout, STDOUT_FD), STDOUT_FD, "restore failed");
        assert_eq!(close(saved_stdout), 0, "close saved stdout failed");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 256];
        loop {
            let count = read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0, "read capture pipe failed");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close read fd failed");
        output
    }
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_i32(&mut self) -> i32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as i32
    }

    fn next_except(&mut self, excluded: i32) -> i32 {
        loop {
            let value = self.next_i32();
            if value != excluded {
                return value;
            }
        }
    }
}

fn config_1_first_stage_failure_matches_for_random_inputs() {
    let libraries = DriverPair::load();
    let mut rng = FixedRng::new(0x18f4_99a2_317c_005d);
    for _ in 0..RANDOM_CASES {
        libraries.assert_match((rng.next_except(1), rng.next_i32(), rng.next_i32()));
    }
}

fn config_2_second_stage_failure_matches_for_random_inputs() {
    let libraries = DriverPair::load();
    let mut rng = FixedRng::new(0xa367_c80e_6d21_f94b);
    for _ in 0..RANDOM_CASES {
        libraries.assert_match((1, rng.next_except(2), rng.next_i32()));
    }
}

fn config_3_third_stage_failure_matches_for_random_inputs() {
    let libraries = DriverPair::load();
    let mut rng = FixedRng::new(0x4ad9_167b_ef03_2c81);
    for _ in 0..RANDOM_CASES {
        libraries.assert_match((1, 2, rng.next_except(3)));
    }
}

fn config_4_success_matches_for_repeated_calls() {
    let libraries = DriverPair::load();
    // This branch has exactly one possible input tuple.
    for _ in 0..RANDOM_CASES {
        libraries.assert_match((1, 2, 3));
    }
}

fn error_1_first_stage_rejection_is_exact() {
    let libraries = DriverPair::load();
    let output = libraries.assert_match((0, i32::MIN, i32::MAX));
    assert_eq!(output, b"Error: x != 1\nOperation failed\nResult: 1\n");
}

fn error_2_second_stage_rejection_is_exact() {
    let libraries = DriverPair::load();
    let output = libraries.assert_match((1, 3, i32::MIN));
    assert_eq!(
        output,
        b"Error: x == 1 but y != 2\nOperation failed\nResult: 2\n"
    );
}

fn error_3_third_stage_rejection_is_exact() {
    let libraries = DriverPair::load();
    let output = libraries.assert_match((1, 2, 4));
    assert_eq!(
        output,
        b"Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n"
    );
}

fn integer_boundary_and_adjacent_values_match() {
    let libraries = DriverPair::load();
    let cases = [
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX),
        (0, 2, 3),
        (2, 2, 3),
        (1, 1, 3),
        (1, 3, 3),
        (1, 2, 2),
        (1, 2, 4),
    ];
    for inputs in cases {
        libraries.assert_match(inputs);
    }
}

#[test]
fn complete_differential_surface() {
    config_1_first_stage_failure_matches_for_random_inputs();
    config_2_second_stage_failure_matches_for_random_inputs();
    config_3_third_stage_failure_matches_for_random_inputs();
    config_4_success_matches_for_repeated_calls();
    error_1_first_stage_rejection_is_exact();
    error_2_second_stage_rejection_is_exact();
    error_3_third_stage_rejection_is_exact();
    integer_boundary_and_adjacent_values_match();
}
