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

use std::io::{self, Read, Write};

/// A byte-oriented view of stdin with a single-byte pushback slot, mirroring
/// the `getc`/`ungetc` pair that C's `scanf` uses internally.
struct CStdin {
    inner: io::Stdin,
    pushback: Option<u8>,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            inner: io::stdin(),
            pushback: None,
        }
    }

    /// Reads one byte, or `None` at end of input (or on a read error, which C
    /// also surfaces as a failed `getc`).
    fn getc(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => None,
        }
    }

    fn ungetc(&mut self, b: u8) {
        self.pushback = Some(b);
    }
}

/// Matches the "C locale" `isspace` set that `scanf` skips over: space, tab,
/// newline, vertical tab, form feed and carriage return. Because whitespace
/// skipping does not stop at a newline, `%d` happily reads across line breaks.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", out)`.
///
/// Returns the number of successfully assigned items (1), 0 on a matching
/// failure, or -1 (EOF) when input ends before any conversion begins. On
/// failure `*out` is left untouched, exactly as in C.
///
/// Out-of-range values follow glibc, which parses `%d` through `strtol`:
/// the value saturates at `long` range and is then truncated to `int`.
fn scanf_d(input: &mut CStdin, out: &mut i32) -> i32 {
    // 1. Skip leading whitespace.
    let mut c = loop {
        match input.getc() {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return -1, // EOF before any conversion
        }
    };

    // 2. Optional sign.
    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.getc() {
            Some(b) => c = b,
            None => return -1,
        }
    }

    // 3. At least one digit is required.
    if !c.is_ascii_digit() {
        input.ungetc(c);
        return 0; // matching failure
    }

    // 4. Accumulate digits, saturating like strtol does.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = (c - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        match input.getc() {
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                input.ungetc(b);
                break;
            }
            None => break,
        }
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    // glibc stores the `long` result straight into the `int` argument.
    *out = value as i32;
    1
}

fn driver(x: i32) {
    // Signed overflow wraps here to match the behavior of the compiled C.
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    let mut stdout = io::stdout();
    let _ = write!(stdout, "{}\n", y);
    let _ = stdout.flush();
}

fn main() {
    let mut x: i32 = 0;
    let mut input = CStdin::new();
    // The C code ignores scanf's return value; on failure `x` stays 0.
    let _ = scanf_d(&mut input, &mut x);
    driver(x);
}
