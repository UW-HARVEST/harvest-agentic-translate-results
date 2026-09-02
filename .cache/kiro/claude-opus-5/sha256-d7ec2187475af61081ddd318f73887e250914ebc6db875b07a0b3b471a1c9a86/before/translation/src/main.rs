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

//! Index into a passed string and print the substring indexed by
//! `[start, stop)`.  If there is no start, use 0.  If there is no stop, use the
//! end of the string.
//!
//! Faithful Rust translation of the original C `main.c`, including its bugs:
//!
//! * `start`/`stop` are C `int`s, so an out-of-range `strtol` result is
//!   truncated to 32 bits.
//! * `start > len` and `stop > len` compare an `int` against a `size_t`, so the
//!   usual arithmetic conversions turn a negative index into a huge unsigned
//!   value and the "off the end of the string" branch is taken.
//! * The third argument's `strtol` call passes `NULL` for `endptr`, so the
//!   subsequent `end == argv[3]` test inspects the stale pointer left over from
//!   parsing `argv[2]`.  Because distinct `argv` strings are distinct objects
//!   that comparison can never be true, so the "Third argument must be an
//!   integer!" message is unreachable.  That is reproduced here.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

#[cfg(unix)]
fn arg_bytes(arg: &OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    arg.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn arg_bytes(arg: &OsString) -> Vec<u8> {
    arg.to_string_lossy().into_owned().into_bytes()
}

/// C `isspace` for the default ("C") locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Base-10 `strtol`.
///
/// Returns the converted `long` value together with the index at which
/// conversion stopped, i.e. the offset that C would store through `endptr`.
/// An index of 0 means no conversion could be performed (`end == nptr`).
/// The value saturates at `LONG_MIN`/`LONG_MAX` on overflow, matching glibc.
fn strtol(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    while i < s.len() && is_space(s[i]) {
        i += 1;
    }

    let negative = match s.get(i) {
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

    let digits_start = i;
    // Accumulate in the negative direction so that LONG_MIN is representable.
    let mut acc: i64 = 0;
    let mut overflow = false;

    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|a| a.checked_sub(digit)) {
                Some(next) => acc = next,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: no conversion performed, endptr == nptr.
        return (0, 0);
    }

    // A positive magnitude of exactly 2^63 fits in the negated accumulator but
    // is still out of range for a positive `long`.
    if !negative && acc == i64::MIN {
        overflow = true;
    }

    if overflow {
        return (if negative { i64::MIN } else { i64::MAX }, i);
    }

    // `acc` currently holds the negated magnitude.
    let value = if negative {
        acc
    } else {
        // Safe: a non-overflowing positive magnitude always negates back.
        acc.wrapping_neg()
    };

    (value, i)
}

fn run(out: &mut dyn Write) -> u8 {
    let argv: Vec<OsString> = std::env::args_os().collect();
    let argc = argv.len();

    if (argc > 4) || (argc == 1) {
        let _ = out.write_all(b"Error: there should be one to three arguments passed:\n");
        let _ = out.write_all(b"<string> [start] [stop]\n");
        return 1;
    }

    let subject = arg_bytes(&argv[1]);
    // strlen(argv[1]): byte length of the NUL-terminated string.
    let len: u64 = subject.len() as u64;

    let start: i32;
    let stop: i32;

    // `char *end;` — set only by the argv[2] conversion below.
    let mut end: usize = 0;
    let mut end_initialized = false;

    if argc >= 3 {
        let second = arg_bytes(&argv[2]);
        let (value, end_index) = strtol(&second);
        end = end_index;
        end_initialized = true;
        // long -> int conversion: truncate to 32 bits.
        start = value as i32;
        if end == 0 {
            // `end == argv[2]`: nothing was converted.  Note: no newline.
            let _ = out.write_all(b"Second argument must be an integer!");
            return 1;
        }
        // int compared against size_t: `start` is converted to u64.
        if (start as u64) > len {
            let _ = out.write_all(b"Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let third = arg_bytes(&argv[3]);
        // The C code passes NULL as endptr here, so `end` keeps its old value.
        let (value, _ignored_end) = strtol(&third);
        stop = value as i32;

        // `end == argv[3]` compares the stale pointer into argv[2] against
        // argv[3]; distinct argv strings never alias, so this is always false.
        let _ = end;
        let stale_end_matches_third_arg = false && end_initialized;
        if stale_end_matches_third_arg {
            let _ = out.write_all(b"Third argument must be an integer!");
            return 1;
        }

        if (stop as u64) > len {
            let _ = out.write_all(b"Error: stop is off the end of the string!\n");
            return 1;
        }

        if stop <= start {
            let _ = out.write_all(b"Error: stop must come after start!\n");
            return 1;
        }
    } else {
        // size_t -> int conversion: truncate to 32 bits.
        stop = len as i32;
    }

    // printf("%.*s\n", stop - start, argv[1] + start)
    let precision = stop.wrapping_sub(start);
    let offset = start as usize;
    let tail = &subject[offset..];
    let slice: &[u8] = if precision < 0 {
        // A negative precision is treated by printf as if it were omitted.
        tail
    } else {
        let n = (precision as usize).min(tail.len());
        &tail[..n]
    };
    let _ = out.write_all(slice);
    let _ = out.write_all(b"\n");

    0
}

fn main() -> ExitCode {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let code = run(&mut lock);
    let _ = lock.flush();
    ExitCode::from(code)
}
