// Translation of c_src/src/main.c to Rust.
// Produces byte-identical output for the same inputs.

use std::io::{self, Read, Write};

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        // C's scanf would just leave f as 0.0 if it can't read. Mirror that.
    }

    // C's scanf("%lf", &f) starts by skipping whitespace, then reads as
    // many characters as match the float grammar.
    let f: f64 = scanf_double(&input).unwrap_or(0.0);

    driver(f);
}

fn driver(f: f64) {
    // raw_double_t u = {.f = f}; printf("%llx %a %.4f\n", u.x, f, f);
    let bits = f.to_bits();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let hex = format_llx(bits);
    let a = format_a(f);
    let dec = format_f4(f);
    writeln!(out, "{} {} {}", hex, a, dec).ok();
}

// ---------- scanf-like double parser ----------

fn scanf_double(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip whitespace (per scanf default)
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let start = i;
    // optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // Try infinity / nan (case-insensitive).
    if try_match_ci(bytes, i, "infinity") {
        let end = i + "infinity".len();
        return parse_float_token(&s[start..end]);
    }
    if try_match_ci(bytes, i, "inf") {
        let end = i + "inf".len();
        return parse_float_token(&s[start..end]);
    }
    if try_match_ci(bytes, i, "nan") {
        // scanf accepts nan(...) too; we ignore the optional payload.
        let end = i + "nan".len();
        return parse_float_token(&s[start..end]);
    }

    // Hex float: 0x or 0X
    if i + 1 < bytes.len()
        && bytes[i] == b'0'
        && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X')
    {
        let mut j = i + 2;
        while j < bytes.len() && (bytes[j] as char).is_ascii_hexdigit() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'.' {
            j += 1;
            while j < bytes.len() && (bytes[j] as char).is_ascii_hexdigit() {
                j += 1;
            }
        }
        if j < bytes.len() && (bytes[j] == b'p' || bytes[j] == b'P') {
            j += 1;
            if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
                j += 1;
            }
            while j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
                j += 1;
            }
        }
        return parse_float_token(&s[start..j]);
    }

    // Decimal float
    let mut j = i;
    while j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
        j += 1;
    }
    if j < bytes.len() && bytes[j] == b'.' {
        j += 1;
        while j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
            j += 1;
        }
    }
    if j > i {
        if j < bytes.len() && (bytes[j] == b'e' || bytes[j] == b'E') {
            let mut k = j + 1;
            if k < bytes.len() && (bytes[k] == b'+' || bytes[k] == b'-') {
                k += 1;
            }
            let exp_start = k;
            while k < bytes.len() && (bytes[k] as char).is_ascii_digit() {
                k += 1;
            }
            if k > exp_start {
                j = k;
            }
        }
    }
    if j == i {
        return None;
    }
    parse_float_token(&s[start..j])
}

fn try_match_ci(bytes: &[u8], pos: usize, kw: &str) -> bool {
    let kw_bytes = kw.as_bytes();
    if pos + kw_bytes.len() > bytes.len() {
        return false;
    }
    for (k, &kb) in kw_bytes.iter().enumerate() {
        if bytes[pos + k].to_ascii_lowercase() != kb.to_ascii_lowercase() {
            return false;
        }
    }
    true
}

fn parse_float_token(tok: &str) -> Option<f64> {
    let trimmed = tok.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Detect hex float and parse manually since Rust's str::parse doesn't
    // accept the C99 hexadecimal float form.
    let (sign_str, rest) = if let Some(stripped) = trimmed.strip_prefix('+') {
        (1.0_f64, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix('-') {
        (-1.0_f64, stripped)
    } else {
        (1.0_f64, trimmed)
    };

    if rest.starts_with("0x") || rest.starts_with("0X") {
        return parse_hex_float(&rest[2..]).map(|v| v * sign_str);
    }

    trimmed.parse::<f64>().ok()
}

fn parse_hex_float(body: &str) -> Option<f64> {
    // body is the part after 0x: optional hex digits, optional .hex digits,
    // optional p[+/-]decimal exponent.
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut int_digits: Vec<u8> = Vec::new();
    while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() {
        int_digits.push(bytes[i]);
        i += 1;
    }
    let mut frac_digits: Vec<u8> = Vec::new();
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() {
            frac_digits.push(bytes[i]);
            i += 1;
        }
    }
    if int_digits.is_empty() && frac_digits.is_empty() {
        return None;
    }
    let mut exp: i64 = 0;
    if i < bytes.len() && (bytes[i] == b'p' || bytes[i] == b'P') {
        i += 1;
        let mut neg = false;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            neg = bytes[i] == b'-';
            i += 1;
        }
        let mut have = false;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            exp = exp.saturating_mul(10).saturating_add((bytes[i] - b'0') as i64);
            i += 1;
            have = true;
        }
        if !have {
            return None;
        }
        if neg {
            exp = -exp;
        }
    }
    if i != bytes.len() {
        return None;
    }
    // Build value: sum of int hex digits as integer, plus frac digits scaled.
    // We do this in f64 directly.
    let mut value: f64 = 0.0;
    for d in &int_digits {
        let v = hex_val(*d);
        value = value * 16.0 + v as f64;
    }
    let mut frac_scale: f64 = 1.0 / 16.0;
    for d in &frac_digits {
        let v = hex_val(*d);
        value += v as f64 * frac_scale;
        frac_scale /= 16.0;
    }
    // Multiply by 2^exp
    Some(value * pow2(exp))
}

fn hex_val(b: u8) -> u32 {
    match b {
        b'0'..=b'9' => (b - b'0') as u32,
        b'a'..=b'f' => (b - b'a' + 10) as u32,
        b'A'..=b'F' => (b - b'A' + 10) as u32,
        _ => 0,
    }
}

fn pow2(mut e: i64) -> f64 {
    // Compute 2^e as f64, handling extreme exponents in steps so we don't
    // lose precision through subnormal flushes.
    let mut result = 1.0_f64;
    while e > 1023 {
        result *= f64::from_bits(0x7FE0000000000000); // 2^1023
        e -= 1023;
    }
    while e < -1022 {
        result *= f64::from_bits(0x0010000000000000); // 2^-1022
        e += 1022;
    }
    let bits: u64 = ((e + 1023) as u64) << 52;
    result * f64::from_bits(bits)
}

// ---------- printf format helpers ----------

fn format_llx(x: u64) -> String {
    // C's "%llx" prints with no leading zeros and lowercase, no "0x" prefix.
    // For 0 it prints "0".
    format!("{:x}", x)
}

fn format_f4(f: f64) -> String {
    // C printf %.4f: "inf"/"-inf" for infinities, "nan"/"-nan" for NaN
    // (glibc prints sign for NaN as well).
    if f.is_nan() {
        // Preserve sign bit of NaN, matching glibc.
        if (f.to_bits() >> 63) & 1 == 1 {
            return "-nan".to_string();
        } else {
            return "nan".to_string();
        }
    }
    if f.is_infinite() {
        return if f.is_sign_negative() { "-inf".to_string() } else { "inf".to_string() };
    }
    // Rust's {:.4} matches C's %.4f for finite doubles, including -0.0.
    format!("{:.4}", f)
}

fn format_a(f: f64) -> String {
    // C printf %a: hex float.
    // Specials:
    //   NaN  -> "nan" or "-nan" (glibc prints sign)
    //   +inf -> "inf"
    //   -inf -> "-inf"
    if f.is_nan() {
        if (f.to_bits() >> 63) & 1 == 1 {
            return "-nan".to_string();
        } else {
            return "nan".to_string();
        }
    }
    if f.is_infinite() {
        return if f.is_sign_negative() { "-inf".to_string() } else { "inf".to_string() };
    }

    let bits = f.to_bits();
    let sign = (bits >> 63) & 1;
    let exp_bits = ((bits >> 52) & 0x7FF) as i32;
    let mantissa = bits & 0x000F_FFFF_FFFF_FFFF;

    let mut out = String::new();
    if sign == 1 {
        out.push('-');
    }

    if exp_bits == 0 && mantissa == 0 {
        // True zero.
        out.push_str("0x0p+0");
        return out;
    }

    let (leading, exponent) = if exp_bits == 0 {
        // Subnormal.
        (0u64, -1022i32)
    } else {
        (1u64, exp_bits - 1023)
    };

    out.push_str("0x");
    out.push(hex_digit(leading as u8));
    if mantissa != 0 {
        // Format mantissa as 13 hex digits, then trim trailing zeros.
        let mant_str = format!("{:013x}", mantissa);
        let trimmed = mant_str.trim_end_matches('0');
        if !trimmed.is_empty() {
            out.push('.');
            out.push_str(trimmed);
        }
    }
    out.push('p');
    if exponent >= 0 {
        out.push('+');
    } else {
        out.push('-');
    }
    out.push_str(&exponent.unsigned_abs().to_string());
    out
}

fn hex_digit(n: u8) -> char {
    if n < 10 {
        (b'0' + n) as char
    } else {
        (b'a' + (n - 10)) as char
    }
}
