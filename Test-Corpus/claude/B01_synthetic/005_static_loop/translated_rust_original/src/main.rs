// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to reproduce identical behavior.

use std::env;
use std::process::ExitCode;

fn static_sum(update: i32) -> i32 {
    // Equivalent to C's `static int sum = 0;` inside the function.
    // We use a thread-local variable here to model the per-process
    // mutable state. Since the program is single-threaded, behavior
    // matches the C original.
    use std::cell::Cell;
    thread_local! {
        static SUM: Cell<i32> = const { Cell::new(0) };
    }
    SUM.with(|s| {
        let new_val = s.get().wrapping_add(update);
        s.set(new_val);
        new_val
    })
}

/// Replicates C's `strtol(s, &end, 10)` for base 10:
///   * Skips leading whitespace (matching the C locale's isspace).
///   * Accepts an optional '+' or '-' sign.
///   * Parses as many decimal digits as possible.
///   * Returns the parsed value (saturating to LONG_MIN/LONG_MAX on overflow,
///     same as glibc's strtol), and the number of bytes consumed from `s`.
///     If no digits were consumed (after sign), `consumed` will be 0,
///     mimicking C's behavior of leaving `*end == s`.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;
    // Skip whitespace (C isspace: ' ', '\t', '\n', '\v', '\f', '\r').
    while i < s.len() {
        match s[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }

    let sign_start = i;
    let mut negative = false;
    if i < s.len() {
        match s[i] {
            b'+' => i += 1,
            b'-' => {
                negative = true;
                i += 1;
            }
            _ => {}
        }
    }

    let digits_start = i;
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflow {
            match value.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            }) {
                Some(v) => value = v,
                None => {
                    overflow = true;
                    value = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: per C strtol, set endptr to original string.
        // Caller checks `consumed == 0` to detect this.
        return (0, 0);
    }

    let _ = sign_start;
    (value, i)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        print!("Error: should only be a single (integer) argument!\n");
        return ExitCode::from(1);
    }

    let arg_bytes = args[1].as_bytes();
    let (parsed, consumed) = strtol_base10(arg_bytes);
    if consumed == 0 {
        // end is set to start of string if nothing parsed
        print!("Error: first argument must be an integer!\n");
        return ExitCode::from(1);
    }

    // C casts the long return of strtol to int via implicit conversion.
    // Per C's implementation-defined behavior for out-of-range values,
    // GCC/Clang on common platforms use a truncating wrap. We do the same.
    let stride: i32 = parsed as i32;

    for i in 0..10i32 {
        print!("{}\n", static_sum(i.wrapping_mul(stride)));
    }

    ExitCode::from(0)
}
