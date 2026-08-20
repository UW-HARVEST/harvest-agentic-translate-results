// Shared implementation for the Rust translation of c_src/src/main.c.
//
// Original C program:
//   reads a single float with scanf("%f", &x) (x pre-initialized to 0.f)
//   then prints the raw bytes of that float, in memory order, as lowercase
//   two-digit hex, followed by a newline.
//
// The C behavior is reproduced exactly, including:
//   * scanf skipping arbitrary leading whitespace (including newlines),
//   * the exact character-collection state machine glibc's `__vfscanf_internal`
//     uses for the `%f` conversion (which is NOT the same as strtof's grammar),
//   * leaving x == +0.0f when the conversion fails (matching failure or EOF),
//   * printing bytes in native memory order (%02x each, then "\n").
//
// Notable glibc `%f` details that are reproduced here (all verified against the
// compiled C program):
//   * the optional leading sign is NOT part of the buffer handed to strtof; it
//     is remembered separately and applied afterwards.  A conversion failure
//     therefore always leaves `x` at *positive* zero, e.g. "-0x" prints
//     00000000, not 00000080.
//   * a "0x"/"0X" prefix with nothing appended to the buffer after it is a
//     matching failure ("0x", "0xp1", "0x,", "0x-" all fail), but a following
//     '.' *is* appended, so "0x." converts to 0.
//   * the exponent character ('e' for decimal, 'p' for hex) is only accepted
//     after at least one digit has been seen, so ".e5" and "0xp1" fail.
//   * after "inf" has been read, if the next character is 'i'/'I' then the
//     complete word "infinity" is required; otherwise the conversion fails.
//     Hence "infin" is a matching failure while "infz" yields infinity.
//   * "nan" never consumes an "(n-char-sequence)" payload, so "nan(1)" yields
//     the default quiet NaN and leaves "(1)" in the stream.

use std::cmp::Ordering;
use std::io::{ErrorKind, Read, Write};

// ---------------------------------------------------------------------------
// Output helpers (mirroring print_hex / driver from the C source)
// ---------------------------------------------------------------------------

pub fn print_hex(p: &[u8], out: &mut impl Write) {
    for &b in p.iter() {
        let _ = write!(out, "{:02x}", b);
    }
    let _ = writeln!(out);
}

pub fn driver(x: f32, out: &mut impl Write) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw = x.to_ne_bytes();
    print_hex(&raw, out);
}

// ---------------------------------------------------------------------------
// scanf("%f") emulation
// ---------------------------------------------------------------------------

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn read_byte(r: &mut impl Read) -> Option<u8> {
    let mut buf = [0u8; 1];
    loop {
        match r.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => return Some(buf[0]),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

fn lower(b: u8) -> u8 {
    b.to_ascii_lowercase()
}

/// Emulates `scanf("%f", &x)`: returns Some(value) on a successful conversion,
/// None on matching failure or input failure (in which case the C code leaves
/// its variable untouched, i.e. at the initial +0.0f).
pub fn scan_float(r: &mut impl Read) -> Option<f32> {
    // Skip leading whitespace (this is what lets scanf read across newlines).
    let mut c: Option<u8> = loop {
        match read_byte(r) {
            None => return None, // input failure / EOF
            Some(b) if is_c_space(b) => continue,
            Some(b) => break Some(b),
        }
    };

    // Optional sign.  glibc keeps this out of the buffer it later hands to
    // strtof and negates the converted value instead.
    let mut negative = false;
    if c == Some(b'-') || c == Some(b'+') {
        negative = c == Some(b'-');
        c = read_byte(r);
        if c.is_none() {
            return None; // EOF right after the sign -> conversion failure
        }
    }

    // "nan" and "inf"/"infinity" are recognised by hand, before any digits.
    if let Some(ch) = c {
        if lower(ch) == b'n' {
            if read_byte(r).map(lower) != Some(b'a') {
                return None;
            }
            if read_byte(r).map(lower) != Some(b'n') {
                return None;
            }
            // No "(n-char-sequence)" payload is consumed by scanf.
            return Some(make_nan(negative));
        }
        if lower(ch) == b'i' {
            if read_byte(r).map(lower) != Some(b'n') {
                return None;
            }
            if read_byte(r).map(lower) != Some(b'f') {
                return None;
            }
            // At least "inf".  If the next character is 'i' the whole word
            // "infinity" must follow, otherwise the conversion fails.
            if let Some(nxt) = read_byte(r) {
                if lower(nxt) == b'i' {
                    for want in b"nity" {
                        if read_byte(r).map(lower) != Some(*want) {
                            return None;
                        }
                    }
                }
            }
            return Some(if negative {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            });
        }
    }

    // ---- glibc's numeric character collection -----------------------------
    let mut buf: Vec<u8> = Vec::new();
    let mut is_hexa = false;
    let mut exp_char = b'e';

    if c == Some(b'0') {
        buf.push(b'0');
        c = read_byte(r);
        if let Some(x) = c {
            if lower(x) == b'x' {
                buf.push(x);
                is_hexa = true;
                exp_char = b'p';
                c = read_byte(r);
            }
        }
    }

    let mut got_digit = false;
    let mut got_dot = false;
    let mut got_e = false;
    loop {
        let ch = match c {
            None => break,
            Some(x) => x,
        };
        if ch.is_ascii_digit() {
            buf.push(ch);
            got_digit = true;
        } else if !got_e && is_hexa && ch.is_ascii_hexdigit() {
            buf.push(ch);
            got_digit = true;
        } else if got_e
            && buf.last().copied() == Some(exp_char)
            && (ch == b'-' || ch == b'+')
        {
            buf.push(ch);
        } else if got_digit && !got_e && lower(ch) == exp_char {
            buf.push(exp_char);
            got_e = true;
            got_dot = true;
        } else if !got_dot && ch == b'.' {
            buf.push(ch);
            got_dot = true;
        } else {
            break;
        }
        c = read_byte(r);
    }

    // Have we read any character?  If we tried to read a number in hexadecimal
    // notation and only the "0x" prefix landed in the buffer, the conversion
    // fails.
    if buf.is_empty() || (is_hexa && buf.len() == 2) {
        return None;
    }

    if is_hexa {
        Some(parse_hex(&buf, negative))
    } else {
        // strtof consuming nothing (buffer == ".") is a conversion failure.
        parse_decimal(&buf, negative)
    }
}

fn eq_ci(a: u8, b: u8) -> bool {
    lower(a) == lower(b)
}

fn hex_val(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a' + 10) as u32),
        b'A'..=b'F' => Some((b - b'A' + 10) as u32),
        _ => None,
    }
}

/// The default quiet NaN produced by glibc's strtof for "nan", with the sign
/// applied afterwards (glibc computes `-d`, which flips the sign bit).
fn make_nan(neg: bool) -> f32 {
    let mut bits: u32 = 0x7fc00000;
    if neg {
        bits |= 0x80000000;
    }
    f32::from_bits(bits)
}

/// Hex float: `0x` hexdigits [. hexdigits] [pP [sign] digits]
fn parse_hex(body: &[u8], neg: bool) -> f32 {
    let mut i = 2usize; // skip "0x"
    let mut mant: u128 = 0;
    let mut sticky = false;
    let mut bin_exp: i32 = 0;
    let mut ndigits = 0usize;

    // Integral hex digits.
    while i < body.len() {
        match hex_val(body[i]) {
            Some(d) => {
                if mant >> 120 == 0 {
                    mant = mant * 16 + d as u128;
                } else {
                    bin_exp += 4;
                    if d != 0 {
                        sticky = true;
                    }
                }
                ndigits += 1;
                i += 1;
            }
            None => break,
        }
    }

    // Optional fractional part.
    if i < body.len() && body[i] == b'.' {
        let mut j = i + 1;
        let mut fdigits = 0usize;
        let mut fmant = mant;
        let mut fsticky = sticky;
        let mut fexp = bin_exp;
        while j < body.len() {
            match hex_val(body[j]) {
                Some(d) => {
                    if fmant >> 120 == 0 {
                        fmant = fmant * 16 + d as u128;
                        fexp -= 4;
                    } else if d != 0 {
                        fsticky = true;
                    }
                    fdigits += 1;
                    j += 1;
                }
                None => break,
            }
        }
        if ndigits > 0 || fdigits > 0 {
            mant = fmant;
            sticky = fsticky;
            bin_exp = fexp;
            ndigits += fdigits;
            i = j;
        }
    }

    if ndigits == 0 {
        // "0x" with no hex digits: strtof matches just the leading "0".
        return if neg { -0.0f32 } else { 0.0f32 };
    }

    // Optional binary exponent.
    if i < body.len() && eq_ci(body[i], b'p') {
        let mut j = i + 1;
        let mut eneg = false;
        if j < body.len() && (body[j] == b'+' || body[j] == b'-') {
            eneg = body[j] == b'-';
            j += 1;
        }
        let mut edigits = 0usize;
        let mut eval: i64 = 0;
        while j < body.len() && body[j].is_ascii_digit() {
            if eval < 10_000_000 {
                eval = eval * 10 + (body[j] - b'0') as i64;
            }
            edigits += 1;
            j += 1;
        }
        if edigits > 0 {
            let signed = if eneg { -eval } else { eval };
            bin_exp = bin_exp.saturating_add(signed as i32);
        }
    }

    make_f32(mant, sticky, bin_exp, neg)
}

/// Builds a correctly rounded f32 for `(mant [+ tiny if sticky]) * 2^exp`.
fn make_f32(mant: u128, sticky: bool, exp: i32, neg: bool) -> f32 {
    let sign_bit: u32 = if neg { 0x80000000 } else { 0 };

    if mant == 0 {
        return f32::from_bits(sign_bit);
    }

    let nbits = 128 - mant.leading_zeros() as i32;
    let shift_norm = nbits - 24; // keep 24 significant bits
    let shift_sub = -149 - exp; // lowest representable bit is 2^-149
    let shift = shift_norm.max(shift_sub);

    let (mut keep, mut e2) = if shift <= 0 {
        (mant << ((-shift) as u32), exp + shift)
    } else {
        let s = shift as u32;
        let (k, cmp) = if s >= 129 {
            (0u128, Ordering::Less)
        } else {
            let k = if s == 128 { 0u128 } else { mant >> s };
            let half = 1u128 << (s - 1);
            let dropped = if s == 128 {
                mant
            } else {
                mant & ((1u128 << s) - 1)
            };
            (k, dropped.cmp(&half))
        };
        let round_up = match cmp {
            Ordering::Greater => true,
            Ordering::Equal => sticky || (k & 1) == 1,
            Ordering::Less => false,
        };
        (if round_up { k + 1 } else { k }, exp + shift)
    };

    if keep == 0 {
        return f32::from_bits(sign_bit);
    }
    if keep >= 1u128 << 24 {
        // Rounding carried into a new binary digit.
        keep >>= 1;
        e2 += 1;
    }

    if keep >= 1u128 << 23 {
        let biased = e2 as i64 + 150;
        if biased >= 255 {
            return f32::from_bits(sign_bit | 0x7f800000); // infinity
        }
        if biased <= 0 {
            return f32::from_bits(sign_bit); // (unreachable in practice)
        }
        let bits = ((biased as u32) << 23) | ((keep as u32) - 0x00800000);
        f32::from_bits(sign_bit | bits)
    } else {
        // Subnormal: value == keep * 2^-149.
        f32::from_bits(sign_bit | keep as u32)
    }
}

/// Decimal: digits [. digits] [eE [sign] digits]
/// Returns None when strtof would consume no characters at all (buffer ".").
fn parse_decimal(body: &[u8], neg: bool) -> Option<f32> {
    let mut i = 0usize;
    let int_start = i;
    while i < body.len() && body[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = &body[int_start..i];
    let mut frac_digits: &[u8] = &[];

    if i < body.len() && body[i] == b'.' {
        let fstart = i + 1;
        let mut j = fstart;
        while j < body.len() && body[j].is_ascii_digit() {
            j += 1;
        }
        if !int_digits.is_empty() || j > fstart {
            frac_digits = &body[fstart..j];
            i = j;
        }
    }

    if int_digits.is_empty() && frac_digits.is_empty() {
        return None; // matching failure
    }

    let total_digits = (int_digits.len() + frac_digits.len()) as i64;
    let mut exp: i64 = 0;
    if i < body.len() && eq_ci(body[i], b'e') {
        let mut j = i + 1;
        let mut eneg = false;
        if j < body.len() && (body[j] == b'+' || body[j] == b'-') {
            eneg = body[j] == b'-';
            j += 1;
        }
        let estart = j;
        let mut eval: i64 = 0;
        let limit = 1_000_000 + total_digits;
        while j < body.len() && body[j].is_ascii_digit() {
            if eval <= limit {
                eval = eval * 10 + (body[j] - b'0') as i64;
            }
            j += 1;
        }
        if j > estart {
            if eval > limit {
                eval = limit;
            }
            exp = if eneg { -eval } else { eval };
        }
    }

    let int_part = if int_digits.is_empty() {
        "0".to_string()
    } else {
        String::from_utf8_lossy(int_digits).into_owned()
    };
    let frac_part = if frac_digits.is_empty() {
        "0".to_string()
    } else {
        String::from_utf8_lossy(frac_digits).into_owned()
    };
    let s = format!(
        "{}{}.{}e{}",
        if neg { "-" } else { "" },
        int_part,
        frac_part,
        exp
    );
    match s.parse::<f32>() {
        Ok(v) => Some(v),
        Err(_) => Some(if neg { -0.0f32 } else { 0.0f32 }),
    }
}

/// The body of the C `main`: read one float from `r`, print its bytes to `out`.
pub fn run(r: &mut impl Read, out: &mut impl Write) -> i32 {
    let mut x: f32 = 0.0;
    if let Some(v) = scan_float(r) {
        x = v;
    }
    driver(x, out);
    0
}
