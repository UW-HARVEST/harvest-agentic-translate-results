// Translation of C code that reads a float from stdin via scanf("%f")
// and prints the raw bytes of the float as hex (little-endian on x86).

use std::io::{self, Read, Write};

fn print_hex(bytes: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut s = String::with_capacity(bytes.len() * 2 + 1);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    out.write_all(s.as_bytes()).unwrap();
}

fn driver(x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}

/// Read all of stdin into a String.
fn read_all_stdin() -> Vec<u8> {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).ok();
    buf
}

/// Parse a float from the byte slice mimicking scanf("%f", ...).
///
/// scanf("%f") behavior:
/// - skips leading whitespace (including newlines)
/// - parses an optional sign
/// - parses digits, optional decimal point and digits, optional exponent
/// - if no valid characters consumed after the sign, the conversion fails
///
/// Returns Some(value) if a valid float was parsed, otherwise None.
fn scanf_float(input: &[u8]) -> Option<f32> {
    let mut i = 0;
    // Skip leading whitespace (matches isspace in the C locale)
    while i < input.len() && is_c_whitespace(input[i]) {
        i += 1;
    }
    let start = i;

    // Optional sign
    if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
        i += 1;
    }

    // Try to parse hex float (0x...) or decimal
    let mut consumed_digits = false;

    // Check for hex float: 0x or 0X
    if i + 1 < input.len()
        && input[i] == b'0'
        && (input[i + 1] == b'x' || input[i + 1] == b'X')
    {
        i += 2;
        // hex digits
        while i < input.len() && is_hex_digit(input[i]) {
            i += 1;
            consumed_digits = true;
        }
        if i < input.len() && input[i] == b'.' {
            i += 1;
            while i < input.len() && is_hex_digit(input[i]) {
                i += 1;
                consumed_digits = true;
            }
        }
        // hex float exponent uses 'p' or 'P'
        if consumed_digits && i < input.len() && (input[i] == b'p' || input[i] == b'P') {
            let exp_start = i;
            i += 1;
            if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
                i += 1;
            }
            let exp_digits_start = i;
            while i < input.len() && input[i].is_ascii_digit() {
                i += 1;
            }
            if i == exp_digits_start {
                // exponent had no digits — back out
                i = exp_start;
            }
        }
    } else {
        // Decimal float
        while i < input.len() && input[i].is_ascii_digit() {
            i += 1;
            consumed_digits = true;
        }
        if i < input.len() && input[i] == b'.' {
            i += 1;
            while i < input.len() && input[i].is_ascii_digit() {
                i += 1;
                consumed_digits = true;
            }
        }
        if consumed_digits && i < input.len() && (input[i] == b'e' || input[i] == b'E') {
            let exp_start = i;
            i += 1;
            if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
                i += 1;
            }
            let exp_digits_start = i;
            while i < input.len() && input[i].is_ascii_digit() {
                i += 1;
            }
            if i == exp_digits_start {
                i = exp_start;
            }
        }
        // Check for inf/infinity and nan (case-insensitive)
        if !consumed_digits {
            if let Some(end) = match_keyword(&input[i..], b"infinity") {
                i += end;
                consumed_digits = true;
            } else if let Some(end) = match_keyword(&input[i..], b"inf") {
                i += end;
                consumed_digits = true;
            } else if let Some(end) = match_keyword(&input[i..], b"nan") {
                i += end;
                consumed_digits = true;
            }
        }
    }

    if !consumed_digits {
        return None;
    }

    let s = std::str::from_utf8(&input[start..i]).ok()?;
    s.parse::<f32>().ok()
}

fn match_keyword(input: &[u8], keyword: &[u8]) -> Option<usize> {
    if input.len() < keyword.len() {
        return None;
    }
    for (a, b) in input.iter().zip(keyword.iter()) {
        if a.to_ascii_lowercase() != *b {
            return None;
        }
    }
    Some(keyword.len())
}

fn is_c_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b)
}

fn main() {
    let buf = read_all_stdin();
    let x: f32 = scanf_float(&buf).unwrap_or(0.0);
    driver(x);
}
