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

//! Rust translation of the original C `driver` program.
//!
//! The C original is:
//!
//! ```c
//! typedef union { uint64_t x; double f; } raw_double_t;
//!
//! void driver(double f) {
//!     raw_double_t u = {.f = f};
//!     printf("%llx %a %.4f\n", u.x, f, f);
//! }
//!
//! int main() {
//!     double f = 0.0f;
//!     scanf("%lf", &f);
//!     driver(f);
//!     return 0;
//! }
//! ```
//!
//! The three conversions (`%llx` of the raw bit pattern, glibc's `%a` hexadecimal
//! float form, and `%.4f`) plus glibc's `scanf("%lf")` input grammar are all
//! reproduced byte-for-byte, including the cases where the C behaves oddly
//! (e.g. a bare `"0x"` is a *matching failure* so the destination keeps its
//! initial `0.0`, whereas `"-0x."` succeeds and yields `-0.0`).

use std::io::{self, Read, Write};

// ---------------------------------------------------------------------------
// main / driver
// ---------------------------------------------------------------------------

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs; a C program
/// keeps the default disposition. Without this, a stdout whose reader has gone
/// away makes the C program die from signal 13 while the Rust one would exit 0.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    // `scanf` pulls from stdin; slurping the whole stream is equivalent here
    // because the program terminates right after the single conversion.
    let mut input: Vec<u8> = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    // `double f = 0.0f;` -- stays 0.0 when the conversion does not succeed.
    let mut f: f64 = 0.0;
    if let Some(v) = scanf_lf(&input) {
        f = v;
    }

    driver(f);
}

fn driver(f: f64) {
    // `raw_double_t u = {.f = f};` then `printf("%llx %a %.4f\n", u.x, f, f);`
    let raw: u64 = f.to_bits();
    let line = format!("{:x} {} {}\n", raw, format_hex_float(f), format_fixed_4(f));

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(line.as_bytes());
    let _ = out.flush();
}

// ---------------------------------------------------------------------------
// printf conversions
// ---------------------------------------------------------------------------

/// glibc's `%a` conversion.
///
/// Normals print as `0x1.<mantissa>p<+/-exp>`, subnormals keep their raw
/// mantissa with a leading `0` digit and a fixed `p-1022` exponent, and zero
/// is spelled `0x0p+0`. Trailing zeros of the mantissa are removed, and the
/// `.` disappears entirely when nothing is left.
fn format_hex_float(f: f64) -> String {
    let bits = f.to_bits();
    let negative = (bits >> 63) != 0;
    let exp_field = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;
    let sign = if negative { "-" } else { "" };

    if exp_field == 0x7ff {
        // `%a` shares infinity/NaN spelling with the other float conversions.
        return format!("{}{}", sign, if mantissa == 0 { "inf" } else { "nan" });
    }

    if exp_field == 0 && mantissa == 0 {
        return format!("{}0x0p+0", sign);
    }

    let (lead, exp) = if exp_field == 0 {
        (0, -1022) // subnormal: not renormalized by glibc
    } else {
        (1, exp_field - 1023)
    };

    // 52 mantissa bits == 13 hex digits, trailing zeros trimmed.
    let mut digits = format!("{:013x}", mantissa);
    while digits.ends_with('0') {
        digits.pop();
    }

    let mut s = String::with_capacity(24);
    s.push_str(sign);
    s.push_str("0x");
    s.push(if lead == 0 { '0' } else { '1' });
    if !digits.is_empty() {
        s.push('.');
        s.push_str(&digits);
    }
    s.push('p');
    s.push(if exp < 0 { '-' } else { '+' });
    s.push_str(&exp.abs().to_string());
    s
}

/// glibc's `%.4f` conversion.
///
/// Rust's `{:.4}` already emits the exactly-rounded decimal expansion with
/// ties resolved to even, which matches glibc under the default rounding mode;
/// only the non-finite spellings differ.
fn format_fixed_4(f: f64) -> String {
    if f.is_nan() {
        return if (f.to_bits() >> 63) != 0 {
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
// scanf("%lf", ...)
// ---------------------------------------------------------------------------

/// True for the characters `isspace` accepts in the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn hex_value(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        _ => b - b'A' + 10,
    }
}

fn starts_with_ci(s: &[u8], word: &[u8]) -> bool {
    s.len() >= word.len()
        && s[..word.len()]
            .iter()
            .zip(word)
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

fn signed_zero(negative: bool) -> f64 {
    if negative {
        -0.0
    } else {
        0.0
    }
}

fn signed_inf(negative: bool) -> f64 {
    if negative {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    }
}

/// glibc's default quiet NaN, with the sign taken from the input.
fn signed_nan(negative: bool) -> f64 {
    let bits: u64 = 0x7ff8_0000_0000_0000 | if negative { 1u64 << 63 } else { 0 };
    f64::from_bits(bits)
}

/// Performs one `%lf` conversion. `None` means the conversion did not store a
/// value (matching or input failure), so the caller keeps its initial value.
fn scanf_lf(input: &[u8]) -> Option<f64> {
    let mut i = 0usize;
    while i < input.len() && is_c_space(input[i]) {
        i += 1;
    }
    let s = &input[i..];

    let mut p = 0usize;
    let mut negative = false;
    if p < s.len() && (s[p] == b'+' || s[p] == b'-') {
        negative = s[p] == b'-';
        p += 1;
    }
    let body = &s[p..];

    // "infinity" before "inf" so the longer spelling wins.
    if starts_with_ci(body, b"infinity") {
        return Some(signed_inf(negative));
    }
    if starts_with_ci(body, b"inf") {
        // Unlike strtod, scanf cannot push back a partially matched
        // "infinity": once a fourth 'i' has been consumed the whole spelling
        // is required, otherwise the conversion is a matching failure.
        if matches!(body.get(3), Some(&c) if c | 0x20 == b'i') {
            return None;
        }
        return Some(signed_inf(negative));
    }
    // A parenthesized payload after "nan" is consumed but ignored by scanf.
    if starts_with_ci(body, b"nan") {
        return Some(signed_nan(negative));
    }

    if body.len() >= 2 && body[0] == b'0' && (body[1] | 0x20) == b'x' {
        // glibc's scanf reports a matching failure for a "0x" prefix that is
        // not followed by a hex digit or a decimal point -- it does not fall
        // back to reading just the leading "0".
        match body.get(2) {
            Some(&c) if c.is_ascii_hexdigit() || c == b'.' => {}
            _ => return None,
        }
        return Some(parse_hex_float(&body[2..], negative));
    }

    parse_decimal_float(body, negative)
}

/// Parses the longest valid decimal floating-point prefix of `s`.
fn parse_decimal_float(s: &[u8], negative: bool) -> Option<f64> {
    let mut p = 0usize;
    let mut digits = 0usize;

    while p < s.len() && s[p].is_ascii_digit() {
        p += 1;
        digits += 1;
    }
    if p < s.len() && s[p] == b'.' {
        p += 1;
        while p < s.len() && s[p].is_ascii_digit() {
            p += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }

    // The exponent only counts if at least one digit follows it.
    let mut end = p;
    if p < s.len() && (s[p] | 0x20) == b'e' {
        let mut q = p + 1;
        if q < s.len() && (s[q] == b'+' || s[q] == b'-') {
            q += 1;
        }
        if q < s.len() && s[q].is_ascii_digit() {
            while q < s.len() && s[q].is_ascii_digit() {
                q += 1;
            }
            end = q;
        }
    }

    // The accepted prefix is exactly the grammar `f64::from_str` understands,
    // and both it and glibc's strtod round correctly to nearest-even.
    let text = std::str::from_utf8(&s[..end]).ok()?;
    let magnitude: f64 = text.parse().ok()?;
    Some(if negative { -magnitude } else { magnitude })
}

/// Parses the longest valid hexadecimal floating-point prefix of `s`, which
/// starts just past the `0x` prefix.
fn parse_hex_float(s: &[u8], negative: bool) -> f64 {
    let mut p = 0usize;
    let mut digits: Vec<u8> = Vec::new();

    while p < s.len() && s[p].is_ascii_hexdigit() {
        digits.push(hex_value(s[p]));
        p += 1;
    }
    let mut frac_digits: i64 = 0;
    if p < s.len() && s[p] == b'.' {
        p += 1;
        while p < s.len() && s[p].is_ascii_hexdigit() {
            digits.push(hex_value(s[p]));
            frac_digits += 1;
            p += 1;
        }
    }

    // No hex digits at all: strtod backtracks to the leading "0" of "0x".
    if digits.is_empty() {
        return signed_zero(negative);
    }

    let mut binary_exp: i64 = 0;
    if p < s.len() && (s[p] | 0x20) == b'p' {
        let mut q = p + 1;
        let mut exp_negative = false;
        if q < s.len() && (s[q] == b'+' || s[q] == b'-') {
            exp_negative = s[q] == b'-';
            q += 1;
        }
        if q < s.len() && s[q].is_ascii_digit() {
            let mut value: i64 = 0;
            while q < s.len() && s[q].is_ascii_digit() {
                if value < 1_000_000 {
                    value = value * 10 + i64::from(s[q] - b'0');
                }
                q += 1;
            }
            binary_exp = if exp_negative { -value } else { value };
        }
    }

    // value == digits(base 16) * 2^(binary_exp - 4 * frac_digits)
    let first_significant = digits.iter().position(|&d| d != 0);
    let significant = match first_significant {
        Some(k) => &digits[k..],
        None => return signed_zero(negative),
    };

    // Keep the top 30 hex digits (120 bits, far more than the 53 needed) and
    // fold everything below into a sticky bit.
    const KEEP: usize = 30;
    let mut mantissa: u128 = 0;
    let mut sticky = false;
    for (idx, &d) in significant.iter().enumerate() {
        if idx < KEEP {
            mantissa = (mantissa << 4) | u128::from(d);
        } else if d != 0 {
            sticky = true;
        }
    }
    let dropped = significant.len().saturating_sub(KEEP) as i128;

    let exp = i128::from(binary_exp) - 4 * i128::from(frac_digits) + 4 * dropped;
    if exp > 2_000 {
        return signed_inf(negative);
    }
    if exp < -3_000 {
        return signed_zero(negative);
    }

    assemble_f64(negative, mantissa, exp as i32, sticky)
}

/// Rounds `mantissa * 2^exp` (with `sticky` marking discarded nonzero low
/// bits) to the nearest `f64`, ties to even.
fn assemble_f64(negative: bool, mantissa: u128, exp: i32, sticky: bool) -> f64 {
    let sign_bit: u64 = if negative { 1u64 << 63 } else { 0 };
    if mantissa == 0 {
        return f64::from_bits(sign_bit);
    }

    let bit_len = (128 - mantissa.leading_zeros()) as i32;
    let value_exp = exp + bit_len - 1; // value is in [2^value_exp, 2^(value_exp+1))

    if value_exp > 1023 {
        return f64::from_bits(sign_bit | 0x7ff0_0000_0000_0000);
    }
    if value_exp < -1075 {
        return f64::from_bits(sign_bit);
    }

    let subnormal = value_exp < -1022;
    // Position of the least significant bit we get to keep.
    let shift: i32 = if subnormal { -1074 - exp } else { bit_len - 53 };

    let quantum: u128 = if shift <= 0 {
        mantissa << ((-shift) as u32)
    } else {
        let s = shift as u32;
        let truncated = if s >= 128 { 0 } else { mantissa >> s };
        let round_bit = s <= 128 && (mantissa >> (s - 1)) & 1 == 1;
        let below_round = if s <= 1 {
            false
        } else if s - 1 >= 128 {
            mantissa != 0
        } else {
            (mantissa & ((1u128 << (s - 1)) - 1)) != 0
        };
        let inexact = below_round || sticky;
        if round_bit && (inexact || (truncated & 1) == 1) {
            truncated + 1
        } else {
            truncated
        }
    };

    if subnormal {
        // The quantum counts multiples of 2^-1074, so it doubles as the raw
        // bit pattern -- including when rounding pushed it up to 2^52, which
        // is exactly the smallest normal.
        return f64::from_bits(sign_bit | (quantum as u64));
    }

    let mut significand = quantum;
    let mut lsb_exp = exp + shift;
    if significand == 1u128 << 53 {
        significand >>= 1;
        lsb_exp += 1;
    }
    let biased = lsb_exp + 52 + 1023;
    if biased >= 0x7ff {
        return f64::from_bits(sign_bit | 0x7ff0_0000_0000_0000);
    }
    let bits = sign_bit
        | ((biased as u64) << 52)
        | ((significand as u64) & 0x000f_ffff_ffff_ffff);
    f64::from_bits(bits)
}
