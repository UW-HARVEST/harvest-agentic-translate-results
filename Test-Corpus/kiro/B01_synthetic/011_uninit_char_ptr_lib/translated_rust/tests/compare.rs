use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::unix::io::FromRawFd;
use std::io::Read;

fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    unsafe {
        let mut pipefd = [0i32; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        libc::dup2(pipefd[1], 1);
        libc::close(pipefd[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let mut buf = Vec::new();
        let mut reader = std::fs::File::from_raw_fd(pipefd[0]);
        libc::fcntl(pipefd[0], libc::F_SETFL, libc::O_NONBLOCK);
        let _ = reader.read_to_end(&mut buf);
        buf
    }
}

fn c_lib() -> Library {
    unsafe { Library::new(std::fs::canonicalize("c_src/build/libdriver.so").unwrap()).unwrap() }
}

fn rust_lib() -> Library {
    unsafe { Library::new(std::fs::canonicalize("target/debug/libdriver.so").unwrap()).unwrap() }
}

#[test]
fn test_printLine() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const i8)> = unsafe { c.get(b"printLine").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const i8)> = unsafe { r.get(b"printLine").unwrap() };

    let input = CString::new("hello world").unwrap();
    let c_out = capture_stdout(|| unsafe { c_fn(input.as_ptr()) });
    let r_out = capture_stdout(|| unsafe { r_fn(input.as_ptr()) });
    assert_eq!(c_out, r_out, "printLine mismatch:\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_printLine_null() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const i8)> = unsafe { c.get(b"printLine").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const i8)> = unsafe { r.get(b"printLine").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let r_out = capture_stdout(|| unsafe { r_fn(std::ptr::null()) });
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn test_good() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c.get(b"good").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r.get(b"good").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "good() mismatch:\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_driver_good() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32)> = unsafe { c.get(b"driver").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(i32)> = unsafe { r.get(b"driver").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn(1) });
    let r_out = capture_stdout(|| unsafe { r_fn(1) });
    assert_eq!(c_out, r_out, "driver(1) mismatch:\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}
