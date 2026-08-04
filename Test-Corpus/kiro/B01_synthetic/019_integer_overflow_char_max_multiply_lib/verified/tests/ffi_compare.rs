use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by `f()` by dup'ing fd 1 into a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush any buffered C stdout
        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        assert!(saved >= 0);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let mut buf = Vec::new();
        let mut reader = std::fs::File::from_raw_fd(pipe_fds[0]);
        reader.read_to_end(&mut buf).unwrap();
        buf
    }
}

fn c_lib() -> Library {
    unsafe {
        Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("c_src/build/libdriver.so"),
        )
        .expect("load C .so")
    }
}

fn rust_lib() -> Library {
    // cargo puts cdylib in target/<profile>/deps or target/<profile>/
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    let path = dir.join("libdriver.so");
    unsafe { Library::new(&path).expect("load Rust .so") }
}

// ---- helpers to call each symbol via libloading ----

fn call_print_hex_char_line(lib: &Library, val: c_char) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_char)> =
            lib.get(b"printHexCharLine").unwrap();
        f(val);
    })
}

fn call_print_line(lib: &Library, s: &CString) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine").unwrap();
        f(s.as_ptr());
    })
}

fn call_print_line_null(lib: &Library) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    })
}

fn call_bad(lib: &Library) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = lib.get(b"bad").unwrap();
        f();
    })
}

fn call_good(lib: &Library) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = lib.get(b"good").unwrap();
        f();
    })
}

fn call_driver(lib: &Library, use_good: c_int) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"driver").unwrap();
        f(use_good);
    })
}

// ---- tests (lowest-level first) ----

#[test]
fn test_print_hex_char_line() {
    let c = c_lib();
    let r = rust_lib();
    for val in [0i8, 1, 2, 0x7f, -1, -128, 0x41, 0x20] {
        let cv = call_print_hex_char_line(&c, val as c_char);
        let rv = call_print_hex_char_line(&r, val as c_char);
        assert_eq!(cv, rv, "printHexCharLine mismatch for input {val}");
    }
}

#[test]
fn test_print_line() {
    let c = c_lib();
    let r = rust_lib();
    let cases = ["hello", "", "data value is too large to perform arithmetic safely."];
    for s in &cases {
        let cs = CString::new(*s).unwrap();
        let cv = call_print_line(&c, &cs);
        let rv = call_print_line(&r, &cs);
        assert_eq!(cv, rv, "printLine mismatch for \"{s}\"");
    }
    // null case
    assert_eq!(call_print_line_null(&c), call_print_line_null(&r), "printLine(NULL) mismatch");
}

#[test]
fn test_bad() {
    assert_eq!(call_bad(&c_lib()), call_bad(&rust_lib()), "bad() mismatch");
}

#[test]
fn test_good() {
    assert_eq!(call_good(&c_lib()), call_good(&rust_lib()), "good() mismatch");
}

#[test]
fn test_driver() {
    let c = c_lib();
    let r = rust_lib();
    for val in [0, 1, -1, 42] {
        let cv = call_driver(&c, val);
        let rv = call_driver(&r, val);
        assert_eq!(cv, rv, "driver({val}) mismatch");
    }
}
