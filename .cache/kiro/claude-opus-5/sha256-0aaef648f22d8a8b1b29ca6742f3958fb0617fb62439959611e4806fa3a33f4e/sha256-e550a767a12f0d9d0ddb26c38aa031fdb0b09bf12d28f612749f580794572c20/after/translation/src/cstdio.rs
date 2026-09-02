//! Bindings to the pieces of C `<stdio.h>` that `driver.c` uses.
//!
//! `driver.c` relies on `sscanf("%d%zn", ...)` and `printf("%d\n", ...)`.
//! Re-implementing those conversions by hand in Rust would risk diverging from
//! the host C library on the corner cases that matter for byte-identical
//! output (leading-whitespace skipping, `+`/`-` handling, what counts as a
//! matching failure vs. an input failure, and the platform's out-of-range
//! behaviour for `%d`). Calling straight through to the same C library the
//! original was linked against guarantees identical results, and it also keeps
//! the process's `stdout` buffering behaviour (and therefore the exact byte
//! stream and flush points) the same as the C build.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// `int sscanf(const char *restrict s, const char *restrict format, ...)`
    pub fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;

    /// `int printf(const char *restrict format, ...)`
    pub fn printf(format: *const c_char, ...) -> c_int;
}
