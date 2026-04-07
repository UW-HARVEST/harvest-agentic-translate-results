use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_float, c_int};

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
const RUST_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

/// Capture stdout produced by calling `f` by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        // flush any pending stdout
        libc::fflush(libc::fdopen(1, b"w\0".as_ptr() as *const c_char));

        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);

        f();

        libc::fflush(libc::fdopen(1, b"w\0".as_ptr() as *const c_char));
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(fds[1]);

        let mut buf = String::new();
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        reader.read_to_string(&mut buf).unwrap();
        buf
    }
}

#[test]
fn test_print_int_line() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    for val in [0i32, 1, -1, 42, 100, -999, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"printIntLine").unwrap();
            f(val);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"printIntLine").unwrap();
            f(val);
        });
        assert_eq!(c_out, r_out, "printIntLine mismatch for {val}");
    }
}

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    let cases: Vec<*const c_char> = vec![
        CString::new("hello").unwrap().into_raw(),
        CString::new("").unwrap().into_raw(),
        std::ptr::null(),
    ];

    for &ptr in &cases {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
            f(ptr);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
            f(ptr);
        });
        assert_eq!(c_out, r_out, "printLine mismatch for ptr {:?}", ptr);
    }

    // Free the CStrings we leaked
    for &ptr in &cases {
        if !ptr.is_null() {
            unsafe { drop(CString::from_raw(ptr as *mut c_char)); }
        }
    }
}

#[test]
fn test_bad() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    for val in [1.0f32, 2.0, 0.5, -3.0, 100.0, 0.001] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_float)> = c_lib.get(b"bad").unwrap();
            f(val);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_float)> = r_lib.get(b"bad").unwrap();
            f(val);
        });
        assert_eq!(c_out, r_out, "bad() mismatch for {val}");
    }
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    for val in [1.0f32, 2.0, 0.0, -5.0, 0.0000001, 0.000002] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_float)> = c_lib.get(b"good").unwrap();
            f(val);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_float)> = r_lib.get(b"good").unwrap();
            f(val);
        });
        assert_eq!(c_out, r_out, "good() mismatch for {val}");
    }
}

#[test]
fn test_driver() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(RUST_LIB).unwrap() };

    let cases: Vec<(f32, f32)> = vec![
        (1.0, 2.0),
        (0.0, 1.0),
        (5.0, -3.0),
    ];

    for (good_data, bad_data) in cases {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_float, c_float)> = c_lib.get(b"driver").unwrap();
            f(good_data, bad_data);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_float, c_float)> = r_lib.get(b"driver").unwrap();
            f(good_data, bad_data);
        });
        assert_eq!(c_out, r_out, "driver() mismatch for ({good_data}, {bad_data})");
    }
}
