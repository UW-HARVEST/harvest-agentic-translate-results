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

/// Minimal `stdin` wrapper that reads one byte at a time and supports a
/// single-byte pushback, mirroring how C's `scanf` peeks at the next
/// character and ungets it when it does not belong to the conversion.
struct Scanner {
    input: io::Stdin,
    /// Byte that was read but pushed back (C's `ungetc`).
    pushback: Option<u8>,
    /// Sticky end-of-file flag, matching C's stream EOF indicator.
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: io::stdin(),
            pushback: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.input.read(&mut buf) {
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
        self.pushback = Some(b);
    }

    /// Equivalent of `scanf("%d", &out)`.
    ///
    /// Returns `Some(value)` when exactly one item was converted (the C return
    /// value of 1) and `None` for either a matching failure (return 0) or an
    /// input failure / EOF (return EOF). The original C code treats both
    /// non-1 outcomes identically by breaking out of the read loop, so they do
    /// not need to be distinguished here.
    fn scan_int(&mut self) -> Option<i32> {
        // `%d` first skips any amount of leading whitespace, including
        // newlines, so a conversion can span line boundaries.
        let mut b = loop {
            match self.next_byte() {
                Some(c) if is_c_space(c) => continue,
                Some(c) => break c,
                // Input failure: EOF before any conversion.
                None => return None,
            }
        };

        // Optional sign.
        let mut negative = false;
        if b == b'+' || b == b'-' {
            negative = b == b'-';
            match self.next_byte() {
                Some(c) => b = c,
                None => return None,
            }
        }

        // At least one decimal digit is required, otherwise this is a matching
        // failure and the offending character stays in the stream.
        if !b.is_ascii_digit() {
            self.unget(b);
            return None;
        }

        // glibc collects the digits and converts them with `strtol`, which
        // saturates at the `long` limits on overflow; the result is then
        // narrowed to `int`. Reproduce that (implementation-defined) behavior.
        let mut acc: i64 = 0;
        let mut saturated = false;
        loop {
            let digit = i64::from(b - b'0');
            if !saturated {
                match acc
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(digit))
                {
                    Some(v) => acc = v,
                    None => saturated = true,
                }
            }

            match self.next_byte() {
                Some(c) if c.is_ascii_digit() => b = c,
                Some(c) => {
                    self.unget(c);
                    break;
                }
                None => break,
            }
        }

        let value: i64 = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            // `acc` is the magnitude; negating cannot overflow because a
            // magnitude large enough to reach i64::MIN would have saturated.
            -acc
        } else {
            acc
        };

        // Narrowing `long` -> `int`, as glibc's `%d` store does.
        Some(value as i32)
    }
}

/// C's whitespace set as recognized by `isspace` in the default locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Translation of:
///
/// ```c
/// void fma_array(int *out, const int *mul1, const int *mul2,
///                const int *add, int len)
/// ```
///
/// The only call site passes the same pointer for all four arguments, so every
/// element is updated in place as `out[i] * out[i] + out[i]`. Element `i` is
/// read before it is written, so the aliasing does not change the result of any
/// individual iteration. Signed overflow (undefined behavior in C) is
/// reproduced as the two's-complement wraparound that the compiled C actually
/// exhibits.
fn fma_array_all_aliased(out: &mut [i32], len: usize) {
    for i in 0..len {
        let v = out[i];
        out[i] = v.wrapping_mul(v).wrapping_add(v);
    }
}

/// Translation of `void driver(int *out, int len)`.
fn driver(out: &mut [i32], len: usize, stdout: &mut impl Write) -> io::Result<()> {
    fma_array_all_aliased(out, len);
    for i in 0..len {
        writeln!(stdout, "{}", out[i])?;
    }
    Ok(())
}

fn main() {
    // `int data[100];` is uninitialized in C, but `driver` only ever touches
    // the first `i` elements, all of which are written by the read loop.
    let mut data = [0i32; 100];

    let mut scanner = Scanner::new();
    let mut i = 0usize;
    while i < 100 {
        match scanner.scan_int() {
            Some(v) => data[i] = v,
            None => break,
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    if driver(&mut data, i, &mut out).is_err() {
        return;
    }
    let _ = out.flush();
}
