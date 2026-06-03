// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output to the original C.

use std::io::{self, Read, Write};

#[repr(C)]
#[derive(Default)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(bytes: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for b in bytes {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(floors: i32) {
    // Equivalent to: house_t house = {0}; in C, which zero-initializes
    // the entire struct including any padding bytes.
    let size = std::mem::size_of::<House>();
    let mut buf = vec![0u8; size];

    // Build the struct then copy its bytes (including padding which remains 0
    // because we wrote each field without disturbing surrounding padding).
    // To guarantee padding bytes are zero, we construct a zeroed buffer and
    // write fields into the buffer at the correct offsets matching #[repr(C)].
    let floors_offset = 0usize;
    let bedrooms_offset = std::mem::offset_of!(House, bedrooms);
    let bathrooms_offset = std::mem::offset_of!(House, bathrooms);

    let floors_val: i32 = floors;
    let bedrooms_val: i32 = 3;
    let bathrooms_val: f64 = 2.0;

    buf[floors_offset..floors_offset + 4].copy_from_slice(&floors_val.to_ne_bytes());
    buf[bedrooms_offset..bedrooms_offset + 4].copy_from_slice(&bedrooms_val.to_ne_bytes());
    buf[bathrooms_offset..bathrooms_offset + 8].copy_from_slice(&bathrooms_val.to_ne_bytes());

    print_hex(&buf);
}

/// Read an integer from stdin, mimicking `scanf("%d", ...)` behavior:
/// skip leading whitespace (including newlines), then read an optional sign
/// followed by decimal digits. If parsing fails, the C code leaves `x`
/// unchanged at its initial value of 0.
fn scanf_int() -> i32 {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return 0;
    }

    let bytes = input.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace as scanf does.
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    if i >= bytes.len() {
        return 0;
    }

    let start = i;

    // Optional sign
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }

    let digits_start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }

    if i == digits_start {
        // No digits - scanf returns no value, x stays 0
        return 0;
    }

    let s = std::str::from_utf8(&bytes[start..i]).unwrap_or("0");
    // Wrap on overflow to mimic C's undefined behavior in a deterministic way.
    s.parse::<i64>().map(|v| v as i32).unwrap_or(0)
}

fn main() {
    let x = scanf_int();
    driver(x);
}
