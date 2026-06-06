use std::io::{self, Read, Write, BufWriter};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    for &b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

/// Mimic C's scanf("%f") behavior: skip leading whitespace, then consume
/// the longest prefix that looks like a float. If no valid float can be
/// parsed, return None (caller will keep its default 0.0).
fn scan_float(input: &str) -> Option<f32> {
    let bytes = input.as_bytes();
    let mut i = 0;
    // Skip leading whitespace (matches isspace: space, tab, \n, \v, \f, \r)
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }
    let start = i;

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let after_sign = i;

    // Check for inf / infinity / nan (case-insensitive)
    let lower_eq = |s: &[u8], pat: &[u8]| -> bool {
        if s.len() < pat.len() {
            return false;
        }
        for k in 0..pat.len() {
            if s[k].to_ascii_lowercase() != pat[k] {
                return false;
            }
        }
        true
    };

    if lower_eq(&bytes[i..], b"infinity") {
        i += 8;
    } else if lower_eq(&bytes[i..], b"inf") {
        i += 3;
    } else if lower_eq(&bytes[i..], b"nan") {
        i += 3;
        // Optional nan(...) sequence
        if i < bytes.len() && bytes[i] == b'(' {
            let save = i;
            i += 1;
            let mut found_close = false;
            while i < bytes.len() {
                let c = bytes[i];
                if c == b')' {
                    i += 1;
                    found_close = true;
                    break;
                }
                if c.is_ascii_alphanumeric() || c == b'_' {
                    i += 1;
                } else {
                    break;
                }
            }
            if !found_close {
                i = save;
            }
        }
    } else {
        // Numeric form: digits, optional fractional part, optional exponent
        let mut has_digits = false;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            has_digits = true;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
                has_digits = true;
            }
        }
        if !has_digits {
            return None;
        }
        // Optional exponent
        if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
            let save = i;
            i += 1;
            if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
            let exp_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == exp_start {
                // No exponent digits; back up
                i = save;
            }
        }
    }

    if i == after_sign {
        // Nothing parsed beyond the sign — not a valid number
        return None;
    }

    let token = &input[start..i];
    token.parse::<f32>().ok()
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok();

    let mut x: f32 = 0.0;
    if let Some(v) = scan_float(&input) {
        x = v;
    }
    driver(x);
}
