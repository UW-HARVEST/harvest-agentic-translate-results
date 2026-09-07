use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::fd::IntoRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type PrintLine = unsafe extern "C" fn(*const c_char);
type NoArg = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn _exit(status: c_int) -> !;
}

const STDOUT_FILENO: c_int = 1;
const C_LIBRARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../c_src/build/libdriver.so");
const RUST_LIBRARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/release/libdriver.so");

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_output_path(label: &str) -> PathBuf {
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-differential-{}-{}-{}.out",
        std::process::id(),
        sequence,
        label
    ))
}

fn assert_libraries_exist() {
    assert!(
        Path::new(C_LIBRARY).is_file(),
        "missing C shared library: {C_LIBRARY}"
    );
    assert!(
        Path::new(RUST_LIBRARY).is_file(),
        "missing Rust shared library: {RUST_LIBRARY}"
    );
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let path = temp_output_path("capture");
    let output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    let output_fd = output.into_raw_fd();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(output_fd, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(output_fd), 0);

        call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut bytes = Vec::new();
    File::open(&path).unwrap().read_to_end(&mut bytes).unwrap();
    std::fs::remove_file(path).unwrap();
    bytes
}

unsafe fn load_function<T: Copy>(library: &Library, symbol: &[u8]) -> T {
    unsafe { *library.get::<T>(symbol).unwrap() }
}

fn load_pair() -> (Library, Library) {
    assert_libraries_exist();
    let c_library = unsafe { Library::new(C_LIBRARY).unwrap() };
    let rust_library = unsafe { Library::new(RUST_LIBRARY).unwrap() };
    (c_library, rust_library)
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn randomized_c_strings() -> Vec<CString> {
    let mut cases = vec![
        CString::new(Vec::<u8>::new()).unwrap(),
        CString::new([b'A']).unwrap(),
        CString::new(b"string".as_slice()).unwrap(),
        CString::new(vec![0xff; 1024]).unwrap(),
    ];
    let mut state = 0x5eed_cafe_d15c_a11eu64;

    for _ in 0..256 {
        let length = (xorshift64(&mut state) % 513) as usize;
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            let byte = ((xorshift64(&mut state) % 255) + 1) as u8;
            bytes.push(byte);
        }
        cases.push(CString::new(bytes).unwrap());
    }

    cases
}

#[derive(Debug, Eq, PartialEq)]
struct IsolatedResult {
    raw_status: i32,
    stdout: Vec<u8>,
}

fn run_isolated(
    library: &str,
    symbol: &str,
    argument: Option<i32>,
    repetitions: usize,
) -> IsolatedResult {
    let path = temp_output_path(symbol);
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("ffi_child")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("DRIVER_DIFF_CHILD", "1")
        .env("DRIVER_DIFF_LIBRARY", library)
        .env("DRIVER_DIFF_SYMBOL", symbol)
        .env("DRIVER_DIFF_OUTPUT", &path)
        .env("DRIVER_DIFF_REPETITIONS", repetitions.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(value) = argument {
        command.env("DRIVER_DIFF_ARGUMENT", value.to_string());
    }

    let status = command.status().unwrap();
    let stdout = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(path);
    IsolatedResult {
        raw_status: status.into_raw(),
        stdout,
    }
}

#[test]
fn ffi_child() {
    if std::env::var_os("DRIVER_DIFF_CHILD").is_none() {
        return;
    }

    let library_path = std::env::var("DRIVER_DIFF_LIBRARY").unwrap();
    let symbol = std::env::var("DRIVER_DIFF_SYMBOL").unwrap();
    let repetitions: usize = std::env::var("DRIVER_DIFF_REPETITIONS")
        .unwrap()
        .parse()
        .unwrap();
    let output_path = std::env::var_os("DRIVER_DIFF_OUTPUT").unwrap();
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)
        .unwrap();
    let output_fd = output.into_raw_fd();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(output_fd, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(output_fd), 0);

        let library = Library::new(library_path).unwrap();
        match symbol.as_str() {
            "bad" | "good" => {
                let function: NoArg = load_function(&library, format!("{symbol}\0").as_bytes());
                for _ in 0..repetitions {
                    function();
                }
            }
            "driver" => {
                let function: Driver = load_function(&library, b"driver\0");
                let argument = std::env::var("DRIVER_DIFF_ARGUMENT")
                    .unwrap()
                    .parse()
                    .unwrap();
                for _ in 0..repetitions {
                    function(argument);
                }
            }
            _ => panic!("unsupported child symbol: {symbol}"),
        }

        assert_eq!(fflush(ptr::null_mut()), 0);
        _exit(0);
    }
}

#[test]
fn config_01_print_line_randomized_strings_match() {
    let (c_library, rust_library) = load_pair();
    let c_print_line: PrintLine = unsafe { load_function(&c_library, b"printLine\0") };
    let rust_print_line: PrintLine = unsafe { load_function(&rust_library, b"printLine\0") };

    for (index, input) in randomized_c_strings().iter().enumerate() {
        let c_output = capture_stdout(|| unsafe { c_print_line(input.as_ptr()) });
        let rust_output = capture_stdout(|| unsafe { rust_print_line(input.as_ptr()) });
        assert_eq!(c_output, rust_output, "randomized string case {index}");
    }
}

#[test]
fn config_02_good_matches() {
    let (c_library, rust_library) = load_pair();
    let c_good: NoArg = unsafe { load_function(&c_library, b"good\0") };
    let rust_good: NoArg = unsafe { load_function(&rust_library, b"good\0") };

    for iteration in 0..64 {
        let c_output = capture_stdout(|| unsafe { c_good() });
        let rust_output = capture_stdout(|| unsafe { rust_good() });
        assert_eq!(c_output, rust_output, "iteration {iteration}");
    }
}

#[test]
fn config_03_bad_matches_as_external_process() {
    for repetitions in [1, 2, 3, 16, 64] {
        for iteration in 0..8 {
            let c_result = run_isolated(C_LIBRARY, "bad", None, repetitions);
            let rust_result = run_isolated(RUST_LIBRARY, "bad", None, repetitions);
            assert_eq!(
                c_result, rust_result,
                "iteration {iteration}, repetitions {repetitions}"
            );
        }
    }
}

#[test]
fn config_04_driver_zero_matches_as_external_process() {
    for repetitions in [1, 2, 3, 16, 64] {
        for iteration in 0..8 {
            let c_result = run_isolated(C_LIBRARY, "driver", Some(0), repetitions);
            let rust_result = run_isolated(RUST_LIBRARY, "driver", Some(0), repetitions);
            assert_eq!(
                c_result, rust_result,
                "iteration {iteration}, repetitions {repetitions}"
            );
        }
    }
}

#[test]
fn config_05_driver_nonzero_randomized_values_match() {
    let (c_library, rust_library) = load_pair();
    let c_driver: Driver = unsafe { load_function(&c_library, b"driver\0") };
    let rust_driver: Driver = unsafe { load_function(&rust_library, b"driver\0") };
    let mut values = vec![i32::MIN, -1, 1, i32::MAX];
    let mut state = 0xa11c_e5ed_0123_4567u64;

    while values.len() < 260 {
        let value = xorshift64(&mut state) as i32;
        if value != 0 {
            values.push(value);
        }
    }

    for (index, value) in values.into_iter().enumerate() {
        let c_output = capture_stdout(|| unsafe { c_driver(value) });
        let rust_output = capture_stdout(|| unsafe { rust_driver(value) });
        assert_eq!(
            c_output, rust_output,
            "randomized nonzero value case {index}: {value}"
        );
    }
}

#[test]
fn error_01_print_line_null_matches_exactly() {
    let (c_library, rust_library) = load_pair();
    let c_print_line: PrintLine = unsafe { load_function(&c_library, b"printLine\0") };
    let rust_print_line: PrintLine = unsafe { load_function(&rust_library, b"printLine\0") };

    for iteration in 0..64 {
        let c_output = capture_stdout(|| unsafe { c_print_line(ptr::null()) });
        let rust_output = capture_stdout(|| unsafe { rust_print_line(ptr::null()) });
        assert_eq!(c_output, rust_output, "iteration {iteration}");
        assert!(c_output.is_empty(), "C emitted bytes for a null pointer");
    }
}
