use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    // Find the Rust cdylib in target/debug/
    let dir = format!("{}/target/debug", env!("CARGO_MANIFEST_DIR"));
    for entry in std::fs::read_dir(&dir).unwrap() {
        let p = entry.unwrap().path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("libdriver") && name.ends_with(".so") && !name.contains(".d") {
                return p.to_string_lossy().into_owned();
            }
        }
    }
    panic!("Rust .so not found in {}", dir);
}

/// Capture stdout produced by calling `f` using pipe + dup2.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush all
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let mut buf = String::new();
        let mut file = std::fs::File::from_raw_fd(fds[0]);
        file.read_to_string(&mut buf).unwrap();
        buf
    }
}

use std::os::unix::io::FromRawFd;

// ---- Tests: lowest-level first ----

#[test]
fn test_print_int_line() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    for val in [0i32, 1, -1, i32::MAX, i32::MIN, 42] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"printIntLine").unwrap();
            f(val);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"printIntLine").unwrap();
            f(val);
        });
        assert_eq!(c_out, r_out, "printIntLine mismatch for {}", val);
    }
}

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // Non-null string
    let s = CString::new("hello world").unwrap();
    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
        f(s.as_ptr());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
        f(s.as_ptr());
    });
    assert_eq!(c_out, r_out, "printLine mismatch for non-null");

    // Null pointer — should produce no output
    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    assert_eq!(c_out, r_out, "printLine mismatch for null");
}

#[test]
fn test_bad() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"bad").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"bad").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "bad() output mismatch");
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() output mismatch");
}

#[test]
fn test_driver() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"driver").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"driver").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "driver() output mismatch");
}
