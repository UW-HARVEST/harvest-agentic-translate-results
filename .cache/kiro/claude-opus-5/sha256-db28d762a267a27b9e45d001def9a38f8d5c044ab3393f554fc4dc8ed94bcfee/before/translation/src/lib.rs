//! Rust translation of `c_src/src/lib.c`.
//!
//! Provides `tool_basename`, which returns a pointer into the caller-owned
//! string just past the last `/` or `\` separator (whichever comes later).
//! The behaviour, including the original code's quirks, is reproduced exactly.

use std::ffi::c_char;

/// Equivalent of C's `strrchr`: returns a pointer to the last occurrence of
/// `needle` in the NUL-terminated string starting at `s`, or null if absent.
///
/// # Safety
///
/// `s` must point to a NUL-terminated string, exactly as the C code requires.
unsafe fn strrchr(s: *const c_char, needle: c_char) -> *mut c_char {
    let mut found: *mut c_char = std::ptr::null_mut();
    let mut p = s as *mut c_char;

    // Walk the whole string; the terminating NUL itself is never a match here
    // because the callers only search for '/' and '\\'.
    loop {
        let c = unsafe { *p };
        if c == 0 {
            return found;
        }
        if c == needle {
            found = p;
        }
        p = unsafe { p.add(1) };
    }
}

/// Returns the file-name component of `path`.
///
/// Both `/` and `\` are accepted as separators; when both are present the one
/// that occurs later in the string wins. If neither is present the input
/// pointer is returned unchanged.
///
/// # Safety
///
/// `path` must point to a NUL-terminated string, matching the C contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let mut path = path;

    let s1 = unsafe { strrchr(path, b'/' as c_char) };
    let s2 = unsafe { strrchr(path, b'\\' as c_char) };

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
