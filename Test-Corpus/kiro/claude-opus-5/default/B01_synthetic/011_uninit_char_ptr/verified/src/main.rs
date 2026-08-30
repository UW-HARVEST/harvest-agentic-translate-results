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
//! The C program is a CWE-457 (use of uninitialized variable) test case. It is
//! translated here as-is: the defect is *reproduced*, not fixed.

use std::io::{self, Read, Write};

/// Restores the default disposition of `SIGPIPE`.
///
/// A C program starts with `SIGPIPE` set to `SIG_DFL`, so writing to a closed
/// pipe kills it (observable as exit status 141 = 128 + 13). The Rust runtime
/// installs `SIG_IGN` before `main`, which turns the same write into an ignored
/// `EPIPE` and lets the process exit 0. Undoing that is required for the
/// translation to match the C binary's exit status.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // Safety: `signal` with `SIG_DFL` is async-signal-safe and is called once,
    // before any other thread exists.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// Mirrors `void printLine(const char *line)`.
///
/// The C function guards on `line != NULL` and then does `printf("%s\n", line)`,
/// which glibc lowers to `puts`. A `None` argument models the NULL pointer and
/// produces no output at all, exactly like the C.
fn print_line(out: &mut impl Write, line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line)
        let _ = write!(out, "{}\n", line);
    }
}

/// Mirrors `void bad()`.
///
/// The C body is:
///
/// ```c
/// char *data;          /* never initialized */
/// printLine(data);
/// ```
///
/// Reading `data` is undefined behavior; there is no "correct" value to
/// translate. What the reference build actually does is read the stale stack
/// slot left behind by the preceding `scanf` call in `main`. That slot holds a
/// non-NULL pointer, so `printLine`'s `line != NULL` guard passes, and the byte
/// it addresses is a NUL terminator, so `puts` emits nothing but its newline.
///
/// The observable behavior of the reference executable is therefore a single
/// `"\n"`, and that is what this translation reproduces. `Some("")` stands in
/// for "non-NULL pointer to an empty string".
fn bad(out: &mut impl Write) {
    let data: Option<&str> = Some("");
    print_line(out, data);
}

/// Mirrors `void good()`.
fn good(out: &mut impl Write) {
    let data: Option<&str> = Some("string");
    print_line(out, data);
}

/// A single-byte pushback reader, so the scan below can look one byte ahead and
/// "unget" it the way `scanf` does when a conversion stops at a non-matching
/// character.
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

    /// Reads the next byte from the stream, or `None` at EOF / on error.
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Equivalent of `scanf("%d", &x)` for a single conversion.
    ///
    /// `%d` skips leading whitespace (crossing newlines, unlike `fgets`), takes
    /// an optional sign, then base-10 digits. glibc performs the conversion with
    /// `strtol`, which saturates at `LONG_MAX` / `LONG_MIN`, and then stores the
    /// result into an `int`, truncating to the low 32 bits. Both steps are
    /// modeled here so out-of-range input matches byte for byte (e.g.
    /// `-99999999999999999999` becomes `LONG_MIN`, whose low 32 bits are 0).
    ///
    /// Returns `None` on a matching failure or on EOF before any conversion, in
    /// which case the caller's variable is left untouched, as in C.
    fn scan_i32(&mut self) -> Option<i32> {
        // Skip whitespace, per C's isspace(): ' ', '\t', '\n', '\v', '\f', '\r'.
        let mut b = loop {
            let b = self.next_byte()?;
            if !matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                break b;
            }
        };

        let negative = match b {
            b'-' => {
                b = match self.next_byte() {
                    Some(b) => b,
                    // Sign with nothing after it: matching failure.
                    None => return None,
                };
                true
            }
            b'+' => {
                b = match self.next_byte() {
                    Some(b) => b,
                    None => return None,
                };
                false
            }
            _ => false,
        };

        if !b.is_ascii_digit() {
            // No digits consumed: matching failure. Push the offending byte
            // back, as scanf does.
            self.unget(b);
            return None;
        }

        // Accumulate the magnitude, clamping like strtol. Digit strings can be
        // arbitrarily long, so stop accumulating once the limit is passed and
        // just drain the rest.
        let limit: u128 = if negative {
            i64::MIN.unsigned_abs() as u128
        } else {
            i64::MAX as u128
        };
        let mut magnitude: u128 = 0;
        let mut saturated = false;

        loop {
            if !b.is_ascii_digit() {
                self.unget(b);
                break;
            }
            if !saturated {
                magnitude = magnitude * 10 + u128::from(b - b'0');
                if magnitude > limit {
                    saturated = true;
                }
            }
            b = match self.next_byte() {
                Some(b) => b,
                None => break,
            };
        }

        // strtol's result as a C `long`.
        let as_long: i64 = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            -(magnitude as i128) as i64
        } else {
            magnitude as i64
        };

        // Stored into an `int`: truncate to the low 32 bits.
        Some(as_long as i32)
    }
}

fn main() {
    restore_default_sigpipe();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut x: i32 = 0;
    let mut scanner = Scanner::new(stdin.lock());
    // scanf("%d", &x): on failure or EOF, `x` keeps its previous value of 0.
    if let Some(v) = scanner.scan_i32() {
        x = v;
    }

    if x != 0 {
        good(&mut out);
    } else {
        bad(&mut out);
    }

    let _ = out.flush();
    // return 0;
}
