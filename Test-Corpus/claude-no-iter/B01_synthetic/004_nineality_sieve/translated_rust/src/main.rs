// Translated from c_src/src/main.c
// Counts from a starting point, stopping when the count ends in 9 (base 10).

use std::io::{self, Write};
use std::process::ExitCode;

/// Mimic C's strtol(s, &end, 10) behavior for the parts the original program
/// relies on. Returns (parsed_long, num_bytes_consumed). If no digits were
/// consumed, num_bytes_consumed will be 0, matching `end == argv[1]`.
fn c_strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut idx = 0;
    // Skip leading whitespace (C isspace: space, \t, \n, \v, \f, \r).
    while idx < s.len() {
        match s[idx] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => idx += 1,
            _ => break,
        }
    }

    // Optional sign.
    let start_after_sign;
    let negative;
    if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        negative = s[idx] == b'-';
        idx += 1;
        start_after_sign = idx;
    } else {
        negative = false;
        start_after_sign = idx;
    }

    // Read digits.
    let digits_start = idx;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while idx < s.len() && s[idx].is_ascii_digit() {
        let d = (s[idx] - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            }) {
                Some(v) => acc = v,
                None => {
                    overflow = true;
                    acc = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        idx += 1;
    }

    if digits_start == idx {
        // No digits were read; per C strtol, end is set to the original
        // string (before any whitespace/sign were consumed... actually
        // C strtol sets endptr to nptr if no conversion took place).
        // The original program checks `end == argv[1]`, so we report 0
        // bytes consumed in that case.
        let _ = start_after_sign;
        return (0, 0);
    }

    (acc, idx)
}

fn run() -> i32 {
    // Collect raw OS args as bytes to preserve C-like byte-level handling.
    // On all common Unix targets argv[i] is a NUL-terminated byte string.
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();

    // argc in C includes argv[0]. The program expects argc == 2, i.e., one
    // user-provided argument.
    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        return 1;
    }

    // Get the argument as bytes (Unix). On non-Unix platforms fall back to
    // a UTF-8 lossy conversion, which is sufficient for typical integer
    // arguments.
    #[cfg(unix)]
    let arg_bytes: Vec<u8> = {
        use std::os::unix::ffi::OsStrExt;
        args[1].as_os_str().as_bytes().to_vec()
    };
    #[cfg(not(unix))]
    let arg_bytes: Vec<u8> = args[1].to_string_lossy().as_bytes().to_vec();

    let (parsed, consumed) = c_strtol_base10(&arg_bytes);
    if consumed == 0 {
        println!("Error: first argument must be an integer!");
        return 1;
    }

    // The original C code stores strtol's result into an `int`, which on
    // typical platforms (including Linux x86_64 where this code is built)
    // is 32 bits. Truncation matches the C behavior of assigning a long
    // to an int.
    let mut val: i32 = parsed as i32;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    loop {
        // Match printf("%d\n", val): decimal int followed by newline.
        // writeln! produces the same bytes for any 32-bit signed integer.
        writeln!(out, "{}", val).expect("failed to write to stdout");
        if val % 10 == 9 {
            break;
        }
        // Original C uses `val++`. Signed overflow is undefined behavior in
        // C, but the typical observed behavior on two's-complement hardware
        // is wrap-around. Use wrapping_add to avoid a Rust panic in
        // debug builds while preserving that observable behavior.
        val = val.wrapping_add(1);
    }

    0
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
