// Rust translation of c_src/src/main.c — produces byte-identical output.
//
// The original C program prints a substring of argv[1] determined by
// optional [start] and [stop] integer arguments. We faithfully reproduce
// the behavior, including the exact error messages and ordering of checks.

use std::env;
use std::io::Write;
use std::process::ExitCode;

/// Mimic C's `strtol(s, &end, 10)`. Returns Some((value, consumed)) when at
/// least one digit was parsed; otherwise returns None to signal the
/// "no conversion" case (where, in C, `end == nptr`).
///
/// Uses i64 to roughly match the range of `long` on a 64-bit platform.
fn c_strtol(s: &[u8]) -> Option<(i64, usize)> {
    let mut i = 0usize;

    // Skip leading whitespace (matches C's isspace for the C locale).
    while i < s.len() && (s[i] as char).is_ascii_whitespace() {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        if s[i] == b'-' {
            negative = true;
        }
        i += 1;
    }

    // Digits.
    let digits_start = i;
    let mut value: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }

    if i == digits_start {
        // No digits parsed: in C, `end` remains equal to `nptr`.
        return None;
    }

    if negative {
        value = value.wrapping_neg();
    }
    Some((value, i))
}

fn run() -> i32 {
    // Use args_os so we don't panic on non-UTF-8 inputs; convert to bytes.
    let args: Vec<std::ffi::OsString> = env::args_os().collect();
    let argc = args.len();

    if argc > 4 || argc == 1 {
        print!("Error: there should be one to three arguments passed:\n");
        print!("<string> [start] [stop]\n");
        return 1;
    }

    let argv1_bytes = args[1].as_encoded_bytes();
    let len: usize = argv1_bytes.len(); // strlen(argv[1])

    let start: i32;
    let stop: i32;

    if argc >= 3 {
        let argv2_bytes = args[2].as_encoded_bytes();
        match c_strtol(argv2_bytes) {
            None => {
                // C prints without a trailing newline.
                print!("Second argument must be an integer!");
                return 1;
            }
            Some((v, _consumed)) => {
                // C: assignment of long to int truncates.
                start = v as i32;
            }
        }
        // C: `start > len` — int promoted to size_t (unsigned), so negative
        // values become huge and trip this branch.
        if (start as usize) > len {
            print!("Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let argv3_bytes = args[3].as_encoded_bytes();

        // C calls strtol with NULL endptr here, so `end` is NOT updated.
        // The subsequent check `if (end == argv[3])` therefore compares a
        // pointer that was set while parsing argv[2] against argv[3] — two
        // distinct argv buffers — and is effectively always false at runtime.
        // We faithfully reproduce this by never taking that branch.
        let parsed = c_strtol(argv3_bytes);

        // Bug-for-bug: do NOT emit "Third argument must be an integer!".
        // (See comment above; the C check on `end == argv[3]` cannot fire.)

        // C strtol returns 0 when no conversion is performed.
        stop = match parsed {
            Some((v, _)) => v as i32,
            None => 0i32,
        };

        if (stop as usize) > len {
            print!("Error: stop is off the end of the string!\n");
            return 1;
        }

        if stop <= start {
            print!("Error: stop must come after start!\n");
            return 1;
        }
    } else {
        // C: int = size_t (truncates on overflow; matches our `as i32`).
        stop = len as i32;
    }

    // printf("%.*s\n", stop - start, argv[1] + start);
    // At this point `start` and `stop` are guaranteed in [0, len] with
    // start <= stop based on the checks above.
    let begin = start as usize;
    let end = stop as usize;
    let slice = &argv1_bytes[begin..end];

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(slice).unwrap();
    handle.write_all(b"\n").unwrap();
    handle.flush().unwrap();

    0
}

fn main() -> ExitCode {
    let code = run();
    // Make sure all buffered output is flushed before we hand back the code.
    let _ = std::io::stdout().flush();
    ExitCode::from(code as u8)
}
