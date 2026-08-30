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

use std::ffi::{c_char, c_double, c_float, c_int};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Format strings, NUL terminated, exactly as they appear in the C source.
const FMT_STR_NL: &[u8] = b"%s\n\0";
const FMT_INT_NL: &[u8] = b"%d\n\0";

/// Reproduce the C `(int)` cast of a `double`.
///
/// The C source performs `(int)(100.0 / data)`. When `data` is zero the
/// quotient is an infinity (or NaN for 0.0/0.0), and converting that to `int`
/// is undefined behavior in C. On x86-64 (and AArch64 with the same observable
/// result) the compiler emits a truncating conversion instruction that yields
/// the "integer indefinite" value `INT_MIN` for NaN and for any value outside
/// the representable range. Rust's `as` cast instead saturates, so the
/// conversion is done explicitly here to keep the original behavior.
fn double_to_int(value: c_double) -> c_int {
    let truncated = value.trunc();
    if truncated >= -2_147_483_648.0f64 && truncated <= 2_147_483_647.0f64 {
        truncated as c_int
    } else {
        c_int::MIN
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(FMT_STR_NL.as_ptr() as *const c_char, line);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(FMT_INT_NL.as_ptr() as *const c_char, int_number);
    }
}

/// Print a NUL terminated byte literal through `printLine`.
fn print_literal(line: &[u8]) {
    unsafe { printLine(line.as_ptr() as *const c_char) }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_float) {
    let result = double_to_int(100.0f64 / c_double::from(data));
    printIntLine(result);
}

fn good_g2b() {
    let data: c_float = 2.0f32;
    {
        let result = double_to_int(100.0f64 / c_double::from(data));
        printIntLine(result);
    }
}

fn good_b2g(data: c_float) {
    if c_double::from(data).abs() > 0.000001f64 {
        let result = double_to_int(100.0f64 / c_double::from(data));
        printIntLine(result);
    } else {
        print_literal(b"This would result in a divide by zero\0");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    print_literal(b"Calling good()...\0");
    good(good_data);
    print_literal(b"Finished good()\0");
    print_literal(b"Calling bad()...\0");
    bad(bad_data);
    print_literal(b"Finished bad()\0");
}
