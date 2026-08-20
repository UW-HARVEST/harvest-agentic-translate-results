//! Executable entry point, mirroring the C program built by
//! `c_src/CMakeLists.txt` (`add_executable(driver src/main.c)`).
//!
//! The implementation lives in `imp.rs`, which is also compiled into the
//! cdylib. It is included by path rather than through the library crate
//! because the library exports a C-ABI symbol named `main`, which would
//! collide with this binary's entry point at link time.

use std::os::raw::c_int;

#[path = "imp.rs"]
mod imp;

/// `SIGPIPE` on Linux.
const SIGPIPE: c_int = 13;
/// `SIG_DFL`; glibc's `sighandler_t` is pointer-sized.
const SIG_DFL: usize = 0;

extern "C" {
    fn signal(signum: c_int, handler: usize) -> usize;
}

fn main() {
    // A C program starts with the default disposition for SIGPIPE, so writing
    // to a pipe whose reader has gone away kills it with signal 13. The Rust
    // runtime instead installs SIG_IGN before `main`, which would turn that
    // into an ignored `EPIPE` and an exit status of 0. Restore the C startup
    // state so the translated program dies exactly like the original.
    //
    // This is deliberately *not* done in the cdylib: a shared library must not
    // change the host process's signal disposition, and the C shared library
    // does not do so either.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }

    // The C `main` returns 0; the process exit status is therefore always 0.
    let _ = imp::run_main();
}
