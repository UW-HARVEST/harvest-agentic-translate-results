// Rust translation of c_src/src/main.c
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

use std::io::{self, Read, Write};

/// Mirrors the `static int y = 123;` file-scope variable in the C source.
///
/// In the C program this global is written to by `scanf` and read back by
/// `multi_stage`, so its pre-set value of 123 is observable whenever `scanf`
/// fails to convert the second field.
struct Globals {
    y: i32,
}

/// A minimal `scanf`-style reader over stdin.
///
/// Reads one byte at a time with a single byte of pushback, which reproduces
/// C's `scanf` behaviour of consuming exactly as much input as a conversion
/// needs while freely skipping over whitespace, including newlines.
struct Scanner<R: Read> {
    input: R,
    pushback: Option<u8>,
    at_eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(input: R) -> Self {
        Scanner {
            input,
            pushback: None,
            at_eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        if self.at_eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.input.read(&mut buf) {
                Ok(0) => {
                    self.at_eof = true;
                    return None;
                }
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.at_eof = true;
                    return None;
                }
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    /// Matches C's `isspace` for the default "C" locale.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.next_byte() {
            if !Self::is_space(b) {
                self.unget(b);
                return;
            }
        }
    }

    /// Performs a single `%d` conversion.
    ///
    /// Returns `None` on input failure (EOF before any character) or matching
    /// failure (no digits), in which case `scanf` leaves the destination
    /// untouched and stops processing the remainder of the format string.
    ///
    /// On overflow this follows glibc, which converts via `strtol` semantics:
    /// the value saturates at `long` range and is then truncated to `int`.
    fn scan_i32(&mut self) -> Option<i32> {
        self.skip_whitespace();

        let mut negative = false;
        let first = self.next_byte()?;
        let mut current = match first {
            b'+' => self.next_byte(),
            b'-' => {
                negative = true;
                self.next_byte()
            }
            other => Some(other),
        };

        let mut magnitude: u64 = 0;
        let mut saw_digit = false;
        while let Some(b) = current {
            if !b.is_ascii_digit() {
                self.unget(b);
                break;
            }
            saw_digit = true;
            magnitude = magnitude
                .saturating_mul(10)
                .saturating_add(u64::from(b - b'0'));
            current = self.next_byte();
        }

        if !saw_digit {
            // Matching failure: the sign (if any) is not pushed back, matching
            // the fact that glibc has already consumed it.
            return None;
        }

        let clamped: i64 = if negative {
            if magnitude > (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                (magnitude as i128).wrapping_neg() as i64
            }
        } else if magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };

        Some(clamped as i32)
    }
}

fn multi_stage<W: Write>(out: &mut W, globals: &Globals, x: i32, z: i32) -> i32 {
    let result;

    // The C code uses `goto fail` for each failing stage, so every error also
    // prints "Operation failed" before returning.
    if x != 1 {
        let _ = write!(out, "Error: x != 1\n");
        result = 1;
    } else if globals.y != 2 {
        let _ = write!(out, "Error: x == 1 but y != 2\n");
        result = 2;
    } else if z != 3 {
        let _ = write!(out, "Error: x == 1 and y == 2, but z != 3\n");
        result = 3;
    } else {
        let _ = write!(out, "Ok!\n");
        return 0;
    }

    let _ = write!(out, "Operation failed\n");
    result
}

fn main() {
    let mut globals = Globals { y: 123 };
    let mut x: i32 = 0;
    let mut z: i32 = 0;

    // scanf("%d %d %d", &x, &y, &z): assignments happen left to right and stop
    // at the first conversion that fails, leaving later targets unmodified.
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());
    if let Some(v) = scanner.scan_i32() {
        x = v;
        if let Some(v) = scanner.scan_i32() {
            globals.y = v;
            if let Some(v) = scanner.scan_i32() {
                z = v;
            }
        }
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let result = multi_stage(&mut out, &globals, x, z);
    let _ = write!(out, "Result: {}\n", result);
    let _ = out.flush();
}
