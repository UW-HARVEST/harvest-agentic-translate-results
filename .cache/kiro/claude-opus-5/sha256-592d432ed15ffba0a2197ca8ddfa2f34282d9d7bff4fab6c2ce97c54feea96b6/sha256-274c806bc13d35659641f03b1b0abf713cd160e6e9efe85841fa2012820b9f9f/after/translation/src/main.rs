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
//! The C program reads a single `int` with `scanf("%d", &x)` and prints
//! `2 * x + 300`. Behaviour preserved here:
//!
//! * On a matching failure (or EOF) `scanf` leaves `x` untouched, so the C
//!   program keeps the initial value `0` and prints `300`.
//! * `%d` skips leading whitespace, accepts an optional sign, then base-10
//!   digits, and stops at the first non-digit (it reads across newlines).
//! * glibc converts into a `long` that saturates at `LONG_MIN`/`LONG_MAX` on
//!   overflow and then truncates that `long` to `int` for `%d`.
//! * `2 * x + 300` is computed with wrapping arithmetic, matching the two's
//!   complement wraparound produced by gcc/clang for `int` overflow.

use std::io::{Read, Write};

/// The `register int` qualifier has no observable effect; it is a hint only.
fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);

    // printf("%d\n", y);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", y);
    let _ = out.flush();
}

/// True for the characters `isspace` recognises in the C locale, which is the
/// set `scanf` skips before a `%d` conversion.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates a single `scanf("%d", ...)` conversion over `input`.
///
/// Returns `Some(value)` when the conversion succeeds, or `None` on input
/// failure (EOF before any non-whitespace) or matching failure (no digits),
/// in which case the caller must leave its variable unmodified.
fn scanf_i32(input: &[u8]) -> Option<i32> {
    let mut i = 0usize;

    // Directive whitespace: skip any amount, including newlines.
    while i < input.len() && is_c_space(input[i]) {
        i += 1;
    }

    // Optional sign.
    let negative = match input.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };

    // At least one digit is required, otherwise it is a matching failure.
    if !matches!(input.get(i), Some(c) if c.is_ascii_digit()) {
        return None;
    }

    // Accumulate as glibc does: build a `long`, clamping at the extremes.
    let mut acc: i64 = 0;
    let mut overflow = false;
    while let Some(&c) = input.get(i) {
        if !c.is_ascii_digit() {
            break;
        }
        let digit = i64::from(c - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            }) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if overflow {
        // ERANGE: glibc stores LONG_MIN / LONG_MAX.
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    // `%d` without a length modifier truncates the converted long to int.
    Some(acc as i32)
}

fn main() {
    let mut x: i32 = 0;

    let mut buf = Vec::new();
    // A failed read behaves like EOF for scanf: `x` is left alone.
    if std::io::stdin().read_to_end(&mut buf).is_ok() {
        if let Some(v) = scanf_i32(&buf) {
            x = v;
        }
    }

    driver(x);
}
