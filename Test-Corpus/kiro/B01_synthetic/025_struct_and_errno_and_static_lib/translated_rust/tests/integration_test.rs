use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::unix::io::FromRawFd;
use std::io::Read;

fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe { libc::fflush(std::ptr::null_mut()); }

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1); }

    f();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);
        libc::close(pipe_fds[1]);
    }

    let mut buf = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut buf).unwrap();
    buf
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // Find the built cdylib
    let target_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug");
    target_dir.join("libdriver.so")
}

#[test]
fn test_driver_valid_input() {
    let c_output = {
        let lib = unsafe { Library::new(c_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(*const i8)> =
            unsafe { lib.get(b"driver").unwrap() };
        let input = CString::new("3").unwrap();
        capture_stdout(|| unsafe { func(input.as_ptr()) })
    };

    let rust_output = {
        let lib = unsafe { Library::new(rust_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(*const i8)> =
            unsafe { lib.get(b"driver").unwrap() };
        let input = CString::new("3").unwrap();
        capture_stdout(|| unsafe { func(input.as_ptr()) })
    };

    assert_eq!(
        String::from_utf8_lossy(&c_output),
        String::from_utf8_lossy(&rust_output),
        "driver(\"3\") output mismatch"
    );
}

#[test]
fn test_driver_invalid_input() {
    let c_output = {
        let lib = unsafe { Library::new(c_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(*const i8)> =
            unsafe { lib.get(b"driver").unwrap() };
        let input = CString::new("abc").unwrap();
        capture_stdout(|| unsafe { func(input.as_ptr()) })
    };

    let rust_output = {
        let lib = unsafe { Library::new(rust_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(*const i8)> =
            unsafe { lib.get(b"driver").unwrap() };
        let input = CString::new("abc").unwrap();
        capture_stdout(|| unsafe { func(input.as_ptr()) })
    };

    assert_eq!(
        String::from_utf8_lossy(&c_output),
        String::from_utf8_lossy(&rust_output),
        "driver(\"abc\") output mismatch"
    );
}

#[test]
fn test_run_directly() {
    let c_output = {
        let lib = unsafe { Library::new(c_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(i32)> =
            unsafe { lib.get(b"run").unwrap() };
        capture_stdout(|| unsafe { func(2) })
    };

    let rust_output = {
        let lib = unsafe { Library::new(rust_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(i32)> =
            unsafe { lib.get(b"run").unwrap() };
        capture_stdout(|| unsafe { func(2) })
    };

    assert_eq!(
        String::from_utf8_lossy(&c_output),
        String::from_utf8_lossy(&rust_output),
        "run(2) output mismatch"
    );
}

#[test]
fn test_driver_zero() {
    let c_output = {
        let lib = unsafe { Library::new(c_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(*const i8)> =
            unsafe { lib.get(b"driver").unwrap() };
        let input = CString::new("0").unwrap();
        capture_stdout(|| unsafe { func(input.as_ptr()) })
    };

    let rust_output = {
        let lib = unsafe { Library::new(rust_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(*const i8)> =
            unsafe { lib.get(b"driver").unwrap() };
        let input = CString::new("0").unwrap();
        capture_stdout(|| unsafe { func(input.as_ptr()) })
    };

    assert_eq!(
        String::from_utf8_lossy(&c_output),
        String::from_utf8_lossy(&rust_output),
        "driver(\"0\") output mismatch"
    );
}

#[test]
fn test_driver_negative() {
    let c_output = {
        let lib = unsafe { Library::new(c_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(*const i8)> =
            unsafe { lib.get(b"driver").unwrap() };
        let input = CString::new("-5").unwrap();
        capture_stdout(|| unsafe { func(input.as_ptr()) })
    };

    let rust_output = {
        let lib = unsafe { Library::new(rust_lib_path()).unwrap() };
        let func: Symbol<unsafe extern "C" fn(*const i8)> =
            unsafe { lib.get(b"driver").unwrap() };
        let input = CString::new("-5").unwrap();
        capture_stdout(|| unsafe { func(input.as_ptr()) })
    };

    assert_eq!(
        String::from_utf8_lossy(&c_output),
        String::from_utf8_lossy(&rust_output),
        "driver(\"-5\") output mismatch"
    );
}
