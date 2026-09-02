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
//! The original is a CWE-190 (integer overflow) test driver. Its behaviour --
//! including the signed-`char` overflow in `bad()` -- is reproduced exactly
//! rather than corrected.

use std::io::{Read, Write};

/// `limits.h` `CHAR_MAX` for the platform the C targets (x86-64 Linux, where
/// plain `char` is signed).
const CHAR_MAX: i8 = i8::MAX;

/// `void printLine(const char * line)`
///
/// The C guards against a NULL pointer; a Rust `&str` can never be null, so the
/// guard is always taken. Kept as a function to mirror the original structure.
fn print_line(line: &str) {
    println!("{}", line);
}

/// `void printHexCharLine(char charHex)`
///
/// `printf("%02x\n", charHex)` promotes the `char` argument to `int` and then
/// `%x` reinterprets those bits as `unsigned int`. For a negative `char` that
/// yields eight hex digits (e.g. `-2` prints as `fffffffe`), not two, because
/// `02` is only a *minimum* field width.
fn print_hex_char_line(char_hex: i8) {
    println!("{:02x}", char_hex as i32 as u32);
}

/// `void bad()`
///
/// `data * 2` is computed in `int` (254) and then truncated back into a signed
/// `char`, wrapping to `-2`. This overflow is the bug under test and is
/// deliberately preserved.
fn bad() {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        let result: i8 = (data as i32 * 2) as i8;
        print_hex_char_line(result);
    }
}

/// `static void goodG2B()`
fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = (data as i32 * 2) as i8;
        print_hex_char_line(result);
    }
}

/// `static void goodB2G()`
///
/// The initial `data = ' '` in the C is immediately overwritten by `CHAR_MAX`;
/// the dead store is retained for fidelity. `CHAR_MAX/2` is integer division on
/// `char`-promoted `int` values, i.e. 63.
#[allow(unused_assignments)]
fn good_b2g() {
    let mut data: i8 = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        if (data as i32) < (CHAR_MAX as i32 / 2) {
            let result: i8 = (data as i32 * 2) as i8;
            print_hex_char_line(result);
        } else {
            print_line("data value is too large to perform arithmetic safely.");
        }
    }
}

/// `void good()`
fn good() {
    good_g2b();
    good_b2g();
}

/// Emulates `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion and `None` on a matching
/// failure or end-of-input, in which case the caller must leave its variable
/// untouched -- exactly as `scanf` does.
///
/// Behaviour mirrored from C / glibc:
/// * leading whitespace is skipped, including newlines, so the scan happily
///   crosses line boundaries;
/// * an optional `+`/`-` sign may precede the digits;
/// * at least one decimal digit is required, otherwise it is a matching
///   failure;
/// * glibc converts via `strtol`, which saturates at `long` bounds, and then
///   stores the result into an `int`, truncating the low 32 bits.
fn scanf_int(input: &mut impl Read) -> Option<i32> {
    let mut byte = [0u8; 1];

    // Read a single byte, treating EOF and I/O errors alike as "no more input".
    let mut next = |buf: &mut [u8; 1]| -> Option<u8> {
        match input.read(buf) {
            Ok(1) => Some(buf[0]),
            _ => None,
        }
    };

    // Skip leading whitespace, per C's isspace().
    let mut c = loop {
        let c = next(&mut byte)?;
        if !matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            break c;
        }
    };

    // Optional sign.
    let negative = match c {
        b'-' => {
            c = next(&mut byte).unwrap_or(0);
            true
        }
        b'+' => {
            c = next(&mut byte).unwrap_or(0);
            false
        }
        _ => false,
    };

    if !c.is_ascii_digit() {
        // Matching failure: no digits were converted.
        return None;
    }

    // Accumulate with saturation at long (i64) bounds, as strtol does.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = (c - b'0') as i64;
        if !saturated {
            match acc
                .checked_mul(10)
                .and_then(|v| v.checked_add(if negative { -digit } else { digit }))
            {
                Some(v) => acc = v,
                None => {
                    saturated = true;
                    acc = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        match next(&mut byte) {
            Some(n) if n.is_ascii_digit() => c = n,
            // The non-digit byte is what scanf would push back; nothing else in
            // this program reads stdin, so it is simply discarded.
            _ => break,
        }
    }

    // Store into an `int`: keep the low 32 bits.
    Some(acc as i32)
}

fn main() {
    let mut x: i32 = 0;
    let stdin = std::io::stdin();
    if let Some(v) = scanf_int(&mut stdin.lock()) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    // `return 0` from main flushes stdio; make sure ours is flushed too before
    // `panic = "abort"` semantics or an early exit could matter.
    let _ = std::io::stdout().flush();
}
