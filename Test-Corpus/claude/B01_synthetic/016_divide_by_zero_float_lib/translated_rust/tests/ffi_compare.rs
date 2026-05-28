// Integration test that compares the C-built libdriver.so against the
// Rust-built libdriver.so by loading both via libloading and capturing
// stdout written during each FFI call.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::{c_char, c_float, c_int};
use std::path::PathBuf;

// libc bindings we need for redirecting stdout reliably.
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

fn c_so_path() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // The integration test runs against the cdylib produced by `cargo test`/build.
    // We try both debug and release locations.
    let manifest = env!("CARGO_MANIFEST_DIR");
    let debug = PathBuf::from(manifest).join("target/debug/libdriver.so");
    let release = PathBuf::from(manifest).join("target/release/libdriver.so");
    if debug.exists() {
        debug
    } else {
        release
    }
}

/// Capture everything written to fd 1 (stdout) while `f` runs.
/// Flushes both libc stdio buffers and Rust's stdout to make sure
/// nothing is left in user-space buffers before we restore fd 1.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Write;
    // Make sure anything already buffered (in either runtime) has been
    // flushed to the *original* stdout, so we don't accidentally capture it.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    // Create a temp file we'll redirect into.
    let tmp_path = std::env::temp_dir().join(format!(
        "ffi_capture_{}_{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let tmp_file = File::create(&tmp_path).expect("create tmp");
    drop(tmp_file);
    let mut tmp = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&tmp_path)
        .expect("open tmp");

    // Save current fd 1, redirect fd 1 to tmp file.
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup failed");
    let tmp_fd = {
        use std::os::unix::io::AsRawFd;
        tmp.as_raw_fd()
    };
    let r = unsafe { dup2(tmp_fd, 1) };
    assert!(r >= 0, "dup2 failed");

    // Run the closure.
    f();

    // Flush before restoring.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    // Restore stdout.
    let r = unsafe { dup2(saved, 1) };
    assert!(r >= 0, "dup2 restore failed");
    unsafe {
        close(saved);
    }

    // Read what was captured.
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read tmp");

    let _ = std::fs::remove_file(&tmp_path);
    out
}

fn load_c() -> Library {
    unsafe { Library::new(c_so_path()).expect("load C .so") }
}
fn load_rust() -> Library {
    unsafe { Library::new(rust_so_path()).expect("load Rust .so") }
}

// ------------------- printIntLine -------------------

fn call_print_int_line(lib: &Library, n: c_int) {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = lib
            .get(b"printIntLine\0")
            .expect("symbol printIntLine");
        f(n);
    }
}

#[test]
fn test_print_int_line() {
    let c = load_c();
    let r = load_rust();
    for &n in &[0i32, 1, -1, 50, 100, -100, i32::MIN, i32::MAX] {
        let c_out = capture_stdout(|| call_print_int_line(&c, n));
        let r_out = capture_stdout(|| call_print_int_line(&r, n));
        assert_eq!(c_out, r_out, "printIntLine mismatch for {}", n);
    }
}

// ------------------- printLine -------------------

fn call_print_line(lib: &Library, s: *const c_char) {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine\0").expect("symbol printLine");
        f(s);
    }
}

#[test]
fn test_print_line_strings() {
    let c = load_c();
    let r = load_rust();
    let cases = [
        "",
        "hello",
        "Calling good()...",
        "Finished good()",
        "This would result in a divide by zero",
        "line with spaces and \t tab",
    ];
    for s in cases.iter() {
        let cs = CString::new(*s).unwrap();
        let c_out = capture_stdout(|| call_print_line(&c, cs.as_ptr()));
        let r_out = capture_stdout(|| call_print_line(&r, cs.as_ptr()));
        assert_eq!(c_out, r_out, "printLine mismatch for {:?}", s);
    }
}

#[test]
fn test_print_line_null() {
    let c = load_c();
    let r = load_rust();
    let c_out = capture_stdout(|| call_print_line(&c, std::ptr::null()));
    let r_out = capture_stdout(|| call_print_line(&r, std::ptr::null()));
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

// ------------------- bad -------------------

fn call_bad(lib: &Library, data: c_float) {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_float)> = lib.get(b"bad\0").expect("bad");
        f(data);
    }
}

#[test]
fn test_bad() {
    let c = load_c();
    let r = load_rust();
    let cases: &[f32] = &[
        2.0, 4.0, 5.0, 10.0, 100.0, 0.5, 1.0, -1.0, -2.0, 50.0, 25.0, 0.001,
        1.234e-3, 7.0, 33.0,
    ];
    for &d in cases {
        let c_out = capture_stdout(|| call_bad(&c, d));
        let r_out = capture_stdout(|| call_bad(&r, d));
        assert_eq!(c_out, r_out, "bad({}) mismatch", d);
    }
}

// ------------------- good -------------------

fn call_good(lib: &Library, data: c_float) {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_float)> = lib.get(b"good\0").expect("good");
        f(data);
    }
}

#[test]
fn test_good() {
    let c = load_c();
    let r = load_rust();
    let cases: &[f32] = &[
        2.0, 4.0, 5.0, 10.0, 100.0, 0.5, 1.0, -1.0, -2.0, 50.0, 25.0,
        0.0, 0.0000001, -0.0000001, 0.000001, -0.000001,
        // boundary values around fabs() > 0.000001
        0.0000011, -0.0000011, 0.0000009, -0.0000009,
    ];
    for &d in cases {
        let c_out = capture_stdout(|| call_good(&c, d));
        let r_out = capture_stdout(|| call_good(&r, d));
        assert_eq!(c_out, r_out, "good({}) mismatch", d);
    }
}

// ------------------- driver -------------------

fn call_driver(lib: &Library, g: c_float, b: c_float) {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_float, c_float)> =
            lib.get(b"driver\0").expect("driver");
        f(g, b);
    }
}

#[test]
fn test_driver() {
    let c = load_c();
    let r = load_rust();
    let cases: &[(f32, f32)] = &[
        (2.0, 5.0),
        (1.0, 2.0),
        (5.0, 100.0),
        (0.5, 4.0),
        (-2.0, -5.0),
        (0.0000001, 1.0),
        (10.0, 0.001),
    ];
    for &(g, b) in cases {
        let c_out = capture_stdout(|| call_driver(&c, g, b));
        let r_out = capture_stdout(|| call_driver(&r, g, b));
        assert_eq!(c_out, r_out, "driver({}, {}) mismatch", g, b);
    }
}
