//! Small helpers that reproduce the exact behaviour of the C standard library
//! functions used by the original program (`atof` and `printf("%f")`).

/// `isspace()` for the "C" locale, as used by `strtod` to skip leading blanks.
#[inline]
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

#[inline]
fn eq_ignore_case_prefix(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len()
        && s[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

#[inline]
fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a') as u32 + 10),
        b'A'..=b'F' => Some((c - b'A') as u32 + 10),
        _ => None,
    }
}

/// `x * 2^e`, i.e. C's `ldexp`/`scalbn`, without overflowing an intermediate
/// power of two.
fn ldexp(mut x: f64, mut e: i32) -> f64 {
    while e > 512 {
        x *= f64::from_bits(0x5FF0_0000_0000_0000); // 2^512
        e -= 512;
        if x == 0.0 || !x.is_finite() {
            return x;
        }
    }
    while e < -512 {
        x *= f64::from_bits(0x2000_0000_0000_0000); // 2^-511
        e += 511;
        if x == 0.0 || !x.is_finite() {
            return x;
        }
    }
    x * f64::from_bits(((e + 1023) as u64) << 52)
}

/// Parses the mantissa/exponent part of a C99 hexadecimal float literal
/// (everything after the leading `0x`). Returns `None` when there is not a
/// single hexadecimal digit, in which case `strtod` only consumes the leading
/// `0`.
fn parse_hex_float(s: &[u8]) -> Option<f64> {
    let mut mant: u128 = 0;
    let mut exp: i64 = 0;
    let mut any = false;
    let mut i = 0usize;

    // integral hex digits
    while i < s.len() {
        match hex_val(s[i]) {
            Some(d) => {
                any = true;
                if mant < (1u128 << 100) {
                    mant = mant * 16 + d as u128;
                } else {
                    exp += 4;
                }
                i += 1;
            }
            None => break,
        }
    }
    // fractional hex digits
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() {
            match hex_val(s[i]) {
                Some(d) => {
                    any = true;
                    if mant < (1u128 << 100) {
                        mant = mant * 16 + d as u128;
                        exp -= 4;
                    }
                    i += 1;
                }
                None => break,
            }
        }
    }
    if !any {
        return None;
    }
    // binary exponent
    if i < s.len() && (s[i] | 0x20) == b'p' {
        let mut k = i + 1;
        let mut sign: i64 = 1;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            if s[k] == b'-' {
                sign = -1;
            }
            k += 1;
        }
        let mut ndig = 0usize;
        let mut val: i64 = 0;
        while k < s.len() && s[k].is_ascii_digit() {
            if val < 1_000_000 {
                val = val * 10 + (s[k] - b'0') as i64;
            }
            k += 1;
            ndig += 1;
        }
        if ndig > 0 {
            exp += sign * val;
        }
    }

    if mant == 0 {
        return Some(0.0);
    }
    let exp = exp.clamp(-100_000, 100_000) as i32;
    Some(ldexp(mant as f64, exp))
}

/// `double atof(const char *nptr)` == `strtod(nptr, NULL)`.
///
/// Operates on raw bytes so that non-UTF-8 command line arguments behave like
/// they do in C.
pub fn atof(s: &[u8]) -> f64 {
    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }
    let sign_start = i;
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    // "inf" / "infinity"
    if eq_ignore_case_prefix(&s[i..], b"inf") {
        return if neg {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // "nan" / "nan(n-char-sequence)"
    if eq_ignore_case_prefix(&s[i..], b"nan") {
        return if neg { -f64::NAN } else { f64::NAN };
    }

    // hexadecimal floating literal
    if i + 1 < s.len() && s[i] == b'0' && (s[i + 1] | 0x20) == b'x' {
        let v = parse_hex_float(&s[i + 2..]).unwrap_or(0.0);
        return if neg { -v } else { v };
    }

    // decimal floating literal: find the longest valid prefix
    let mut j = i;
    let mut digits = 0usize;
    while j < s.len() && s[j].is_ascii_digit() {
        j += 1;
        digits += 1;
    }
    if j < s.len() && s[j] == b'.' {
        j += 1;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        // no conversion performed
        return 0.0;
    }
    let mut end = j;
    if j < s.len() && (s[j] | 0x20) == b'e' {
        let mut k = j + 1;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            k += 1;
        }
        let mut edigits = 0usize;
        while k < s.len() && s[k].is_ascii_digit() {
            k += 1;
            edigits += 1;
        }
        if edigits > 0 {
            end = k;
        }
    }

    // The subject sequence consists solely of ASCII characters.
    let text = std::str::from_utf8(&s[sign_start..end]).unwrap_or("0");
    text.parse::<f64>().unwrap_or(0.0)
}

/// Formats a `double` exactly like glibc's `printf("%f", v)`.
pub fn printf_f(v: f64) -> String {
    if v.is_nan() {
        // glibc honours the sign bit of NaN
        if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        }
    } else if v.is_infinite() {
        if v.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        }
    } else {
        // Rust's fixed-precision formatting is exact and rounds ties to even,
        // matching glibc in the default rounding mode.
        format!("{:.6}", v)
    }
}
