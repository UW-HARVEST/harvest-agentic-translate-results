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

//! Translation of `c_src/src/main.c`.
//!
//! The original is written with digraphs and the `<iso646.h>` alternative
//! operator spellings, which de-sugar to:
//!
//! ```c
//! #include <stdio.h>
//! #include <iso646.h>
//!
//! void driver(int x, int y) {
//!     int result = x | ~y;      /* x bitor compl y */
//!     printf("%d", result);
//!     puts("");
//! }
//!
//! int main() {
//!     int x = 0, y = 0;
//!     scanf("%d", &x);
//!     scanf("%d", &y);
//!     driver(x, y);
//!     return 0;
//! }
//! ```
//!
//! Behaviour that must be preserved byte-for-byte:
//!
//! * `x` and `y` are initialised to `0` and are left **untouched** when a
//!   `scanf` conversion fails (matching failure or input failure), so bad or
//!   missing input yields `0` for the affected variable. The return values of
//!   `scanf` are discarded, so a failure is silent.
//! * `scanf("%d", ...)` skips leading whitespace (including newlines), accepts
//!   an optional sign, then decimal digits only.
//! * glibc performs the `%d` conversion through `strtol` (a 64-bit `long` on
//!   LP64) and then *assigns* the resulting `long` to an `int`, so out-of-range
//!   input is first clamped to `LONG_MAX`/`LONG_MIN` and afterwards truncated
//!   to 32 bits.
//! * On a matching failure glibc has already consumed the leading whitespace
//!   and an optional sign character; only the single character that actually
//!   failed to match is pushed back. For the input `"--5"` the first conversion
//!   therefore fails having eaten just the first `-`, leaving `"-5"` for the
//!   second directive.
//! * Input is consumed **lazily**: `scanf` reads only as much as it needs, so
//!   the program terminates promptly even when `stdin` never reaches
//!   end-of-file (e.g. `yes 5 | driver`).

use std::io::{BufRead, ErrorKind, Write};
use std::os::raw::c_int;

/// Incremental `scanf`-style scanner over a buffered byte stream.
///
/// glibc's `scanf` needs a single character of look-ahead, which it implements
/// with `ungetc`. `BufRead::fill_buf` + `consume` provide exactly the same
/// peek-then-commit primitive without any extra buffering, and crucially they
/// keep the read lazy: only the bytes actually required by the directives are
/// pulled from the underlying descriptor.
pub struct Scanner<R: BufRead> {
    inner: R,
    /// Sticky end-of-file / error indicator.
    ///
    /// C streams latch their EOF and error flags: once `scanf` has seen the end
    /// of the stream, every later call fails immediately without touching the
    /// descriptor again. Mirroring that keeps behaviour identical for streams
    /// that could yield more data after a short read.
    eof: bool,
}

impl<R: BufRead> Scanner<R> {
    pub fn new(inner: R) -> Scanner<R> {
        Scanner { inner, eof: false }
    }

    /// Look at the next byte without consuming it, or `None` at end-of-file.
    ///
    /// A read error is reported as end-of-file: `inchar()` in glibc yields
    /// `EOF` in that case too, which makes the directive fail and leaves the
    /// destination variable untouched.
    fn peek(&mut self) -> Option<u8> {
        if self.eof {
            return None;
        }
        loop {
            match self.inner.fill_buf() {
                Ok([]) => {
                    self.eof = true;
                    return None;
                }
                Ok(buf) => return Some(buf[0]),
                // A restartable read is retried, matching the read loop inside
                // the C library rather than reporting a spurious EOF.
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Consume the byte that [`Scanner::peek`] just returned.
    fn bump(&mut self) {
        self.inner.consume(1);
    }

    /// `isspace()` in the `"C"` locale. The program never calls `setlocale`, so
    /// it always runs in the `"C"` locale no matter what the environment says.
    fn is_space(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
    }

    /// Emulates a single `scanf("%d", out)` directive.
    ///
    /// Returns `true` when the conversion succeeded (and `out` was written),
    /// `false` on an input or matching failure, leaving `out` untouched exactly
    /// like C.
    pub fn scan_int(&mut self, out: &mut i32) -> bool {
        // Leading whitespace is consumed unconditionally, even when the
        // directive later fails to match.
        while let Some(c) = self.peek() {
            if Scanner::<R>::is_space(c) {
                self.bump();
            } else {
                break;
            }
        }

        let negative = match self.peek() {
            Some(b'-') => {
                self.bump();
                true
            }
            Some(b'+') => {
                self.bump();
                false
            }
            _ => false,
        };

        let mut saw_digit = false;
        // Accumulate in `i128` and clamp to the `long` range the way `strtol`
        // does. Accumulation stops as soon as the value is provably out of
        // range, which keeps arbitrarily long digit runs cheap and prevents the
        // accumulator itself from overflowing.
        let mut acc: i128 = 0;
        let mut saturated = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            saw_digit = true;
            self.bump();
            if !saturated {
                acc = acc * 10 + i128::from(c - b'0');
                if acc > i128::from(i64::MAX) + 1 {
                    saturated = true;
                }
            }
        }

        if !saw_digit {
            // Matching failure. glibc pushes back only the single offending
            // character (which was peeked, never consumed); an already
            // consumed sign character stays consumed.
            return false;
        }

        let magnitude = if negative { -acc } else { acc };
        // `strtol` clamps on overflow (and sets `ERANGE`); glibc then assigns
        // the resulting `long` to an `int`, truncating the value.
        let as_long: i64 = if magnitude > i128::from(i64::MAX) {
            i64::MAX
        } else if magnitude < i128::from(i64::MIN) {
            i64::MIN
        } else {
            magnitude as i64
        };

        *out = as_long as i32;
        true
    }
}

/// `void driver(int x, int y)` from the C source.
///
/// ```c
/// int result = x bitor compl y;   /* x | ~y */
/// printf("%d", result);
/// puts("");
/// ```
///
/// Write errors are discarded, just as the C code ignores the return values of
/// `printf` and `puts`.
pub fn driver_impl(x: c_int, y: c_int) {
    let result: c_int = x | !y; // x bitor compl y
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // printf("%d", result);
    let _ = write!(out, "{}", result);
    // puts("");
    let _ = writeln!(out);
    let _ = out.flush();
}

/// `int main()` from the C source, minus the process-level plumbing.
///
/// Returns the value the C `main` returns, so it can be used both as the
/// process exit status and as the return value of the exported `main` symbol.
pub fn c_main() -> c_int {
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    let stdin = std::io::stdin();
    let mut input = Scanner::new(stdin.lock());
    input.scan_int(&mut x);
    input.scan_int(&mut y);
    driver_impl(x, y);
    0
}

/// Restore the C program's `SIGPIPE` disposition.
///
/// A C program starts with `SIGPIPE` set to `SIG_DFL`, so writing to a pipe
/// whose reader has gone away terminates the process with signal 13. The Rust
/// runtime instead installs `SIG_IGN` before `main` runs, which would turn that
/// into a silently ignored `EPIPE` and a `0` exit status. Undoing it keeps the
/// observable process behaviour identical to the C build.
#[cfg(unix)]
pub fn restore_default_sigpipe() {
    // `signal(2)`; `SIG_DFL` is the null handler and `SIGPIPE` is 13 on Linux.
    const SIGPIPE: c_int = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: c_int, handler: usize) -> usize;
    }
    // SAFETY: `signal` is async-signal-safe and this merely restores the
    // disposition the process would have had without the Rust runtime's
    // start-up code. It runs before any thread is spawned.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
pub fn restore_default_sigpipe() {}
