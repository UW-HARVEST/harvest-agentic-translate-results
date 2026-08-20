// Rust translation of c_src/src/main.c
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

use std::io::{self, BufReader, Read, StdinLock, Write};

/// A byte-at-a-time reader over stdin that supports one byte of lookahead,
/// mirroring how C's `scanf` consumes only the characters it needs.
struct Scanner {
    inner: BufReader<StdinLock<'static>>,
    peeked: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            inner: BufReader::new(io::stdin().lock()),
            peeked: None,
            eof: false,
        }
    }

    /// Look at the next byte without consuming it. `None` means EOF (or a
    /// read error, which C's stdio also reports as a stream failure).
    fn peek(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
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
            Ok(_) => {
                self.peeked = Some(buf[0]);
                Some(buf[0])
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => self.peek(),
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    /// Consume the next byte. Every call site peeks first, so the byte is
    /// already cached; peeking again keeps this correct regardless.
    fn bump(&mut self) {
        if self.peeked.is_none() {
            let _ = self.peek();
        }
        self.peeked = None;
    }
}

/// C `isspace` for the default "C" locale (note: includes vertical tab, which
/// Rust's `u8::is_ascii_whitespace` omits).
fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | b'\x0c' | b'\r')
}

/// Emulates `scanf("%d", &x)`: leading whitespace (including newlines) is
/// skipped, then an optional sign followed by decimal digits is consumed.
/// Returns `None` on input failure or matching failure, in which case the
/// caller's variable is left untouched (as in C).
///
/// On overflow glibc's `%d` saturates at `LONG_MAX`/`LONG_MIN` (its internal
/// `strtol` behavior) and then truncates the result to `int`; that is
/// reproduced here.
fn scan_i32(sc: &mut Scanner) -> Option<i32> {
    // Skip leading whitespace; EOF here is an input failure.
    loop {
        match sc.peek() {
            Some(b) if c_isspace(b) => sc.bump(),
            Some(_) => break,
            None => return None,
        }
    }

    let mut negative = false;
    match sc.peek() {
        Some(b'+') => sc.bump(),
        Some(b'-') => {
            negative = true;
            sc.bump();
        }
        _ => {}
    }

    let mut saw_digit = false;
    let mut acc: i64 = 0;
    let mut overflowed = false;
    while let Some(b) = sc.peek() {
        if !b.is_ascii_digit() {
            break;
        }
        sc.bump();
        saw_digit = true;
        let digit = i64::from(b - b'0');
        if !overflowed {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflowed = true,
            }
        }
    }

    if !saw_digit {
        // Matching failure: no digits were converted.
        return None;
    }

    let wide: i64 = if overflowed {
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

    // Assignment to an `int *` truncates.
    Some(wide as i32)
}

fn print_hex(out: &mut impl Write, bytes: &[u8]) {
    for &b in bytes {
        let _ = write!(out, "{:02x}", b);
    }
    let _ = writeln!(out);
}

fn driver(out: &mut impl Write, x: i32) {
    // C reinterprets the bytes of the `int` in host byte order.
    print_hex(out, &x.to_ne_bytes());
}

fn main() {
    let mut sc = Scanner::new();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    if let Some(v) = scan_i32(&mut sc) {
        x = v;
    }
    driver(&mut out, x);

    let _ = out.flush();
}
