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
//! Index into a passed string and print the substring indexed by [start, stop).
//! If there is no start, use 0.
//! If there is no stop, use the end of the string.
//!
//! This is a faithful, byte-for-byte-compatible translation. Several
//! questionable behaviors of the original C are intentionally preserved; see
//! the comments at each site.

use std::ffi::OsString;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

/// Result of a `strtol(nptr, &end, 10)` call.
struct StrtolResult {
    /// The converted value, as returned by C `strtol` (a `long`).
    value: i64,
    /// Number of bytes consumed, i.e. `end - nptr`. Zero means "no conversion
    /// could be performed", which in C is signalled by `end == nptr`.
    consumed: usize,
}

/// Faithful emulation of C `strtol(s, &end, 10)`.
///
/// * Skips leading whitespace as recognized by `isspace` in the C locale.
/// * Accepts an optional `+`/`-` sign.
/// * Accepts base-10 digits.
/// * On overflow, saturates to `LONG_MAX` / `LONG_MIN` (C also sets `errno`,
///   which the original program never inspects).
/// * If no digits are present, performs no conversion and reports
///   `consumed == 0` so the caller can reproduce the `end == nptr` test.
fn strtol_base10(s: &[u8]) -> StrtolResult {
    let mut i: usize = 0;

    // isspace() in the "C" locale: space, \t, \n, \v, \f, \r
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    // Accumulate the magnitude in u64 so that LONG_MIN is representable.
    let mut magnitude: u64 = 0;
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = u64::from(s[i] - b'0');
        if !overflowed {
            let next = magnitude
                .checked_mul(10)
                .and_then(|acc| acc.checked_add(digit));
            if let Some(acc) = next {
                magnitude = acc;
            } else {
                overflowed = true;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: C leaves *endptr == nptr and returns 0.
        return StrtolResult {
            value: 0,
            consumed: 0,
        };
    }

    // Saturate exactly like strtol does at the LONG_MAX / LONG_MIN boundaries.
    let limit: u64 = if negative {
        1u64 << 63 // magnitude of LONG_MIN
    } else {
        i64::MAX as u64
    };

    let value = if overflowed || magnitude > limit {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // Wrapping negate handles magnitude == 2^63 (LONG_MIN) correctly.
        magnitude.wrapping_neg() as i64
    } else {
        magnitude as i64
    };

    StrtolResult {
        value,
        consumed: i,
    }
}

/// Bytes of a command-line argument, as C would see them via `char *`.
#[cfg(unix)]
fn arg_bytes(arg: &OsString) -> Vec<u8> {
    arg.as_bytes().to_vec()
}

/// Non-Unix fallback: arguments are recovered from their UTF-8 encoding.
#[cfg(not(unix))]
fn arg_bytes(arg: &OsString) -> Vec<u8> {
    arg.to_string_lossy().into_owned().into_bytes()
}

fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    // C's argc counts argv[0] (the program name) just like args_os() does.
    let argc = args.len();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if (argc > 4) || (argc == 1) {
        let _ = out.write_all(b"Error: there should be one to three arguments passed:\n");
        let _ = out.write_all(b"<string> [start] [stop]\n");
        let _ = out.flush();
        std::process::exit(1);
    }

    // argc == 0 (no argv[0] at all) makes the C dereference argv[1] == NULL.
    // Reproduce the resulting crash rather than inventing a friendlier result.
    if argc == 0 {
        let _ = out.flush();
        std::process::abort();
    }

    let arg1 = arg_bytes(&args[1]);
    // size_t len = strlen(argv[1]);
    let len: usize = arg1.len();

    // int start, stop;  -- deliberately 32-bit, as in the C.
    let start: i32;
    let stop: i32;

    // `char *end;` is uninitialized in the C. It is only ever assigned by the
    // strtol() call below, which runs whenever argc >= 3.
    let mut end_consumed: Option<usize> = None;

    if argc >= 3 {
        let arg2 = arg_bytes(&args[2]);
        let parsed = strtol_base10(&arg2);
        // strtol returns long; assigning to int truncates (gcc: wraps).
        start = parsed.value as i32;
        end_consumed = Some(parsed.consumed);

        if parsed.consumed == 0 {
            // end == argv[2]: note the C omits the trailing newline here.
            let _ = out.write_all(b"Second argument must be an integer!");
            let _ = out.flush();
            std::process::exit(1);
        }

        // `start > len` compares int against size_t. The usual arithmetic
        // conversions promote `start` to an unsigned 64-bit value, so any
        // NEGATIVE start becomes enormous and trips this check. Preserved.
        if (i64::from(start) as u64) > (len as u64) {
            let _ = out.write_all(b"Error: start is off the end of the string!\n");
            let _ = out.flush();
            std::process::exit(1);
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let arg3 = arg_bytes(&args[3]);
        // The C passes NULL as endptr here, so `end` is NOT updated by this
        // call and still points into argv[2].
        stop = strtol_base10(&arg3).value as i32;

        // `if (end == argv[3])` therefore compares a pointer into argv[2]
        // against the start of argv[3]. Since argc == 4 implies argc >= 3, end
        // was set above to argv[2] + consumed with 0 < consumed <=
        // strlen(argv[2]), so it can never equal argv[3]. This branch is dead
        // code and its message is unreachable; a non-integer third argument
        // silently yields stop == 0 instead. Preserved by never taking it.
        //
        // (Verified by differential testing against the C: the string "Third
        // argument must be an integer!" is never produced for any input.)
        let _end_points_into_argv2 = end_consumed;

        // Same signed/unsigned trap as for `start`: a negative stop compares
        // greater than len and reports being "off the end".
        if (i64::from(stop) as u64) > (len as u64) {
            let _ = out.write_all(b"Error: stop is off the end of the string!\n");
            let _ = out.flush();
            std::process::exit(1);
        }

        if stop <= start {
            let _ = out.write_all(b"Error: stop must come after start!\n");
            let _ = out.flush();
            std::process::exit(1);
        }
    } else {
        // single-line else statement just to make style checking sad
        // `stop = len` narrows size_t to int.
        stop = len as i32;
    }

    // char arithmetic: skip ahead `start` characters in the array.
    // printf("%.*s\n", stop - start, argv[1] + start);
    // The checks above guarantee 0 <= start <= len and 0 <= stop - start, so
    // the precision is never negative here.
    let offset = start as usize;
    let precision = stop.wrapping_sub(start);
    let tail = &arg1[offset.min(arg1.len())..];
    // "%.*s" prints at most `precision` bytes, stopping early at a NUL. C
    // strings from argv contain no interior NUL, so the length cap suffices.
    let take = if precision <= 0 {
        0
    } else {
        (precision as usize).min(tail.len())
    };
    let _ = out.write_all(&tail[..take]);
    let _ = out.write_all(b"\n");
    let _ = out.flush();

    std::process::exit(0);
}
