// Integration test that loads BOTH the C .so AND the Rust .so via libloading
// and compares the output of `sieve` byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::PathBuf;
use std::sync::Mutex;

// Tests run in parallel and all share stdout (fd 1). Capturing stdout via
// dup/dup2 is a process-global operation so all tests must serialize on this
// mutex to avoid stomping on each other.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

type SieveFn = unsafe extern "C" fn(c_int);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libSieve.so")
}

fn rust_lib_path() -> PathBuf {
    // Try debug first, then release
    let dbg = workspace_root().join("target/debug/libSieve.so");
    if dbg.exists() {
        return dbg;
    }
    workspace_root().join("target/release/libSieve.so")
}

/// Capture everything written to file descriptor 1 (stdout) while running `f`.
/// Uses dup/dup2 + a pipe so output emitted by `printf` from a dynamic library
/// is captured properly.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush any C-level buffered stdout first.
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup failed");

        let mut pipefds = [0 as c_int; 2];
        let rc = libc::pipe(pipefds.as_mut_ptr());
        assert_eq!(rc, 0, "pipe failed");

        let read_fd = pipefds[0];
        let write_fd = pipefds[1];

        let rc = libc::dup2(write_fd, 1);
        assert_eq!(rc, 1, "dup2 failed");
        libc::close(write_fd);

        // Run the user code.
        f();

        // Flush after the call so any buffered C output is in the pipe.
        libc::fflush(std::ptr::null_mut());

        // Restore stdout.
        let rc = libc::dup2(saved, 1);
        assert_eq!(rc, 1, "dup2 restore failed");
        libc::close(saved);

        // Read all data from the pipe.
        let mut file = File::from_raw_fd(read_fd);
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read pipe failed");
        // file dropped -> read end closed
        let _ = file.into_raw_fd(); // not needed; let drop close
        buf
    }
}

fn run_one(lib_path: &PathBuf, val: c_int) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).expect("load library");
        let sieve: Symbol<SieveFn> = lib.get(b"sieve").expect("symbol sieve");
        let _g = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let out = capture_stdout(|| sieve(val));
        // Library is dropped here, unloaded.
        drop(sieve);
        drop(lib);
        out
    }
}

fn assert_match(val: c_int) {
    let c_path = c_lib_path();
    let rust_path = rust_lib_path();
    assert!(c_path.exists(), "C .so not found at {:?}", c_path);
    assert!(rust_path.exists(), "Rust .so not found at {:?}", rust_path);

    let c_out = run_one(&c_path, val);
    let rust_out = run_one(&rust_path, val);

    assert_eq!(
        c_out, rust_out,
        "Mismatch for val={}:\nC:    {:?}\nRust: {:?}",
        val,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
}

#[test]
fn sieve_zero() {
    assert_match(0);
}

#[test]
fn sieve_one() {
    assert_match(1);
}

#[test]
fn sieve_already_nine() {
    assert_match(9);
}

#[test]
fn sieve_already_negative_one() {
    // -1 % 10 in C is -1, not 9, so it should keep counting
    assert_match(-1);
}

#[test]
fn sieve_negative_starts_at_minus_five() {
    assert_match(-5);
}

#[test]
fn sieve_starts_at_minus_nine() {
    // -9 % 10 == -9 in C, so it should NOT terminate immediately;
    // counts -9, -8, ..., until 9.
    assert_match(-9);
}

#[test]
fn sieve_starts_at_ten() {
    assert_match(10);
}

#[test]
fn sieve_starts_at_twenty_three() {
    assert_match(23);
}

#[test]
fn sieve_starts_at_one_hundred() {
    assert_match(100);
}

#[test]
fn sieve_starts_at_minus_thirteen() {
    assert_match(-13);
}

#[test]
fn sieve_large_value() {
    assert_match(99_999);
}
