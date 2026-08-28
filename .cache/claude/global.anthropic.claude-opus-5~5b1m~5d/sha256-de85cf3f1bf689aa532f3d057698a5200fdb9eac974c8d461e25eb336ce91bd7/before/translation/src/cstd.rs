//! Small re-implementations of the C standard library facilities used by the
//! original program: `atof()` (i.e. `strtod()` semantics) and the subset of
//! `printf()` conversion used for `"%f"`.
//!
//! These are needed because Rust's own float parsing/formatting differ from C
//! in several observable ways (leading whitespace, trailing garbage, hex
//! floats, `nan`/`inf` spelling, ...).

/// C `isspace()` for the "C" locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a') as u32 + 10),
        b'A'..=b'F' => Some((c - b'A') as u32 + 10),
        _ => None,
    }
}

/// Case-insensitive check whether `s[at..]` starts with `pat` (`pat` must be
/// lowercase ASCII).
fn starts_with_ci(s: &[u8], at: usize, pat: &[u8]) -> bool {
    if s.len() < at + pat.len() {
        return false;
    }
    s[at..at + pat.len()]
        .iter()
        .zip(pat.iter())
        .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

const POS_NAN_BITS: u64 = 0x7ff8_0000_0000_0000;
const NEG_NAN_BITS: u64 = 0xfff8_0000_0000_0000;

fn signed_nan(neg: bool) -> f64 {
    f64::from_bits(if neg { NEG_NAN_BITS } else { POS_NAN_BITS })
}

/// `nan(n-char-sequence)`: glibc stores the parsed number in the mantissa bits
/// (keeping the quiet bit set).
fn nan_with_payload(neg: bool, payload: u64) -> f64 {
    let mantissa = (payload & 0x000f_ffff_ffff_ffff) | 0x0008_0000_0000_0000;
    let sign = if neg { 1u64 << 63 } else { 0 };
    f64::from_bits(sign | 0x7ff0_0000_0000_0000 | mantissa)
}

/// C `strtoull(s, &end, 0)` restricted to what a `nan(...)` payload may hold:
/// the whole slice must be a valid unsigned integer (no sign, no whitespace).
/// Overflow saturates at `ULLONG_MAX`, like `strtoull` does.
fn strtoull_base0(s: &[u8]) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let (digits, base): (&[u8], u64) = if s.len() > 1 && s[0] == b'0' && (s[1] | 0x20) == b'x' {
        (&s[2..], 16)
    } else if s[0] == b'0' {
        (&s[1..], 8)
    } else {
        (s, 10)
    };
    if digits.is_empty() {
        // "0" is a valid (octal) zero; "0x" is not a complete number.
        return if base == 8 { Some(0) } else { None };
    }
    let mut acc: u64 = 0;
    let mut overflow = false;
    for &c in digits {
        let d = match hex_val(c) {
            Some(d) if (d as u64) < base => d as u64,
            _ => return None,
        };
        match acc.checked_mul(base).and_then(|v| v.checked_add(d)) {
            Some(v) => acc = v,
            None => overflow = true,
        }
    }
    if overflow {
        Some(u64::MAX)
    } else {
        Some(acc)
    }
}

fn signed_inf(neg: bool) -> f64 {
    if neg {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    }
}

fn signed_zero(neg: bool) -> f64 {
    if neg {
        -0.0
    } else {
        0.0
    }
}

/// C `atof()`: `strtod()` while ignoring the end pointer and errors.
pub fn atof(s: &[u8]) -> f64 {
    strtod(s).0
}

/// C `strtod()`. Returns the converted value and the number of bytes consumed
/// (0 when no conversion could be performed).
pub fn strtod(s: &[u8]) -> (f64, usize) {
    let len = s.len();
    let mut i = 0usize;

    while i < len && is_space(s[i]) {
        i += 1;
    }

    let mut neg = false;
    if i < len && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    // inf / infinity
    if starts_with_ci(s, i, b"infinity") {
        return (signed_inf(neg), i + 8);
    }
    if starts_with_ci(s, i, b"inf") {
        return (signed_inf(neg), i + 3);
    }

    // nan / nan(n-char-sequence)
    if starts_with_ci(s, i, b"nan") {
        let mut end = i + 3;
        let mut payload: Option<u64> = None;
        if end < len && s[end] == b'(' {
            let mut k = end + 1;
            while k < len && (s[k].is_ascii_alphanumeric() || s[k] == b'_') {
                k += 1;
            }
            if k < len && s[k] == b')' {
                payload = strtoull_base0(&s[end + 1..k]);
                end = k + 1;
            }
        }
        return match payload {
            Some(p) => (nan_with_payload(neg, p), end),
            None => (signed_nan(neg), end),
        };
    }

    // hexadecimal floating point: 0x... / 0X...
    if i + 1 < len && s[i] == b'0' && (s[i + 1] == b'x' || s[i + 1] == b'X') {
        if let Some(r) = parse_hex(s, i + 2, neg) {
            return r;
        }
        // No hex digits after the prefix: the longest valid subject sequence is
        // just "0", which converts to (signed) zero.
        return (signed_zero(neg), i + 1);
    }

    parse_decimal(s, i, neg)
}

/// Decimal form: `digits [ '.' digits ] [ ('e'|'E') [sign] digits ]`
fn parse_decimal(s: &[u8], start: usize, neg: bool) -> (f64, usize) {
    let len = s.len();
    let mut i = start;
    let mut ndigits = 0usize;

    let int_start = i;
    while i < len && is_digit(s[i]) {
        i += 1;
        ndigits += 1;
    }
    let int_end = i;

    let mut frac_start = i;
    let mut frac_end = i;
    if i < len && s[i] == b'.' {
        i += 1;
        frac_start = i;
        while i < len && is_digit(s[i]) {
            i += 1;
            ndigits += 1;
        }
        frac_end = i;
    }

    if ndigits == 0 {
        // No conversion performed.
        return (0.0, 0);
    }

    let mut end = i;
    let mut exp_text: &[u8] = b"";
    if i < len && (s[i] == b'e' || s[i] == b'E') {
        let mut j = i + 1;
        if j < len && (s[j] == b'+' || s[j] == b'-') {
            j += 1;
        }
        if j < len && is_digit(s[j]) {
            while j < len && is_digit(s[j]) {
                j += 1;
            }
            exp_text = &s[i + 1..j];
            end = j;
        }
    }

    // Rebuild a string that Rust's float parser accepts (it is correctly
    // rounded, just like glibc's strtod).
    let mut text = String::with_capacity(ndigits + 8);
    if int_end > int_start {
        text.push_str(core::str::from_utf8(&s[int_start..int_end]).unwrap_or("0"));
    } else {
        text.push('0');
    }
    text.push('.');
    if frac_end > frac_start {
        text.push_str(core::str::from_utf8(&s[frac_start..frac_end]).unwrap_or(""));
    } else {
        text.push('0');
    }
    if !exp_text.is_empty() {
        text.push('e');
        text.push_str(core::str::from_utf8(exp_text).unwrap_or("0"));
    }

    let mut value: f64 = match text.parse::<f64>() {
        Ok(v) => v,
        Err(_) => 0.0,
    };
    if neg {
        value = -value;
    }
    (value, end)
}

/// Hexadecimal form: `hexdigits [ '.' hexdigits ] [ ('p'|'P') [sign] digits ]`.
/// `start` points just past the `0x` prefix. Returns `None` when there is no
/// hex digit at all.
fn parse_hex(s: &[u8], start: usize, neg: bool) -> Option<(f64, usize)> {
    let len = s.len();
    let mut i = start;

    // Mantissa collected as an integer `m` (with a sticky bit for dropped low
    // digits) scaled by 2^extra, and the number of fractional hex digits.
    let mut m: u128 = 0;
    let mut sticky = false;
    let mut extra: i64 = 0;
    let mut frac_digits: i64 = 0;
    let mut any = false;

    const CAP: u128 = 1u128 << 100;

    let push = |d: u32, m: &mut u128, sticky: &mut bool, extra: &mut i64| {
        if *m < CAP {
            *m = *m * 16 + d as u128;
        } else {
            *extra += 4;
            if d != 0 {
                *sticky = true;
            }
        }
    };

    while i < len {
        match hex_val(s[i]) {
            Some(d) => {
                push(d, &mut m, &mut sticky, &mut extra);
                any = true;
                i += 1;
            }
            None => break,
        }
    }
    if i < len && s[i] == b'.' {
        i += 1;
        while i < len {
            match hex_val(s[i]) {
                Some(d) => {
                    push(d, &mut m, &mut sticky, &mut extra);
                    frac_digits += 1;
                    any = true;
                    i += 1;
                }
                None => break,
            }
        }
    }
    if !any {
        return None;
    }

    let mut end = i;
    let mut pexp: i64 = 0;
    if i < len && (s[i] == b'p' || s[i] == b'P') {
        let mut j = i + 1;
        let mut esign: i64 = 1;
        if j < len && (s[j] == b'+' || s[j] == b'-') {
            if s[j] == b'-' {
                esign = -1;
            }
            j += 1;
        }
        if j < len && is_digit(s[j]) {
            let mut v: i64 = 0;
            while j < len && is_digit(s[j]) {
                v = (v * 10 + (s[j] - b'0') as i64).min(1_000_000);
                j += 1;
            }
            pexp = esign * v;
            end = j;
        }
    }

    let exp = (pexp + extra - 4 * frac_digits).clamp(-2_000_000, 2_000_000) as i32;
    Some((make_f64(neg, m, sticky, exp), end))
}

/// Round `(-1)^neg * m * 2^exp` (with `sticky` marking discarded nonzero low
/// bits) to the nearest `f64`, ties to even.
fn make_f64(neg: bool, m: u128, sticky: bool, exp: i32) -> f64 {
    let mut m = m;
    let mut sticky = sticky;
    let mut exp: i64 = exp as i64;

    if m == 0 {
        // Digits are only ever dropped once `m` is huge, so `sticky` cannot be
        // set here: the value really is zero.
        return signed_zero(neg);
    }

    // Make room so the sticky bit can be folded into bit 0 without changing the
    // rounding decision, then keep the mantissa reasonably small.
    let bl = |v: u128| -> i64 { 128 - v.leading_zeros() as i64 };

    if bl(m) < 56 {
        let sh = 56 - bl(m);
        m <<= sh as u32;
        exp -= sh;
    }
    if bl(m) > 64 {
        let sh = bl(m) - 64;
        let mask = (1u128 << sh as u32) - 1;
        if m & mask != 0 {
            sticky = true;
        }
        m >>= sh as u32;
        exp += sh;
    }
    if sticky {
        m |= 1;
    }

    let mlen = bl(m); // 56 ..= 64
    let e_unbiased = exp + mlen - 1;

    // Number of low bits to discard.
    let mut drop = mlen - 53;
    if e_unbiased < -1022 {
        // Subnormal result: quantum is 2^-1074.
        drop = -1074 - exp;
        if drop >= mlen + 2 {
            return signed_zero(neg);
        }
        if drop <= 0 {
            drop = 0;
        }
    }

    let mut q: u128;
    let e2: i64;
    if drop <= 0 {
        q = m;
        e2 = exp;
    } else {
        let shift = drop as u32;
        let lost = m & ((1u128 << shift) - 1);
        let half = 1u128 << (shift - 1);
        q = m >> shift;
        if lost > half || (lost == half && (q & 1) == 1) {
            q += 1;
        }
        e2 = exp + drop;
    }

    if q == 0 {
        return signed_zero(neg);
    }

    let qlen = bl(q);
    let e = e2 + qlen - 1; // unbiased exponent of the rounded value

    let sign_bits: u64 = if neg { 1u64 << 63 } else { 0 };

    if e > 1023 {
        return signed_inf(neg);
    }
    if e >= -1022 {
        // Normal number: q has at most 53 significant bits here.
        let frac = if qlen >= 53 {
            (q >> (qlen - 53) as u32) as u64
        } else {
            (q as u64) << (53 - qlen) as u32
        } & ((1u64 << 52) - 1);
        let biased = (e + 1023) as u64;
        f64::from_bits(sign_bits | (biased << 52) | frac)
    } else {
        // Subnormal: value is q * 2^e2 with e2 >= -1074.
        let sh = e2 + 1074;
        let bits = if sh >= 0 {
            (q as u64) << sh as u32
        } else {
            (q >> (-sh) as u32) as u64
        };
        f64::from_bits(sign_bits | (bits & ((1u64 << 52) - 1)))
    }
}

/// `printf("%f", x)` where the argument is a C `float` that the default
/// argument promotions widen to `double`. The sign of a NaN is taken from the
/// `float` itself so that nothing depends on how a float-to-double cast treats
/// NaN payloads.
pub fn printf_f_float(x: f32) -> String {
    if x.is_nan() {
        return if x.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    printf_f(x as f64)
}

/// Format a `double` the way glibc's `printf("%f", x)` does.
pub fn printf_f(x: f64) -> String {
    if x.is_nan() {
        return if x.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.6}", x)
}
