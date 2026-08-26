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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C program reads a single integer with `scanf("%d", &x)` and prints
//! `2 * x + 300` using `printf("%d\n", y)`. The translation reproduces the
//! original behaviour bit-for-bit, including:
//!
//! * `scanf` skipping leading whitespace (spaces, tabs, newlines, ...), so the
//!   value may appear on any line;
//! * a matching failure or end-of-file leaving `x` at its initial value of `0`;
//! * glibc's `%d` conversion accumulating the digits into a 64-bit `long`
//!   (saturating at `LONG_MAX` / `LONG_MIN` on overflow) and then storing the
//!   truncated low 32 bits into the `int` destination;
//! * the wrapping 32-bit arithmetic of `2*x` and `y += 300`.

use std::io::{self, Read, Write};

/// Bytes that C's `isspace()` reports as whitespace in the "C" locale, i.e. the
/// characters that `scanf`'s `%d` directive silently consumes before the number.
fn is_c_space(b: u8) -> bool {
    b == b' '        // space
        || b == b'\t' // horizontal tab
        || b == b'\n' // line feed
        || b == 0x0b_u8 // vertical tab
        || b == 0x0c_u8 // form feed
        || b == b'\r' // carriage return
}

/// Reads exactly one byte, returning `None` on end-of-file.
fn next_byte<R: Read>(reader: &mut R) -> Option<u8> {
    let mut buf = [0u8; 1];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => return Some(buf[0]),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// Equivalent of `scanf("%d", &x)`: returns `Some(value)` when the conversion
/// succeeds and `None` on a matching failure or input failure (in which case the
/// C code leaves `x` untouched).
fn scanf_d<R: Read>(reader: &mut R) -> Option<i32> {
    // Skip leading whitespace; `%d` crosses newlines while doing so.
    let mut cur = loop {
        let b = next_byte(reader)?;
        if !is_c_space(b) {
            break b;
        }
    };

    // Optional sign.
    let negative = match cur {
        b'-' => {
            cur = next_byte(reader).unwrap_or(0);
            true
        }
        b'+' => {
            cur = next_byte(reader).unwrap_or(0);
            false
        }
        _ => false,
    };

    // At least one decimal digit is required, otherwise this is a matching
    // failure and the destination is left alone.
    if !cur.is_ascii_digit() {
        return None;
    }

    // Accumulate the magnitude the way glibc's `strtol` does: saturate at the
    // `long` limit while still consuming every remaining digit.
    let limit: u64 = if negative {
        // |LONG_MIN| == 2^63
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude: u64 = 0;
    let mut saturated = false;

    loop {
        let digit = u64::from(cur - b'0');
        if !saturated {
            match magnitude
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
            {
                Some(v) if v <= limit => magnitude = v,
                _ => {
                    saturated = true;
                    magnitude = limit;
                }
            }
        }

        match next_byte(reader) {
            Some(b) if b.is_ascii_digit() => cur = b,
            // The first non-digit terminates the conversion (glibc pushes it
            // back onto the stream; the C program never reads again).
            _ => break,
        }
    }

    // Value as stored in a `long`, then truncated to the `int` destination.
    let as_long: i64 = if negative {
        (magnitude.wrapping_neg()) as i64
    } else {
        magnitude as i64
    };
    Some(as_long as i32)
}

/// Faithful translation of the C `driver` function (`register` is a no-op hint).
fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", y);
    let _ = out.flush();
}

fn main() {
    let mut x: i32 = 0;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    if let Some(v) = scanf_d(&mut input) {
        x = v;
    }
    driver(x);
}
