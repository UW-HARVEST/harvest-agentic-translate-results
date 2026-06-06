// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of c_src/src/driver.c

use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Equivalent of C's `strchr`: returns a pointer to the first occurrence of
/// `c` in the null-terminated C string `s`, or null if not found.
///
/// # Safety
/// `s` must be a valid pointer to a null-terminated C string.
unsafe fn rust_strchr(s: *const c_char, c: c_char) -> *const c_char {
    let mut p = s;
    loop {
        let ch = *p;
        if ch == c {
            return p;
        }
        if ch == 0 {
            return std::ptr::null();
        }
        p = p.add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(input: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s = input;
    loop {
        s = rust_strchr(s, c);
        if s.is_null() {
            break;
        }
        res += 1;
        s = s.add(1);
    }
    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let fmt_a = b"A: %d\n\0".as_ptr() as *const c_char;
    let fmt_x = b"x: %d\n\0".as_ptr() as *const c_char;
    printf(fmt_a, foo(input, b'A' as c_char));
    printf(fmt_x, foo(input, b'x' as c_char));
}
