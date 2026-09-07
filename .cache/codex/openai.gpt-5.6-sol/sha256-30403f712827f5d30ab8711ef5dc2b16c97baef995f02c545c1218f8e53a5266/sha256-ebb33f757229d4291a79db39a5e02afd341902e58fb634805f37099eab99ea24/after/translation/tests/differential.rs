use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type IntFn = unsafe extern "C" fn(c_int);
type DriverFn = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const NEGATIVE_ERROR: &[u8] = b"ERROR: Array index is negative.\n";
const OUT_OF_BOUNDS_ERROR: &[u8] = b"ERROR: Array index is out-of-bounds\n";
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

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

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn negative_i32(&mut self) -> i32 {
        (self.next_u32() | 0x8000_0000) as i32
    }

    fn upper_invalid_index(&mut self) -> i32 {
        10 + (self.next_u32() % (i32::MAX as u32 - 9)) as i32
    }
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn assert_libraries_exist() {
    assert!(
        c_library().is_file(),
        "C shared library is missing; build c_src/build/libdriver.so first"
    );
    assert!(
        rust_library().is_file(),
        "Rust release shared library is missing; run cargo build --release first"
    );
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    unsafe {
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .unwrap();
    }
    output
}

fn run_print_line(path: &Path, values: &[Option<CString>]) -> Vec<u8> {
    let library = unsafe { Library::new(path) }.unwrap();
    let function: PrintLine = unsafe { *library.get(b"printLine\0").unwrap() };
    capture_stdout(|| {
        for value in values {
            let pointer = value.as_ref().map_or(ptr::null(), |value| value.as_ptr());
            unsafe { function(pointer) };
        }
    })
}

fn run_int_function(path: &Path, symbol: &[u8], values: &[i32]) -> Vec<u8> {
    let library = unsafe { Library::new(path) }.unwrap();
    let function: IntFn = unsafe { *library.get(symbol).unwrap() };
    capture_stdout(|| {
        for &value in values {
            unsafe { function(value) };
        }
    })
}

fn run_driver(path: &Path, values: &[(i32, i32)]) -> Vec<u8> {
    let library = unsafe { Library::new(path) }.unwrap();
    let function: DriverFn = unsafe { *library.get(b"driver\0").unwrap() };
    capture_stdout(|| {
        for &(good_data, bad_data) in values {
            unsafe { function(good_data, bad_data) };
        }
    })
}

fn assert_print_line_matches(values: &[Option<CString>]) -> Vec<u8> {
    assert_libraries_exist();
    let c_output = run_print_line(&c_library(), values);
    let rust_output = run_print_line(&rust_library(), values);
    assert_eq!(rust_output, c_output);
    c_output
}

fn assert_int_function_matches(symbol: &[u8], values: &[i32]) -> Vec<u8> {
    assert_libraries_exist();
    let c_output = run_int_function(&c_library(), symbol, values);
    let rust_output = run_int_function(&rust_library(), symbol, values);
    assert_eq!(rust_output, c_output);
    c_output
}

fn assert_driver_matches(values: &[(i32, i32)]) -> Vec<u8> {
    assert_libraries_exist();
    let c_output = run_driver(&c_library(), values);
    let rust_output = run_driver(&rust_library(), values);
    assert_eq!(rust_output, c_output);
    c_output
}

fn run_boundary_subprocess(path: &Path, operation: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ffi_boundary_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DRIVER_BOUNDARY_LIBRARY", path)
        .env("DRIVER_BOUNDARY_OPERATION", operation)
        .output()
        .unwrap()
}

fn assert_boundary_process_matches(operation: &str) {
    assert_libraries_exist();
    let c_output = run_boundary_subprocess(&c_library(), operation);
    let rust_output = run_boundary_subprocess(&rust_library(), operation);
    assert_eq!(rust_output.status.code(), c_output.status.code());
    assert_eq!(rust_output.stdout, c_output.stdout);
    assert_eq!(rust_output.stderr, c_output.stderr);
}

#[test]
fn ffi_boundary_worker() {
    let Ok(path) = std::env::var("DRIVER_BOUNDARY_LIBRARY") else {
        return;
    };
    let operation = std::env::var("DRIVER_BOUNDARY_OPERATION").unwrap();
    let library = unsafe { Library::new(path) }.unwrap();

    match operation.as_str() {
        "bad_one_past" => {
            let function: IntFn = unsafe { *library.get(b"bad\0").unwrap() };
            unsafe { function(10) };
        }
        "driver_bad_one_past" => {
            let function: DriverFn = unsafe { *library.get(b"driver\0").unwrap() };
            let mut rng = Lcg::new(0x2d61_b970_e4ac_835f);
            unsafe { function(0, 10) };
            unsafe { function(9, 10) };
            for _ in 0..128 {
                unsafe { function((rng.next_u32() % 10) as i32, 10) };
            }
        }
        _ => panic!("unknown boundary operation: {operation}"),
    }

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
    }
}

#[test]
fn config_1_print_line_non_null_strings() {
    let mut rng = Lcg::new(0x16b4_1d2f_9557_c1a3);
    let mut values = vec![
        Some(CString::new("").unwrap()),
        Some(CString::new("a").unwrap()),
        Some(CString::new("ERROR: Array index is negative.").unwrap()),
    ];
    for _ in 0..256 {
        let length = (rng.next_u32() % 65) as usize;
        let bytes: Vec<u8> = (0..length)
            .map(|_| b' ' + (rng.next_u32() % 95) as u8)
            .collect();
        values.push(Some(CString::new(bytes).unwrap()));
    }
    assert_print_line_matches(&values);
}

#[test]
fn config_2_print_int_line_all_integer_shapes() {
    let mut rng = Lcg::new(0x6d8f_d19a_803b_21ce);
    let mut values = vec![i32::MIN, -1, 0, 1, i32::MAX];
    values.extend((0..512).map(|_| rng.next_i32()));
    assert_int_function_matches(b"printIntLine\0", &values);
}

#[test]
fn config_3_bad_in_bounds() {
    let mut rng = Lcg::new(0x38dc_a4be_c732_91f0);
    let mut values = vec![0, 9];
    values.extend((0..256).map(|_| (rng.next_u32() % 10) as i32));
    assert_int_function_matches(b"bad\0", &values);
}

#[test]
fn config_4_good_in_bounds() {
    let mut rng = Lcg::new(0xd48a_b3c0_21e9_7645);
    let mut values = vec![0, 9];
    values.extend((0..256).map(|_| (rng.next_u32() % 10) as i32));
    assert_int_function_matches(b"good\0", &values);
}

#[test]
fn config_5_driver_full_valid_pipeline() {
    let mut rng = Lcg::new(0x5ec0_88f1_a473_d29b);
    let mut values = vec![(0, 0), (0, 9), (9, 0), (9, 9)];
    values.extend((0..128).map(|_| ((rng.next_u32() % 10) as i32, (rng.next_u32() % 10) as i32)));
    assert_driver_matches(&values);
}

#[test]
fn error_1_print_line_null_is_silent() {
    let output = assert_print_line_matches(&[None]);
    assert!(output.is_empty());
}

#[test]
fn error_2_bad_negative_index() {
    let mut rng = Lcg::new(0xbe92_55f3_0c16_4ad8);
    let mut values = vec![i32::MIN, -2, -1];
    values.extend((0..128).map(|_| rng.negative_i32()));
    let output = assert_int_function_matches(b"bad\0", &values);
    assert_eq!(output, NEGATIVE_ERROR.repeat(values.len()));
}

#[test]
fn error_3_good_lower_bound() {
    let mut rng = Lcg::new(0x805a_4be3_d269_f17c);
    let mut values = vec![i32::MIN, -2, -1];
    values.extend((0..128).map(|_| rng.negative_i32()));
    let output = assert_int_function_matches(b"good\0", &values);
    assert_eq!(
        output
            .windows(OUT_OF_BOUNDS_ERROR.len())
            .filter(|window| *window == OUT_OF_BOUNDS_ERROR)
            .count(),
        values.len()
    );
}

#[test]
fn error_4_good_upper_bound() {
    let mut rng = Lcg::new(0x271d_f4c8_9a63_b50e);
    let mut values = vec![10, 11, i32::MAX];
    values.extend((0..128).map(|_| rng.upper_invalid_index()));
    let output = assert_int_function_matches(b"good\0", &values);
    assert_eq!(
        output
            .windows(OUT_OF_BOUNDS_ERROR.len())
            .filter(|window| *window == OUT_OF_BOUNDS_ERROR)
            .count(),
        values.len()
    );
}

#[test]
fn error_5_driver_good_data_lower_bound() {
    let mut rng = Lcg::new(0x9fab_264e_713d_08c5);
    let mut values = vec![(i32::MIN, 0), (-1, 9)];
    values.extend((0..64).map(|_| (rng.negative_i32(), (rng.next_u32() % 10) as i32)));
    let output = assert_driver_matches(&values);
    assert_eq!(
        output
            .windows(OUT_OF_BOUNDS_ERROR.len())
            .filter(|window| *window == OUT_OF_BOUNDS_ERROR)
            .count(),
        values.len()
    );
}

#[test]
fn error_6_driver_good_data_upper_bound() {
    let mut rng = Lcg::new(0x4317_aec9_6d20_58bf);
    let mut values = vec![(10, 0), (i32::MAX, 9)];
    values.extend((0..64).map(|_| (rng.upper_invalid_index(), (rng.next_u32() % 10) as i32)));
    let output = assert_driver_matches(&values);
    assert_eq!(
        output
            .windows(OUT_OF_BOUNDS_ERROR.len())
            .filter(|window| *window == OUT_OF_BOUNDS_ERROR)
            .count(),
        values.len()
    );
}

#[test]
fn error_7_driver_bad_data_negative() {
    let mut rng = Lcg::new(0x7c19_0fb2_e845_36ad);
    let mut values = vec![(0, i32::MIN), (9, -1)];
    values.extend((0..64).map(|_| ((rng.next_u32() % 10) as i32, rng.negative_i32())));
    let output = assert_driver_matches(&values);
    assert_eq!(
        output
            .windows(NEGATIVE_ERROR.len())
            .filter(|window| *window == NEGATIVE_ERROR)
            .count(),
        values.len()
    );
}

#[test]
fn generic_boundary_bad_one_past_buffer() {
    assert_boundary_process_matches("bad_one_past");
}

#[test]
fn generic_boundary_driver_bad_data_one_past_buffer() {
    assert_boundary_process_matches("driver_bad_one_past");
}
