// Copyright 2025 MIT Lincoln Laboratory
// Translated from the original C source. See c_src/ for the original
// MIT-licensed source code and full license text.

use std::cell::Cell;
use std::process::ExitCode;

thread_local! {
    static STATIC_SUM_TOTAL: Cell<i32> = const { Cell::new(0) };
}

fn static_sum(update: i32) -> i32 {
    STATIC_SUM_TOTAL.with(|cell| {
        let new_total = cell.get().wrapping_add(update);
        cell.set(new_total);
        new_total
    })
}

/// Mimic C's `strtol(s, &end, 10)` for the purposes of this program.
///
/// Returns a tuple `(value, parsed_len)` where `parsed_len` is the number of
/// bytes consumed from `s`. If `parsed_len == 0`, that means `end == s` in C
/// terms (nothing was parsed).
///
/// The returned value is the parsed `long` truncated to `i32`, which mirrors
/// the C code assigning the `strtol` result to an `int`.
fn c_strtol_base10(s: &[u8]) -> (i32, usize) {
    let mut idx = 0usize;

    // Skip leading whitespace, matching C's isspace for the "C" locale on
    // typical inputs: space, tab, newline, vertical tab, form feed, carriage
    // return.
    while idx < s.len() {
        match s[idx] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => idx += 1,
            _ => break,
        }
    }

    let sign_start = idx;
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
    // Accumulate using i64 so we can saturate when the value would exceed
    // an i32, mirroring the assignment of strtol's `long` result to `int`.
    // strtol itself saturates to LONG_MIN/LONG_MAX on overflow and sets errno;
    // for the inputs this program targets, an i64 accumulator is sufficient
    // and we then truncate to i32 like the original C code does.
    let mut acc: i64 = 0;
    let mut overflowed = false;
    while idx < s.len() {
        let c = s[idx];
        if !c.is_ascii_digit() {
            break;
        }
        let digit = (c - b'0') as i64;
        if !overflowed {
            let next = if negative {
                acc.checked_mul(10).and_then(|v| v.checked_sub(digit))
            } else {
                acc.checked_mul(10).and_then(|v| v.checked_add(digit))
            };
            match next {
                Some(v) => acc = v,
                None => {
                    overflowed = true;
                    acc = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        idx += 1;
    }

    if digits_start == sign_start && idx == digits_start {
        // No digits and no sign processed at all -> nothing parsed.
        return (0, 0);
    }
    if idx == digits_start {
        // A sign was consumed but no digits followed; C's strtol treats this
        // as "nothing parsed" and leaves `end` at the original string start.
        return (0, 0);
    }

    // Truncate to i32 the same way `int stride = strtol(...)` does in C.
    let value = acc as i32;
    (value, idx)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    // C's `argc != 2` means we need exactly one argument after the program name.
    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let arg = args[1].as_bytes();
    let (stride, parsed_len) = c_strtol_base10(arg);
    if parsed_len == 0 {
        // end == argv[1] in the original C code.
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    for i in 0..10i32 {
        let update = i.wrapping_mul(stride);
        println!("{}", static_sum(update));
    }

    ExitCode::from(0)
}
