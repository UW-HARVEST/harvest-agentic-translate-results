// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust.

use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[allow(dead_code)]
fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

fn print_int_line(int_number: c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, int_number);
    }
}

fn bad() {
    // Original C uses alloca(10) (10 bytes) but writes 10 ints (40 bytes),
    // which overflows the buffer. The only observable output is data[0],
    // which equals source[0] == 0 in either case.
    let mut data: [c_int; 10] = [0; 10];
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let mut data: [c_int; 10] = [0; 10];
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
