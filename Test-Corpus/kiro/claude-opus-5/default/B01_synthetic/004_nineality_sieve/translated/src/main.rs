// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/main.c`.
//!
//! Count from a starting point,
//! stopping when the count ends in 9 (base 10).

use std::io::Write;
use std::os::unix::ffi::OsStrExt;

/// `long` on the LP64 target the C was built for.
type CLong = i64;

/// Mirrors the C-locale `isspace()` used by `strtol()` to skip leading
/// whitespace: space, tab, newline, vertical tab, form feed, carriage return.
fn is_c_space(b: u8) -> bool {
    b == b' ' || (0x09..=0x0d).contains(&b)
}

/// Faithful re-implementation of `strtol(nptr, &end, 10)`.
///
/// Returns the converted value together with the byte offset that C would
/// store into `end`. Per the C standard, when no conversion can be performed
/// the offset is 0 (i.e. `end == nptr`) and the value is 0. On overflow the
/// result saturates to `LONG_MAX` / `LONG_MIN`, exactly as the C library does
/// (the C code ignores `errno`, so we do too).
fn strtol_base10(nptr: &[u8]) -> (CLong, usize) {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < nptr.len() && is_c_space(nptr[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < nptr.len() && (nptr[i] == b'+' || nptr[i] == b'-') {
        negative = nptr[i] == b'-';
        i += 1;
    }

    let digits_start = i;

    // The magnitude limit differs for the negative side, just like in glibc.
    let limit: u128 = if negative {
        CLong::MIN.unsigned_abs() as u128
    } else {
        CLong::MAX as u128
    };

    let mut acc: u128 = 0;
    let mut overflowed = false;
    while i < nptr.len() && nptr[i].is_ascii_digit() {
        if !overflowed {
            acc = acc * 10 + u128::from(nptr[i] - b'0');
            if acc > limit {
                overflowed = true;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: no conversion performed, `end` is reset to the
        // start of the whole string (not to just past any whitespace/sign).
        return (0, 0);
    }

    let value = if overflowed {
        if negative {
            CLong::MIN
        } else {
            CLong::MAX
        }
    } else if negative {
        -(acc as i128) as CLong
    } else {
        acc as CLong
    };

    (value, i)
}

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = args.len();

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    // `printf` failures are not checked by the C code, so ignore write errors.
    macro_rules! emit {
        ($($arg:tt)*) => {
            let _ = write!(out, $($arg)*);
        };
    }

    if argc != 2 {
        emit!("Error: should only be a single (integer) argument!\n");
        let _ = out.flush();
        std::process::exit(1);
    }

    let arg1 = args[1].as_bytes();
    let (parsed, end_offset) = strtol_base10(arg1);
    if end_offset == 0 {
        // end is set to start of string if nothing parsed
        emit!("Error: first argument must be an integer!\n");
        let _ = out.flush();
        std::process::exit(1);
    }

    // C narrows the `long` result of strtol into an `int`; reproduce the
    // implementation-defined two's-complement truncation.
    let mut val = parsed as i32;

    loop {
        emit!("{}\n", val);
        // C's `%` truncates toward zero, so negative values never yield 9.
        if val % 10 == 9 {
            break;
        }
        // Signed overflow is UB in C; gcc/clang in practice wrap here.
        val = val.wrapping_add(1);
    }

    let _ = out.flush();
    std::process::exit(0);
}
