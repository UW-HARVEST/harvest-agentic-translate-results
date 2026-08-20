// Rust translation of c_src/src/main.c
//
// The C program reads one `double` with `scanf("%lf", &f)` (leaving `f` at its
// initial value of 0.0 when the conversion fails) and then prints
//
//     printf("%llx %a %.4f\n", u.x, f, f);
//
// where `u.x` is the raw 64-bit pattern of the double.  Reproducing the output
// byte-for-byte therefore requires emulating:
//
//   * glibc's `scanf("%lf")` token collection (including its quirks, such as a
//     lone "0x" being a matching failure while "0x." converts to 0.0),
//   * glibc's `strtod` value determination (correctly rounded decimal and
//     hexadecimal floating point, "inf"/"infinity"/"nan"),
//   * glibc's `%llx`, `%a` and `%.4f` output formatting.

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Byte source (emulates stdio's one-character-at-a-time reads / EOF stickiness)
// ---------------------------------------------------------------------------

struct ByteSource<R: Read> {
    inner: R,
    eof: bool,
}

impl<R: Read> ByteSource<R> {
    fn new(inner: R) -> Self {
        ByteSource { inner, eof: false }
    }

    /// Returns the next byte, or `None` on EOF / error (mirrors `getc`).
    fn next_byte(&mut self) -> Option<u8> {
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn lower(c: u8) -> u8 {
    if c.is_ascii_uppercase() {
        c | 0x20
    } else {
        c
    }
}

// ---------------------------------------------------------------------------
// scanf("%lf") emulation
// ---------------------------------------------------------------------------

/// Emulates a single `%lf` conversion.  Returns `None` when the conversion
/// fails (input failure or matching failure), in which case the C program
/// leaves its variable untouched.
fn scan_double<R: Read>(src: &mut ByteSource<R>) -> Option<f64> {
    // Skip leading white space; EOF here is an input failure.
    let mut c = loop {
        match src.next_byte() {
            None => return None,
            Some(ch) if is_space(ch) => continue,
            Some(ch) => break ch,
        }
    };

    // Work buffer, exactly as glibc accumulates it before calling strtod.
    let mut w: Vec<u8> = Vec::new();
    let mut got_sign = false;

    if c == b'-' || c == b'+' {
        got_sign = true;
        w.push(c);
        // EOF right after the sign is a matching failure.
        c = src.next_byte()?;
    }

    // "nan"
    if lower(c) == b'n' {
        w.push(c);
        let a = src.next_byte()?;
        if lower(a) != b'a' {
            return None;
        }
        w.push(a);
        let n = src.next_byte()?;
        if lower(n) != b'n' {
            return None;
        }
        w.push(n);
        // glibc's scanf does not feed an "(n-char-sequence)" suffix to strtod,
        // so the result is always the default quiet NaN.
        return strtod_prefix(&w);
    }

    // "inf" / "infinity"
    if lower(c) == b'i' {
        w.push(c);
        let n = src.next_byte()?;
        if lower(n) != b'n' {
            return None;
        }
        w.push(n);
        let f = src.next_byte()?;
        if lower(f) != b'f' {
            return None;
        }
        w.push(f);
        if let Some(x) = src.next_byte() {
            if lower(x) == b'i' {
                w.push(x);
                for expect in [b'n', b'i', b't', b'y'] {
                    let y = src.next_byte()?;
                    if lower(y) != expect {
                        return None;
                    }
                    w.push(y);
                }
            }
            // Otherwise the character is pushed back (irrelevant here).
        }
        return strtod_prefix(&w);
    }

    // Ordinary number: look for a "0x"/"0X" prefix first.
    let mut is_hexa = false;
    let mut exp_char = b'e';
    let mut cur: Option<u8> = Some(c);
    let mut got_digit = false;

    if cur == Some(b'0') {
        w.push(b'0');
        cur = src.next_byte();
        match cur {
            Some(x) if lower(x) == b'x' => {
                w.push(x);
                is_hexa = true;
                exp_char = b'p';
                cur = src.next_byte();
            }
            _ => {
                // Maybe it is a hexadecimal digit.
                got_digit = true;
            }
        }
    }

    let mut got_dot = false;
    let mut got_e = false;

    while let Some(ch) = cur {
        if ch.is_ascii_digit() {
            w.push(ch);
            got_digit = true;
        } else if is_hexa && ch.is_ascii_hexdigit() {
            w.push(ch);
            got_digit = true;
        } else if got_e && *w.last().unwrap() == exp_char && (ch == b'-' || ch == b'+') {
            w.push(ch);
        } else if got_digit && !got_e && lower(ch) == exp_char {
            w.push(exp_char);
            got_e = true;
            got_dot = true;
        } else if ch == b'.' && !got_dot {
            w.push(b'.');
            got_dot = true;
        } else {
            break;
        }
        cur = src.next_byte();
    }

    // Nothing at all, or only a "0x" prefix, is a matching failure.
    if w.is_empty() || (is_hexa && w.len() == 2 + got_sign as usize) {
        return None;
    }

    strtod_prefix(&w)
}

// ---------------------------------------------------------------------------
// strtod emulation (longest valid prefix; `None` when nothing is consumed)
// ---------------------------------------------------------------------------

fn starts_with_ci(s: &[u8], pat: &[u8]) -> bool {
    s.len() >= pat.len() && s.iter().zip(pat.iter()).all(|(&a, &b)| lower(a) == b)
}

fn strtod_prefix(s: &[u8]) -> Option<f64> {
    let mut i = 0usize;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let rest = &s[i..];

    if starts_with_ci(rest, b"inf") {
        return Some(if neg {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    if starts_with_ci(rest, b"nan") {
        let bits: u64 = if neg {
            0xfff8_0000_0000_0000
        } else {
            0x7ff8_0000_0000_0000
        };
        return Some(f64::from_bits(bits));
    }

    if rest.len() >= 2 && rest[0] == b'0' && lower(rest[1]) == b'x' {
        return match parse_hex(&rest[2..], neg) {
            Some(v) => Some(v),
            // "0x" without hex digits: only the leading '0' is consumed.
            None => Some(if neg { -0.0f64 } else { 0.0f64 }),
        };
    }

    parse_dec(rest, neg)
}

/// Decimal floating point, delegated to Rust's correctly rounded parser.
fn parse_dec(rest: &[u8], neg: bool) -> Option<f64> {
    let mut i = 0usize;
    let int_start = i;
    while i < rest.len() && rest[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = &rest[int_start..i];

    let mut frac_digits: &[u8] = &[];
    if i < rest.len() && rest[i] == b'.' {
        let j = i + 1;
        let mut k = j;
        while k < rest.len() && rest[k].is_ascii_digit() {
            k += 1;
        }
        if int_digits.is_empty() && k == j {
            return None; // just a '.' -> no conversion
        }
        frac_digits = &rest[j..k];
        i = k;
    }
    if int_digits.is_empty() && frac_digits.is_empty() {
        return None;
    }

    let mut exp_sign = b'+';
    let mut exp_digits: &[u8] = b"0";
    if i < rest.len() && lower(rest[i]) == b'e' {
        let mut k = i + 1;
        let mut sign = b'+';
        if k < rest.len() && (rest[k] == b'+' || rest[k] == b'-') {
            sign = rest[k];
            k += 1;
        }
        let ds = k;
        while k < rest.len() && rest[k].is_ascii_digit() {
            k += 1;
        }
        if k > ds {
            exp_sign = sign;
            exp_digits = &rest[ds..k];
        }
    }

    // Guard against absurdly long exponent digit strings.
    let mut ed: &[u8] = exp_digits;
    while ed.len() > 1 && ed[0] == b'0' {
        ed = &ed[1..];
    }
    let exp_text: Vec<u8> = if ed.len() > 9 {
        b"999999999".to_vec()
    } else {
        ed.to_vec()
    };

    let mut canon: Vec<u8> = Vec::with_capacity(int_digits.len() + frac_digits.len() + 16);
    if neg {
        canon.push(b'-');
    }
    if int_digits.is_empty() {
        canon.push(b'0');
    } else {
        canon.extend_from_slice(int_digits);
    }
    canon.push(b'.');
    if frac_digits.is_empty() {
        canon.push(b'0');
    } else {
        canon.extend_from_slice(frac_digits);
    }
    canon.push(b'e');
    canon.push(exp_sign);
    canon.extend_from_slice(&exp_text);

    let text = std::str::from_utf8(&canon).ok()?;
    match text.parse::<f64>() {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

/// Hexadecimal floating point ("0x" already consumed).
fn parse_hex(rest: &[u8], neg: bool) -> Option<f64> {
    let mut i = 0usize;
    let int_start = i;
    while i < rest.len() && rest[i].is_ascii_hexdigit() {
        i += 1;
    }
    let int_digits = &rest[int_start..i];

    let mut frac_digits: &[u8] = &[];
    if i < rest.len() && rest[i] == b'.' {
        let j = i + 1;
        let mut k = j;
        while k < rest.len() && rest[k].is_ascii_hexdigit() {
            k += 1;
        }
        if int_digits.is_empty() && k == j {
            return None;
        }
        frac_digits = &rest[j..k];
        i = k;
    }
    if int_digits.is_empty() && frac_digits.is_empty() {
        return None;
    }

    let mut pexp: i64 = 0;
    if i < rest.len() && lower(rest[i]) == b'p' {
        let mut k = i + 1;
        let mut eneg = false;
        if k < rest.len() && (rest[k] == b'+' || rest[k] == b'-') {
            eneg = rest[k] == b'-';
            k += 1;
        }
        let ds = k;
        while k < rest.len() && rest[k].is_ascii_digit() {
            k += 1;
        }
        if k > ds {
            let mut v: i64 = 0;
            for &d in &rest[ds..k] {
                v = v.saturating_mul(10).saturating_add((d - b'0') as i64);
                if v > 1 << 40 {
                    v = 1 << 40;
                }
            }
            pexp = if eneg { -v } else { v };
        }
    }

    Some(hex_to_double(int_digits, frac_digits, pexp, neg))
}

fn hex_digit_val(c: u8) -> u128 {
    match c {
        b'0'..=b'9' => (c - b'0') as u128,
        b'a'..=b'f' => (c - b'a' + 10) as u128,
        _ => (c - b'A' + 10) as u128,
    }
}

const SIGN_BIT: u64 = 0x8000_0000_0000_0000;

fn hex_to_double(int_digits: &[u8], frac_digits: &[u8], pexp: i64, neg: bool) -> f64 {
    let sign = if neg { SIGN_BIT } else { 0 };

    // value = <digits> * 2^e2, where the digits are the concatenation of the
    // integer and fractional hex digits.
    let mut e2 = pexp.saturating_sub((frac_digits.len() as i64).saturating_mul(4));
    e2 = e2.clamp(-(1i64 << 50), 1i64 << 50);

    let mut acc: u128 = 0;
    let mut sticky = false;
    let mut extra_exp: i64 = 0;
    let mut seen_nonzero = false;

    for &d in int_digits.iter().chain(frac_digits.iter()) {
        let v = hex_digit_val(d);
        if !seen_nonzero {
            if v == 0 {
                continue; // skip leading zeros
            }
            seen_nonzero = true;
        }
        // Keep the accumulator below 2^124 so that every shift below stays
        // well defined; anything past that only contributes a sticky bit.
        if acc >> 120 == 0 {
            acc = (acc << 4) | v;
        } else {
            extra_exp += 4;
            if v != 0 {
                sticky = true;
            }
        }
    }

    if acc == 0 {
        return f64::from_bits(sign);
    }
    e2 = e2.saturating_add(extra_exp);

    let bl = 128 - acc.leading_zeros() as i64;
    let e_pre = bl - 1 + e2;

    if e_pre > 1024 {
        return f64::from_bits(sign | 0x7ff0_0000_0000_0000);
    }

    let subnormal_target = e_pre < -1022;
    let p: i64 = if subnormal_target { e_pre + 1075 } else { 53 };
    let drop = bl - p;

    // More bits dropped than the significand has: the rounding bit is zero, so
    // the result is zero.
    if drop > bl {
        return f64::from_bits(sign);
    }

    let (q, s) = if drop <= 0 {
        (acc << (-drop) as u32, e2 + drop)
    } else {
        let d = drop as u32;
        let mut quotient = acc >> d;
        let round_bit = (acc >> (d - 1)) & 1;
        let lower_mask = (1u128 << (d - 1)) - 1;
        let rest_nonzero = (acc & lower_mask) != 0 || sticky;
        if round_bit == 1 && (rest_nonzero || (quotient & 1) == 1) {
            quotient += 1;
        }
        (quotient, e2 + drop)
    };

    if q == 0 {
        return f64::from_bits(sign);
    }

    if subnormal_target {
        // s == -1074 here; the IEEE encoding is continuous across the
        // subnormal/normal boundary, so the rounded significand can be used
        // directly as the mantissa field.
        return f64::from_bits(sign | (q as u64));
    }

    let mut qq = q;
    let mut ss = s;
    let bl_q = 128 - qq.leading_zeros() as i64;
    if bl_q == 54 {
        qq >>= 1;
        ss += 1;
    }
    let e_final = 52 + ss;
    if e_final > 1023 {
        return f64::from_bits(sign | 0x7ff0_0000_0000_0000);
    }
    let biased = (e_final + 1023) as u64;
    f64::from_bits(sign | (biased << 52) | ((qq as u64) & 0x000f_ffff_ffff_ffff))
}

// ---------------------------------------------------------------------------
// printf formatting
// ---------------------------------------------------------------------------

/// glibc's "%a" for a double.
fn format_a(f: f64) -> String {
    let bits = f.to_bits();
    let negative = bits & SIGN_BIT != 0;
    let exponent_field = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    if exponent_field == 0x7ff {
        let special = if mantissa == 0 { "inf" } else { "nan" };
        return if negative {
            format!("-{}", special)
        } else {
            special.to_string()
        };
    }

    let leading = if exponent_field == 0 { '0' } else { '1' };
    let (expnegative, exponent) = if exponent_field == 0 {
        if mantissa == 0 {
            (false, 0i64)
        } else {
            (true, 1022i64) // denormalized number
        }
    } else if exponent_field >= 1023 {
        (false, exponent_field - 1023)
    } else {
        (true, 1023 - exponent_field)
    };

    let mut digits = format!("{:013x}", mantissa);
    while digits.ends_with('0') {
        digits.pop();
    }

    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str("0x");
    out.push(leading);
    if !digits.is_empty() {
        out.push('.');
        out.push_str(&digits);
    }
    out.push('p');
    out.push(if expnegative { '-' } else { '+' });
    out.push_str(&exponent.to_string());
    out
}

/// glibc's "%.4f" for a double.
fn format_f4(f: f64) -> String {
    if f.is_nan() {
        return if f.to_bits() & SIGN_BIT != 0 {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if f.is_infinite() {
        return if f < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.4}", f)
}

// ---------------------------------------------------------------------------

fn driver(f: f64) {
    let x = f.to_bits();
    let out = format!("{:x} {} {}\n", x, format_a(f), format_f4(f));
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}

fn main() {
    let mut f: f64 = 0.0;
    let stdin = std::io::stdin();
    let mut src = ByteSource::new(stdin.lock());
    if let Some(v) = scan_double(&mut src) {
        f = v;
    }
    driver(f);
}
