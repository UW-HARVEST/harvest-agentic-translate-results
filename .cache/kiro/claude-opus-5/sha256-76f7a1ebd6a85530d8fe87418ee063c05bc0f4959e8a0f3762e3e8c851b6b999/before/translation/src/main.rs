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
//
// Rust translation of c_src/src/main.c. Behavior (including quirks) is
// preserved exactly; no bugs in the original are "fixed".

use std::io::{self, Read, Write};

/// Byte-oriented view of stdin that supports a one-byte pushback, mirroring the
/// way C's `scanf` consumes a stream (it only ever needs one character of
/// lookahead, which it pushes back with `ungetc` on a matching failure).
struct Stream {
    inner: io::Stdin,
    pushed_back: Option<u8>,
    at_eof: bool,
}

impl Stream {
    fn new() -> Self {
        Stream {
            inner: io::stdin(),
            pushed_back: None,
            at_eof: false,
        }
    }

    /// Equivalent of `getc`: returns `None` at end of input.
    fn getc(&mut self) -> Option<u8> {
        if let Some(b) = self.pushed_back.take() {
            return Some(b);
        }
        if self.at_eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
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

    /// Equivalent of `ungetc`.
    fn ungetc(&mut self, b: u8) {
        self.pushed_back = Some(b);
    }
}

/// C's `isspace` for the default "C" locale.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Return value of a single `scanf("%d", &x)` call.
enum ScanResult {
    /// One item assigned successfully.
    Ok(i32),
    /// A matching failure occurred (`scanf` returns 0).
    MatchFailure,
    /// Input failure before any conversion (`scanf` returns EOF).
    Eof,
}

/// Faithful implementation of a lone `scanf("%d", &target)` directive:
/// skip leading whitespace, accept an optional sign, then require at least one
/// decimal digit. Digits are accumulated with `long` (64-bit) saturation, just
/// as glibc's `strtol`-based conversion does, and the result is then truncated
/// to `int` on assignment.
fn scanf_d(stream: &mut Stream) -> ScanResult {
    // Leading whitespace is skipped; hitting EOF here is an input failure.
    let mut c = loop {
        match stream.getc() {
            None => return ScanResult::Eof,
            Some(b) if is_space(b) => continue,
            Some(b) => break b,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match stream.getc() {
            None => return ScanResult::Eof,
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        // No digits consumed: matching failure, offending char pushed back.
        stream.ungetc(c);
        return ScanResult::MatchFailure;
    }

    // Accumulate as a 64-bit `long`, saturating like strtol does.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = i64::from(c - b'0');
        if !saturated {
            match acc
                .checked_mul(10)
                .and_then(|v| if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                })
            {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        match stream.getc() {
            None => break,
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                stream.ungetc(b);
                break;
            }
        }
    }

    if saturated {
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    // Assignment to `int` truncates.
    ScanResult::Ok(acc as i32)
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
    let mut data = [0i32; 100];
    let mut stream = Stream::new();

    let mut i: usize = 0;
    while i < 100 {
        match scanf_d(&mut stream) {
            ScanResult::Ok(v) => data[i] = v,
            _ => break,
        }
        i += 1;
    }

    let result = call_fma(&data, i);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", result);
    let _ = out.flush();
}
