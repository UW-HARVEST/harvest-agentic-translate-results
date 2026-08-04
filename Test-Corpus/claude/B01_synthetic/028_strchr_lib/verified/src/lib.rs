// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(input: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s: *const c_char = input;
    loop {
        let found = strchr(s, c as c_int);
        if found.is_null() {
            break;
        }
        s = found;
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
