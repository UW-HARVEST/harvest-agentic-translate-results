use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};

fn c_lib_path() -> String {
    std::fs::canonicalize("c_src/build/libdriver.so")
        .expect("C .so not found")
        .to_str()
        .unwrap()
        .to_string()
}

fn rust_lib_path() -> String {
    std::fs::canonicalize("target/debug/libdriver.so")
        .expect("Rust .so not found")
        .to_str()
        .unwrap()
        .to_string()
}

fn call_and_capture<F>(lib_path: &str, invoke: F) -> Vec<u8>
where
    F: FnOnce(&Library),
{
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    let mut fds = [0i32; 2];
    unsafe { assert_eq!(libc::pipe(fds.as_mut_ptr()), 0); }
    let read_fd = fds[0];
    let write_fd = fds[1];

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe {
        libc::dup2(write_fd, 1);
        libc::close(write_fd);
    }

    let lib = unsafe { Library::new(lib_path).expect("failed to load library") };
    invoke(&lib);

    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);
    }

    let mut buf = Vec::new();
    let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    unsafe {
        let flags = libc::fcntl(read_fd, libc::F_GETFL);
        libc::fcntl(read_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let _ = read_file.read_to_end(&mut buf);
    buf
}

#[test]
fn test_print_line() {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    let test_str = CString::new("hello world").unwrap();

    let c_out = call_and_capture(&c_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"printLine").unwrap();
        f(test_str.as_ptr());
    });
    let r_out = call_and_capture(&r_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"printLine").unwrap();
        f(test_str.as_ptr());
    });

    assert_eq!(c_out, r_out, "printLine output mismatch");
    assert_eq!(c_out, b"hello world\n");
}

#[test]
fn test_print_line_null() {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();

    let c_out = call_and_capture(&c_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let r_out = call_and_capture(&r_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });

    assert_eq!(c_out, r_out, "printLine(NULL) output mismatch");
    assert!(c_out.is_empty());
}

#[test]
fn test_good() {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();

    let c_out = call_and_capture(&c_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = lib.get(b"good").unwrap();
        f();
    });
    let r_out = call_and_capture(&r_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = lib.get(b"good").unwrap();
        f();
    });

    assert_eq!(c_out, r_out, "good() output mismatch");
    assert_eq!(c_out, b"helperGood1 string\n");
}

#[test]
fn test_driver_good() {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();

    let c_out = call_and_capture(&c_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"driver").unwrap();
        f(1);
    });
    let r_out = call_and_capture(&r_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"driver").unwrap();
        f(1);
    });

    assert_eq!(c_out, r_out, "driver(1) output mismatch");
    assert_eq!(c_out, b"helperGood1 string\n");
}

// bad() and driver(0) invoke UB in C (dangling pointer). Just verify no crash.
#[test]
fn test_bad_no_crash() {
    let r_path = rust_lib_path();
    let _ = call_and_capture(&r_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = lib.get(b"bad").unwrap();
        f();
    });
}

#[test]
fn test_driver_bad_no_crash() {
    let r_path = rust_lib_path();
    let _ = call_and_capture(&r_path, |lib| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"driver").unwrap();
        f(0);
    });
}
