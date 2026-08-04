// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn printIntLine(intNumber: c_int) {
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, intNumber);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    printIntLine(int_sum);
    // Original C: `intOne + intTwo;` — statement with no effect; preserved here
    // by computing the value and discarding it. `intSum` is intentionally not
    // updated (this is the "bad" example in the original code).
    let _ = int_one.wrapping_add(int_two);
    printIntLine(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    printIntLine(int_sum);
    int_sum = int_one.wrapping_add(int_two);
    printIntLine(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    let s1 = b"Calling good()...\0".as_ptr() as *const c_char;
    printLine(s1);
    good();
    let s2 = b"Finished good()\0".as_ptr() as *const c_char;
    printLine(s2);
    let s3 = b"Calling bad()...\0".as_ptr() as *const c_char;
    printLine(s3);
    bad();
    let s4 = b"Finished bad()\0".as_ptr() as *const c_char;
    printLine(s4);
}
