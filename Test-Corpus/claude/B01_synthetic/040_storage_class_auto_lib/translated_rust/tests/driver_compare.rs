// Integration test that loads the C and Rust shared libraries via libloading
// and compares the byte-level stdout output of the `driver` function.

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_char;
use std::path::PathBuf;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    static stdout: *mut c_void;
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // Search both the immediate target/debug and target/release locations.
    let candidates = [
        project_root().join("target/debug/libdriver.so"),
        project_root().join("target/release/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

/// Capture everything written to file descriptor 1 (stdout) by the closure.
/// Flushes C stdio (`stdout` FILE*) and Rust's `io::stdout()` before
/// restoring the original fd, so that buffered output is included.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Write;

    // Flush before redirect so previously buffered data goes to original fd.
    unsafe {
        let _ = fflush(stdout);
    }
    let _ = std::io::stdout().lock().flush();

    let tmp_path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("create temp file");

    use std::os::unix::io::AsRawFd;
    let tmp_fd = file.as_raw_fd();

    let saved_fd = unsafe { dup(1) };
    assert!(saved_fd >= 0, "dup failed");

    let dup2_ret = unsafe { dup2(tmp_fd, 1) };
    assert!(dup2_ret >= 0, "dup2 failed");

    f();

    // Flush both C stdio and Rust stdout so all buffered content lands in the file.
    unsafe {
        let _ = fflush(stdout);
    }
    let _ = std::io::stdout().lock().flush();

    // Restore original stdout fd.
    let _ = unsafe { dup2(saved_fd, 1) };
    let _ = unsafe { close(saved_fd) };

    // Read the captured contents.
    file.seek(SeekFrom::Start(0)).expect("seek temp file");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read temp file");
    drop(file);
    let _ = fs::remove_file(&tmp_path);
    let _ = (tmp_fd, c_char::default());
    buf
}

unsafe fn call_driver(lib: &Library, x: c_int) -> Vec<u8> {
    let sym: Symbol<unsafe extern "C" fn(c_int)> =
        lib.get(b"driver\0").expect("locate driver symbol");
    capture_stdout(|| sym(x))
}

fn run_compare(x: c_int) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_out = call_driver(&c_lib, x);
        let r_out = call_driver(&rust_lib, x);

        assert_eq!(
            c_out,
            r_out,
            "driver({x}) mismatch:\n  C={c_out:?}\n  Rust={r_out:?}",
            x = x,
            c_out = String::from_utf8_lossy(&c_out),
            r_out = String::from_utf8_lossy(&r_out),
        );
    }
}

#[test]
fn driver_basic_zero() {
    run_compare(0);
}

#[test]
fn driver_basic_positive() {
    run_compare(1);
    run_compare(7);
    run_compare(42);
    run_compare(1000);
}

#[test]
fn driver_basic_negative() {
    run_compare(-1);
    run_compare(-150);
    run_compare(-300);
    run_compare(-1000);
}

#[test]
fn driver_overflow_edges() {
    // 2*x can overflow; we want byte-identical output even when wrapping.
    run_compare(i32::MAX);
    run_compare(i32::MIN);
    run_compare(i32::MAX - 1);
    run_compare(i32::MIN + 1);
    run_compare(i32::MAX / 2);
    run_compare(i32::MIN / 2);
}

#[test]
fn driver_assorted_values() {
    for x in [-12345, -100, -1, 0, 1, 100, 12345, 99999, -99999] {
        run_compare(x);
    }
}

#[test]
fn driver_symbol_export_present() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let _c: Symbol<unsafe extern "C" fn(c_int)> =
            c_lib.get(b"driver\0").expect("C lib must export driver");
        let _r: Symbol<unsafe extern "C" fn(c_int)> = rust_lib
            .get(b"driver\0")
            .expect("Rust lib must export driver");
    }
}
