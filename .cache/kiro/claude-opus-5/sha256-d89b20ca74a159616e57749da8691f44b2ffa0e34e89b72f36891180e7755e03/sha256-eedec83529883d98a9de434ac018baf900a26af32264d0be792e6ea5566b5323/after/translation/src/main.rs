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
//! Behavioral notes (kept faithful to the C original):
//! * `scanf("%d", &x)` skips leading whitespace (including newlines), then reads
//!   an optional sign followed by decimal digits. If the conversion fails, `x`
//!   keeps its initial value of `0`.
//! * `driver(x)` prints nothing when `x <= 0`, since the `for` condition is
//!   checked before the first iteration.
//! * `j += 2` is `int` arithmetic; on the platforms the C targets this wraps,
//!   so `wrapping_add` is used to reproduce it rather than panicking.
//! * The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, which the C
//!   program does not do. `restore_default_sigpipe` puts the default
//!   disposition back so that a closed stdout kills this process the same way
//!   it kills the C one (exit via signal 13 rather than a silent exit 0).

use std::io::{self, Read, Write};

/// Reinstate the default `SIGPIPE` disposition that the C program runs with.
///
/// Rust's runtime installs `SIG_IGN` for `SIGPIPE` before `main` so that writes
/// to a closed pipe surface as `EPIPE` errors. The C program inherits the
/// default action instead, so writing to a closed stdout terminates it by
/// signal. Resetting the handler keeps the observable exit status identical.
fn restore_default_sigpipe() {
    #[cfg(unix)]
    {
        const SIGPIPE: i32 = 13;
        const SIG_DFL: usize = 0;
        extern "C" {
            fn signal(signum: i32, handler: usize) -> usize;
        }
        // Safety: `signal` with `SIG_DFL` is async-signal-safe and is called
        // once, before any other thread exists.
        unsafe {
            signal(SIGPIPE, SIG_DFL);
        }
    }
}

/// A minimal byte-at-a-time stdin reader with one byte of push-back, which is
/// what `scanf` needs in order to leave the first non-matching character in the
/// stream.
struct Scanner {
    input: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: io::stdin(),
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
        let mut buf = [0u8; 1];
        match self.input.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            Ok(_) => {
                self.eof = true;
                None
            }
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn unread(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Emulates `scanf("%d", &out)`. Returns `true` when a value was assigned
    /// (i.e. `scanf` would have returned 1).
    fn scan_int(&mut self, out: &mut i32) -> bool {
        // Skip whitespace, exactly as the C standard's whitespace set.
        let mut byte = loop {
            match self.next_byte() {
                Some(b) if is_space(b) => continue,
                Some(b) => break b,
                None => return false,
            }
        };

        let mut negative = false;
        if byte == b'+' || byte == b'-' {
            negative = byte == b'-';
            match self.next_byte() {
                Some(b) => byte = b,
                None => return false,
            }
        }

        if !byte.is_ascii_digit() {
            // Matching failure: push the offending byte back and assign nothing.
            self.unread(byte);
            return false;
        }

        // glibc converts `%d` through `strtol`, which clamps to `LONG_MAX` /
        // `LONG_MIN` on overflow and only then narrows the result to `int`.
        // Accumulate the magnitude separately so the clamp happens on the
        // signed value, matching that behaviour.
        let mut magnitude: u64 = 0;
        let mut overflowed = false;
        loop {
            let digit = u64::from(byte - b'0');
            match magnitude
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
            {
                Some(v) => magnitude = v,
                None => overflowed = true,
            }
            match self.next_byte() {
                Some(b) if b.is_ascii_digit() => byte = b,
                Some(b) => {
                    self.unread(b);
                    break;
                }
                None => break,
            }
        }

        const NEG_MIN_MAGNITUDE: u64 = 1u64 << 63; // |i64::MIN|
        let value: i64 = if negative {
            if overflowed || magnitude > NEG_MIN_MAGNITUDE {
                i64::MIN
            } else {
                (magnitude as i64).wrapping_neg()
            }
        } else if overflowed || magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };

        *out = value as i32;
        true
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
}

fn driver<W: Write>(x: i32, out: &mut W) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        let _ = writeln!(out, "{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

fn main() {
    restore_default_sigpipe();

    let mut x: i32 = 0;
    let mut scanner = Scanner::new();
    scanner.scan_int(&mut x);

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    driver(x, &mut out);
    let _ = out.flush();
}
