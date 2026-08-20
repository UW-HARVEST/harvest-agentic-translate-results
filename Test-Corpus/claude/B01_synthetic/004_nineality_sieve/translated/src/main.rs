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

//! Count from a starting point,
//! stopping when the count ends in 9 (base 10).
//!
//! Rust translation of the original C `driver` program. Behavior (including
//! quirks such as C's truncating `%` for negative operands and the truncation
//! of `strtol`'s `long` result into an `int`) is reproduced exactly.

use std::ffi::OsString;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

/// Returns the raw bytes of a command line argument, the way C's `argv` sees them.
fn arg_bytes(arg: &OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().into_owned().into_bytes()
    }
}

/// `isspace` for the C locale, as used by `strtol` when skipping leading blanks.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful port of `strtol(s, &end, 10)`.
///
/// Returns the parsed `long` value together with the offset that C would store
/// into `end`. When no conversion can be performed the offset is `0`, which is
/// how the caller detects `end == argv[1]`.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Digits.
    let digits_start = i;
    let mut acc: u64 = 0;
    let mut overflow = false;
    // Magnitude limit: LONG_MAX for positives, |LONG_MIN| for negatives.
    let limit: u64 = if negative {
        i64::MIN.unsigned_abs()
    } else {
        i64::MAX as u64
    };

    while i < s.len() && s[i].is_ascii_digit() {
        let digit = u64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) if v <= limit => acc = v,
                _ => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: no conversion performed, `end` == start of string.
        return (0, 0);
    }

    let value = if overflow {
        // strtol clamps and sets ERANGE; the clamped value is what the caller sees.
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

    (value, i)
}

/// Rust's runtime ignores `SIGPIPE`; a C program does not. Restore the default
/// disposition so that a closed stdout terminates this program exactly as it
/// terminated the original binary.
#[cfg(unix)]
fn reset_sigpipe() {
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
fn reset_sigpipe() {}

fn main() -> ExitCode {
    reset_sigpipe();

    let argv: Vec<OsString> = std::env::args_os().collect();
    let argc = argv.len();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    if argc != 2 {
        let _ = out.write_all(b"Error: should only be a single (integer) argument!\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    let arg = arg_bytes(&argv[1]);
    let (parsed, end) = strtol_base10(&arg);
    // `int val = strtol(...)` truncates the long result to 32 bits.
    let mut val = parsed as i32;
    if end == 0 {
        // end is set to start of string if nothing parsed
        let _ = out.write_all(b"Error: first argument must be an integer!\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    let mut buf = itoa_i32();
    loop {
        let _ = out.write_all(format_i32(&mut buf, val));
        if val % 10 == 9 {
            break;
        }
        // Signed overflow is undefined in C; reproduce the wraparound that the
        // original binary exhibits on two's-complement hardware.
        val = val.wrapping_add(1);
    }

    let _ = out.flush();
    ExitCode::SUCCESS
}

/// Scratch buffer for decimal formatting: at most 11 digits/sign plus a newline.
fn itoa_i32() -> [u8; 12] {
    [0u8; 12]
}

/// Renders `val` exactly as `printf("%d\n", val)` would, into `buf`.
fn format_i32(buf: &mut [u8; 12], val: i32) -> &[u8] {
    let mut tmp = [0u8; 11];
    let mut n = 0usize;
    let mut magnitude = (val as i64).unsigned_abs();

    if magnitude == 0 {
        tmp[n] = b'0';
        n += 1;
    } else {
        while magnitude > 0 {
            tmp[n] = b'0' + (magnitude % 10) as u8;
            magnitude /= 10;
            n += 1;
        }
    }

    let mut len = 0usize;
    if val < 0 {
        buf[len] = b'-';
        len += 1;
    }
    for k in (0..n).rev() {
        buf[len] = tmp[k];
        len += 1;
    }
    buf[len] = b'\n';
    len += 1;

    &buf[..len]
}
