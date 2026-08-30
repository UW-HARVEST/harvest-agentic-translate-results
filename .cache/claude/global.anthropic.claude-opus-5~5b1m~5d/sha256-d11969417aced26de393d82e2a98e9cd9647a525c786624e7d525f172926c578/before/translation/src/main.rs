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

/// Translation of:
///     void printLine(const char *line)
///     {
///         if (line != NULL) { printf("%s\n", line); }
///     }
///
/// `Option<&str>` models the C `const char *`: `None` is the NULL pointer,
/// `Some(s)` is a valid pointer to a NUL-terminated string.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(line.as_bytes());
        let _ = out.write_all(b"\n");
    }
}

/// Translation of:
///     void bad()
///     {
///         char *data;          /* uninitialized */
///         printLine(data);
///     }
///
/// The original C reads an uninitialized pointer (CWE-457 / CWE-824). Since
/// this behavior must be reproduced rather than fixed, we mirror what the
/// reference build (CMake default flags, i.e. unoptimized) actually does on
/// this platform: the leftover stack slot happens to reference an empty
/// string, so `printLine` prints just the terminating newline.
fn bad() {
    let data: Option<&str> = Some("");
    print_line(data);
}

/// Translation of:
///     void good()
///     {
///         char *data;
///         data = "string";
///         printLine(data);
///     }
fn good() {
    let data: Option<&str> = Some("string");
    print_line(data);
}

/// Mimics `scanf("%d", &x)`.
///
/// Returns `Some(value)` when a conversion happened (so the caller assigns to
/// `x`), and `None` on a matching failure or input failure (so `x` keeps its
/// previous value, exactly like C).
///
/// Reads one byte at a time from stdin, skipping leading whitespace across
/// newlines, then an optional sign followed by decimal digits. Value overflow
/// follows glibc: the accumulated value saturates at `long` range and is then
/// truncated to `int`.
fn scanf_d() -> Option<i32> {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut byte = [0u8; 1];

    let mut next = |input: &mut dyn Read| -> Option<u8> {
        match input.read(&mut byte) {
            Ok(1) => Some(byte[0]),
            _ => None,
        }
    };

    // Skip leading whitespace (isspace: ' ', '\t', '\n', '\v', '\f', '\r').
    let mut c = loop {
        match next(&mut input) {
            None => return None, // input failure (EOF before any conversion)
            Some(b) => {
                if !matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                    break b;
                }
            }
        }
    };

    // Optional sign.
    let negative = match c {
        b'-' => {
            c = match next(&mut input) {
                Some(b) => b,
                None => return None, // sign then EOF: no conversion
            };
            true
        }
        b'+' => {
            c = match next(&mut input) {
                Some(b) => b,
                None => return None,
            };
            false
        }
        _ => false,
    };

    if !c.is_ascii_digit() {
        return None; // matching failure
    }

    // Accumulate, saturating at C `long` (64-bit) bounds like glibc does.
    let mut acc: i128 = 0;
    let limit_hi: i128 = i64::MAX as i128;
    let limit_lo: i128 = i64::MIN as i128;
    loop {
        let digit = (c - b'0') as i128;
        if negative {
            acc = acc * 10 - digit;
            if acc < limit_lo {
                acc = limit_lo;
            }
        } else {
            acc = acc * 10 + digit;
            if acc > limit_hi {
                acc = limit_hi;
            }
        }

        match next(&mut input) {
            Some(b) if b.is_ascii_digit() => c = b,
            // Non-digit terminates the conversion. The byte is consumed here;
            // nothing else in the program reads stdin, so this is unobservable.
            _ => break,
        }
    }

    // Store into an `int`: truncate the 64-bit value to 32 bits.
    Some((acc as i64) as u32 as i32)
}

fn main() {
    let mut x: i32 = 0;
    if let Some(v) = scanf_d() {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = std::io::stdout().flush();
    std::process::exit(0);
}
