//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared library):
//!   * `tool_basename`
//!
//! Behaviour is reproduced exactly as written in the C sources, including any
//! quirks: no bug fixes, identical order of checks, identical return values.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// libc's `char *strrchr(const char *s, int c)`.
    ///
    /// The C source obtains this from `#include <string.h>`; calling the same
    /// libc routine (rather than re-implementing it) keeps the observable
    /// behaviour bit-identical for *every* input, including the inputs on which
    /// the C code has undefined behaviour and simply faults: a NULL `path`, or a
    /// buffer that is not NUL-terminated. A hand-written Rust scan loop would
    /// instead hit Rust's debug-only null-pointer assertion and abort with
    /// `SIGABRT` (plus a message on stderr) where the C library raises
    /// `SIGSEGV`.
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
}

/// ```c
/// char *tool_basename(char *path)
/// ```
///
/// Returns a pointer into `path` just past the last path separator (`/` or
/// `\`), or `path` itself when no separator is present.
///
/// # Safety
/// `path` must point to a NUL-terminated byte string; exactly as in C, a NULL
/// or unterminated `path` is undefined behaviour (the C code does not check
/// either).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let mut path = path;

    // s1 = strrchr(path, '/');
    let s1: *mut c_char = strrchr(path, b'/' as c_int);
    // s2 = strrchr(path, '\\');
    let s2: *mut c_char = strrchr(path, b'\\' as c_int);

    if !s1.is_null() && !s2.is_null() {
        path = if s1 > s2 { s1.add(1) } else { s2.add(1) };
    } else if !s1.is_null() {
        path = s1.add(1);
    } else if !s2.is_null() {
        path = s2.add(1);
    }

    path
}
