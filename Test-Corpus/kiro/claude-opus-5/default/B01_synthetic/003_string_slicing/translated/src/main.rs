// Rust translation of c_src/src/main.c
//
// Index into a passed string and print the substring indexed by [start, stop).
// If there is no start, use 0.
// If there is no stop, use the end of the string.
//
// This translation reproduces the original C behavior exactly, including its
// bugs. See the notes at each site marked `C QUIRK`.

use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

/// Bytes that C's `isspace()` accepts in the default "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Result of a `strtol(nptr, &end, 10)` call.
struct StrtolResult {
    /// The converted value, saturated like `strtol` does on ERANGE
    /// (LONG_MAX / LONG_MIN). `long` is 64-bit on Linux x86_64.
    value: i64,
    /// Number of bytes consumed, i.e. the offset of `end` within `nptr`.
    /// Zero means no conversion was performed (`end == nptr`).
    end_offset: usize,
}

/// Faithful model of `strtol(nptr, &end, 10)` operating on a NUL-terminated
/// byte string represented as `s` (without the terminator).
fn strtol_base10(s: &[u8]) -> StrtolResult {
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

    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;

    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !overflow {
            // Accumulate magnitude in the negative direction so that
            // LONG_MIN is representable.
            match acc.checked_mul(10).and_then(|v| v.checked_sub(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits: no conversion performed, `end` is set back to `nptr`.
        return StrtolResult {
            value: 0,
            end_offset: 0,
        };
    }

    let value = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        acc
    } else {
        // `acc` holds the negated magnitude; negate it back.
        match acc.checked_neg() {
            Some(v) => v,
            None => i64::MAX,
        }
    };

    StrtolResult {
        value,
        end_offset: i,
    }
}

fn main() -> ExitCode {
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_bytes().to_vec())
        .collect();
    let argc = argv.len();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if (argc > 4) || (argc == 1) {
        let _ = out.write_all(b"Error: there should be one to three arguments passed:\n");
        let _ = out.write_all(b"<string> [start] [stop]\n");
        let _ = out.flush();
        return ExitCode::from(1);
    }

    // strlen(argv[1]) -- size_t
    let len: u64 = argv[1].len() as u64;

    let start: i32;
    let stop: i32;

    // `char *end;` -- uninitialized in C. It is only ever read when argc >= 3,
    // in which case the strtol below has assigned it. `end_offset` models the
    // offset of `end` within argv[2].
    let mut end_offset: usize = 0;

    if argc >= 3 {
        let parsed = strtol_base10(&argv[2]);
        // `start` is an int, so the long result is truncated.
        start = parsed.value as i32;
        end_offset = parsed.end_offset;

        // `end == argv[2]` -- no conversion could be performed.
        if end_offset == 0 {
            // No trailing newline, exactly as in the C.
            let _ = out.write_all(b"Second argument must be an integer!");
            let _ = out.flush();
            return ExitCode::from(1);
        }

        // C QUIRK: `start > len` compares an int against a size_t, so `start`
        // is converted to unsigned. Any negative start becomes a huge value
        // and is therefore always rejected here.
        if (start as i64 as u64) > len {
            let _ = out.write_all(b"Error: start is off the end of the string!\n");
            let _ = out.flush();
            return ExitCode::from(1);
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        // Note: the C passes NULL as endptr here, so `end` is NOT updated.
        stop = strtol_base10(&argv[3]).value as i32;

        // C QUIRK: this checks `end == argv[3]`, but `end` still points into
        // argv[2] (it was last set by the strtol above). Two distinct argv
        // strings can never share an address, so this branch is dead code and
        // a non-numeric third argument silently yields 0.
        let _ = end_offset;
        let end_equals_argv3 = false;
        if end_equals_argv3 {
            let _ = out.write_all(b"Third argument must be an integer!");
            let _ = out.flush();
            return ExitCode::from(1);
        }

        // C QUIRK: same signed/unsigned comparison as above; a negative stop
        // is always rejected here.
        if (stop as i64 as u64) > len {
            let _ = out.write_all(b"Error: stop is off the end of the string!\n");
            let _ = out.flush();
            return ExitCode::from(1);
        }

        // Both operands are int here, so this is a signed comparison.
        if stop <= start {
            let _ = out.write_all(b"Error: stop must come after start!\n");
            let _ = out.flush();
            return ExitCode::from(1);
        }
    } else {
        // `stop = len` truncates size_t to int.
        stop = len as i32;
    }

    // printf("%.*s\n", stop - start, argv[1] + start)
    let precision = stop.wrapping_sub(start);
    let offset = start as usize;
    let available = &argv[1][offset..];
    let printed: &[u8] = if precision < 0 {
        // A negative precision is treated by printf as if it were omitted.
        available
    } else {
        let n = (precision as usize).min(available.len());
        &available[..n]
    };

    let _ = out.write_all(printed);
    let _ = out.write_all(b"\n");
    let _ = out.flush();

    ExitCode::from(0)
}
