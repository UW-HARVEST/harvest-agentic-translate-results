// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::io::{self, Read, Write, BufWriter};

/// Equivalent of the C `driver` function:
///
/// void driver(int x) {
///     for (int i = 0, j = 0; i < x; i++, j += 2) {
///         printf("%d %d\n", i, j);
///     }
/// }
fn driver(x: i32) {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        // Use wrapping arithmetic to mirror C's integer overflow behavior.
        let _ = writeln!(out, "{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

/// Read all remaining whitespace-separated integers from stdin (scanf-like
/// behavior: reads across newlines and skips leading whitespace).
fn read_next_int(buf: &str, pos: &mut usize) -> Option<i32> {
    let bytes = buf.as_bytes();
    // Skip whitespace.
    while *pos < bytes.len() && (bytes[*pos] as char).is_whitespace() {
        *pos += 1;
    }
    if *pos >= bytes.len() {
        return None;
    }
    let start = *pos;
    if bytes[*pos] == b'+' || bytes[*pos] == b'-' {
        *pos += 1;
    }
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_digit() {
        *pos += 1;
    }
    let token = &buf[start..*pos];
    token.parse::<i32>().ok()
}

fn main() {
    // Read all of stdin (matches scanf behavior of reading across newlines).
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let mut pos = 0usize;
    if let Some(x) = read_next_int(&input, &mut pos) {
        driver(x);
    }
}
