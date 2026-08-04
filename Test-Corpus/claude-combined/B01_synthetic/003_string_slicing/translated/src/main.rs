// Rust translation of c_src/src/main.c. Reproduces the C program's behavior
// (including its quirks) byte-for-byte on stdout/stderr.

use std::io::Write;
use std::process::ExitCode;

/// A small subset of C's strtol with base 10. Returns (value, bytes_consumed).
/// `bytes_consumed == 0` corresponds to C's `endptr == nptr` (no conversion).
/// On overflow, the value is clamped to i64::MAX or i64::MIN, mirroring the
/// LONG_MAX/LONG_MIN clamping behavior of strtol on 64-bit Linux. The caller
/// is expected to truncate to i32 to match the C `int` assignment.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0;
    // C's isspace: ' ', '\t', '\n', '\v', '\f', '\r'
    while i < s.len()
        && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut val: i64 = 0;
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflowed {
            let stepped = val.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            });
            match stepped {
                Some(v) => val = v,
                None => {
                    overflowed = true;
                    val = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: C sets *endptr = nptr (the original input).
        return (0, 0);
    }

    (val, i)
}

fn run() -> i32 {
    // Collect args as raw bytes; on Unix OsString::into_vec gives the exact
    // byte content of the argument as the kernel delivered it. We avoid
    // requiring valid UTF-8 because the C version operates on raw bytes.
    let args_os: Vec<std::ffi::OsString> = std::env::args_os().collect();

    #[cfg(unix)]
    let args: Vec<Vec<u8>> = {
        use std::os::unix::ffi::OsStringExt;
        args_os.into_iter().map(|s| s.into_vec()).collect()
    };
    #[cfg(not(unix))]
    let args: Vec<Vec<u8>> = args_os
        .into_iter()
        .map(|s| s.to_string_lossy().into_owned().into_bytes())
        .collect();

    let argc = args.len();
    let mut stdout = std::io::stdout().lock();

    if argc > 4 || argc == 1 {
        let _ = stdout.write_all(b"Error: there should be one to three arguments passed:\n");
        let _ = stdout.write_all(b"<string> [start] [stop]\n");
        return 1;
    }

    let argv1: &[u8] = &args[1];
    // strlen(argv[1]) -- this is the byte length up to NUL. Since argv entries
    // are NUL-terminated and the buffer above contains the bytes up to (but
    // excluding) the NUL terminator, the buffer length equals strlen.
    let len: usize = argv1.len();

    let mut start: i32 = 0;
    let stop: i32;

    if argc >= 3 {
        let argv2: &[u8] = &args[2];
        let (val, consumed) = strtol_base10(argv2);
        if consumed == 0 {
            // No newline in the C source ("...integer!" without \n).
            let _ = stdout.write_all(b"Second argument must be an integer!");
            return 1;
        }
        // C truncates the long return value of strtol to int via assignment.
        start = val as i32;
        // C: `if (start > len)` -- here `start` (int, signed) is promoted to
        // size_t (unsigned) for the comparison, so negative starts compare as
        // larger than `len`. `i32 as usize` reproduces this on 64-bit and on
        // 32-bit platforms alike.
        if (start as usize) > len {
            let _ = stdout.write_all(b"Error: start is off the end of the string!\n");
            return 1;
        }
    }

    if argc == 4 {
        let argv3: &[u8] = &args[3];
        // C passes NULL to strtol here, so the "end" pointer isn't updated
        // by this call. The subsequent C check `if (end == argv[3])` uses the
        // stale `end` value from the argv[2] strtol call -- which is a pointer
        // into argv[2] and can therefore never equal argv[3]. The check is
        // effectively dead; we replicate that by simply not performing it.
        let (val, _consumed) = strtol_base10(argv3);
        stop = val as i32;

        if (stop as usize) > len {
            let _ = stdout.write_all(b"Error: stop is off the end of the string!\n");
            return 1;
        }

        if stop <= start {
            let _ = stdout.write_all(b"Error: stop must come after start!\n");
            return 1;
        }
    } else {
        // C: `stop = len;` -- size_t truncated to int. `as i32` matches the
        // implementation-defined low-bits truncation on common platforms.
        stop = len as i32;
    }

    // printf("%.*s\n", stop - start, argv[1] + start);
    // Both invariants below hold from the validation above:
    //   - 0 <= start <= len
    //   - stop - start >= 0
    //   - start + (stop - start) == stop <= len  (in the argc==4 branch)
    //     or stop == len in the else branch
    let prec = (stop - start) as usize;
    let slice_start = start as usize;
    // Defensive bound: %.*s additionally stops at NUL (none in argv) and at
    // string end. argv1 has no embedded NUL, so just min with available length.
    let avail = len.saturating_sub(slice_start);
    let take = prec.min(avail);
    let _ = stdout.write_all(&argv1[slice_start..slice_start + take]);
    let _ = stdout.write_all(b"\n");

    0
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
