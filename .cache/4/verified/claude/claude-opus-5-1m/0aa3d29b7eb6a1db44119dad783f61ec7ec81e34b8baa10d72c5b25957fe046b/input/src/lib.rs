//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared library):
//!   * `tool_basename`
//!
//! Behaviour is reproduced exactly as written in the C sources, including any
//! quirks: no bug fixes, identical order of checks, identical return values.

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_char;

/// Faithful re-implementation of C's `strrchr(s, c)`.
///
/// Returns a pointer to the last occurrence of the byte `c` in the
/// NUL-terminated string `s` (the terminating NUL is considered part of the
/// string, matching the C standard), or NULL if `c` does not occur.
///
/// # Safety
/// `s` must point to a NUL-terminated byte string (the C code does not
/// tolerate a NULL pointer here either).
unsafe fn strrchr(s: *const c_char, c: c_char) -> *mut c_char {
    let mut p = s;
    let mut found: *const c_char = std::ptr::null();

    loop {
        let ch = *p;
        if ch == c {
            found = p;
        }
        if ch == 0 {
            break;
        }
        p = p.add(1);
    }

    found as *mut c_char
}

/// ```c
/// char *tool_basename(char *path)
/// ```
///
/// Returns a pointer into `path` just past the last path separator (`/` or
/// `\`), or `path` itself when no separator is present.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let mut path = path;

    let s1: *mut c_char = strrchr(path, b'/' as c_char);
    let s2: *mut c_char = strrchr(path, b'\\' as c_char);

    if !s1.is_null() && !s2.is_null() {
        path = if s1 > s2 { s1.add(1) } else { s2.add(1) };
    } else if !s1.is_null() {
        path = s1.add(1);
    } else if !s2.is_null() {
        path = s2.add(1);
    }

    path
}
