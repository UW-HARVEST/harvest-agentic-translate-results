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
//! The translation is deliberately *pointer faithful*: it operates on the very
//! same `int argc, char **argv` pair the C `main` receives, so that every
//! observable behavior of the original -- including its pointer comparisons,
//! its signed/unsigned comparison quirks and its `long`-to-`int` truncation --
//! is reproduced bit for bit.

use core::ffi::{c_char, c_int, c_long};
use std::io::Write;

// ---------------------------------------------------------------------------
// Small C library routines, reimplemented in Rust
// ---------------------------------------------------------------------------

/// Read one byte exactly the way the C code does: an ordinary load that simply
/// faults when the pointer is invalid.
///
/// `read_volatile` (rather than `*p`) is deliberate. A plain dereference is
/// preceded by a Rust-inserted null-pointer *check* in builds with debug
/// assertions, which aborts the process (`SIGABRT`) instead of faulting; the C
/// has no such check and dies with `SIGSEGV`. The volatile load reproduces the
/// C behavior in every profile.
///
/// # Safety
/// Same as a C load through `s`.
#[inline]
unsafe fn load(s: *const c_char) -> u8 {
    core::ptr::read_volatile(s) as u8
}

/// `strlen(3)`.
///
/// # Safety
/// `s` must point to a NUL-terminated byte string.
pub unsafe fn strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while load(s.add(n)) != 0 {
        n += 1;
    }
    n
}

/// `strnlen(3)`: length of `s`, but never scanning more than `max` bytes.
///
/// # Safety
/// `s` must point to a NUL-terminated byte string, or to at least `max` bytes.
unsafe fn strnlen(s: *const c_char, max: usize) -> usize {
    let mut n: usize = 0;
    while n < max && load(s.add(n)) != 0 {
        n += 1;
    }
    n
}

/// Result of a `strtol(nptr, &end, 10)` call.
pub struct Strtol {
    /// The converted value, as returned by C `strtol` (a `long`).
    pub value: c_long,
    /// The value C stores through `endptr`. Equal to `nptr` when no conversion
    /// could be performed.
    pub end: *mut c_char,
}

/// Faithful emulation of C `strtol(s, &end, 10)`.
///
/// * Skips leading whitespace as recognized by `isspace` in the C locale.
/// * Accepts an optional `+`/`-` sign.
/// * Accepts base-10 digits.
/// * On overflow, saturates to `LONG_MAX` / `LONG_MIN` (C also sets `errno`,
///   which the original program never inspects).
/// * If no digits are present, performs no conversion and reports `end == s`.
///
/// # Safety
/// `s` must point to a NUL-terminated byte string.
pub unsafe fn strtol_base10(s: *const c_char) -> Strtol {
    let mut i: usize = 0;

    // isspace() in the "C" locale: space, \t, \n, \v, \f, \r
    loop {
        let c = load(s.add(i));
        if matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            i += 1;
        } else {
            break;
        }
    }

    let mut negative = false;
    let c = load(s.add(i));
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        i += 1;
    }

    let digits_start = i;
    // Accumulate the magnitude in u64 so that LONG_MIN is representable.
    let mut magnitude: u64 = 0;
    let mut overflowed = false;
    loop {
        let c = load(s.add(i));
        if !c.is_ascii_digit() {
            break;
        }
        let digit = u64::from(c - b'0');
        if !overflowed {
            match magnitude
                .checked_mul(10)
                .and_then(|acc| acc.checked_add(digit))
            {
                Some(acc) => magnitude = acc,
                None => overflowed = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: C leaves *endptr == nptr and returns 0.
        return Strtol {
            value: 0,
            end: s as *mut c_char,
        };
    }

    // Saturate exactly like strtol does at the LONG_MAX / LONG_MIN boundaries.
    let limit: u64 = if negative {
        1u64 << 63 // magnitude of LONG_MIN
    } else {
        c_long::MAX as u64
    };

    let value = if overflowed || magnitude > limit {
        if negative {
            c_long::MIN
        } else {
            c_long::MAX
        }
    } else if negative {
        // Wrapping negate handles magnitude == 2^63 (LONG_MIN) correctly.
        magnitude.wrapping_neg() as c_long
    } else {
        magnitude as c_long
    };

    Strtol {
        value,
        end: (s as *mut c_char).add(i),
    }
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

/// `printf("<literal>")` for the fixed message strings of the program.
fn put(bytes: &[u8]) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}

/// `printf("%.*s\n", precision, s)`.
///
/// A negative precision is taken as if the precision were omitted (C99
/// 7.19.6.1p5), i.e. the whole string is printed.
///
/// # Safety
/// `s` must point to a NUL-terminated byte string.
unsafe fn put_precision_str_nl(precision: c_int, s: *const c_char) {
    let n = if precision < 0 {
        strlen(s)
    } else {
        strnlen(s, precision as usize)
    };
    let bytes = core::slice::from_raw_parts(s as *const u8, n);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(bytes);
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}

// ---------------------------------------------------------------------------
// The program itself
// ---------------------------------------------------------------------------

/// Translation of the C `main`.
///
/// Returns the value the C `main` returns; it never calls `exit()`, exactly
/// like the original, so it is safe to invoke through the FFI boundary.
///
/// # Safety
/// `argv` must be the `char **argv` a C `main` would receive: an array of
/// NUL-terminated strings whose element `argc` is NULL. The function reads
/// `argv[1]`, `argv[2]` and `argv[3]` under exactly the same conditions the C
/// does -- including the case `argc == 0`, where the C reads past `argv[0]`.
pub unsafe fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if (argc > 4) || (argc == 1) {
        put(b"Error: there should be one to three arguments passed:\n");
        put(b"<string> [start] [stop]\n");
        return 1;
    }

    let argv1 = *argv.add(1);
    // size_t len = strlen(argv[1]);
    let len: usize = strlen(argv1);

    // int start, stop;  -- deliberately 32-bit, as in the C.
    let start: c_int;
    let stop: c_int;

    // `char *end;` is uninitialized in the C. It is only ever *read* on paths
    // where argc >= 3, and on those paths strtol() has already written it.
    let mut end: *mut c_char = core::ptr::null_mut();

    if argc >= 3 {
        let argv2 = *argv.add(2);
        let parsed = strtol_base10(argv2);
        // strtol returns long; assigning to int truncates (gcc: wraps).
        start = parsed.value as c_int;
        end = parsed.end;

        if end == argv2 {
            // note the C omits the trailing newline here.
            put(b"Second argument must be an integer!");
            return 1;
        }

        // `start > len` compares int against size_t. The usual arithmetic
        // conversions promote `start` to an unsigned 64-bit value, so any
        // NEGATIVE start becomes enormous and trips this check. Preserved.
        if (start as i64 as u64) > (len as u64) {
            put(b"Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let argv3 = *argv.add(3);
        // The C passes NULL as endptr here, so `end` is NOT updated by this
        // call and still points into argv[2].
        stop = strtol_base10(argv3).value as c_int;

        // `if (end == argv[3])` compares a pointer into argv[2] against the
        // start of argv[3]. Reproduced as the very same pointer comparison.
        if end == argv3 {
            put(b"Third argument must be an integer!");
            return 1;
        }

        // Same signed/unsigned trap as for `start`: a negative stop compares
        // greater than len and reports being "off the end".
        if (stop as i64 as u64) > (len as u64) {
            put(b"Error: stop is off the end of the string!\n");
            return 1;
        }

        if stop <= start {
            put(b"Error: stop must come after start!\n");
            return 1;
        }
    } else {
        // single-line else statement just to make style checking sad
        // `stop = len` narrows size_t to int.
        stop = len as c_int;
    }

    /* char arithmetic: skip ahead `start` characters in the array */
    // printf("%.*s\n", stop - start, argv[1] + start);
    put_precision_str_nl(
        stop.wrapping_sub(start),
        argv1.offset(start as isize) as *const c_char,
    );

    0
}
