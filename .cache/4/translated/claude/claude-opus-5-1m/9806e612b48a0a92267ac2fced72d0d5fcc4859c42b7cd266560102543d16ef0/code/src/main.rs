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

use std::io::{self, Read, Write};

/// Minimal emulation of C's `scanf("%d")` conversion over the whole of stdin.
///
/// `scanf` treats the input as a single stream and freely reads across newline
/// boundaries, so the entire stream is slurped up front and consumed from a
/// cursor.  Leading whitespace is skipped, an optional sign is accepted, and
/// then a run of decimal digits is consumed.  A conversion that finds no digits
/// is a matching failure and leaves the destination variable untouched (which is
/// exactly what the C program relies on for its `int x = 0, y = 0;` defaults).
struct Scanner {
    buf: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn from_stdin() -> Scanner {
        let mut buf = Vec::new();
        // A read error behaves like end-of-input for our purposes.
        let _ = io::stdin().read_to_end(&mut buf);
        Scanner { buf, pos: 0 }
    }

    /// C's `isspace` for the default "C" locale.
    fn is_c_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Perform one `%d` conversion.  Returns `None` on input failure (EOF
    /// before any non-whitespace character) or matching failure (no digits).
    fn scan_int(&mut self) -> Option<i32> {
        // %d skips any amount of leading whitespace, including newlines.
        while self.pos < self.buf.len() && Scanner::is_c_space(self.buf[self.pos]) {
            self.pos += 1;
        }
        if self.pos >= self.buf.len() {
            return None; // input failure
        }

        let start = self.pos;
        let mut negative = false;
        if self.buf[self.pos] == b'+' || self.buf[self.pos] == b'-' {
            negative = self.buf[self.pos] == b'-';
            self.pos += 1;
        }

        let digits_start = self.pos;
        // Accumulate in i128 and saturate: glibc clamps an out-of-range value to
        // the `long` limits and then the `%d` store truncates it to `int`.
        let mut value: i128 = 0;
        while self.pos < self.buf.len() && self.buf[self.pos].is_ascii_digit() {
            let digit = i128::from(self.buf[self.pos] - b'0');
            if value <= (i64::MAX as i128) {
                value = value * 10 + digit;
            }
            self.pos += 1;
        }

        if self.pos == digits_start {
            // Matching failure: no digits were converted.
            self.pos = start;
            return None;
        }

        let signed: i128 = if negative { -value } else { value };
        let clamped = signed.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        Some(clamped as i32)
    }
}

/// Labels of the original `foo` body, used to reproduce its `goto`/`continue`
/// control flow exactly.
enum State {
    /// The `while (x > 0 || y > 0)` condition test.
    WhileCond,
    /// Top of the loop body (`printf("loop\n")` and the `goto label2` test).
    Body,
    /// `label1:`
    Label1,
    /// `label2:`
    Label2,
}

fn foo(out: &mut impl Write, mut x: i32, mut y: i32) {
    let mut state = State::WhileCond;
    loop {
        match state {
            State::WhileCond => {
                if x > 0 || y > 0 {
                    state = State::Body;
                } else {
                    return;
                }
            }
            State::Body => {
                let _ = out.write_all(b"loop\n");

                if x == 1 && y == 4 {
                    state = State::Label2; // goto label2;
                } else {
                    state = State::Label1; // fall through to label1
                }
            }
            State::Label1 => {
                if x > 0 {
                    let _ = out.write_all(b"x\n");
                    x = x.wrapping_sub(1);
                }
                state = State::Label2; // fall through to label2
            }
            State::Label2 => {
                if y == 0 {
                    state = State::WhileCond; // continue;
                    continue;
                }
                let _ = out.write_all(b"y\n");
                y = y.wrapping_sub(1);
                state = if x < 3 {
                    State::Label1 // goto label1;
                } else {
                    State::WhileCond // end of body
                };
            }
        }
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    let mut scanner = Scanner::from_stdin();
    // scanf("%d %d", &x, &y): the second conversion is only attempted if the
    // first one succeeded, and a failed conversion leaves its variable alone.
    if let Some(v) = scanner.scan_int() {
        x = v;
        if let Some(v) = scanner.scan_int() {
            y = v;
        }
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    foo(&mut out, x, y);
    let _ = out.flush();
}
