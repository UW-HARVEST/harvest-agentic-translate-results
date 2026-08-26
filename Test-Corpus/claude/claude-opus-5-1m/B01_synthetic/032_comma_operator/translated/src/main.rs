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

use std::io::{self, BufWriter, Read, Write};

/// Minimal byte-at-a-time reader over stdin with one byte of pushback,
/// mirroring how C's `scanf` pulls characters from the stream (it may read
/// across newlines while skipping leading whitespace).
struct Scanner {
    inner: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            inner: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// C's `isspace` for the default "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)`: returns `Some(value)` on a successful
/// conversion, `None` on input failure (EOF) or matching failure, in which
/// case the caller's variable is left untouched (as in C).
fn scan_int(sc: &mut Scanner) -> Option<i32> {
    // Skip leading whitespace.
    let mut b = loop {
        match sc.next_byte() {
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
            None => return None, // input failure
        }
    };

    // Optional sign.
    let negative = match b {
        b'-' => {
            b = match sc.next_byte() {
                Some(c) => c,
                None => return None,
            };
            true
        }
        b'+' => {
            b = match sc.next_byte() {
                Some(c) => c,
                None => return None,
            };
            false
        }
        _ => false,
    };

    if !b.is_ascii_digit() {
        // Matching failure: push the offending character back.
        sc.unget(b);
        return None;
    }

    // Accumulate digits.  glibc converts into a `long` (saturating on
    // overflow, like strtol) and then stores the low 32 bits into the `int`.
    let mut acc: i64 = 0;
    let mut overflow = false;
    loop {
        let d = (b - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        match sc.next_byte() {
            Some(c) if c.is_ascii_digit() => b = c,
            Some(c) => {
                sc.unget(c);
                break;
            }
            None => break,
        }
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

    Some(value as i32)
}

fn driver<W: Write>(out: &mut W, x: i32) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        let _ = writeln!(out, "{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut sc = Scanner::new();
    if let Some(v) = scan_int(&mut sc) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
    let _ = out.flush();
}
