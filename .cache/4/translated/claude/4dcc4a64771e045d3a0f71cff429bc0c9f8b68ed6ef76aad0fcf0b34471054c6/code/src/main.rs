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

/// void fma_array(int *restrict out, const int *mul1, const int *mul2,
///                const int *add, int len)
fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: i32) {
    let mut i: i32 = 0;
    while i < len {
        let idx = i as usize;
        // C: out[i] = mul1[i] * mul2[i] + add[i];  (int arithmetic)
        out[idx] = mul1[idx]
            .wrapping_mul(mul2[idx])
            .wrapping_add(add[idx]);
        i += 1;
    }
}

/// int call_fma(const int *data, int len)
fn call_fma(data: &[i32], len: i32) -> i32 {
    if len == 0 {
        return 0;
    }
    let n = len as usize;
    // int out[len]; int ones[len]; int zeros[len];  (VLAs, uninitialized)
    let mut out: Vec<i32> = vec![0; n];
    let mut ones: Vec<i32> = vec![0; n];
    let mut zeros: Vec<i32> = vec![0; n];

    out[0] = 0;
    let mut i: i32 = 0;
    while i < len {
        ones[i as usize] = 1;
        zeros[i as usize] = 0;
        i += 1;
    }

    fma_array(&mut out, &ones, &data[..n], &zeros, len);
    out[n - 1]
}

/// Outcome of one `scanf("%d", ...)` conversion.
enum ScanResult {
    /// Conversion succeeded; scanf returned 1.
    Value(i32),
    /// Matching failure: input present but not a valid integer; scanf returned 0.
    MatchFailure,
    /// Input failure: end of input (or read error) before any conversion; scanf
    /// returned EOF.
    Eof,
}

/// A byte-at-a-time reader over stdin with a single byte of pushback, mirroring
/// the way C's `scanf` consumes only as much of the stream as it needs.
struct Scanner<R: Read> {
    reader: R,
    buf: [u8; 4096],
    len: usize,
    pos: usize,
    pushback: Option<u8>,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(reader: R) -> Self {
        Scanner {
            reader,
            buf: [0u8; 4096],
            len: 0,
            pos: 0,
            pushback: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        loop {
            if self.pos < self.len {
                let b = self.buf[self.pos];
                self.pos += 1;
                return Some(b);
            }
            if self.eof {
                return None;
            }
            match self.reader.read(&mut self.buf) {
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

    fn unget(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    /// Emulates glibc's `scanf("%d", &v)` for a single conversion:
    ///   * leading whitespace (C locale `isspace`) is skipped,
    ///   * an optional '+'/'-' sign is accepted,
    ///   * one or more decimal digits are required,
    ///   * the value saturates at the platform `long`/`long long` bounds and is
    ///     then truncated to `int`.
    fn scan_int(&mut self) -> ScanResult {
        // Skip whitespace. Hitting end of input here is an input failure (EOF).
        let mut c = loop {
            match self.next_byte() {
                None => return ScanResult::Eof,
                Some(b) => {
                    if is_c_space(b) {
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
                None => return ScanResult::MatchFailure,
                Some(b) => c = b,
            }
        }

        if !c.is_ascii_digit() {
            self.unget(c);
            return ScanResult::MatchFailure;
        }

        // Accumulate the magnitude, flagging (and then ignoring) any excess so
        // that the running value never overflows.
        let mut magnitude: u128 = 0;
        let mut overflowed = false;
        loop {
            let digit = u128::from(c - b'0');
            if !overflowed {
                if magnitude > u128::from(u64::MAX) {
                    overflowed = true;
                } else {
                    magnitude = magnitude * 10 + digit;
                }
            }
            match self.next_byte() {
                None => break,
                Some(b) => {
                    if b.is_ascii_digit() {
                        c = b;
                    } else {
                        self.unget(b);
                        break;
                    }
                }
            }
        }

        // glibc saturates at LONG_MIN/LONG_MAX before narrowing to int.
        let wide: i64 = if negative {
            if overflowed || magnitude >= (i64::MAX as u128) + 1 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if overflowed || magnitude > i64::MAX as u128 {
            i64::MAX
        } else {
            magnitude as i64
        };

        ScanResult::Value(wide as i32)
    }
}

/// C-locale `isspace`.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn main() {
    // int data[100];
    let mut data: [i32; 100] = [0; 100];
    let mut scanner = Scanner::new(io::stdin());

    let mut i: i32 = 0;
    while i < 100 {
        match scanner.scan_int() {
            ScanResult::Value(v) => data[i as usize] = v,
            ScanResult::MatchFailure | ScanResult::Eof => break,
        }
        i += 1;
    }

    let result = call_fma(&data, i);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", result);
    let _ = out.flush();
}
