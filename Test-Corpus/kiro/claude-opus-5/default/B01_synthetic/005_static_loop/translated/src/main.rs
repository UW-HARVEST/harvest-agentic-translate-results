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

thread_local! {
    /// Mirrors `static int sum = 0;` inside `static_sum` in the C source.
    static SUM: Cell<i32> = const { Cell::new(0) };
}

/// C:
/// ```c
/// int static_sum(int update) {
///   static int sum = 0;
///   sum += update;
///   return sum;
/// }
/// ```
/// Signed overflow is undefined behavior in C; gcc/clang wrap on two's
/// complement targets, so `wrapping_add` reproduces the observed behavior.
fn static_sum(update: i32) -> i32 {
    SUM.with(|sum| {
        let next = sum.get().wrapping_add(update);
        sum.set(next);
        next
    })
}

/// True for the characters `isspace()` accepts in the C locale.
fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Faithful `strtol(nptr, &end, 10)`.
///
/// Returns the parsed `long` value plus the offset that `end` would point at.
/// An offset of 0 means nothing was parsed (C sets `end == nptr`). On range
/// overflow the value saturates to `LONG_MAX`/`LONG_MIN`, matching glibc (the
/// C code never inspects `errno`, so saturation is all that is observable).
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: i64 = 0;
    let mut saturated = false;

    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !saturated {
            // Accumulate the magnitude in the negative domain so that
            // LONG_MIN is representable without overflowing.
            match acc.checked_mul(10).and_then(|v| v.checked_sub(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: value is 0 and end is reset to the start.
        return (0, 0);
    }

    let value = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        acc
    } else {
        // `-acc` cannot overflow here: an unnegatable magnitude would have
        // been flagged as saturated above.
        match acc.checked_neg() {
            Some(v) => v,
            None => i64::MAX,
        }
    };

    (value, i)
}

/// Bytes of a command-line argument exactly as the C `char *` would see them.
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

/// Maintain a running total using a static variable
fn main() -> ExitCode {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = argv.len();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if argc != 2 {
        let _ = out.write_all(b"Error: should only be a single (integer) argument!\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    let arg1 = arg_bytes(&argv[1]);
    let (parsed, end) = strtol_base10(&arg1);

    // `int stride = strtol(...)`: truncate long -> int as gcc does.
    let stride = parsed as i32;

    if end == 0 {
        // end is set to start of string if nothing parsed
        let _ = out.write_all(b"Error: first argument must be an integer!\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    for i in 0..10i32 {
        // `i * stride` is int multiplication in C; wrap like gcc does.
        let value = static_sum(i.wrapping_mul(stride));
        let _ = writeln!(out, "{}", value);
    }

    let _ = out.flush();
    ExitCode::from(0)
}
