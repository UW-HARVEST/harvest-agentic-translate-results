// Rust translation of c_src/src/main.c
//
// The C program is:
//
//     int main() {
//         float x = 0.f;
//         scanf("%f", &x);
//         driver(x);          /* prints the 4 bytes of `x` as lowercase hex */
//         return 0;
//     }
//
// Behaviour that has to be reproduced byte for byte:
//   * `scanf("%f", &x)` skips leading whitespace (including newlines), then
//     consumes the longest sequence of characters that can be part of a
//     floating point number and converts it with `strtof`.
//   * On a matching/input failure `x` keeps its initial value of `+0.0f`.
//   * The bytes of the resulting `float` are printed in memory order with
//     "%02x" followed by a single '\n'.
//
// Bugs / quirks of the C library implementation are intentionally preserved
// (e.g. `"0x"` without hex digits is a matching failure, `"nan(1)"` is read as
// a plain quiet NaN because scanf does not consume the payload, an incomplete
// `"infin"` is an input failure, ...).

use std::io::{self, BufReader, Read, Stdin, Write};

// ---------------------------------------------------------------------------
// stdin, read one byte at a time (mirrors C's getc based scanf)
// ---------------------------------------------------------------------------

struct ByteReader {
    inner: BufReader<Stdin>,
}

impl ByteReader {
    fn new() -> Self {
        ByteReader {
            inner: BufReader::new(io::stdin()),
        }
    }

    /// Equivalent of C's `getc`; `None` stands for `EOF`.
    fn getc(&mut self) -> Option<u8> {
        let mut b = [0u8; 1];
        loop {
            match self.inner.read(&mut b) {
                Ok(0) => return None,
                Ok(_) => return Some(b[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }
}

/// `isspace()` in the "C" locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn lower(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a' + 10) as u32),
        b'A'..=b'F' => Some((c - b'A' + 10) as u32),
        _ => None,
    }
}

fn signed_zero(neg: bool) -> f32 {
    if neg {
        -0.0f32
    } else {
        0.0f32
    }
}

fn signed_inf(neg: bool) -> f32 {
    if neg {
        f32::NEG_INFINITY
    } else {
        f32::INFINITY
    }
}

/// Quiet NaN exactly as produced by glibc's `strtof` ("nan" / "-nan").
fn signed_nan(neg: bool) -> f32 {
    if neg {
        f32::from_bits(0xffc0_0000)
    } else {
        f32::from_bits(0x7fc0_0000)
    }
}

// ---------------------------------------------------------------------------
// scanf("%f") emulation
// ---------------------------------------------------------------------------

/// Collects the characters accepted by the `%f` conversion and converts them.
///
/// Returns `None` for a matching/input failure, in which case the C code leaves
/// its `float` untouched.
fn scan_float(rdr: &mut ByteReader) -> Option<f32> {
    // Skip leading whitespace.  EOF while skipping is an input failure.
    let mut c = loop {
        match rdr.getc() {
            None => return None,
            Some(ch) => {
                if !is_space(ch) {
                    break ch;
                }
            }
        }
    };

    let mut buf: Vec<u8> = Vec::new();
    let mut sign_len = 0usize;

    // Optional sign.
    if c == b'-' || c == b'+' {
        buf.push(c);
        sign_len = 1;
        c = rdr.getc()?; // EOF right after the sign -> failure
    }

    // "nan"
    if lower(c) == b'n' {
        buf.push(c);
        let a = rdr.getc()?;
        if lower(a) != b'a' {
            return None;
        }
        buf.push(a);
        let n = rdr.getc()?;
        if lower(n) != b'n' {
            return None;
        }
        buf.push(n);
        // scanf does not consume a "(n-char-sequence)" payload.
        return Some(c_strtof(&buf));
    }

    // "inf" / "infinity"
    if lower(c) == b'i' {
        buf.push(c);
        let n = rdr.getc()?;
        if lower(n) != b'n' {
            return None;
        }
        buf.push(n);
        let f = rdr.getc()?;
        if lower(f) != b'f' {
            return None;
        }
        buf.push(f);
        // Optionally the rest of "infinity".
        if let Some(ch) = rdr.getc() {
            if lower(ch) == b'i' {
                buf.push(ch);
                for want in [b'n', b'i', b't', b'y'] {
                    let d = rdr.getc()?;
                    if lower(d) != want {
                        return None;
                    }
                    buf.push(d);
                }
            }
            // else: the character is pushed back (nothing else reads stdin).
        }
        return Some(c_strtof(&buf));
    }

    // Ordinary number.
    let mut got_digit = false;
    let mut got_dot = false;
    let mut got_e = false;
    let mut hexa = false;
    let mut exp_char = b'e';

    let mut cur = Some(c);

    if c == b'0' {
        buf.push(c);
        got_digit = true;
        cur = rdr.getc();
        if let Some(ch) = cur {
            if lower(ch) == b'x' {
                buf.push(ch);
                hexa = true;
                exp_char = b'p';
                got_digit = false;
                cur = rdr.getc();
            }
        }
    }

    while let Some(ch) = cur {
        if ch.is_ascii_digit() {
            buf.push(ch);
            got_digit = true;
        } else if !got_e && hexa && hex_val(ch).is_some() {
            buf.push(ch);
            got_digit = true;
        } else if got_e && buf.last() == Some(&exp_char) && (ch == b'-' || ch == b'+') {
            buf.push(ch);
        } else if got_digit && !got_e && lower(ch) == exp_char {
            // glibc stores the lowercase exponent character.
            buf.push(exp_char);
            got_e = true;
            got_dot = true;
        } else if ch == b'.' && !got_dot {
            buf.push(ch);
            got_dot = true;
        } else {
            break;
        }
        cur = rdr.getc();
    }

    // Nothing usable read, or only the "0x" prefix of a hex float.
    if buf.len() == sign_len {
        return None;
    }
    if hexa && buf.len() == sign_len + 2 {
        return None;
    }

    Some(c_strtof(&buf))
}

// ---------------------------------------------------------------------------
// strtof() emulation (longest valid prefix, correctly rounded)
// ---------------------------------------------------------------------------

fn starts_with_ci(s: &[u8], pat: &[u8]) -> bool {
    s.len() >= pat.len() && s[..pat.len()].iter().zip(pat).all(|(a, b)| lower(*a) == *b)
}

fn c_strtof(s: &[u8]) -> f32 {
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

    let rest = &s[i..];
    if starts_with_ci(rest, b"infinity") || starts_with_ci(rest, b"inf") {
        return signed_inf(neg);
    }
    if starts_with_ci(rest, b"nan") {
        return signed_nan(neg);
    }

    // Hexadecimal floating point?
    if i + 1 < len && s[i] == b'0' && lower(s[i + 1]) == b'x' {
        let mut j = i + 2;
        let mut mant: u64 = 0;
        let mut sticky = false;
        let mut exp2: i64 = 0;
        let mut any_digit = false;
        let mut seen_dot = false;

        while j < len {
            let ch = s[j];
            if let Some(d) = hex_val(ch) {
                any_digit = true;
                if mant >> 59 != 0 {
                    // No room left: the digit only contributes to the sticky
                    // bit (and to the exponent when left of the point).
                    if d != 0 {
                        sticky = true;
                    }
                    if !seen_dot {
                        exp2 += 4;
                    }
                } else {
                    mant = (mant << 4) | d as u64;
                    if seen_dot {
                        exp2 -= 4;
                    }
                }
                j += 1;
            } else if ch == b'.' && !seen_dot {
                seen_dot = true;
                j += 1;
            } else {
                break;
            }
        }

        if any_digit {
            // Optional binary exponent, only valid with at least one digit.
            if j < len && lower(s[j]) == b'p' {
                let mut k = j + 1;
                let mut eneg = false;
                if k < len && (s[k] == b'+' || s[k] == b'-') {
                    eneg = s[k] == b'-';
                    k += 1;
                }
                let dstart = k;
                let mut e: i64 = 0;
                while k < len && s[k].is_ascii_digit() {
                    if e < 1_000_000 {
                        e = e * 10 + (s[k] - b'0') as i64;
                    }
                    k += 1;
                }
                if k > dstart {
                    exp2 += if eneg { -e } else { e };
                }
            }
            return assemble_f32(neg, mant, exp2, sticky);
        }
        // "0x" without hex digits: only the leading "0" is converted.
        return signed_zero(neg);
    }

    // Decimal.
    let int_start = i;
    while i < len && s[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = &s[int_start..i];

    let mut frac_digits: &[u8] = &[];
    if i < len && s[i] == b'.' {
        let fs = i + 1;
        let mut j = fs;
        while j < len && s[j].is_ascii_digit() {
            j += 1;
        }
        if !int_digits.is_empty() || j > fs {
            frac_digits = &s[fs..j];
            i = j;
        }
    }

    if int_digits.is_empty() && frac_digits.is_empty() {
        // No conversion performed at all -> strtof returns +0.0.
        return 0.0f32;
    }

    let mut exp10: i64 = 0;
    if i < len && lower(s[i]) == b'e' {
        let mut j = i + 1;
        let mut eneg = false;
        if j < len && (s[j] == b'+' || s[j] == b'-') {
            eneg = s[j] == b'-';
            j += 1;
        }
        let dstart = j;
        let mut e: i64 = 0;
        while j < len && s[j].is_ascii_digit() {
            if e < 1_000_000_000 {
                e = e * 10 + (s[j] - b'0') as i64;
            }
            j += 1;
        }
        if j > dstart {
            exp10 = if eneg { -e } else { e };
        }
    }

    // All digits, decimal point removed.
    let mut all: Vec<u8> = Vec::with_capacity(int_digits.len() + frac_digits.len());
    all.extend_from_slice(int_digits);
    all.extend_from_slice(frac_digits);

    let lz = all.iter().take_while(|&&d| d == b'0').count();
    let mut sig_end = all.len();
    while sig_end > lz && all[sig_end - 1] == b'0' {
        sig_end -= 1;
    }
    let sig = &all[lz..sig_end];

    if sig.is_empty() {
        return signed_zero(neg);
    }

    // value == 0.<sig> * 10^dp
    let dp: i128 = exp10 as i128 + int_digits.len() as i128 - lz as i128;
    if dp > 45 {
        return signed_inf(neg);
    }
    if dp < -50 {
        return signed_zero(neg);
    }

    let mut text = String::with_capacity(sig.len() + 16);
    text.push_str("0.");
    text.push_str(std::str::from_utf8(sig).unwrap_or("0"));
    text.push('e');
    text.push_str(&dp.to_string());

    let v: f32 = text.parse().unwrap_or(0.0f32);
    if neg {
        -v
    } else {
        v
    }
}

/// Rounds `mant * 2^exp2` (plus a non representable remainder if `sticky`) to
/// the nearest `f32`, ties to even.
fn assemble_f32(neg: bool, mut mant: u64, mut exp2: i64, mut sticky: bool) -> f32 {
    if mant == 0 {
        return signed_zero(neg);
    }

    // Normalise the mantissa to exactly 40 significant bits.
    let bl = 64 - mant.leading_zeros() as i64;
    const TARGET: i64 = 40;
    if bl < TARGET {
        let shift = (TARGET - bl) as u32;
        mant <<= shift;
        exp2 -= shift as i64;
    } else if bl > TARGET {
        let shift = (bl - TARGET) as u32;
        if mant & ((1u64 << shift) - 1) != 0 {
            sticky = true;
        }
        mant >>= shift;
        exp2 += shift as i64;
    }

    // Exponent of the most significant bit.
    let e_val = exp2 + (TARGET - 1);
    if e_val > 127 {
        return signed_inf(neg);
    }
    if e_val < -200 {
        return signed_zero(neg);
    }

    let mut ulp_exp = if e_val - 23 > -149 { e_val - 23 } else { -149 };
    let shift = ulp_exp - exp2; // always >= 1 here
    if shift >= 64 {
        return signed_zero(neg);
    }
    let shift = shift as u32;

    let low = mant & ((1u64 << shift) - 1);
    let mut q = mant >> shift;
    let half = 1u64 << (shift - 1);
    let round_up = if low > half {
        true
    } else if low < half {
        false
    } else {
        sticky || (q & 1) == 1
    };
    if round_up {
        q += 1;
    }

    if q == 0 {
        return signed_zero(neg);
    }

    let bits: u32;
    if ulp_exp == -149 && q < (1u64 << 23) {
        bits = q as u32; // subnormal
    } else {
        if q >= (1u64 << 24) {
            q >>= 1;
            ulp_exp += 1;
        }
        let e = ulp_exp + 23;
        if e > 127 {
            return signed_inf(neg);
        }
        bits = (((e + 127) as u32) << 23) | ((q as u32) & 0x007f_ffff);
    }

    let v = f32::from_bits(bits);
    if neg {
        -v
    } else {
        v
    }
}

// ---------------------------------------------------------------------------
// the program itself
// ---------------------------------------------------------------------------

fn print_hex(p: &[u8]) {
    let mut out = String::with_capacity(p.len() * 2 + 1);
    for b in p {
        out.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        out.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    out.push('\n');
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}

fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut rdr = ByteReader::new();
    let x: f32 = scan_float(&mut rdr).unwrap_or(0.0f32);
    driver(x);
}
