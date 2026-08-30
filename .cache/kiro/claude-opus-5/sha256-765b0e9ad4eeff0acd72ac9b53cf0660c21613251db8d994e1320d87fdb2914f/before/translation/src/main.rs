// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/main.c` (a CWE-369 divide-by-zero driver).
//!
//! Behaviour is reproduced exactly, bugs included: `bad()` performs the
//! division without guarding against a zero divisor, so the `(int)` cast of
//! an infinite / NaN quotient is replicated with x86-64 `cvttsd2si`
//! semantics (the "integer indefinite" value `INT_MIN`).

mod catof;
mod cio;

use catof::atof;
use cio::{fgets, printf_int_line, printf_line};

/// `#define CHAR_ARRAY_SIZE 20`
const CHAR_ARRAY_SIZE: usize = 20;

/// `void printLine (const char * line)`
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        printf_line(line);
    }
}

/// `void printIntLine (int intNumber)`
fn print_int_line(int_number: i32) {
    printf_int_line(int_number);
}

/// C's `(int)` cast of a `double`, as compiled on x86-64 (`cvttsd2si`).
///
/// Out-of-range and NaN operands are undefined behaviour in C; the hardware
/// yields the "integer indefinite" value `0x80000000` (`i32::MIN`). Rust's
/// `as` would instead saturate, so the conversion is done by hand.
fn double_to_int(value: f64) -> i32 {
    if value.is_nan() {
        return i32::MIN;
    }
    let truncated = value.trunc();
    if truncated >= 2_147_483_648.0 || truncated < -2_147_483_648.0 {
        return i32::MIN;
    }
    truncated as i32
}

/// `void bad()`
fn bad() {
    let mut data: f32 = 0.0;
    {
        // char inputBuffer[CHAR_ARRAY_SIZE];
        match fgets(CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = atof(&input_buffer) as f32;
            }
            None => {
                print_line(Some("fgets() failed."));
            }
        }
    }
    {
        // The C code divides unconditionally; `data` may still be 0.0F.
        let result = double_to_int(100.0 / f64::from(data));
        print_int_line(result);
    }
}

/// `static void goodG2B()`
fn good_g2b() {
    let data: f32 = 2.0;
    {
        let result = double_to_int(100.0 / f64::from(data));
        print_int_line(result);
    }
}

/// `static void goodB2G()`
fn good_b2g() {
    let mut data: f32 = 0.0;
    {
        match fgets(CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = atof(&input_buffer) as f32;
            }
            None => {
                print_line(Some("fgets() failed."));
            }
        }
    }
    if f64::from(data).abs() > 0.000001 {
        let result = double_to_int(100.0 / f64::from(data));
        print_int_line(result);
    } else {
        print_line(Some("This would result in a divide by zero"));
    }
}

/// `void good()`
fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
    cio::flush();
}
