use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

/// Capture stdout from a closure that writes to stdout via C printf.
fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe { libc::fflush(libc::fdopen(1, b"w\0".as_ptr() as *const c_char)) };

    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);

    let old_stdout = unsafe { libc::dup(1) };
    assert!(old_stdout >= 0);
    unsafe { libc::dup2(fds[1], 1) };
    unsafe { libc::close(fds[1]) };

    f();

    unsafe { libc::fflush(libc::fdopen(1, b"w\0".as_ptr() as *const c_char)) };

    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut buf = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    reader.read_to_end(&mut buf).unwrap();
    buf
}

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let base = std::path::PathBuf::from(manifest);
    let p = base.join("target/debug/libdriver.so");
    if p.exists() { return p; }
    base.join("target/release/libdriver.so")
}

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c_lib.get(b"printLine").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { r_lib.get(b"printLine").unwrap() };

    let msg = CString::new("hello").unwrap();
    let c_out = capture_stdout(|| unsafe { c_fn(msg.as_ptr()) });
    let r_out = capture_stdout(|| unsafe { r_fn(msg.as_ptr()) });
    assert_eq!(c_out, r_out, "printLine mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));

    // NULL case
    let c_null = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let r_null = capture_stdout(|| unsafe { r_fn(std::ptr::null()) });
    assert_eq!(c_null, r_null, "printLine(NULL) mismatch");
}

#[test]
fn test_bad() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"bad").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r_lib.get(b"bad").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "bad() mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"good").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r_lib.get(b"good").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "good() mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_driver() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r_lib.get(b"driver").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "driver() mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}
