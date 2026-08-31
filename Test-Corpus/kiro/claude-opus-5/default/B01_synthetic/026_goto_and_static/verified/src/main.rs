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

use std::io::{Read, Write};
use std::sync::atomic::{AtomicI32, Ordering};

/// Mirrors `static int y = 123;` in the C source. `scanf` writes directly into
/// this object, so if the conversion for the second field never happens the
/// value observed by `multi_stage` is still the initializer, 123.
static Y: AtomicI32 = AtomicI32::new(123);

fn get_y() -> i32 {
    Y.load(Ordering::Relaxed)
}

fn set_y(v: i32) {
    Y.store(v, Ordering::Relaxed);
}

/// A byte-oriented view of stdin with a single byte of pushback, which is all
/// `scanf`'s `%d` conversion needs (it un-reads the first non-matching byte).
struct CStdin {
    inner: std::io::Stdin,
    pushback: Option<u8>,
    eof: bool,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            inner: std::io::stdin(),
            pushback: None,
            eof: false,
        }
    }

    fn getc(&mut self) -> Option<u8> {
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
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn ungetc(&mut self, b: u8) {
        self.pushback = Some(b);
    }
}

/// True for the bytes that C's `isspace` accepts in the "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// One `%d` conversion.
///
/// Returns `Some(value)` on a successful conversion, `None` on either a
/// matching failure or an input failure. Like `scanf`, leading whitespace
/// (newlines included) is skipped, an optional sign is accepted, and the first
/// byte that cannot be part of the number is pushed back.
///
/// glibc collects the digits and hands them to `strtol`, which saturates at
/// `LONG_MAX`/`LONG_MIN` on overflow; the result is then stored through an
/// `int *`, truncating it. The saturate-then-truncate below reproduces that.
fn scan_int(input: &mut CStdin) -> Option<i32> {
    // Skip whitespace.
    let mut b = loop {
        match input.getc() {
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
            None => return None, // input failure
        }
    };

    let mut negative = false;
    if b == b'+' || b == b'-' {
        negative = b == b'-';
        match input.getc() {
            Some(c) => b = c,
            None => return None, // sign with nothing after it: matching failure
        }
    }

    if !b.is_ascii_digit() {
        input.ungetc(b);
        return None; // matching failure
    }

    let mut acc: i64 = 0;
    let mut overflow = false;
    loop {
        let digit = i64::from(b - b'0');
        if !overflow {
            let step = acc
                .checked_mul(10)
                .and_then(|v| if negative { v.checked_sub(digit) } else { v.checked_add(digit) });
            match step {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        match input.getc() {
            Some(c) if c.is_ascii_digit() => b = c,
            Some(c) => {
                input.ungetc(c);
                break;
            }
            None => break,
        }
    }

    if overflow {
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    Some(acc as i32)
}

/// `scanf("%d %d %d", &x, &y, &z)`.
///
/// Whitespace directives in the format match any run of whitespace (including
/// none), and `%d` skips leading whitespace on its own, so the three
/// conversions are simply attempted in order. Assignment stops at the first
/// failure, leaving later variables untouched.
fn scanf_three(input: &mut CStdin, x: &mut i32, z: &mut i32) {
    match scan_int(input) {
        Some(v) => *x = v,
        None => return,
    }
    match scan_int(input) {
        Some(v) => set_y(v),
        None => return,
    }
    if let Some(v) = scan_int(input) {
        *z = v;
    }
}

fn multi_stage(out: &mut impl Write, x: i32, z: i32) -> i32 {
    let result;

    // Errors are reported in exactly this order; the C code funnels every
    // failure through a `goto fail` that also prints "Operation failed".
    if x != 1 {
        let _ = write!(out, "Error: x != 1\n");
        result = 1;
    } else if get_y() != 2 {
        let _ = write!(out, "Error: x == 1 but y != 2\n");
        result = 2;
    } else if z != 3 {
        let _ = write!(out, "Error: x == 1 and y == 2, but z != 3\n");
        result = 3;
    } else {
        let _ = write!(out, "Ok!\n");
        return 0;
    }

    // fail:
    let _ = write!(out, "Operation failed\n");
    result
}

fn main() {
    let mut input = CStdin::new();
    let stdout = std::io::stdout();
    // C's stdout is fully buffered when redirected; a single buffered writer
    // flushed at exit keeps the byte stream identical either way.
    let mut out = std::io::BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    let mut z: i32 = 0;
    scanf_three(&mut input, &mut x, &mut z);

    let result = multi_stage(&mut out, x, z);
    let _ = write!(out, "Result: {}\n", result);

    let _ = out.flush();
}
