use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::raw::c_char;
use std::os::unix::io::FromRawFd;

fn c_lib_path() -> String {
    format!("{}/c_src/build/libdriver.so", env!("CARGO_MANIFEST_DIR"))
}

fn rust_lib_path() -> String {
    format!("{}/target/debug/libdriver.so", env!("CARGO_MANIFEST_DIR"))
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    // Flush both C and Rust stdout before redirecting
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush all C streams
    }
    use std::io::Write;
    std::io::stdout().flush().ok();

    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);

    let old_stdout = unsafe { libc::dup(1) };
    assert!(old_stdout >= 0);
    unsafe { libc::dup2(fds[1], 1) };
    unsafe { libc::close(fds[1]) };

    f();

    // Flush again after the call
    unsafe { libc::fflush(std::ptr::null_mut()) };
    std::io::stdout().flush().ok();

    // Restore stdout
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let mut buf = String::new();
    reader.read_to_string(&mut buf).unwrap();
    buf
}

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c_lib.get(b"printLine").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { r_lib.get(b"printLine").unwrap() };

    let msg = CString::new("hello world").unwrap();
    let c_out = capture_stdout(|| unsafe { c_fn(msg.as_ptr()) });
    let r_out = capture_stdout(|| unsafe { r_fn(msg.as_ptr()) });
    assert_eq!(c_out, r_out, "printLine(\"hello world\") mismatch");

    let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let r_out = capture_stdout(|| unsafe { r_fn(std::ptr::null()) });
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn test_bad() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"bad").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r_lib.get(b"bad").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "bad() mismatch");
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"good").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r_lib.get(b"good").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "good() mismatch");
}

#[test]
fn test_driver() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r_lib.get(b"driver").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "driver() mismatch");
}
