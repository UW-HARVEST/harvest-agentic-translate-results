use std::io::{Read, Write};

fn print_hex(p: &[u8]) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut s = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    out.write_all(s.as_bytes()).unwrap();
}

fn driver(x: f32) {
    let raw = x.to_le_bytes();
    print_hex(&raw);
}

/// Mimic C's `scanf("%f", ...)` parsing: skip leading whitespace, then
/// consume the longest prefix matching a C float literal. If nothing
/// matches, the value is left unchanged (here, 0.0).
fn read_float_scanf(input: &[u8]) -> f32 {
    let n = input.len();
    let mut i = 0;
    // Skip whitespace
    while i < n && (input[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let start = i;

    // Optional sign
    if i < n && (input[i] == b'+' || input[i] == b'-') {
        i += 1;
    }

    // Check for inf / infinity / nan (case-insensitive)
    let lower = |b: u8| -> u8 {
        if (b'A'..=b'Z').contains(&b) {
            b + 32
        } else {
            b
        }
    };

    let starts_with_ci = |bytes: &[u8], pat: &[u8]| -> bool {
        if bytes.len() < pat.len() {
            return false;
        }
        for k in 0..pat.len() {
            if lower(bytes[k]) != pat[k] {
                return false;
            }
        }
        true
    };

    if starts_with_ci(&input[i..], b"infinity") {
        i += 8;
    } else if starts_with_ci(&input[i..], b"inf") {
        i += 3;
    } else if starts_with_ci(&input[i..], b"nan") {
        i += 3;
        // Skip optional (n-char-sequence)
        if i < n && input[i] == b'(' {
            let mut j = i + 1;
            while j < n && input[j] != b')' {
                let c = input[j];
                if !(c.is_ascii_alphanumeric() || c == b'_') {
                    break;
                }
                j += 1;
            }
            if j < n && input[j] == b')' {
                i = j + 1;
            }
        }
    } else if i + 1 < n
        && input[i] == b'0'
        && (input[i + 1] == b'x' || input[i + 1] == b'X')
    {
        // Hex float
        i += 2;
        let mut has_digits = false;
        while i < n && (input[i] as char).is_ascii_hexdigit() {
            i += 1;
            has_digits = true;
        }
        if i < n && input[i] == b'.' {
            i += 1;
            while i < n && (input[i] as char).is_ascii_hexdigit() {
                i += 1;
                has_digits = true;
            }
        }
        if !has_digits {
            // Roll back the 0x to just match "0"
            i = start
                + (if input[start] == b'+' || input[start] == b'-' {
                    1
                } else {
                    0
                })
                + 1;
        } else if i < n && (input[i] == b'p' || input[i] == b'P') {
            let exp_start = i;
            i += 1;
            if i < n && (input[i] == b'+' || input[i] == b'-') {
                i += 1;
            }
            let exp_digit_start = i;
            while i < n && (input[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i == exp_digit_start {
                // No digits after p — exponent is invalid; back up
                i = exp_start;
            }
        }
    } else {
        // Decimal float
        let mut has_digits = false;
        while i < n && (input[i] as char).is_ascii_digit() {
            i += 1;
            has_digits = true;
        }
        if i < n && input[i] == b'.' {
            i += 1;
            while i < n && (input[i] as char).is_ascii_digit() {
                i += 1;
                has_digits = true;
            }
        }
        if has_digits && i < n && (input[i] == b'e' || input[i] == b'E') {
            let exp_start = i;
            i += 1;
            if i < n && (input[i] == b'+' || input[i] == b'-') {
                i += 1;
            }
            let exp_digit_start = i;
            while i < n && (input[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i == exp_digit_start {
                i = exp_start;
            }
        }

        if !has_digits {
            // Could not parse anything — return 0.0 (initial value)
            return 0.0;
        }
    }

    let s = match std::str::from_utf8(&input[start..i]) {
        Ok(s) => s,
        Err(_) => return 0.0,
    };

    // Rust's f32::parse handles "inf"/"infinity"/"nan" and decimal/exp,
    // but does not handle hex floats. Fall back to f64 then cast for
    // hex floats via a small custom parser.
    if let Ok(v) = s.parse::<f32>() {
        return v;
    }

    // Hex float fallback
    parse_hex_float(s).unwrap_or(0.0)
}

fn parse_hex_float(s: &str) -> Option<f32> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut sign = 1.0f64;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1.0;
        }
        i += 1;
    }
    if i + 1 >= bytes.len() || bytes[i] != b'0' || (bytes[i + 1] != b'x' && bytes[i + 1] != b'X') {
        return None;
    }
    i += 2;

    let mut mantissa: f64 = 0.0;
    let mut frac_scale: f64 = 1.0;
    let mut in_frac = false;
    let mut any_digit = false;

    while i < bytes.len() {
        let c = bytes[i];
        if c == b'.' && !in_frac {
            in_frac = true;
            i += 1;
            continue;
        }
        let digit_val = match c {
            b'0'..=b'9' => (c - b'0') as i32,
            b'a'..=b'f' => (c - b'a' + 10) as i32,
            b'A'..=b'F' => (c - b'A' + 10) as i32,
            _ => break,
        };
        any_digit = true;
        if in_frac {
            frac_scale /= 16.0;
            mantissa += (digit_val as f64) * frac_scale;
        } else {
            mantissa = mantissa * 16.0 + (digit_val as f64);
        }
        i += 1;
    }

    if !any_digit {
        return None;
    }

    let mut exp: i32 = 0;
    if i < bytes.len() && (bytes[i] == b'p' || bytes[i] == b'P') {
        i += 1;
        let mut exp_sign = 1i32;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            if bytes[i] == b'-' {
                exp_sign = -1;
            }
            i += 1;
        }
        let mut got_exp_digit = false;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            exp = exp.saturating_mul(10).saturating_add((bytes[i] - b'0') as i32);
            got_exp_digit = true;
            i += 1;
        }
        if !got_exp_digit {
            return None;
        }
        exp *= exp_sign;
    }

    let val = sign * mantissa * (2.0f64).powi(exp);
    Some(val as f32)
}

fn main() {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).unwrap();
    let x = read_float_scanf(&buf);
    driver(x);
}
