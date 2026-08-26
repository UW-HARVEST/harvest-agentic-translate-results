// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output to the original C program.

use std::env;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

/// Mimic C's strtol(nptr, &end, 10):
/// - Skips leading whitespace
/// - Optional sign
/// - Reads decimal digits
/// Returns (value as i32 [truncated like assignment to `int`],
///          number of bytes after which `end` would point,
///          whether any digits were parsed)
///
/// If no digits were parsed, the C `end` pointer equals `nptr` (i.e. parse failure).
fn c_strtol(s: &[u8]) -> (i32, bool) {
    let mut i = 0usize;
    // Skip whitespace as defined by C's isspace for the "C" locale.
    while i < s.len()
        && matches!(
            s[i],
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c
        )
    {
        i += 1;
    }

    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    // Use i64 with saturating arithmetic to mimic strtol's overflow clamping
    // before being truncated to int by the assignment.
    let mut val: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        val = val.saturating_mul(10).saturating_add(d);
        i += 1;
    }

    if i == digits_start {
        // No digits consumed -- C's strtol sets *endptr = nptr.
        return (0, false);
    }

    if neg {
        val = val.saturating_neg();
    }

    // Truncation to int (i32) on assignment to `int start`/`int stop` in C.
    (val as i32, true)
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().collect();
    let argc = args.len();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if argc > 4 || argc == 1 {
        let _ = out.write_all(b"Error: there should be one to three arguments passed:\n");
        let _ = out.write_all(b"<string> [start] [stop]\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    let arg1 = args[1].as_bytes();
    let len: usize = arg1.len(); // size_t equivalent

    let start: i32;
    let stop: i32;

    if argc >= 3 {
        let arg2 = args[2].as_bytes();
        let (val, ok) = c_strtol(arg2);
        start = val;
        if !ok {
            // C: printf without trailing newline.
            let _ = out.write_all(b"Second argument must be an integer!");
            let _ = out.flush();
            return ExitCode::from(1);
        }
        // C: `start > len` where start is int, len is size_t.
        // C promotes int to size_t; negative ints become huge unsigned values.
        // Rust `as usize` on a negative i32 sign-extends to a huge usize, matching C.
        if (start as usize) > len {
            let _ = out.write_all(b"Error: start is off the end of the string!\n");
            let _ = out.flush();
            return ExitCode::from(1);
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let arg3 = args[3].as_bytes();
        let (val, _ok) = c_strtol(arg3);
        stop = val;

        // BUG REPRODUCTION:
        // The C code is `if (end == argv[3])`, but `end` here still points
        // somewhere inside argv[2] (not updated, since strtol(argv[3], NULL, 10)
        // was called with NULL endptr). argv[3] is a different memory pointer
        // from anything inside argv[2], so this comparison is always false.
        // We therefore NEVER print "Third argument must be an integer!".

        if (stop as usize) > len {
            let _ = out.write_all(b"Error: stop is off the end of the string!\n");
            let _ = out.flush();
            return ExitCode::from(1);
        }

        if stop <= start {
            let _ = out.write_all(b"Error: stop must come after start!\n");
            let _ = out.flush();
            return ExitCode::from(1);
        }
    } else {
        // C truncates `size_t len` to `int stop`. Mimic that truncation.
        stop = len as i32;
    }

    // printf("%.*s\n", stop - start, argv[1] + start);
    // The precision is `stop - start` (int). If non-positive, no chars printed.
    // start has already been validated to be in [0, len], and either:
    //  - argc == 4: stop > start strictly (stop <= start triggers error).
    //  - argc != 4: stop = len, start in [0, len], so stop - start in [0, len].
    // The %.*s precision limits the number of bytes written from argv[1]+start.
    let diff: i32 = stop.wrapping_sub(start);
    if diff > 0 {
        let s_offset = start as usize;
        let width = diff as usize;
        // Slice cannot exceed argv[1] in our valid paths; clamp defensively.
        let end_idx = s_offset.saturating_add(width).min(arg1.len());
        let _ = out.write_all(&arg1[s_offset..end_idx]);
    }
    let _ = out.write_all(b"\n");
    let _ = out.flush();

    ExitCode::from(0)
}
