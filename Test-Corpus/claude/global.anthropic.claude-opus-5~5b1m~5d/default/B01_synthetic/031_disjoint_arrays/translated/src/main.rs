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

/// Minimal emulation of C's `stdin` stream: a buffered byte source with a
/// single-byte pushback slot (equivalent to `ungetc`).
struct CStdin {
    inner: io::Stdin,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    pushed: Option<u8>,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            inner: io::stdin(),
            buf: Vec::new(),
            pos: 0,
            eof: false,
            pushed: None,
        }
    }

    /// Reads one byte, or `None` on end of input (C's `getc`).
    fn getc(&mut self) -> Option<u8> {
        if let Some(c) = self.pushed.take() {
            return Some(c);
        }
        if self.pos >= self.buf.len() {
            if self.eof {
                return None;
            }
            let mut chunk = [0u8; 8192];
            match self.inner.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
                    self.pos = 0;
                }
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }

    /// Pushes one byte back onto the stream (C's `ungetc`).
    fn ungetc(&mut self, c: u8) {
        self.pushed = Some(c);
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates `scanf("%d", out)`: returns the number of assigned items
/// (1 on success, 0 on matching failure, -1 / EOF on input failure).
fn scanf_d(input: &mut CStdin, out: &mut i32) -> i32 {
    // Leading whitespace is skipped; running out of input here is an
    // input failure (EOF), not a matching failure.
    let mut c = loop {
        match input.getc() {
            None => return -1,
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.getc() {
            None => return -1,
            Some(n) => c = n,
        }
    }

    if !c.is_ascii_digit() {
        // Matching failure: the offending character stays in the stream.
        input.ungetc(c);
        return 0;
    }

    // Magnitude accumulated with saturation, then clamped to `long` range
    // and truncated to `int`, matching glibc's strtol-based conversion.
    let mut magnitude: u64 = 0;
    loop {
        let digit = (c - b'0') as u64;
        magnitude = magnitude.saturating_mul(10).saturating_add(digit);
        match input.getc() {
            None => break,
            Some(n) => {
                if n.is_ascii_digit() {
                    c = n;
                } else {
                    input.ungetc(n);
                    break;
                }
            }
        }
    }

    let as_long: i64 = if negative {
        if magnitude > (i64::MAX as u64) + 1 {
            i64::MIN
        } else if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else if magnitude > i64::MAX as u64 {
        i64::MAX
    } else {
        magnitude as i64
    };

    *out = as_long as i32;
    1
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
    let mut out: Vec<i32> = vec![0; len];
    let mut ones: Vec<i32> = vec![0; len];
    let mut zeros: Vec<i32> = vec![0; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

fn main() {
    let mut input = CStdin::new();
    let mut data = [0i32; 100];
    let mut i: usize = 0;
    while i < 100 {
        let mut value = 0i32;
        if scanf_d(&mut input, &mut value) != 1 {
            break;
        }
        data[i] = value;
        i += 1;
    }

    let result = call_fma(&data, i);
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = write!(lock, "{}\n", result);
    let _ = lock.flush();
}
