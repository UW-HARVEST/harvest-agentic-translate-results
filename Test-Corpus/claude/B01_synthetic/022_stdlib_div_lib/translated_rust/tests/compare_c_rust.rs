// Integration test that loads BOTH the C-built shared library and the Rust-built
// shared library at runtime via libloading and compares their stdout for the
// `driver(int, int)` function across many input combinations.

use libloading::{Library, Symbol};
use std::ffi::{c_int, CString};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::PathBuf;

type DriverFn = unsafe extern "C" fn(c_int, c_int);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // The Rust cdylib is built into target/<profile>/libdriver.so.
    // For integration tests, $CARGO_MANIFEST_DIR/target/debug is used.
    // We try debug first, then release, to be robust.
    let base = workspace_root().join("target");
    let debug = base.join("debug").join("libdriver.so");
    if debug.exists() {
        return debug;
    }
    base.join("release").join("libdriver.so")
}

/// Capture everything written to stdout (file descriptor 1) by the closure
/// and return the captured bytes. This works for both Rust and C/libc writes
/// because we redirect at the OS file-descriptor level.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Make sure all buffered data is flushed before we redirect.
    let _ = std::io::stdout().flush();
    unsafe {
        // Flush all open output streams (passing NULL fflushes all).
        libc::fflush(std::ptr::null_mut());
    }

    // Save the current stdout fd.
    let saved_stdout = unsafe { libc::dup(1) };
    assert!(saved_stdout >= 0, "failed to dup stdout");

    // Open a temp file to receive stdout output.
    let tmp = tempfile_rw();
    let tmp_fd = tmp.as_raw_fd();

    // Replace fd 1 with the temp file.
    let rc = unsafe { libc::dup2(tmp_fd, 1) };
    assert_eq!(rc, 1, "failed to dup2 onto stdout");

    f();

    // Flush again before restoring.
    let _ = std::io::stdout().flush();
    unsafe {
        // Flush all open output streams (passing NULL fflushes all).
        libc::fflush(std::ptr::null_mut());
    }

    // Restore the original stdout fd.
    let rc = unsafe { libc::dup2(saved_stdout, 1) };
    assert_eq!(rc, 1, "failed to restore stdout");
    unsafe { libc::close(saved_stdout) };

    // Read the captured bytes from the temp file.
    let mut tmp = tmp;
    tmp.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).unwrap();
    buf
}

/// Create an unnamed read/write temp file using libc's tmpfile-style approach
/// via `memfd_create` if available, falling back to a regular tmpfile.
fn tempfile_rw() -> std::fs::File {
    // Try memfd_create first (Linux-specific).
    let name = CString::new("driver_capture").unwrap();
    // 0 = no special flags; MFD_CLOEXEC = 1 if we want close-on-exec, but we
    // are not exec'ing, so 0 is fine.
    let fd = unsafe { libc::syscall(libc::SYS_memfd_create, name.as_ptr(), 0u32) };
    if fd >= 0 {
        return unsafe { std::fs::File::from_raw_fd(fd as i32) };
    }
    // Fallback: create a regular temp file.
    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("failed to open temp file");
    let _ = std::fs::remove_file(&path); // unlink while open
    f
}

fn run_driver(lib_path: &std::path::Path, x: c_int, y: c_int) -> Vec<u8> {
    let lib = unsafe { Library::new(lib_path) }
        .unwrap_or_else(|e| panic!("failed to load {:?}: {}", lib_path, e));
    let driver: Symbol<DriverFn> =
        unsafe { lib.get(b"driver\0") }.expect("driver symbol not found");
    let out = capture_stdout(|| unsafe { driver(x, y) });
    // Drop the Symbol before the Library is dropped.
    drop(driver);
    drop(lib);
    out
}

#[test]
fn driver_outputs_match_for_basic_cases() {
    let cases: &[(c_int, c_int)] = &[
        (10, 3),
        (0, 1),
        (1, 1),
        (-10, 3),
        (10, -3),
        (-10, -3),
        (7, 7),
        (1, -1),
        (-1, 1),
        (i32::MAX, 1),
        (i32::MIN + 1, 1), // avoid INT_MIN / -1 overflow
        (i32::MAX, 7),
        (i32::MIN, 2),
        (123456, 789),
        (-123456, 789),
        (123456, -789),
        (-123456, -789),
        (5, 2),
        (5, -2),
        (-5, 2),
        (-5, -2),
        (1000, 999),
        (1000, 1001),
    ];

    for &(x, y) in cases {
        let c_out = run_driver(&c_lib_path(), x, y);
        let r_out = run_driver(&rust_lib_path(), x, y);
        assert_eq!(
            c_out, r_out,
            "mismatch for driver({}, {}):\n  C    : {:?}\n  Rust : {:?}",
            x,
            y,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn driver_exhaustive_small_range() {
    // Exhaustively sweep small inputs (skip y == 0).
    for x in -25..=25 {
        for y in -10..=10 {
            if y == 0 {
                continue;
            }
            let c_out = run_driver(&c_lib_path(), x, y);
            let r_out = run_driver(&rust_lib_path(), x, y);
            assert_eq!(
                c_out, r_out,
                "mismatch for driver({}, {}):\n  C    : {:?}\n  Rust : {:?}",
                x,
                y,
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
        }
    }
}
