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

use std::cell::Cell;
use std::io::Write;
use std::process::ExitCode;

/// Mirrors `int static_sum(int update)` with its function-local `static int sum = 0;`.
///
/// The C `static` lives for the whole process, so a thread-local `Cell` gives the
/// same observable behavior for this single-threaded program while staying in
/// safe Rust.
mod static_sum_state {
    use std::cell::Cell;
    thread_local! {
        pub static SUM: Cell<i32> = const { Cell::new(0) };
    }
}

fn static_sum(update: i32) -> i32 {
    static_sum_state::SUM.with(|sum: &Cell<i32>| {
        // `sum += update;` on `int`. Signed overflow is UB in C but wraps in
        // practice on the usual targets; reproduce the wrapping result.
        let new = sum.get().wrapping_add(update);
        sum.set(new);
        new
    })
}

/// Faithful re-implementation of C `strtol(nptr, &end, 10)`.
///
/// Returns `(value, consumed)` where `consumed` is the number of bytes the
/// equivalent of `end` would have advanced past the start of `nptr`. Per the C
/// standard, if no conversion is performed `consumed` is 0 (i.e. `end == nptr`),
/// which is exactly what the original program tests for.
fn strtol_base10(nptr: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading white space, as recognized by isspace() in the "C" locale.
    while i < nptr.len() && matches!(nptr[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    // Optional sign.
    let negative = match nptr.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };

    // Digit sequence.
    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < nptr.len() && nptr[i].is_ascii_digit() {
        let d = i64::from(nptr[i] - b'0');
        if !overflow {
            // Accumulate the magnitude, clamping like strtol does on ERANGE.
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            }) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits converted: strtol stores nptr in *endptr and returns 0.
        return (0, 0);
    }

    if overflow {
        // strtol returns LONG_MAX / LONG_MIN and sets errno to ERANGE.
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    (acc, i)
}

fn main() -> ExitCode {
    // `argc` / `argv` as the C program sees them.
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = argv.len();

    if argc != 2 {
        print!("Error: should only be a single (integer) argument!\n");
        flush();
        return ExitCode::from(1);
    }

    let arg1: &[u8] = os_str_bytes(&argv[1]);

    let (parsed, consumed) = strtol_base10(arg1);
    // `int stride = strtol(...)`: implicit long -> int conversion truncates.
    let stride: i32 = parsed as i32;

    if consumed == 0 {
        // end is set to start of string if nothing parsed
        print!("Error: first argument must be an integer!\n");
        flush();
        return ExitCode::from(1);
    }

    for i in 0..10i32 {
        // `i * stride` on `int`; reproduce wrapping on overflow.
        print!("{}\n", static_sum(i.wrapping_mul(stride)));
    }

    flush();
    ExitCode::from(0)
}

fn flush() {
    let _ = std::io::stdout().flush();
}

#[cfg(unix)]
fn os_str_bytes(s: &std::ffi::OsString) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    s.as_os_str().as_bytes()
}

#[cfg(not(unix))]
fn os_str_bytes(s: &std::ffi::OsString) -> &[u8] {
    // On non-unix targets argv is Unicode; the lossy path never allocates for
    // valid UTF-8, and only the leading ASCII bytes matter to strtol.
    match s.to_str() {
        Some(v) => v.as_bytes(),
        None => b"",
    }
}
