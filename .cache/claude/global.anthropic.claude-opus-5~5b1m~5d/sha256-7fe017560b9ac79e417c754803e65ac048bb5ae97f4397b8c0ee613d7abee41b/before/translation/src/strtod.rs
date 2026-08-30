//! A reimplementation of glibc's `strtod` for the C locale, including its
//! `endptr` and `ERANGE` behaviour.
//!
//! glibc sets `ERANGE` when the conversion overflows (the result is an
//! infinity) and when it underflows.  Underflow is reported when the
//! infinitely precise value, rounded to 53 significant bits with an unbounded
//! exponent, stays below `2^-1022` *and* the conversion to `double` was
//! inexact.  So exactly representable subnormals do not raise `ERANGE`, and
//! neither do tiny values whose plain 53 bit rounding already reaches the
//! smallest normal number.

use crate::bignum::Big;
use std::cmp::Ordering;

pub struct Conversion {
    /// Converted value (0.0 when no conversion could be performed).
    pub value: f64,
    /// Number of bytes consumed; 0 means "no conversion", i.e. `endptr == nptr`.
    pub consumed: usize,
    /// Whether `errno` would have been set to `ERANGE`.
    pub erange: bool,
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_hex_digit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn hex_val(c: u8) -> u32 {
    match c {
        b'0'..=b'9' => (c - b'0') as u32,
        b'a'..=b'f' => (c - b'a') as u32 + 10,
        _ => (c - b'A') as u32 + 10,
    }
}

fn eq_ignore_case_prefix(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len()
        && s[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

fn apply_sign(v: f64, neg: bool) -> f64 {
    if neg {
        -v
    } else {
        v
    }
}

/// Parse a run of decimal digits into an exponent, saturating at a magnitude
/// far beyond anything that can influence the result.
fn parse_exp_digits(s: &[u8], mut i: usize, neg: bool) -> (i64, usize) {
    let mut val: i64 = 0;
    while i < s.len() && is_digit(s[i]) {
        if val < 1_000_000_000_000 {
            val = val * 10 + (s[i] - b'0') as i64;
        }
        i += 1;
    }
    (if neg { -val } else { val }, i)
}

pub fn strtod(s: &[u8]) -> Conversion {
    let no_conversion = Conversion {
        value: 0.0,
        consumed: 0,
        erange: false,
    };

    let mut i = 0usize;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }

    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    // Hexadecimal form: 0x / 0X
    if i + 1 < s.len() && s[i] == b'0' && (s[i + 1] | 0x20) == b'x' {
        let zero_pos = i; // the '0' itself
        let mut j = i + 2;
        let mut digits: Vec<u8> = Vec::new();
        let mut frac_len: usize = 0;
        while j < s.len() && is_hex_digit(s[j]) {
            digits.push(s[j]);
            j += 1;
        }
        if j < s.len() && s[j] == b'.' {
            let dot = j;
            j += 1;
            while j < s.len() && is_hex_digit(s[j]) {
                digits.push(s[j]);
                frac_len += 1;
                j += 1;
            }
            if digits.is_empty() {
                // "0x.": no digits at all, roll back to just before the dot.
                j = dot;
            }
        }
        if digits.is_empty() {
            // No hex digits: glibc converts just the leading "0".
            return Conversion {
                value: apply_sign(0.0, neg),
                consumed: zero_pos + 1,
                erange: false,
            };
        }
        // Optional binary exponent.
        let mut pexp: i64 = 0;
        if j < s.len() && (s[j] | 0x20) == b'p' {
            let mut k = j + 1;
            let mut eneg = false;
            if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
                eneg = s[k] == b'-';
                k += 1;
            }
            if k < s.len() && is_digit(s[k]) {
                let (e, end) = parse_exp_digits(s, k, eneg);
                pexp = e;
                j = end;
            }
        }
        let (value, erange) = hex_value(&digits, frac_len, pexp);
        return Conversion {
            value: apply_sign(value, neg),
            consumed: j,
            erange,
        };
    }

    // Infinity / NaN
    let rest = &s[i..];
    if eq_ignore_case_prefix(rest, b"infinity") {
        return Conversion {
            value: apply_sign(f64::INFINITY, neg),
            consumed: i + 8,
            erange: false,
        };
    }
    if eq_ignore_case_prefix(rest, b"inf") {
        return Conversion {
            value: apply_sign(f64::INFINITY, neg),
            consumed: i + 3,
            erange: false,
        };
    }
    if eq_ignore_case_prefix(rest, b"nan") {
        let mut j = i + 3;
        if j < s.len() && s[j] == b'(' {
            let mut k = j + 1;
            while k < s.len() && (s[k].is_ascii_alphanumeric() || s[k] == b'_') {
                k += 1;
            }
            if k < s.len() && s[k] == b')' {
                j = k + 1;
            }
        }
        return Conversion {
            value: apply_sign(f64::NAN, neg),
            consumed: j,
            erange: false,
        };
    }

    // Decimal form
    let mut digits: Vec<u8> = Vec::new();
    let mut frac_len: usize = 0;
    let mut j = i;
    while j < s.len() && is_digit(s[j]) {
        digits.push(s[j]);
        j += 1;
    }
    if j < s.len() && s[j] == b'.' {
        let dot = j;
        j += 1;
        while j < s.len() && is_digit(s[j]) {
            digits.push(s[j]);
            frac_len += 1;
            j += 1;
        }
        if digits.is_empty() {
            // ".": no digits at all, roll back to just before the dot.
            j = dot;
        }
    }
    if digits.is_empty() {
        return no_conversion;
    }
    let mut exp10: i64 = 0;
    if j < s.len() && (s[j] | 0x20) == b'e' {
        let mut k = j + 1;
        let mut eneg = false;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            eneg = s[k] == b'-';
            k += 1;
        }
        if k < s.len() && is_digit(s[k]) {
            let (e, end) = parse_exp_digits(s, k, eneg);
            exp10 = e;
            j = end;
        }
    }

    let (value, erange) = decimal_value(&digits, frac_len, exp10);
    Conversion {
        value: apply_sign(value, neg),
        consumed: j,
        erange,
    }
}

/// Value of a decimal mantissa (`digits`, of which the last `frac_len` are
/// fractional) scaled by 10^`exp10`, together with the `ERANGE` flag.
fn decimal_value(digits: &[u8], frac_len: usize, exp10: i64) -> (f64, bool) {
    // Significant digits: strip leading and trailing zeros, folding the removed
    // trailing zeros into the decimal exponent.
    let first = digits.iter().position(|&c| c != b'0');
    let first = match first {
        Some(p) => p,
        None => return (0.0, false), // value is exactly zero
    };
    let last = digits.iter().rposition(|&c| c != b'0').unwrap();
    let sig = &digits[first..=last];
    let trailing_zeros = (digits.len() - 1 - last) as i64;
    let e10 = exp10
        .saturating_sub(frac_len as i64)
        .saturating_add(trailing_zeros);

    // Correctly rounded conversion, with the exponent clamped to a value that
    // is guaranteed to produce the same infinity / zero result.
    let lo = -(sig.len() as i64 + 400);
    let hi = 400i64;
    let clamped = if e10 < lo {
        lo
    } else if e10 > hi {
        hi
    } else {
        e10
    };
    let mut text = String::with_capacity(sig.len() + 24);
    text.push_str(std::str::from_utf8(sig).unwrap());
    text.push('e');
    text.push_str(&clamped.to_string());
    let value: f64 = text.parse().unwrap_or(f64::INFINITY);

    if value.is_infinite() {
        return (value, true); // overflow
    }
    // Underflow: glibc raises ERANGE when the exact value is below
    // `2^-1022 - 2^-1076` (i.e. rounding it at plain 53 bit precision does not
    // reach the smallest normal number) and it is not exactly representable on
    // the subnormal grid.
    if value.abs() < f64::MIN_POSITIVE {
        // The exact value is necessarily below the threshold here.
        if !is_exact_subnormal(sig, e10) {
            return (value, true);
        }
    } else if value.abs() == f64::MIN_POSITIVE && below_underflow_threshold(sig, e10) {
        return (value, true);
    }
    (value, false)
}

/// Is `sig * 10^e10` (a positive value) strictly smaller than
/// `2^-1022 - 2^-1076` = `(2^54 - 1) * 2^-1076`?
fn below_underflow_threshold(sig: &[u8], e10: i64) -> bool {
    if e10 >= 0 {
        return false;
    }
    let f = -(e10 as i128);
    let len = sig.len() as i128;
    // value < 10^(len - f) and value >= 10^(len - 1 - f)
    if len - f <= -309 {
        return true;
    }
    if len - 1 - f >= -307 {
        return false;
    }
    // Borderline: compare sig * 2^1076 against (2^54 - 1) * 10^f exactly.
    let mut a = Big::from_digits(sig);
    a.shl_bits(1076);
    let mut b = Big::pow10(f as usize);
    b.mul_add_small(134_217_727, 0); // 2^27 - 1
    b.mul_add_small(134_217_729, 0); // 2^27 + 1
    a.cmp_big(&b) == Ordering::Less
}

/// Is `sig * 10^e10` an exact integer multiple of 2^-1074 (i.e. exactly
/// representable as a subnormal double)?
fn is_exact_subnormal(sig: &[u8], e10: i64) -> bool {
    if e10 >= 0 {
        return true;
    }
    let f = -e10;
    // sig must be divisible by 5^f, which requires sig >= 5^f.
    if (sig.len() as f64) < 0.69897 * (f as f64) {
        return false;
    }
    let f = f as usize;
    let mut n = Big::from_digits(sig);
    let mut left = f;
    while left > 0 {
        let k = left.min(13);
        let mut d: u32 = 1;
        for _ in 0..k {
            d *= 5;
        }
        if n.divmod_small(d) != 0 {
            return false;
        }
        left -= k;
    }
    if f > 1074 {
        let mut bits = f - 1074;
        while bits > 0 {
            let k = bits.min(31);
            if n.divmod_small(1u32 << k) != 0 {
                return false;
            }
            bits -= k;
        }
    }
    true
}

/// Value of a hexadecimal mantissa (`digits`, of which the last `frac_len` are
/// fractional) scaled by 2^`pexp`, together with the `ERANGE` flag.
fn hex_value(digits: &[u8], frac_len: usize, pexp: i64) -> (f64, bool) {
    let mut sig: u128 = 0;
    let mut sticky = false;
    let mut dropped_bits: i128 = 0;
    for &c in digits {
        let v = hex_val(c) as u128;
        if sig == 0 && v == 0 {
            continue;
        }
        if sig >> 120 != 0 {
            sticky |= v != 0;
            dropped_bits += 4;
        } else {
            sig = (sig << 4) | v;
        }
    }
    if sig == 0 {
        return (0.0, false);
    }
    let exp2 = (pexp as i128) - 4 * (frac_len as i128) + dropped_bits;
    round_to_double(sig, sticky, exp2)
}

/// Round `sig * 2^exp2` (plus a sticky remainder below the last bit of `sig`)
/// to `keep` significant bits, to nearest with ties to even.  Returns the
/// rounded significand, its binary exponent and whether anything was lost.
fn round_keep(sig: u128, sticky: bool, exp2: i128, keep: i128) -> (u128, i128, bool) {
    let bl = (128 - sig.leading_zeros()) as i128;
    let shift = bl - keep;
    let (m, inexact) = if shift <= 0 {
        (sig << ((-shift) as u32), sticky)
    } else if shift >= 129 {
        (0u128, true)
    } else if shift == 128 {
        let half = 1u128 << 127;
        let up = sig > half || (sig == half && sticky);
        (if up { 1u128 } else { 0u128 }, true)
    } else {
        let sh = shift as u32;
        let dropped = sig & ((1u128 << sh) - 1);
        let half = 1u128 << (sh - 1);
        let q = sig >> sh;
        let up = dropped > half || (dropped == half && (sticky || (q & 1) == 1));
        (if up { q + 1 } else { q }, sticky || dropped != 0)
    };
    (m, exp2 + shift, inexact)
}

/// Round `sig * 2^exp2` (plus a sticky remainder below the last bit of `sig`)
/// to the nearest double, reporting whether `ERANGE` would be raised.
fn round_to_double(sig: u128, sticky: bool, exp2: i128) -> (f64, bool) {
    let bl = (128 - sig.leading_zeros()) as i128;
    let e_msb = exp2 + bl - 1; // exponent of the most significant bit
    let tiny = e_msb < -1022;

    if e_msb > 1024 {
        return (f64::INFINITY, true);
    }

    // Rounding at full 53 bit precision (as if the exponent range were
    // unbounded).
    let (m53, e53, _) = round_keep(sig, sticky, exp2, 53);

    if tiny {
        // If rounding at full precision already reaches the smallest normal
        // number, glibc does not consider the result tiny and raises nothing.
        let bl53 = (128 - m53.leading_zeros()) as i128;
        if m53 != 0 && e53 + bl53 - 1 >= -1022 {
            return (f64::MIN_POSITIVE, false);
        }
        // Otherwise round on the subnormal grid; ERANGE if that is inexact.
        let keep = e_msb + 1075;
        let (m, e, inexact) = round_keep(sig, sticky, exp2, keep);
        (build(m, e), inexact)
    } else {
        // Overflow check after rounding.
        let bl53 = (128 - m53.leading_zeros()) as i128;
        if e53 + bl53 - 1 > 1023 {
            return (f64::INFINITY, true);
        }
        (build(m53, e53), false)
    }
}

/// Exactly `m * 2^e`, where the product is known to be representable.
fn build(mut m: u128, mut e: i128) -> f64 {
    if m == 0 {
        return 0.0;
    }
    while e < -1074 && m % 2 == 0 {
        m /= 2;
        e += 1;
    }
    if e < -1074 {
        // Unreachable: rounding always lands on a representable value.
        return 0.0;
    }
    (m as f64) * pow2(e as i32)
}

/// Exact 2^e for -1074 <= e <= 1023.
fn pow2(e: i32) -> f64 {
    if e >= -1022 {
        f64::from_bits(((e + 1023) as u64) << 52)
    } else {
        f64::from_bits(1u64 << (e + 1074))
    }
}

