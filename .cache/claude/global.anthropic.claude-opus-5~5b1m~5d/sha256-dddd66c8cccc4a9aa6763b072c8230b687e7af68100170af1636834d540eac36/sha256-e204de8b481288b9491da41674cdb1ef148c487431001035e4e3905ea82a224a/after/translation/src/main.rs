// Translation of c_src/src/main.c (StaticLoop driver) to Rust.
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

use std::cell::Cell;
use std::io::Write;
use std::process::ExitCode;

thread_local! {
    /// Mirrors `static int sum = 0;` inside `static_sum`.
    static SUM: Cell<i32> = const { Cell::new(0) };
}

/// int static_sum(int update) { static int sum = 0; sum += update; return sum; }
fn static_sum(update: i32) -> i32 {
    SUM.with(|sum| {
        let new = sum.get().wrapping_add(update);
        sum.set(new);
        new
    })
}

/// Is this byte whitespace according to the C locale `isspace`?
fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulation of `strtol(nptr, &end, 10)`.
///
/// Returns the parsed value (saturated to LONG_MIN/LONG_MAX on overflow, as C
/// does) together with the number of bytes consumed. A consumed count of zero
/// means `end == nptr`, i.e. nothing was parsed.
fn c_strtol_base10(nptr: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < nptr.len() && c_isspace(nptr[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < nptr.len() && (nptr[i] == b'+' || nptr[i] == b'-') {
        negative = nptr[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: u64 = 0;
    let mut overflow = false;

    // Positive magnitude limit; for negatives the magnitude may reach 2^63.
    let cutoff: u64 = if negative {
        i64::MIN.unsigned_abs()
    } else {
        i64::MAX as u64
    };

    while i < nptr.len() && nptr[i].is_ascii_digit() {
        let digit = u64::from(nptr[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) if v <= cutoff => acc = v,
                _ => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits converted: strtol returns 0 and sets end back to nptr.
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

fn arg_bytes(arg: &std::ffi::OsString) -> Vec<u8> {
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

fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = args.len();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if argc != 2 {
        let _ = out.write_all(b"Error: should only be a single (integer) argument!\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    let raw = arg_bytes(&args[1]);
    let (parsed, consumed) = c_strtol_base10(&raw);
    if consumed == 0 {
        // end is set to start of string if nothing parsed
        let _ = out.write_all(b"Error: first argument must be an integer!\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    // `int stride = strtol(...)` truncates the long to int.
    let stride = parsed as i32;

    let mut i: i32 = 0;
    while i < 10 {
        let value = static_sum(i.wrapping_mul(stride));
        let _ = out.write_all(format!("{}\n", value).as_bytes());
        i += 1;
    }

    let _ = out.flush();
    ExitCode::from(0)
}
