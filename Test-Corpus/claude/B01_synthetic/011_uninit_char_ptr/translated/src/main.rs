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

// Rust translation of c_src/src/main.c
//
// The C program reads an int with scanf("%d", &x) and then either calls good()
// (which prints "string") or bad() (which passes an *uninitialized* char* to
// printLine()).
//
// bad() is technically undefined behavior in C.  As built by the supplied
// CMakeLists.txt (no CMAKE_BUILD_TYPE, i.e. no optimization) the uninitialized
// stack slot holds leftover garbage from the preceding scanf() call: a non-NULL
// pointer whose first byte happens to be '\0'.  printLine() therefore passes
// the NULL check and printf("%s\n", line) emits a single '\n'.  That observed
// behavior is reproduced faithfully below (an empty, non-NULL string), rather
// than "fixed".

use std::io::{Read, Write};

/// Mirrors `void printLine(const char *line)`.
///
/// `None` models a NULL pointer, `Some(s)` a valid C string.
fn print_line(line: Option<&str>, out: &mut impl Write) {
    if let Some(line) = line {
        // printf("%s\n", line);
        let _ = write!(out, "{}\n", line);
    }
}

/// Mirrors `void bad()`: `char *data;` is never initialized.
///
/// See the note above: the unoptimized C build reads a non-NULL pointer that
/// references an empty string, so exactly one newline is printed.
fn bad(out: &mut impl Write) {
    let data: Option<&str> = Some("");
    print_line(data, out);
}

/// Mirrors `void good()`: `data = "string";`
fn good(out: &mut impl Write) {
    let data: Option<&str> = Some("string");
    print_line(data, out);
}

/// Reads a single byte from stdin, or `None` on EOF.
fn read_byte(stdin: &mut impl Read) -> Option<u8> {
    let mut b = [0u8; 1];
    loop {
        match stdin.read(&mut b) {
            Ok(0) => return None,
            Ok(_) => return Some(b[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// True for the characters that C's `isspace()` (C locale) accepts; these are
/// the ones a scanf conversion skips before a `%d` directive.
fn is_c_space(b: u8) -> bool {
    match b {
        b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => true,
        _ => false,
    }
}

/// Emulates `scanf("%d", &x)` as implemented by glibc.
///
/// Returns `Some(value)` when the conversion succeeds (scanf returned 1) and
/// `None` on a matching failure or EOF, in which case the caller leaves its
/// variable untouched (the C code initializes `x` to 0).
///
/// glibc accumulates the digits into a `long`, saturating at LONG_MIN/LONG_MAX
/// on overflow, then stores the low 32 bits into the `int` argument.  E.g.
/// "99999999999999999999" yields -1 and "4294967296" yields 0.
fn scanf_int(stdin: &mut impl Read) -> Option<i32> {
    // Skip leading whitespace (scanf crosses newlines while doing so).
    let mut cur = loop {
        match read_byte(stdin) {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return None, // EOF before any input: scanf returns EOF
        }
    };

    // Optional sign.
    let negative = match cur {
        b'-' | b'+' => {
            let neg = cur == b'-';
            match read_byte(stdin) {
                Some(b) => cur = b,
                None => return None, // sign then EOF: matching failure
            }
            neg
        }
        _ => false,
    };

    if !cur.is_ascii_digit() {
        // No digits at all: matching failure, nothing is stored.
        return None;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        if !saturated {
            let digit = i64::from(cur - b'0');
            acc = match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            }) {
                Some(v) => v,
                None => {
                    saturated = true;
                    if negative {
                        i64::MIN
                    } else {
                        i64::MAX
                    }
                }
            };
        }

        match read_byte(stdin) {
            Some(b) if b.is_ascii_digit() => cur = b,
            // The non-digit terminator is pushed back by scanf; nothing else
            // reads stdin afterwards, so there is no observable difference.
            _ => break,
        }
    }

    // Truncate the long to int, exactly as the %d store does.
    Some(acc as i32)
}

fn main() {
    let mut stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&mut stdin) {
        x = v;
    }

    if x != 0 {
        good(&mut out);
    } else {
        bad(&mut out);
    }

    let _ = out.flush();
}
