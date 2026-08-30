// Rust translation of c_src/src/driver.c
//
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

#![allow(non_snake_case)]

use std::ffi::{c_char, c_double, c_float, c_int};

// The C code emits all of its output through C `stdio` (`printf`). To guarantee
// byte-identical output *and* identical interleaving/buffering behaviour with
// the original shared library, we route everything through the very same
// `printf` from libc rather than Rust's own `std::io::stdout`.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `"%s\n"` format string, NUL terminated.
const FMT_STR_NL: &[u8; 4] = b"%s\n\0";
/// `"%d\n"` format string, NUL terminated.
const FMT_INT_NL: &[u8; 4] = b"%d\n\0";

/// Reproduces the C `(int)` cast from a `double`, exactly as the original
/// library behaves when built for x86-64 (`cvttsd2si`): values that are NaN or
/// whose truncation does not fit into an `int` yield the "integer indefinite"
/// value `INT_MIN`. Rust's `as` operator saturates instead, so it cannot be
/// used directly here.
#[inline]
fn c_double_to_int(value: c_double) -> c_int {
    if value.is_nan() {
        return c_int::MIN;
    }
    let truncated = value.trunc();
    if truncated >= 2147483648.0 || truncated < -2147483648.0 {
        return c_int::MIN;
    }
    truncated as c_int
}

/// void printLine (const char * line)
#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_STR_NL.as_ptr() as *const c_char, line);
        }
    }
}

/// Convenience helper for printing a Rust string literal (which must already be
/// NUL terminated) through `printLine`.
#[inline]
fn print_line_bytes(line: &[u8]) {
    debug_assert_eq!(line.last(), Some(&0u8));
    printLine(line.as_ptr() as *const c_char);
}

/// void printIntLine (int intNumber)
#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(FMT_INT_NL.as_ptr() as *const c_char, intNumber);
    }
}

/// void bad(float data)
#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_float) {
    let result: c_int = c_double_to_int(100.0f64 / (data as c_double));
    printIntLine(result);
}

/// static void goodG2B()
fn goodG2B() {
    let data: c_float;
    data = 2.0f32;
    {
        let result: c_int = c_double_to_int(100.0f64 / (data as c_double));
        printIntLine(result);
    }
}

/// static void goodB2G(float data)
fn goodB2G(data: c_float) {
    if (data as c_double).abs() > 0.000001f64 {
        let result: c_int = c_double_to_int(100.0f64 / (data as c_double));
        printIntLine(result);
    } else {
        print_line_bytes(b"This would result in a divide by zero\0");
    }
}

/// void good(float data)
#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_float) {
    goodG2B();
    goodB2G(data);
}

/// void driver(float goodData, float badData)
#[unsafe(no_mangle)]
pub extern "C" fn driver(goodData: c_float, badData: c_float) {
    print_line_bytes(b"Calling good()...\0");
    good(goodData);
    print_line_bytes(b"Finished good()\0");
    print_line_bytes(b"Calling bad()...\0");
    bad(badData);
    print_line_bytes(b"Finished bad()\0");
}
