use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// Capture stdout produced by a closure that calls FFI functions.
fn capture_stdout(f: impl FnOnce()) -> String {
    // flush rust stdout first
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    unsafe {
        let mut pipe_fds = [0i32; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let old_stdout = dup(1);
        dup2(pipe_fds[1], 1);

        f();

        // flush libc stdout
        fflush(std::ptr::null_mut());
        dup2(old_stdout, 1);
        close(old_stdout);
        close(pipe_fds[1]);

        let mut buf = String::new();
        let mut read_end = std::fs::File::from_raw_fd(pipe_fds[0]);
        read_end.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so");
    unsafe { Library::new(path).expect("load Rust .so") }
}

#[test]
fn test_print_line_non_null() {
    let c = c_lib();
    let r = rust_lib();

    let test_strings = ["hello", "", "A", "test with spaces", "123"];
    for s in &test_strings {
        let cs = CString::new(*s).unwrap();
        let c_out = {
            let func: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { c.get(b"printLine").unwrap() };
            capture_stdout(|| unsafe { func(cs.as_ptr()) })
        };
        let r_out = {
            let func: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { r.get(b"printLine").unwrap() };
            capture_stdout(|| unsafe { func(cs.as_ptr()) })
        };
        assert_eq!(c_out, r_out, "printLine mismatch for {:?}", s);
    }
}

#[test]
fn test_print_line_null() {
    let c = c_lib();
    let r = rust_lib();

    let c_out = {
        let func: Symbol<unsafe extern "C" fn(*const c_char)> =
            unsafe { c.get(b"printLine").unwrap() };
        capture_stdout(|| unsafe { func(std::ptr::null()) })
    };
    let r_out = {
        let func: Symbol<unsafe extern "C" fn(*const c_char)> =
            unsafe { r.get(b"printLine").unwrap() };
        capture_stdout(|| unsafe { func(std::ptr::null()) })
    };
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn test_driver() {
    let c = c_lib();
    let r = rust_lib();

    // Test various data values: 0, 1, 50, 99, 100, 200, -1
    let test_values: &[c_int] = &[0, 1, 5, 50, 98, 99, 100, 200];
    for &val in test_values {
        let c_out = {
            let func: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { func(val) })
        };
        let r_out = {
            let func: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { func(val) })
        };
        assert_eq!(c_out, r_out, "driver({}) mismatch: C={:?} Rust={:?}", val, c_out, r_out);
    }
}
