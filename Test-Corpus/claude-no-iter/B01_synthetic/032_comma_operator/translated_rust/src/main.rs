use std::io::{self, Read, Write, BufWriter};

fn driver<W: Write>(x: i32, out: &mut W) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        writeln!(out, "{} {}", i, j).unwrap();
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

/// Mimic C's scanf("%d", &x) for a single integer.
/// - Skips leading whitespace (matches isspace: space, \t, \n, \v, \f, \r).
/// - Parses an optional sign followed by decimal digits.
/// - Returns the parsed integer, or None if no valid integer was found.
/// - Stops at the first non-digit character (which is left in the buffer).
/// - On overflow, C scanf has undefined behavior; we saturate to match
///   typical implementations' tendency to wrap, but for non-overflowing
///   inputs the behavior is byte-identical.
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n'
            || c == 0x0B || c == 0x0C || c == b'\r'
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
            value = value
                .wrapping_mul(10)
                .wrapping_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos == digits_start {
        // No digits parsed; restore pos and signal failure
        *pos = start;
        return None;
    }

    let signed = sign.wrapping_mul(value);
    Some(signed as i32)
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).expect("failed to read stdin");

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut pos = 0usize;
    let x: i32 = scanf_int(&buf, &mut pos).unwrap_or(0);

    driver(x, &mut out);
}
