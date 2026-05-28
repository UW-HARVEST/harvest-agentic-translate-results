// Integration test that compares the C-built shared library against the
// Rust-built shared library through the FFI boundary, exactly as an
// external caller would invoke them.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::IntoRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize all tests since we redirect the global stdout fd.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libdriver.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // tests run from CARGO_MANIFEST_DIR; the cdylib lives in target/<profile>
    // We need to make sure it's been built.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try debug first (where cargo test puts things by default), fall back to release.
    let debug = p.join("debug").join("libdriver.so");
    if debug.exists() {
        return debug;
    }
    p.push("release");
    p.push("libdriver.so");
    p
}

/// Capture stdout produced by `f` (which calls into a shared library).
/// Returns the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush any pending output on the C/Rust libc stdout streams.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    // Save current stdout fd.
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "failed to dup stdout");

    // Create a temp file and redirect fd 1 to it.
    let tmp_path = std::env::temp_dir().join(format!(
        "ffi_compare_{}_{}.out",
        std::process::id(),
        rand_u64(),
    ));
    let tmp_file = File::create(&tmp_path).expect("create temp");
    let tmp_fd = tmp_file.into_raw_fd();
    let r = unsafe { dup2(tmp_fd, 1) };
    assert!(r >= 0, "failed to dup2");
    unsafe { close(tmp_fd) };

    // Call the function under test.
    f();

    // Flush stdout from the C runtime to make sure the bytes hit our file.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    // Restore original stdout.
    let r = unsafe { dup2(saved_stdout, 1) };
    assert!(r >= 0, "failed to restore stdout");
    unsafe { close(saved_stdout) };

    // Read the captured bytes back.
    let mut out = Vec::new();
    let mut f = File::open(&tmp_path).expect("open temp");
    f.read_to_end(&mut out).expect("read temp");
    let _ = std::fs::remove_file(&tmp_path);
    out
}

fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (std::process::id() as u64).wrapping_mul(0x9E3779B97F4A7C15)
}

unsafe fn call_print_line(lib: &Library, s: Option<&CString>) -> Vec<u8> {
    let sym: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { lib.get(b"printLine\0") }.expect("printLine missing");
    let p = match s {
        Some(c) => c.as_ptr(),
        None => std::ptr::null(),
    };
    capture_stdout(|| unsafe { sym(p) })
}

unsafe fn call_driver(lib: &Library, data: c_int) -> Vec<u8> {
    let sym: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { lib.get(b"driver\0") }.expect("driver missing");
    capture_stdout(|| unsafe { sym(data) })
}

#[test]
fn print_line_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    // Null pointer: should print nothing.
    let c_out = unsafe { call_print_line(&c_lib, None) };
    let r_out = unsafe { call_print_line(&r_lib, None) };
    assert_eq!(c_out, r_out, "null printLine mismatch");
    assert!(c_out.is_empty(), "expected null printLine to output nothing");

    // Empty string: should print just "\n".
    let s = CString::new("").unwrap();
    let c_out = unsafe { call_print_line(&c_lib, Some(&s)) };
    let r_out = unsafe { call_print_line(&r_lib, Some(&s)) };
    assert_eq!(c_out, r_out, "empty printLine mismatch");
    assert_eq!(c_out, b"\n");

    // Some content.
    let s = CString::new("Hello, world!").unwrap();
    let c_out = unsafe { call_print_line(&c_lib, Some(&s)) };
    let r_out = unsafe { call_print_line(&r_lib, Some(&s)) };
    assert_eq!(c_out, r_out, "hello printLine mismatch");
    assert_eq!(c_out, b"Hello, world!\n");

    // String with embedded special characters.
    let s = CString::new("line\twith\tspecial\rchars and %s and %%").unwrap();
    let c_out = unsafe { call_print_line(&c_lib, Some(&s)) };
    let r_out = unsafe { call_print_line(&r_lib, Some(&s)) };
    assert_eq!(c_out, r_out, "special chars printLine mismatch");

    // Long string.
    let big = "A".repeat(2000);
    let s = CString::new(big).unwrap();
    let c_out = unsafe { call_print_line(&c_lib, Some(&s)) };
    let r_out = unsafe { call_print_line(&r_lib, Some(&s)) };
    assert_eq!(c_out, r_out, "long printLine mismatch");
}

#[test]
fn driver_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    // Test a range of valid `data` values that exercise strncpy + printLine.
    // Per the C source, valid values are 0..=99 (inclusive of 0).
    // Negative values and values >= 100 trigger different code paths:
    //   - data >= 100: skip strncpy entirely; dest stays "" (empty).
    //   - data < 0:    cast to size_t produces huge value -> UB territory; skip.
    //   - 0 <= data <= 99: strncpy(dest, source, data); dest[data] = '\0';
    let values: Vec<c_int> = vec![0, 1, 2, 5, 10, 50, 98, 99, 100, 101, 200, i32::MAX];
    for v in values {
        let c_out = unsafe { call_driver(&c_lib, v) };
        let r_out = unsafe { call_driver(&r_lib, v) };
        assert_eq!(
            c_out, r_out,
            "driver({}) mismatch: c={:?} rust={:?}",
            v,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}
