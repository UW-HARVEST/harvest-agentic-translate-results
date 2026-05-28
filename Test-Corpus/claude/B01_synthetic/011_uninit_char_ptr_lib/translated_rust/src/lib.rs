// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output.

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // char *data; printLine(data); -- intentionally uninitialized to
    // reproduce the original C behavior (CWE-457 style).
    // We use std::mem::zeroed() to avoid Rust's UB warning while still
    // matching what would commonly happen in practice on many platforms.
    // The original C code reads an uninitialized stack value, which is UB;
    // here we read a deterministic "uninitialized" value (zeroed) so that
    // the function is callable without invoking Rust-level UB.
    let data: *const c_char = unsafe { std::mem::zeroed() };
    printLine(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    // char *data; data = "string"; printLine(data);
    let data = b"string\0".as_ptr() as *const c_char;
    printLine(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
