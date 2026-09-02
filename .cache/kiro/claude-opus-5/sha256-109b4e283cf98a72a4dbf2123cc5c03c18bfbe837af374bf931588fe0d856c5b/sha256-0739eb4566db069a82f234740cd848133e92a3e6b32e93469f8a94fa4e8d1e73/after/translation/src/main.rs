// Rust translation of c_src/src/main.c
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

use std::io::{Read, Write};

/// static void print_hex(unsigned char *p, int len)
fn print_hex(p: &[u8]) {
    let mut out = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        // printf("%02x", p[i]);
        out.push_str(&format!("{:02x}", b));
    }
    // printf("\n");
    out.push('\n');
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}

/// void driver(float x)
fn driver(x: f32) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    // Native byte order, exactly like memcpy of the object representation.
    let raw: [u8; 4] = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    // float x = 0.f;
    let mut x: f32 = 0.0;

    // scanf("%f", &x);
    // On matching failure or input failure, `x` is left untouched (0.0f).
    let mut input: Vec<u8> = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    if let Some(v) = scan_f(&input) {
        x = v;
    }

    // driver(x);
    driver(x);
    // return 0;
}

// ---------------------------------------------------------------------------
// scanf("%f", ...) emulation
//
// The conversion skips leading whitespace, then consumes the longest initial
// subsequence of the input that has the form of a strtof() subject sequence.
// If no such sequence exists the conversion fails and nothing is stored.
// ---------------------------------------------------------------------------

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn lower(b: u8) -> u8 {
    if b.is_ascii_uppercase() {
        b + 32
    } else {
        b
    }
}

fn hex_val(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a' + 10) as u32),
        b'A'..=b'F' => Some((b - b'A' + 10) as u32),
        _ => None,
    }
}

fn starts_with_ci(s: &[u8], pat: &[u8]) -> bool {
    s.len() >= pat.len() && s[..pat.len()].iter().zip(pat).all(|(a, b)| lower(*a) == *b)
}

fn scan_f(input: &[u8]) -> Option<f32> {
    let mut i = 0usize;

    // Leading whitespace is skipped by the %f directive.
    while i < input.len() && is_c_space(input[i]) {
        i += 1;
    }
    if i >= input.len() {
        return None; // input failure: nothing stored
    }

    // Optional sign.
    let mut negative = false;
    if input[i] == b'+' || input[i] == b'-' {
        negative = input[i] == b'-';
        i += 1;
    }
    let rest = &input[i..];

    // "infinity" / "inf". scanf's scanner commits to "infinity" as soon as it
    // sees a fourth 'i', so a truncated "infin..." is a matching failure
    // rather than a successful "inf".
    if starts_with_ci(rest, b"infinity") {
        let bits: u32 = 0x7f80_0000 | (if negative { 0x8000_0000 } else { 0 });
        return Some(f32::from_bits(bits));
    }
    if starts_with_ci(rest, b"inf") {
        if rest.len() > 3 && lower(rest[3]) == b'i' {
            return None;
        }
        let bits: u32 = 0x7f80_0000 | (if negative { 0x8000_0000 } else { 0 });
        return Some(f32::from_bits(bits));
    }

    // "nan". Note that scanf's own float scanner never forwards a
    // parenthesised n-char-sequence payload to strtof, so a payload such as
    // "nan(1)" still yields the default quiet NaN.
    if starts_with_ci(rest, b"nan") {
        let mut bits: u32 = 0x7fc0_0000; // default quiet NaN
        if negative {
            bits |= 0x8000_0000;
        }
        return Some(f32::from_bits(bits));
    }

    // Hexadecimal form. Once the "0x"/"0X" prefix has been consumed the
    // scanner is committed to it: a significand consisting of at least one hex
    // digit or a radix point is required, otherwise the whole conversion fails
    // (it does *not* fall back to matching just the leading "0").
    if rest.len() >= 2 && rest[0] == b'0' && (rest[1] == b'x' || rest[1] == b'X') {
        if rest.len() >= 3 && (hex_val(rest[2]).is_some() || rest[2] == b'.') {
            return Some(parse_hex_float(&rest[2..], negative));
        }
        return None;
    }

    // Decimal form.
    let mut j = 0usize;
    let int_start = 0usize;
    let mut digits = 0usize;
    while j < rest.len() && rest[j].is_ascii_digit() {
        j += 1;
        digits += 1;
    }
    let int_end = j;
    let mut frac_start = j;
    let mut frac_end = j;
    if j < rest.len() && rest[j] == b'.' {
        j += 1;
        frac_start = j;
        while j < rest.len() && rest[j].is_ascii_digit() {
            j += 1;
            digits += 1;
        }
        frac_end = j;
    }
    if digits == 0 {
        return None; // matching failure: nothing stored
    }
    // Optional exponent part, only consumed when it is complete.
    let mut dec_exp: i64 = 0;
    if j < rest.len() && (rest[j] == b'e' || rest[j] == b'E') {
        let mut k = j + 1;
        let mut eneg = false;
        if k < rest.len() && (rest[k] == b'+' || rest[k] == b'-') {
            eneg = rest[k] == b'-';
            k += 1;
        }
        let mut v: i64 = 0;
        let mut exp_digits = 0usize;
        while k < rest.len() && rest[k].is_ascii_digit() {
            v = v.saturating_mul(10).saturating_add((rest[k] - b'0') as i64);
            if v > 1_000_000_000 {
                v = 1_000_000_000;
            }
            k += 1;
            exp_digits += 1;
        }
        if exp_digits > 0 {
            dec_exp = if eneg { -v } else { v };
        }
    }

    Some(decimal_to_f32(
        negative,
        &rest[int_start..int_end],
        &rest[frac_start..frac_end],
        dec_exp,
    ))
}

/// Correctly rounded decimal -> f32, matching strtof() for arbitrarily long
/// significands and arbitrarily large exponents.
fn decimal_to_f32(negative: bool, int_part: &[u8], frac_part: &[u8], exp: i64) -> f32 {
    let sign_bit: u32 = if negative { 0x8000_0000 } else { 0 };

    // value == digits * 10^(exp - frac_part.len())
    let mut digits: Vec<u8> = Vec::with_capacity(int_part.len() + frac_part.len());
    digits.extend_from_slice(int_part);
    digits.extend_from_slice(frac_part);
    let mut dexp: i64 = exp.saturating_sub(frac_part.len() as i64);

    // Strip leading zeros.
    let lead = digits.iter().take_while(|&&b| b == b'0').count();
    digits.drain(..lead);
    if digits.is_empty() {
        return f32::from_bits(sign_bit); // (signed) zero
    }
    // Strip trailing zeros (exact transformation).
    while digits.len() > 1 && *digits.last().unwrap() == b'0' {
        digits.pop();
        dexp = dexp.saturating_add(1);
    }

    // Truncate an over-long significand, keeping a sticky digit. The cut-off is
    // far beyond the longest exact halfway case of a binary32 value, so this
    // never changes the rounding direction.
    const KEEP: usize = 800;
    if digits.len() > KEEP {
        let dropped = digits.len() - KEEP;
        let sticky = digits[KEEP..].iter().any(|&b| b != b'0');
        digits.truncate(KEEP);
        dexp = dexp.saturating_add(dropped as i64);
        if sticky {
            digits.push(b'1');
            dexp = dexp.saturating_sub(1);
        }
    }

    // Decimal magnitude: 10^(mag-1) <= value < 10^mag
    let mag = dexp.saturating_add(digits.len() as i64);
    if mag > 60 {
        return f32::from_bits(sign_bit | 0x7f80_0000); // certain overflow
    }
    if mag < -60 {
        return f32::from_bits(sign_bit); // certain underflow to zero
    }

    let mut text = String::with_capacity(digits.len() + 24);
    if negative {
        text.push('-');
    }
    // digits are ASCII decimal digits only
    text.push_str(std::str::from_utf8(&digits).unwrap());
    text.push('e');
    text.push_str(&dexp.to_string());
    // Rust's decimal parser is correctly rounded, matching strtof().
    text.parse::<f32>().unwrap_or(f32::from_bits(sign_bit))
}

/// Parses a hexadecimal floating literal (input starts right after "0x")
/// with correct round-to-nearest-even rounding into f32.
fn parse_hex_float(s: &[u8], negative: bool) -> f32 {
    let mut mant: u128 = 0;
    let mut skipped: i64 = 0; // significand digits dropped on the right
    let mut frac_digits: i64 = 0;
    let mut seen_point = false;
    let mut sticky = false;
    let mut i = 0usize;

    while i < s.len() {
        let b = s[i];
        if b == b'.' && !seen_point {
            seen_point = true;
            i += 1;
            continue;
        }
        let d = match hex_val(b) {
            Some(d) => d,
            None => break,
        };
        if seen_point {
            frac_digits += 1;
        }
        if mant <= (u128::MAX >> 4) {
            mant = mant * 16 + d as u128;
        } else {
            if d != 0 {
                sticky = true;
            }
            skipped += 1;
        }
        i += 1;
    }

    // Optional binary exponent part, consumed only when complete.
    let mut pexp: i64 = 0;
    if i < s.len() && (s[i] == b'p' || s[i] == b'P') {
        let mut k = i + 1;
        let mut eneg = false;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            eneg = s[k] == b'-';
            k += 1;
        }
        let mut v: i64 = 0;
        let mut n = 0usize;
        while k < s.len() && s[k].is_ascii_digit() {
            v = v.saturating_mul(10).saturating_add((s[k] - b'0') as i64);
            if v > 1_000_000_000 {
                v = 1_000_000_000;
            }
            k += 1;
            n += 1;
        }
        if n > 0 {
            pexp = if eneg { -v } else { v };
        }
    }

    let sign_bit: u32 = if negative { 0x8000_0000 } else { 0 };

    if mant == 0 {
        return f32::from_bits(sign_bit);
    }

    // value == mant * 2^exp2 (plus a sticky low remainder)
    let exp2: i64 = 4 * skipped - 4 * frac_digits + pexp;

    let nbits: i64 = (128 - mant.leading_zeros()) as i64;
    let lead_exp = exp2.saturating_add(nbits - 1);

    // Fast rejections to keep the shift arithmetic in range.
    if lead_exp > 128 {
        return f32::from_bits(sign_bit | 0x7f80_0000); // overflow -> inf
    }
    if lead_exp < -200 {
        return f32::from_bits(sign_bit); // underflow -> zero
    }

    let mut target_e = std::cmp::max(lead_exp - 23, -149);
    let shift = target_e - exp2;

    let mut m: u128;
    if shift <= 0 {
        m = mant << ((-shift) as u32);
    } else if shift >= 128 {
        m = 0;
        sticky = sticky || mant != 0;
        // everything shifted out; round based on comparison with half
        let half_pos = shift - 1;
        let round_up = if half_pos < 128 {
            let half_bit = (mant >> (half_pos as u32)) & 1;
            let low_mask = (1u128 << (half_pos as u32)) - 1;
            let low = mant & low_mask;
            half_bit == 1 && (low != 0 || sticky)
        } else {
            false
        };
        if round_up {
            m = 1;
        }
        return encode(sign_bit, m, target_e);
    } else {
        let sh = shift as u32;
        let dropped_mask = (1u128 << sh) - 1;
        let dropped = mant & dropped_mask;
        m = mant >> sh;
        let half = 1u128 << (sh - 1);
        let round_up = if dropped > half {
            true
        } else if dropped == half {
            sticky || (m & 1) == 1
        } else {
            false
        };
        if round_up {
            m += 1;
        }
        // Rounding may carry into the next binade.
        if m >= (1u128 << 24) {
            m >>= 1;
            target_e += 1;
        }
        return encode(sign_bit, m, target_e);
    }

    // shift <= 0 path: exact, no rounding needed.
    if m >= (1u128 << 24) {
        // Cannot happen (target_e was chosen to keep 24 bits), but stay safe.
        while m >= (1u128 << 24) {
            m >>= 1;
            target_e += 1;
        }
    }
    encode(sign_bit, m, target_e)
}

fn encode(sign_bit: u32, m: u128, target_e: i64) -> f32 {
    if m == 0 {
        return f32::from_bits(sign_bit);
    }
    if m >= (1u128 << 23) {
        let exp_field = target_e + 150;
        if exp_field >= 255 {
            return f32::from_bits(sign_bit | 0x7f80_0000);
        }
        if exp_field <= 0 {
            // Should not happen for normalized m; fall back to zero.
            return f32::from_bits(sign_bit);
        }
        let bits = sign_bit | ((exp_field as u32) << 23) | ((m as u32) & 0x7f_ffff);
        f32::from_bits(bits)
    } else {
        // Subnormal: target_e == -149
        f32::from_bits(sign_bit | (m as u32))
    }
}
