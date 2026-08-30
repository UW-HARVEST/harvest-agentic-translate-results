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

/// A minimal buffered reader over stdin that supports one byte of pushback,
/// mirroring the behaviour of C's `stdin` stream as used by `scanf`.
struct CStdin {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    fn fill(&mut self) {
        if self.pos < self.buf.len() || self.eof {
            return;
        }
        let mut chunk = [0u8; 4096];
        loop {
            match std::io::stdin().read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
                    self.pos = 0;
                    return;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return;
                }
            }
        }
    }

    fn getc(&mut self) -> Option<u8> {
        self.fill();
        if self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    /// Push the most recently read byte back onto the stream (`ungetc`).
    fn ungetc(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion (assignment), or `None`
/// on input failure / matching failure, in which case the C code leaves the
/// destination object untouched.
fn scanf_d(inp: &mut CStdin) -> Option<i32> {
    // Skip leading whitespace.
    let mut c = loop {
        match inp.getc() {
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
            None => return None, // input failure (EOF before any conversion)
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match inp.getc() {
            Some(n) => c = n,
            None => {
                // Matching failure: sign with no digits following.
                return None;
            }
        }
    }

    if !c.is_ascii_digit() {
        inp.ungetc();
        return None; // matching failure
    }

    // Accumulate as a C `long` (64-bit), saturating like glibc's strtol,
    // then truncate to `int` on assignment.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = (c - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            }) {
                Some(v) => acc = v,
                None => {
                    saturated = true;
                    acc = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        match inp.getc() {
            Some(n) if n.is_ascii_digit() => c = n,
            Some(_) => {
                inp.ungetc();
                break;
            }
            None => break,
        }
    }

    // Assignment to `int` truncates the low 32 bits.
    Some(acc as i32)
}

fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    let out = std::io::stdout();
    let mut out = out.lock();
    let _ = write!(out, "{}\n", y);
    let _ = out.flush();
}

fn main() {
    let mut inp = CStdin::new();
    let mut x: i32 = 0;
    if let Some(v) = scanf_d(&mut inp) {
        x = v;
    }
    driver(x);
}
