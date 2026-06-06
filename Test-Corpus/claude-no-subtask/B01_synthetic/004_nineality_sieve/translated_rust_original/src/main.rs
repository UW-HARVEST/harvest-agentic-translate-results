use std::env;
use std::process::ExitCode;

/// Mimics C's strtol(s, &end, 10).
/// Returns (parsed_value_as_i64, number_of_bytes_consumed_from_start).
/// If nothing was parsed, returns (0, 0).
/// Saturates to i64::MAX or i64::MIN on overflow (like glibc strtol).
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut idx = 0usize;

    // Skip leading whitespace (matches C's isspace for "C" locale).
    while idx < s.len() {
        let c = s[idx];
        if c == b' '
            || c == b'\t'
            || c == b'\n'
            || c == b'\r'
            || c == 0x0b /* \v */
            || c == 0x0c /* \f */
        {
            idx += 1;
        } else {
            break;
        }
    }

    // Optional sign.
    let mut negative = false;
    let sign_start = idx;
    if idx < s.len() {
        if s[idx] == b'+' {
            idx += 1;
        } else if s[idx] == b'-' {
            negative = true;
            idx += 1;
        }
    }

    // Parse digits.
    let digits_start = idx;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while idx < s.len() {
        let c = s[idx];
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as i64;
        if !overflow {
            // Compute next value while watching for overflow.
            // Use checked arithmetic on the magnitude to detect overflow.
            if negative {
                match acc.checked_mul(10).and_then(|v| v.checked_sub(d)) {
                    Some(v) => acc = v,
                    None => {
                        overflow = true;
                        acc = i64::MIN;
                    }
                }
            } else {
                match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => acc = v,
                    None => {
                        overflow = true;
                        acc = i64::MAX;
                    }
                }
            }
        }
        idx += 1;
    }

    if digits_start == idx {
        // No digits parsed: end pointer is set to original start (idx 0
        // semantics handled by caller with == argv[1]).
        // Per C strtol, if no conversion is performed, endptr is set to
        // the original nptr (before whitespace/sign).
        let _ = sign_start;
        return (0, 0);
    }

    (acc, idx)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let arg = &args[1];
    let (parsed, consumed) = strtol_base10(arg.as_bytes());
    if consumed == 0 {
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    // Mimic C's `int val = strtol(...)`: cast/truncate to i32.
    let mut val: i32 = parsed as i32;

    loop {
        println!("{}", val);
        // C's `val % 10 == 9` with negative numbers: in C99+, % follows
        // truncation toward zero, so for negative val ending in 9 the
        // result is -9, not 9. Match that behavior.
        if val % 10 == 9 {
            break;
        }
        // Match C's signed integer increment, which is UB on overflow but
        // typically wraps on two's-complement targets. Use wrapping_add
        // to avoid Rust panics in debug mode.
        val = val.wrapping_add(1);
    }

    ExitCode::from(0)
}
