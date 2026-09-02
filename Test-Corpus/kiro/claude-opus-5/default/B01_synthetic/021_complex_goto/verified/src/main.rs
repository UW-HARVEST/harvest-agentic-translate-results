// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{Read, Write};

/// Minimal stdin byte reader with single-byte pushback, mirroring the
/// getc/ungetc behavior that `scanf` relies on.
struct Scanner {
    input: std::io::Stdin,
    pushback: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: std::io::stdin(),
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
            match self.input.read(&mut buf) {
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

    /// A literal whitespace directive in a scanf format string: consume any
    /// (possibly zero) run of whitespace characters.
    fn skip_whitespace(&mut self) {
        while let Some(b) = self.getc() {
            if !is_space(b) {
                self.ungetc(b);
                break;
            }
        }
    }

    /// The `%d` conversion: skip leading whitespace, then convert an optionally
    /// signed decimal integer. Returns `None` on input failure (EOF before any
    /// character) or matching failure (no digits), leaving the caller's
    /// variable untouched, exactly as C `scanf` does.
    ///
    /// glibc performs the conversion at `long` width and then narrows to `int`,
    /// saturating at `LONG_MIN`/`LONG_MAX` on overflow; that is reproduced here
    /// with an `i64` accumulator saturated and then truncated to `i32`.
    fn scan_i32(&mut self) -> Option<i32> {
        self.skip_whitespace();

        let mut negative = false;
        let first = self.getc()?;
        let mut cur = match first {
            b'+' => {
                let c = self.getc();
                match c {
                    Some(c) => c,
                    None => return None,
                }
            }
            b'-' => {
                negative = true;
                let c = self.getc();
                match c {
                    Some(c) => c,
                    None => return None,
                }
            }
            c => c,
        };

        if !cur.is_ascii_digit() {
            self.ungetc(cur);
            return None;
        }

        let mut acc: i64 = 0;
        let mut overflow = false;
        loop {
            let digit = (cur - b'0') as i64;
            if !overflow {
                match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                    Some(v) => acc = v,
                    None => overflow = true,
                }
            }
            match self.getc() {
                Some(c) if c.is_ascii_digit() => cur = c,
                Some(c) => {
                    self.ungetc(c);
                    break;
                }
                None => break,
            }
        }

        let value: i64 = if overflow {
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

        Some(value as i32)
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Faithful translation of the C `foo`, including its `goto` control flow.
///
/// `label1` and `label2` are both inside the `while` body, so `goto label1`
/// re-enters the body without re-testing the loop condition, while `continue`
/// jumps back to the condition test. The inner `loop` below plays the role of
/// the label region; `entry_at_label1` selects whether control lands on
/// `label1` or falls through directly to `label2`.
fn foo(mut x: i32, mut y: i32) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    'while_loop: while x > 0 || y > 0 {
        let _ = out.write_all(b"loop\n");

        // if (x == 1 && y == 4) goto label2;
        let mut entry_at_label1 = !(x == 1 && y == 4);

        loop {
            if entry_at_label1 {
                // label1:
                if x > 0 {
                    let _ = out.write_all(b"x\n");
                    x = x.wrapping_sub(1);
                }
            }

            // label2:
            if y == 0 {
                continue 'while_loop;
            }
            let _ = out.write_all(b"y\n");
            y = y.wrapping_sub(1);
            if x < 3 {
                // goto label1;
                entry_at_label1 = true;
                continue;
            }
            break;
        }
    }

    let _ = out.flush();
}

fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    // scanf("%d %d", &x, &y);
    let mut scanner = Scanner::new();
    if let Some(v) = scanner.scan_i32() {
        x = v;
        scanner.skip_whitespace();
        if let Some(v) = scanner.scan_i32() {
            y = v;
        }
    }

    foo(x, y);
}
