//! Reproduction of glibc's `scanf("%lf", ...)` conversion, including its
//! character-at-a-time consumption, its one-character push-back, and the
//! `strtod` call it performs on the collected work buffer.

use std::io::Read;

/// A one-byte-lookahead reader over a byte stream, mirroring `getc`/`ungetc`.
pub struct Reader<R: Read> {
    inner: R,
    peeked: Option<Option<u8>>,
}

impl<R: Read> Reader<R> {
    pub fn new(inner: R) -> Self {
        Reader {
            inner,
            peeked: None,
        }
    }

    fn read_raw(&mut self) -> Option<u8> {
        let mut b = [0u8; 1];
        loop {
            match self.inner.read(&mut b) {
                Ok(0) => return None,
                Ok(_) => return Some(b[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    /// `getc` without consuming.
    pub fn peek(&mut self) -> Option<u8> {
        if self.peeked.is_none() {
            let v = self.read_raw();
            self.peeked = Some(v);
        }
        self.peeked.unwrap()
    }

    /// `getc`.
    pub fn next_byte(&mut self) -> Option<u8> {
        match self.peeked.take() {
            Some(v) => v,
            None => self.read_raw(),
        }
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn lower(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_xdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

const QUIET_NAN_BITS: u64 = 0x7ff8_0000_0000_0000;

/// Consume `word` case-insensitively. Returns false as soon as a byte fails to
/// match (glibc has already consumed those bytes and reports a matching
/// failure, which is why partial matches such as "infinit" fail outright).
fn match_ci<R: Read>(r: &mut Reader<R>, word: &[u8]) -> bool {
    for &w in word {
        match r.peek() {
            Some(c) if lower(c) == w => {
                r.next_byte();
            }
            _ => return false,
        }
    }
    true
}

/// `scanf("%lf", &f)`: returns `None` on input or matching failure, in which
/// case the caller must leave its variable untouched.
pub fn scan_lf<R: Read>(r: &mut Reader<R>) -> Option<f64> {
    // Skip leading whitespace.
    while let Some(c) = r.peek() {
        if is_c_space(c) {
            r.next_byte();
        } else {
            break;
        }
    }

    // Work buffer glibc fills and later hands to strtod.
    let mut wp: Vec<u8> = Vec::new();

    let mut c = r.peek();

    // Optional sign.
    let mut negative = false;
    if c == Some(b'-') || c == Some(b'+') {
        let s = c.unwrap();
        negative = s == b'-';
        wp.push(s);
        r.next_byte();
        c = r.peek();
    }

    // "nan" / "inf" / "infinity".
    if let Some(ch) = c {
        if lower(ch) == b'n' {
            if !match_ci(r, b"nan") {
                return None;
            }
            // Optional n-char-sequence in parentheses; the payload is ignored.
            if r.peek() == Some(b'(') {
                r.next_byte();
                loop {
                    match r.next_byte() {
                        None | Some(b')') => break,
                        _ => {}
                    }
                }
            }
            let bits = if negative {
                QUIET_NAN_BITS | (1u64 << 63)
            } else {
                QUIET_NAN_BITS
            };
            return Some(f64::from_bits(bits));
        }
        if lower(ch) == b'i' {
            if !match_ci(r, b"inf") {
                return None;
            }
            if r.peek().map(lower) == Some(b'i') && !match_ci(r, b"inity") {
                return None;
            }
            return Some(if negative {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            });
        }
    }

    let mut got_digit = false;
    let mut got_dot = false;
    // Radix point seen in the mantissa (the exponent branch also sets got_dot).
    let mut got_radix = false;
    let mut got_e = false;
    let mut hexa = false;
    let mut exp_char = b'e';

    if c == Some(b'0') {
        wp.push(b'0');
        r.next_byte();
        c = r.peek();
        match c {
            Some(x) if lower(x) == b'x' => {
                wp.push(x);
                hexa = true;
                exp_char = b'p';
                r.next_byte();
                c = r.peek();
            }
            _ => got_digit = true,
        }
    }

    loop {
        let ch = match c {
            Some(x) => x,
            None => break,
        };
        if is_digit(ch) {
            wp.push(ch);
            // Digits belonging to the exponent do not count as mantissa digits
            // for the "bare 0x prefix" error check below.
            if !got_e {
                got_digit = true;
            }
        } else if !got_e && hexa && is_xdigit(ch) {
            wp.push(ch);
            got_digit = true;
        } else if got_e
            && wp.last().copied() == Some(exp_char)
            && (ch == b'+' || ch == b'-')
        {
            wp.push(ch);
        } else if !wp.is_empty() && !got_e && lower(ch) == exp_char {
            wp.push(exp_char);
            got_e = true;
            got_dot = true;
        } else if !got_dot && ch == b'.' {
            wp.push(ch);
            got_dot = true;
            got_radix = true;
        } else {
            break;
        }
        r.next_byte();
        c = r.peek();
    }

    // Nothing at all was collected.
    if wp.is_empty() {
        return None;
    }
    // A bare "0x" prefix (no hex digit and no radix character) is an error.
    if hexa && !got_digit && !got_radix {
        return None;
    }

    // strtod on the work buffer; if it converts nothing this is a failure.
    match strtod_prefix(&wp) {
        Some((v, n)) if n > 0 => Some(v),
        _ => None,
    }
}

fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => c - b'A' + 10,
    }
}

/// `strtod` restricted to what the scanf work buffer can hold. Returns the
/// value together with the number of bytes consumed (0 / `None` meaning
/// "nothing convertible").
fn strtod_prefix(s: &[u8]) -> Option<(f64, usize)> {
    let mut i = 0usize;
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Hexadecimal form.
    if i + 1 < s.len() && s[i] == b'0' && lower(s[i + 1]) == b'x' {
        let mut digits: Vec<u8> = Vec::new();
        let mut frac_digits: i64 = 0;
        let mut any = false;
        let mut j = i + 2;
        while j < s.len() && is_xdigit(s[j]) {
            digits.push(hex_val(s[j]));
            any = true;
            j += 1;
        }
        if j < s.len() && s[j] == b'.' {
            let mut k = j + 1;
            while k < s.len() && is_xdigit(s[k]) {
                digits.push(hex_val(s[k]));
                frac_digits += 1;
                any = true;
                k += 1;
            }
            if any {
                j = k;
            }
        }
        if !any {
            // Only "0x" was present: strtod converts the leading "0".
            return Some((if negative { -0.0 } else { 0.0 }, i + 1));
        }
        let mut end = j;
        let mut pexp: i64 = 0;
        if j < s.len() && lower(s[j]) == b'p' {
            let mut k = j + 1;
            let mut esign: i64 = 1;
            if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
                if s[k] == b'-' {
                    esign = -1;
                }
                k += 1;
            }
            if k < s.len() && is_digit(s[k]) {
                let mut v: i64 = 0;
                while k < s.len() && is_digit(s[k]) {
                    if v < 1_000_000_000 {
                        v = v * 10 + (s[k] - b'0') as i64;
                    }
                    k += 1;
                }
                pexp = esign * v.min(1_000_000_000);
                end = k;
            }
        }
        return Some((hex_to_f64(&digits, frac_digits, pexp, negative), end));
    }

    // Decimal form.
    let mut j = i;
    let mut mantissa_digits = 0usize;
    let int_start = j;
    while j < s.len() && is_digit(s[j]) {
        j += 1;
        mantissa_digits += 1;
    }
    let int_end = j;
    let mut frac_start = j;
    let mut frac_end = j;
    if j < s.len() && s[j] == b'.' {
        j += 1;
        frac_start = j;
        while j < s.len() && is_digit(s[j]) {
            j += 1;
            mantissa_digits += 1;
        }
        frac_end = j;
    }
    if mantissa_digits == 0 {
        return None;
    }
    let mut end = j;
    let mut exp_text = String::new();
    if j < s.len() && lower(s[j]) == b'e' {
        let mut k = j + 1;
        let mut esign = "";
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            if s[k] == b'-' {
                esign = "-";
            }
            k += 1;
        }
        if k < s.len() && is_digit(s[k]) {
            let dstart = k;
            while k < s.len() && is_digit(s[k]) {
                k += 1;
            }
            // Clamp absurd exponents; the outcome (inf or zero) is unchanged.
            let mut v: i64 = 0;
            for &d in &s[dstart..k] {
                if v < 1_000_000_000 {
                    v = v * 10 + (d - b'0') as i64;
                }
            }
            exp_text = format!("{}{}", esign, v.min(1_000_000_000));
            end = k;
        }
    }

    let int_part = &s[int_start..int_end];
    let frac_part = &s[frac_start..frac_end];
    let mut t = String::new();
    if negative {
        t.push('-');
    }
    if int_part.is_empty() {
        t.push('0');
    } else {
        t.push_str(std::str::from_utf8(int_part).unwrap());
    }
    t.push('.');
    if frac_part.is_empty() {
        t.push('0');
    } else {
        t.push_str(std::str::from_utf8(frac_part).unwrap());
    }
    if !exp_text.is_empty() {
        t.push('e');
        t.push_str(&exp_text);
    }
    let v: f64 = t.parse().unwrap_or(0.0);
    Some((v, end))
}

fn shift_right(m: u128, k: i64) -> u128 {
    if k >= 128 {
        0
    } else if k <= 0 {
        m
    } else {
        m >> k
    }
}

fn bit_at(m: u128, k: i64) -> u32 {
    if k < 0 || k >= 128 {
        0
    } else {
        ((m >> k) & 1) as u32
    }
}

/// Any set bit strictly below position `k`.
fn any_below(m: u128, k: i64) -> bool {
    if k <= 0 {
        false
    } else if k >= 128 {
        m != 0
    } else {
        (m & ((1u128 << k) - 1)) != 0
    }
}

/// Build the correctly rounded (round-to-nearest, ties-to-even) `f64` closest
/// to `digits * 16^-frac_digits * 2^pexp`.
fn hex_to_f64(digits: &[u8], frac_digits: i64, pexp: i64, negative: bool) -> f64 {
    let mut mant: u128 = 0;
    let mut sticky = false;
    let mut extra: i64 = 0;

    for &d in digits {
        if mant == 0 && d == 0 {
            continue; // leading zeros carry no weight
        }
        if mant.leading_zeros() >= 4 {
            mant = (mant << 4) | d as u128;
        } else {
            if d != 0 {
                sticky = true;
            }
            extra += 4;
        }
    }

    if mant == 0 {
        return if negative { -0.0 } else { 0.0 };
    }

    let exp2 = pexp
        .saturating_sub(frac_digits.saturating_mul(4))
        .saturating_add(extra);

    let bit_len = 128 - mant.leading_zeros() as i64;
    let shift_normal = bit_len - 53;
    let shift_sub = (-1074i64).saturating_sub(exp2);
    let shift = shift_normal.max(shift_sub);

    let (q, e2) = if shift <= 0 {
        (mant, exp2)
    } else {
        let mut q = shift_right(mant, shift);
        let half = bit_at(mant, shift - 1);
        let rest = any_below(mant, shift - 1) || sticky;
        if half == 1 && (rest || (q & 1) == 1) {
            q += 1;
        }
        (q, exp2.saturating_add(shift))
    };

    make_f64(q as u64, e2, negative)
}

/// `q * 2^e2` where `q <= 2^53` is already rounded to the target precision.
fn make_f64(q: u64, e2: i64, negative: bool) -> f64 {
    let sign_bit = if negative { 1u64 << 63 } else { 0 };
    if q == 0 {
        return f64::from_bits(sign_bit);
    }
    let q_bits = 64 - q.leading_zeros() as i64;
    let e = e2 + q_bits - 1;
    if e > 1023 {
        return f64::from_bits(sign_bit | 0x7ff0_0000_0000_0000);
    }
    let bits = if e >= -1022 {
        let significand = q << (53 - q_bits);
        let frac = significand & 0x000f_ffff_ffff_ffff;
        (((e + 1023) as u64) << 52) | frac
    } else {
        // Subnormal: e2 is never below -1074, so this shift is non-negative and
        // the encoding of a subnormal significand is just the scaled integer.
        q << (e2 + 1074)
    };
    f64::from_bits(sign_bit | bits)
}
