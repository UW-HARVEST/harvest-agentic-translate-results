use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

type NoArgFn = unsafe extern "C" fn();
type DriverFn = unsafe extern "C" fn(c_int);
type PrintHexCharLineFn = unsafe extern "C" fn(c_char);
type PrintLineFn = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct StdoutCapture {
    _lock: MutexGuard<'static, ()>,
    saved_stdout: c_int,
    path: PathBuf,
}

impl StdoutCapture {
    fn start() -> Self {
        let lock = STDOUT_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let path = std::env::temp_dir().join(format!(
            "driver-differential-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .expect("create stdout capture file");

        unsafe {
            assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before redirect");
            let saved_stdout = dup(1);
            assert!(saved_stdout >= 0, "duplicate stdout");
            assert_eq!(dup2(file.as_raw_fd(), 1), 1, "redirect stdout");
            Self {
                _lock: lock,
                saved_stdout,
                path,
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.restore();
        let output = fs::read(&self.path).expect("read captured stdout");
        fs::remove_file(&self.path).expect("remove stdout capture file");
        output
    }

    fn restore(&mut self) {
        if self.saved_stdout >= 0 {
            unsafe {
                assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
                assert_eq!(dup2(self.saved_stdout, 1), 1, "restore stdout");
                assert_eq!(close(self.saved_stdout), 0, "close saved stdout");
            }
            self.saved_stdout = -1;
        }
    }
}

impl Drop for StdoutCapture {
    fn drop(&mut self) {
        self.restore();
        if self.path.exists() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    std::env::current_exe()
        .expect("resolve integration-test executable")
        .parent()
        .expect("integration-test deps directory")
        .parent()
        .expect("Cargo profile directory")
        .join("libdriver.so")
}

fn capture_library(path: &Path, invoke: impl FnOnce(&Library)) -> Vec<u8> {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );
    let capture = StdoutCapture::start();
    unsafe {
        let library = Library::new(path).expect("load shared library");
        invoke(&library);
        drop(library);
    }
    capture.finish()
}

fn compare_libraries(invoke: impl Fn(&Library)) -> Vec<u8> {
    let c_output = capture_library(&c_library_path(), &invoke);
    let rust_output = capture_library(&rust_library_path(), invoke);
    assert_eq!(rust_output, c_output);
    c_output
}

unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
    unsafe {
        *library
            .get::<T>(name)
            .unwrap_or_else(|error| panic!("load symbol {:?}: {error}", name))
    }
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

#[test]
fn symbols_are_loadable_from_both_shared_libraries() {
    for path in [c_library_path(), rust_library_path()] {
        unsafe {
            let library = Library::new(&path).expect("load shared library");
            let _: NoArgFn = symbol(&library, b"bad\0");
            let _: DriverFn = symbol(&library, b"driver\0");
            let _: NoArgFn = symbol(&library, b"good\0");
            let _: PrintHexCharLineFn = symbol(&library, b"printHexCharLine\0");
            let _: PrintLineFn = symbol(&library, b"printLine\0");
        }
    }
}

#[test]
fn config_1_print_line_non_null_strings() {
    let mut inputs = vec![
        CString::new(Vec::<u8>::new()).unwrap(),
        CString::new(b"x".to_vec()).unwrap(),
        CString::new(b"format tokens: %s %x %%".to_vec()).unwrap(),
        CString::new(vec![0xff, b'%', b'x']).unwrap(),
    ];
    let mut state = 0x74d0_c3a5_91e2_b68fu64;
    for _ in 0..512 {
        let length = (next_random(&mut state) % 257) as usize;
        let bytes = (0..length)
            .map(|_| (next_random(&mut state) % 255 + 1) as u8)
            .collect::<Vec<_>>();
        inputs.push(CString::new(bytes).unwrap());
    }

    compare_libraries(|library| unsafe {
        let print_line: PrintLineFn = symbol(library, b"printLine\0");
        for input in &inputs {
            print_line(input.as_ptr());
        }
    });
}

#[test]
fn config_2_print_hex_char_line_full_domain() {
    compare_libraries(|library| unsafe {
        let print_hex_char_line: PrintHexCharLineFn = symbol(library, b"printHexCharLine\0");
        for value in i8::MIN..=i8::MAX {
            print_hex_char_line(value as c_char);
        }
    });
}

#[test]
fn config_3_bad() {
    let output = compare_libraries(|library| unsafe {
        let bad: NoArgFn = symbol(library, b"bad\0");
        bad();
    });
    assert_eq!(output, b"fffffffe\n");
}

#[test]
fn config_4_good() {
    let output = compare_libraries(|library| unsafe {
        let good: NoArgFn = symbol(library, b"good\0");
        good();
    });
    assert_eq!(
        output,
        b"04\ndata value is too large to perform arithmetic safely.\n"
    );
}

#[test]
fn config_5_driver_zero() {
    compare_libraries(|library| unsafe {
        let driver: DriverFn = symbol(library, b"driver\0");
        driver(0);
    });
}

#[test]
fn config_6_driver_negative_values() {
    let mut values = vec![c_int::MIN, -1];
    let mut state = 0xa183_74f2_091c_d6e5u64;
    for _ in 0..512 {
        let value = (next_random(&mut state) as c_int) | c_int::MIN;
        values.push(if value == 0 { -1 } else { value });
    }

    compare_libraries(|library| unsafe {
        let driver: DriverFn = symbol(library, b"driver\0");
        for &value in &values {
            driver(value);
        }
    });
}

#[test]
fn config_7_driver_positive_values() {
    let mut values = vec![1, c_int::MAX];
    let mut state = 0x2f90_c5d1_e7b8_436au64;
    for _ in 0..512 {
        let value = (next_random(&mut state) as c_int) & c_int::MAX;
        values.push(if value == 0 { 1 } else { value });
    }

    compare_libraries(|library| unsafe {
        let driver: DriverFn = symbol(library, b"driver\0");
        for &value in &values {
            driver(value);
        }
    });
}

#[test]
fn error_1_print_line_null_pointer() {
    let output = compare_libraries(|library| unsafe {
        let print_line: PrintLineFn = symbol(library, b"printLine\0");
        print_line(ptr::null());
    });
    assert!(output.is_empty());
}

#[test]
fn error_2_good_rejects_char_max_multiplication() {
    let output = compare_libraries(|library| unsafe {
        let good: NoArgFn = symbol(library, b"good\0");
        good();
    });
    assert_eq!(
        output,
        b"04\ndata value is too large to perform arithmetic safely.\n"
    );
    assert!(
        !output
            .windows(b"fffffffe".len())
            .any(|part| part == b"fffffffe")
    );
}
