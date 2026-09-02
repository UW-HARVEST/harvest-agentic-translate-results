//! A faithful re-implementation of C's `atof`, i.e. `strtod(s, NULL)`.
//!
//! Accepts an optional run of whitespace, an optional sign, then one of:
//! a decimal significand with optional exponent, a hexadecimal significand
//! (`0x...`) with optional binary exponent, `inf`/`infinity`, or
//! `nan`/`nan(chars)` — all case-insensitive. If no conversion can be
//! performed the result is `0.0`, exactly as C specifies.

/// `atof(buffer)`. The slice is treated as a C string: it ends at the first
/// NUL byte, if any.
pub fn atof(bytes: &[u8]) -> f64 {
    let s = match bytes.iter().position(|&b| b == 0) {
        Some(n) => &bytes[..n],
        None => bytes,
    };
    strtod(s)
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn eq_ignore_case(bytes: &[u8], i: usize, word: &[u8]) -> bool {
    if bytes.len() - i < word.len() {
        return false;
    }
    bytes[i..i + word.len()]
        .iter()
        .zip(word)
        .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

fn strtod(bytes: &[u8]) -> f64 {
    let mut i = 0usize;

    // Leading whitespace.
    while i < bytes.len() && is_c_space(bytes[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }

    let magnitude = if eq_ignore_case(bytes, i, b"inf") {
        f64::INFINITY
    } else if eq_ignore_case(bytes, i, b"nan") {
        f64::NAN
    } else if eq_ignore_case(bytes, i, b"0x") && has_hex_digit(bytes, i + 2) {
        parse_hex(bytes, i + 2)
    } else {
        match parse_decimal(bytes, i) {
            Some(value) => value,
            // No valid subject sequence: C returns +0.0 regardless of sign.
            None => return 0.0,
        }
    };

    if negative {
        -magnitude
    } else {
        magnitude
    }
}

fn has_hex_digit(bytes: &[u8], mut i: usize) -> bool {
    // A hex significand needs at least one hex digit, possibly after the
    // radix point.
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
    }
    i < bytes.len() && bytes[i].is_ascii_hexdigit()
}

/// Parses `digits[.digits][(e|E)[sign]digits]`, returning `None` when there is
/// no digit at all (no conversion performed).
fn parse_decimal(bytes: &[u8], start: usize) -> Option<f64> {
    let mut i = start;
    let mut int_digits: Vec<u8> = Vec::new();
    let mut frac_digits: Vec<u8> = Vec::new();

    while i < bytes.len() && bytes[i].is_ascii_digit() {
        int_digits.push(bytes[i]);
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        let after_point = i + 1;
        let mut j = after_point;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            frac_digits.push(bytes[j]);
            j += 1;
        }
        // The point is only part of the subject sequence if a digit was seen
        // somewhere.
        if !int_digits.is_empty() || !frac_digits.is_empty() {
            i = j;
        }
    }
    if int_digits.is_empty() && frac_digits.is_empty() {
        return None;
    }

    // Optional decimal exponent; consumed only if well formed.
    let mut exponent: Vec<u8> = Vec::new();
    if i < bytes.len() && (bytes[i] | 0x20) == b'e' {
        let mut j = i + 1;
        let mut sign = b'+';
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            sign = bytes[j];
            j += 1;
        }
        let digits_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > digits_start {
            exponent.push(sign);
            exponent.extend_from_slice(&bytes[digits_start..j]);
        }
    }

    // Hand the normalised form to Rust's correctly-rounded decimal parser.
    let mut normalised = String::new();
    if int_digits.is_empty() {
        normalised.push('0');
    } else {
        normalised.push_str(&String::from_utf8_lossy(&int_digits));
    }
    normalised.push('.');
    if frac_digits.is_empty() {
        normalised.push('0');
    } else {
        normalised.push_str(&String::from_utf8_lossy(&frac_digits));
    }
    if !exponent.is_empty() {
        normalised.push('e');
        normalised.push_str(&String::from_utf8_lossy(&exponent));
    }

    Some(normalised.parse::<f64>().unwrap_or(0.0))
}

/// Parses a hexadecimal significand (the caller has already consumed `0x`
/// and verified that at least one hex digit follows).
fn parse_hex(bytes: &[u8], start: usize) -> f64 {
    let mut i = start;
    // Significand bits, accumulated exactly for as long as they fit; `sticky`
    // records that nonzero bits were dropped past the 128-bit window.
    let mut mantissa: u128 = 0;
    let mut sticky = false;
    let mut exponent: i64 = 0;
    let mut seen_point = false;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'.' {
            if seen_point {
                break;
            }
            seen_point = true;
            i += 1;
            continue;
        }
        let digit = match hex_value(b) {
            Some(d) => d,
            None => break,
        };
        if mantissa >> 124 == 0 {
            mantissa = (mantissa << 4) | u128::from(digit);
        } else {
            // Out of room: keep the exponent aligned and remember the loss.
            exponent += 4;
            if digit != 0 {
                sticky = true;
            }
        }
        if seen_point {
            exponent -= 4;
        }
        i += 1;
    }

    // Optional binary exponent; consumed only if well formed.
    if i < bytes.len() && (bytes[i] | 0x20) == b'p' {
        let mut j = i + 1;
        let mut negative = false;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            negative = bytes[j] == b'-';
            j += 1;
        }
        let digits_start = j;
        let mut value: i64 = 0;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            value = value.saturating_mul(10).saturating_add(i64::from(bytes[j] - b'0'));
            j += 1;
        }
        if j > digits_start {
            exponent = if negative {
                exponent.saturating_sub(value)
            } else {
                exponent.saturating_add(value)
            };
        }
    }

    if mantissa == 0 {
        return 0.0;
    }
    // Clamp far beyond the representable range; the sign is applied later.
    let exponent = exponent.clamp(-100_000, 100_000) as i32;
    compose(mantissa, exponent, sticky)
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Rounds `mantissa * 2^exponent` (mantissa nonzero, `sticky` marking further
/// dropped nonzero bits) to the nearest `f64`, ties to even.
fn compose(mantissa: u128, exponent: i32, sticky: bool) -> f64 {
    let significant_bits = 128 - mantissa.leading_zeros() as i32;
    // Unbiased exponent of the value: 2^e <= value < 2^(e+1).
    let e = significant_bits - 1 + exponent;

    if e > 1023 {
        return f64::INFINITY;
    }

    // Bits of precision available at this magnitude (53 while normal, fewer
    // once the value falls into the subnormal range).
    let precision = if e < -1022 { e + 1075 } else { 53 };

    if precision <= 0 {
        // Below half of the smallest subnormal, or exactly at it (ties to even
        // gives zero).
        if precision == 0 {
            let exactly_half = mantissa == 1u128 << (significant_bits - 1) && !sticky;
            if exactly_half {
                return 0.0;
            }
            return f64::from_bits(1);
        }
        return 0.0;
    }

    let shift = significant_bits - precision;
    let mut quantised: u128;
    let mut round_up = false;
    if shift <= 0 {
        quantised = mantissa << (-shift) as u32;
    } else {
        quantised = mantissa >> shift;
        let remainder = mantissa & ((1u128 << shift) - 1);
        let half = 1u128 << (shift - 1);
        round_up = remainder > half || (remainder == half && (sticky || quantised & 1 == 1));
    }
    if round_up {
        quantised += 1;
    }

    if e < -1022 {
        // Subnormal encoding: the value is `quantised * 2^-1074`, and a carry
        // out of the significand rolls smoothly into the normal range.
        return f64::from_bits(quantised as u64);
    }

    // Normal encoding; a carry bumps the exponent.
    let mut e = e;
    if quantised >> 53 != 0 {
        quantised >>= 1;
        e += 1;
        if e > 1023 {
            return f64::INFINITY;
        }
    }
    let biased = (e + 1023) as u64;
    f64::from_bits((biased << 52) | (quantised as u64 & ((1u64 << 52) - 1)))
}
