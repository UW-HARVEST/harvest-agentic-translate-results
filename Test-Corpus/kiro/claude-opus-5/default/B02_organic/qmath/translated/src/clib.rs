//! Replication of the C standard library routines that the original program
//! relies on: `atof` (i.e. `strtod`) for input parsing and the `%f`
//! conversion of `printf` for output formatting.
//!
//! These are re-implemented rather than approximated with Rust's own parsing
//! and formatting so that the observable behaviour (including the way invalid
//! input silently becomes `0.0`, and the way non-finite values are spelled)
//! stays byte-identical to the C program built against glibc.

/// Bytes that C's `isspace()` considers whitespace in the "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn hex_val(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a') as u32 + 10),
        b'A'..=b'F' => Some((b - b'A') as u32 + 10),
        _ => None,
    }
}

fn eq_ignore_case(hay: &[u8], needle: &[u8]) -> bool {
    hay.len() >= needle.len()
        && hay[..needle.len()]
            .iter()
            .zip(needle.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

/// `double atof(const char *)` — equivalent to `strtod(s, NULL)`.
///
/// Parses the longest valid prefix; if no conversion can be performed the
/// result is `0.0`, exactly like the C library (`atof` reports no errors).
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

    let rest = &s[i..];

    // Hexadecimal form: 0x<hex digits>[.<hex digits>][p<exp>]
    if rest.len() >= 2 && rest[0] == b'0' && (rest[1] == b'x' || rest[1] == b'X') {
        if let Some(v) = parse_hex(&rest[2..], negative) {
            return v;
        }
        // "0x" with no valid hex mantissa: strtod consumes just the leading
        // "0", yielding zero.
        return if negative { -0.0 } else { 0.0 };
    }

    // Infinity: "inf" or "infinity".
    if eq_ignore_case(rest, b"inf") {
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // NaN: "nan" optionally followed by a parenthesised character sequence.
    if eq_ignore_case(rest, b"nan") {
        return if negative { -f64::NAN } else { f64::NAN };
    }

    parse_decimal(rest, negative)
}

/// Decimal floating point: digits, optional fractional part, optional
/// exponent. Returns `0.0` when nothing convertible is present.
fn parse_decimal(s: &[u8], negative: bool) -> f64 {
    let mut j = 0usize;
    let mut int_digits = 0usize;
    while j < s.len() && s[j].is_ascii_digit() {
        j += 1;
        int_digits += 1;
    }

    let mut frac_digits = 0usize;
    let point = j < s.len() && s[j] == b'.';
    if point {
        j += 1;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
            frac_digits += 1;
        }
    }

    if int_digits == 0 && frac_digits == 0 {
        // No conversion performed: strtod returns positive zero even when a
        // sign was present.
        return 0.0;
    }

    // Mantissa end (before any exponent).
    let mant_end = j;

    // Optional exponent; only consumed when it is well formed.
    let mut end = mant_end;
    if j < s.len() && (s[j] == b'e' || s[j] == b'E') {
        let mut k = j + 1;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            k += 1;
        }
        if k < s.len() && s[k].is_ascii_digit() {
            while k < s.len() && s[k].is_ascii_digit() {
                k += 1;
            }
            end = k;
        }
    }

    // Rust's `f64` parser is correctly rounded, matching glibc's strtod, so it
    // can be reused once the exact token has been isolated.
    let token = std::str::from_utf8(&s[..end]).unwrap_or("0");
    let mut value: f64 = token.parse().unwrap_or(0.0);
    if negative {
        value = -value;
    }
    value
}

/// Hexadecimal floating point after the `0x` prefix. Returns `None` when there
/// is no digit at all (so the caller can fall back to consuming only the `0`).
fn parse_hex(s: &[u8], negative: bool) -> Option<f64> {
    // Collect significant nibbles, keeping 16 of them (64 bits) plus a sticky
    // bit which records whether anything nonzero was discarded. 64 bits is
    // more than the 53 + guard + sticky needed for correct rounding.
    const KEEP: u32 = 16;

    let mut mant: u128 = 0;
    let mut kept: u32 = 0;
    let mut sticky = false;
    let mut seen_digit = false;
    let mut started = false; // leading zeros suppressed
    // Number of nibbles that were consumed but not stored, and the count of
    // nibbles after the radix point (each worth 2^-4).
    let mut dropped: i64 = 0;
    let mut frac_nibbles: i64 = 0;

    let mut i = 0usize;
    let mut seen_point = false;
    while i < s.len() {
        let b = s[i];
        if b == b'.' {
            if seen_point {
                break;
            }
            seen_point = true;
            i += 1;
            continue;
        }
        let v = match hex_val(b) {
            Some(v) => v,
            None => break,
        };
        seen_digit = true;
        if seen_point {
            frac_nibbles += 1;
        }
        if v != 0 {
            started = true;
        }
        if started {
            if kept < KEEP {
                mant = (mant << 4) | v as u128;
                kept += 1;
            } else {
                dropped += 1;
                if v != 0 {
                    sticky = true;
                }
            }
        }
        i += 1;
    }

    if !seen_digit {
        return None;
    }

    // Optional binary exponent.
    let mut pexp: i64 = 0;
    if i < s.len() && (s[i] == b'p' || s[i] == b'P') {
        let mut k = i + 1;
        let mut neg = false;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            neg = s[k] == b'-';
            k += 1;
        }
        if k < s.len() && s[k].is_ascii_digit() {
            let mut e: i64 = 0;
            while k < s.len() && s[k].is_ascii_digit() {
                e = e.saturating_mul(10).saturating_add((s[k] - b'0') as i64);
                if e > 1 << 40 {
                    e = 1 << 40;
                }
                k += 1;
            }
            pexp = if neg { -e } else { e };
        }
    }

    if mant == 0 {
        return Some(if negative { -0.0 } else { 0.0 });
    }

    // value == mant * 2^exp
    let exp = pexp + 4 * dropped - 4 * frac_nibbles;
    Some(compose(mant, exp, sticky, negative))
}

/// Round `mant * 2^exp` (plus an infinitesimal `sticky` remainder) to the
/// nearest `f64`, ties to even, and apply the sign.
fn compose(mut mant: u128, mut exp: i64, sticky: bool, negative: bool) -> f64 {
    let bit_len = |m: u128| -> u32 { 128 - m.leading_zeros() };

    // Folding sticky into the lowest bit is safe: with 61+ significant bits
    // present, bit 0 is always strictly below the rounding position.
    if sticky {
        mant |= 1;
    }

    let l = bit_len(mant) as i64;
    // Bits that must be discarded: enough for 53-bit precision, and enough to
    // keep the unit-in-last-place no finer than 2^-1074 (subnormal grid).
    let mut drop = std::cmp::max(l - 53, -1074 - exp);
    if drop < 0 {
        drop = 0;
    }

    if drop > 0 {
        if drop >= 128 {
            return if negative { -0.0 } else { 0.0 };
        }
        let d = drop as u32;
        let rem = mant & ((1u128 << d) - 1);
        let half = 1u128 << (d - 1);
        let mut q = mant >> d;
        if rem > half || (rem == half && (q & 1) == 1) {
            q += 1;
        }
        mant = q;
        exp += drop;
    }

    if mant == 0 {
        return if negative { -0.0 } else { 0.0 };
    }

    let lq = bit_len(mant) as i64;
    let e = exp + lq - 1; // exponent of the leading bit

    if e > 1023 {
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    let bits: u64 = if e >= -1022 {
        // Normal: shift so the implicit leading bit sits at position 52.
        let shift = 53 - lq;
        let frac = if shift >= 0 {
            mant << shift as u32
        } else {
            mant >> (-shift) as u32
        };
        let frac = (frac as u64) & ((1u64 << 52) - 1);
        (((e + 1023) as u64) << 52) | frac
    } else {
        // Subnormal: the unit in the last place is 2^-1074.
        let shift = exp + 1074;
        let frac = if shift >= 0 {
            mant << shift as u32
        } else {
            mant >> (-shift) as u32
        };
        frac as u64
    };

    let sign = if negative { 1u64 << 63 } else { 0 };
    f64::from_bits(sign | bits)
}

/// The `%f` conversion of `printf` with the default precision of 6.
pub fn format_f(value: f64) -> String {
    if value.is_nan() {
        // glibc prints the sign of a NaN.
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
    format!("{:.6}", value)
}
