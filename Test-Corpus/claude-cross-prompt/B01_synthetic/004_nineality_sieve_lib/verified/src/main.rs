// Translated from c_src/src/sieve.c
//
// The original C package is a shared library exposing `sieve(int)`.
// This executable reads an integer from stdin (mimicking C's
// `scanf("%d", &val)`) and calls the translated `sieve` function.

use std::io::{self, Read, Write};

/// Count from a starting point, stopping when the count ends in 9 (base 10).
///
/// Mirrors the C implementation in c_src/src/sieve.c exactly.
fn sieve(mut val: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    loop {
        // Match C's `printf("%d\n", val);`
        let _ = writeln!(out, "{}", val);
        if val % 10 == 9 {
            break;
        }
        // Use wrapping_add to mirror typical 2's-complement behavior of C
        // when integer overflow occurs (which would be undefined behavior
        // in C, but most compilers wrap in practice).
        val = val.wrapping_add(1);
    }
}

/// Read all of stdin and parse the first integer token, mimicking
/// `scanf("%d", &val)` semantics (skip leading whitespace, read an
/// optional sign, then digits).
fn read_int_from_stdin() -> Option<i32> {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return None;
    }
    let bytes = buf.as_bytes();
    let mut i = 0;
    // Skip leading whitespace (matches C's scanf for %d).
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    let token = std::str::from_utf8(&bytes[start..i]).ok()?;
    // Use wrapping behavior similar to C on overflow: parse as i64 then truncate.
    if let Ok(v) = token.parse::<i32>() {
        Some(v)
    } else if let Ok(v64) = token.parse::<i64>() {
        Some(v64 as i32)
    } else {
        None
    }
}

fn main() {
    if let Some(val) = read_int_from_stdin() {
        sieve(val);
    }
}
