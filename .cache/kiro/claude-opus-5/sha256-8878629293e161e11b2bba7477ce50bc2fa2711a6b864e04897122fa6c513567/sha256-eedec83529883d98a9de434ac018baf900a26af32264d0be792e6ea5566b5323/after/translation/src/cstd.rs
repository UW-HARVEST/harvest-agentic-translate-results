//! Faithful re-implementation of the pieces of the C standard library that the
//! original program relies on (`atof` and the `%f` conversion of `printf`).
//!
//! These are hand written rather than delegated to Rust's own parsing /
//! formatting because the C versions never fail: `atof` on unparsable text
//! quietly yields `0.0`, and `printf("%f")` renders infinities and NaNs as
//! `inf` / `nan` instead of Rust's `inf` / `NaN`.

/// The characters `isspace()` accepts in the C locale.
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a') as u32 + 10),
        b'A'..=b'F' => Some((c - b'A') as u32 + 10),
        _ => None,
    }
}

/// Case insensitive check that `s[at..]` starts with the (lowercase) `needle`.
fn starts_with_ci(s: &[u8], at: usize, needle: &[u8]) -> bool {
    if s.len() < at + needle.len() {
        return false;
    }
    s[at..at + needle.len()]
        .iter()
        .zip(needle)
        .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

/// `atof()` — i.e. `strtod()` without the end pointer, and without reporting
/// errors. Anything that is not a valid numeric prefix converts to `0.0`.
pub fn atof(s: &[u8]) -> f64 {
    let mut i = 0usize;

    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // inf / infinity
    if starts_with_ci(s, i, b"inf") {
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // nan / nan(chars)
    if starts_with_ci(s, i, b"nan") {
        return if negative { -f64::NAN } else { f64::NAN };
    }

    // Hexadecimal floating point: 0x1.8p3
    if i + 1 < s.len() && s[i] == b'0' && (s[i + 1] | 0x20) == b'x' {
        if let Some(v) = parse_hex_float(&s[i + 2..]) {
            return if negative { -v } else { v };
        }
        // "0x" with no hex digits following: only the leading `0` is consumed.
        return if negative { -0.0 } else { 0.0 };
    }

    parse_decimal_float(s, i, negative)
}

/// Extract the longest valid decimal floating point prefix and convert it.
fn parse_decimal_float(s: &[u8], mut i: usize, negative: bool) -> f64 {
    let start = i;
    let mut digits = 0usize;

    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        // No conversion could be performed, so `strtod` returns +0.0 and the
        // leading sign is *not* applied: "-." and "--3" both yield 0.0, not -0.0.
        return 0.0;
    }
    let mantissa_end = i;

    // An exponent only counts when at least one digit follows it.
    let mut number_end = mantissa_end;
    if i < s.len() && (s[i] | 0x20) == b'e' {
        let mut j = i + 1;
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            j += 1;
        }
        if j < s.len() && s[j].is_ascii_digit() {
            while j < s.len() && s[j].is_ascii_digit() {
                j += 1;
            }
            number_end = j;
        }
    }

    // Safe: the slice contains only ASCII digits, '.', 'e'/'E' and a sign.
    let text = core::str::from_utf8(&s[start..number_end]).unwrap_or("0");
    // Rust's parser is correctly rounded, matching glibc's strtod. Parse the
    // magnitude and apply the sign afterwards so that "-0" stays negative zero.
    let magnitude: f64 = text.parse().unwrap_or(0.0);
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

/// Parse the part of a C99 hex float that follows `0x`. Returns `None` when no
/// hex digit is present (in which case `strtod` only consumes the `0`).
fn parse_hex_float(s: &[u8]) -> Option<f64> {
    let mut mantissa: u128 = 0;
    let mut exponent: i64 = 0;
    let mut sticky = false;
    let mut saw_digit = false;
    let mut saw_point = false;
    let mut i = 0usize;

    while i < s.len() {
        if s[i] == b'.' {
            if saw_point {
                break;
            }
            saw_point = true;
            i += 1;
            continue;
        }
        let d = match hex_val(s[i]) {
            Some(d) => d,
            None => break,
        };
        saw_digit = true;
        if mantissa >> 124 == 0 {
            mantissa = (mantissa << 4) | d as u128;
            if saw_point {
                exponent -= 4;
            }
        } else {
            // No room left; remember that non-zero bits were dropped.
            if d != 0 {
                sticky = true;
            }
            if !saw_point {
                exponent += 4;
            }
        }
        i += 1;
    }

    if !saw_digit {
        return None;
    }

    // Optional binary exponent.
    if i < s.len() && (s[i] | 0x20) == b'p' {
        let mut j = i + 1;
        let mut exp_negative = false;
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            exp_negative = s[j] == b'-';
            j += 1;
        }
        if j < s.len() && s[j].is_ascii_digit() {
            let mut e: i64 = 0;
            while j < s.len() && s[j].is_ascii_digit() {
                e = (e * 10 + (s[j] - b'0') as i64).min(1_000_000);
                j += 1;
            }
            exponent += if exp_negative { -e } else { e };
        }
    }

    Some(scale_to_f64(mantissa, sticky, exponent))
}

/// Round `mantissa * 2^exponent` (with `sticky` recording discarded low bits)
/// to the nearest `f64`, ties to even.
fn scale_to_f64(mantissa: u128, mut sticky: bool, exponent: i64) -> f64 {
    if mantissa == 0 {
        return 0.0;
    }

    let mut m = mantissa;
    let mut e = exponent;

    // Normalize to exactly 54 significant bits (53 kept + 1 guard bit).
    let bits = 128 - m.leading_zeros() as i64;
    let drop = bits - 54;
    if drop > 0 {
        if m & ((1u128 << drop) - 1) != 0 {
            sticky = true;
        }
        m >>= drop;
    } else if drop < 0 {
        m <<= -drop;
    }
    e += drop;

    // Exponent of the unit in the last place of the result.
    let ulp_exp = if e + 1 < -1074 { -1074 } else { e + 1 };
    let shift = ulp_exp - e;
    if shift >= 129 {
        return 0.0;
    }
    let shift = shift as u32;

    let quotient = m >> shift;
    let round_bit = (m >> (shift - 1)) & 1;
    let rest = m & ((1u128 << (shift - 1)) - 1);

    let mut result = quotient;
    if round_bit == 1 && (rest != 0 || sticky || (quotient & 1) == 1) {
        result += 1;
    }

    if ulp_exp == -1074 {
        // Subnormal (or the smallest normal, which the encoding produces for
        // free once the significand carries into bit 52).
        return f64::from_bits(result as u64);
    }

    let mut ulp_exp = ulp_exp;
    if result == 1u128 << 53 {
        result >>= 1;
        ulp_exp += 1;
    }

    let biased = ulp_exp + 52 + 1023;
    if biased >= 2047 {
        return f64::INFINITY;
    }
    f64::from_bits(((biased as u64) << 52) | ((result as u64) & ((1u64 << 52) - 1)))
}

/// Render a `double` the way glibc's `printf("%f")` does.
pub fn format_f(value: f64) -> String {
    if value.is_nan() {
        // glibc honours the sign bit of a NaN.
        return if value.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        return if value < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    // Rust's fixed-precision formatting is exact and rounds ties to even, as
    // does glibc under the default rounding mode.
    format!("{:.6}", value)
}
