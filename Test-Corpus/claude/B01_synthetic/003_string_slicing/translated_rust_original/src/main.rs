// Translation of c_src/src/main.c to Rust.
// Goal: byte-identical output for the same inputs, including reproducing
// any bugs in the original C code.

use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

/// Mimic C's `strtol(s, &end, 10)` semantics enough for this program.
///
/// Returns `(value, consumed_bytes)`.  If `consumed_bytes == 0`, no
/// conversion was performed (matching the C check `end == arg`).
///
/// Behaviour matched:
///   - Skip leading ASCII whitespace.
///   - Optional leading '+' or '-'.
///   - Decimal digits only (base 10).
///   - On overflow, saturates to LONG_MAX/LONG_MIN (good enough for our use).
fn strtol_base10(bytes: &[u8]) -> (i64, usize) {
    let mut idx = 0usize;

    // Skip leading whitespace (matches isspace for ASCII).
    while idx < bytes.len() {
        let c = bytes[idx];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0b /* vt */ || c == 0x0c /* ff */
        {
            idx += 1;
        } else {
            break;
        }
    }

    // Optional sign.
    let mut negative = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        negative = bytes[idx] == b'-';
        idx += 1;
    }

    // Must have at least one digit for a successful conversion.
    let digits_start = idx;
    let mut value: i64 = 0;
    let mut overflow = false;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let d = (bytes[idx] - b'0') as i64;
        if !overflow {
            match value
                .checked_mul(10)
                .and_then(|v| if negative { v.checked_sub(d) } else { v.checked_add(d) })
            {
                Some(v) => value = v,
                None => {
                    overflow = true;
                    value = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        idx += 1;
    }

    if idx == digits_start {
        // No digits consumed: report "no conversion" by returning consumed=0,
        // matching C's `end == nptr` semantics.
        return (0, 0);
    }

    (value, idx)
}

fn main() -> ExitCode {
    // Collect raw argv as byte slices (matches C's char** view).
    let args_os: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = args_os.len();
    let argv: Vec<&[u8]> = args_os.iter().map(|s| s.as_bytes()).collect();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if argc > 4 || argc == 1 {
        let _ = out.write_all(b"Error: there should be one to three arguments passed:\n");
        let _ = out.write_all(b"<string> [start] [stop]\n");
        return ExitCode::from(1);
    }

    let len = argv[1].len(); // size_t in C (strlen of argv[1] as bytes)

    let start: i32;
    let stop: i32;

    // Track whether the argv[2] strtol "succeeded" (i.e., end != argv[2]).
    // The C code reuses `end` for the argv[3] check (a bug we preserve below):
    // since the argv[3] strtol passes NULL for end, `end` still points into
    // argv[2], and the check `end == argv[3]` is therefore always false.
    // We model this by computing the same boolean: argv2_end_ptr_equals_argv3,
    // which is always false because they are different argv slots.
    let mut argv2_consumed_eq_zero = false;

    if argc >= 3 {
        let (v, consumed) = strtol_base10(argv[2]);
        argv2_consumed_eq_zero = consumed == 0;
        if argv2_consumed_eq_zero {
            let _ = out.write_all(b"Second argument must be an integer!");
            return ExitCode::from(1);
        }
        // C truncates `long` to `int` when assigning to `start`.
        start = v as i32;

        // C compares `int start > size_t len` -> int is converted to size_t.
        // A negative `start` becomes a huge unsigned value and triggers this.
        let start_as_usize_like_c = start as i64 as usize_like;
        if start_as_usize_like_c > len {
            let _ = out.write_all(b"Error: start is off the end of the string!\n");
            return ExitCode::from(1);
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        // C: strtol(argv[3], NULL, 10) — note: passes NULL, so `end` is NOT
        // updated.  Then the C code checks `if (end == argv[3])`, which uses
        // the stale `end` from the argv[2] call — a bug we faithfully
        // reproduce here: the comparison is between two different argv
        // pointers, so it is always false.
        let (v, _consumed) = strtol_base10(argv[3]);
        let stale_end_equals_argv3 = false; // always false in C (different argv slots)
        let _ = argv2_consumed_eq_zero; // silence unused warning if any
        if stale_end_equals_argv3 {
            let _ = out.write_all(b"Third argument must be an integer!");
            return ExitCode::from(1);
        }

        stop = v as i32;

        // Same signed-int vs size_t promotion as above.
        let stop_as_usize_like_c = stop as i64 as usize_like;
        if stop_as_usize_like_c > len {
            let _ = out.write_all(b"Error: stop is off the end of the string!\n");
            return ExitCode::from(1);
        }

        if stop <= start {
            let _ = out.write_all(b"Error: stop must come after start!\n");
            return ExitCode::from(1);
        }
    } else {
        stop = len as i32; // C: stop = len; len is size_t, narrowed to int.
    }

    // C: printf("%.*s\n", stop - start, argv[1] + start);
    // Width is `int`; it can be negative, in which case printf treats it as
    // "no precision" (i.e., prints up to the null terminator).  In our valid
    // path, `start <= len` and `stop > start` (or `stop == len` from the else
    // branch), so the slice is well-defined.
    let width = (stop as i64) - (start as i64);
    if width < 0 {
        // Match C printf("%.*s", negative, ...) which treats negative
        // precision as "precision omitted" — for %s that means print up to
        // the NUL.  argv[1] in C is NUL-terminated, so this prints the
        // suffix starting at `start`.
        let s_off = start as usize;
        if s_off <= argv[1].len() {
            let _ = out.write_all(&argv[1][s_off..]);
        }
    } else {
        let s_off = start as usize;
        let s_end = s_off.saturating_add(width as usize).min(argv[1].len());
        if s_off <= argv[1].len() {
            let _ = out.write_all(&argv[1][s_off..s_end]);
        }
    }
    let _ = out.write_all(b"\n");

    ExitCode::from(0)
}

// Helper: emulate C's size_t for the sole purpose of unsigned promotion in
// comparisons.  On all modern Unix targets size_t == usize.
#[allow(non_camel_case_types)]
type usize_like = usize;
