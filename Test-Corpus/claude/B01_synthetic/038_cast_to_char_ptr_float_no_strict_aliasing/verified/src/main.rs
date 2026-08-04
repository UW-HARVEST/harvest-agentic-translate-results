use std::io::{self, Read, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: f32) {
    // memcpy the raw bytes of x. On little-endian machines (x86_64),
    // this is f32::to_le_bytes.
    let raw = x.to_le_bytes();
    print_hex(&raw);
}

/// Mimics scanf("%f", &x). Skips leading whitespace, then attempts to
/// parse a float in C's format. Returns the parsed float, or None if
/// no valid float was found.
fn scanf_float(input: &[u8]) -> Option<f32> {
    let mut i = 0;
    let n = input.len();

    // Skip leading whitespace
    while i < n && (input[i] as char).is_ascii_whitespace() {
        i += 1;
    }

    let start = i;

    // Optional sign
    if i < n && (input[i] == b'+' || input[i] == b'-') {
        i += 1;
    }

    let num_start = i;

    // Check for hex prefix (C's strtof supports it)
    let is_hex = i + 1 < n
        && input[i] == b'0'
        && (input[i + 1] == b'x' || input[i + 1] == b'X');

    if is_hex {
        i += 2;
        let mantissa_start = i;
        while i < n && (input[i] as char).is_ascii_hexdigit() {
            i += 1;
        }
        if i < n && input[i] == b'.' {
            i += 1;
            while i < n && (input[i] as char).is_ascii_hexdigit() {
                i += 1;
            }
        }
        if i == mantissa_start {
            return None;
        }
        // Optional binary exponent
        if i < n && (input[i] == b'p' || input[i] == b'P') {
            i += 1;
            if i < n && (input[i] == b'+' || input[i] == b'-') {
                i += 1;
            }
            while i < n && (input[i] as char).is_ascii_digit() {
                i += 1;
            }
        }
    } else {
        // Decimal: digits, optional . digits, optional exponent
        let mut saw_digits = false;
        while i < n && (input[i] as char).is_ascii_digit() {
            i += 1;
            saw_digits = true;
        }
        if i < n && input[i] == b'.' {
            i += 1;
            while i < n && (input[i] as char).is_ascii_digit() {
                i += 1;
                saw_digits = true;
            }
        }
        if !saw_digits {
            // Could be inf/infinity or nan (C accepts these). For
            // simplicity, try to fall through to parsing whatever was
            // collected.
            return None;
        }
        if i < n && (input[i] == b'e' || input[i] == b'E') {
            i += 1;
            if i < n && (input[i] == b'+' || input[i] == b'-') {
                i += 1;
            }
            while i < n && (input[i] as char).is_ascii_digit() {
                i += 1;
            }
        }
    }

    if num_start == i {
        return None;
    }

    let s = std::str::from_utf8(&input[start..i]).ok()?;
    s.parse::<f32>().ok()
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).ok();

    let mut x: f32 = 0.0;
    if let Some(parsed) = scanf_float(&buf) {
        x = parsed;
    }
    driver(x);
}
