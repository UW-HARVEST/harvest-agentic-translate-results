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

use std::io::{self, Read, Write};

/// Minimal emulation of C's `scanf("%d", &x)` over stdin.
///
/// Mirrors the C library behaviour: leading whitespace (including newlines) is
/// skipped, an optional sign may follow, then decimal digits are consumed until
/// a non-digit is seen. Returns `Some(value)` on a successful conversion (the
/// `scanf` return value would be 1) and `None` on a matching failure or EOF
/// before any digit (`0` or `EOF`), in which case the caller leaves the
/// destination variable untouched.
///
/// As with glibc, a value that does not fit in a C `long` saturates at
/// `LONG_MAX`/`LONG_MIN` and the result is then truncated to a C `int`.
fn scanf_int(input: &mut dyn Read) -> Option<i32> {
    // Read one byte at a time so no more input is consumed than `scanf` would.
    let mut byte = [0u8; 1];
    let mut next = || -> Option<u8> {
        loop {
            match input.read(&mut byte) {
                Ok(0) => return None,
                Ok(_) => return Some(byte[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    };

    // Skip whitespace, exactly the set matched by C's isspace() in the C locale.
    let mut c = loop {
        match next() {
            Some(b' ') | Some(b'\t') | Some(b'\n') | Some(0x0b) | Some(0x0c) | Some(b'\r') => {
                continue
            }
            Some(other) => break other,
            None => return None, // EOF before any conversion.
        }
    };

    // Optional sign.
    let negative = match c {
        b'-' => {
            c = match next() {
                Some(v) => v,
                None => return None, // Matching failure: sign with no digits.
            };
            true
        }
        b'+' => {
            c = match next() {
                Some(v) => v,
                None => return None,
            };
            false
        }
        _ => false,
    };

    if !c.is_ascii_digit() {
        return None; // Matching failure.
    }

    // Accumulate the magnitude, saturating the way strtol does.
    let mut saturated = false;
    let mut acc: u64 = 0;
    loop {
        let digit = u64::from(c - b'0');
        match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) => acc = v,
            None => saturated = true,
        }
        // glibc's strtol clamps at LONG_MAX / LONG_MIN.
        let limit = if negative {
            i64::MIN.unsigned_abs()
        } else {
            i64::MAX as u64
        };
        if acc > limit {
            saturated = true;
        }

        match next() {
            Some(v) if v.is_ascii_digit() => c = v,
            // A non-digit terminates the conversion (it would be pushed back
            // with ungetc, which is unobservable here), as does EOF.
            _ => break,
        }
    }

    let as_long: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };

    // Assignment of the long result to an `int` object truncates.
    Some(as_long as i32)
}

fn driver(x: i32, out: &mut dyn Write) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        // Ignore write errors, as printf's return value is ignored in the C.
        let _ = writeln!(out, "{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

fn main() {
    let mut x: i32 = 0;

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    if let Some(v) = scanf_int(&mut handle) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    driver(x, &mut out);
    let _ = out.flush();
}
