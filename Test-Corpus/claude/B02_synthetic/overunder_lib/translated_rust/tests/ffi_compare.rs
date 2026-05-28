// Integration tests that load both the C and Rust shared libraries via libloading
// and verify byte-identical behavior across the FFI boundary.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_double, c_int};
use std::path::PathBuf;
use std::sync::Mutex;

// Global lock to serialize tests that manipulate stdout.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct DataBlock {
    id: c_int,
    value: c_double,
    label: [c_char; 20],
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Tests run with the dev-profile of integration tests, but the cdylib for
    // the lib itself lives in target/{profile}/. Use the release build that
    // we always build before tests.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let release = manifest.join("target/release/liboverunder_lib.so");
    if release.exists() {
        return release;
    }
    let debug = manifest.join("target/debug/liboverunder_lib.so");
    debug
}

unsafe fn load_libs() -> (Library, Library) {
    let c = Library::new(c_lib_path()).expect("Failed to load C library");
    let r = Library::new(rust_lib_path()).expect("Failed to load Rust library");
    (c, r)
}

// Redirect stdout to /dev/null while a closure runs, so test runs aren't noisy
// from the heavy printf usage in `overunder`.
fn silence_stdout<F: FnOnce() -> R, R>(f: F) -> R {
    use std::os::unix::io::AsRawFd;
    let stdout_fd = 1;
    let saved = unsafe { libc_dup(stdout_fd) };
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("open /dev/null");
    unsafe { libc_dup2(devnull.as_raw_fd(), stdout_fd) };
    // Flush libc stdout before/after switching so buffered text doesn't leak.
    unsafe { libc_fflush_stdout() };
    let result = f();
    unsafe { libc_fflush_stdout() };
    unsafe { libc_dup2(saved, stdout_fd) };
    unsafe { libc_close(saved) };
    result
}

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

unsafe fn libc_dup(fd: c_int) -> c_int {
    dup(fd)
}
unsafe fn libc_dup2(a: c_int, b: c_int) -> c_int {
    dup2(a, b)
}
unsafe fn libc_close(fd: c_int) -> c_int {
    close(fd)
}
unsafe fn libc_fflush_stdout() {
    fflush(core::ptr::null_mut());
}

#[test]
fn test_safe_double_to_int() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
            c_lib.get(b"safe_double_to_int").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
            r_lib.get(b"safe_double_to_int").unwrap();

        let test_values = [
            0.0,
            1.0,
            -1.0,
            42.7,
            -42.7,
            1e15,
            -1e15,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            c_int::MAX as f64,
            c_int::MIN as f64,
            c_int::MAX as f64 + 1.0,
            c_int::MIN as f64 - 1.0,
            2147483646.5,
            -2147483647.5,
        ];

        for &v in test_values.iter() {
            let cv = c_fn(v);
            let rv = r_fn(v);
            assert_eq!(cv, rv, "safe_double_to_int mismatch for {}", v);
        }
    }
}

#[test]
fn test_process_with_fallthrough() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            c_lib.get(b"process_with_fallthrough").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            r_lib.get(b"process_with_fallthrough").unwrap();

        for code in -3..=10 {
            for base in [-100, -1, 0, 1, 50, 999, c_int::MAX, c_int::MIN] {
                let cv = c_fn(code, base);
                let rv = r_fn(code, base);
                assert_eq!(cv, rv, "process_with_fallthrough mismatch (code={}, base={})", code, base);
            }
        }
    }
}

#[test]
fn test_handle_pointer_operations() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"handle_pointer_operations").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r_lib.get(b"handle_pointer_operations").unwrap();

        for v in [-5_000, -1, 0, 1, 2, 10, 1000, 1_000_000, c_int::MAX, c_int::MIN, c_int::MAX / 2] {
            let cv = c_fn(v);
            let rv = r_fn(v);
            assert_eq!(cv, rv, "handle_pointer_operations mismatch for {}", v);
        }
    }
}

#[test]
fn test_copy_data_block() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn(*mut DataBlock, *const DataBlock)> =
            c_lib.get(b"copy_data_block").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut DataBlock, *const DataBlock)> =
            r_lib.get(b"copy_data_block").unwrap();

        // Build a source filled with deterministic bytes so we can compare.
        let mut src = DataBlock {
            id: 0x12345678,
            value: 3.14159265358979,
            label: [0; 20],
        };
        let s = b"HelloWorldXYZ12345";
        for (i, b) in s.iter().enumerate() {
            src.label[i] = *b as c_char;
        }

        let mut c_dest = DataBlock { id: 0, value: 0.0, label: [0; 20] };
        let mut r_dest = DataBlock { id: 0, value: 0.0, label: [0; 20] };

        c_fn(&mut c_dest as *mut _, &src as *const _);
        r_fn(&mut r_dest as *mut _, &src as *const _);

        // Compare raw bytes since memcpy semantics include any padding.
        let csz = std::mem::size_of::<DataBlock>();
        let cb = std::slice::from_raw_parts(&c_dest as *const _ as *const u8, csz);
        let rb = std::slice::from_raw_parts(&r_dest as *const _ as *const u8, csz);
        assert_eq!(cb, rb, "copy_data_block produced different byte output");
    }
}

#[test]
fn test_overunder_returns() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"overunder").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"overunder").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 2, 3, 4),
            (-1, -2, -3, -4),
            (10, 20, 30, 40),
            (100, 200, 300, 400),
            (5, 7, 11, 13),
            (-5, 7, -11, 13),
            (1000, -1000, 999, -999),
            (12345, 67890, 13579, 24680),
            (1, 1, 1, 1),
            (3, 4, 5, 6),
            (-7, 0, 0, 7),
        ];

        let _g = STDOUT_LOCK.lock().unwrap();
        for &(a, b, c, d) in cases {
            let (cv, rv) = silence_stdout(|| {
                let cv = c_fn(a, b, c, d);
                let rv = r_fn(a, b, c, d);
                (cv, rv)
            });
            assert_eq!(cv, rv, "overunder mismatch for ({}, {}, {}, {})", a, b, c, d);
        }
    }
}

// Capture stdout and ensure both libraries produce byte-identical printf output.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::AsRawFd;

    let stdout_fd = 1;
    let saved = unsafe { dup(stdout_fd) };

    // Use a temp file to capture.
    let tmp_path = std::env::temp_dir().join(format!(
        "stdcap_{}_{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        let f_handle = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .unwrap();
        unsafe { dup2(f_handle.as_raw_fd(), stdout_fd) };
    }

    unsafe { fflush(core::ptr::null_mut()) };
    f();
    unsafe { fflush(core::ptr::null_mut()) };

    unsafe { dup2(saved, stdout_fd) };
    unsafe { close(saved) };

    let mut buf = Vec::new();
    let mut read_handle = std::fs::File::open(&tmp_path).unwrap();
    read_handle.read_to_end(&mut buf).unwrap();
    let _ = std::fs::remove_file(&tmp_path);
    buf
}

#[test]
fn test_overunder_stdout_matches() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"overunder").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"overunder").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 2, 3, 4),
            (-1, -2, -3, -4),
            (10, 20, 30, 40),
            (5, 7, 11, 13),
            (-5, 7, -11, 13),
            (12345, 67890, 13579, 24680),
        ];

        let _g = STDOUT_LOCK.lock().unwrap();
        for &(a, b, c, d) in cases {
            let c_out = capture_stdout(|| {
                let _ = c_fn(a, b, c, d);
            });
            let r_out = capture_stdout(|| {
                let _ = r_fn(a, b, c, d);
            });
            assert_eq!(
                c_out, r_out,
                "stdout mismatch for ({}, {}, {}, {}):\nC:\n{}\nRust:\n{}",
                a, b, c, d,
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
        }
    }
}
