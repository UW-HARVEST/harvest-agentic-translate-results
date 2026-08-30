// Translation of c_src/src/main.c to Rust.
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

/// `void driver(int x)` — `auto int y = 2*x; y += 300; printf("%d\n", y);`
///
/// The `auto` storage-class specifier has no runtime effect in C. The
/// arithmetic is performed on 32-bit `int`; signed overflow is undefined in C
/// but wraps two's-complement on the target compilers, so wrapping ops are
/// used here to mirror the emitted machine behavior.
fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    // printf("%d\n", y)
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", y);
    let _ = out.flush();
}

/// Reads one byte from stdin, or `None` at EOF.
///
/// Reading a single byte at a time keeps stdin positioned exactly where C's
/// `scanf` would leave it (aside from the single character of pushback, which
/// this program never observes since it performs no further reads).
fn read_byte() -> Option<u8> {
    let mut b = [0u8; 1];
    loop {
        match std::io::stdin().read(&mut b) {
            Ok(0) => return None,
            Ok(_) => return Some(b[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion and `None` on an input or
/// matching failure (in which case the caller leaves its variable untouched,
/// just as C does). `%d` skips leading whitespace (including newlines), accepts
/// an optional sign, then a run of decimal digits.
///
/// On overflow, glibc accumulates the digit run and converts it with
/// `strtol`, which saturates at `LONG_MAX`/`LONG_MIN`; the result is then
/// stored through an `int *`, truncating to 32 bits. That is reproduced here
/// with a saturating i64 accumulation followed by a wrapping cast.
fn scanf_d() -> Option<i32> {
    // Skip leading whitespace.
    let mut c = loop {
        match read_byte() {
            None => return None, // input failure (EOF before any conversion)
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        c = match read_byte() {
            None => return None, // sign then EOF: matching failure
            Some(b) => b,
        };
    }

    if !c.is_ascii_digit() {
        return None; // matching failure
    }

    let mut acc: i64 = 0;
    loop {
        let digit = (c - b'0') as i64;
        acc = acc.saturating_mul(10);
        acc = if negative {
            acc.saturating_sub(digit)
        } else {
            acc.saturating_add(digit)
        };

        match read_byte() {
            Some(b) if b.is_ascii_digit() => c = b,
            // Non-digit is pushed back by scanf; nothing else reads stdin.
            _ => break,
        }
    }

    Some(acc as i32)
}

fn main() {
    let mut x: i32 = 0;
    if let Some(v) = scanf_d() {
        x = v;
    }
    driver(x);
    // return 0;
}
