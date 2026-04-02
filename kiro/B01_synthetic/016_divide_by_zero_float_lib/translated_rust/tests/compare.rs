use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::unix::io::FromRawFd;
use std::io::Read;

/// Capture stdout produced by a closure using pipe + dup2.
fn capture_stdout(f: impl FnOnce()) -> String {
    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);
        f();
        libc::fflush(std::ptr::null_mut()); // flush C stdio
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(fds[1]);
        let mut r = std::fs::File::from_raw_fd(fds[0]);
        let mut buf = String::new();
        r.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn c_lib() -> Library {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(&path).expect("Failed to load C libdriver.so") }
}

#[test]
fn test_printIntLine() {
    let lib = c_lib();
    for &val in &[0i32, 1, -1, 42, 100, -999, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(i32)> = lib.get(b"printIntLine").unwrap();
            f(val);
        });
        let rust_out = capture_stdout(|| {
            driver::print_int_line_for_test(val);
        });
        assert_eq!(c_out, rust_out, "printIntLine mismatch for {val}");
    }
}

#[test]
fn test_printLine() {
    let lib = c_lib();
    let cases = ["hello", "world", "", "This would result in a divide by zero"];
    for s in &cases {
        let cs = CString::new(*s).unwrap();
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const i8)> = lib.get(b"printLine").unwrap();
            f(cs.as_ptr());
        });
        let rust_out = capture_stdout(|| {
            driver::print_line_for_test(&cs);
        });
        assert_eq!(c_out, rust_out, "printLine mismatch for {s:?}");
    }
    // Test NULL
    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const i8)> = lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let rust_out = capture_stdout(|| {
        driver::print_line_null_for_test();
    });
    assert_eq!(c_out, rust_out, "printLine mismatch for NULL");
}

#[test]
fn test_bad() {
    let lib = c_lib();
    for &val in &[1.0f32, 2.0, 0.5, -3.0, 100.0, 0.001] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(f32)> = lib.get(b"bad").unwrap();
            f(val);
        });
        let rust_out = capture_stdout(|| {
            driver::bad(val);
        });
        assert_eq!(c_out, rust_out, "bad() mismatch for {val}");
    }
}

#[test]
fn test_good() {
    let lib = c_lib();
    for &val in &[2.0f32, 5.0, 0.0, -1.0, 0.0000001, 100.0] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(f32)> = lib.get(b"good").unwrap();
            f(val);
        });
        let rust_out = capture_stdout(|| {
            driver::good(val);
        });
        assert_eq!(c_out, rust_out, "good() mismatch for {val}");
    }
}

#[test]
fn test_driver() {
    let lib = c_lib();
    let cases: &[(f32, f32)] = &[(2.0, 5.0), (0.0, 1.0), (100.0, -3.0)];
    for &(g, b) in cases {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(f32, f32)> = lib.get(b"driver").unwrap();
            f(g, b);
        });
        let rust_out = capture_stdout(|| {
            driver::driver(g, b);
        });
        assert_eq!(c_out, rust_out, "driver() mismatch for ({g}, {b})");
    }
}
