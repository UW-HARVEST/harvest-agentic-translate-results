// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
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

use std::io::{Read, Write};

/// Mirrors `void driver(int x)`.
///
/// `register int y = 2*x; y += 300; printf("%d\n", y);`
///
/// Signed overflow is UB in C; gcc/clang at the codegen level wrap on two's
/// complement hardware, so wrapping arithmetic reproduces the observed
/// behavior for extreme inputs.
fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);

    // printf("%d\n", y)
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", y);
    let _ = out.flush();
}

/// A one-byte-lookahead reader over stdin, so that we consume exactly the
/// bytes `scanf` would consume (it pushes back the first non-matching byte).
struct Scanner {
    src: std::io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            src: std::io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.src.read(&mut buf) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => {
                self.peeked = Some(buf[0]);
                Some(buf[0])
            }
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn bump(&mut self) {
        self.peeked = None;
    }

    /// Emulates `scanf("%d", &out)` for the C locale.
    ///
    /// Returns the number of successfully assigned items (1), 0 on a matching
    /// failure, or -1 (EOF) when input ends before any conversion begins.
    /// On failure `out` is left untouched, exactly like C.
    fn scan_int(&mut self, out: &mut i32) -> i32 {
        // Skip leading whitespace, as the %d directive does.
        loop {
            match self.peek() {
                Some(b) if is_c_space(b) => self.bump(),
                Some(_) => break,
                None => return -1, // EOF before any conversion
            }
        }

        let mut negative = false;
        match self.peek() {
            Some(b'+') => self.bump(),
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            Some(_) => {}
            None => return -1,
        }

        // glibc builds up the numeric text and hands it to strtol, which
        // saturates at LONG_MIN/LONG_MAX on overflow; the long result is then
        // stored through an `int *`, i.e. truncated to 32 bits.
        let mut digits = 0usize;
        let mut acc: i64 = 0;
        let mut overflow = false;
        loop {
            match self.peek() {
                Some(b) if b.is_ascii_digit() => {
                    let d = i64::from(b - b'0');
                    if !overflow {
                        match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                            Some(v) => acc = v,
                            None => overflow = true,
                        }
                    }
                    digits += 1;
                    self.bump();
                }
                _ => break,
            }
        }

        if digits == 0 {
            // Matching failure: no digits were consumed.
            return 0;
        }

        let value: i64 = if overflow {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            acc.wrapping_neg()
        } else {
            acc
        };

        *out = value as i32; // truncating store through `int *`
        1
    }
}

/// Whitespace per the C locale `isspace`.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn main() {
    let mut x: i32 = 0;
    let mut scanner = Scanner::new();
    // Return value is ignored by the C code, just as here; on failure `x`
    // keeps its initial value of 0.
    let _ = scanner.scan_int(&mut x);
    driver(x);
    // return 0;
}
