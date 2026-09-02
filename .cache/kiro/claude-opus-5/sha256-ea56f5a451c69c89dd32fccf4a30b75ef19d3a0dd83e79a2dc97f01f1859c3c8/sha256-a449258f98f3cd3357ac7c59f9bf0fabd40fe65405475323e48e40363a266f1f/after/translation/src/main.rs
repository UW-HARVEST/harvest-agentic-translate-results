// Rust translation of c_src/src/main.c
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

/// Equivalent of `static void print_hex(unsigned char *p, int len)`.
///
/// Prints each byte as two lowercase hex digits (`%02x`), then a newline.
fn print_hex(out: &mut impl Write, p: &[u8], len: usize) {
    for i in 0..len {
        let _ = write!(out, "{:02x}", p[i]);
    }
    let _ = write!(out, "\n");
}

/// Equivalent of `void driver(int x)`.
///
/// `char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));` copies the object
/// representation of the `int`, i.e. its native-endian byte layout.
fn driver(out: &mut impl Write, x: i32) {
    let raw = x.to_ne_bytes();
    print_hex(out, &raw, raw.len());
}

/// Faithful model of glibc's `scanf("%d", &x)` for a single conversion.
///
/// Returns `None` on input failure (EOF before any non-whitespace) or matching
/// failure (no digits found), in which case the caller leaves `x` untouched -
/// exactly like C, where `x` keeps its initial value of 0.
///
/// glibc performs the conversion into a `long` (via the same accumulation
/// strtol uses, clamping to `LONG_MAX`/`LONG_MIN` on overflow) and then stores
/// the low bytes into the `int` destination, so out-of-range input truncates.
fn scanf_d(input: &mut impl Read) -> Option<i32> {
    let mut byte = [0u8; 1];

    // Read one byte; None on EOF or read error (C treats both as input failure).
    let mut next = || -> Option<u8> {
        match input.read(&mut byte) {
            Ok(1) => Some(byte[0]),
            _ => None,
        }
    };

    // %d skips leading whitespace (isspace: ' ', \t, \n, \v, \f, \r).
    let mut c = loop {
        let c = next()?;
        if !matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            break c;
        }
    };

    // Optional sign.
    let negative = match c {
        b'-' => {
            c = next()?;
            true
        }
        b'+' => {
            c = next()?;
            false
        }
        _ => false,
    };

    // At least one decimal digit is required, otherwise matching failure.
    if !c.is_ascii_digit() {
        return None;
    }

    let mut acc: u64 = 0;
    let mut overflow = false;
    loop {
        let digit = u64::from(c - b'0');
        match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) => acc = v,
            None => overflow = true,
        }

        match next() {
            Some(n) if n.is_ascii_digit() => c = n,
            // The non-digit byte is pushed back by scanf; nothing else reads
            // stdin in this program, so it is simply dropped here.
            _ => break,
        }
    }

    const LONG_MAX: u64 = i64::MAX as u64;
    const LONG_MIN_MAG: u64 = 1u64 << 63;

    let value: i64 = if negative {
        if overflow || acc > LONG_MIN_MAG {
            i64::MIN
        } else {
            (acc as i64).wrapping_neg()
        }
    } else if overflow || acc > LONG_MAX {
        i64::MAX
    } else {
        acc as i64
    };

    // Store into the `int` destination: truncate to the low 32 bits.
    Some(value as i32)
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, which makes a
/// write to a closed pipe return `EPIPE` instead of killing the process. A C
/// program keeps the default disposition, so `printf` to a broken pipe
/// terminates it with signal 13 (shell status 141). Restore the C behavior so
/// the exit status matches.
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

    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    if let Some(v) = scanf_d(&mut input) {
        x = v;
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, x);
    let _ = out.flush();
}
