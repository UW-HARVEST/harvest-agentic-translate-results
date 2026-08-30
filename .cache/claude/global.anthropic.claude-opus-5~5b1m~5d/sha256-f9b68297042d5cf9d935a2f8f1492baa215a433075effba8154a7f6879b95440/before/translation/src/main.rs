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

/// Mirrors `void printLine(const char *line)`: prints "%s\n" when the pointer
/// is non-NULL, otherwise does nothing.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(line.as_bytes());
        let _ = out.write_all(b"\n");
    }
}

/// Mirrors `static char *helperBad()`, which returns the address of a
/// function-local array. GCC (as verified against the reference build)
/// substitutes a null pointer for the returned dangling address, so this
/// helper yields "no string" and `printLine` prints nothing.
fn helper_bad() -> Option<&'static str> {
    let _char_string: [u8; 17] = *b"helperBad string\0";
    None
}

fn bad() {
    print_line(helper_bad());
}

/// Mirrors `static char *helperGood1()`, whose array has static storage
/// duration and so remains valid after the function returns.
fn helper_good1() -> Option<&'static str> {
    static CHAR_STRING: &str = "helperGood1 string";
    Some(CHAR_STRING)
}

fn good() {
    print_line(helper_good1());
}

/// Emulates `scanf("%d", &x)`: skips leading whitespace, accepts an optional
/// sign followed by decimal digits, and leaves the destination untouched on a
/// matching/input failure. Reads one byte at a time so that no more input is
/// consumed than the conversion requires (a single pushback byte is retained
/// internally, matching C's ungetc-style behavior; the program exits right
/// after, so nothing observable depends on it).
fn scanf_d(x: &mut i32) -> i32 {
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];

    let mut next = |handle: &mut dyn Read| -> Option<u8> {
        match handle.read(&mut byte) {
            Ok(1) => Some(byte[0]),
            _ => None,
        }
    };

    // Skip whitespace (as isspace() does for the C locale).
    let mut c = loop {
        match next(&mut handle) {
            Some(b) => {
                if !matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
                    break b;
                }
            }
            // Input failure before any conversion: EOF -> return EOF.
            None => return -1,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match next(&mut handle) {
            Some(b) => c = b,
            // Sign consumed but no digit: matching failure.
            None => return 0,
        }
    }

    if !c.is_ascii_digit() {
        // Matching failure: `x` is left unmodified.
        return 0;
    }

    // Accumulate with glibc-like saturation at the long boundaries, then
    // truncate to int, which is what the reference implementation does.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = i64::from(c - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        match next(&mut handle) {
            Some(b) if b.is_ascii_digit() => c = b,
            // Non-digit terminator would be pushed back by ungetc; nothing
            // reads stdin afterwards, so it is simply dropped here.
            _ => break,
        }
    }

    if saturated {
        // glibc clamps an overflowing conversion to LONG_MAX / LONG_MIN
        // before truncating to int.
        acc = if negative { i64::MIN } else { i64::MAX };
    } else if negative {
        acc = acc.wrapping_neg();
    }
    *x = acc as i32;
    1
}

fn main() {
    let mut x: i32 = 0;
    let _ = scanf_d(&mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = std::io::stdout().flush();
    std::process::exit(0);
}
