use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(out: &mut W, bytes: &[u8]) {
    for b in bytes {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(out, &raw);
}

/// Mimic C's `scanf("%f", ...)`: skip leading whitespace, then match the
/// longest prefix of the remainder that looks like a C float literal and
/// parse it. Returns Some((value, bytes_consumed_including_skipped_ws))
/// or None if no valid float was matched (in which case the variable
/// stays at its initial value).
fn scan_float(input: &[u8]) -> Option<f32> {
    let mut i = 0usize;
    // Skip whitespace
    while i < input.len() && (input[i] as char).is_whitespace() {
        i += 1;
    }
    let start = i;

    // Optional sign
    if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
        i += 1;
    }

    let after_sign = i;

    // Check for inf/infinity (case-insensitive)
    let rest = &input[i..];
    let lower_starts_with = |s: &[u8], pat: &[u8]| -> bool {
        if s.len() < pat.len() { return false; }
        for k in 0..pat.len() {
            if s[k].to_ascii_lowercase() != pat[k] {
                return false;
            }
        }
        true
    };

    if lower_starts_with(rest, b"infinity") {
        i += 8;
    } else if lower_starts_with(rest, b"inf") {
        i += 3;
    } else if lower_starts_with(rest, b"nan") {
        i += 3;
        // optional "(...)"
        if i < input.len() && input[i] == b'(' {
            let mut j = i + 1;
            while j < input.len() && input[j] != b')' {
                let c = input[j];
                if !(c.is_ascii_alphanumeric() || c == b'_') {
                    break;
                }
                j += 1;
            }
            if j < input.len() && input[j] == b')' {
                i = j + 1;
            }
        }
    } else {
        // Try hex float: 0x... or 0X...
        if i + 1 < input.len() && input[i] == b'0' && (input[i+1] == b'x' || input[i+1] == b'X') {
            i += 2;
            let mut have_digit = false;
            while i < input.len() && input[i].is_ascii_hexdigit() {
                i += 1;
                have_digit = true;
            }
            if i < input.len() && input[i] == b'.' {
                i += 1;
                while i < input.len() && input[i].is_ascii_hexdigit() {
                    i += 1;
                    have_digit = true;
                }
            }
            if !have_digit {
                return None;
            }
            // optional p exponent
            if i < input.len() && (input[i] == b'p' || input[i] == b'P') {
                let exp_start = i;
                i += 1;
                if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
                    i += 1;
                }
                let exp_digit_start = i;
                while i < input.len() && input[i].is_ascii_digit() {
                    i += 1;
                }
                if i == exp_digit_start {
                    // No digits after p - back out the p
                    i = exp_start;
                }
            }
        } else {
            // Decimal float
            let mut have_digit = false;
            while i < input.len() && input[i].is_ascii_digit() {
                i += 1;
                have_digit = true;
            }
            if i < input.len() && input[i] == b'.' {
                i += 1;
                while i < input.len() && input[i].is_ascii_digit() {
                    i += 1;
                    have_digit = true;
                }
            }
            if !have_digit {
                // No digits seen - if we only consumed sign, fail
                if i == after_sign {
                    return None;
                }
                return None;
            }
            // optional exponent
            if i < input.len() && (input[i] == b'e' || input[i] == b'E') {
                let exp_start = i;
                i += 1;
                if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
                    i += 1;
                }
                let exp_digit_start = i;
                while i < input.len() && input[i].is_ascii_digit() {
                    i += 1;
                }
                if i == exp_digit_start {
                    i = exp_start;
                }
            }
        }
    }

    if i == after_sign {
        return None;
    }

    // Parse the matched slice
    let s = std::str::from_utf8(&input[start..i]).ok()?;
    // Rust's f32::from_str accepts most of these forms; for hex floats it does not.
    // Try directly first.
    if let Ok(v) = s.parse::<f32>() {
        return Some(v);
    }
    // Hex float fallback
    parse_hex_float(s)
}

fn parse_hex_float(s: &str) -> Option<f32> {
    // Manually parse C99 hex float: [+-]?0[xX]hexdigits[.hexdigits][pP[+-]?digits]
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut sign: f64 = 1.0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' { sign = -1.0; }
        i += 1;
    }
    if i + 1 >= bytes.len() || bytes[i] != b'0' || (bytes[i+1] != b'x' && bytes[i+1] != b'X') {
        return None;
    }
    i += 2;
    let mut mantissa: f64 = 0.0;
    let mut have_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
        let d = (bytes[i] as char).to_digit(16).unwrap() as f64;
        mantissa = mantissa * 16.0 + d;
        i += 1;
        have_digit = true;
    }
    let mut frac_exp: i32 = 0;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
            let d = (bytes[i] as char).to_digit(16).unwrap() as f64;
            mantissa = mantissa * 16.0 + d;
            frac_exp -= 4;
            i += 1;
            have_digit = true;
        }
    }
    if !have_digit { return None; }
    let mut bin_exp: i32 = 0;
    if i < bytes.len() && (bytes[i] == b'p' || bytes[i] == b'P') {
        i += 1;
        let mut exp_sign: i32 = 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            if bytes[i] == b'-' { exp_sign = -1; }
            i += 1;
        }
        let mut e: i32 = 0;
        let mut have_e_digit = false;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            e = e.saturating_mul(10).saturating_add((bytes[i] - b'0') as i32);
            i += 1;
            have_e_digit = true;
        }
        if !have_e_digit { return None; }
        bin_exp = exp_sign * e;
    }
    let total_exp = frac_exp + bin_exp;
    let value = sign * mantissa * 2f64.powi(total_exp);
    Some(value as f32)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let mut x: f32 = 0.0;
    if let Some(v) = scan_float(&input) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
    out.flush().unwrap();
}
