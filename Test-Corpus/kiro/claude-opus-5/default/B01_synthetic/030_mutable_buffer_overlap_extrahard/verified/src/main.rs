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

/// Buffered byte-level reader over stdin, used to reproduce `scanf("%d", ...)`
/// character consumption semantics.
struct Scanner {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    src: io::Stdin,
}

/// Outcome of a single `scanf("%d")` directive, mirroring the C return value.
enum ScanInt {
    /// Successful conversion (scanf returned 1).
    Value(i32),
    /// Matching failure (scanf returned 0).
    MatchFailure,
    /// Input failure before any conversion (scanf returned EOF).
    Eof,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            buf: Vec::new(),
            pos: 0,
            eof: false,
            src: io::stdin(),
        }
    }

    /// Look at the next byte without consuming it.
    fn peek(&mut self) -> Option<u8> {
        while self.pos >= self.buf.len() {
            if self.eof {
                return None;
            }
            self.buf.clear();
            self.pos = 0;
            let mut chunk = [0u8; 8192];
            match self.src.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        Some(self.buf[self.pos])
    }

    /// Consume the next byte.
    fn bump(&mut self) {
        self.pos += 1;
    }

    /// Matches C's `isspace` for the default "C" locale, which is the set of
    /// characters that a `%d` directive skips over (including newlines).
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Equivalent of `scanf("%d", &out)` for a single integer.
    ///
    /// Leading whitespace (newlines included) is skipped, then an optional sign
    /// and one or more decimal digits are consumed. Out-of-range values follow
    /// glibc, which parses into a `long` (saturating) and then narrows to `int`.
    fn scan_i32(&mut self) -> ScanInt {
        // Skip leading whitespace; hitting EOF here is an input failure (EOF).
        loop {
            match self.peek() {
                None => return ScanInt::Eof,
                Some(b) if Self::is_space(b) => self.bump(),
                Some(_) => break,
            }
        }

        let mut negative = false;
        match self.peek() {
            Some(b'+') => self.bump(),
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            _ => {}
        }

        let mut acc: i64 = 0;
        let mut saw_digit = false;
        while let Some(b) = self.peek() {
            if !b.is_ascii_digit() {
                break;
            }
            saw_digit = true;
            let digit = i64::from(b - b'0');
            acc = acc.saturating_mul(10);
            acc = if negative {
                acc.saturating_sub(digit)
            } else {
                acc.saturating_add(digit)
            };
            self.bump();
        }

        if !saw_digit {
            // No digits after optional sign: matching failure (scanf returns 0).
            return ScanInt::MatchFailure;
        }

        ScanInt::Value(acc as i32)
    }
}

/// Translation of:
///     void fma_array(int *out, const int *mul1, const int *mul2,
///                    const int *add, int len)
///
/// The only call site passes the same buffer for all four pointers, so this is
/// expressed as an in-place operation. Each element depends solely on its own
/// index, so the aliasing is semantically identical to the C original.
/// Signed multiply/add wrap, matching the two's-complement behavior emitted by
/// the C compiler.
fn fma_array_aliased(out: &mut [i32], len: usize) {
    for i in 0..len {
        let mul1 = out[i];
        let mul2 = out[i];
        let add = out[i];
        out[i] = mul1.wrapping_mul(mul2).wrapping_add(add);
    }
}

/// Translation of `void driver(int *out, int len)`.
fn driver<W: Write>(out: &mut [i32], len: usize, w: &mut W) {
    fma_array_aliased(out, len);
    for i in 0..len {
        let _ = writeln!(w, "{}", out[i]);
    }
}

fn main() {
    // int data[100]; — uninitialized in C, but only the first `i` entries are
    // ever read, and those are always written by a successful scanf.
    let mut data = [0i32; 100];
    let mut scanner = Scanner::new();

    let mut i: usize = 0;
    while i < 100 {
        match scanner.scan_i32() {
            ScanInt::Value(v) => data[i] = v,
            ScanInt::MatchFailure | ScanInt::Eof => break,
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut w = io::BufWriter::new(stdout.lock());
    driver(&mut data, i, &mut w);
    let _ = w.flush();
}
