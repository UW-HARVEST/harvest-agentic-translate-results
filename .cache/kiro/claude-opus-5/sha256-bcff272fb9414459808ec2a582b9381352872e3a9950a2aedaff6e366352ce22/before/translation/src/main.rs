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
//! The C program is a CWE-457 (use of an uninitialized variable) test case. It
//! reads an integer with `scanf("%d", &x)` and then either takes the `good()`
//! path (a properly initialized `char *`) or the `bad()` path (an
//! uninitialized `char *` handed straight to `printLine`).
//!
//! The `bad()` path is undefined behavior in C, so it has no portable meaning.
//! It is reproduced here to match the observable output of the reference build
//! (the `CMakeLists.txt` build, which sets no optimization flags): on that
//! build the indeterminate pointer is non-NULL and the byte it addresses is
//! `\0`, so `printf("%s\n", line)` emits a single newline. See `bad()` below.

use std::io::{Read, Write};

/// Mirrors `void printLine(const char *line)`.
///
/// `Option<&[u8]>` stands in for the C pointer: `None` is `NULL` (printed
/// nothing, exactly like the C NULL guard) and `Some(bytes)` is the
/// NUL-terminated string body that `%s` would emit.
fn print_line(out: &mut impl Write, line: Option<&[u8]>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        let _ = out.write_all(line);
        let _ = out.write_all(b"\n");
    }
}

/// Mirrors `void bad()`.
///
/// The C body is:
/// ```c
/// char *data;        /* never initialized */
/// printLine(data);
/// ```
///
/// The bug is preserved rather than fixed: no valid string is produced here.
/// The value passed matches what the reference build actually observed in that
/// indeterminate pointer -- a non-NULL address whose first byte is the string
/// terminator -- which makes `printLine` write just the trailing newline.
fn bad(out: &mut impl Write) {
    let data: Option<&[u8]> = Some(b"");
    print_line(out, data);
}

/// Mirrors `void good()`.
fn good(out: &mut impl Write) {
    let data: Option<&[u8]> = Some(b"string");
    print_line(out, data);
}

/// True for the characters C's `isspace` accepts in the default locale, which
/// is the set `scanf` skips before a `%d` conversion.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Reads one byte from stdin, or `None` at EOF.
///
/// Reading a byte at a time keeps stdin consumption close to `scanf`, which
/// stops as soon as the conversion is complete (pushing back the one
/// terminating character). Nothing else in the program reads stdin, so the
/// single extra byte consumed in place of `ungetc` cannot affect output.
fn next_byte(input: &mut impl Read) -> Option<u8> {
    let mut buf = [0u8; 1];
    loop {
        match input.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => return Some(buf[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// Performs `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion and `None` on EOF or a
/// matching failure; in the failure cases the caller leaves `x` untouched,
/// just as `scanf` does.
///
/// Overflow follows glibc: the digits are accumulated with `long` (64-bit)
/// range and saturate at its limits, then the result is truncated into `int`.
/// That is why an input of `4294967296` yields `0` and takes the `bad()` path.
fn scanf_i32(input: &mut impl Read) -> Option<i32> {
    // Skip leading whitespace; EOF here is an input failure.
    let mut b = loop {
        let b = next_byte(input)?;
        if !is_c_space(b) {
            break b;
        }
    };

    // Optional sign.
    let negative = match b {
        b'-' => {
            b = next_byte(input)?;
            true
        }
        b'+' => {
            b = next_byte(input)?;
            false
        }
        _ => false,
    };

    // At least one digit is required, otherwise it is a matching failure.
    if !b.is_ascii_digit() {
        return None;
    }

    let mut acc: i64 = 0;
    loop {
        let digit = i64::from(b - b'0');
        acc = if negative {
            acc.saturating_mul(10).saturating_sub(digit)
        } else {
            acc.saturating_mul(10).saturating_add(digit)
        };

        match next_byte(input) {
            Some(next) if next.is_ascii_digit() => b = next,
            // Trailing non-digit stands in for the character scanf ungets.
            _ => break,
        }
    }

    // long -> int conversion, i.e. keep the low 32 bits.
    Some(acc as i32)
}

fn main() {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    if let Some(value) = scanf_i32(&mut input) {
        x = value;
    }

    if x != 0 {
        good(&mut out);
    } else {
        bad(&mut out);
    }

    let _ = out.flush();
    std::process::exit(0);
}
