//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `c_src/include/lib.h`):
//!   char *tool_basename(char *path);
//!
//! The header contains no namespace/renaming macros, so the linker symbol is
//! `tool_basename` verbatim.

use std::ffi::c_char;
use std::ptr;

/// Faithful re-implementation of C `strrchr`.
///
/// Returns a pointer to the last occurrence of `c` in the NUL-terminated string
/// `s`, or NULL if `c` does not occur. As in C, the terminating NUL is part of
/// the searched string, so `c == 0` yields a pointer to the terminator.
///
/// # Safety
/// `s` must point to a NUL-terminated string.
unsafe fn strrchr(s: *const c_char, c: c_char) -> *mut c_char {
    let mut last: *mut c_char = ptr::null_mut();
    let mut p = s as *mut c_char;

    loop {
        let ch = unsafe { *p };
        if ch == c {
            last = p;
        }
        if ch == 0 {
            break;
        }
        p = unsafe { p.add(1) };
    }

    last
}

/// Return the final path component of `path`, treating both `/` and `\` as
/// separators.
///
/// Translated verbatim from `c_src/src/lib.c`. The original performs no NULL
/// check on `path`; that behaviour (a crash on NULL) is preserved rather than
/// "fixed". The `s1 > s2` pointer comparison picks whichever separator occurs
/// later in the string.
///
/// # Safety
/// `path` must point to a NUL-terminated string, exactly as the C version
/// requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let mut path = path;

    let s1: *mut c_char = unsafe { strrchr(path, b'/' as c_char) };
    let s2: *mut c_char = unsafe { strrchr(path, b'\\' as c_char) };

    if !s1.is_null() && !s2.is_null() {
        path = if s1 > s2 {
            unsafe { s1.add(1) }
        } else {
            unsafe { s2.add(1) }
        };
    } else if !s1.is_null() {
        path = unsafe { s1.add(1) };
    } else if !s2.is_null() {
        path = unsafe { s2.add(1) };
    }

    path
}
