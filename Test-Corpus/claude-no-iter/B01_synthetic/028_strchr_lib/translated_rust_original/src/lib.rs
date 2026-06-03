// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;
use std::io::{self, Write};

/// Reproduces C's `strchr` semantics: returns the index of the first byte
/// equal to `c` in the NUL-terminated input starting at `start`, or `None`
/// if not found. If `c` is `0`, returns the position of the NUL terminator.
fn strchr_index(bytes: &[u8], start: usize, c: u8) -> Option<usize> {
    let mut i = start;
    loop {
        let b = bytes[i];
        if b == c {
            return Some(i);
        }
        if b == 0 {
            return None;
        }
        i += 1;
    }
}

/// Returns the C-string length (number of bytes before the first NUL).
///
/// # Safety
/// `in_ptr` must point to a valid NUL-terminated C string.
unsafe fn c_strlen(in_ptr: *const c_char) -> usize {
    let mut len = 0usize;
    while unsafe { *in_ptr.add(len) } != 0 {
        len += 1;
    }
    len
}

fn foo_impl(bytes: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    let mut s: usize = 0;
    // Mirrors: for (const char *s = in; s = strchr(s, c); s++)
    while let Some(found) = strchr_index(bytes, s, c) {
        res = res.wrapping_add(1);
        s = found + 1;
    }
    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(in_ptr: *const c_char, c: c_char) -> std::ffi::c_int {
    if in_ptr.is_null() {
        return 0;
    }
    let len = unsafe { c_strlen(in_ptr) };
    // We need to also include the NUL byte so strchr_index can detect end-of-string.
    let bytes = unsafe { std::slice::from_raw_parts(in_ptr as *const u8, len + 1) };
    foo_impl(bytes, c as u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_ptr: *const c_char) {
    if in_ptr.is_null() {
        return;
    }
    let len = unsafe { c_strlen(in_ptr) };
    let bytes = unsafe { std::slice::from_raw_parts(in_ptr as *const u8, len + 1) };

    let a_count = foo_impl(bytes, b'A');
    let x_count = foo_impl(bytes, b'x');

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "A: {}\n", a_count);
    let _ = write!(handle, "x: {}\n", x_count);
    let _ = handle.flush();
}
