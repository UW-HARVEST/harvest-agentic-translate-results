// Translated from C: counts from a starting integer,
// stopping when the count ends in 9 (base 10).

use std::env;
use std::process::ExitCode;

/// Mimics C's strtol(s, &end, 10) behavior.
/// Returns (parsed_value_as_long, num_bytes_consumed).
/// If no digits were parsed, num_bytes_consumed == 0.
/// The parsed value is saturated to i64::MAX or i64::MIN on overflow
/// (matching C strtol's saturation/errno=ERANGE behavior with the value
/// before truncation to int).
fn c_strtol_base10(bytes: &[u8]) -> (i64, usize) {
    let mut idx: usize = 0;

    // Skip leading whitespace (matches isspace for ASCII whitespace).
    while idx < bytes.len() {
        let c = bytes[idx];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0b /* vertical tab */ || c == 0x0c /* form feed */ {
            idx += 1;
        } else {
            break;
        }
    }

    let sign_start = idx;

    // Optional sign.
    let mut negative = false;
    if idx < bytes.len() {
        match bytes[idx] {
            b'+' => { idx += 1; }
            b'-' => { negative = true; idx += 1; }
            _ => {}
        }
    }

    let digits_start = idx;

    // Parse digits.
    let mut acc: i64 = 0;
    let mut overflowed = false;
    while idx < bytes.len() {
        let c = bytes[idx];
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as i64;
        if !overflowed {
            // Check overflow with respect to the sign.
            if negative {
                // Build as if negative: acc*10 - d, must stay >= i64::MIN.
                match acc.checked_mul(10).and_then(|v| v.checked_sub(d)) {
                    Some(v) => acc = v,
                    None => {
                        overflowed = true;
                        acc = i64::MIN;
                    }
                }
            } else {
                match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => acc = v,
                    None => {
                        overflowed = true;
                        acc = i64::MAX;
                    }
                }
            }
        }
        idx += 1;
    }

    if digits_start == idx {
        // No digits: end pointer in C is set to the original start of string.
        // We return 0 consumed so the caller knows nothing was parsed.
        // Note: C also doesn't move past the sign in this case.
        let _ = sign_start; // silence unused if ever
        return (0, 0);
    }

    (acc, idx)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let arg = args[1].as_bytes();
    let (parsed, consumed) = c_strtol_base10(arg);
    if consumed == 0 {
        // end == argv[1]: nothing parsed.
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    // Truncate from long to int as C does (val is `int`).
    // This matches the implementation-defined conversion that on
    // typical platforms is a wrapping truncation.
    let mut val: i32 = parsed as i32;

    loop {
        println!("{}", val);
        // Match C's signed `%` semantics: in Rust, `i32::rem` is also
        // truncated toward zero, so behavior is identical to C99+.
        if val % 10 == 9 {
            break;
        }
        // C: val++ on signed int overflow is UB; on common platforms
        // this wraps. Use wrapping_add to mirror that wrapping behavior.
        val = val.wrapping_add(1);
    }

    ExitCode::from(0)
}
