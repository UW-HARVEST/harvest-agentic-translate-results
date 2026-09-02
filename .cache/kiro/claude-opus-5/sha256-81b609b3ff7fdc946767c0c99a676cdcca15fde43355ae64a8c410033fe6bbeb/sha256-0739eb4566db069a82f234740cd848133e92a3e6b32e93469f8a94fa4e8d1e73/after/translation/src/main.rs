// Rust translation of c_src/src/main.c
//
// Original C copyright header (reproduced for attribution):
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

use std::io::{self, BufReader, Read, Write};
use std::sync::atomic::{AtomicI32, Ordering};

/// Mirrors the C file-scope `static int y = 123;`.
///
/// In the C program `y` is a global that `scanf` writes into as its *second*
/// conversion, and which `multi_stage` then reads directly (it is never passed
/// as a parameter). An atomic is used so the global shape is preserved without
/// needing `unsafe`.
static Y: AtomicI32 = AtomicI32::new(123);

/// Byte-oriented reader over stdin with a single byte of pushback, used to
/// reproduce `scanf`'s incremental scanning behavior.
struct Scanner<R: Read> {
    inner: R,
    /// A byte that was read but not consumed by a conversion (`ungetc`).
    pushback: Option<u8>,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(inner: R) -> Self {
        Scanner {
            inner,
            pushback: None,
            eof: false,
        }
    }

    /// Reads the next byte, or `None` at end of input.
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
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
                // `scanf` retries on EINTR; any other error behaves like EOF
                // for this program, which never inspects `ferror`.
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Pushes a byte back so the next `next_byte` returns it (`ungetc`).
    fn unget(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    /// C `isspace` in the default "C" locale.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Performs one `%d` conversion.
    ///
    /// Returns `Some(value)` on a successful conversion, or `None` on an input
    /// failure (EOF before any non-whitespace) or a matching failure (no digits
    /// present). On `None` the caller must leave its variable untouched, which
    /// is what C does when a conversion does not complete.
    ///
    /// Overflow reproduces glibc: the digits are accumulated as a `long`
    /// saturating at `LONG_MAX` / `LONG_MIN` (strtol semantics), and that
    /// `long` is then truncated to `int` on assignment. This was verified
    /// against the compiled C program: `4294967297` and `8589934593` yield
    /// `x == 1` (plain truncation), while `18446744073709551617` does not
    /// (it saturates to `LONG_MAX`, i.e. `-1` as an `int`).
    fn scan_int(&mut self) -> Option<i32> {
        // `%d` skips any amount of leading whitespace, including newlines.
        let mut b = loop {
            let b = self.next_byte()?;
            if !Self::is_space(b) {
                break b;
            }
        };

        // Optional sign.
        let negative = match b {
            b'-' => {
                b = self.next_byte()?;
                true
            }
            b'+' => {
                b = self.next_byte()?;
                false
            }
            _ => false,
        };

        // At least one digit is required, otherwise this is a matching failure.
        if !b.is_ascii_digit() {
            self.unget(b);
            return None;
        }

        // Accumulate with strtol-style saturation on `long` (64-bit here).
        let mut acc: i64 = 0;
        let mut saturated = false;
        loop {
            let digit = i64::from(b - b'0');
            if !saturated {
                acc = match acc
                    .checked_mul(10)
                    .and_then(|v| if negative {
                        v.checked_sub(digit)
                    } else {
                        v.checked_add(digit)
                    })
                {
                    Some(v) => v,
                    None => {
                        saturated = true;
                        if negative {
                            i64::MIN
                        } else {
                            i64::MAX
                        }
                    }
                };
            }

            match self.next_byte() {
                Some(nb) if nb.is_ascii_digit() => b = nb,
                Some(nb) => {
                    // Trailing non-digit is not consumed by the conversion.
                    self.unget(nb);
                    break;
                }
                None => break,
            }
        }

        // Assignment to an `int*` truncates the `long`.
        Some(acc as i32)
    }
}

fn multi_stage(out: &mut impl Write, x: i32, z: i32) -> i32 {
    let mut result: i32 = 0;

    // The C body uses `goto fail`; a labeled block reproduces the same control
    // flow, including that the success path `return`s *before* the fail label.
    'fail: {
        if x != 1 {
            let _ = write!(out, "Error: x != 1\n");
            result = 1;
            break 'fail;
        }

        if Y.load(Ordering::Relaxed) != 2 {
            let _ = write!(out, "Error: x == 1 but y != 2\n");
            result = 2;
            break 'fail;
        }

        if z != 3 {
            let _ = write!(out, "Error: x == 1 and y == 2, but z != 3\n");
            result = 3;
            break 'fail;
        }

        let _ = write!(out, "Ok!\n");
        return result;
    }

    // fail:
    let _ = write!(out, "Operation failed\n");
    result
}

fn main() {
    let mut x: i32 = 0;
    let mut z: i32 = 0;

    let stdin = io::stdin();
    let mut scanner = Scanner::new(BufReader::new(stdin.lock()));

    // scanf("%d %d %d", &x, &y, &z);
    //
    // The return value is ignored by the C program, exactly as here. Literal
    // spaces in the format match optional whitespace, which `%d`'s own leading
    // whitespace skip already covers. Conversions stop at the first failure,
    // leaving the remaining variables at their previous values (x = 0,
    // y = 123, z = 0).
    if let Some(v) = scanner.scan_int() {
        x = v;
        if let Some(v) = scanner.scan_int() {
            Y.store(v, Ordering::Relaxed);
            if let Some(v) = scanner.scan_int() {
                z = v;
            }
        }
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    let result = multi_stage(&mut out, x, z);
    let _ = write!(out, "Result: {}\n", result);
    let _ = out.flush();
}
