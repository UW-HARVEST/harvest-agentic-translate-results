// Copyright 2025 MIT Lincoln Laboratory
// Translated from c_src/src/main.c to Rust.
// Reproduces the behavior of the original C program byte-for-byte.

use std::io::{self, Read, Write};

fn driver(x: i32) {
    // y = 2*x; y += 300; using wrapping arithmetic to mimic C int overflow
    let mut y: i32 = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    // printf("%d\n", y);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", y);
}

/// Mimic C scanf("%d", &x). Skips leading whitespace, then reads optional
/// sign followed by digits. On parse failure, leaves x unchanged (matching
/// scanf semantics where the destination is not assigned).
///
/// Returns true if a value was successfully parsed.
fn scanf_int(input: &[u8], pos: &mut usize, out: &mut i32) -> bool {
    // Skip leading whitespace (matches C isspace for the "C" locale).
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0b /* \v */ || c == 0x0c /* \f */
        {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= input.len() {
        return false;
    }

    let start = *pos;
    let mut sign: i64 = 1;
    if input[*pos] == b'+' {
        *pos += 1;
    } else if input[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }

    let digits_start = *pos;
    let mut value: i64 = 0;
    while *pos < input.len() {
        let c = input[*pos];
        if c.is_ascii_digit() {
            // Accumulate using i64 then wrap to i32 to mimic C int overflow.
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos == digits_start {
        // No digits consumed; rewind sign and report failure.
        *pos = start;
        return false;
    }

    let signed_value = sign.wrapping_mul(value);
    // Truncate to i32 like C int.
    *out = signed_value as i32;
    true
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let mut x: i32 = 0;
    let mut pos: usize = 0;
    let _ = scanf_int(&input, &mut pos, &mut x);

    driver(x);
}
