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

//! Rust translation of c_src/src/main.c (CWE-562: return of stack variable address).
//!
//! Observable behavior is preserved exactly, including the buggy `bad()` path.

use std::io::{Read, Write};

/// C: void printLine(const char *line) { if (line != NULL) printf("%s\n", line); }
///
/// A NULL `const char *` maps to `None`.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = write!(out, "{}\n", line);
    }
}

/// C: static char *helperBad() { char charString[] = "helperBad string"; return charString; }
///
/// This returns the address of a stack-local array, which is dangling as soon as
/// the function returns (CWE-562). GCC diagnoses this
/// (`-Wreturn-local-addr`) and codegens the return value as a null pointer, so
/// the caller observes NULL rather than the string contents. The bug is
/// deliberately NOT fixed here: `None` reproduces the observed NULL return, so
/// `printLine` prints nothing.
fn helper_bad() -> Option<&'static str> {
    None
}

/// C: void bad() { printLine(helperBad()); }
fn bad() {
    print_line(helper_bad());
}

/// C: static char *helperGood1() { static char charString[] = "helperGood1 string"; return charString; }
///
/// The array has static storage duration, so the returned pointer stays valid.
fn helper_good1() -> Option<&'static str> {
    Some("helperGood1 string")
}

/// C: void good() { printLine(helperGood1()); }
fn good() {
    print_line(helper_good1());
}

/// Byte-at-a-time reader over stdin, so only the bytes that `scanf` would
/// consume are consumed (scanf skips leading whitespace, including newlines,
/// and stops at the first character that cannot extend the conversion).
struct Scanner {
    bytes: std::io::Bytes<std::io::Stdin>,
    peeked: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            bytes: std::io::stdin().bytes(),
            peeked: None,
            eof: false,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        if self.peeked.is_none() && !self.eof {
            match self.bytes.next() {
                Some(Ok(b)) => self.peeked = Some(b),
                _ => self.eof = true,
            }
        }
        self.peeked
    }

    fn take(&mut self) -> Option<u8> {
        let b = self.peek();
        self.peeked = None;
        b
    }

    /// C `isspace` for the "C" locale.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Equivalent of `scanf("%d", &x)`: returns `Some(value)` on a successful
    /// conversion, `None` on matching failure or EOF (in which case the C code
    /// leaves `x` at its initial value).
    ///
    /// glibc converts the digit run with `strtol` semantics: out-of-range values
    /// saturate at LONG_MIN/LONG_MAX and the result is then truncated to `int`.
    fn scan_int(&mut self) -> Option<i32> {
        while let Some(b) = self.peek() {
            if Self::is_space(b) {
                self.take();
            } else {
                break;
            }
        }

        let mut negative = false;
        match self.peek() {
            Some(b'-') => {
                negative = true;
                self.take();
            }
            Some(b'+') => {
                self.take();
            }
            _ => {}
        }

        let mut saw_digit = false;
        let mut acc: i64 = 0;
        let mut overflow = false;
        while let Some(b) = self.peek() {
            if !b.is_ascii_digit() {
                break;
            }
            self.take();
            saw_digit = true;
            let digit = i64::from(b - b'0');
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }

        if !saw_digit {
            // Matching failure (or EOF before any digit): no assignment is made.
            return None;
        }

        let value: i64 = if overflow {
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

        // (int) truncation of the long result.
        Some(value as i32)
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut scanner = Scanner::new();
    if let Some(v) = scanner.scan_int() {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = std::io::stdout().flush();
}
