use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::io::Read;

extern "C" {
    fn fflush(stream: *mut libc::c_void) -> libc::c_int;
}

/// Capture stdout produced by `f()` by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::os::unix::io::FromRawFd;

    unsafe {
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        fflush(std::ptr::null_mut()); // flush all C streams

        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let read_fd = fds[0];
        let write_fd = fds[1];

        let old_stdout = libc::dup(1);
        assert!(old_stdout >= 0);
        libc::dup2(write_fd, 1);
        libc::close(write_fd);

        f();

        std::io::stdout().flush().unwrap();
        fflush(std::ptr::null_mut());

        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut pipe_read = std::fs::File::from_raw_fd(read_fd);
        let mut buf = String::new();
        pipe_read.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(&path).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so");
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

#[test]
fn test_print_line() {
    let c = c_lib();
    let r = rust_lib();

    let test_strings = ["hello", "bad()", "", "line with spaces", "special: !@#$%"];

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

        assert_eq!(c_out, r_out, "printLine mismatch for input {:?}", s);
    }

    // Test NULL
    let c_null = {
        let func: Symbol<unsafe extern "C" fn(*const c_char)> =
            unsafe { c.get(b"printLine").unwrap() };
        capture_stdout(|| unsafe { func(std::ptr::null()) })
    };
    let r_null = {
        let func: Symbol<unsafe extern "C" fn(*const c_char)> =
            unsafe { r.get(b"printLine").unwrap() };
        capture_stdout(|| unsafe { func(std::ptr::null()) })
    };
    assert_eq!(c_null, r_null, "printLine mismatch for NULL input");
}

#[test]
fn test_bad() {
    let c = c_lib();
    let r = rust_lib();

    let c_out = {
        let func: Symbol<unsafe extern "C" fn()> = unsafe { c.get(b"bad").unwrap() };
        capture_stdout(|| unsafe { func() })
    };
    let r_out = {
        let func: Symbol<unsafe extern "C" fn()> = unsafe { r.get(b"bad").unwrap() };
        capture_stdout(|| unsafe { func() })
    };
    assert_eq!(c_out, r_out, "bad() output mismatch");
}

#[test]
fn test_good() {
    let c = c_lib();
    let r = rust_lib();

    let c_out = {
        let func: Symbol<unsafe extern "C" fn()> = unsafe { c.get(b"good").unwrap() };
        capture_stdout(|| unsafe { func() })
    };
    let r_out = {
        let func: Symbol<unsafe extern "C" fn()> = unsafe { r.get(b"good").unwrap() };
        capture_stdout(|| unsafe { func() })
    };
    assert_eq!(c_out, r_out, "good() output mismatch");
}

#[test]
fn test_main() {
    let c = c_lib();
    let r = rust_lib();

    let c_out = {
        let func: Symbol<unsafe extern "C" fn(c_int, *const *const c_char) -> c_int> =
            unsafe { c.get(b"main").unwrap() };
        capture_stdout(|| {
            let ret = unsafe { func(0, std::ptr::null()) };
            assert_eq!(ret, 0, "C main returned non-zero");
        })
    };
    let r_out = {
        let func: Symbol<unsafe extern "C" fn(c_int, *const *const c_char) -> c_int> =
            unsafe { r.get(b"main").unwrap() };
        capture_stdout(|| {
            let ret = unsafe { func(0, std::ptr::null()) };
            assert_eq!(ret, 0, "Rust main returned non-zero");
        })
    };
    assert_eq!(c_out, r_out, "main() output mismatch");
}
