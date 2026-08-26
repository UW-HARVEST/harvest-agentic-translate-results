use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::fs;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicU64, Ordering};

enum File {}

unsafe extern "C" {
    static mut stdin: *mut File;
    static mut stdout: *mut File;

    fn close(fd: RawFd) -> c_int;
    fn dup(fd: RawFd) -> RawFd;
    fn dup2(old_fd: RawFd, new_fd: RawFd) -> RawFd;
    fn fflush(stream: *mut File) -> c_int;
    fn freopen(path: *const c_char, mode: *const c_char, stream: *mut File) -> *mut File;
}

const STDOUT_FILENO: RawFd = 1;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct Observation {
    status: ExitStatus,
    return_value: Option<c_int>,
    stdout: Vec<u8>,
}

struct TempCase {
    input: PathBuf,
    data: PathBuf,
    output: PathBuf,
    result: PathBuf,
}

impl TempCase {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let prefix = format!("driver-diff-{}-{id}", std::process::id());
        let directory = std::env::temp_dir();
        Self {
            input: directory.join(format!("{prefix}-input")),
            data: directory.join(format!("{prefix}-data")),
            output: directory.join(format!("{prefix}-output")),
            result: directory.join(format!("{prefix}-result")),
        }
    }
}

impl Drop for TempCase {
    fn drop(&mut self) {
        for path in [&self.input, &self.data, &self.output, &self.result] {
            let _ = fs::remove_file(path);
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver_c.so")
}

fn rust_library() -> PathBuf {
    let executable = std::env::current_exe().expect("locate integration test executable");
    executable
        .parent()
        .and_then(Path::parent)
        .expect("locate Cargo profile directory")
        .join("libdriver.so")
}

fn c_string(path: &Path) -> CString {
    CString::new(path.as_os_str().as_bytes()).expect("path has no NUL byte")
}

unsafe fn redirect_stdio(input: &Path, output: &Path) -> RawFd {
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "dup(stdout) failed");

    let input_path = c_string(input);
    let output_path = c_string(output);
    let read_mode = c"rb";
    let write_mode = c"wb";

    assert!(
        !unsafe { freopen(input_path.as_ptr(), read_mode.as_ptr(), stdin) }.is_null(),
        "freopen(stdin) failed"
    );
    assert!(
        !unsafe { freopen(output_path.as_ptr(), write_mode.as_ptr(), stdout) }.is_null(),
        "freopen(stdout) failed"
    );
    saved_stdout
}

unsafe fn restore_stdout(saved_stdout: RawFd) {
    assert_eq!(unsafe { fflush(stdout) }, 0, "fflush(stdout) failed");
    assert_eq!(
        unsafe { dup2(saved_stdout, STDOUT_FILENO) },
        STDOUT_FILENO,
        "dup2(stdout) failed"
    );
    assert_eq!(
        unsafe { close(saved_stdout) },
        0,
        "close(stdout copy) failed"
    );
}

unsafe fn call_symbol(library: &Library, symbol: &str, data: Option<&[u8]>) -> c_int {
    match symbol {
        "printLine" => {
            let function: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { library.get(b"printLine") }.expect("load printLine");
            match data {
                Some(bytes) => {
                    let value = CString::new(bytes).expect("test data has no interior NUL");
                    unsafe { function(value.as_ptr()) };
                }
                None => unsafe { function(std::ptr::null()) },
            }
            0
        }
        "bad" => {
            let function: Symbol<unsafe extern "C" fn()> =
                unsafe { library.get(b"bad") }.expect("load bad");
            unsafe { function() };
            0
        }
        "good" => {
            let function: Symbol<unsafe extern "C" fn()> =
                unsafe { library.get(b"good") }.expect("load good");
            unsafe { function() };
            0
        }
        "main" => {
            let function: Symbol<unsafe extern "C" fn() -> c_int> =
                unsafe { library.get(b"main") }.expect("load main");
            unsafe { function() }
        }
        other => panic!("unknown symbol {other}"),
    }
}

#[test]
fn ffi_worker() {
    let Some(library_path) = std::env::var_os("DRIVER_DIFF_LIBRARY") else {
        return;
    };
    let symbol = std::env::var("DRIVER_DIFF_SYMBOL").expect("worker symbol");
    let input_path = PathBuf::from(std::env::var_os("DRIVER_DIFF_INPUT").expect("worker input"));
    let output_path = PathBuf::from(std::env::var_os("DRIVER_DIFF_OUTPUT").expect("worker output"));
    let result_path = PathBuf::from(std::env::var_os("DRIVER_DIFF_RESULT").expect("worker result"));
    let data = std::env::var_os("DRIVER_DIFF_DATA")
        .map(fs::read)
        .transpose()
        .unwrap();

    let library = Library::from(
        unsafe { UnixLibrary::open(Some(library_path), RTLD_NOW | RTLD_LOCAL) }
            .expect("load shared library"),
    );
    let saved_stdout = unsafe { redirect_stdio(&input_path, &output_path) };
    let return_value = unsafe { call_symbol(&library, &symbol, data.as_deref()) };
    unsafe { restore_stdout(saved_stdout) };
    fs::write(result_path, return_value.to_ne_bytes()).expect("write worker result");
}

fn invoke(library: &Path, symbol: &str, input: &[u8], data: Option<&[u8]>) -> Observation {
    assert!(
        library.is_file(),
        "shared library does not exist: {}",
        library.display()
    );

    let files = TempCase::new();
    fs::write(&files.input, input).expect("write worker input");
    if let Some(bytes) = data {
        fs::write(&files.data, bytes).expect("write worker data");
    }

    let mut command = Command::new(std::env::current_exe().expect("locate test executable"));
    command
        .arg("--exact")
        .arg("ffi_worker")
        .arg("--nocapture")
        .env("DRIVER_DIFF_LIBRARY", library)
        .env("DRIVER_DIFF_SYMBOL", symbol)
        .env("DRIVER_DIFF_INPUT", &files.input)
        .env("DRIVER_DIFF_OUTPUT", &files.output)
        .env("DRIVER_DIFF_RESULT", &files.result);
    if data.is_some() {
        command.env("DRIVER_DIFF_DATA", &files.data);
    }

    let child = command.output().expect("run FFI worker");
    let output_bytes = fs::read(&files.output).unwrap_or_default();
    let return_value = fs::read(&files.result).ok().and_then(|bytes| {
        let array: [u8; size_of::<c_int>()] = bytes.try_into().ok()?;
        Some(c_int::from_ne_bytes(array))
    });
    Observation {
        status: child.status,
        return_value,
        stdout: output_bytes,
    }
}

fn assert_differential(symbol: &str, input: &[u8], data: Option<&[u8]>) {
    let c = invoke(&c_library(), symbol, input, data);
    let rust = invoke(&rust_library(), symbol, input, data);

    assert_eq!(
        rust.status, c.status,
        "{symbol}: process status differs\nC: {c:?}\nRust: {rust:?}"
    );
    assert!(
        c.status.success(),
        "{symbol}: both workers failed\nC: {c:?}\nRust: {rust:?}"
    );
    assert_eq!(
        rust.return_value, c.return_value,
        "{symbol}: return value differs\nC: {c:?}\nRust: {rust:?}"
    );
    assert_eq!(
        rust.stdout, c.stdout,
        "{symbol}: stdout differs\nC: {c:?}\nRust: {rust:?}"
    );
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
}

#[test]
fn config_01_print_line_empty() {
    for _ in 0..16 {
        assert_differential("printLine", b"", Some(b""));
    }
}

#[test]
fn config_02_print_line_randomized_strings() {
    let mut rng = Rng(0x5245_5052_4f44_5543);
    for _ in 0..64 {
        let length = (rng.next_u64() % 256 + 1) as usize;
        let bytes: Vec<u8> = (0..length)
            .map(|_| (rng.next_u64() % 255 + 1) as u8)
            .collect();
        assert_differential("printLine", b"", Some(&bytes));
    }
}

#[test]
fn config_03_bad() {
    for _ in 0..16 {
        assert_differential("bad", b"", None);
    }
}

#[test]
fn config_04_good() {
    for _ in 0..16 {
        assert_differential("good", b"", None);
    }
}

#[test]
fn config_05_main_zero() {
    for input in [b"0".as_slice(), b"+0", b"-0", b"000000"] {
        for _ in 0..4 {
            assert_differential("main", input, None);
        }
    }
}

#[test]
fn config_06_main_randomized_nonzero_ints() {
    let mut rng = Rng(0x494e_5445_4745_5253);
    for index in 0..64 {
        let mut value = rng.next_u64() as i32;
        if value == 0 {
            value = if index % 2 == 0 { i32::MIN } else { i32::MAX };
        }
        assert_differential("main", value.to_string().as_bytes(), None);
    }
}

#[test]
fn config_07_main_no_conversion() {
    let mut rng = Rng(0x4e4f_434f_4e56_4552);
    for _ in 0..16 {
        let length = (rng.next_u64() % 32 + 1) as usize;
        let mut input = Vec::with_capacity(length);
        input.push(b'g' + (rng.next_u64() % 20) as u8);
        input.extend((1..length).map(|_| b'a' + (rng.next_u64() % 26) as u8));
        assert_differential("main", &input, None);
    }
}

#[test]
fn config_08_main_eof() {
    for _ in 0..16 {
        assert_differential("main", b"", None);
    }
}

#[test]
fn error_01_print_line_null() {
    for _ in 0..16 {
        assert_differential("printLine", b"", None);
    }
}
