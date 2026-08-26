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

//! Faithful Rust translation of `c_src/src/main.c`.
//!
//! The C program is a CWE-190 (integer overflow) demonstration driver. It reads
//! a single integer with `scanf("%d", &x)` and then either exercises the buggy
//! (`bad`) or the fixed (`good`) code path. Byte-for-byte output compatibility
//! with the original (glibc / x86-64 Linux, where `char` is signed) is required,
//! including the signed-char-to-`int` promotion that makes `printf("%02x")`
//! print a sign extended 32-bit value.
//!
//! This module holds the implementation. It is shared verbatim by the `driver`
//! binary (`src/main.rs`) and by the `cdylib` C-ABI surface (`src/lib.rs`) via
//! `#[path]` module inclusion, so both artifacts run exactly the same code.

use std::io::{Read, Write};

/// `CHAR_MAX` for platforms where `char` is signed (x86-64 Linux).
pub const CHAR_MAX: i8 = i8::MAX;

/// Equivalent of C's `printLine`.
///
/// The C version guards against a NULL pointer; `Option` models that. The
/// payload is a raw byte slice rather than a `&str` because C's `printf("%s")`
/// (which GCC lowers to `puts`) copies bytes verbatim and places no UTF-8
/// requirement on them.
pub fn print_line(line: Option<&[u8]>) {
    if let Some(line) = line {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        // `printf("%s\n", line)` == the bytes followed by one newline.
        let _ = stdout.write_all(line);
        let _ = stdout.write_all(b"\n");
    }
}

/// Equivalent of C's `printHexCharLine`.
///
/// In C the `char` argument is promoted to `int` for the variadic call, and
/// `%02x` then reinterprets those 32 bits as an `unsigned int`. So a negative
/// char such as `-2` prints as `fffffffe`, not `fe`.
pub fn print_hex_char_line(char_hex: i8) {
    let promoted = char_hex as i32; // default argument promotion (sign extend)
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    // `%02x` reinterprets the promoted `int` as `unsigned int`.
    let _ = write!(stdout, "{:02x}\n", promoted as u32);
}

/// Equivalent of C's `bad`: `CHAR_MAX * 2` overflows the `char` result.
pub fn bad() {
    let data: i8;
    data = CHAR_MAX;
    if data > 0 {
        // `data * 2` is computed in `int` then truncated back into a `char`.
        let result: i8 = ((data as i32) * 2) as i8;
        print_hex_char_line(result);
    }
}

/// Equivalent of C's `goodG2B`: a small value that cannot overflow.
fn good_g2b() {
    let data: i8;
    data = 2;
    if data > 0 {
        let result: i8 = ((data as i32) * 2) as i8;
        print_hex_char_line(result);
    }
}

/// Equivalent of C's `goodB2G`: range-checks before doubling.
#[allow(unused_assignments)] // the dead store to `data` exists in the C source
fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        // CHAR_MAX / 2 is integer division => 63.
        if (data as i32) < (CHAR_MAX as i32) / 2 {
            let result: i8 = ((data as i32) * 2) as i8;
            print_hex_char_line(result);
        } else {
            print_line(Some(
                b"data value is too large to perform arithmetic safely.",
            ));
        }
    }
}

/// Equivalent of C's `good`.
pub fn good() {
    good_g2b();
    good_b2g();
}

/// `isspace()` in the C locale: horizontal tab, newline, vertical tab, form
/// feed, carriage return and space.
///
/// Note this deliberately does *not* use Rust's `u8::is_ascii_whitespace`,
/// which omits the vertical tab (0x0B) and would therefore fail to skip it.
fn c_isspace(b: u8) -> bool {
    matches!(b, 0x09..=0x0d | b' ')
}

/// Minimal stdin byte source that only consumes what `scanf` would consume.
struct Stdin {
    inner: std::io::Stdin,
    peeked: Option<u8>,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: std::io::stdin(),
            peeked: None,
        }
    }

    /// Reads one byte, or `None` at EOF / on error (matching `getc` -> EOF).
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(1) => return Some(buf[0]),
                Ok(_) => return None,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    /// Pushes a byte back, like `ungetc`.
    fn unread(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// Emulates glibc's `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion and `None` on a matching
/// failure or input failure (in which case the C code leaves `x` untouched).
///
/// glibc parses the digits with `strtol` semantics: out-of-range values saturate
/// to `LONG_MAX` / `LONG_MIN` (64-bit here) and the result is then truncated
/// into the `int` object, so e.g. `99999999999999999999` yields `-1` and
/// `4294967296` yields `0`.
fn scanf_i32(input: &mut Stdin) -> Option<i32> {
    // %d first skips any amount of leading whitespace.
    let mut cur = loop {
        match input.next_byte() {
            Some(b) if c_isspace(b) => continue,
            Some(b) => break b,
            None => return None, // input failure (EOF before any conversion)
        }
    };

    let mut negative = false;
    if cur == b'+' || cur == b'-' {
        negative = cur == b'-';
        match input.next_byte() {
            Some(b) => cur = b,
            None => return None, // sign with nothing after it: no conversion
        }
    }

    if !cur.is_ascii_digit() {
        input.unread(cur);
        return None; // matching failure
    }

    // Accumulate the magnitude, keeping it bounded so arbitrarily long digit
    // runs cannot overflow the accumulator itself.
    const CAP: u128 = (i64::MAX as u128) + 2; // anything >= this saturates
    let mut magnitude: u128 = 0;
    loop {
        magnitude = magnitude * 10 + u128::from(cur - b'0');
        if magnitude > CAP {
            magnitude = CAP;
        }
        match input.next_byte() {
            Some(b) if b.is_ascii_digit() => cur = b,
            Some(b) => {
                input.unread(b);
                break;
            }
            None => break,
        }
    }

    // strtol-style saturation into `long`, then truncation into `int`.
    let as_long: i64 = if negative {
        if magnitude > (i64::MAX as u128) + 1 {
            i64::MIN
        } else {
            (-(magnitude as i128)) as i64
        }
    } else if magnitude > i64::MAX as u128 {
        i64::MAX
    } else {
        magnitude as i64
    };

    Some(as_long as i32)
}

/// Equivalent of C's `main`.
pub fn program_main() -> i32 {
    let mut x: i32 = 0;
    let mut input = Stdin::new();
    if let Some(v) = scanf_i32(&mut input) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    // Match C's flush-at-exit behavior for stdout.
    let _ = std::io::stdout().flush();
    0
}
