//! A faithful re-implementation of glibc's `strtod` for `double`, including the
//! exact `endptr` placement and the exact conditions under which `errno` is set
//! to `ERANGE`.
//!
//! Behaviours reproduced (verified against glibc 2.34 on x86-64):
//!  * leading C-locale whitespace and an optional sign are skipped, but if no
//!    conversion can be performed `endptr` is reset to the start of the string,
//!  * decimal forms, C99 hexadecimal forms (`0x1.8p3`, exponent optional as an
//!    accepted glibc extension), `inf`/`infinity` and `nan`/`nan(chars)`,
//!  * correct round-to-nearest-even conversion, including subnormals,
//!  * `ERANGE` on overflow (the rounded result is infinite) and on underflow,
//!    where underflow means IEEE "tininess after rounding" *and* inexactness:
//!    the value rounded to 53 significant bits with an unbounded exponent is
//!    smaller than `DBL_MIN`, and the returned double differs from the exact
//!    value.  This is why `strtod("0x1p-1023")` does *not* set `ERANGE` (exact)
//!    while `strtod("1e-320")` does.

/// Result of a `strtod` call.
pub struct Strtod {
    /// The converted value.
    pub value: f64,
    /// Number of bytes consumed, i.e. `endptr - nptr`.
    pub consumed: usize,
    /// Whether the conversion stored `ERANGE` in `errno`.
    pub erange: bool,
}

/// `T = 2^-1022 - 2^-1076`, the exact threshold below which a value is "tiny"
/// after rounding to 53 significant bits (a value exactly equal to `T` rounds
/// up to `DBL_MIN` because of round-half-to-even, hence it is *not* tiny).
/// Stored as `0.<TINY_THRESHOLD_DIGITS> * 10^TINY_THRESHOLD_EXP10`.
const TINY_THRESHOLD_EXP10: i64 = -307;
const TINY_THRESHOLD_DIGITS: &[u8] = b"2225073858507201259573821257020768020077017763406988739288376763306013328417497570685406341460323054239108249322037716056011260300124027377191834796392769721437078990836532798904431849864732504110467273084696977812028716236556967935895657351868202788722494811530151317616366333296945953431369222190308053787694940411743707809822580740988880551617907119002148759401915892151482081924890263312702257321184750771861452224096212631698623638776860141838061165702263776640907648194435536054336373727978014593100678660492117516784908521511159767373323339191983221326853519128338784891913380715532840971003878993627240686726663397609149834349831344879676653469091559130189899114521124782380547341009775590676096291585949697743018930811385869272811532937339507043361663818359375";

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

fn hex_val(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a') as u32 + 10),
        b'A'..=b'F' => Some((b - b'A') as u32 + 10),
        _ => None,
    }
}

fn lower(b: u8) -> u8 {
    b.to_ascii_lowercase()
}

fn signed(value: f64, negative: bool) -> f64 {
    if negative {
        -value
    } else {
        value
    }
}

fn nan_with_sign(negative: bool) -> f64 {
    let bits: u64 = 0x7ff8_0000_0000_0000 | if negative { 1u64 << 63 } else { 0 };
    f64::from_bits(bits)
}

/// Convert the initial portion of `s` to a `double`, C style.
pub fn strtod(s: &[u8]) -> Strtod {
    let len = s.len();
    let mut i = 0usize;

    while i < len && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < len && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Hexadecimal form: 0x / 0X followed by at least one hex digit.
    if i + 1 < len && s[i] == b'0' && lower(s[i + 1]) == b'x' {
        if let Some(res) = parse_hex(s, i + 2, negative) {
            return res;
        }
        // Not a valid hex form; fall through and let the decimal scanner
        // consume the leading "0" (glibc behaviour: strtod("0x") consumes "0").
    }

    if let Some(res) = parse_decimal(s, i, negative) {
        return res;
    }

    if let Some(res) = parse_inf_nan(s, i, negative) {
        return res;
    }

    // No conversion performed: endptr == nptr.
    Strtod {
        value: 0.0,
        consumed: 0,
        erange: false,
    }
}

fn parse_inf_nan(s: &[u8], start: usize, negative: bool) -> Option<Strtod> {
    let len = s.len();
    let matches_ci = |word: &[u8]| -> bool {
        start + word.len() <= len
            && s[start..start + word.len()]
                .iter()
                .zip(word.iter())
                .all(|(a, b)| lower(*a) == *b)
    };

    if matches_ci(b"infinity") {
        return Some(Strtod {
            value: signed(f64::INFINITY, negative),
            consumed: start + 8,
            erange: false,
        });
    }
    if matches_ci(b"inf") {
        return Some(Strtod {
            value: signed(f64::INFINITY, negative),
            consumed: start + 3,
            erange: false,
        });
    }
    if matches_ci(b"nan") {
        let mut consumed = start + 3;
        // Optional n-char-sequence in parentheses.
        if consumed < len && s[consumed] == b'(' {
            let mut j = consumed + 1;
            while j < len && (s[j].is_ascii_alphanumeric() || s[j] == b'_') {
                j += 1;
            }
            if j < len && s[j] == b')' {
                consumed = j + 1;
            }
        }
        return Some(Strtod {
            value: nan_with_sign(negative),
            consumed,
            erange: false,
        });
    }
    None
}

/// Scan an optional exponent part introduced by one of `markers`.
/// Returns the index just past the exponent and its value, or `None` when there
/// is no (complete) exponent, in which case nothing is consumed.
fn scan_exponent(s: &[u8], at: usize, markers: [u8; 2]) -> Option<(usize, i64)> {
    let len = s.len();
    if at >= len || (lower(s[at]) != markers[0] && lower(s[at]) != markers[1]) {
        return None;
    }
    let mut j = at + 1;
    let mut exp_negative = false;
    if j < len && (s[j] == b'+' || s[j] == b'-') {
        exp_negative = s[j] == b'-';
        j += 1;
    }
    let digits_start = j;
    let mut exp: i64 = 0;
    while j < len && is_digit(s[j]) {
        // Saturate: astronomically large exponents only need to stay huge.
        exp = exp.saturating_mul(10).saturating_add((s[j] - b'0') as i64);
        if exp > 1_000_000_000 {
            exp = 1_000_000_000;
        }
        j += 1;
    }
    if j == digits_start {
        return None;
    }
    Some((j, if exp_negative { -exp } else { exp }))
}

fn parse_decimal(s: &[u8], start: usize, negative: bool) -> Option<Strtod> {
    let len = s.len();
    let mut i = start;
    let mut digits: Vec<u8> = Vec::new();
    let mut int_digits = 0usize;

    while i < len && is_digit(s[i]) {
        digits.push(s[i]);
        int_digits += 1;
        i += 1;
    }
    let mut frac_digits = 0usize;
    if i < len && s[i] == b'.' {
        let point = i;
        i += 1;
        while i < len && is_digit(s[i]) {
            digits.push(s[i]);
            frac_digits += 1;
            i += 1;
        }
        if int_digits == 0 && frac_digits == 0 {
            // Just a '.' with no digits: no conversion.
            i = point;
        }
    }
    if int_digits == 0 && frac_digits == 0 {
        return None;
    }

    let mut exp10: i64 = 0;
    if let Some((next, e)) = scan_exponent(s, i, [b'e', b'e']) {
        i = next;
        exp10 = e;
    }

    // Normalised representation: value = 0.<digits without leading zeros> * 10^p10
    let lead_zeros = digits.iter().take_while(|d| **d == b'0').count();
    let sig: &[u8] = &digits[lead_zeros..];

    if sig.is_empty() {
        // The value is exactly zero; no range error.
        return Some(Strtod {
            value: signed(0.0, negative),
            consumed: i,
            erange: false,
        });
    }

    let p10 = (sig.len() as i64)
        .saturating_add(exp10)
        .saturating_sub(frac_digits as i64);

    // Obvious over/underflow, short-circuited so that the huge exponents which
    // C accepts never reach the float parser.
    if p10 > 350 {
        return Some(Strtod {
            value: signed(f64::INFINITY, negative),
            consumed: i,
            erange: true,
        });
    }
    if p10 < -350 {
        return Some(Strtod {
            value: signed(0.0, negative),
            consumed: i,
            erange: true,
        });
    }

    let mut text = String::with_capacity(sig.len() + 24);
    text.push_str("0.");
    for d in sig {
        text.push(*d as char);
    }
    text.push('e');
    text.push_str(&p10.to_string());
    let magnitude: f64 = text.parse().unwrap_or(0.0);

    let erange = if magnitude.is_infinite() {
        true
    } else if decimal_is_tiny(sig, p10) {
        // Underflow additionally requires the result to be inexact.
        !decimal_equals_double(sig, p10, magnitude)
    } else {
        false
    };

    Some(Strtod {
        value: signed(magnitude, negative),
        consumed: i,
        erange,
    })
}

/// Compare `0.<sig> * 10^p10` (with `sig` free of leading zeros and non-empty)
/// against the tininess threshold `2^-1022 - 2^-1076`.
fn decimal_is_tiny(sig: &[u8], p10: i64) -> bool {
    match p10.cmp(&TINY_THRESHOLD_EXP10) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => {
            compare_digits(sig, TINY_THRESHOLD_DIGITS) == std::cmp::Ordering::Less
        }
    }
}

/// Compare two zero-padded digit strings as fractions `0.a` vs `0.b`.
fn compare_digits(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    for k in 0..n {
        let da = a.get(k).copied().unwrap_or(b'0');
        let db = b.get(k).copied().unwrap_or(b'0');
        if da != db {
            return da.cmp(&db);
        }
    }
    std::cmp::Ordering::Equal
}

/// Is `0.<sig> * 10^p10` exactly equal to the (finite, non-negative) double
/// `value`?  Used only on the tiny path, where `value` is subnormal or
/// `DBL_MIN`, so its exact decimal expansion has at most 1074 fraction digits.
fn decimal_equals_double(sig: &[u8], p10: i64, value: f64) -> bool {
    let exact = format!("{:.1080}", value);
    let bytes = exact.as_bytes();
    let dot = match bytes.iter().position(|b| *b == b'.') {
        Some(p) => p,
        None => return false,
    };
    let mut digits: Vec<u8> = Vec::with_capacity(bytes.len());
    digits.extend_from_slice(&bytes[..dot]);
    digits.extend_from_slice(&bytes[dot + 1..]);
    let int_len = dot as i64;
    let lead = digits.iter().take_while(|d| **d == b'0').count();
    let vsig = &digits[lead..];
    if vsig.is_empty() {
        return false; // the double is zero, the decimal is not
    }
    let vp10 = int_len - lead as i64;
    // Trailing zeros are irrelevant for both operands (compare_digits pads).
    p10 == vp10 && compare_digits(sig, vsig) == std::cmp::Ordering::Equal
}

/// Parse the part of a hexadecimal floating constant following `0x`.
/// Returns `None` if there is no hex digit, i.e. if the form is invalid.
fn parse_hex(s: &[u8], start: usize, negative: bool) -> Option<Strtod> {
    let len = s.len();
    let mut i = start;

    // value = acc * 2^e2 (plus a sticky remainder below acc's lowest bit)
    let mut acc: u64 = 0;
    let mut sticky = false;
    let mut e2: i64 = 0;
    let mut ndigits = 0usize;

    let push = |d: u32, acc: &mut u64, sticky: &mut bool, e2: &mut i64| {
        if *acc <= (u64::MAX - 15) / 16 {
            *acc = *acc * 16 + d as u64;
        } else {
            *e2 = e2.saturating_add(4);
            if d != 0 {
                *sticky = true;
            }
        }
    };

    while i < len {
        match hex_val(s[i]) {
            Some(d) => {
                push(d, &mut acc, &mut sticky, &mut e2);
                ndigits += 1;
                i += 1;
            }
            None => break,
        }
    }
    if i < len && s[i] == b'.' {
        let point = i;
        i += 1;
        let mut frac = 0usize;
        while i < len {
            match hex_val(s[i]) {
                Some(d) => {
                    push(d, &mut acc, &mut sticky, &mut e2);
                    e2 = e2.saturating_sub(4);
                    ndigits += 1;
                    frac += 1;
                    i += 1;
                }
                None => break,
            }
        }
        if ndigits == 0 && frac == 0 {
            i = point;
        }
    }
    if ndigits == 0 {
        return None;
    }

    if let Some((next, e)) = scan_exponent(s, i, [b'p', b'p']) {
        i = next;
        e2 = e2.saturating_add(e);
    }

    let (magnitude, inexact, tiny) = assemble_double(acc, sticky, e2);
    let erange = if magnitude.is_infinite() {
        true
    } else {
        tiny && inexact
    };

    Some(Strtod {
        value: signed(magnitude, negative),
        consumed: i,
        erange,
    })
}

fn bit_len(v: u64) -> i64 {
    (64 - v.leading_zeros()) as i64
}

/// Round `acc * 2^e2` (with a sticky remainder below the low bit of `acc` when
/// `sticky`) to the nearest multiple of `2^lsb`, ties to even.
/// Returns the multiplier and whether the rounding was inexact.
fn round_to_lsb(acc: u64, sticky: bool, e2: i64, lsb: i64) -> (u64, bool) {
    let drop = lsb.saturating_sub(e2);
    if drop <= 0 {
        let shift = (-drop) as u64;
        if shift >= 64 {
            // Cannot happen for the exponents used below, but stay safe.
            return (0, true);
        }
        return (acc << shift, sticky);
    }
    if drop > 64 {
        // Everything is dropped and the value is below half of 2^lsb.
        return (0, acc != 0 || sticky);
    }
    let a = acc as u128;
    let d = drop as u32;
    let m = a >> d;
    let low = a & ((1u128 << d) - 1);
    let half = 1u128 << (d - 1);
    let inexact = low != 0 || sticky;
    let round_up = low > half || (low == half && (sticky || (m & 1) == 1));
    let m = if round_up { m + 1 } else { m };
    (m as u64, inexact)
}

/// Build a `double` out of `mantissa * 2^lsb` (mantissa has at most 54 bits and
/// is only 54 bits wide when it is exactly `2^53`).
fn compose(mantissa: u64, lsb: i64) -> f64 {
    if mantissa == 0 {
        return 0.0;
    }
    let mut mantissa = mantissa;
    let mut lsb = lsb;
    let mut nb = bit_len(mantissa);
    while nb > 53 {
        mantissa >>= 1;
        lsb += 1;
        nb -= 1;
    }
    let ue = nb - 1 + lsb;
    if ue > 1023 {
        return f64::INFINITY;
    }
    if ue >= -1022 {
        let biased = (ue + 1023) as u64;
        let shift = (52 - (nb - 1)) as u64;
        let frac = (mantissa << shift) & ((1u64 << 52) - 1);
        f64::from_bits((biased << 52) | frac)
    } else {
        // Subnormal: lsb is -1074 by construction.
        f64::from_bits(mantissa)
    }
}

/// Convert `acc * 2^e2` (+ sticky) into a `double`, reporting whether the
/// conversion was inexact and whether the value is tiny (i.e. below `DBL_MIN`
/// once rounded to 53 significant bits with an unbounded exponent range).
fn assemble_double(acc: u64, sticky: bool, e2: i64) -> (f64, bool, bool) {
    if acc == 0 {
        // Note: sticky can only be set once acc is huge, so the value is zero.
        return (0.0, false, false);
    }
    let ue = bit_len(acc) - 1 + e2;

    // Tininess after rounding: round to 53 significant bits, unbounded exponent.
    let lsb53 = ue.saturating_sub(52);
    let (m53, _) = round_to_lsb(acc, sticky, e2, lsb53);
    let tiny = if m53 == 0 {
        true
    } else {
        bit_len(m53) - 1 + lsb53 < -1022
    };

    // The actual double, rounded once at the correct precision.
    let lsb = lsb53.max(-1074);
    let (mantissa, inexact) = round_to_lsb(acc, sticky, e2, lsb);
    (compose(mantissa, lsb), inexact, tiny)
}
