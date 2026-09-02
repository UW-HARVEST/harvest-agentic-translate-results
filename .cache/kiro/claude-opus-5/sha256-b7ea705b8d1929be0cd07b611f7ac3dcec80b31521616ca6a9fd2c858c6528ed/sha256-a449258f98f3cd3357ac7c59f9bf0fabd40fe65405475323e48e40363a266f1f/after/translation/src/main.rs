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

use std::io::{Read, Write};

/// Minimal `scanf`-style reader over stdin.
///
/// Reads one byte at a time straight from the file descriptor so that, like C's
/// `scanf`, we never consume more input than a conversion requires (this keeps
/// interactive/terminal behavior identical: the program proceeds as soon as the
/// needed fields have been read instead of waiting for EOF).
struct Scanner<R: Read> {
    src: R,
    pushback: Option<u8>,
    at_eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(src: R) -> Self {
        Scanner {
            src,
            pushback: None,
            at_eof: false,
        }
    }

    /// Equivalent of `getc()`: returns `None` on EOF (or read error, which C's
    /// stdio also reports as a failed read).
    fn getc(&mut self) -> Option<u8> {
        if let Some(c) = self.pushback.take() {
            return Some(c);
        }
        if self.at_eof {
            return None;
        }
        let mut byte = [0u8; 1];
        loop {
            match self.src.read(&mut byte) {
                Ok(0) => {
                    self.at_eof = true;
                    return None;
                }
                Ok(_) => return Some(byte[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.at_eof = true;
                    return None;
                }
            }
        }
    }

    /// Equivalent of `ungetc()`: one byte of pushback, which is all any single
    /// `%d` conversion needs.
    fn ungetc(&mut self, c: u8) {
        self.pushback = Some(c);
    }

    /// Matches a whitespace directive in the format string: consumes zero or
    /// more whitespace characters.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.getc() {
            if !is_space(c) {
                self.ungetc(c);
                return;
            }
        }
    }

    /// Performs a single `%d` conversion.
    ///
    /// Returns `None` on either an input failure (EOF before any character of
    /// the field) or a matching failure (no digits), which is all `scanf`
    /// needs to know here: in both cases the conversion is abandoned and the
    /// caller's variable is left untouched.
    ///
    /// Out-of-range values reproduce glibc's behavior on 64-bit Linux, where
    /// the field is converted with `strtol` semantics (saturating at
    /// `LONG_MAX`/`LONG_MIN`) and the result is then truncated to `int`.
    fn scan_int(&mut self) -> Option<i32> {
        // A numeric conversion always skips leading whitespace first.
        self.skip_whitespace();

        let mut negative = false;
        let mut c = self.getc()?;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            c = match self.getc() {
                Some(next) => next,
                // Sign at EOF: input failure, nothing assigned.
                None => return None,
            };
        }

        if !c.is_ascii_digit() {
            // Matching failure: no digits in the field.
            self.ungetc(c);
            return None;
        }

        let mut magnitude: i128 = 0;
        let mut saturated = false;
        loop {
            if !saturated {
                magnitude = magnitude * 10 + i128::from(c - b'0');
                if magnitude > i128::from(i64::MAX) {
                    saturated = true;
                }
            }
            match self.getc() {
                Some(next) if next.is_ascii_digit() => c = next,
                Some(next) => {
                    self.ungetc(next);
                    break;
                }
                None => break,
            }
        }

        let wide: i64 = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            -(magnitude as i64)
        } else {
            magnitude as i64
        };

        // `long` -> `int` narrowing, as glibc's scanf does when storing.
        Some(wide as i32)
    }
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Restores the default `SIGPIPE` disposition.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before calling `main`, which a
/// C program does not do: writing to a closed pipe makes the C program die from
/// `SIGPIPE`, whereas an ignoring process merely sees `EPIPE` from `write` and
/// carries on. Resetting to `SIG_DFL` restores C's observable behavior
/// (`printf` to a closed stdout kills the process by signal 13).
fn restore_default_sigpipe() {
    extern "C" {
        fn signal(sig: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    // SAFETY: `signal` with `SIG_DFL` only restores the kernel's default
    // disposition; it installs no Rust code as a handler.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// Reproduces the hardware trap the C program takes when `div()` performs an
/// undefined division (divisor of zero, or `INT_MIN / -1`): the process dies
/// from SIGFPE with nothing written to stdout.
fn die_with_sigfpe() -> ! {
    extern "C" {
        fn raise(sig: i32) -> i32;
    }
    const SIGFPE: i32 = 8;
    // SAFETY: `raise` is async-signal-safe and takes only a plain integer.
    // The default SIGFPE disposition terminates the process, so this does not
    // return; `abort` is only a fallback if the signal has been blocked or
    // handled by something outside this program.
    unsafe {
        raise(SIGFPE);
    }
    std::process::abort();
}

fn main() {
    // The C program runs with the process's default signal dispositions.
    restore_default_sigpipe();

    // int x = 1, y = 1;
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    // scanf("%d %d", &x, &y);
    // Conversions stop at the first failure, leaving later variables at their
    // initial values.
    let stdin = std::io::stdin();
    let mut scanner = Scanner::new(stdin.lock());
    if let Some(v) = scanner.scan_int() {
        x = v;
        // The literal space in the format matches optional whitespace; the
        // following %d skips leading whitespace anyway.
        scanner.skip_whitespace();
        if let Some(v) = scanner.scan_int() {
            y = v;
        }
    }

    // div_t result = div(x, y);
    // C's div() truncates toward zero, which matches Rust's / and %.
    let (quot, rem) = match (x.checked_div(y), x.checked_rem(y)) {
        (Some(q), Some(r)) => (q, r),
        _ => die_with_sigfpe(),
    };

    // printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "quotient: {}, remainder: {}\n", quot, rem);
    let _ = out.flush();

    // return 0;
}
