// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // Original C uses alloca(10) which allocates only 10 bytes, then writes
    // 10 ints (40 bytes) - undefined behavior in C, but `source` is zero-
    // initialized, so the observable output `data[0]` is 0.
    let source: [c_int; 10] = [0; 10];
    let mut data: [c_int; 10] = [0; 10];
    let mut i: usize = 0;
    while i < 10 {
        data[i] = source[i];
        i += 1;
    }
    printIntLine(data[0]);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let source: [c_int; 10] = [0; 10];
    let mut data: [c_int; 10] = [0; 10];
    let mut i: usize = 0;
    while i < 10 {
        data[i] = source[i];
        i += 1;
    }
    printIntLine(data[0]);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
