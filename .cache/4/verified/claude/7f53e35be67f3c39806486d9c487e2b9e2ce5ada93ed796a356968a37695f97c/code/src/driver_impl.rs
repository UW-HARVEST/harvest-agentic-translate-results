// Translated from c_src/src/main.c
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

//! Shared implementation used by both the `driver` binary and the `driver`
//! cdylib. This is the direct translation of `c_src/src/main.c`.

use std::io::{self, Read, Write};

/// Minimal buffered byte reader over stdin with one byte of push-back,
/// mirroring the character-at-a-time consumption of C's `scanf`.
pub struct Stdin {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl Default for Stdin {
    fn default() -> Self {
        Self::new()
    }
}

impl Stdin {
    pub fn new() -> Self {
        Stdin {
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    /// Returns the next byte, or `None` at end of input / on read error.
    fn getc(&mut self) -> Option<u8> {
        if self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            self.pos += 1;
            return Some(c);
        }
        if self.eof {
            return None;
        }
        let mut chunk = [0u8; 4096];
        loop {
            match io::stdin().read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
                    self.pos = 1;
                    return Some(self.buf[0]);
                }
            }
        }
    }

    /// Pushes the most recently read byte back onto the stream (ungetc).
    fn ungetc(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)`: skips leading whitespace (including newlines),
/// accepts an optional sign followed by decimal digits. On overflow glibc
/// saturates at the `long` limits and the result is then truncated to `int`.
/// Returns `Some(value)` on a successful conversion, `None` on matching
/// failure or input failure (leaving the destination untouched, as C does).
pub fn scanf_int(input: &mut Stdin) -> Option<i32> {
    // Skip whitespace.
    let mut c = loop {
        match input.getc() {
            None => return None,
            Some(c) if is_space(c) => continue,
            Some(c) => break c,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.getc() {
            None => return None,
            Some(next) => c = next,
        }
    }

    if !c.is_ascii_digit() {
        // Matching failure: put back the offending character.
        input.ungetc();
        return None;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = i64::from(c - b'0');
        if !saturated {
            if negative {
                match acc.checked_mul(10).and_then(|v| v.checked_sub(digit)) {
                    Some(v) => acc = v,
                    None => {
                        acc = i64::MIN;
                        saturated = true;
                    }
                }
            } else {
                match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                    Some(v) => acc = v,
                    None => {
                        acc = i64::MAX;
                        saturated = true;
                    }
                }
            }
        }

        match input.getc() {
            None => break,
            Some(next) => {
                if next.is_ascii_digit() {
                    c = next;
                } else {
                    input.ungetc();
                    break;
                }
            }
        }
    }

    // Truncation of the converted `long` to `int`.
    Some(acc as i32)
}

/// `void driver(int x)` from `c_src/src/main.c`.
///
/// `y = 2*x; y += 300; printf("%d\n", y);` — signed overflow is wrapped, which
/// is what the C compiler emits for this code at the project's default
/// optimization level.
pub fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    let mut out = io::stdout();
    let _ = write!(out, "{}\n", y);
    // C's stdout is line buffered on a terminal and fully buffered otherwise;
    // Rust's `Stdout` is line buffered, so an explicit flush is not required
    // for correctness of the byte stream, only for its timing.
    let _ = out.flush();
}

/// `int main(void)` from `c_src/src/main.c`.
pub fn run() -> i32 {
    let mut input = Stdin::new();
    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&mut input) {
        x = v;
    }
    driver(x);
    0
}
