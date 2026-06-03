// Copyright 2025 MIT Lincoln Laboratory
// Rust translation that reproduces byte-identical output of the original C.

use std::io::{self, Read, Write, BufWriter};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    // No explicit padding needed: i32 i32 are 8 bytes, f64 aligns to 8.
    bathrooms: f64,
}

fn print_hex<W: Write>(w: &mut W, bytes: &[u8]) {
    for b in bytes {
        write!(w, "{:02x}", b).unwrap();
    }
    writeln!(w).unwrap();
}

fn driver<W: Write>(w: &mut W, floors: i32) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    // Mirror `char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));`
    // by serializing the struct using the platform's little-endian layout
    // (matching x86/x86_64 native layout that the original C produces).
    let mut raw = [0u8; 16];
    raw[0..4].copy_from_slice(&house.floors.to_le_bytes());
    raw[4..8].copy_from_slice(&house.bedrooms.to_le_bytes());
    raw[8..16].copy_from_slice(&house.bathrooms.to_le_bytes());
    print_hex(w, &raw);
}

/// Mimic C `scanf("%d", &x)` semantics:
///  * skip leading whitespace
///  * read optional sign and digits
///  * stop at first non-digit
///  * on matching failure, leave x unchanged (here: 0)
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip whitespace
    while i < input.len() && (input[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= input.len() {
        return 0;
    }
    let mut neg = false;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        neg = true;
        i += 1;
    }
    let start = i;
    let mut val: i64 = 0;
    while i < input.len() && (input[i] as char).is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((input[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        // No digits read => matching failure; x remains 0.
        return 0;
    }
    if neg {
        val = val.wrapping_neg();
    }
    val as i32
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();
    let x = scanf_int(&buf);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
}
