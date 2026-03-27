use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;

/// Capture stdout from a closure that calls C functions writing to stdout via printf.
/// We dup stdout to a pipe, call the closure, then read the pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    // Flush before capturing
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipe_fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { libc::dup(1) };
    assert!(saved_stdout >= 0);
    unsafe { libc::dup2(pipe_fds[1], 1) };
    unsafe { libc::close(pipe_fds[1]) };

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(saved_stdout, 1) };
    unsafe { libc::close(saved_stdout) };

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

use std::os::unix::io::FromRawFd;

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let so = manifest.join("target/debug/libdriver.so");
    unsafe { Library::new(so).expect("Failed to load Rust .so") }
}

// ---- Level 1: printIntLine ----
#[test]
fn test_print_int_line() {
    let c = c_lib();
    let r = rust_lib();

    for val in [-1, 0, 1, 42, i32::MAX, i32::MIN] {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c.get(b"printIntLine").unwrap() };
            capture_stdout(|| unsafe { f(val) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r.get(b"printIntLine").unwrap() };
            capture_stdout(|| unsafe { f(val) })
        };
        assert_eq!(c_out, r_out, "printIntLine mismatch for val={val}");
    }
}

// ---- Level 1: printLine ----
#[test]
fn test_print_line() {
    let c = c_lib();
    let r = rust_lib();

    let cases: Vec<Option<CString>> = vec![
        Some(CString::new("hello").unwrap()),
        Some(CString::new("").unwrap()),
        Some(CString::new("test 123!@#").unwrap()),
        None, // NULL
    ];

    for case in &cases {
        let ptr = case.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { c.get(b"printLine").unwrap() };
            capture_stdout(|| unsafe { f(ptr) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { r.get(b"printLine").unwrap() };
            capture_stdout(|| unsafe { f(ptr) })
        };
        assert_eq!(c_out, r_out, "printLine mismatch for {:?}", case);
    }
}

// ---- Level 2: bad (only with safe indices 0-9 and negative) ----
#[test]
fn test_bad() {
    let c = c_lib();
    let r = rust_lib();

    for data in [-1, 0, 1, 5, 9] {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c.get(b"bad").unwrap() };
            capture_stdout(|| unsafe { f(data) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r.get(b"bad").unwrap() };
            capture_stdout(|| unsafe { f(data) })
        };
        assert_eq!(c_out, r_out, "bad() mismatch for data={data}");
    }
}

// ---- Level 2: good ----
#[test]
fn test_good() {
    let c = c_lib();
    let r = rust_lib();

    for data in [-1, 0, 5, 9, 10, 100] {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c.get(b"good").unwrap() };
            capture_stdout(|| unsafe { f(data) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r.get(b"good").unwrap() };
            capture_stdout(|| unsafe { f(data) })
        };
        assert_eq!(c_out, r_out, "good() mismatch for data={data}");
    }
}

// ---- Level 3: driver (only with safe badData indices) ----
#[test]
fn test_driver() {
    let c = c_lib();
    let r = rust_lib();

    let cases = [(5, 3), (0, 0), (9, 9), (-1, -1), (10, 5)];
    for (good_data, bad_data) in cases {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int, c_int)> =
                unsafe { c.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(good_data, bad_data) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int, c_int)> =
                unsafe { r.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(good_data, bad_data) })
        };
        assert_eq!(
            c_out, r_out,
            "driver() mismatch for goodData={good_data}, badData={bad_data}"
        );
    }
}
