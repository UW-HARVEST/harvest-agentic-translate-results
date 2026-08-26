use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::io::Read;

const C_LIB: &str = env!("C_LIB_PATH");
const RUST_LIB: &str = env!("RUST_LIB_PATH");

/// Capture stdout (including C printf) by dup2-ing a pipe over fd 1.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush all
        let mut pipefd = [0i32; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        libc::dup2(pipefd[1], 1);
        libc::close(pipefd[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let mut buf = String::new();
        let mut reader = std::fs::File::from_raw_fd(pipefd[0]);
        // set non-blocking so we don't hang
        libc::fcntl(pipefd[0], libc::F_SETFL, libc::O_NONBLOCK);
        let _ = reader.read_to_string(&mut buf);
        buf
    }
}

use std::os::unix::io::FromRawFd;

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    let cases: &[&[u8]] = &[b"hello\0", b"test 123\0", b"\0"];
    for &input in cases {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { c_lib.get(b"printLine").unwrap() };
            capture_stdout(|| unsafe { f(input.as_ptr() as *const c_char) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { r_lib.get(b"printLine").unwrap() };
            capture_stdout(|| unsafe { f(input.as_ptr() as *const c_char) })
        };
        assert_eq!(c_out, r_out, "printLine mismatch for {:?}", input);
    }

    // NULL case — should print nothing
    let c_out = {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            unsafe { c_lib.get(b"printLine").unwrap() };
        capture_stdout(|| unsafe { f(std::ptr::null()) })
    };
    let r_out = {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            unsafe { r_lib.get(b"printLine").unwrap() };
        capture_stdout(|| unsafe { f(std::ptr::null()) })
    };
    assert_eq!(c_out, r_out, "printLine NULL mismatch");
}

#[test]
fn test_print_int_line() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    for val in [0, 1, -1, 42, -999, i32::MAX, i32::MIN] {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c_lib.get(b"printIntLine").unwrap() };
            capture_stdout(|| unsafe { f(val) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r_lib.get(b"printIntLine").unwrap() };
            capture_stdout(|| unsafe { f(val) })
        };
        assert_eq!(c_out, r_out, "printIntLine mismatch for {}", val);
    }
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    // Safe inputs for good(): in-bounds and out-of-bounds
    for val in [0, 5, 9, -1, 10, 100] {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c_lib.get(b"good").unwrap() };
            capture_stdout(|| unsafe { f(val) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r_lib.get(b"good").unwrap() };
            capture_stdout(|| unsafe { f(val) })
        };
        assert_eq!(c_out, r_out, "good() mismatch for data={}", val);
    }
}

#[test]
fn test_bad_safe_inputs() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    // Only test in-bounds and negative (safe) inputs for bad()
    // Out-of-bounds positive values cause UB in both C and Rust
    for val in [0, 1, 5, 9, -1, -100] {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c_lib.get(b"bad").unwrap() };
            capture_stdout(|| unsafe { f(val) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r_lib.get(b"bad").unwrap() };
            capture_stdout(|| unsafe { f(val) })
        };
        assert_eq!(c_out, r_out, "bad() mismatch for data={}", val);
    }
}

#[test]
fn test_driver_safe() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    // Use safe values: goodData in-bounds for good(), badData in-bounds for bad()
    let cases: &[(c_int, c_int)] = &[(3, 5), (0, 0), (9, 9), (-1, -1), (10, 2)];
    for &(g, b) in cases {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int, c_int)> =
                unsafe { c_lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(g, b) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int, c_int)> =
                unsafe { r_lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(g, b) })
        };
        assert_eq!(c_out, r_out, "driver() mismatch for ({}, {})", g, b);
    }
}
