use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(out: &mut W, p: &[u8]) {
    for &b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: i32) {
    let raw = x.to_ne_bytes();
    print_hex(out, &raw);
}

/// Emulate `scanf("%d", &x)` for a single integer read from stdin bytes.
/// Returns Some(value) on successful match, None on failure.
/// Reads beyond newlines as scanf does (skips whitespace including newlines).
fn scanf_int(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (space, tab, newline, vertical tab, form feed, carriage return)
    while *pos < bytes.len() {
        let c = bytes[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r' {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= bytes.len() {
        return None;
    }

    let mut negative = false;
    let c = bytes[*pos];
    if c == b'+' {
        *pos += 1;
    } else if c == b'-' {
        negative = true;
        *pos += 1;
    }

    // Need at least one digit
    let start = *pos;
    let mut value: i64 = 0;
    let mut had_digit = false;
    while *pos < bytes.len() {
        let c = bytes[*pos];
        if c.is_ascii_digit() {
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            had_digit = true;
            *pos += 1;
        } else {
            break;
        }
    }

    if !had_digit {
        // Roll back if we consumed sign but no digits
        *pos = start;
        return None;
    }

    if negative {
        value = value.wrapping_neg();
    }

    // Truncate to i32 (matching C int on typical platform)
    Some(value as i32)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).ok();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut pos = 0;
    let x = scanf_int(&input, &mut pos).unwrap_or(0);

    driver(&mut out, x);
}
