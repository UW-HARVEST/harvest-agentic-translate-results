// Rust translation of the C library in c_src/.
//
// Original copyright notice from the C sources:
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

use core::ffi::{c_char, c_double, c_float, c_int};

// The C code emits all of its output through the C runtime's `printf`, writing
// to the process-wide `stdout` FILE stream.  Calling the very same `printf`
// keeps the produced bytes -- and the buffering/interleaving behaviour with any
// other C code linked into the same process -- byte-for-byte identical.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Format string `"%s\n"` used by `printLine`.
static FMT_S: [c_char; 4] = [b'%' as c_char, b's' as c_char, b'\n' as c_char, 0];
/// Format string `"%d\n"` used by `printIntLine`.
static FMT_D: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// Reproduces the C semantics of `(int)double_value`.
///
/// A C cast from a floating point type to `int` is undefined behaviour when the
/// truncated value cannot be represented in an `int` (this includes NaN and the
/// infinities).  On x86-64 -- the platform this library is built for -- the
/// compiler emits `cvttsd2si`, which yields the "integer indefinite" value
/// `0x80000000` (`INT_MIN`) for every such invalid conversion.  Rust's `as`
/// operator instead saturates, so the conversion is emulated explicitly here in
/// order to preserve the original (buggy) behaviour exactly.
#[inline]
fn c_double_to_int(value: c_double) -> c_int {
    if value.is_nan() {
        return c_int::MIN;
    }
    let truncated = value.trunc();
    if truncated >= -2147483648.0 && truncated <= 2147483647.0 {
        truncated as c_int
    } else {
        // NaN / infinities / out-of-range -> integer indefinite value.
        c_int::MIN
    }
}

/// `void printLine (const char * line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_S.as_ptr(), line);
        }
    }
}

/// `void printIntLine (int intNumber)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(FMT_D.as_ptr(), intNumber);
    }
}

/// `void bad(float data)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_float) {
    // int result = (int)(100.0 / data);
    let result: c_int = c_double_to_int(100.0f64 / (data as c_double));
    unsafe {
        printIntLine(result);
    }
}

/// `static void goodG2B()`
fn goodG2B() {
    let data: c_float;
    data = 2.0f32;
    {
        let result: c_int = c_double_to_int(100.0f64 / (data as c_double));
        unsafe {
            printIntLine(result);
        }
    }
}

/// `static void goodB2G(float data)`
fn goodB2G(data: c_float) {
    // if (fabs(data) > 0.000001) -- `data` is promoted to double, and the
    // literal is a double, not a float.
    if (data as c_double).abs() > 0.000001f64 {
        let result: c_int = c_double_to_int(100.0f64 / (data as c_double));
        unsafe {
            printIntLine(result);
        }
    } else {
        unsafe {
            printLine(c"This would result in a divide by zero".as_ptr());
        }
    }
}

/// `void good(float data)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_float) {
    goodG2B();
    goodB2G(data);
}

/// `void driver(float goodData, float badData)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(goodData: c_float, badData: c_float) {
    unsafe {
        printLine(c"Calling good()...".as_ptr());
        good(goodData);
        printLine(c"Finished good()".as_ptr());
        printLine(c"Calling bad()...".as_ptr());
        bad(badData);
        printLine(c"Finished bad()".as_ptr());
    }
}
