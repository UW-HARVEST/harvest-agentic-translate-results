// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::io::{self, Read, Write};

#[repr(C)]
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
    // Mirror `house_t house = {0};` then field assignments.
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };

    // Build the byte representation matching the C struct layout on x86_64.
    // Layout: [floors:i32][bedrooms:i32][bathrooms:f64], no padding (4+4+8 = 16 bytes).
    let mut bytes = [0u8; std::mem::size_of::<House>()];
    let f = house.floors.to_le_bytes();
    let b = house.bedrooms.to_le_bytes();
    let ba = house.bathrooms.to_le_bytes();
    bytes[0..4].copy_from_slice(&f);
    bytes[4..8].copy_from_slice(&b);
    bytes[8..16].copy_from_slice(&ba);
    print_hex(&bytes);
}

/// Mimic C's `scanf("%d", &x)` for a single decimal integer:
/// - Skip leading whitespace (including newlines).
/// - Optional sign.
/// - Read digits until a non-digit or EOF.
/// - If no digits are read, behave like scanf returning 0 matches; the C code
///   leaves `x` at its initialized value of 0 in that case.
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip whitespace
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }
    if i >= input.len() {
        return 0;
    }
    let mut neg = false;
    if input[i] == b'-' {
        neg = true;
        i += 1;
    } else if input[i] == b'+' {
        i += 1;
    }
    let mut any = false;
    let mut val: i64 = 0;
    while i < input.len() {
        let c = input[i];
        if c.is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            any = true;
            i += 1;
        } else {
            break;
        }
    }
    if !any {
        return 0;
    }
    let result = if neg { val.wrapping_neg() } else { val };
    result as i32
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();
    let x = scanf_int(&buf);
    driver(x);
}
