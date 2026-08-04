// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior preserved byte-for-byte.

use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

// Format strings (NUL-terminated) used by printf.
const FMT_STR_NEWLINE: &[u8] = b"%s\n\0";
const FMT_INT_NEWLINE: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_STR_NEWLINE.as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(FMT_INT_NEWLINE.as_ptr() as *const c_char, int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    printIntLine(int_sum);
    // Mirror the C source: `intOne + intTwo;` is a no-op statement.
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
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
}
