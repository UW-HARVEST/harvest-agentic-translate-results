// Rust translation of c_src/src/main.c
// Reproduces the original C behavior, including its bugs.
//
// The original program indexes into a passed string and prints the substring
// indexed by [start, stop). It also has a bug where the "third argument must
// be an integer" check uses a stale `end` pointer from parsing argv[2], so
// that check effectively never fires.

use std::env;
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::process::ExitCode;

/// Mimic C's `strtol(s, &end, 10)`.
///
/// Returns `(value, consumed)` where `consumed` is the number of bytes from
/// the start of `s` that were considered part of the number (mirroring the
/// position of the `end` pointer relative to `s`). If no conversion took
/// place, `consumed` is 0 (matching `end == s` in C).
fn parse_strtol(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading whitespace (matches C isspace() classes that strtol skips).
    while i < s.len()
        && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    {
        i += 1;
    }

    // Optional sign.
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    // Digits.
    let digit_start = i;
    let mut val: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflow {
            match val.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => val = v,
                None => {
                    overflow = true;
                    val = i64::MAX;
                }
            }
        }
        i += 1;
    }

    // No digits => no conversion performed; end points to original string.
    if i == digit_start {
        return (0, 0);
    }

    let result = if neg {
        if overflow {
            i64::MIN
        } else {
            -val
        }
    } else {
        val
    };

    (result, i)
}

fn main() -> ExitCode {
    // Use OsString -> bytes to handle argv as raw bytes the way C does.
    let args: Vec<Vec<u8>> = env::args_os().map(|s| s.into_vec()).collect();
    let argc = args.len();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if argc > 4 || argc == 1 {
        let _ = out.write_all(b"Error: there should be one to three arguments passed:\n");
        let _ = out.write_all(b"<string> [start] [stop]\n");
        return ExitCode::from(1);
    }

    let arg1 = &args[1];
    let len: usize = arg1.len(); // strlen(argv[1])

    // C: `int start, stop;`
    let start: i32;
    let stop: i32;

    if argc >= 3 {
        let (val, consumed) = parse_strtol(&args[2]);
        start = val as i32; // truncate long -> int as C does (impl-defined; matches GCC)
        if consumed == 0 {
            // Note: original C has no '\n' here.
            let _ = out.write_all(b"Second argument must be an integer!");
            return ExitCode::from(1);
        }
        // C: `if (start > len)` compares int to size_t; the int is converted
        // to size_t, so negative values become huge and trigger this check.
        if (start as i64) < 0 || (start as usize) > len {
            let _ = out.write_all(b"Error: start is off the end of the string!\n");
            return ExitCode::from(1);
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        // C calls strtol(argv[3], NULL, 10) so `end` is not updated here.
        let (val, _consumed) = parse_strtol(&args[3]);
        stop = val as i32;

        // Reproduce the original C bug exactly: the check `if (end == argv[3])`
        // compares a pointer into argv[2]'s storage with argv[3]'s pointer,
        // which is never true. So the check effectively never fires; we
        // intentionally do NOT perform any "third argument must be an integer"
        // validation, matching the bug in the C source.

        if (stop as i64) < 0 || (stop as usize) > len {
            let _ = out.write_all(b"Error: stop is off the end of the string!\n");
            return ExitCode::from(1);
        }

        if stop <= start {
            let _ = out.write_all(b"Error: stop must come after start!\n");
            return ExitCode::from(1);
        }
    } else {
        stop = len as i32;
    }

    // C: printf("%.*s\n", stop - start, argv[1] + start);
    let n = (stop - start) as usize;
    let s_start = start as usize;
    let _ = out.write_all(&arg1[s_start..s_start + n]);
    let _ = out.write_all(b"\n");

    ExitCode::from(0)
}
