use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    // Flush before redirecting
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()) };
    let read_fd = pipes[0];
    let write_fd = pipes[1];

    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1) };
    unsafe { libc::close(write_fd) };

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut buf = String::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_string(&mut buf).unwrap();
    buf
}

use std::os::unix::io::FromRawFd;

// ---- printIntLine ----
#[test]
fn test_print_int_line() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"printIntLine").unwrap() };

    for val in [0, 1, -1, 42, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe { c_fn(val) });
        let r_out = capture_stdout(|| unsafe { driver::printIntLine(val) });
        assert_eq!(c_out, r_out, "printIntLine({val}) mismatch");
    }
}

// ---- printLine ----
#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c_lib.get(b"printLine").unwrap() };

    // Non-null string
    let s = CString::new("hello").unwrap();
    let c_out = capture_stdout(|| unsafe { c_fn(s.as_ptr()) });
    let r_out = capture_stdout(|| unsafe { driver::printLine(s.as_ptr()) });
    assert_eq!(c_out, r_out, "printLine(\"hello\") mismatch");

    // Null pointer - should print nothing
    let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let r_out = capture_stdout(|| unsafe { driver::printLine(std::ptr::null()) });
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

// ---- good ----
#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"good").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { driver::good() });
    assert_eq!(c_out, r_out, "good() mismatch");
}

// ---- driver(1) calls good ----
#[test]
fn test_driver_good() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn(1) });
    let r_out = capture_stdout(|| driver::driver(1));
    assert_eq!(c_out, r_out, "driver(1) mismatch");
}

// Note: We skip testing bad() and driver(0) because they invoke undefined
// behavior (buffer overflow via alloca(10) writing 10 ints). The C version
// may crash or produce unpredictable output. We only test well-defined paths.
