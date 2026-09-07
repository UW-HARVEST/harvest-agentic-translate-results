//! Phase D — proves both libraries write through the *same* `FILE` stream.
//!
//! Byte-comparing `stderr` is not by itself enough to show the Rust port uses
//! glibc's `stderr` `FILE`: a port that called `write(2)` directly, or that used
//! Rust's own `std::io::stderr()`, would emit the same bytes while an
//! observer's `FILE`-level buffering behaved differently.
//!
//! This test switches `stderr` to **fully buffered** mode and checks that
//! neither library's output reaches the file descriptor until `fflush` — i.e.
//! both go through the C stream. It lives in its own integration-test binary
//! (hence its own process) because `setvbuf` must run before any I/O on the
//! stream and its effect is process-global.

mod common;

use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::os::fd::AsRawFd;

use common::{c_pow, rust_pow, PowFn};

unsafe extern "C" {
    static mut stderr: *mut c_void;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn __errno_location() -> *mut c_int;
}

const IOFBF: c_int = 0; // _IOFBF — fully buffered

/// Calls `f` with fd 2 pointing at a fresh file and reports what had reached
/// that file *before* `fflush` and what was there *after*.
fn observe_buffering(f: PowFn, base: f64, exponent: f64, tag: &str) -> (Vec<u8>, Vec<u8>) {
    let path = std::env::temp_dir().join(format!("pow_buf_{}_{tag}.err", std::process::id()));
    let file = File::create(&path).expect("create temp file");

    let (before, after) = unsafe {
        let saved = dup(2);
        assert!(saved >= 0);
        assert!(dup2(file.as_raw_fd(), 2) >= 0);

        *__errno_location() = 0;
        let v = f(base, exponent);
        assert_eq!(v, -1.0, "[{tag}] expected the error sentinel, got {v:?}");

        let before = std::fs::read(&path).expect("read before flush");
        fflush(stderr);
        let after = std::fs::read(&path).expect("read after flush");

        assert!(dup2(saved, 2) >= 0);
        close(saved);
        (before, after)
    };

    drop(file);
    let _ = std::fs::remove_file(&path);
    (before, after)
}

#[test]
fn d9_both_libraries_write_through_glibc_stderr_stream() {
    // Must happen before anything writes to stderr in this process.
    let rc = unsafe { setvbuf(stderr, std::ptr::null_mut(), IOFBF, 4096) };
    assert_eq!(rc, 0, "setvbuf(stderr, _IOFBF) failed");

    let (c_before, c_after) = observe_buffering(c_pow(), -2.0, 0.5, "c");
    let (r_before, r_after) = observe_buffering(rust_pow(), -2.0, 0.5, "rust");

    // The interesting part: with a fully buffered stream the diagnostic must
    // still be sitting in the FILE buffer, not yet at the fd.
    assert!(
        c_before.is_empty(),
        "sanity check failed: the C library's output reached fd 2 before fflush \
         even though stderr is fully buffered ({:?})",
        String::from_utf8_lossy(&c_before)
    );
    assert_eq!(
        c_before,
        r_before,
        "buffering behaviour differs: with a fully buffered stderr the C library had \
         {} byte(s) at the fd before fflush but Rust had {} ({:?}) — the Rust port is not \
         writing through glibc's `stderr` FILE",
        c_before.len(),
        r_before.len(),
        String::from_utf8_lossy(&r_before),
    );

    assert_eq!(
        String::from_utf8_lossy(&c_after),
        String::from_utf8_lossy(&r_after),
        "flushed stderr contents differ"
    );
    assert_eq!(c_after, r_after, "flushed stderr bytes differ");
    assert!(
        !c_after.is_empty(),
        "nothing was written at all — the test is not exercising the fprintf"
    );
}
