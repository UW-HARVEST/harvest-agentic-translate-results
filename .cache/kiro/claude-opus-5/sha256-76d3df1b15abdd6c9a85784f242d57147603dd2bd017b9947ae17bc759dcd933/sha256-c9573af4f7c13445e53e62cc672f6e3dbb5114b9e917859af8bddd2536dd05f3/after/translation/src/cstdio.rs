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
//!
//! # Which `sscanf`?
//!
//! glibc ships two distinct implementations and `<stdio.h>` redirects the
//! source-level name `sscanf` to `__isoc99_sscanf` whenever the translation
//! unit is compiled for C99 or later. `nm -D --undefined-only` on the C `.so`
//! confirms the C build imports `__isoc99_sscanf@GLIBC_2.7`, not
//! `sscanf@GLIBC_2.2.5`, and the two are genuinely different function pointers
//! at run time. The variants differ in whether `%a` is the C99 float
//! conversion or the older GNU "allocate the string" extension. That makes no
//! difference for the fixed `"%d%zn"` format used here, but importing the exact
//! symbol the C object file imports removes the whole question rather than
//! relying on the two implementations agreeing.

use std::ffi::{c_char, c_int};

#[cfg(target_env = "gnu")]
unsafe extern "C" {
    /// `int sscanf(const char *restrict s, const char *restrict format, ...)`
    ///
    /// Bound to `__isoc99_sscanf`, which is what glibc's `<stdio.h>` makes
    /// `driver.c`'s call to `sscanf` resolve to.
    #[link_name = "__isoc99_sscanf"]
    pub fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
}

#[cfg(not(target_env = "gnu"))]
unsafe extern "C" {
    /// `int sscanf(const char *restrict s, const char *restrict format, ...)`
    ///
    /// Non-glibc targets have a single `sscanf`, so no redirection applies.
    pub fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
}

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...)`
    ///
    /// `printf` is not redirected by glibc, so the C build and this crate
    /// import the identical `printf@GLIBC_2.2.5` and therefore share the same
    /// `stdout` `FILE` object and buffering state.
    pub fn printf(format: *const c_char, ...) -> c_int;
}
