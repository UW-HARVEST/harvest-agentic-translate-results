// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{self, Read, Write};

// ---------------------------------------------------------------------------
// Byte-at-a-time stdin reader with push-back, mirroring the way the C library
// consumes characters for a `scanf` conversion.
// ---------------------------------------------------------------------------

struct Input {
    stdin: io::Stdin,
    pending: Vec<u8>,
    at_eof: bool,
}

impl Input {
    fn new() -> Self {
        Input {
            stdin: io::stdin(),
            pending: Vec::new(),
            at_eof: false,
        }
    }

    fn next(&mut self) -> Option<u8> {
        if let Some(b) = self.pending.pop() {
            return Some(b);
        }
        if self.at_eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.stdin.read(&mut buf) {
                Ok(0) => {
                    self.at_eof = true;
                    return None;
                }
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.at_eof = true;
                    return None;
                }
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.pending.push(b);
    }

    fn unget_opt(&mut self, b: Option<u8>) {
        if let Some(b) = b {
            self.pending.push(b);
        }
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn lower(b: u8) -> u8 {
    b.to_ascii_lowercase()
}

fn hex_val(b: u8) -> Option<u128> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u128),
        b'a'..=b'f' => Some((b - b'a' + 10) as u128),
        b'A'..=b'F' => Some((b - b'A' + 10) as u128),
        _ => None,
    }
}

const NAN_POS_BITS: u64 = 0x7ff8_0000_0000_0000;
const NAN_NEG_BITS: u64 = 0xfff8_0000_0000_0000;

// ---------------------------------------------------------------------------
// scanf("%lf", &f)
//
// Returns Some(value) on a successful conversion, None on matching or input
// failure (in which case the caller leaves the target variable untouched,
// exactly like C).
// ---------------------------------------------------------------------------

fn scan_double(inp: &mut Input) -> Option<f64> {
    // Skip leading white space.
    let mut c = loop {
        match inp.next() {
            None => return None,
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    // Optional sign.
    let mut neg = false;
    if c == b'+' || c == b'-' {
        neg = c == b'-';
        c = match inp.next() {
            Some(b) => b,
            None => return None,
        };
    }

    // "inf" / "infinity" (case insensitive).  Note that glibc's scanf commits
    // to the long spelling as soon as it sees the 'i' of "inity": a partial
    // match such as "infi" is a matching failure (unlike strtod).
    if lower(c) == b'i' {
        for want in [b'n', b'f'] {
            match inp.next() {
                Some(b) if lower(b) == want => {}
                other => {
                    inp.unget_opt(other);
                    return None;
                }
            }
        }
        match inp.next() {
            Some(b) if lower(b) == b'i' => {
                for want in [b'n', b'i', b't', b'y'] {
                    match inp.next() {
                        Some(b) if lower(b) == want => {}
                        other => {
                            inp.unget_opt(other);
                            return None;
                        }
                    }
                }
            }
            other => inp.unget_opt(other),
        }
        return Some(if neg {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }

    // "nan" with an optional parenthesised n-char-sequence.
    if lower(c) == b'n' {
        for want in [b'a', b'n'] {
            match inp.next() {
                Some(b) if lower(b) == want => {}
                other => {
                    inp.unget_opt(other);
                    return None;
                }
            }
        }
        match inp.next() {
            Some(b'(') => loop {
                match inp.next() {
                    Some(b) if b.is_ascii_alphanumeric() || b == b'_' => continue,
                    Some(b')') => break,
                    Some(other) => {
                        inp.unget(other);
                        break;
                    }
                    None => break,
                }
            },
            other => inp.unget_opt(other),
        }
        return Some(f64::from_bits(if neg { NAN_NEG_BITS } else { NAN_POS_BITS }));
    }

    // Hexadecimal form: "0x..." / "0X...".
    if c == b'0' {
        match inp.next() {
            Some(x) if x == b'x' || x == b'X' => {
                return scan_hex(inp, neg);
            }
            other => inp.unget_opt(other),
        }
        // Fall through to the decimal path with the leading '0'.
    }

    scan_decimal(inp, c, neg)
}

fn scan_decimal(inp: &mut Input, first: u8, neg: bool) -> Option<f64> {
    let mut int_digits = String::new();
    let mut frac_digits = String::new();

    let mut cur = Some(first);
    while let Some(b) = cur {
        if b.is_ascii_digit() {
            int_digits.push(b as char);
            cur = inp.next();
        } else {
            break;
        }
    }
    if cur == Some(b'.') {
        cur = inp.next();
        while let Some(b) = cur {
            if b.is_ascii_digit() {
                frac_digits.push(b as char);
                cur = inp.next();
            } else {
                break;
            }
        }
    }

    if int_digits.is_empty() && frac_digits.is_empty() {
        inp.unget_opt(cur);
        return None;
    }

    // Optional decimal exponent; backtracked when no digits follow.
    let mut exp_part = String::new();
    if let Some(b) = cur {
        if b == b'e' || b == b'E' {
            let mut eaten: Vec<u8> = vec![b];
            let mut c2 = inp.next();
            let mut esign = "";
            if let Some(s) = c2 {
                if s == b'+' || s == b'-' {
                    eaten.push(s);
                    if s == b'-' {
                        esign = "-";
                    }
                    c2 = inp.next();
                }
            }
            let mut digits = String::new();
            while let Some(d) = c2 {
                if d.is_ascii_digit() {
                    digits.push(d as char);
                    eaten.push(d);
                    c2 = inp.next();
                } else {
                    break;
                }
            }
            if digits.is_empty() {
                inp.unget_opt(c2);
                for &x in eaten.iter().rev() {
                    inp.unget(x);
                }
            } else {
                exp_part = format!("e{}{}", esign, digits);
                inp.unget_opt(c2);
            }
        } else {
            inp.unget(b);
        }
    }

    let mut text = String::new();
    if int_digits.is_empty() {
        text.push('0');
    } else {
        text.push_str(&int_digits);
    }
    text.push('.');
    if frac_digits.is_empty() {
        text.push('0');
    } else {
        text.push_str(&frac_digits);
    }
    text.push_str(&exp_part);

    let mag = parse_decimal_magnitude(&text);
    Some(if neg { -mag } else { mag })
}

/// Parses a normalised decimal literal ("ddd.ddd[e[-]ddd]") into the nearest
/// f64, saturating absurd exponents the same way the C library does.
fn parse_decimal_magnitude(text: &str) -> f64 {
    if let Ok(v) = text.parse::<f64>() {
        return v;
    }
    // Only an out-of-range exponent can make the above fail; clamp it.
    let (mantissa, exp) = match text.find(['e', 'E']) {
        Some(pos) => (&text[..pos], &text[pos + 1..]),
        None => (text, ""),
    };
    let negexp = exp.starts_with('-');
    let clamped = format!("{}e{}100000", mantissa, if negexp { "-" } else { "" });
    clamped.parse::<f64>().unwrap_or(0.0)
}

/// Scans the part after a "0x" / "0X" prefix.
///
/// glibc's scanf only accepts the prefix when it is followed by at least one
/// hexadecimal digit or by a radix point; "0x" on its own is a matching
/// failure, whereas "0x." converts as the plain "0" that strtod finds.
fn scan_hex(inp: &mut Input, neg: bool) -> Option<f64> {
    // Significand digits.
    let mut mant: u128 = 0;
    let mut sticky = false;
    let mut dropped_bits: i64 = 0;
    let mut frac_count: i64 = 0;
    let mut any_digit = false;

    let mut cur = inp.next();
    let push = |d: u128, mant: &mut u128, sticky: &mut bool, dropped: &mut i64| {
        if (*mant >> 124) != 0 {
            *dropped += 4;
            if d != 0 {
                *sticky = true;
            }
        } else {
            *mant = (*mant << 4) | d;
        }
    };

    while let Some(b) = cur {
        match hex_val(b) {
            Some(d) => {
                any_digit = true;
                push(d, &mut mant, &mut sticky, &mut dropped_bits);
                cur = inp.next();
            }
            None => break,
        }
    }
    if cur == Some(b'.') {
        cur = inp.next();
        while let Some(b) = cur {
            match hex_val(b) {
                Some(d) => {
                    any_digit = true;
                    frac_count += 1;
                    push(d, &mut mant, &mut sticky, &mut dropped_bits);
                    cur = inp.next();
                }
                None => break,
            }
        }
        if !any_digit {
            // "0x." with no digits at all: the subject sequence is just "0".
            inp.unget_opt(cur);
            return Some(if neg { -0.0 } else { 0.0 });
        }
    } else if !any_digit {
        // "0x" not followed by a hex digit or a radix point: matching failure.
        inp.unget_opt(cur);
        return None;
    }

    // Optional binary exponent, backtracked when no digits follow.
    let mut pexp: i64 = 0;
    if let Some(b) = cur {
        if b == b'p' || b == b'P' {
            let mut eaten: Vec<u8> = vec![b];
            let mut c2 = inp.next();
            let mut esign: i64 = 1;
            if let Some(s) = c2 {
                if s == b'+' || s == b'-' {
                    eaten.push(s);
                    if s == b'-' {
                        esign = -1;
                    }
                    c2 = inp.next();
                }
            }
            let mut have = false;
            let mut value: i64 = 0;
            while let Some(d) = c2 {
                if d.is_ascii_digit() {
                    have = true;
                    eaten.push(d);
                    if value < 1_000_000 {
                        value = value * 10 + (d - b'0') as i64;
                    }
                    c2 = inp.next();
                } else {
                    break;
                }
            }
            if !have {
                inp.unget_opt(c2);
                for &x in eaten.iter().rev() {
                    inp.unget(x);
                }
            } else {
                pexp = esign * value;
                inp.unget_opt(c2);
            }
        } else {
            inp.unget(b);
        }
    }

    if mant == 0 {
        return Some(if neg { -0.0 } else { 0.0 });
    }

    let exp2 = pexp - 4 * frac_count + dropped_bits;
    let bits = round_to_double(mant, sticky, exp2);
    let v = f64::from_bits(bits);
    Some(if neg { -v } else { v })
}

/// Rounds (mant + sticky) * 2^exp2 (mant > 0) to the nearest f64 using
/// round-half-to-even, returning the raw bit pattern of the magnitude.
fn round_to_double(mant: u128, sticky: bool, exp2: i64) -> u64 {
    let nbits: i64 = 128 - mant.leading_zeros() as i64;
    let mut shift: i64 = nbits - 53;
    if exp2 + shift < -1074 {
        shift = -1074 - exp2;
    }
    let mut e = exp2 + shift;

    let mut m: u128;
    if shift <= 0 {
        m = mant << ((-shift) as u32);
    } else {
        let s = shift as u32;
        let (q, round_bit, rest) = if s >= 129 {
            (0u128, false, mant != 0 || sticky)
        } else if s == 128 {
            let rb = (mant >> 127) & 1 == 1;
            let rest = (mant & ((1u128 << 127) - 1)) != 0 || sticky;
            (0u128, rb, rest)
        } else {
            let q = mant >> s;
            let rb = (mant >> (s - 1)) & 1 == 1;
            let low_mask = if s == 1 { 0u128 } else { (1u128 << (s - 1)) - 1 };
            let rest = (mant & low_mask) != 0 || sticky;
            (q, rb, rest)
        };
        m = q;
        if round_bit && (rest || (m & 1) == 1) {
            m += 1;
        }
    }
    if m >= (1u128 << 53) {
        m >>= 1;
        e += 1;
    }
    while m < (1u128 << 52) && e > -1074 {
        m <<= 1;
        e -= 1;
    }

    if m == 0 {
        return 0;
    }
    if m < (1u128 << 52) {
        // Subnormal (e == -1074).
        return m as u64;
    }
    let biased = e + 1075;
    if biased >= 2047 {
        return 0x7ff0_0000_0000_0000;
    }
    ((biased as u64) << 52) | ((m as u64) & 0x000f_ffff_ffff_ffff)
}

// ---------------------------------------------------------------------------
// printf("%llx %a %.4f\n", ...)
// ---------------------------------------------------------------------------

fn fmt_hex_float(f: f64) -> String {
    let bits = f.to_bits();
    let sign = if (bits >> 63) != 0 { "-" } else { "" };
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let mant = bits & 0x000f_ffff_ffff_ffff;

    if exp == 0x7ff {
        return if mant == 0 {
            format!("{}inf", sign)
        } else {
            format!("{}nan", sign)
        };
    }
    if exp == 0 && mant == 0 {
        return format!("{}0x0p+0", sign);
    }
    let lead = if exp == 0 { '0' } else { '1' };
    let e = if exp == 0 { -1022 } else { exp - 1023 };
    let digits = format!("{:013x}", mant);
    let trimmed = digits.trim_end_matches('0');
    if trimmed.is_empty() {
        format!("{}0x{}p{:+}", sign, lead, e)
    } else {
        format!("{}0x{}.{}p{:+}", sign, lead, trimmed, e)
    }
}

fn fmt_fixed4(f: f64) -> String {
    if f.is_nan() {
        return if (f.to_bits() >> 63) != 0 {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    format!("{:.4}", f)
}

fn driver(f: f64) {
    let bits = f.to_bits();
    let out = format!("{:x} {} {}\n", bits, fmt_hex_float(f), fmt_fixed4(f));
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}

fn main() {
    let mut f: f64 = 0.0;
    let mut inp = Input::new();
    if let Some(v) = scan_double(&mut inp) {
        f = v;
    }
    driver(f);
}
