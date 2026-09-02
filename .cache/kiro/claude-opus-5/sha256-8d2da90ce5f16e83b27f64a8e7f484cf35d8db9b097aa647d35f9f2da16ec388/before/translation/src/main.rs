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

use std::io::{Read, Write};

/// Mirrors the C struct:
///
/// ```c
/// typedef struct {
///     unsigned int x : 2;
///     unsigned int y : 3;
///     bool b : 1;
///     int z;
/// } foo_t;
/// ```
///
/// Bit-field stores truncate the assigned value to the declared width, so the
/// masks below reproduce what the C compiler emits on assignment.
struct FooT {
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

impl FooT {
    fn new(x: u32, y: u32, b: bool, z: i32) -> Self {
        FooT {
            x: x & 0x3, // 2-bit unsigned field
            y: y & 0x7, // 3-bit unsigned field
            b,          // 1-bit bool field: already 0 or 1
            z,
        }
    }
}

/// `printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);`
///
/// `foo->b` is a `bool` bit-field, which the default argument promotions turn
/// into `int` 0 or 1.
fn print_foo(foo: &FooT, out: &mut impl Write) {
    let _ = write!(
        out,
        "{} {} {} {}\n",
        foo.x,
        foo.y,
        if foo.b { 1 } else { 0 },
        foo.z
    );
}

fn driver(x: u32, y: u32, b: bool, z: i32, out: &mut impl Write) {
    let foo = FooT::new(x, y, b, z);
    print_foo(&foo, out);
}

/// A byte-oriented view of stdin that emulates C `scanf` conversion behaviour.
///
/// The C program only ever reads with `%u`/`%d`, which skip leading whitespace
/// (including newlines) and stop at the first character that cannot be part of
/// the number, leaving that character unread. On a matching or input failure the
/// destination variable is left untouched, exactly as in C.
struct Scanner {
    buf: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn from_stdin() -> Self {
        let mut buf = Vec::new();
        // A read error behaves like end-of-input for the conversions below.
        let _ = std::io::stdin().read_to_end(&mut buf);
        Scanner { buf, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    /// `isspace()` in the C locale.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    /// Collects an optional sign plus a run of decimal digits, consuming them.
    /// Returns `None` on input failure (EOF) or matching failure (no digits),
    /// mirroring glibc, which consumes the sign but leaves the offending
    /// non-digit character in the stream.
    fn scan_digits(&mut self) -> Option<(bool, Vec<u8>)> {
        self.skip_whitespace();
        if self.peek().is_none() {
            return None; // input failure
        }
        let mut negative = false;
        match self.peek() {
            Some(b'-') => {
                negative = true;
                self.pos += 1;
            }
            Some(b'+') => {
                self.pos += 1;
            }
            _ => {}
        }
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return None; // matching failure
        }
        Some((negative, self.buf[start..self.pos].to_vec()))
    }

    /// `scanf("%d", dst)`; leaves `dst` unchanged on failure.
    ///
    /// glibc hands the collected text to `strtol`, which saturates at
    /// `LONG_MAX`/`LONG_MIN` (64-bit here) before the result is stored into an
    /// `int`, i.e. truncated to the low 32 bits.
    fn scan_i32(&mut self, dst: &mut i32) {
        if let Some((negative, digits)) = self.scan_digits() {
            let mut acc: i64 = 0;
            let mut saturated = false;
            for d in &digits {
                let d = i64::from(d - b'0');
                match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => acc = v,
                    None => {
                        saturated = true;
                        break;
                    }
                }
            }
            let value: i64 = if saturated {
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
            *dst = value as i32;
        }
    }

    /// `scanf("%u", dst)`; leaves `dst` unchanged on failure.
    ///
    /// glibc uses `strtoul`, which accepts a leading sign and negates modulo
    /// `ULONG_MAX + 1`, saturating at `ULONG_MAX` on overflow; the result is
    /// then stored into an `unsigned int`, i.e. truncated to 32 bits.
    fn scan_u32(&mut self, dst: &mut u32) {
        if let Some((negative, digits)) = self.scan_digits() {
            let mut acc: u64 = 0;
            let mut saturated = false;
            for d in &digits {
                let d = u64::from(d - b'0');
                match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => acc = v,
                    None => {
                        saturated = true;
                        break;
                    }
                }
            }
            let value: u64 = if saturated {
                u64::MAX
            } else if negative {
                acc.wrapping_neg()
            } else {
                acc
            };
            *dst = value as u32;
        }
    }
}

fn main() {
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    let mut scanner = Scanner::from_stdin();
    scanner.scan_u32(&mut x);
    scanner.scan_u32(&mut y);
    scanner.scan_i32(&mut b);
    scanner.scan_i32(&mut z);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    driver(x, y, b != 0, z, &mut out); // !!b
    let _ = out.flush();
}
