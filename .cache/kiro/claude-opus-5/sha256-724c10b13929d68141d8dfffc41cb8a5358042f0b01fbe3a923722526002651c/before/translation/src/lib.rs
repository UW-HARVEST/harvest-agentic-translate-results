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

// C identifiers are kept verbatim so the exported ABI matches the original.
#![allow(non_snake_case)]

use std::ffi::{c_char, c_float, c_int};
use std::ptr;

// The C code prints via stdio. Calling into libc's printf keeps stream
// buffering, locale and formatting behaviour bit-for-bit identical to the
// original, and keeps output correctly interleaved with any C caller.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Replicates C's `(int)` cast of a `double` on x86-64 (SSE2 `cvttsd2si`).
///
/// The original C code performs `(int)(100.0 / data)`, which is undefined
/// behaviour when the quotient is infinite (a divide-by-zero, the very bug this
/// code demonstrates) or otherwise outside the range of `int`. On x86-64 the
/// hardware conversion yields the "integer indefinite" value `INT_MIN` in that
/// case. Rust's `as` cast instead saturates, so the check is done by hand to
/// reproduce the C behaviour rather than fix it.
fn double_to_int_c(value: f64) -> c_int {
    if value.is_nan() || value >= 2147483648.0 || value <= -2147483649.0 {
        c_int::MIN
    } else {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line != ptr::null() {
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), intNumber);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_float) {
    let result: c_int = double_to_int_c(100.0 / (data as f64));
    unsafe {
        printIntLine(result);
    }
}

fn good_g2b() {
    let data: c_float;
    data = 2.0f32;
    {
        let result: c_int = double_to_int_c(100.0 / (data as f64));
        unsafe {
            printIntLine(result);
        }
    }
}

fn good_b2g(data: c_float) {
    if (data as f64).abs() > 0.000001 {
        let result: c_int = double_to_int_c(100.0 / (data as f64));
        unsafe {
            printIntLine(result);
        }
    } else {
        unsafe {
            printLine(c"This would result in a divide by zero".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}

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
