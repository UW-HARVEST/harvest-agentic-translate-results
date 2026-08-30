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

/// Byte-oriented reader over stdin with a single byte of pushback, mirroring
/// the way C's stdio stream is consumed by `scanf`.
struct Scanner {
    input: Box<dyn Read>,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: Box::new(std::io::stdin()),
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            self.pos += 1;
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut chunk = [0u8; 8192];
        loop {
            match self.input.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
                    self.pos = 1;
                    return Some(self.buf[0]);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Push the most recently read byte back onto the stream (C's `ungetc`).
    fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// Equivalent of `scanf("%d", &x)`: returns `Some(value)` when the
    /// conversion succeeds (scanf returning 1), and `None` on either a
    /// matching failure (scanf returning 0) or end of input (EOF).
    fn scan_int(&mut self) -> Option<i32> {
        // Leading whitespace is skipped, newlines included.
        let mut c = loop {
            match self.next_byte() {
                None => return None,
                Some(b) => {
                    if is_space(b) {
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
                None => return None,
                Some(b) => c = b,
            }
        }

        if !c.is_ascii_digit() {
            // Matching failure: the offending character stays in the stream.
            self.unget();
            return None;
        }

        // Accumulate the magnitude, saturating the way glibc's strtol-based
        // conversion does before the result is truncated to `int`.
        let mut mag: u128 = 0;
        loop {
            if mag <= u128::from(u64::MAX) {
                mag = mag * 10 + u128::from(c - b'0');
            }
            match self.next_byte() {
                None => break,
                Some(b) => {
                    if b.is_ascii_digit() {
                        c = b;
                    } else {
                        self.unget();
                        break;
                    }
                }
            }
        }

        let wide: i64 = if negative {
            if mag >= (i64::MAX as u128) + 1 {
                i64::MIN
            } else {
                -(mag as i64)
            }
        } else if mag > i64::MAX as u128 {
            i64::MAX
        } else {
            mag as i64
        };

        Some(wide as i32)
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out = vec![0i32; len];
    let mut ones = vec![0i32; len];
    let mut zeros = vec![0i32; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, &data[..len], &zeros, len);
    out[len - 1]
}

fn main() {
    // Matches the C `int data[100];` (indeterminate contents until read).
    let mut data = [0i32; 100];
    let mut scanner = Scanner::new();

    let mut i: usize = 0;
    while i < 100 {
        match scanner.scan_int() {
            Some(v) => data[i] = v,
            None => break,
        }
        i += 1;
    }

    let result = call_fma(&data, i);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", result);
    let _ = out.flush();
}
