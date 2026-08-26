// Translation of c_src/src/main.c to Rust.
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
/// The bit-field widths truncate the stored values, so they are modeled by
/// masking on assignment.
struct Foo {
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

impl Foo {
    fn new(x: u32, y: u32, b: bool, z: i32) -> Foo {
        Foo {
            x: x & 0x3, // unsigned int x : 2
            y: y & 0x7, // unsigned int y : 3
            b,          // bool b : 1  (already 0 or 1)
            z,
        }
    }
}

fn print_foo(foo: &Foo, out: &mut dyn Write) {
    // printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    let _ = write!(
        out,
        "{} {} {} {}\n",
        foo.x,
        foo.y,
        if foo.b { 1 } else { 0 },
        foo.z
    );
}

fn driver(x: u32, y: u32, b: bool, z: i32, out: &mut dyn Write) {
    let foo = Foo::new(x, y, b, z);
    print_foo(&foo, out);
}

/// C `isspace` for the default locale.
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// A byte oriented reader over stdin with a single character pushback slot,
/// used to reproduce `scanf`'s consumption behavior (including the `ungetc`
/// of the first character that does not belong to the conversion).
struct Scanner<R: Read> {
    src: R,
    buf: [u8; 4096],
    pos: usize,
    len: usize,
    pushback: Option<u8>,
    at_eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(src: R) -> Scanner<R> {
        Scanner {
            src,
            buf: [0u8; 4096],
            pos: 0,
            len: 0,
            pushback: None,
            at_eof: false,
        }
    }

    /// `getc`: returns `None` on EOF (or read error, which C also treats as a
    /// stream failure).
    fn getc(&mut self) -> Option<u8> {
        if let Some(c) = self.pushback.take() {
            return Some(c);
        }
        if self.pos == self.len {
            if self.at_eof {
                return None;
            }
            loop {
                match self.src.read(&mut self.buf) {
                    Ok(0) => {
                        self.at_eof = true;
                        return None;
                    }
                    Ok(n) => {
                        self.pos = 0;
                        self.len = n;
                        break;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        self.at_eof = true;
                        return None;
                    }
                }
            }
        }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }

    /// `ungetc`: pushing back EOF is a no-op, as in C.
    fn ungetc(&mut self, c: Option<u8>) {
        if let Some(b) = c {
            self.pushback = Some(b);
        }
    }

    /// Shared scanning of an optionally signed decimal integer, as glibc's
    /// `scanf` does for `%d`/`%u`: skip leading whitespace, accept one sign,
    /// then collect decimal digits; the terminating character is pushed back.
    ///
    /// Returns `None` on input failure (EOF before any character) or matching
    /// failure (no digits), in which case the destination is left unmodified.
    /// Otherwise returns `(negative, magnitude, overflowed)`.
    fn scan_decimal(&mut self) -> Option<(bool, u64, bool)> {
        let mut c = self.getc();
        while let Some(b) = c {
            if is_c_space(b) {
                c = self.getc();
            } else {
                break;
            }
        }
        if c.is_none() {
            // Input failure: EOF while skipping whitespace.
            return None;
        }

        let mut negative = false;
        if let Some(b) = c {
            if b == b'-' || b == b'+' {
                negative = b == b'-';
                c = self.getc();
            }
        }

        let mut digits: usize = 0;
        let mut magnitude: u64 = 0;
        let mut overflow = false;
        while let Some(b) = c {
            if !b.is_ascii_digit() {
                break;
            }
            digits += 1;
            let d = u64::from(b - b'0');
            match magnitude.checked_mul(10).and_then(|m| m.checked_add(d)) {
                Some(v) => magnitude = v,
                None => overflow = true,
            }
            c = self.getc();
        }

        // The character that terminated the number is not part of it.
        self.ungetc(c);

        if digits == 0 {
            // Matching failure (e.g. a lone sign or a non-digit).
            return None;
        }

        Some((negative, magnitude, overflow))
    }

    /// `scanf("%u", &dst)` for a 32-bit `unsigned int`: glibc parses via
    /// `strtoul` (unsigned long, 64-bit here) and then narrows by truncation.
    fn scan_u32(&mut self) -> Option<u32> {
        let (negative, magnitude, overflow) = self.scan_decimal()?;
        let value: u64 = if overflow {
            u64::MAX // strtoul saturates to ULONG_MAX on overflow
        } else if negative {
            magnitude.wrapping_neg()
        } else {
            magnitude
        };
        Some(value as u32)
    }

    /// `scanf("%d", &dst)` for a 32-bit `int`: glibc parses via `strtol`
    /// (long, 64-bit here) and then narrows by truncation.
    fn scan_i32(&mut self) -> Option<i32> {
        let (negative, magnitude, overflow) = self.scan_decimal()?;
        let value: i64 = if overflow {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else {
            let signed: i128 = if negative {
                -(magnitude as i128)
            } else {
                magnitude as i128
            };
            if signed > i64::MAX as i128 {
                i64::MAX
            } else if signed < i64::MIN as i128 {
                i64::MIN
            } else {
                signed as i64
            }
        };
        Some(value as i32)
    }
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    // unsigned int x = 0, y = 0;
    // int b = 0, z = 0;
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    // Each scanf is attempted regardless of whether earlier ones succeeded;
    // on failure the destination keeps its previous value.
    if let Some(v) = scanner.scan_u32() {
        x = v;
    }
    if let Some(v) = scanner.scan_u32() {
        y = v;
    }
    if let Some(v) = scanner.scan_i32() {
        b = v;
    }
    if let Some(v) = scanner.scan_i32() {
        z = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(x, y, b != 0, z, &mut out); // driver(x, y, !!b, z)
    let _ = out.flush();
}
