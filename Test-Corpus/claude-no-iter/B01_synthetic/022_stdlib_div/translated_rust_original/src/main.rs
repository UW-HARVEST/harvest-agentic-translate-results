// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output for the same inputs.

use std::io::{self, BufWriter, Read, Write};

fn parse_int(bytes: &[u8], mut pos: usize) -> Option<(i32, usize)> {
    // Skip leading whitespace (matches scanf "%d" behavior).
    while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= bytes.len() {
        return None;
    }

    let mut negative = false;
    if bytes[pos] == b'-' {
        negative = true;
        pos += 1;
    } else if bytes[pos] == b'+' {
        pos += 1;
    }

    let digit_start = pos;
    while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
        pos += 1;
    }

    if pos == digit_start {
        // No digits read; conversion failed.
        return None;
    }

    // Use wrapping arithmetic; C's behavior on overflow is undefined,
    // and we want to avoid panics for unusual inputs.
    let mut val: i32 = 0;
    for &b in &bytes[digit_start..pos] {
        let d = (b - b'0') as i32;
        val = val.wrapping_mul(10).wrapping_add(d);
    }
    if negative {
        val = val.wrapping_neg();
    }

    Some((val, pos))
}

fn main() {
    // Match the C initialization: int x = 1, y = 1;
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    // Read all of stdin (scanf reads across newlines).
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let bytes = input.as_bytes();

    // scanf("%d %d", &x, &y);
    // If a conversion fails, the corresponding variable retains its prior value.
    let mut pos = 0usize;
    if let Some((val, new_pos)) = parse_int(bytes, pos) {
        x = val;
        pos = new_pos;
        if let Some((val2, _)) = parse_int(bytes, pos) {
            y = val2;
        }
    }

    // div_t result = div(x, y);
    // C99 div() uses truncation toward zero, which matches Rust's i32 / and %.
    let quot = x / y;
    let rem = x % y;

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    // printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
    writeln!(out, "quotient: {}, remainder: {}", quot, rem).unwrap();
}
