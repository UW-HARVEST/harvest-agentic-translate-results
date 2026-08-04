use std::env;
use std::process::ExitCode;

/// Mimics C's strtol(s, &end, 10) behavior.
/// Returns (value, num_chars_consumed). If num_chars_consumed == 0 then
/// nothing was parsed (matches `end == argv[1]` in the original C).
///
/// Saturates on overflow like C strtol (LONG_MAX / LONG_MIN), where `long`
/// is 64-bit on Linux x86_64.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut idx = 0usize;

    // Skip leading whitespace (matches C isspace for the standard locale).
    while idx < s.len() {
        match s[idx] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => idx += 1,
            _ => break,
        }
    }

    let start_after_ws = idx;

    // Optional sign.
    let mut negative = false;
    if idx < s.len() {
        match s[idx] {
            b'+' => idx += 1,
            b'-' => {
                negative = true;
                idx += 1;
            }
            _ => {}
        }
    }

    let digits_start = idx;
    let mut value: i64 = 0;
    let mut overflow = false;

    while idx < s.len() && s[idx].is_ascii_digit() {
        let d = (s[idx] - b'0') as i64;
        if !overflow {
            // Detect overflow against i64 bounds.
            if negative {
                // Building up positive then negating; check against i64::MIN.
                // -value*10 - d should be >= i64::MIN.
                match value.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) if -v >= i64::MIN.wrapping_add(0) => value = v,
                    _ => {
                        overflow = true;
                    }
                }
            } else {
                match value.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => value = v,
                    None => {
                        overflow = true;
                    }
                }
            }
        }
        idx += 1;
    }

    // If no digits were consumed, end pointer is reset to the start of the
    // original string (i.e., 0 chars consumed total).
    if digits_start == idx {
        return (0, 0);
    }

    let result = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // value here is the positive magnitude; negate.
        value.wrapping_neg()
    } else {
        value
    };

    let _ = start_after_ws;
    (result, idx)
}

fn static_sum(update: i32, sum: &mut i32) -> i32 {
    *sum = sum.wrapping_add(update);
    *sum
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let arg1 = args[1].as_bytes();
    let (value_i64, consumed) = strtol_base10(arg1);
    if consumed == 0 {
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    // Match C's `int stride = strtol(...)` truncation: cast long (i64) to int
    // (i32) takes the low 32 bits.
    let stride: i32 = value_i64 as i32;

    let mut sum: i32 = 0;
    for i in 0..10i32 {
        let update = i.wrapping_mul(stride);
        let s = static_sum(update, &mut sum);
        println!("{}", s);
    }

    ExitCode::from(0)
}
