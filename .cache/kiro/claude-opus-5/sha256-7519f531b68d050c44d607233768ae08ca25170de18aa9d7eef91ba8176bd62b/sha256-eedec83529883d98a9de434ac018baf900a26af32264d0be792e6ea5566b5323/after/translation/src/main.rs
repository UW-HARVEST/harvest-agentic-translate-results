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
//! * stdin is consumed lazily, one buffer refill at a time, and only while the
//!   conversion still needs another character. `scanf` never waits for EOF once
//!   it has seen the character that ends the number, so neither may this
//!   program: on an endless stream such as `yes 1 | driver` the C prints `302`
//!   and exits, and reading stdin to EOF here would hang instead.

use std::io::{BufRead, ErrorKind, Write};

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

/// Returns the next byte of `input` without consuming it, or `None` at EOF or
/// on a read error (both of which make `scanf` give up without storing).
///
/// One `fill_buf` is at most one `read` syscall, so a partially filled buffer is
/// returned as soon as it is available. That is what lets the scan finish
/// without waiting for the writer to close the stream.
fn peek<R: BufRead>(input: &mut R) -> Option<u8> {
    loop {
        match input.fill_buf() {
            Ok([]) => return None,
            Ok(buf) => return Some(buf[0]),
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// Emulates a single `scanf("%d", ...)` conversion, pulling from `input` only
/// as far as the conversion requires.
///
/// Returns `Some(value)` when the conversion succeeds, or `None` on input
/// failure (EOF or a read error before any digit) or matching failure (a
/// non-digit where a digit was required), in which case the caller must leave
/// its variable unmodified.
fn scanf_i32<R: BufRead>(input: &mut R) -> Option<i32> {
    // Directive whitespace: skip any amount, including newlines.
    while let Some(c) = peek(input) {
        if !is_c_space(c) {
            break;
        }
        input.consume(1);
    }

    // Optional sign.
    let negative = match peek(input) {
        Some(b'-') => {
            input.consume(1);
            true
        }
        Some(b'+') => {
            input.consume(1);
            false
        }
        _ => false,
    };

    // At least one digit is required, otherwise it is a matching failure.
    if !matches!(peek(input), Some(c) if c.is_ascii_digit()) {
        return None;
    }

    // Accumulate as glibc does: build a `long`, clamping at the extremes.
    let mut acc: i64 = 0;
    let mut overflow = false;
    while let Some(c) = peek(input) {
        if !c.is_ascii_digit() {
            // The terminating character stays unread, as `scanf` ungets it.
            break;
        }
        input.consume(1);
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

    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    if let Some(v) = scanf_i32(&mut input) {
        x = v;
    }

    driver(x);
}
