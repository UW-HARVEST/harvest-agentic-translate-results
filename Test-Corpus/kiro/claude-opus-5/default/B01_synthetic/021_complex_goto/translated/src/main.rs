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

//! Rust translation of c_src/src/main.c.
//!
//! The C source uses `goto` labels inside a `while` loop, so the control flow is
//! reproduced here with an explicit state machine whose states correspond to the
//! jump targets in the original program.

use std::io::{Read, Write};

/// Byte-oriented stdin reader with one byte of push-back, mirroring the way C's
/// `scanf` consumes only the characters it needs and leaves the rest in the
/// stream.
struct Scanner {
    input: std::io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: std::io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    /// Reads the next byte, or `None` at end of input.
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.input.read(&mut buf) {
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

    /// Pushes a byte back so the next read returns it again (equivalent to
    /// `ungetc` on the stdio stream).
    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Emulates a single `%d` conversion: skip leading whitespace, then an
    /// optional sign followed by decimal digits. Returns `None` when the
    /// conversion fails (matching failure or EOF), in which case the caller
    /// leaves its variable untouched, exactly as `scanf` does.
    fn scan_i32(&mut self) -> Option<i32> {
        // %d skips any amount of leading whitespace, including newlines.
        let mut c = loop {
            match self.next_byte() {
                Some(b) if b.is_ascii_whitespace() => continue,
                Some(b) => break b,
                None => return None,
            }
        };

        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            match self.next_byte() {
                Some(b) => c = b,
                None => return None,
            }
        }

        if !c.is_ascii_digit() {
            // Matching failure: the offending character stays in the stream.
            self.unget(c);
            return None;
        }

        // Accumulate in i64 with saturation, then truncate to int, which is how
        // glibc's strtol-based conversion behaves for out-of-range input.
        let mut value: i64 = 0;
        loop {
            let digit = (c - b'0') as i64;
            value = value
                .saturating_mul(10)
                .saturating_add(if negative { -digit } else { digit });
            match self.next_byte() {
                Some(b) if b.is_ascii_digit() => c = b,
                Some(b) => {
                    self.unget(b);
                    break;
                }
                None => break,
            }
        }

        Some(value as i32)
    }
}

/// Jump targets of the original `while` loop body.
enum State {
    /// Evaluate the `while` condition.
    Condition,
    /// First statement of the loop body.
    BodyTop,
    /// `label1:`
    Label1,
    /// `label2:`
    Label2,
}

fn foo(out: &mut impl Write, mut x: i32, mut y: i32) {
    let mut state = State::Condition;

    loop {
        match state {
            State::Condition => {
                if x > 0 || y > 0 {
                    state = State::BodyTop;
                } else {
                    break;
                }
            }

            State::BodyTop => {
                let _ = write!(out, "loop\n");

                if x == 1 && y == 4 {
                    state = State::Label2; // goto label2;
                } else {
                    state = State::Label1; // fall through to label1
                }
            }

            State::Label1 => {
                if x > 0 {
                    let _ = write!(out, "x\n");
                    x = x.wrapping_sub(1);
                }
                state = State::Label2; // fall through to label2
            }

            State::Label2 => {
                if y == 0 {
                    state = State::Condition; // continue;
                    continue;
                }
                let _ = write!(out, "y\n");
                y = y.wrapping_sub(1);
                if x < 3 {
                    state = State::Label1; // goto label1;
                } else {
                    state = State::Condition; // end of loop body
                }
            }
        }
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    let mut scanner = Scanner::new();
    // scanf("%d %d", &x, &y): the second conversion is only attempted if the
    // first one succeeds.
    if let Some(v) = scanner.scan_i32() {
        x = v;
        if let Some(v) = scanner.scan_i32() {
            y = v;
        }
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    foo(&mut out, x, y);
    let _ = out.flush();
}
