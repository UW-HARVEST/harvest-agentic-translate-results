// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{self, Read, Write};

/// Byte-at-a-time stdin reader with a single-byte pushback slot, mirroring the
/// `getc`/`ungetc` behavior that C's `scanf` relies on. Reading one byte at a
/// time means the stream is never advanced past the characters the conversion
/// actually consumes.
struct Scanner {
    stdin: io::Stdin,
    pushback: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            stdin: io::stdin(),
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
            match self.stdin.read(&mut buf) {
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
    /// Returns the number of successfully assigned items (1) or 0 when the
    /// conversion fails / hits end of input, leaving `out` untouched, exactly as
    /// C does.
    fn scan_i32(&mut self, out: &mut i32) -> i32 {
        // A `%d` conversion first skips any amount of leading whitespace,
        // including newlines, so a value may sit on a later line.
        let mut c = loop {
            match self.next_byte() {
                None => return 0,
                Some(b) => {
                    if b == b' '
                        || b == b'\t'
                        || b == b'\n'
                        || b == b'\r'
                        || b == 0x0b
                        || b == 0x0c
                    {
                        continue;
                    }
                    break b;
                }
            }
        };

        // Optional sign.
        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            match self.next_byte() {
                None => return 0,
                Some(b) => c = b,
            }
        }

        // At least one digit is required, otherwise the conversion fails and the
        // offending character is pushed back.
        if !c.is_ascii_digit() {
            self.unget(c);
            return 0;
        }

        // glibc accumulates into a `long` and saturates at LONG_MIN/LONG_MAX,
        // then narrows the result to `int`.
        let mut value: i64 = 0;
        let mut saturated = false;
        loop {
            let digit = i64::from(c - b'0');
            if !saturated {
                match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                    Some(v) => value = v,
                    None => saturated = true,
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

        let wide: i64 = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            -value
        } else {
            value
        };

        *out = wide as i32;
        1
    }
}

fn driver<W: Write>(x: i32, w: &mut W) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        let _ = writeln!(w, "{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut scanner = Scanner::new();
    let _ = scanner.scan_i32(&mut x);

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    driver(x, &mut out);
    let _ = out.flush();
}
