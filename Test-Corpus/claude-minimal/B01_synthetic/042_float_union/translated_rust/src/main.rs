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

use std::io::{self, Read};

/// Format a `f64` in C's `%a` (hex float) style, matching glibc's output.
fn format_hex_float(f: f64) -> String {
    let bits = f.to_bits();
    let sign = (bits >> 63) & 1;
    let exp_bits = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    let sign_str = if sign == 1 { "-" } else { "" };

    // Special values: NaN and infinity.
    if exp_bits == 0x7ff {
        if mantissa == 0 {
            return format!("{}inf", sign_str);
        } else {
            return "nan".to_string();
        }
    }

    // Zero (positive or negative).
    if exp_bits == 0 && mantissa == 0 {
        return format!("{}0x0p+0", sign_str);
    }

    let (leading_digit, unbiased_exp) = if exp_bits == 0 {
        // Subnormal numbers.
        (0u8, -1022i32)
    } else {
        // Normal numbers.
        (1u8, exp_bits - 1023)
    };

    // Build the mantissa as 13 hex digits, then trim trailing zeros.
    let mut hex_digits = format!("{:013x}", mantissa);
    while hex_digits.ends_with('0') {
        hex_digits.pop();
    }

    let mantissa_part = if hex_digits.is_empty() {
        format!("{}", leading_digit)
    } else {
        format!("{}.{}", leading_digit, hex_digits)
    };

    let exp_sign = if unbiased_exp >= 0 { "+" } else { "-" };
    let exp_abs = unbiased_exp.unsigned_abs();

    format!("{}0x{}p{}{}", sign_str, mantissa_part, exp_sign, exp_abs)
}

fn driver(f: f64) {
    let bits = f.to_bits();
    // C's "%llx" prints lowercase hex with no leading zeros.
    println!("{:x} {} {:.4}", bits, format_hex_float(f), f);
}

fn main() {
    // Match the C program's `scanf("%lf", &f)` behavior: read a double from
    // standard input. We read the entire input and parse the first whitespace-
    // delimited token as a double, mirroring how scanf skips leading whitespace.
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");

    let f: f64 = input
        .split_whitespace()
        .next()
        .and_then(|tok| tok.parse::<f64>().ok())
        .unwrap_or(0.0);

    driver(f);
}
