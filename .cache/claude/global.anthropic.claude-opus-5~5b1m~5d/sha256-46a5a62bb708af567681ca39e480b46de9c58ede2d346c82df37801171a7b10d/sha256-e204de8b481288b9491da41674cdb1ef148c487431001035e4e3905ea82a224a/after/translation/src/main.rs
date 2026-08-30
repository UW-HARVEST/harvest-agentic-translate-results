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

/*
Count from a starting point,
stopping when the count ends in 9 (base 10).
*/

use std::ffi::OsString;
use std::io::{BufWriter, Write};

/// Mirrors C's `isspace()` in the default "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful re-implementation of `strtol(nptr, &end, 10)` for a byte string.
///
/// Returns the parsed `long` value (saturating at `LONG_MIN`/`LONG_MAX` like
/// glibc does when the input is out of range) plus the offset that C would
/// have stored in `end`. An offset of 0 means "nothing was parsed", which is
/// exactly the `end == argv[1]` condition the original program tests.
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
    // Magnitude limit: 2^63 - 1 for positives, 2^63 for negatives.
    let cutoff: u64 = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };

    while i < s.len() && s[i].is_ascii_digit() {
        let d = u64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) if v <= cutoff => acc = v,
                _ => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: C sets *end to the original pointer and
        // strtol returns 0.
        return (0, 0);
    }

    let value = if overflow {
        // glibc sets errno to ERANGE and returns LONG_MAX / LONG_MIN.
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // Handles the LONG_MIN magnitude (2^63) without overflowing.
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };

    (value, i)
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which a C
/// program does not do. Restore the default disposition so that writing to a
/// closed stdout terminates the process exactly like the original C binary
/// (e.g. `driver 0 | head -1`).
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

fn main() {
    reset_sigpipe();

    let args: Vec<OsString> = std::env::args_os().collect();
    let argc = args.len();

    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    // printf() failures are ignored by the original program, so ignore ours.
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

    let arg1 = arg_bytes(&args[1]);
    let (parsed, end) = strtol_base10(&arg1);
    // `int val = strtol(...)` truncates the long to 32 bits.
    let mut val: i32 = parsed as i32;

    if end == 0 {
        // end is set to start of string if nothing parsed
        emit!("Error: first argument must be an integer!\n");
        let _ = out.flush();
        std::process::exit(1);
    }

    loop {
        emit!("{}\n", val);
        if val % 10 == 9 {
            break;
        }
        // Signed overflow is UB in C; reproduce the wrap-around that the
        // compiled C actually performs on this target.
        val = val.wrapping_add(1);
    }

    let _ = out.flush();
    std::process::exit(0);
}
