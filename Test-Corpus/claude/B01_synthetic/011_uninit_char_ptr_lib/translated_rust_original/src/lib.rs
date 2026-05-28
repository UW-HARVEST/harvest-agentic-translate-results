// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output.

use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

fn bad() {
    // char *data; printLine(data); -- intentionally uninitialized to
    // reproduce the original C behavior (CWE-457 style).
    let data: *const c_char = unsafe { MaybeUninit::uninit().assume_init() };
    print_line(data);
}

fn good() {
    // char *data; data = "string"; printLine(data);
    let data = b"string\0".as_ptr() as *const c_char;
    print_line(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
