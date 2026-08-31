// Rust translation of c_src/src/main.c
//
// Original C copyright notice:
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

/// Buffered byte reader over stdin with a single-byte pushback slot, mirroring
/// the behaviour of a C `FILE *` stream as used by `scanf`/`ungetc`.
struct Stdin {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    pushback: Option<u8>,
    src: io::Stdin,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            buf: Vec::new(),
            pos: 0,
            eof: false,
            pushback: None,
            src: io::stdin(),
        }
    }

    /// Equivalent of `getc()`: returns `None` at end of input.
    fn getc(&mut self) -> Option<u8> {
        if let Some(c) = self.pushback.take() {
            return Some(c);
        }
        loop {
            if self.pos < self.buf.len() {
                let c = self.buf[self.pos];
                self.pos += 1;
                return Some(c);
            }
            if self.eof {
                return None;
            }
            self.buf.clear();
            self.pos = 0;
            self.buf.resize(65536, 0);
            match self.src.read(&mut self.buf) {
                Ok(0) => {
                    self.buf.clear();
                    self.eof = true;
                    return None;
                }
                Ok(n) => self.buf.truncate(n),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    self.buf.clear();
                }
                Err(_) => {
                    self.buf.clear();
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Equivalent of `ungetc()`.
    fn ungetc(&mut self, c: u8) {
        self.pushback = Some(c);
    }
}

/// C `isspace()` for the default locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates `scanf("%d", &out)`, returning the number of assigned items
/// (1 on success, 0 on matching failure, -1 (EOF) on input failure), which is
/// what the C code compares against.
fn scanf_d(input: &mut Stdin, out: &mut i32) -> i32 {
    // Leading whitespace is skipped, including across newlines.
    let mut c = loop {
        match input.getc() {
            None => return -1, // EOF before any conversion
            Some(c) if is_space(c) => continue,
            Some(c) => break c,
        }
    };

    let negative = match c {
        b'-' => {
            c = match input.getc() {
                Some(c) => c,
                None => return -1,
            };
            true
        }
        b'+' => {
            c = match input.getc() {
                Some(c) => c,
                None => return -1,
            };
            false
        }
        _ => false,
    };

    if !c.is_ascii_digit() {
        // Matching failure: the offending character stays in the stream.
        input.ungetc(c);
        return 0;
    }

    // glibc converts via strtol (saturating at long bounds) and then assigns
    // the resulting `long` to an `int`, i.e. truncating to 32 bits.
    let mut magnitude: u64 = 0;
    let mut overflow = false;
    loop {
        let digit = u64::from(c - b'0');
        match magnitude
            .checked_mul(10)
            .and_then(|m| m.checked_add(digit))
        {
            Some(m) => magnitude = m,
            None => overflow = true,
        }
        let cutoff: u64 = if negative {
            i64::MAX as u64 + 1
        } else {
            i64::MAX as u64
        };
        if magnitude > cutoff {
            overflow = true;
        }
        match input.getc() {
            Some(next) if next.is_ascii_digit() => c = next,
            Some(next) => {
                input.ungetc(next);
                break;
            }
            None => break,
        }
    }

    let as_long: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (magnitude as i64).wrapping_neg()
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
    let mut out = vec![0i32; len];
    let mut ones = vec![0i32; len];
    let mut zeros = vec![0i32; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

fn main() {
    // `int data[100]` is uninitialised in C, but only the first `i` entries
    // (the ones actually read) are ever consumed.
    let mut data = [0i32; 100];
    let mut input = Stdin::new();

    let mut i = 0usize;
    while i < 100 {
        if scanf_d(&mut input, &mut data[i]) != 1 {
            break;
        }
        i += 1;
    }

    let result = call_fma(&data, i);

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let _ = write!(stdout, "{}\n", result);
    let _ = stdout.flush();
}
