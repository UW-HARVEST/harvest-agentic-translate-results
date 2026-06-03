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

fn print_hex(p: &[u8]) {
    for b in p {
        print!("{:02x}", b);
    }
    println!();
}

fn driver(x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}

/// Parse a float from the input string in a manner similar to C's `scanf("%f", ...)`.
/// Skips leading whitespace, then consumes the longest valid prefix that forms a float.
/// Returns the parsed float, or 0.0 if no valid float prefix is found (mirroring the C
/// program's behavior where `x` is initialized to 0.0 before scanf).
fn scanf_float(input: &str) -> f32 {
    let trimmed = input.trim_start();
    let bytes = trimmed.as_bytes();
    let mut end = 0usize;

    // Optional sign
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }

    let int_start = end;
    // Integer part digits
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    let had_int_digits = end > int_start;

    // Fractional part
    let mut had_frac_digits = false;
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        let frac_start = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        had_frac_digits = end > frac_start;
    }

    if !had_int_digits && !had_frac_digits {
        return 0.0;
    }

    // Optional exponent
    let pre_exp = end;
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut e = end + 1;
        if e < bytes.len() && (bytes[e] == b'+' || bytes[e] == b'-') {
            e += 1;
        }
        let exp_digits_start = e;
        while e < bytes.len() && bytes[e].is_ascii_digit() {
            e += 1;
        }
        if e > exp_digits_start {
            end = e;
        } else {
            // No exponent digits; revert
            end = pre_exp;
        }
    }

    let candidate = &trimmed[..end];
    candidate.parse::<f32>().unwrap_or(0.0)
}

fn main() {
    let mut input = String::new();
    // Mirror C's scanf which reads from stdin; if read fails, leave x = 0.0.
    let _ = io::stdin().read_to_string(&mut input);
    let x = scanf_float(&input);
    driver(x);
}
