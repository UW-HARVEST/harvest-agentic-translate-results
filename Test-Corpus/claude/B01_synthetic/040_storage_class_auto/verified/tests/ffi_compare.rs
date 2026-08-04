// Integration tests that load BOTH the C-built .so and the Rust-built .so
// via libloading, invoke their exported functions through the FFI boundary,
// and assert that the outputs match byte-for-byte.
//
// We never call the Rust functions directly; everything goes through dlopen
// just like an external caller would, so the `#[no_mangle]` export wrappers
// are exercised.
//
// The whole comparison flow lives in a single test function because the
// libtest harness runs `#[test]` functions on multiple threads and we have
// to monopolise FD 1 (process-global stdout) while capturing output.

use libloading::{Library, Symbol};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut core::ffi::c_void) -> i32;
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libdriver.so");
    p
}

fn rust_so_path() -> PathBuf {
    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("target");
        p.to_string_lossy().into_owned()
    });
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let mut p = PathBuf::from(target_dir);
    p.push(profile);
    p.push("libdriver.so");
    p
}

/// Run a closure while FD 1 is redirected to a temp file; returns the bytes
/// captured. The original FD 1 is restored on exit.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let tmp_path = std::env::temp_dir().join(format!(
        "ffi_capture_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let mut tmp = File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&tmp_path)
        .expect("create temp file");

    // Flush Rust stdout before the swap so previously-buffered bytes go to the
    // real stdout, not into our capture file.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(core::ptr::null_mut());
    }

    let saved_fd = unsafe { dup(1) };
    assert!(saved_fd >= 0, "dup(1) failed");
    let new_fd = tmp.as_raw_fd();
    let rc = unsafe { dup2(new_fd, 1) };
    assert!(rc >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Flush libc stdout (printf is line-buffered when isatty, otherwise
    // block-buffered; either way we want all bytes flushed before restoring
    // FD 1) and Rust stdout.
    unsafe {
        fflush(core::ptr::null_mut());
    }
    let _ = std::io::stdout().flush();

    let rc = unsafe { dup2(saved_fd, 1) };
    assert!(rc >= 0, "dup2 restore failed");
    unsafe {
        close(saved_fd);
    }

    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).expect("read");
    drop(tmp);
    let _ = std::fs::remove_file(&tmp_path);

    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }

    buf
}

unsafe fn load_driver(lib: &Library) -> Symbol<'_, unsafe extern "C" fn(i32)> {
    lib.get(b"driver\0").expect("driver symbol")
}

fn run_driver_via_lib(path: &std::path::Path, x: i32) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let lib = Library::new(path).expect("load .so");
        let driver = load_driver(&lib);
        driver(x);
        fflush(core::ptr::null_mut());
        drop(driver);
    })
}

fn assert_driver_match(x: i32) {
    let c_out = run_driver_via_lib(&c_so_path(), x);
    let rust_out = run_driver_via_lib(&rust_so_path(), x);
    assert_eq!(
        c_out, rust_out,
        "driver({}) mismatch\nC:    {:?}\nRust: {:?}",
        x,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
    );
}

/// Drive `driver(x)` for many values of x and confirm both implementations
/// produce identical bytes on stdout. All cases live in one test so the
/// libtest harness can't run them on parallel threads (which would race on
/// FD 1, our stdout capture target).
#[test]
fn driver_outputs_match_across_inputs() {
    // First confirm both libraries load and expose the public API symbols.
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let _: Symbol<unsafe extern "C" fn(i32)> =
            c_lib.get(b"driver\0").expect("C exports driver");
        let _: Symbol<unsafe extern "C" fn() -> i32> =
            c_lib.get(b"main\0").expect("C exports main");

        let rust_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let _: Symbol<unsafe extern "C" fn(i32)> =
            rust_lib.get(b"driver\0").expect("Rust exports driver");
        let _: Symbol<unsafe extern "C" fn() -> i32> =
            rust_lib.get(b"main\0").expect("Rust exports main");
    }

    // Zero, small positives, small negatives.
    for x in [
        0, 1, 2, 5, 10, 42, 100, 999, 1_000, 1_000_000,
        -1, -2, -10, -42, -150, -1_000, -1_000_000,
    ] {
        assert_driver_match(x);
    }

    // Boundary values: 2*x + 300 wraps at i32::MAX/i32::MIN. Both
    // implementations must wrap identically.
    for x in [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        1_000_000_000,
        -1_000_000_000,
        1_073_741_823,
        -1_073_741_824,
        1_073_741_824,  // Triggers signed overflow when doubled.
        -1_073_741_825i64 as i32,
    ] {
        assert_driver_match(x);
    }
}
