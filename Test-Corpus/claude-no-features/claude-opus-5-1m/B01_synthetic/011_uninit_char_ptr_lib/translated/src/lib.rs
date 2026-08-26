// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;
use std::ffi::c_int;
use std::mem::MaybeUninit;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        printf(fmt, line);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // Mirror the C source: `char *data;` is left uninitialized
    // before being passed to printLine.
    #[allow(invalid_value)]
    let data: *const c_char = MaybeUninit::<*const c_char>::uninit().assume_init();
    printLine(data);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: *const c_char;
    data = b"string\0".as_ptr() as *const c_char;
    printLine(data);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
