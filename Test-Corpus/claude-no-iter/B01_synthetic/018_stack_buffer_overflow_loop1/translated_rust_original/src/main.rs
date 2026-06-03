// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust, preserving original behavior.

use std::io::{self, Read, Write};

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

#[allow(dead_code)]
fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn bad() {
    // C code does: data = (int *)alloca(10);
    // This allocates only 10 bytes, but the loop writes 10 ints (40 bytes on
    // typical systems) — a buffer overflow. Since the source array is all
    // zeros, data[0] still ends up being 0 in the printed value. We reproduce
    // the observable behavior (printing 0) using safe Rust.
    let source: [i32; 10] = [0; 10];
    let mut data: [i32; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let source: [i32; 10] = [0; 10];
    let mut data: [i32; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

/// Read all of stdin and parse the first integer the way C's scanf("%d", ...)
/// would: skip leading whitespace, accept an optional sign, then read decimal
/// digits until a non-digit is encountered. If no integer is found, the value
/// stays at the initialized 0 (matching the C program where `x` is initialized
/// to 0 before scanf).
fn scanf_int_or_zero() -> i32 {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return 0;
    }
    let bytes = buf.as_bytes();
    let mut i = 0usize;
    // Skip whitespace (matches C isspace defaults: space, tab, newline, vt, ff, cr).
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }
    if i >= bytes.len() {
        return 0;
    }
    let mut negative = false;
    if bytes[i] == b'+' {
        i += 1;
    } else if bytes[i] == b'-' {
        negative = true;
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        // No digits read; matches scanf failure where x retains its prior value (0).
        return 0;
    }
    if negative {
        value = value.wrapping_neg();
    }
    value as i32
}

fn main() {
    let x: i32 = scanf_int_or_zero();

    if x != 0 {
        good();
    } else {
        bad();
    }

    // Make sure output is flushed before exit.
    let _ = io::stdout().flush();
}
