// Rust translation of c_src/src/main.c
//
// Original copyright header from the C source:
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

/// Mirrors the C struct with bit-fields:
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
/// Values assigned to the bit-fields are truncated to their declared widths.
struct FooT {
    x: u32, // 2 bits
    y: u32, // 3 bits
    b: bool, // 1 bit
    z: i32,
}

impl FooT {
    fn new(x: u32, y: u32, b: bool, z: i32) -> FooT {
        FooT {
            x: x & 0x3,
            y: y & 0x7,
            b, // storing 0 or 1 into a 1-bit bool bit-field is lossless
            z,
        }
    }
}

fn print_foo(foo: &FooT) {
    // printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    let out = std::io::stdout();
    let mut out = out.lock();
    let _ = write!(
        out,
        "{} {} {} {}\n",
        foo.x,
        foo.y,
        if foo.b { 1 } else { 0 },
        foo.z
    );
    let _ = out.flush();
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = FooT::new(x, y, b, z);
    print_foo(&foo);
}

/// A byte-at-a-time reader over stdin with a one-byte pushback buffer, so that
/// consecutive `scanf` calls behave like C's buffered stream (one character of
/// lookahead is put back with `ungetc`).
struct Scanner {
    inner: std::io::Stdin,
    pushback: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Scanner {
        Scanner {
            inner: std::io::stdin(),
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
        match self.inner.read(&mut buf) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => Some(buf[0]),
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn unread(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    fn skip_ws(&mut self) {
        loop {
            match self.next_byte() {
                Some(b) if is_c_space(b) => continue,
                Some(b) => {
                    self.unread(b);
                    return;
                }
                None => return,
            }
        }
    }

    /// Scans an optionally signed decimal integer the way glibc's `scanf`
    /// does: skip whitespace, accept an optional sign, then decimal digits.
    /// Returns `None` on matching failure / input failure (in which case the
    /// destination is left untouched, as in C).
    ///
    /// The returned pair is (negative, magnitude), where the magnitude
    /// saturates at u64::MAX, matching glibc's use of strtoul/strtol.
    fn scan_int_parts(&mut self) -> Option<(bool, u64)> {
        self.skip_ws();
        let mut negative = false;
        let first = self.next_byte()?;
        let mut cur = match first {
            b'+' => self.next_byte(),
            b'-' => {
                negative = true;
                self.next_byte()
            }
            other => Some(other),
        };

        let mut digits = 0usize;
        let mut mag: u64 = 0;
        let mut overflow = false;
        loop {
            match cur {
                Some(c) if c.is_ascii_digit() => {
                    digits += 1;
                    let d = u64::from(c - b'0');
                    match mag.checked_mul(10).and_then(|m| m.checked_add(d)) {
                        Some(m) => mag = m,
                        None => overflow = true,
                    }
                    cur = self.next_byte();
                }
                Some(c) => {
                    self.unread(c);
                    break;
                }
                None => break,
            }
        }

        if digits == 0 {
            // Matching failure: the offending character has already been
            // pushed back (or we hit EOF).
            return None;
        }
        if overflow {
            mag = u64::MAX;
        }
        Some((negative, mag))
    }

    /// scanf("%u", &dst)
    fn scan_u(&mut self, dst: &mut u32) -> bool {
        match self.scan_int_parts() {
            Some((negative, mag)) => {
                // glibc parses via strtoul: on overflow the result is
                // ULONG_MAX; otherwise a leading '-' negates modulo 2^64.
                let val: u64 = if mag == u64::MAX {
                    u64::MAX
                } else if negative {
                    mag.wrapping_neg()
                } else {
                    mag
                };
                *dst = val as u32;
                true
            }
            None => false,
        }
    }

    /// scanf("%d", &dst)
    fn scan_d(&mut self, dst: &mut i32) -> bool {
        match self.scan_int_parts() {
            Some((negative, mag)) => {
                // glibc parses via strtol: saturates at LONG_MIN/LONG_MAX.
                let val: i64 = if negative {
                    if mag > (i64::MAX as u64) + 1 {
                        i64::MIN
                    } else if mag == (i64::MAX as u64) + 1 {
                        i64::MIN
                    } else {
                        -(mag as i64)
                    }
                } else if mag > i64::MAX as u64 {
                    i64::MAX
                } else {
                    mag as i64
                };
                *dst = val as i32;
                true
            }
            None => false,
        }
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
}

fn main() {
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    let mut sc = Scanner::new();
    let _ = sc.scan_u(&mut x);
    let _ = sc.scan_u(&mut y);
    let _ = sc.scan_d(&mut b);
    let _ = sc.scan_d(&mut z);

    driver(x, y, b != 0, z);
}
