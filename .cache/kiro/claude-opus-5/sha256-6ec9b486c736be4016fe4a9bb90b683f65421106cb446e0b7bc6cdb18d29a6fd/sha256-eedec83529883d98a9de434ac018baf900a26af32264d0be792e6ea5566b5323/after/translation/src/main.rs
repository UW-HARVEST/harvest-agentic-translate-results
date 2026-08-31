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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C program reads a single `int` with `scanf("%d", &x)` and then prints the
//! raw object representation of that `int` as lowercase hex bytes, in memory
//! order, followed by a newline.

use std::io::{Read, Write};

// The Rust runtime sets SIGPIPE to SIG_IGN before `main` runs, which a C program
// does not do. With the signal ignored, a write to a broken pipe merely returns
// EPIPE, so this program would exit 0 where the C program is killed by SIGPIPE
// (wait status 141 as a shell reports it). Restoring the default disposition
// makes the two behave identically. `signal` comes from libc, which every
// `*-unknown-linux-gnu` Rust binary already links.
extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

/// Undo the Rust runtime's `SIGPIPE` masking so a broken stdout kills us the way
/// it kills the C program.
fn restore_default_sigpipe() {
    // Safety: `signal` with `SIG_DFL` is async-signal-safe and simply resets the
    // disposition of one signal; there are no other threads at this point.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// Mirrors `static void print_hex(unsigned char *p, int len)`.
///
/// Prints each byte with the `%02x` conversion, then a single newline.
fn print_hex(p: &[u8], len: usize) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut buf = String::with_capacity(len * 2 + 1);
    for i in 0..len {
        // "%02x": lowercase hex, zero padded to two columns. A byte never
        // exceeds two hex digits, so the width never truncates or widens.
        buf.push_str(&format!("{:02x}", p[i]));
    }
    buf.push('\n');
    let _ = out.write_all(buf.as_bytes());
    let _ = out.flush();
}

/// Mirrors `void driver(int x)`.
///
/// The C code reinterprets `&x` as `unsigned char *` and walks `sizeof(int)`
/// bytes, so the output depends on the platform's byte order. `to_ne_bytes`
/// reproduces that exact object representation.
fn driver(x: i32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes, std::mem::size_of::<i32>());
}

/// A single-byte-at-a-time reader over stdin with one byte of push-back, so the
/// consumption of stdin matches `scanf`'s: it stops as soon as it sees a
/// character that cannot extend the current conversion, and pushes that
/// character back.
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

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut b = [0u8; 1];
        match self.input.read(&mut b) {
            Ok(1) => Some(b[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    fn push_back(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Reproduces `scanf("%d", &x)`.
    ///
    /// Returns `Some(value)` on a successful conversion and `None` on a matching
    /// failure or input failure; in the failing cases the caller must leave its
    /// variable untouched, exactly as C does.
    ///
    /// The `%d` conversion:
    ///   1. skips leading whitespace (as `isspace`: ' ', \t, \n, \v, \f, \r),
    ///   2. accepts an optional '+' or '-',
    ///   3. accepts one or more decimal digits.
    ///
    /// Out-of-range input follows glibc, which converts with `strtol`
    /// (saturating at `LONG_MAX` / `LONG_MIN`) and then stores the result into
    /// an `int`, truncating it. On a 64-bit platform that makes, for example,
    /// a huge positive literal become -1 and a huge negative literal become 0.
    fn scan_int(&mut self) -> Option<i32> {
        // 1. Skip leading whitespace. This crosses newlines, matching scanf.
        let mut c = loop {
            match self.next_byte() {
                None => return None, // input failure before any conversion
                Some(b) => {
                    if matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                        continue;
                    }
                    break b;
                }
            }
        };

        // 2. Optional sign.
        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            match self.next_byte() {
                None => return None, // sign then EOF: matching failure
                Some(b) => c = b,
            }
        }

        // 3. At least one digit is required.
        if !c.is_ascii_digit() {
            self.push_back(c);
            return None; // matching failure
        }

        // `strtol` semantics: saturate the magnitude at the `long` limits.
        let limit: u128 = if negative {
            1u128 << 63 // magnitude of LONG_MIN
        } else {
            (1u128 << 63) - 1 // LONG_MAX
        };
        let mut magnitude: u128 = 0;
        let mut saturated = false;

        loop {
            if c.is_ascii_digit() {
                if !saturated {
                    magnitude = magnitude * 10 + u128::from(c - b'0');
                    if magnitude > limit {
                        magnitude = limit;
                        saturated = true;
                    }
                }
            } else {
                self.push_back(c);
                break;
            }
            match self.next_byte() {
                None => break,
                Some(b) => c = b,
            }
        }

        let as_long: i64 = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            // `magnitude <= 2^63`, so negating in i128 is always representable.
            -(magnitude as i128) as i64
        } else {
            magnitude as i64
        };

        // Storing a `long` into an `int` argument truncates.
        Some(as_long as i32)
    }
}

fn main() {
    restore_default_sigpipe();
    // `int x = 0;` — the initial value survives a failed scanf.
    let mut x: i32 = 0;
    let mut scanner = Scanner::new();
    if let Some(v) = scanner.scan_int() {
        x = v;
    }
    driver(x);
}
