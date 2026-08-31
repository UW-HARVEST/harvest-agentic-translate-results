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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C program declares a struct with bit-fields, reads four integers from
//! stdin with `scanf`, and prints the (truncated) bit-field values.

use std::io::{self, BufReader, Read, Write};

/// Mirror of the C `foo_t`:
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
/// Rust has no bit-fields, so the widths are emulated by masking on
/// construction, exactly as a C compiler does when storing into the field.
#[derive(Clone, Copy)]
struct Foo {
    /// `unsigned int x : 2` — only the low 2 bits are retained.
    x: u32,
    /// `unsigned int y : 3` — only the low 3 bits are retained.
    y: u32,
    /// `bool b : 1` — a 1-bit `_Bool` holds 0 or 1.
    b: bool,
    /// Plain `int z`.
    z: i32,
}

impl Foo {
    /// Equivalent of `foo_t foo = {.x = x, .y = y, .b = b, .z = z};`
    fn new(x: u32, y: u32, b: bool, z: i32) -> Foo {
        Foo {
            x: x & 0x3,
            y: y & 0x7,
            b,
            z,
        }
    }
}

/// `printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);`
///
/// The bit-fields undergo the default argument promotions to `int` before
/// being passed to the variadic call; the promoted values are small and
/// non-negative, so `%u`/`%d` print the same digits either way.
fn print_foo(foo: &Foo) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(
        out,
        "{} {} {} {}\n",
        foo.x,
        foo.y,
        i32::from(foo.b),
        foo.z
    );
    let _ = out.flush();
}

/// `void driver(unsigned int x, unsigned int y, bool b, int z)`
fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo::new(x, y, b, z);
    print_foo(&foo);
}

/// True for the characters C's `isspace` accepts in the default locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// A byte-oriented stdin reader with the single-character pushback that
/// `scanf` relies on when it has to reject a non-matching character.
struct Scanner<R: Read> {
    inner: BufReader<R>,
    pushback: Option<u8>,
}

impl<R: Read> Scanner<R> {
    fn new(reader: R) -> Scanner<R> {
        Scanner {
            inner: BufReader::new(reader),
            pushback: None,
        }
    }

    /// Reads the next byte, or `None` at end of input / on a read error.
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    /// Pushes one byte back, like `ungetc`. `scanf` never needs more than one.
    fn unget(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    /// Consumes leading whitespace, as every `scanf` numeric directive does.
    fn skip_whitespace(&mut self) {
        while let Some(b) = self.next_byte() {
            if !is_space(b) {
                self.unget(b);
                return;
            }
        }
    }

    /// Shared scan of `[+|-]digits` in base 10.
    ///
    /// Returns `(negative, magnitude, overflowed, matched)`. On a matching
    /// failure the offending byte is pushed back but an already-consumed sign
    /// character stays consumed — glibc only has room for one pushback, and
    /// the caller's variable is left untouched.
    fn scan_decimal(&mut self) -> Option<(bool, u64, bool)> {
        self.skip_whitespace();

        let mut negative = false;
        let mut first = match self.next_byte() {
            Some(b) => b,
            None => return None, // EOF before any conversion
        };
        if first == b'+' || first == b'-' {
            negative = first == b'-';
            first = match self.next_byte() {
                Some(b) => b,
                None => return None,
            };
        }
        if !first.is_ascii_digit() {
            self.unget(first);
            return None; // matching failure
        }

        let mut magnitude: u64 = 0;
        let mut overflowed = false;
        let mut cur = Some(first);
        while let Some(b) = cur {
            if !b.is_ascii_digit() {
                self.unget(b);
                break;
            }
            let digit = u64::from(b - b'0');
            match magnitude
                .checked_mul(10)
                .and_then(|m| m.checked_add(digit))
            {
                Some(m) => magnitude = m,
                None => overflowed = true,
            }
            cur = self.next_byte();
        }

        Some((negative, magnitude, overflowed))
    }

    /// `scanf("%u", &v)` for an `unsigned int` destination.
    ///
    /// glibc converts the collected digits with `strtoul` and assigns the
    /// result to the `unsigned int*`, so a signed input wraps and an
    /// out-of-range input saturates at `ULONG_MAX` before truncation.
    fn scan_u32(&mut self) -> Option<u32> {
        let (negative, magnitude, overflowed) = self.scan_decimal()?;
        let value: u64 = if overflowed {
            u64::MAX
        } else if negative {
            magnitude.wrapping_neg()
        } else {
            magnitude
        };
        Some(value as u32)
    }

    /// `scanf("%d", &v)` for an `int` destination.
    ///
    /// glibc uses `strtol`, which clamps to `LONG_MIN`/`LONG_MAX` on range
    /// errors; the 64-bit result is then truncated into the `int`.
    fn scan_i32(&mut self) -> Option<i32> {
        let (negative, magnitude, overflowed) = self.scan_decimal()?;
        let value: i64 = if negative {
            if overflowed || magnitude > (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                (magnitude as i64).wrapping_neg()
            }
        } else if overflowed || magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };
        Some(value as i32)
    }
}

fn main() {
    // Same initializers as the C code; a failed scanf leaves these unchanged.
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    let mut scanner = Scanner::new(io::stdin());

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

    driver(x, y, b != 0, z); // `!!b`
}
