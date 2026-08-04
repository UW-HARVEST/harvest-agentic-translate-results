use std::io::{self, Read, Write};

/// Reads a float from stdin in a manner that mimics C's `scanf("%f", ...)`.
///
/// Skips leading whitespace and consumes the longest prefix of a valid
/// floating-point literal. Returns 0.0 if no characters could be read,
/// matching the program's initialized value of `x`.
fn scanf_float(input: &[u8], pos: &mut usize) -> f32 {
    // Skip leading whitespace.
    while *pos < input.len() && (input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }

    let start = *pos;
    let mut buf: Vec<u8> = Vec::new();

    // Optional sign.
    if *pos < input.len() && (input[*pos] == b'+' || input[*pos] == b'-') {
        buf.push(input[*pos]);
        *pos += 1;
    }

    // Check for "inf"/"infinity" or "nan" (case-insensitive).
    let remaining_lower: String = input[*pos..]
        .iter()
        .take(8)
        .map(|&b| (b as char).to_ascii_lowercase())
        .collect();

    if remaining_lower.starts_with("infinity") {
        buf.extend_from_slice(&input[*pos..*pos + 8]);
        *pos += 8;
    } else if remaining_lower.starts_with("inf") {
        buf.extend_from_slice(&input[*pos..*pos + 3]);
        *pos += 3;
    } else if remaining_lower.starts_with("nan") {
        buf.extend_from_slice(&input[*pos..*pos + 3]);
        *pos += 3;
    } else {
        // Read digits before decimal point.
        let mut had_digits = false;
        while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
            buf.push(input[*pos]);
            *pos += 1;
            had_digits = true;
        }

        // Optional decimal point and following digits.
        if *pos < input.len() && input[*pos] == b'.' {
            buf.push(input[*pos]);
            *pos += 1;
            while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
                buf.push(input[*pos]);
                *pos += 1;
                had_digits = true;
            }
        }

        if !had_digits {
            // No digits found; parsing fails. Restore position and return 0.0.
            *pos = start;
            return 0.0;
        }

        // Optional exponent.
        if *pos < input.len() && (input[*pos] == b'e' || input[*pos] == b'E') {
            let exp_start = *pos;
            let mut exp_buf: Vec<u8> = Vec::new();
            exp_buf.push(input[*pos]);
            *pos += 1;
            if *pos < input.len() && (input[*pos] == b'+' || input[*pos] == b'-') {
                exp_buf.push(input[*pos]);
                *pos += 1;
            }
            let mut exp_digits = false;
            while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
                exp_buf.push(input[*pos]);
                *pos += 1;
                exp_digits = true;
            }
            if exp_digits {
                buf.extend_from_slice(&exp_buf);
            } else {
                // Roll back the exponent characters; they aren't part of the number.
                *pos = exp_start;
            }
        }
    }

    let s = std::str::from_utf8(&buf).unwrap_or("");
    s.parse::<f32>().unwrap_or(0.0)
}

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

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let mut pos: usize = 0;
    let x: f32 = scanf_float(&input, &mut pos);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, x);
}
