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
//! The original source is written with C digraphs (`%:` = `#`, `<%` = `{`,
//! `%>` = `}`) and the `<iso646.h>` alternative operator spellings
//! (`bitor` = `|`, `compl` = `~`), so the computation is `x | ~y`.

use std::io::{self, Read, Write};

/// A byte-oriented stdin reader with single-byte "peek" (ungetc) semantics,
/// mirroring how C's `scanf` consumes characters from the stream.
struct Stdin {
    inner: io::Stdin,
    buf: [u8; 4096],
    pos: usize,
    len: usize,
    eof: bool,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: io::stdin(),
            buf: [0u8; 4096],
            pos: 0,
            len: 0,
            eof: false,
        }
    }

    /// Look at the next byte without consuming it. `None` means end-of-input.
    fn peek(&mut self) -> Option<u8> {
        while self.pos == self.len {
            if self.eof {
                return None;
            }
            match self.inner.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.len = n;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        Some(self.buf[self.pos])
    }

    /// Consume the byte previously returned by `peek`.
    fn bump(&mut self) {
        if self.pos < self.len {
            self.pos += 1;
        }
    }
}

/// True for the characters C's `isspace` treats as whitespace in the C locale.
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Equivalent of `scanf("%d", &out)`.
///
/// Returns `Some(value)` on a successful conversion; `None` on matching
/// failure or end-of-input, in which case the caller's variable is left
/// untouched (exactly as C does). Leading whitespace is skipped, including
/// across newlines. Consumed input follows glibc: an optional sign is
/// consumed even when no digits follow, and only the single offending
/// non-digit byte is pushed back.
fn scanf_i32(input: &mut Stdin) -> Option<i32> {
    // Skip leading whitespace (spans newlines, like scanf).
    loop {
        match input.peek() {
            Some(c) if is_c_space(c) => input.bump(),
            Some(_) => break,
            None => return None, // input failure (EOF)
        }
    }

    // Optional sign.
    let mut negative = false;
    match input.peek() {
        Some(b'-') => {
            negative = true;
            input.bump();
        }
        Some(b'+') => {
            input.bump();
        }
        _ => {}
    }

    // Digit sequence; base 10 only for %d.
    let mut magnitude: u64 = 0;
    let mut saw_digit = false;
    let mut overflowed = false;
    while let Some(c) = input.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        input.bump();
        saw_digit = true;
        let d = u64::from(c - b'0');
        if !overflowed {
            match magnitude.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => magnitude = v,
                None => overflowed = true,
            }
        }
    }

    if !saw_digit {
        // Matching failure: the offending byte stays in the stream.
        return None;
    }

    // glibc converts the digit string with strtol (clamping at LONG_MIN /
    // LONG_MAX on a 64-bit target) and then narrows the result to `int`.
    const NEG_LIMIT: u64 = 1u64 << 63; // magnitude of LONG_MIN
    let as_long: i64 = if negative {
        if overflowed || magnitude > NEG_LIMIT {
            i64::MIN
        } else {
            (magnitude as i128).wrapping_neg() as i64
        }
    } else if overflowed || magnitude > i64::MAX as u64 {
        i64::MAX
    } else {
        magnitude as i64
    };

    Some(as_long as i32)
}

/// `void driver(int x, int y)` from the original C.
fn driver(x: i32, y: i32, out: &mut impl Write) {
    let result = x | !y;
    // printf("%d", result);
    let _ = write!(out, "{}", result);
    // puts("");
    let _ = writeln!(out);
}

fn main() {
    let mut input = Stdin::new();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut x: i32 = 0;
    let mut y: i32 = 0;

    // scanf("%d", &x); return value ignored, as in the C.
    if let Some(v) = scanf_i32(&mut input) {
        x = v;
    }
    // scanf("%d", &y);
    if let Some(v) = scanf_i32(&mut input) {
        y = v;
    }

    driver(x, y, &mut out);

    let _ = out.flush();
}
