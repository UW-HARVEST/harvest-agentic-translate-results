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

/// A byte-oriented reader over stdin with a single byte of pushback,
/// mimicking C's `stdin` stream semantics as used by `scanf`.
struct CStdin {
    inner: io::Stdin,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            inner: io::stdin(),
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    fn fill(&mut self) -> bool {
        if self.pos < self.buf.len() {
            return true;
        }
        if self.eof {
            return false;
        }
        let mut chunk = [0u8; 4096];
        loop {
            match self.inner.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return false;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
                    self.pos = 0;
                    return true;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return false;
                }
            }
        }
    }

    fn getc(&mut self) -> Option<u8> {
        if !self.fill() {
            return None;
        }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }

    /// Push back the most recently read byte (`ungetc`).
    fn ungetc(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
}

/// Result of a `scanf("%d", ...)`-style conversion.
enum ScanResult {
    /// One item successfully converted.
    Value(i32),
    /// Matching failure (returns 0 in C).
    NoMatch,
    /// Input failure before any conversion (returns EOF in C).
    Eof,
}

/// Emulates `scanf("%d", &x)`: skip leading whitespace, then read an
/// optional sign followed by decimal digits.
fn scan_int(input: &mut CStdin) -> ScanResult {
    // Skip whitespace (this crosses newlines, like scanf).
    let mut c = loop {
        match input.getc() {
            None => return ScanResult::Eof,
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.getc() {
            None => return ScanResult::NoMatch,
            Some(n) => c = n,
        }
    }

    if !c.is_ascii_digit() {
        input.ungetc();
        return ScanResult::NoMatch;
    }

    // glibc converts the digit run with strtol semantics: out-of-range
    // values saturate to LONG_MAX / LONG_MIN and are then truncated to int.
    let limit: u64 = if negative {
        // |LONG_MIN|
        1u64 << 63
    } else {
        i64::MAX as u64
    };
    let mut acc: u64 = 0;
    let mut out_of_range = false;
    loop {
        if !out_of_range {
            match acc
                .checked_mul(10)
                .and_then(|v| v.checked_add(u64::from(c - b'0')))
            {
                Some(v) if v <= limit => acc = v,
                _ => out_of_range = true,
            }
        }
        match input.getc() {
            None => break,
            Some(n) => {
                if n.is_ascii_digit() {
                    c = n;
                } else {
                    input.ungetc();
                    break;
                }
            }
        }
    }

    let value: i64 = if out_of_range {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };
    // Assignment of the (long) result to an int object truncates.
    ScanResult::Value(value as i32)
}

fn fma_array(out: &mut [i32], len: usize) {
    // In the original C, out, mul1, mul2 and add all alias the same array.
    for i in 0..len {
        let v = out[i];
        out[i] = v.wrapping_mul(v).wrapping_add(v);
    }
}

fn driver<W: Write>(out: &mut [i32], len: usize, w: &mut W) {
    fma_array(out, len);
    for i in 0..len {
        let _ = writeln!(w, "{}", out[i]);
    }
}

fn main() {
    let mut data = [0i32; 100];
    let mut input = CStdin::new();

    let mut i: usize = 0;
    while i < 100 {
        match scan_int(&mut input) {
            ScanResult::Value(v) => data[i] = v,
            _ => break,
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut w = io::BufWriter::new(stdout.lock());
    driver(&mut data, i, &mut w);
    let _ = w.flush();
}
