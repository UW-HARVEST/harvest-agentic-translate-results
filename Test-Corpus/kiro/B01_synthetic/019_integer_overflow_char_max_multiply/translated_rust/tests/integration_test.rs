use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libdriver.so", manifest)
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    use std::io::{Read, Write};
    use std::os::unix::io::FromRawFd;

    // flush both C and Rust stdout before redirect
    std::io::stdout().flush().ok();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);

    let old_stdout = unsafe { libc::dup(1) };
    assert!(old_stdout >= 0);
    unsafe { libc::dup2(fds[1], 1) };
    unsafe { libc::close(fds[1]) };

    f();

    // flush both C and Rust stdout after call
    std::io::stdout().flush().ok();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    // restore stdout
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    // set pipe read end to non-blocking to avoid hanging
    unsafe {
        let flags = libc::fcntl(fds[0], libc::F_GETFL);
        libc::fcntl(fds[0], libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    let mut pipe_read = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let mut buf = String::new();
    let _ = pipe_read.read_to_string(&mut buf);
    buf
}

#[test]
fn test_print_hex_char_line() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(c_char)> =
            lib.get(b"printHexCharLine").unwrap();

        for val in &[0i8, 1, 2, 4, 127, -1, -2, -128] {
            let c_out = capture_stdout(|| { c_fn(*val); });
            let rust_out = capture_stdout(|| { driver::printHexCharLine(*val); });
            assert_eq!(c_out, rust_out, "printHexCharLine mismatch for {}", val);
        }
    }
}

#[test]
fn test_print_line() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine").unwrap();

        let test_str = CString::new("hello world").unwrap();
        let c_out = capture_stdout(|| { c_fn(test_str.as_ptr()); });
        let rust_out = capture_stdout(|| { driver::printLine(test_str.as_ptr()); });
        assert_eq!(c_out, rust_out, "printLine mismatch");

        let c_out_null = capture_stdout(|| { c_fn(std::ptr::null()); });
        let rust_out_null = capture_stdout(|| { driver::printLine(std::ptr::null()); });
        assert_eq!(c_out_null, rust_out_null, "printLine NULL mismatch");
    }
}

#[test]
fn test_bad() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn()> = lib.get(b"bad").unwrap();

        let c_out = capture_stdout(|| { c_fn(); });
        let rust_out = capture_stdout(|| { driver::bad(); });
        assert_eq!(c_out, rust_out, "bad() mismatch: C={:?} Rust={:?}", c_out, rust_out);
    }
}

#[test]
fn test_good() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn()> = lib.get(b"good").unwrap();

        let c_out = capture_stdout(|| { c_fn(); });
        let rust_out = capture_stdout(|| { driver::good(); });
        assert_eq!(c_out, rust_out, "good() mismatch: C={:?} Rust={:?}", c_out, rust_out);
    }
}
