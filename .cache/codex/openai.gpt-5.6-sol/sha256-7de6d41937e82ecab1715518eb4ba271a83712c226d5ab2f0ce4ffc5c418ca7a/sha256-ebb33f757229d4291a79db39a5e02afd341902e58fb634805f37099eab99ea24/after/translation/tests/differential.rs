use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

#[derive(Debug)]
enum Operation {
    Driver(Vec<u8>),
    NullDriver,
    Runs(Vec<c_int>),
}

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    stdout: Vec<u8>,
    termination: Termination,
}

#[derive(Debug, Eq, PartialEq)]
enum Termination {
    Exit(c_int),
    Signal(c_int),
    Other(c_int),
}

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

    fn range(&mut self, low: i32, high: i32) -> i32 {
        assert!(low <= high);
        let width = (i64::from(high) - i64::from(low) + 1) as u64;
        (i64::from(low) + i64::try_from(u64::from(self.next_u32()) % width).unwrap()) as i32
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        manifest.join("../c_src/build/libdriver.so"),
        manifest.join("target/release/libdriver.so"),
    )
}

fn c_string_bytes(text: &str) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    assert!(!bytes.contains(&0));
    bytes.push(0);
    bytes
}

fn capture(path: &Path, operation: &Operation) -> Outcome {
    assert!(
        path.is_file(),
        "shared object is missing: {}",
        path.display()
    );

    unsafe {
        fflush(ptr::null_mut());
    }

    let mut fds = [-1; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        unsafe {
            close(fds[0]);
            if dup2(fds[1], 1) < 0 {
                _exit(121);
            }
            close(fds[1]);

            let Ok(library) = Library::new(path) else {
                _exit(122);
            };

            match operation {
                Operation::Driver(input) => {
                    let Ok(driver) = library.get::<DriverFn>(b"driver\0") else {
                        _exit(123);
                    };
                    driver(input.as_ptr().cast());
                }
                Operation::NullDriver => {
                    let Ok(driver) = library.get::<DriverFn>(b"driver\0") else {
                        _exit(123);
                    };
                    driver(ptr::null());
                }
                Operation::Runs(values) => {
                    let Ok(run) = library.get::<RunFn>(b"run\0") else {
                        _exit(124);
                    };
                    for &value in values {
                        run(value);
                    }
                }
            }

            fflush(ptr::null_mut());
            _exit(0);
        }
    }

    unsafe {
        close(fds[1]);
    }

    let mut stdout = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let count = unsafe { read(fds[0], chunk.as_mut_ptr().cast(), chunk.len()) };
        assert!(count >= 0, "read failed");
        if count == 0 {
            break;
        }
        stdout.extend_from_slice(&chunk[..count as usize]);
    }
    unsafe {
        close(fds[0]);
    }

    let mut status = 0;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);

    let termination = if status & 0x7f == 0 {
        Termination::Exit((status >> 8) & 0xff)
    } else if status & 0x7f != 0x7f {
        Termination::Signal(status & 0x7f)
    } else {
        Termination::Other(status)
    };

    Outcome {
        stdout,
        termination,
    }
}

fn compare(operation: Operation) -> Outcome {
    let (c_path, rust_path) = library_paths();
    let c = capture(&c_path, &operation);
    let rust = capture(&rust_path, &operation);
    assert_eq!(rust, c, "operation: {operation:?}");
    c
}

fn compare_success(operation: Operation) -> Vec<u8> {
    let outcome = compare(operation);
    assert_eq!(outcome.termination, Termination::Exit(0));
    outcome.stdout
}

fn compare_driver_success(input: Vec<u8>) -> Vec<u8> {
    compare_success(Operation::Driver(input))
}

fn compare_driver_error(input: Vec<u8>) {
    let output = compare_driver_success(input);
    assert_eq!(output, b"An error occurred\n");
}

#[test]
fn config_01_run_negative_randomized() {
    let mut rng = Lcg::new(0x01a5_5eed);
    for _ in 0..32 {
        compare_success(Operation::Runs(vec![rng.range(-1_000_000, -1)]));
    }
}

#[test]
fn config_02_run_zero() {
    compare_success(Operation::Runs(vec![0]));
}

#[test]
fn config_03_run_positive_randomized() {
    let mut rng = Lcg::new(0x03a5_5eed);
    for _ in 0..32 {
        compare_success(Operation::Runs(vec![rng.range(1, 1_000_000)]));
    }
}

#[test]
fn config_04_run_int_min() {
    compare_success(Operation::Runs(vec![i32::MIN]));
}

#[test]
fn config_05_run_int_max() {
    compare_success(Operation::Runs(vec![i32::MAX]));
}

#[test]
fn config_06_run_repeated_state_randomized() {
    let mut rng = Lcg::new(0x06a5_5eed);
    let values = (0..64).map(|_| rng.range(-10_000, 10_000)).collect();
    let output = compare_success(Operation::Runs(values));
    assert_eq!(output.iter().filter(|&&byte| byte == b'\n').count(), 64 * 4);
}

#[test]
fn config_07_driver_canonical_decimal_randomized() {
    let mut rng = Lcg::new(0x07a5_5eed);
    for _ in 0..32 {
        let value = rng.range(-1_000_000, 1_000_000);
        compare_driver_success(c_string_bytes(&value.to_string()));
    }
}

#[test]
fn config_08_driver_leading_whitespace_randomized() {
    let mut rng = Lcg::new(0x08a5_5eed);
    let whitespace = [" ", "\t", "\n", "\r", "\u{000b}", "\u{000c}", " \t"];
    for index in 0..32 {
        let value = rng.range(-1_000_000, 1_000_000);
        let input = format!("{}{}", whitespace[index % whitespace.len()], value);
        compare_driver_success(c_string_bytes(&input));
    }
}

#[test]
fn config_09_driver_explicit_sign_randomized() {
    let mut rng = Lcg::new(0x09a5_5eed);
    for index in 0..32 {
        let magnitude = rng.range(0, 1_000_000);
        let input = if index % 2 == 0 {
            format!("+{magnitude}")
        } else {
            format!("-{magnitude}")
        };
        compare_driver_success(c_string_bytes(&input));
    }
}

#[test]
fn config_10_driver_trailing_suffix_randomized() {
    let mut rng = Lcg::new(0x10a5_5eed);
    let suffixes = ["x", " bedrooms", "!", "_tail", "\tignored"];
    for index in 0..32 {
        let value = rng.range(-1_000_000, 1_000_000);
        let input = format!("{value}{}", suffixes[index % suffixes.len()]);
        compare_driver_success(c_string_bytes(&input));
    }
}

#[test]
fn config_11_driver_embedded_nul_randomized() {
    let mut rng = Lcg::new(0x11a5_5eed);
    for _ in 0..32 {
        let value = rng.range(-1_000_000, 1_000_000);
        let mut input = value.to_string().into_bytes();
        input.extend_from_slice(b"\0ignored-after-nul\0");
        compare_driver_success(input);
    }
}

#[test]
fn config_12_driver_int_min() {
    compare_driver_success(c_string_bytes(&i32::MIN.to_string()));
}

#[test]
fn config_13_driver_int_max() {
    compare_driver_success(c_string_bytes(&i32::MAX.to_string()));
}

#[test]
fn config_14_driver_composes_two_run_calls() {
    let mut rng = Lcg::new(0x14a5_5eed);
    for _ in 0..32 {
        let value = rng.range(-100_000, 100_000);
        let output = compare_driver_success(c_string_bytes(&value.to_string()));
        assert_eq!(output.iter().filter(|&&byte| byte == b'\n').count(), 8);
    }
}

#[test]
fn error_01_no_decimal_conversion_randomized() {
    let invalid = [
        "", " ", "\t", "\n", "x", "house", "+", "-", " +", " -", "_123", "x99",
    ];
    for input in invalid {
        compare_driver_error(c_string_bytes(input));
    }

    let mut rng = Lcg::new(0xe1a5_5eed);
    for _ in 0..32 {
        let mut input = String::from("x");
        for _ in 0..rng.range(1, 24) {
            input.push(char::from(b'a' + (rng.next_u32() % 26) as u8));
        }
        compare_driver_error(c_string_bytes(&input));
    }
}

#[test]
fn error_02_long_range_error_randomized() {
    let mut rng = Lcg::new(0xe2a5_5eed);
    for index in 0..32 {
        let len = rng.range(40, 200) as usize;
        let digit = char::from(b'1' + (rng.next_u32() % 9) as u8);
        let magnitude: String = std::iter::repeat_n(digit, len).collect();
        let input = if index % 2 == 0 {
            magnitude
        } else {
            format!("-{magnitude}")
        };
        compare_driver_error(c_string_bytes(&input));
    }

    compare_driver_error(c_string_bytes(&"9".repeat(4096)));
}

#[test]
fn error_03_below_int_min_randomized() {
    compare_driver_error(c_string_bytes(&(i64::from(i32::MIN) - 1).to_string()));
    let mut rng = Lcg::new(0xe3a5_5eed);
    for _ in 0..32 {
        let value = i64::from(i32::MIN) - i64::from(rng.range(1, 1_000_000));
        compare_driver_error(c_string_bytes(&value.to_string()));
    }
}

#[test]
fn error_04_above_int_max_randomized() {
    compare_driver_error(c_string_bytes(&(i64::from(i32::MAX) + 1).to_string()));
    let mut rng = Lcg::new(0xe4a5_5eed);
    for _ in 0..32 {
        let value = i64::from(i32::MAX) + i64::from(rng.range(1, 1_000_000));
        compare_driver_error(c_string_bytes(&value.to_string()));
    }
}

#[test]
fn generic_null_driver_pointer_matches_termination() {
    let outcome = compare(Operation::NullDriver);
    assert!(
        matches!(outcome.termination, Termination::Signal(_)),
        "C unexpectedly survived a null input: {outcome:?}"
    );
}

#[test]
fn both_shared_objects_expose_every_c_symbol() {
    let (c_path, rust_path) = library_paths();
    for path in [&c_path, &rust_path] {
        let library = unsafe { Library::new(path) }.unwrap();
        unsafe {
            let _: libloading::Symbol<'_, DriverFn> = library.get(b"driver\0").unwrap();
            let _: libloading::Symbol<'_, RunFn> = library.get(b"run\0").unwrap();
        }
    }
}
