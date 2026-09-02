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
//! The original is a CWE-457 (use of uninitialized variable) demonstration: `bad()`
//! declares `int *data;` without initializing it and then dereferences it. That is
//! undefined behavior in C. The translation is *not* allowed to "fix" the bug, so it
//! reproduces the behavior observed from the reference build produced by the shipped
//! `CMakeLists.txt` (no `CMAKE_BUILD_TYPE`, i.e. unoptimized): the stale stack slot
//! read through the uninitialized pointer yields `0`, so `bad()` prints `0\n`
//! deterministically. See `UNINITIALIZED_POINTER_READ` below.

use std::io::{self, Read, Write};

/// Value observed when the reference (unoptimized) build of the C program dereferences
/// the uninitialized `int *data` in `bad()`. Kept as a named constant so the origin of
/// the value is explicit rather than looking like intentional program logic.
const UNINITIALIZED_POINTER_READ: i32 = 0;

/// `void printIntPtrLine(const int *intNumber)` -> `printf("%d\n", *intNumber);`
fn print_int_ptr_line(int_number: &i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // `%d` on an int matches Rust's Display for i32 exactly (including the `-` sign).
    let _ = write!(out, "{}\n", *int_number);
}

/// `void bad()` — dereferences an uninitialized pointer.
fn bad() {
    // `int *data;` is indeterminate; the reference build reads 0 through it.
    let data: i32 = UNINITIALIZED_POINTER_READ;
    print_int_ptr_line(&data);
}

/// `void good()` — takes the address of an initialized local.
fn good() {
    let data: i32;
    data = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

/// Byte source that mimics C's `stdin` for the purposes of a single `scanf` call:
/// bytes are consumed one at a time, and a single byte of lookahead can be returned
/// (the equivalent of `ungetc`) when a conversion stops on a non-matching character.
struct Stdin {
    reader: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            reader: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.reader.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// True for the bytes C's `isspace` accepts in the default "C" locale. `scanf`'s `%d`
/// directive skips these before converting, which is why it reads across newlines.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion and `None` on EOF or a matching
/// failure, in which case the caller leaves its variable untouched — exactly like C.
///
/// glibc implements `%d` via `strtol`, so the digits are accumulated into a `long`
/// (`i64` on this target) that saturates at `LONG_MAX` / `LONG_MIN` on overflow, and
/// the saturated `long` is then truncated when stored into the `int` argument. That
/// is reproduced here, including its surprising consequences (e.g. `4294967296`
/// truncates to `0`, while `2147483648` saturates to `LONG_MAX` and truncates to `-1`).
fn scanf_int(input: &mut Stdin) -> Option<i32> {
    // Skip leading whitespace.
    let mut b = loop {
        let b = input.next_byte()?;
        if !is_c_space(b) {
            break b;
        }
    };

    // Optional sign.
    let mut negative = false;
    if b == b'+' || b == b'-' {
        negative = b == b'-';
        b = match input.next_byte() {
            Some(next) => next,
            // Sign followed by EOF: matching failure.
            None => return None,
        };
    }

    // Digit sequence (base 10 only, so a leading "0x" stops at the 'x').
    let mut magnitude: i64 = 0;
    let mut overflowed = false;
    let mut saw_digit = false;
    loop {
        if !b.is_ascii_digit() {
            input.unget(b);
            break;
        }
        saw_digit = true;
        let digit = i64::from(b - b'0');
        if !overflowed {
            match magnitude
                .checked_mul(10)
                .and_then(|acc| acc.checked_add(digit))
            {
                Some(acc) => magnitude = acc,
                None => overflowed = true,
            }
        }
        match input.next_byte() {
            Some(next) => b = next,
            None => break,
        }
    }

    if !saw_digit {
        // Matching failure: no digits were converted.
        return None;
    }

    let as_long: i64 = if overflowed {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -magnitude
    } else {
        magnitude
    };

    // Truncating store of the `long` into the `int` conversion target.
    Some(as_long as i32)
}

fn main() {
    let mut x: i32 = 0;
    let mut input = Stdin::new();
    // A failed conversion leaves `x` at its initial value of 0, matching the C.
    if let Some(value) = scanf_int(&mut input) {
        x = value;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = io::stdout().flush();
}
