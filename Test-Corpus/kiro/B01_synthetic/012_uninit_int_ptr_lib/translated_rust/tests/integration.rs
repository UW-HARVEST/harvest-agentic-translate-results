use libloading::{Library, Symbol};
use std::ffi::c_int;

fn rust_so() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/target/debug/libdriver.so", manifest)
}

fn c_so() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libdriver.so", manifest)
}

extern "C" { fn fflush(stream: *mut libc::FILE) -> c_int; }

/// Capture stdout from a closure that calls C printf via FFI.
fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    unsafe { fflush(std::ptr::null_mut()); }

    let mut pipe_fds = [0 as c_int; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1); }

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);
        libc::close(pipe_fds[1]);
    }

    let mut buf = vec![0u8; 4096];
    let n = unsafe { libc::read(pipe_fds[0], buf.as_mut_ptr() as *mut _, buf.len()) };
    unsafe { libc::close(pipe_fds[0]); }
    buf.truncate(if n > 0 { n as usize } else { 0 });
    buf
}

#[test]
fn test_print_int_ptr_line() {
    let c_lib = unsafe { Library::new(c_so()).unwrap() };
    let r_lib = unsafe { Library::new(rust_so()).unwrap() };

    for val in &[42i32, -7, 0, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_int)> =
                c_lib.get(b"printIntPtrLine").unwrap();
            f(val);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_int)> =
                r_lib.get(b"printIntPtrLine").unwrap();
            f(val);
        });
        assert_eq!(c_out, r_out, "printIntPtrLine({}) mismatch", val);
    }
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(c_so()).unwrap() };
    let r_lib = unsafe { Library::new(rust_so()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() mismatch");
    assert_eq!(c_out, b"5\n", "good() should print '5\\n'");
}

#[test]
fn test_driver_good() {
    let c_lib = unsafe { Library::new(c_so()).unwrap() };
    let r_lib = unsafe { Library::new(rust_so()).unwrap() };

    for arg in &[1i32, 2, 100, -1] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"driver").unwrap();
            f(*arg);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"driver").unwrap();
            f(*arg);
        });
        assert_eq!(c_out, r_out, "driver({}) mismatch", arg);
    }
}
