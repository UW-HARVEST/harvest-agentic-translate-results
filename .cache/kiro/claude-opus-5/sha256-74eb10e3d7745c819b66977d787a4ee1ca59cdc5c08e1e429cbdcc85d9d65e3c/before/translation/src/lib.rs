// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/driver.c`.
//!
//! Output is produced through the C library's `printf` so that formatting *and*
//! stdio buffering match the original byte for byte, including when the calling
//! program interleaves its own C `stdio` output with calls into this library.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...)`
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `CHAR_MAX` from `<limits.h>`, tracking the platform's signedness of `char`
/// exactly as the C preprocessor would.
const CHAR_MAX: c_char = c_char::MAX;

/// `void printLine(const char * line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

/// `void printHexCharLine(char charHex)`
///
/// `charHex` undergoes the default argument promotion to `int` before reaching
/// `printf`, and `%02x` then reinterprets that `int` as `unsigned int`. With a
/// signed `char` a negative value therefore prints as eight hex digits (e.g.
/// `fffffffe`), which is the original program's behaviour and is preserved here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printHexCharLine(charHex: c_char) {
    unsafe {
        printf(
            b"%02x\n\0".as_ptr() as *const c_char,
            charHex as c_int,
        )
    };
}

/// `void bad()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: c_char;
    data = CHAR_MAX;
    if data > 0 {
        // char result = data * 2;  -- promoted to int, multiplied, truncated back
        let result: c_char = (data as c_int).wrapping_mul(2) as c_char;
        unsafe { printHexCharLine(result) };
    }
}

/// `static void goodG2B()`
fn goodG2B() {
    let data: c_char;
    data = 2;
    if data > 0 {
        let result: c_char = (data as c_int).wrapping_mul(2) as c_char;
        unsafe { printHexCharLine(result) };
    }
}

/// `static void goodB2G()`
fn goodB2G() {
    let mut data: c_char;
    // The original assigns ' ' and then immediately overwrites it; the dead
    // store is kept for fidelity.
    data = b' ' as c_char;
    let _ = data;
    data = CHAR_MAX;
    if data > 0 {
        if (data as c_int) < (CHAR_MAX as c_int / 2) {
            let result: c_char = (data as c_int).wrapping_mul(2) as c_char;
            unsafe { printHexCharLine(result) };
        } else {
            unsafe {
                printLine(
                    b"data value is too large to perform arithmetic safely.\0".as_ptr()
                        as *const c_char,
                )
            };
        }
    }
}

/// `void good()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}

/// `void driver(int useGood)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
