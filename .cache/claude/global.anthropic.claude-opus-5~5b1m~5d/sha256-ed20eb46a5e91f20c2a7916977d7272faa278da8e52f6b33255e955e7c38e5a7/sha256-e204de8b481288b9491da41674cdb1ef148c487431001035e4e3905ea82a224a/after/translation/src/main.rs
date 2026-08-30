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

        // Accumulate the *magnitude* in an unsigned 64-bit value, saturating on
        // overflow, exactly as glibc's `%d` does via `strtol` on LP64 platforms.
        let mut mag: u64 = 0;
        let mut overflow = false;
        loop {
            let digit = (b - b'0') as u64;
            match mag.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => mag = v,
                None => overflow = true,
            }
            match self.next_byte() {
                Some(c) if c.is_ascii_digit() => b = c,
                Some(c) => {
                    self.push_back(c);
                    break;
                }
                None => break,
            }
        }

        // strtol clamps to LONG_MAX / LONG_MIN on range error; the resulting
        // `long` is then truncated to `int` by the %d conversion.
        const LONG_MAX_MAG: u64 = i64::MAX as u64; //  9223372036854775807
        const LONG_MIN_MAG: u64 = i64::MAX as u64 + 1; // 9223372036854775808
        let value: i64 = if negative {
            if overflow || mag > LONG_MIN_MAG {
                i64::MIN
            } else {
                (mag as i64).wrapping_neg()
            }
        } else if overflow || mag > LONG_MAX_MAG {
            i64::MAX
        } else {
            mag as i64
        };
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
                    x = x.wrapping_sub(1);
                }
            }

            // label2:
            if y == 0 {
                continue 'while_loop;
            }
            let _ = write!(out, "y\n");
            // `y` may be negative here (the C relies on wrap-around), so decrement
            // with explicit wrapping rather than depending on the build profile.
            y = y.wrapping_sub(1);
            if x < 3 {
                state = LABEL1; // goto label1;
                continue;
            }
            break;
        }
    }
}

/// The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main`, whereas a C
/// program keeps the default disposition. Restore `SIG_DFL` so that, like the C,
/// this program is terminated by `SIGPIPE` when its stdout reader goes away
/// instead of silently churning through the rest of the loop.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

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
