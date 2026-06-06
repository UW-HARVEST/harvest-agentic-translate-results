// Rust translation of c_src/src/main.c
// Reproduces the same byte-identical output for the same inputs.

use std::env;
use std::process;

/// Parses a base-10 integer in a way compatible with C's strtol():
/// - Skips leading whitespace
/// - Accepts an optional sign (`+` or `-`)
/// - Reads as many decimal digits as possible
/// - Returns `None` if no digits were consumed (matching the C check
///   `if (end == argv[1])`).
/// - On overflow, saturates to i64::MAX / i64::MIN (matching strtol's
///   ERANGE behavior).
fn parse_strtol(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    let mut idx = 0usize;

    // Skip whitespace (matching C's isspace for the "C" locale).
    while idx < bytes.len() {
        match bytes[idx] {
            b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r' => idx += 1,
            _ => break,
        }
    }

    // Optional sign.
    let mut neg = false;
    if idx < bytes.len() {
        if bytes[idx] == b'+' {
            idx += 1;
        } else if bytes[idx] == b'-' {
            neg = true;
            idx += 1;
        }
    }

    let digits_start = idx;
    let mut value: i64 = 0;
    let mut overflow = false;

    while idx < bytes.len() {
        let c = bytes[idx];
        if c.is_ascii_digit() {
            let d = (c - b'0') as i64;
            if !overflow {
                let next = value.checked_mul(10).and_then(|v| {
                    if neg {
                        v.checked_sub(d)
                    } else {
                        v.checked_add(d)
                    }
                });
                match next {
                    Some(v) => value = v,
                    None => {
                        overflow = true;
                        value = if neg { i64::MIN } else { i64::MAX };
                    }
                }
            }
            idx += 1;
        } else {
            break;
        }
    }

    if idx == digits_start {
        // Nothing parsed: matches C's "end == argv[1]" condition.
        None
    } else {
        Some(value)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 3 {
        println!("Error: should only be two (integer) arguments!");
        process::exit(1);
    }

    // strtol-equivalent parse for the first argument.
    let initial_long = match parse_strtol(&args[1]) {
        Some(v) => v,
        None => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };
    // C truncates the long return of strtol() to int.
    let mut initial_value: i32 = initial_long as i32;

    // strtol-equivalent parse for the second argument.
    let iterations_long = match parse_strtol(&args[2]) {
        Some(v) => v,
        None => {
            println!("Error: second argument must be an integer!");
            process::exit(1);
        }
    };
    let iterations: i32 = iterations_long as i32;

    // The original C code uses a `static int inner = 1;` inside `static_alias`.
    // Since `static_alias` is the only function that touches it, we can model
    // the same lifetime and behavior with a local mutable variable here.
    let mut inner: i32 = 1;

    // The original C code's `running_sum` may alias either the local
    // `initial_value` or the static `inner`. We model that with a flag.
    let mut pointing_to_inner = false;

    for _ in 0..iterations {
        if !pointing_to_inner {
            // running_sum currently points to `initial_value`.
            // Equivalent to static_alias(&mut initial_value).
            if initial_value >= inner {
                // inner += *outer; return &inner;
                inner = inner.wrapping_add(initial_value);
                pointing_to_inner = true;
            } else {
                // *outer += inner; return outer;
                initial_value = initial_value.wrapping_add(inner);
                // pointing_to_inner stays false
            }
        } else {
            // running_sum currently points to `inner` (outer == &inner).
            // *outer >= inner reduces to inner >= inner -> always true.
            // inner += *outer reduces to inner += inner.
            // The return value &inner keeps pointing_to_inner = true.
            inner = inner.wrapping_add(inner);
        }

        let value_to_print: i32 = if pointing_to_inner {
            inner
        } else {
            initial_value
        };
        println!("{}", value_to_print);
    }
}
