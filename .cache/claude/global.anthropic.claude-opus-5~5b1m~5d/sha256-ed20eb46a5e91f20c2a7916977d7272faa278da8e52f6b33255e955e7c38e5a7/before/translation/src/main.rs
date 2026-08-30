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

/// Minimal stdin byte reader with one byte of push-back, used to emulate the
/// character-at-a-time consumption behavior of C's `scanf`.
struct Scanner {
    input: std::io::Stdin,
    pushed: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: std::io::stdin(),
            pushed: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushed.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.input.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    fn push_back(&mut self, b: u8) {
        self.pushed = Some(b);
    }

    /// Emulates a single `%d` conversion. Returns `Some(value)` on a successful
    /// conversion, `None` on input failure (EOF before any input) or matching
    /// failure (no digits), leaving the caller's variable untouched, exactly as
    /// C's `scanf` does.
    fn scan_i32(&mut self) -> Option<i32> {
        // Skip leading whitespace (as isspace() does).
        let mut b = loop {
            match self.next_byte() {
                Some(c) if matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c') => continue,
                Some(c) => break c,
                None => return None,
            }
        };

        // Optional sign.
        let mut negative = false;
        if b == b'+' || b == b'-' {
            negative = b == b'-';
            match self.next_byte() {
                Some(c) => b = c,
                None => return None,
            }
        }

        if !b.is_ascii_digit() {
            // Matching failure: the offending character stays in the stream.
            self.push_back(b);
            return None;
        }

        // Accumulate in a 64-bit value with saturation, then truncate to int,
        // mirroring glibc's strtol-based implementation on LP64 platforms.
        let mut acc: i64 = 0;
        loop {
            let digit = (b - b'0') as i64;
            acc = acc
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
                .unwrap_or(i64::MAX);
            match self.next_byte() {
                Some(c) if c.is_ascii_digit() => b = c,
                Some(c) => {
                    self.push_back(c);
                    break;
                }
                None => break,
            }
        }

        let value = if negative { acc.wrapping_neg() } else { acc };
        Some(value as i32)
    }
}

fn foo(mut x: i32, mut y: i32, out: &mut impl Write) {
    // States for the goto targets inside the loop body.
    const LABEL1: u8 = 1;
    const LABEL2: u8 = 2;

    'while_loop: while x > 0 || y > 0 {
        let _ = write!(out, "loop\n");

        let mut state = if x == 1 && y == 4 {
            LABEL2 // goto label2;
        } else {
            LABEL1
        };

        loop {
            if state == LABEL1 {
                // label1:
                if x > 0 {
                    let _ = write!(out, "x\n");
                    x -= 1;
                }
            }

            // label2:
            if y == 0 {
                continue 'while_loop;
            }
            let _ = write!(out, "y\n");
            y -= 1;
            if x < 3 {
                state = LABEL1; // goto label1;
                continue;
            }
            break;
        }
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    let mut scanner = Scanner::new();
    if let Some(v) = scanner.scan_i32() {
        x = v;
        if let Some(v) = scanner.scan_i32() {
            y = v;
        }
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    foo(x, y, &mut out);
    let _ = out.flush();
}
