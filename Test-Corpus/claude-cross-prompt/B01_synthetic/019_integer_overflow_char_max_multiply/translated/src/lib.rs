// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust, preserving exact behavior.

#![allow(non_snake_case)]
#![allow(unused_assignments)]

use std::ffi::c_char;
use std::ffi::c_int;

const CHAR_MAX: c_char = c_char::MAX;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn scanf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printHexCharLine(char_hex: c_char) {
    // printf("%02x\n", charHex);
    // In C: char is promoted to int (sign-extended) when passed as variadic.
    // The %x format treats it as unsigned int. We need to mimic this exactly.
    let promoted: c_int = char_hex as c_int;
    printf(b"%02x\n\0".as_ptr() as *const c_char, promoted);
}

fn goodG2B() {
    let data: c_char;
    data = 2;
    if data > 0 {
        // result = data * 2 stored in char (signed wrap)
        let result: c_char = data.wrapping_mul(2);
        unsafe {
            printHexCharLine(result);
        }
    }
}

fn goodB2G() {
    let mut data: c_char;
    data = b' ' as c_char;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: c_char = data.wrapping_mul(2);
            unsafe {
                printHexCharLine(result);
            }
        } else {
            let msg = b"data value is too large to perform arithmetic safely.\0";
            unsafe {
                printLine(msg.as_ptr() as *const c_char);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data: c_char;
    data = CHAR_MAX;
    if data > 0 {
        let result: c_char = data.wrapping_mul(2);
        unsafe {
            printHexCharLine(result);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    goodG2B();
    goodB2G();
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr() as *const c_char, &mut x as *mut c_int);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
