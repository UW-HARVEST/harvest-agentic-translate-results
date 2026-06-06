// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust, preserving original behavior.

use std::env;
use std::process::ExitCode;

/// Mimic C's `strtol(s, &end, 10)` for the parts we need:
/// - Skips leading ASCII whitespace.
/// - Accepts an optional '+' or '-' sign.
/// - Parses base-10 digits, accumulating into a `long` (i64).
/// - Returns the parsed value (saturated like C on overflow) and the number
///   of bytes consumed from the start of `s`. If no digits were parsed,
///   the consumed-count is 0 (matching `end == argv[1]` in C).
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut idx: usize = 0;

    // Skip leading whitespace (matches C isspace for typical ASCII).
    while idx < s.len() {
        let c = s[idx];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            idx += 1;
        } else {
            break;
        }
    }

    // Optional sign.
    let sign_pos = idx;
    let negative = if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        let neg = s[idx] == b'-';
        idx += 1;
        neg
    } else {
        false
    };

    // Must have at least one digit; otherwise nothing was parsed.
    let digits_start = idx;
    let mut value: i64 = 0;
    let mut overflow = false;
    while idx < s.len() {
        let c = s[idx];
        if !(b'0'..=b'9').contains(&c) {
            break;
        }
        let d = (c - b'0') as i64;
        if !overflow {
            // Detect overflow against signed range.
            if negative {
                // value * 10 - d should fit in i64 (signed range)
                let next = value.checked_mul(10).and_then(|v| v.checked_sub(d));
                match next {
                    Some(v) => value = v,
                    None => {
                        overflow = true;
                        value = i64::MIN;
                    }
                }
            } else {
                let next = value.checked_mul(10).and_then(|v| v.checked_add(d));
                match next {
                    Some(v) => value = v,
                    None => {
                        overflow = true;
                        value = i64::MAX;
                    }
                }
            }
        }
        idx += 1;
    }

    if idx == digits_start {
        // No digits consumed -> nothing was parsed; mimic C strtol where
        // `end` is set to the start of the string.
        // Reset idx to 0 so the caller sees `end == start`.
        return (0, 0);
    }

    // If we had a sign but no digits, we already returned above.
    let _ = sign_pos;

    (value, idx)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let arg_bytes = args[1].as_bytes();
    let (parsed_long, consumed) = strtol_base10(arg_bytes);
    if consumed == 0 {
        // end == argv[1] in C
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    // C truncates the `long` result of strtol into `int` (32-bit).
    let mut val: i32 = parsed_long as i32;

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        // Match C's typical signed int wrap-around (technically UB in C,
        // but in practice wraps on common compilers/targets).
        val = val.wrapping_add(1);
    }

    ExitCode::from(0)
}
