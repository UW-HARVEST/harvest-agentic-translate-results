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

//! Rust translation of the original C `driver`.
//!
//! The translation intentionally reproduces the original behaviour bug-for-bug,
//! including the signed-char arithmetic overflow in `bad()` and the resulting
//! sign-extended `%02x` output.

use std::io::{Read, Write};

/// `char` on the reference platform (Linux / x86-64) is **signed**, so the C
/// `char` type maps onto `i8` and `CHAR_MAX` is 127.
const CHAR_MAX: i8 = i8::MAX;

/// C: `void printLine(const char * line)`
///
/// The C function guards against a NULL pointer, so the parameter is modelled
/// as an `Option<&str>`; `None` stands in for NULL and produces no output.
fn print_line<W: Write>(out: &mut W, line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        let _ = write!(out, "{}\n", line);
    }
}

/// C: `void printHexCharLine(char charHex)`
///
/// `printf("%02x\n", charHex)` promotes the `char` argument to `int` and then
/// `%x` reinterprets those bits as `unsigned int`. A negative `char` therefore
/// prints as a sign-extended 32-bit value (e.g. -2 prints as `fffffffe`), and
/// the `02` minimum field width has no effect in that case.
fn print_hex_char_line<W: Write>(out: &mut W, char_hex: i8) {
    let promoted = char_hex as i32; // default argument promotion to int
    let _ = write!(out, "{:02x}\n", promoted as u32);
}

/// C: `void bad()`
///
/// `data * 2` is evaluated in `int` (127 * 2 == 254) and then narrowed back to
/// `char`, which wraps to -2 on a two's-complement platform. This overflow is
/// the original defect and is preserved verbatim.
fn bad<W: Write>(out: &mut W) {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        let result: i8 = (data as i32 * 2) as i8;
        print_hex_char_line(out, result);
    }
}

/// C: `static void goodG2B()`
fn good_g2b<W: Write>(out: &mut W) {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = (data as i32 * 2) as i8;
        print_hex_char_line(out, result);
    }
}

/// C: `static void goodB2G()`
///
/// The initial `data = ' '` assignment is immediately overwritten by
/// `data = CHAR_MAX`, exactly as in the original.
fn good_b2g<W: Write>(out: &mut W) {
    #[allow(unused_assignments)]
    let mut data: i8 = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        // Integer division, matching C: CHAR_MAX / 2 == 63.
        if data < (CHAR_MAX / 2) {
            let result: i8 = (data as i32 * 2) as i8;
            print_hex_char_line(out, result);
        } else {
            print_line(
                out,
                Some("data value is too large to perform arithmetic safely."),
            );
        }
    }
}

/// C: `void good()`
fn good<W: Write>(out: &mut W) {
    good_g2b(out);
    good_b2g(out);
}

/// Byte-oriented view of stdin with a single byte of push-back, mirroring the
/// way `scanf` consumes the stream (it reads across newlines and leaves any
/// unconsumed byte available to later reads).
struct Scanner<R: Read> {
    inner: R,
    peeked: Option<u8>,
}

impl<R: Read> Scanner<R> {
    fn new(inner: R) -> Self {
        Scanner {
            inner,
            peeked: None,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    fn unread(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Equivalent of `scanf("%d", &out)`.
    ///
    /// Returns `true` when a value was successfully converted and stored. On a
    /// matching failure or end of input the target is left untouched, which is
    /// why `main` keeps `x == 0` in those cases.
    ///
    /// Out-of-range input follows the glibc implementation: the value is
    /// accumulated as a `long`, saturated at `LONG_MAX`/`LONG_MIN`, and then
    /// truncated on assignment to `int`.
    fn scan_int(&mut self, out: &mut i32) -> bool {
        // Skip leading whitespace, exactly as the %d conversion does.
        let mut b = loop {
            match self.next_byte() {
                None => return false, // EOF before any conversion
                Some(c) if is_c_space(c) => continue,
                Some(c) => break c,
            }
        };

        let mut negative = false;
        if b == b'+' || b == b'-' {
            negative = b == b'-';
            match self.next_byte() {
                None => return false, // sign with no digits: matching failure
                Some(c) => b = c,
            }
        }

        if !b.is_ascii_digit() {
            self.unread(b);
            return false; // matching failure
        }

        // Accumulate with saturation at the platform `long` bounds.
        let mut acc: i128 = 0;
        let mut saturated = false;
        loop {
            if !saturated {
                acc = acc * 10 + i128::from(b - b'0');
                if acc > i128::from(i64::MAX) {
                    saturated = true;
                }
            }
            match self.next_byte() {
                None => break,
                Some(c) if c.is_ascii_digit() => b = c,
                Some(c) => {
                    self.unread(c);
                    break;
                }
            }
        }

        let as_long: i64 = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            // -acc always fits: acc <= i64::MAX here.
            -(acc as i64)
        } else {
            acc as i64
        };

        // Truncating store into an `int`.
        *out = as_long as i32;
        true
    }
}

/// C's `isspace` for the default "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut scanner = Scanner::new(stdin.lock());
    let mut out = std::io::BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    // Return value ignored, just like the original; x stays 0 on failure.
    let _ = scanner.scan_int(&mut x);

    if x != 0 {
        good(&mut out);
    } else {
        bad(&mut out);
    }

    let _ = out.flush();
}
