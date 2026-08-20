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

use std::io::{self, Read, Write};

/// Byte-at-a-time reader over stdin that supports a one byte "pushback",
/// mirroring the way C's `scanf` peeks at (and ungets) the character that
/// terminates a conversion.
struct Scanner<R: Read> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
    len: usize,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(inner: R) -> Self {
        Scanner {
            inner,
            buf: vec![0u8; 8192],
            pos: 0,
            len: 0,
            eof: false,
        }
    }

    /// Returns the next byte from the stream, or `None` at end-of-file.
    fn next_byte(&mut self) -> Option<u8> {
        loop {
            if self.pos < self.len {
                let b = self.buf[self.pos];
                self.pos += 1;
                return Some(b);
            }
            if self.eof {
                return None;
            }
            match self.inner.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.len = n;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Push the most recently read byte back onto the stream (`ungetc`).
    fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// Emulates `scanf("%d", &out)`.
    ///
    /// Returns 1 on a successful conversion, 0 on a matching failure and -1
    /// (EOF) if end-of-file is hit before any non-whitespace input is seen.
    ///
    /// Overflow follows glibc: the digits are accumulated with `strtol`
    /// semantics (saturating at `long` range) and the resulting `long` is then
    /// assigned to an `int`, i.e. truncated to the low 32 bits.
    fn scan_int(&mut self, out: &mut i32) -> i32 {
        // Skip leading whitespace, exactly the set isspace() matches in the C
        // locale: space, \t, \n, \v, \f, \r.
        let mut c = loop {
            match self.next_byte() {
                None => return -1, // EOF before any input item
                Some(b) => {
                    if matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                        continue;
                    }
                    break b;
                }
            }
        };

        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            match self.next_byte() {
                None => return -1, // EOF right after the sign
                Some(b) => c = b,
            }
        }

        if !c.is_ascii_digit() {
            // Matching failure: put the offending character back.
            self.unget();
            return 0;
        }

        let mut acc: i64 = 0;
        let mut overflow = false;
        loop {
            let digit = (c - b'0') as i64;
            if !overflow {
                if negative {
                    match acc
                        .checked_mul(10)
                        .and_then(|v| v.checked_sub(digit))
                    {
                        Some(v) => acc = v,
                        None => overflow = true,
                    }
                } else {
                    match acc
                        .checked_mul(10)
                        .and_then(|v| v.checked_add(digit))
                    {
                        Some(v) => acc = v,
                        None => overflow = true,
                    }
                }
            }
            match self.next_byte() {
                None => break,
                Some(b) => {
                    if b.is_ascii_digit() {
                        c = b;
                    } else {
                        // Terminating character is not consumed by scanf.
                        self.unget();
                        break;
                    }
                }
            }
        }

        if overflow {
            // strtol saturates at LONG_MAX / LONG_MIN, then the value is
            // stored into an int (low 32 bits kept).
            acc = if negative { i64::MIN } else { i64::MAX };
        }

        *out = acc as i32;
        1
    }
}

/// Equivalent of the C `fma_array(out, out, out, out, len)` call: every pointer
/// argument aliases the same buffer, so each element is replaced by
/// `x * x + x`, reading the old value before storing the new one.
///
/// Signed overflow is undefined behaviour in C; gcc/clang produce two's
/// complement wraparound here, which `wrapping_*` reproduces.
fn fma_array_aliased(out: &mut [i32], len: usize) {
    for i in 0..len {
        let x = out[i];
        out[i] = x.wrapping_mul(x).wrapping_add(x);
    }
}

fn driver<W: Write>(out: &mut [i32], len: usize, w: &mut W) {
    fma_array_aliased(out, len);
    for i in 0..len {
        let _ = writeln!(w, "{}", out[i]);
    }
}

fn main() {
    // `int data[100];` -- only the first `i` entries are ever read back, so the
    // (uninitialised in C) tail is never observed.
    let mut data = [0i32; 100];

    let mut scanner = Scanner::new(io::stdin());

    let mut i: usize = 0;
    while i < 100 {
        let mut value: i32 = 0;
        if scanner.scan_int(&mut value) != 1 {
            break;
        }
        data[i] = value;
        i += 1;
    }

    let stdout = io::stdout();
    let mut w = io::BufWriter::new(stdout.lock());
    driver(&mut data, i, &mut w);
    let _ = w.flush();
}
