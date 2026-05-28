// Translated from c_src/src/main.c
// Maintains byte-identical output to the C executable.

use std::env;
use std::process::ExitCode;

/// Mimics C `strtol(s, &end, 10)`.
/// Returns the parsed `i64` value and the number of bytes consumed.
/// If no digits were parsed, returns `(0, 0)`, matching C's behavior of
/// setting `end` back to the start of the string.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i: usize = 0;

    // Skip leading whitespace (matching C `isspace` for the C locale).
    while i < s.len() {
        let c = s[i];
        if c == b' '
            || c == b'\t'
            || c == b'\n'
            || c == b'\r'
            || c == 0x0b
            || c == 0x0c
        {
            i += 1;
        } else {
            break;
        }
    }

    // Optional sign.
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        if s[i] == b'-' {
            neg = true;
        }
        i += 1;
    }

    // Digits.
    let digit_start = i;
    let mut val: i64 = 0;
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflowed {
            if neg {
                match val.checked_mul(10).and_then(|v| v.checked_sub(d)) {
                    Some(v) => val = v,
                    None => {
                        val = i64::MIN;
                        overflowed = true;
                    }
                }
            } else {
                match val.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => val = v,
                    None => {
                        val = i64::MAX;
                        overflowed = true;
                    }
                }
            }
        }
        i += 1;
    }

    if i == digit_start {
        // No digits parsed: in C, `end` is reset to the original `nptr`.
        (0, 0)
    } else {
        (val, i)
    }
}

fn run() -> i32 {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 3 {
        println!("Error: should only be two (integer) arguments!");
        return 1;
    }

    let arg1 = args[1].as_bytes();
    let (parsed1, consumed1) = strtol_base10(arg1);
    if consumed1 == 0 {
        // end == argv[1]
        println!("Error: first argument must be an integer!");
        return 1;
    }
    // C assigns `long` -> `int`, which on typical platforms truncates.
    let initial_value: i32 = parsed1 as i32;

    let arg2 = args[2].as_bytes();
    let (parsed2, consumed2) = strtol_base10(arg2);
    if consumed2 == 0 {
        // end == argv[2]
        println!("Error: second argument must be an integer!");
        return 1;
    }
    let iterations: i32 = parsed2 as i32;

    // The C code maintains a static `inner = 1` inside `static_alias`, and
    // `running_sum` is a pointer that is either to `initial_value` (outer) or
    // to `inner`. We model the alias state using a boolean flag instead of a
    // raw pointer, replicating the exact arithmetic on each iteration.
    let mut outer: i32 = initial_value;
    let mut inner: i32 = 1;
    let mut points_to_inner: bool = false;

    let mut i: i32 = 0;
    while i < iterations {
        if points_to_inner {
            // *outer == inner, so the condition `*outer >= inner` is true.
            // `inner += *outer` becomes `inner += inner` (i.e. doubles).
            // Then `running_sum = &inner` (still points to inner).
            inner = inner.wrapping_add(inner);
        } else if outer >= inner {
            inner = inner.wrapping_add(outer);
            points_to_inner = true;
        } else {
            outer = outer.wrapping_add(inner);
            // running_sum still points to outer; flag remains false.
        }

        let printed = if points_to_inner { inner } else { outer };
        println!("{}", printed);

        i = i.wrapping_add(1);
    }

    0
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
