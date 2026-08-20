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

use std::cell::Cell;
use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

thread_local! {
    // Mirrors `static int sum = 0;` inside static_sum()
    static SUM: Cell<i32> = const { Cell::new(0) };
}

fn static_sum(update: i32) -> i32 {
    SUM.with(|sum| {
        // C: sum += update;  (int arithmetic, wraps on typical implementations)
        let new = sum.get().wrapping_add(update);
        sum.set(new);
        new
    })
}

/// Returns `true` for the characters that C's `isspace()` accepts in the
/// default ("C") locale, which `strtol` skips before the number.
fn is_c_space(b: u8) -> bool {
    b == b' ' || (0x09..=0x0d).contains(&b)
}

/// Faithful re-implementation of `strtol(nptr, &end, 10)`.
///
/// Returns the converted value together with the byte offset that C would
/// store into `end`. When no conversion can be performed the offset is 0,
/// i.e. `end == nptr`.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading white space.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Digits. Saturate like strtol does (returns LONG_MAX / LONG_MIN).
    let cutoff: u64 = if negative {
        (i64::MAX as u64) + 1 // magnitude of LONG_MIN
    } else {
        i64::MAX as u64
    };
    let cutlim = cutoff % 10;
    let cutdiv = cutoff / 10;

    let digits_start = i;
    let mut acc: u64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = u64::from(s[i] - b'0');
        if overflow || acc > cutdiv || (acc == cutdiv && digit > cutlim) {
            overflow = true;
        } else {
            acc = acc * 10 + digit;
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: no conversion performed, end == nptr.
        return (0, 0);
    }

    let value = if overflow {
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

/*
  Maintain a running total using a static variable
 */
fn main() -> ExitCode {
    let argv: Vec<OsString> = std::env::args_os().collect();
    let argc = argv.len();

    let mut out = std::io::stdout();

    if argc != 2 {
        let _ = out.write_all(b"Error: should only be a single (integer) argument!\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    let raw = arg_bytes(&argv[1]);
    let (parsed, end) = strtol_base10(&raw);
    if end == 0 {
        // end is set to start of string if nothing parsed
        let _ = out.write_all(b"Error: first argument must be an integer!\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    // C truncates the `long` returned by strtol into an `int`.
    let stride: i32 = parsed as i32;

    for i in 0..10i32 {
        let line = format!("{}\n", static_sum(i.wrapping_mul(stride)));
        let _ = out.write_all(line.as_bytes());
    }
    let _ = out.flush();

    ExitCode::SUCCESS
}
