//! ERRORS.md row **E2** — `EPIPE` while `SIGPIPE` is ignored, at the FFI level.
//!
//! This is the counterpart to row E1. E1 covers the *executable*, which runs with
//! `SIGPIPE` at its default disposition and is therefore killed by the signal. A
//! shared library, by contrast, inherits whatever disposition its host process
//! has — and a Rust host (like this test binary) ignores `SIGPIPE`. In that
//! situation the failing write must surface as a discarded `EPIPE` and `main`
//! must still return 0, because the C code throws `printf`'s return value away.
//!
//! This test lives in its own test binary on purpose: a failed write latches
//! sticky state in each runtime's buffered stdout (glibc sets the `FILE` error
//! indicator; Rust's `LineWriter` may retain the unwritten bytes), which would
//! contaminate any other test sharing the process.

mod common;

use common::*;

#[test]
fn e2_epipe_ignored_returns_zero() {
    // Make the precondition explicit rather than relying on the Rust runtime's
    // default: SIGPIPE must be ignored, so the write fails instead of killing us.
    let previous = unsafe { signal(SIGPIPE, SIG_IGN) };

    let ret = assert_same_so("fd1=pipe with no reader, SIGPIPE ignored", |f| {
        let (rfd, wfd) = make_pipe();
        // Destroy the read end first: from here on, any write must fail EPIPE.
        unsafe { close(rfd) };
        let ret = with_fd1(wfd, || unsafe { f() });
        unsafe { close(wfd) };
        ret
    });

    assert_eq!(
        ret, 0,
        "printf's error is discarded by the C code, so main must still return 0"
    );

    unsafe { signal(SIGPIPE, previous) };
}
