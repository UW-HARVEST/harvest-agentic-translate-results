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
//
// Rust translation of c_src/src/main.c. Behavior is intentionally identical to
// the original, including C's wrapping integer arithmetic and glibc's `scanf`
// `%d` conversion semantics (long-range clamping followed by truncation to
// `int`).

use std::io::{self, Read, Write};

/// A byte-oriented view of stdin with a single-byte pushback slot, mirroring
/// the way C's stdio streams allow a conversion to "unread" the delimiter byte
/// that terminated it.
struct Stdin {
    inner: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    /// Reads the next byte, or `None` at end-of-file / on a read error
    /// (`scanf` treats both as an input failure).
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    /// Pushes a byte back so the next `next_byte` call returns it again.
    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// True for the bytes that C's `isspace` accepts in the default "C" locale, the
/// set that a `scanf` conversion skips before converting.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates a single `scanf("%d", ...)` conversion.
///
/// Returns `Some(value)` on a successful conversion, or `None` on an input or
/// matching failure (in which case the caller leaves its variable untouched,
/// exactly as C does).
///
/// Overflow follows glibc: the digits are accumulated in `long` range and
/// saturate at `LONG_MAX` / `LONG_MIN`, and the result is then truncated to
/// `int`.
fn scanf_d(input: &mut Stdin) -> Option<i32> {
    // Skip leading whitespace. Hitting EOF here is an input failure.
    let mut b = loop {
        let b = input.next_byte()?;
        if !is_c_space(b) {
            break b;
        }
    };

    // Optional sign.
    let negative = match b {
        b'-' => {
            b = input.next_byte()?;
            true
        }
        b'+' => {
            b = input.next_byte()?;
            false
        }
        _ => false,
    };

    // At least one digit is required, otherwise this is a matching failure.
    if !b.is_ascii_digit() {
        input.unget(b);
        return None;
    }

    // `long` is 64-bit on the reference platform (LP64 Linux).
    let limit: u128 = if negative {
        1u128 << 63 // magnitude of LONG_MIN
    } else {
        (1u128 << 63) - 1 // LONG_MAX
    };

    let mut magnitude: u128 = 0;

    loop {
        magnitude = magnitude * 10 + u128::from(b - b'0');
        if magnitude > limit {
            // Saturate; further digits are consumed but cannot change the value.
            magnitude = limit;
        }

        match input.next_byte() {
            Some(next) if next.is_ascii_digit() => b = next,
            Some(next) => {
                input.unget(next);
                break;
            }
            None => break,
        }
    }

    let as_long: i64 = if negative {
        // `limit` is the magnitude of LONG_MIN, so this cannot overflow.
        (magnitude as i64).wrapping_neg()
    } else {
        magnitude as i64
    };

    // Truncation to `int`, as the C conversion stores into an `int` object.
    Some(as_long as i32)
}

fn driver(x: i32) {
    // C: `auto int y = 2*x;` -- `auto` is only a storage-class specifier here.
    // Signed overflow wraps on the reference platform.
    let mut y: i32 = x.wrapping_mul(2);
    y = y.wrapping_add(300);

    // C's `printf` returns a negative value on a write error and `main` ignores
    // it, so a failing stdout (a full device, a closed descriptor) is silent and
    // the process still exits 0. `println!` would panic there, so write the
    // bytes directly and discard the error.
    let mut out = io::stdout();
    let _ = out.write_all(format!("{}\n", y).as_bytes());
}

/// Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, while a C
/// program keeps the default disposition and is therefore *killed* by signal 13
/// when it writes to a broken pipe. Restore the default so the exit status
/// matches the C program's in that case.
#[cfg(unix)]
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

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    let mut x: i32 = 0;
    let mut input = Stdin::new();
    if let Some(value) = scanf_d(&mut input) {
        x = value;
    }
    driver(x);

    // Ensure stdout is flushed before exiting, like C's exit-time flush.
    // Errors are discarded: C's exit-time flush failure is silent too.
    let _ = io::stdout().flush();
}
