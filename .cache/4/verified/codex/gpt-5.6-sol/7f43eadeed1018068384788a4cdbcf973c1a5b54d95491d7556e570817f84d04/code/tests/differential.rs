use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

type VoidFn = unsafe extern "C" fn();
type DriverFn = unsafe extern "C" fn(c_int);
type PrintLineFn = unsafe extern "C" fn(*const c_char);
type PrintHexCharLineFn = unsafe extern "C" fn(c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const TOO_LARGE: &[u8] = b"data value is too large to perform arithmetic safely.\n";
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

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

    fn nonzero_byte(&mut self) -> u8 {
        (self.next_u32() % 255 + 1) as u8
    }
}

struct StdoutRedirect {
    saved_stdout: c_int,
}

impl StdoutRedirect {
    fn to_file(file: &File) -> Self {
        unsafe {
            assert_eq!(fflush(std::ptr::null_mut()), 0);
            let saved_stdout = dup(STDOUT_FILENO);
            assert!(saved_stdout >= 0, "dup(stdout) failed");
            assert_eq!(
                dup2(file.as_raw_fd(), STDOUT_FILENO),
                STDOUT_FILENO,
                "dup2(capture, stdout) failed"
            );
            Self { saved_stdout }
        }
    }
}

impl Drop for StdoutRedirect {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(fflush(std::ptr::null_mut()), 0);
            assert_eq!(
                dup2(self.saved_stdout, STDOUT_FILENO),
                STDOUT_FILENO,
                "dup2(saved stdout, stdout) failed"
            );
            assert_eq!(close(self.saved_stdout), 0, "close(saved stdout) failed");
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let test_binary = env::current_exe().expect("locate integration test binary");
    test_binary
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("locate Cargo target directory")
        .join("release")
        .join("libdriver.so")
}

fn capture_path(label: &str) -> PathBuf {
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "driver-differential-{}-{id}-{label}.bin",
        std::process::id()
    ))
}

fn probe(library: &Path, mode: &str, label: &str) -> Vec<u8> {
    assert!(
        library.is_file(),
        "shared library does not exist: {}",
        library.display()
    );

    let capture = capture_path(label);
    let output = Command::new(env::current_exe().expect("locate integration test binary"))
        .arg("--exact")
        .arg("ffi_probe")
        .arg("--nocapture")
        .env("DRIVER_PROBE_LIBRARY", library)
        .env("DRIVER_PROBE_MODE", mode)
        .env("DRIVER_CAPTURE_PATH", &capture)
        .output()
        .expect("run FFI probe child");

    assert!(
        output.status.success(),
        "probe failed for {} mode {mode}\nstdout:\n{}\nstderr:\n{}",
        library.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytes = fs::read(&capture).expect("read FFI output capture");
    fs::remove_file(&capture).expect("remove FFI output capture");
    bytes
}

fn compare_mode(mode: &str) -> Vec<u8> {
    let c_output = probe(&c_library_path(), mode, "c");
    let rust_output = probe(&rust_library_path(), mode, "rust");
    assert_eq!(rust_output, c_output, "byte mismatch in mode {mode}");
    c_output
}

fn occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

unsafe fn run_mode(library: &Library, mode: &str) {
    match mode {
        "noop" => {}
        "print_line_empty" => {
            let function: Symbol<PrintLineFn> =
                unsafe { library.get(b"printLine\0").expect("load printLine") };
            let line = CString::new(Vec::<u8>::new()).unwrap();
            unsafe { function(line.as_ptr()) };
        }
        "print_line_one" => {
            let function: Symbol<PrintLineFn> =
                unsafe { library.get(b"printLine\0").expect("load printLine") };
            let mut random = XorShift64::new(0x7612_9d48_ea37_c5b1);
            for _ in 0..512 {
                let line = CString::new([random.nonzero_byte()]).unwrap();
                unsafe { function(line.as_ptr()) };
            }
        }
        "print_line_many" => {
            let function: Symbol<PrintLineFn> =
                unsafe { library.get(b"printLine\0").expect("load printLine") };
            let mut random = XorShift64::new(0xbb67_ae85_84ca_a73b);
            for _ in 0..512 {
                let len = random.next_u32() as usize % 255 + 2;
                let bytes: Vec<u8> = (0..len).map(|_| random.nonzero_byte()).collect();
                let line = CString::new(bytes).unwrap();
                unsafe { function(line.as_ptr()) };
            }
        }
        "print_hex_negative" => {
            let function: Symbol<PrintHexCharLineFn> = unsafe {
                library
                    .get(b"printHexCharLine\0")
                    .expect("load printHexCharLine")
            };
            let mut random = XorShift64::new(0x3c6e_f372_fe94_f82b);
            unsafe {
                function(c_char::MIN);
                function(-1);
            }
            for _ in 0..512 {
                let value = -((random.next_u32() % 128 + 1) as c_int) as c_char;
                unsafe { function(value) };
            }
        }
        "print_hex_zero" => {
            let function: Symbol<PrintHexCharLineFn> = unsafe {
                library
                    .get(b"printHexCharLine\0")
                    .expect("load printHexCharLine")
            };
            unsafe { function(0) };
        }
        "print_hex_positive" => {
            let function: Symbol<PrintHexCharLineFn> = unsafe {
                library
                    .get(b"printHexCharLine\0")
                    .expect("load printHexCharLine")
            };
            let mut random = XorShift64::new(0xa54f_f53a_5f1d_36f1);
            unsafe {
                function(1);
                function(c_char::MAX);
            }
            for _ in 0..512 {
                let value = (random.next_u32() % c_char::MAX as u32 + 1) as c_char;
                unsafe { function(value) };
            }
        }
        "bad" => {
            let function: Symbol<VoidFn> = unsafe { library.get(b"bad\0").expect("load bad") };
            for _ in 0..64 {
                unsafe { function() };
            }
        }
        "good" | "good_too_large" => {
            let function: Symbol<VoidFn> = unsafe { library.get(b"good\0").expect("load good") };
            for _ in 0..64 {
                unsafe { function() };
            }
        }
        "driver_zero" => {
            let function: Symbol<DriverFn> =
                unsafe { library.get(b"driver\0").expect("load driver") };
            for _ in 0..64 {
                unsafe { function(0) };
            }
        }
        "driver_nonzero" => {
            let function: Symbol<DriverFn> =
                unsafe { library.get(b"driver\0").expect("load driver") };
            let mut random = XorShift64::new(0x510e_527f_ade6_82d1);
            unsafe {
                function(c_int::MIN);
                function(c_int::MAX);
                function(-1);
                function(1);
            }
            for _ in 0..512 {
                let mut value = random.next_u32() as c_int;
                if value == 0 {
                    value = 1;
                }
                unsafe { function(value) };
            }
        }
        "print_line_null" => {
            let function: Symbol<PrintLineFn> =
                unsafe { library.get(b"printLine\0").expect("load printLine") };
            for _ in 0..64 {
                unsafe { function(std::ptr::null()) };
            }
        }
        unknown => panic!("unknown probe mode: {unknown}"),
    }
}

#[test]
fn ffi_probe() {
    let Ok(library_path) = env::var("DRIVER_PROBE_LIBRARY") else {
        return;
    };
    let mode = env::var("DRIVER_PROBE_MODE").expect("probe mode");
    let capture_path = env::var("DRIVER_CAPTURE_PATH").expect("capture path");
    let capture_file = File::create(capture_path).expect("create capture file");
    let library = unsafe { Library::new(library_path).expect("load shared library") };

    let redirect = StdoutRedirect::to_file(&capture_file);
    unsafe { run_mode(&library, &mode) };
    drop(redirect);
}

#[test]
fn all_c_exports_load_from_both_libraries() {
    for path in [c_library_path(), rust_library_path()] {
        let library = unsafe { Library::new(&path).expect("load shared library") };
        for symbol in [
            b"bad\0".as_slice(),
            b"driver\0",
            b"good\0",
            b"printHexCharLine\0",
            b"printLine\0",
        ] {
            let _: Symbol<*mut c_void> = unsafe {
                library
                    .get(symbol)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
            };
        }
    }
}

#[test]
fn config_01_print_line_empty() {
    compare_mode("print_line_empty");
}

#[test]
fn config_02_print_line_one_byte() {
    compare_mode("print_line_one");
}

#[test]
fn config_03_print_line_many_bytes() {
    compare_mode("print_line_many");
}

#[test]
fn config_04_print_hex_negative() {
    compare_mode("print_hex_negative");
}

#[test]
fn config_05_print_hex_zero() {
    compare_mode("print_hex_zero");
}

#[test]
fn config_06_print_hex_positive() {
    compare_mode("print_hex_positive");
}

#[test]
fn config_07_bad() {
    compare_mode("bad");
}

#[test]
fn config_08_good() {
    compare_mode("good");
}

#[test]
fn config_09_driver_zero() {
    compare_mode("driver_zero");
}

#[test]
fn config_10_driver_nonzero() {
    compare_mode("driver_nonzero");
}

#[test]
fn error_01_print_line_null_returns_without_output() {
    let null_output = compare_mode("print_line_null");
    let no_call_output = compare_mode("noop");
    assert_eq!(null_output, no_call_output);
    assert!(null_output.is_empty());
}

#[test]
fn error_02_good_rejects_oversized_internal_value() {
    let output = compare_mode("good_too_large");
    assert_eq!(occurrences(&output, TOO_LARGE), 64);
}
