use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const ARRAY_SIZE: usize = 256 * 1024;
const CHILD_LIB: &str = "DRIVER_DIFF_CHILD_LIB";
const CHILD_MODE: &str = "DRIVER_DIFF_CHILD_MODE";
const CHILD_VALUE: &str = "DRIVER_DIFF_CHILD_VALUE";
const CHILD_RESULT: &str = "DRIVER_DIFF_CHILD_RESULT";

type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;
type PerformFn = unsafe extern "C" fn();

static LIBRARY_LOCK: Mutex<()> = Mutex::new(());
static C_LIBRARY: OnceLock<PathBuf> = OnceLock::new();
static RUST_LIBRARY: OnceLock<PathBuf> = OnceLock::new();

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

#[derive(Debug, Eq, PartialEq)]
struct CallResult {
    return_code: c_int,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct ChildOutcome {
    status: ExitStatus,
    result: Option<CallResult>,
}

fn c_library() -> PathBuf {
    C_LIBRARY
        .get_or_init(|| {
            let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
            let output_dir = manifest.join("target/differential-c");
            fs::create_dir_all(&output_dir).expect("create C shared-object directory");
            let output = output_dir.join("libdriver_c.so");
            let status = Command::new("cc")
                .args(["-O2", "-fPIC", "-shared", "-o"])
                .arg(&output)
                .arg(manifest.join("c_src/src/main.c"))
                .status()
                .expect("compile C shared object");
            assert!(status.success(), "C shared-object build failed");
            output
        })
        .clone()
}

fn rust_library() -> PathBuf {
    RUST_LIBRARY
        .get_or_init(|| {
            let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
            let target = manifest.join("target/differential-rust");
            let status = Command::new(env!("CARGO"))
                .args(["build", "--no-default-features", "--lib", "--target-dir"])
                .arg(&target)
                .current_dir(manifest)
                .status()
                .expect("build Rust shared object");
            assert!(status.success(), "Rust shared-object build failed");
            target.join("debug/libdriver.so")
        })
        .clone()
}

unsafe fn array_symbol(library: &Library) -> *mut c_int {
    *library.get::<*mut c_int>(b"array\0").expect("array export")
}

fn run_transform(path: &Path, input: &[c_int]) -> Vec<c_int> {
    assert_eq!(input.len(), ARRAY_SIZE);
    let _guard = LIBRARY_LOCK.lock().unwrap();
    unsafe {
        let library = Library::new(path).expect("load shared object");
        let array = array_symbol(&library);
        std::ptr::copy_nonoverlapping(input.as_ptr(), array, ARRAY_SIZE);
        let perform: Symbol<PerformFn> = library
            .get(b"perform_expensive_operations\0")
            .expect("perform export");
        perform();
        std::slice::from_raw_parts(array, ARRAY_SIZE).to_vec()
    }
}

fn generated_input(seed: u64, class: InputClass) -> Vec<c_int> {
    let mut state = seed;
    let mut values = Vec::with_capacity(ARRAY_SIZE);
    for _ in 0..ARRAY_SIZE {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let raw = (state >> 32) as u32;
        values.push(match class {
            InputClass::Nonnegative => (raw & 0x7fff_ffff) as c_int,
            InputClass::Negative => (raw | 0x8000_0000) as c_int,
            InputClass::Mixed => raw as c_int,
        });
    }
    match class {
        InputClass::Nonnegative => values[..3].copy_from_slice(&[0, 1, c_int::MAX]),
        InputClass::Negative => values[..3].copy_from_slice(&[-1, -2, c_int::MIN]),
        InputClass::Mixed => values[..5].copy_from_slice(&[c_int::MIN, -1, 0, 1, c_int::MAX]),
    }
    values
}

#[derive(Clone, Copy)]
enum InputClass {
    Nonnegative,
    Negative,
    Mixed,
}

fn assert_transform_class(class: InputClass) {
    for seed in [1, 0x1234_5678, 0xdead_beef, u64::MAX] {
        let input = generated_input(seed, class);
        let c_output = run_transform(&c_library(), &input);
        let rust_output = run_transform(&rust_library(), &input);
        if c_output != rust_output {
            let index = c_output
                .iter()
                .zip(&rust_output)
                .position(|(c, rust)| c != rust)
                .unwrap();
            panic!(
                "full-array mismatch for seed {seed} at index {index}: C={}, Rust={}",
                c_output[index], rust_output[index]
            );
        }
    }
}

fn unique_temp_path(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "driver-differential-{}-{}-{label}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn write_result(path: &Path, result: &CallResult) {
    let mut file = File::create(path).expect("create child result");
    file.write_all(&result.return_code.to_ne_bytes()).unwrap();
    file.write_all(&(result.stdout.len() as u64).to_ne_bytes())
        .unwrap();
    file.write_all(&result.stdout).unwrap();
    file.write_all(&(result.stderr.len() as u64).to_ne_bytes())
        .unwrap();
    file.write_all(&result.stderr).unwrap();
}

fn read_u64(cursor: &mut &[u8]) -> u64 {
    let (bytes, rest) = cursor.split_at(std::mem::size_of::<u64>());
    *cursor = rest;
    u64::from_ne_bytes(bytes.try_into().unwrap())
}

fn read_result(path: &Path) -> CallResult {
    let bytes = fs::read(path).expect("read child result");
    let mut cursor = bytes.as_slice();
    let (return_code, rest) = cursor.split_at(std::mem::size_of::<c_int>());
    cursor = rest;
    let stdout_len = read_u64(&mut cursor) as usize;
    let (stdout, rest) = cursor.split_at(stdout_len);
    cursor = rest;
    let stderr_len = read_u64(&mut cursor) as usize;
    let (stderr, rest) = cursor.split_at(stderr_len);
    assert!(rest.is_empty(), "trailing child result bytes");
    CallResult {
        return_code: c_int::from_ne_bytes(return_code.try_into().unwrap()),
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
    }
}

unsafe fn capture_main(main: MainFn, argc: c_int, argv: *mut *mut c_char) -> CallResult {
    let stdout_path = unique_temp_path("stdout");
    let stderr_path = unique_temp_path("stderr");
    let stdout_file = File::create(&stdout_path).expect("create stdout capture");
    let stderr_file = File::create(&stderr_path).expect("create stderr capture");
    let saved_stdout = dup(1);
    let saved_stderr = dup(2);
    assert!(saved_stdout >= 0 && saved_stderr >= 0);
    assert_eq!(dup2(stdout_file.as_raw_fd(), 1), 1);
    assert_eq!(dup2(stderr_file.as_raw_fd(), 2), 2);

    let return_code = main(argc, argv);
    fflush(std::ptr::null_mut());
    assert_eq!(dup2(saved_stdout, 1), 1);
    assert_eq!(dup2(saved_stderr, 2), 2);
    close(saved_stdout);
    close(saved_stderr);
    drop(stdout_file);
    drop(stderr_file);

    let stdout = fs::read(&stdout_path).expect("read stdout capture");
    let stderr = fs::read(&stderr_path).expect("read stderr capture");
    let _ = fs::remove_file(stdout_path);
    let _ = fs::remove_file(stderr_path);
    CallResult {
        return_code,
        stdout,
        stderr,
    }
}

use std::os::fd::AsRawFd;

fn run_child_call() {
    let library_path = PathBuf::from(std::env::var_os(CHILD_LIB).expect("child library"));
    let mode = std::env::var(CHILD_MODE).expect("child mode");
    let value = std::env::var(CHILD_VALUE).unwrap_or_default();
    let result_path = PathBuf::from(std::env::var_os(CHILD_RESULT).expect("result path"));

    unsafe {
        let library = Library::new(library_path).expect("load child shared object");
        let main: Symbol<MainFn> = library.get(b"main\0").expect("main export");
        let result = match mode.as_str() {
            "normal" => {
                let program = CString::new("driver").unwrap();
                let value = CString::new(value).unwrap();
                let mut argv = vec![
                    program.as_ptr() as *mut c_char,
                    value.as_ptr() as *mut c_char,
                    std::ptr::null_mut(),
                ];
                capture_main(*main, 2, argv.as_mut_ptr())
            }
            "argc" => {
                let program = CString::new(value).unwrap();
                let mut argv = vec![program.as_ptr() as *mut c_char, std::ptr::null_mut()];
                capture_main(*main, 1, argv.as_mut_ptr())
            }
            "null_argv" => capture_main(*main, 2, std::ptr::null_mut()),
            "null_seed" => {
                let program = CString::new("driver").unwrap();
                let mut argv = vec![program.as_ptr() as *mut c_char, std::ptr::null_mut()];
                capture_main(*main, 2, argv.as_mut_ptr())
            }
            other => panic!("unknown child mode {other}"),
        };
        write_result(&result_path, &result);
    }
}

fn invoke_child(library: &Path, mode: &str, value: &str) -> ChildOutcome {
    let result_path = unique_temp_path("result");
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ffi_child", "--nocapture"])
        .env(CHILD_LIB, library)
        .env(CHILD_MODE, mode)
        .env(CHILD_VALUE, value)
        .env(CHILD_RESULT, &result_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run FFI child");
    let result = result_path.exists().then(|| read_result(&result_path));
    let _ = fs::remove_file(result_path);
    ChildOutcome { status, result }
}

fn compare_main_case(mode: &str, value: &str) {
    let (c, rust) = std::thread::scope(|scope| {
        let c = scope.spawn(|| invoke_child(&c_library(), mode, value));
        let rust = scope.spawn(|| invoke_child(&rust_library(), mode, value));
        (c.join().unwrap(), rust.join().unwrap())
    });
    assert_eq!(
        c.status.code(),
        rust.status.code(),
        "exit status for {value:?}"
    );
    assert_eq!(
        c.status.signal(),
        rust.status.signal(),
        "signal for {value:?}"
    );
    assert_eq!(c.result, rust.result, "main output for {value:?}");
}

fn compare_main_cases(mode: &str, values: &[&str]) {
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for value in values {
            handles.push(scope.spawn(move || compare_main_case(mode, value)));
        }
        for handle in handles {
            handle.join().unwrap();
        }
    });
}

#[test]
fn ffi_child() {
    if std::env::var_os(CHILD_LIB).is_some() {
        run_child_call();
    }
}

#[test]
fn symbols_are_loadable_and_array_is_zero_initialized() {
    let _guard = LIBRARY_LOCK.lock().unwrap();
    for path in [c_library(), rust_library()] {
        unsafe {
            let library = Library::new(path).unwrap();
            let array = array_symbol(&library);
            let _: Symbol<MainFn> = library.get(b"main\0").unwrap();
            let _: Symbol<PerformFn> = library.get(b"perform_expensive_operations\0").unwrap();
            assert!(std::slice::from_raw_parts(array, ARRAY_SIZE)
                .iter()
                .all(|value| *value == 0));
        }
    }
}

#[test]
fn array_external_read_write_matches() {
    let _guard = LIBRARY_LOCK.lock().unwrap();
    for seed in [7, 19, 0xfeed_face, u64::MAX] {
        let values = generated_input(seed, InputClass::Mixed);
        for path in [c_library(), rust_library()] {
            unsafe {
                let library = Library::new(path).unwrap();
                let array = array_symbol(&library);
                std::ptr::copy_nonoverlapping(values.as_ptr(), array, ARRAY_SIZE);
                assert_eq!(std::slice::from_raw_parts(array, ARRAY_SIZE), values);
            }
        }
    }
}

#[test]
fn operation_matches_for_nonnegative_inputs() {
    assert_transform_class(InputClass::Nonnegative);
}

#[test]
fn operation_matches_for_negative_inputs() {
    assert_transform_class(InputClass::Negative);
}

#[test]
fn operation_matches_for_mixed_inputs() {
    assert_transform_class(InputClass::Mixed);
}

#[test]
fn main_matches_for_canonical_decimal_seeds() {
    compare_main_cases("normal", &["0", "1", "123456789", "4294967295"]);
}

#[test]
fn main_matches_for_whitespace_and_plus_seeds() {
    compare_main_cases("normal", &[" 0", "+1", " \t+42", " 4294967295"]);
}

#[test]
fn main_matches_for_empty_seed() {
    compare_main_case("normal", "");
}

#[test]
fn main_matches_for_negative_zero_spellings() {
    compare_main_cases("normal", &["-0", "-00", " \t-000"]);
}

#[test]
fn rejects_wrong_argc() {
    compare_main_cases("argc", &["driver", "d", "randomized-program-name"]);
}

#[test]
fn rejects_trailing_seed_input() {
    compare_main_cases("normal", &["1x", "+", "42 ", "0xff"]);
}

#[test]
fn rejects_strtoul_overflow() {
    compare_main_cases(
        "normal",
        &[
            "18446744073709551616",
            "99999999999999999999",
            "1000000000000000000000000000000000",
        ],
    );
}

#[test]
fn rejects_seed_above_uint_max() {
    compare_main_cases(
        "normal",
        &["4294967296", "4294967297", "9223372036854775807"],
    );
}

#[test]
fn null_argv_signal_matches() {
    compare_main_case("null_argv", "");
}

#[test]
fn null_seed_signal_matches() {
    compare_main_case("null_seed", "");
}
