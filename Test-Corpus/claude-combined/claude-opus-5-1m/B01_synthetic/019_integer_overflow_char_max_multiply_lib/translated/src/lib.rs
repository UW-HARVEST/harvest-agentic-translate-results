// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust. Behavior preserved byte-for-byte from the original C.

#![allow(non_snake_case)]
#![allow(unused_assignments)]

use std::ffi::c_char;
use std::ffi::c_int;

// CHAR_MAX value matches C's CHAR_MAX. On Linux x86_64, `char` is signed and
// CHAR_MAX is 127.
const CHAR_MAX: c_char = c_char::MAX;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printHexCharLine(char_hex: c_char) {
    unsafe {
        // %02x in C with a `char` argument gets promoted to int via default
        // argument promotion. Replicate by passing as c_int.
        printf(b"%02x\n\0".as_ptr() as *const c_char, char_hex as c_int);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: c_char;
    data = CHAR_MAX;
    if data > 0 {
        // Replicate C's signed char overflow on multiplication. In C this is
        // technically undefined, but in practice with GCC/Clang it wraps.
        let result: c_char = data.wrapping_mul(2);
        unsafe {
            printHexCharLine(result);
        }
    }
}

fn goodG2B() {
    let data: c_char;
    data = 2;
    if data > 0 {
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
            unsafe {
                printLine(b"data value is too large to perform arithmetic safely.\0".as_ptr() as *const c_char);
            }
        }
    }
    // suppress unused_assignments warning for the initial assignment of ' '
    let _ = data;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe {
            good();
        }
    } else {
        unsafe {
            bad();
        }
    }
}
