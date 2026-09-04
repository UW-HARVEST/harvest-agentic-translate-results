use libloading::Library;
use std::env;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int, c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library must be built before tests")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libdriver.so")
        .canonicalize()
        .expect("release Rust shared library must be built before tests")
}

unsafe fn capture_stdout(driver: Driver, x: i32, y: i32) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture mutex poisoned");

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);

    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    unsafe { driver(x, y) };
    let flush_result = unsafe { fflush(ptr::null_mut()) };
    let restore_result = unsafe { dup2(saved_stdout, 1) };
    let close_saved_result = unsafe { close(saved_stdout) };

    assert_eq!(flush_result, 0);
    assert_eq!(restore_result, 1);
    assert_eq!(close_saved_result, 0);

    let mut output = Vec::new();
    let mut chunk = [0_u8; 128];
    loop {
        let bytes_read = unsafe {
            read(
                pipe_fds[0],
                chunk.as_mut_ptr().cast::<c_void>(),
                chunk.len(),
            )
        };
        assert!(bytes_read >= 0, "read from capture pipe failed");
        if bytes_read == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..bytes_read as usize]);
    }
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0);
    output
}

struct FixedRng(u64);

impl FixedRng {
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

    fn positive(&mut self, maximum: u32) -> i32 {
        (1 + self.next_u32() % maximum) as i32
    }

    fn any_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

fn signed_division_cases(
    rng: &mut FixedRng,
    numerator_sign: i32,
    denominator_sign: i32,
    exact: bool,
) -> Vec<(i32, i32)> {
    (0..128)
        .map(|_| {
            let denominator_magnitude = if exact {
                rng.positive(30_000)
            } else {
                2 + (rng.next_u32() % 29_999) as i32
            };
            let quotient_magnitude = rng.positive(30_000);
            let remainder_magnitude = if exact {
                0
            } else {
                1 + (rng.next_u32() % (denominator_magnitude as u32 - 1)) as i32
            };
            let numerator_magnitude =
                quotient_magnitude * denominator_magnitude + remainder_magnitude;
            (
                numerator_sign * numerator_magnitude,
                denominator_sign * denominator_magnitude,
            )
        })
        .collect()
}

unsafe fn compare_row(row: usize, cases: &[(i32, i32)], c_driver: Driver, rust_driver: Driver) {
    assert!(
        cases.len() >= 64,
        "CONFIGS.md row {row} needs many randomized cases"
    );
    for &(x, y) in cases {
        let c_output = unsafe { capture_stdout(c_driver, x, y) };
        let rust_output = unsafe { capture_stdout(rust_driver, x, y) };
        assert_eq!(
            c_output, rust_output,
            "CONFIGS.md row {row} diverged for driver({x}, {y})"
        );
    }
}

#[test]
fn valid_configuration_surface_matches_byte_for_byte() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let c_driver: Driver = *unsafe { c_library.get(b"driver\0") }.expect("C driver export");
    let rust_driver: Driver =
        *unsafe { rust_library.get(b"driver\0") }.expect("Rust driver export");

    let mut rng = FixedRng::new(0x7d68_29f4_01ac_e35b);

    let row_1: Vec<_> = (0..128)
        .map(|_| (0, rng.positive(i32::MAX as u32)))
        .collect();
    let row_2: Vec<_> = (0..128)
        .map(|_| (0, -rng.positive(i32::MAX as u32)))
        .collect();

    let row_3 = signed_division_cases(&mut rng, 1, 1, true);
    let row_4 = signed_division_cases(&mut rng, 1, 1, false);
    let row_5 = signed_division_cases(&mut rng, 1, -1, true);
    let row_6 = signed_division_cases(&mut rng, 1, -1, false);
    let row_7 = signed_division_cases(&mut rng, -1, 1, true);
    let row_8 = signed_division_cases(&mut rng, -1, 1, false);
    let row_9 = signed_division_cases(&mut rng, -1, -1, true);
    let row_10 = signed_division_cases(&mut rng, -1, -1, false);

    let row_11: Vec<_> = (0..128)
        .map(|index| {
            let x = if index % 2 == 0 { i32::MIN } else { i32::MAX };
            let y = loop {
                let candidate = rng.any_i32();
                if candidate != 0 && !(x == i32::MIN && candidate == -1) {
                    break candidate;
                }
            };
            (x, y)
        })
        .collect();

    let row_12: Vec<_> = (0..128)
        .map(|index| {
            let y = if index % 2 == 0 { i32::MIN } else { i32::MAX };
            (rng.any_i32(), y)
        })
        .collect();

    let rows = [
        row_1, row_2, row_3, row_4, row_5, row_6, row_7, row_8, row_9, row_10, row_11, row_12,
    ];
    for (index, cases) in rows.iter().enumerate() {
        unsafe { compare_row(index + 1, cases, c_driver, rust_driver) };
    }
}

fn run_crash_child(library: &Path, x: i32, y: i32) -> ExitStatus {
    Command::new(env::current_exe().expect("current integration-test executable"))
        .arg("--exact")
        .arg("crash_child")
        .arg("--nocapture")
        .env("DRIVER_CRASH_LIBRARY", library)
        .env("DRIVER_CRASH_X", x.to_string())
        .env("DRIVER_CRASH_Y", y.to_string())
        .status()
        .expect("run isolated crash child")
}

#[test]
fn crash_child() {
    let Some(library_path) = env::var_os("DRIVER_CRASH_LIBRARY") else {
        return;
    };
    let x: i32 = env::var("DRIVER_CRASH_X")
        .expect("child x")
        .parse()
        .expect("integer child x");
    let y: i32 = env::var("DRIVER_CRASH_Y")
        .expect("child y")
        .parse()
        .expect("integer child y");

    let library = unsafe { Library::new(library_path) }.expect("load crash-test shared library");
    let driver: Driver = *unsafe { library.get(b"driver\0") }.expect("driver export");
    unsafe { driver(x, y) };
    panic!("driver({x}, {y}) unexpectedly returned");
}

#[cfg(unix)]
#[test]
fn error_surface_matches_exact_signal() {
    use std::os::unix::process::ExitStatusExt;

    let c_path = c_library_path();
    let rust_path = rust_library_path();

    let zero_divisor_numerators = [i32::MIN, -1, 0, 1, i32::MAX];
    for x in zero_divisor_numerators {
        let c_status = run_crash_child(&c_path, x, 0);
        let rust_status = run_crash_child(&rust_path, x, 0);
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "ERRORS.md row 1 diverged for driver({x}, 0)"
        );
        assert_eq!(
            c_status.signal(),
            Some(8),
            "C ground truth did not produce SIGFPE for driver({x}, 0)"
        );
    }

    let c_status = run_crash_child(&c_path, i32::MIN, -1);
    let rust_status = run_crash_child(&rust_path, i32::MIN, -1);
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "ERRORS.md row 2 diverged"
    );
    assert_eq!(
        c_status.signal(),
        Some(8),
        "C ground truth did not produce SIGFPE for INT_MIN / -1"
    );
}
