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
//! The original C is a CWE-562 (Return of Stack Variable Address) test case.
//! Its behavior is reproduced as-is; no defects are corrected.

// The trailing-newline `write!` and the explicit `match` on `next_byte()` both
// mirror the C control flow one-to-one; clippy's terser alternatives would
// obscure that correspondence.
#![allow(clippy::write_with_newline, clippy::question_mark)]

use std::io::{Read, Write};

/// `void printLine(const char *line)`
///
/// The C function guards against a NULL pointer before printing, so a NULL
/// argument produces no output at all (not even the trailing newline).
/// `Option<&str>` models the nullable `const char *`.
fn print_line<W: Write>(out: &mut W, line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        let _ = write!(out, "{}\n", line);
    }
}

/// `static char *helperBad()`
///
/// The C original declares `char charString[] = "helperBad string";` as an
/// automatic (stack) array and returns its address, which dangles the moment
/// the function returns. GCC diagnoses this (`-Wreturn-local-addr`) and
/// substitutes a null pointer for the return value -- the generated assembly
/// for `helperBad` ends in `movl $0, %eax; ret`, discarding the buffer
/// entirely. The observable consequence is that `printLine` takes its NULL
/// branch and `bad()` emits nothing.
///
/// This bug is preserved deliberately: returning `None` reproduces the
/// reference executable's output byte for byte without invoking undefined
/// behavior in Rust.
fn helper_bad() -> Option<&'static str> {
    // The stack array is built and then thrown away, exactly as compiled.
    let _char_string = *b"helperBad string\0";
    None
}

/// `void bad()`
fn bad<W: Write>(out: &mut W) {
    print_line(out, helper_bad());
}

/// `static char *helperGood1()`
///
/// The C original uses `static char charString[]`, giving the buffer static
/// storage duration, so returning its address is well defined.
fn helper_good1() -> Option<&'static str> {
    static CHAR_STRING: &str = "helperGood1 string";
    Some(CHAR_STRING)
}

/// `void good()`
fn good<W: Write>(out: &mut W) {
    print_line(out, helper_good1());
}

/// `isspace()` in the C locale.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// A byte reader with a single-byte pushback, mirroring how the C standard
/// library's `ungetc`-style lookahead lets `scanf` stop at (and put back) the
/// first character that cannot belong to the current conversion.
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

    /// Read one byte, or `None` at EOF / on a read error.
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

    /// `scanf("%d", &x)` for a single conversion.
    ///
    /// Returns `Some(value)` when the conversion succeeds; returns `None` on
    /// either an input failure (EOF before any non-whitespace) or a matching
    /// failure (no digits), in both of which cases C leaves the destination
    /// object untouched.
    ///
    /// Leading whitespace is skipped without regard to line boundaries, so the
    /// scan happily reads across newlines just as `scanf` does.
    fn scan_int(&mut self) -> Option<i32> {
        // Skip leading whitespace, including newlines.
        let mut b = loop {
            match self.next_byte() {
                None => return None, // input failure
                Some(b) if is_space(b) => continue,
                Some(b) => break b,
            }
        };

        // Optional sign.
        let mut negative = false;
        if b == b'+' || b == b'-' {
            negative = b == b'-';
            match self.next_byte() {
                None => return None, // sign then EOF: matching failure
                Some(nb) => b = nb,
            }
        }

        // At least one digit is required.
        if !b.is_ascii_digit() {
            self.unread(b);
            return None; // matching failure
        }

        // Accumulate the magnitude the way strtol does: saturate at the
        // long range on overflow, then let the assignment to `int` truncate.
        let mut magnitude: u64 = 0;
        let mut overflowed = false;
        loop {
            let digit = u64::from(b - b'0');
            match magnitude
                .checked_mul(10)
                .and_then(|acc| acc.checked_add(digit))
            {
                Some(acc) => magnitude = acc,
                None => overflowed = true,
            }
            match self.next_byte() {
                None => break,
                Some(nb) if nb.is_ascii_digit() => b = nb,
                Some(nb) => {
                    self.unread(nb);
                    break;
                }
            }
        }

        // strtol clamps to LONG_MAX / LONG_MIN.
        let long_min_magnitude = 1u64 << 63; // |LONG_MIN|
        let as_long: i64 = if negative {
            if overflowed || magnitude > long_min_magnitude {
                i64::MIN
            } else {
                (-(magnitude as i128)) as i64
            }
        } else if overflowed || magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };

        // `int x` receives the value; the narrowing conversion truncates.
        Some(as_long as i32)
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // int x = 0;
    let mut x: i32 = 0;
    // scanf("%d", &x);  -- x keeps its initial 0 if the conversion fails.
    if let Some(value) = scanner.scan_int() {
        x = value;
    }

    if x != 0 {
        good(&mut out);
    } else {
        bad(&mut out);
    }

    let _ = out.flush();

    // return 0;
}
