// Translation of c_src/src/main.c to Rust.
// Counts from a starting integer (parsed from argv[1] using strtol semantics),
// printing each value on its own line, stopping after printing a value whose
// last decimal digit is 9 (matching C's truncated-toward-zero modulo).

use std::env;
use std::process::ExitCode;

/// Mimic C's `strtol(str, &end, 10)` for our purposes.
///
/// Returns (value_as_i32, bytes_consumed). `bytes_consumed == 0` indicates
/// that no integer characters were parsed (the equivalent of `end == argv[1]`).
///
/// Behaviour mirrors the C library:
/// * leading ASCII whitespace is skipped,
/// * an optional '+' or '-' sign is accepted,
/// * decimal digits are then consumed,
/// * the integer value is accumulated in i64 then cast to i32 (truncating
///   toward zero, matching the C cast `int val = strtol(...)`).
fn strtol_base10(s: &[u8]) -> (i32, usize) {
    let mut i = 0usize;

    // Skip leading whitespace (C's isspace in the "C" locale).
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) {
        i += 1;
    }

    let start_after_ws = i;

    // Optional sign.
    let mut negative = false;
    if i < s.len() {
        match s[i] {
            b'+' => {
                i += 1;
            }
            b'-' => {
                negative = true;
                i += 1;
            }
            _ => {}
        }
    }

    let digits_start = i;
    let mut value: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        // Use wrapping arithmetic in i64; C strtol clamps but the result is
        // then cast to int which truncates. We just take the low 32 bits at
        // the end via `as i32`, which matches the eventual C truncation for
        // values that fit in i64. For values that overflow i64 the behaviour
        // is undefined in C anyway; we use wrapping here to avoid panicking.
        value = value.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }

    // If no digits were consumed, the C function leaves *end == nptr.
    // Replicate that by returning bytes_consumed == 0.
    if i == digits_start {
        return (0, 0);
    }

    if negative {
        value = value.wrapping_neg();
    }

    let _ = start_after_ws; // silence unused warning if any
    (value as i32, i)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let arg = args[1].as_bytes();
    let (mut val, consumed) = strtol_base10(arg);
    if consumed == 0 {
        // end is set to start of string if nothing parsed
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    loop {
        println!("{}", val);
        // C's `%` truncates toward zero, so for negative values `val % 10`
        // yields a non-positive remainder and is never equal to 9.
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }

    ExitCode::from(0)
}
