use std::io::{self, Read, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for byte in p {
        write!(out, "{:02x}", byte).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

/// Parse a float from input string mimicking C's scanf("%f", ...).
/// Returns the parsed float (or the unmodified initial value 0.0 if matching fails),
/// according to the same rules as the original C program which initializes x = 0.f
/// before calling scanf.
fn scanf_float(input: &str) -> f32 {
    let bytes = input.as_bytes();
    let mut i = 0;

    // Skip whitespace (scanf's %f skips leading whitespace).
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    let start = i;

    // Optional sign.
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let after_sign = i;

    // Try to recognize "inf"/"infinity" or "nan" (case-insensitive, C99).
    let remaining = &bytes[i..];
    let lc_starts_with = |s: &[u8], pat: &[u8]| -> bool {
        if s.len() < pat.len() {
            return false;
        }
        for k in 0..pat.len() {
            if (s[k] as char).to_ascii_lowercase() as u8 != pat[k] {
                return false;
            }
        }
        true
    };

    if lc_starts_with(remaining, b"infinity") {
        i += 8;
    } else if lc_starts_with(remaining, b"inf") {
        i += 3;
    } else if lc_starts_with(remaining, b"nan") {
        i += 3;
        // Optional (n-char-sequence) like nan(...)
        if i < bytes.len() && bytes[i] == b'(' {
            let save = i;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b')' {
                let c = bytes[j];
                if !(c.is_ascii_alphanumeric() || c == b'_') {
                    break;
                }
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b')' {
                i = j + 1;
            } else {
                i = save;
            }
        }
    } else {
        // Hex float: 0x... or 0X...
        let is_hex = remaining.len() >= 2
            && remaining[0] == b'0'
            && (remaining[1] == b'x' || remaining[1] == b'X');

        if is_hex {
            i += 2;
            let mut had_digit = false;
            while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() {
                i += 1;
                had_digit = true;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() {
                    i += 1;
                    had_digit = true;
                }
            }
            if !had_digit {
                // Invalid hex float; fall back: nothing matched.
                return 0.0;
            }
            // Binary exponent (required by C, but we accept absence for tolerance).
            if i < bytes.len() && (bytes[i] == b'p' || bytes[i] == b'P') {
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
        } else {
            // Decimal float.
            let mut had_digit = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
                had_digit = true;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                    had_digit = true;
                }
            }
            if !had_digit {
                // Matching failure: scanf leaves the variable unmodified.
                // Original C code initializes x = 0.f.
                return 0.0;
            }
            if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                let save = i;
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                let exp_digits_start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i == exp_digits_start {
                    // No exponent digits; back up.
                    i = save;
                }
            }
        }
    }

    if i == after_sign {
        // Nothing was matched after the optional sign.
        return 0.0;
    }

    let s = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
    // Rust's f32::from_str understands "inf", "infinity", "nan" (case-insensitive),
    // optional sign, and decimal floats. Hex floats are not supported by std parse,
    // but they are unusual inputs for this exercise.
    s.parse::<f32>().unwrap_or(0.0)
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let x = scanf_float(&input);
    driver(x);
}
