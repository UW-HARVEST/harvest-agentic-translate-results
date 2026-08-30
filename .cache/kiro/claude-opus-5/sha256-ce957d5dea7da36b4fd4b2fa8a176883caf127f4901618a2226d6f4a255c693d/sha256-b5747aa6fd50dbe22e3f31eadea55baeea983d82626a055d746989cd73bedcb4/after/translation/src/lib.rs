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
//! Output is produced through the C library's `printf` so that formatting and
//! stdout buffering are byte-identical to the original, even when this library
//! is linked into a C program that also writes to stdout.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `CHAR_MAX` from `<limits.h>`: 127 where `char` is signed, 255 where it is
/// unsigned. `c_char` mirrors the platform's `char` signedness, so deriving the
/// constant from it reproduces the C value on each target.
const CHAR_MAX: c_char = c_char::MAX;

/// C: `void printLine(const char * line)`
#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// C: `void printHexCharLine(char charHex)`
#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    // printf("%02x\n", charHex);
    //
    // The argument undergoes the C default argument promotion to `int`, so a
    // negative `char` (e.g. -2) is printed as the full 32-bit unsigned value
    // ("fffffffe"), exactly as the C code does.
    unsafe {
        printf(c"%02x\n".as_ptr(), char_hex as c_int);
    }
}

/// C: `void bad()`
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data: c_char = CHAR_MAX;
    if data > 0 {
        // `data * 2` is computed in `int` and truncated on assignment to
        // `char`; the overflow is reproduced, not fixed.
        let result: c_char = ((data as c_int) * 2) as c_char;
        printHexCharLine(result);
    }
}

/// C: `static void goodG2B()`
fn good_g2b() {
    let data: c_char = 2;
    if data > 0 {
        let result: c_char = ((data as c_int) * 2) as c_char;
        printHexCharLine(result);
    }
}

/// C: `static void goodB2G()`
#[allow(unused_assignments)]
fn good_b2g() {
    // The original assigns `' '` and then immediately overwrites it with
    // `CHAR_MAX`; the dead store is preserved for fidelity.
    let mut data: c_char = b' ' as c_char;
    data = CHAR_MAX;
    if data > 0 {
        if (data as c_int) < (CHAR_MAX as c_int) / 2 {
            let result: c_char = ((data as c_int) * 2) as c_char;
            printHexCharLine(result);
        } else {
            printLine(c"data value is too large to perform arithmetic safely.".as_ptr());
        }
    }
}

/// C: `void good()`
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    good_g2b();
    good_b2g();
}

/// C: `void driver(int useGood)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
