// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(out: &mut W, bytes: &[u8]) {
    for b in bytes {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(out, &bytes);
}

/// Reads scanf("%f", ...) style: skips leading whitespace, then parses a float
/// according to C's strtof grammar (as much as we need).
/// Returns Some(f) when a float was successfully consumed, None on EOF/match
/// failure.
fn scan_float(input: &[u8], pos: &mut usize) -> Option<f32> {
    // Skip leading whitespace (matches C isspace for typical ASCII inputs).
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0b || c == 0x0c
        {
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos >= input.len() {
        return None;
    }

    let start = *pos;

    // Optional sign
    if input[*pos] == b'+' || input[*pos] == b'-' {
        *pos += 1;
    }

    let digits_or_dot_start = *pos;

    // Integer part digits
    let mut had_digits = false;
    while *pos < input.len() && input[*pos].is_ascii_digit() {
        *pos += 1;
        had_digits = true;
    }

    // Fractional part
    if *pos < input.len() && input[*pos] == b'.' {
        *pos += 1;
        while *pos < input.len() && input[*pos].is_ascii_digit() {
            *pos += 1;
            had_digits = true;
        }
    }

    if !had_digits {
        // Could be inf/nan handling in C, but skip for simplicity; on match
        // failure rewind position and return None.
        *pos = start;
        return None;
    }

    // Exponent
    if *pos < input.len() && (input[*pos] == b'e' || input[*pos] == b'E') {
        let exp_start = *pos;
        *pos += 1;
        if *pos < input.len() && (input[*pos] == b'+' || input[*pos] == b'-') {
            *pos += 1;
        }
        let mut had_exp_digits = false;
        while *pos < input.len() && input[*pos].is_ascii_digit() {
            *pos += 1;
            had_exp_digits = true;
        }
        if !had_exp_digits {
            // Roll back to before exponent marker.
            *pos = exp_start;
        }
    }

    let _ = digits_or_dot_start;
    let s = std::str::from_utf8(&input[start..*pos]).ok()?;
    s.parse::<f32>().ok()
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).expect("failed to read stdin");

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut pos = 0usize;
    while let Some(x) = scan_float(&input, &mut pos) {
        driver(&mut out, x);
    }

    out.flush().unwrap();
}
