use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()) };
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(fds[1], 1) };
    unsafe { libc::close(fds[1]) };

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = format!("{}/target/debug/libdriver.so", manifest);
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

// --- printIntLine ---

#[test]
fn test_print_int_line() {
    let c = c_lib();
    let rs = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c.get(b"printIntLine").unwrap() };
    let rs_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { rs.get(b"printIntLine").unwrap() };

    for val in [0, 1, -1, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe { c_fn(val) });
        let rs_out = capture_stdout(|| unsafe { rs_fn(val) });
        assert_eq!(c_out, rs_out, "printIntLine mismatch for {val}");
    }
}

// --- printLine ---

#[test]
fn test_print_line() {
    let c = c_lib();
    let rs = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c.get(b"printLine").unwrap() };
    let rs_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { rs.get(b"printLine").unwrap() };

    let s = CString::new("hello world").unwrap();
    let c_out = capture_stdout(|| unsafe { c_fn(s.as_ptr()) });
    let rs_out = capture_stdout(|| unsafe { rs_fn(s.as_ptr()) });
    assert_eq!(c_out, rs_out, "printLine mismatch for non-null");

    let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let rs_out = capture_stdout(|| unsafe { rs_fn(std::ptr::null()) });
    assert_eq!(c_out, rs_out, "printLine mismatch for null");
}

// --- bad ---

#[test]
fn test_bad() {
    let c = c_lib();
    let rs = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn()> =
        unsafe { c.get(b"bad").unwrap() };
    let rs_fn: Symbol<unsafe extern "C" fn()> =
        unsafe { rs.get(b"bad").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let rs_out = capture_stdout(|| unsafe { rs_fn() });
    assert_eq!(c_out, rs_out, "bad() output mismatch");
}

// --- good ---

#[test]
fn test_good() {
    let c = c_lib();
    let rs = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn()> =
        unsafe { c.get(b"good").unwrap() };
    let rs_fn: Symbol<unsafe extern "C" fn()> =
        unsafe { rs.get(b"good").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let rs_out = capture_stdout(|| unsafe { rs_fn() });
    assert_eq!(c_out, rs_out, "good() output mismatch");
}

// --- driver ---

#[test]
fn test_driver() {
    let c = c_lib();
    let rs = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn()> =
        unsafe { c.get(b"driver").unwrap() };
    let rs_fn: Symbol<unsafe extern "C" fn()> =
        unsafe { rs.get(b"driver").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let rs_out = capture_stdout(|| unsafe { rs_fn() });
    assert_eq!(c_out, rs_out, "driver() output mismatch");
}
