//! Emulation of C's `scanf("%f", &x)` conversion as implemented by glibc.
//!
//! glibc's `vfscanf` collects a candidate token for `%f` and hands it to
//! `strtof`; the resulting value is what we must reproduce bit-for-bit. The
//! grammar accepted is:
//!
//!   * optional sign
//!   * `inf` / `infinity` (case-insensitive)
//!   * `nan` (case-insensitive)
//!   * hexadecimal form `0xh.hhhp[+-]ddd`
//!   * decimal form `ddd.ddde[+-]ddd`
//!
//! Two behaviours are specific to `vfscanf` and differ from a bare `strtof`
//! call; both were verified empirically against glibc 2.34 and are reproduced
//! here because the program's output depends on them:
//!
//!   * `vfscanf` matches the token against "infinity" and accepts it only if
//!     exactly 3 or exactly 8 characters matched. A partial match such as
//!     `"infi"` or `"infinit"` is a *conversion failure*, so the destination is
//!     left untouched, whereas `strtof("infi")` would yield infinity.
//!   * `vfscanf` never collects the `(n-char-sequence)` that may follow `nan`,
//!     so NaN payloads are ignored: `scanf` on `"nan(123)"` yields a plain quiet
//!     NaN, while `strtof("nan(123)")` yields a NaN carrying payload 123.
//!
//! Incomplete exponents are backed off exactly as `strtod` does (`"1e"` parses
//! as `1.0`), and a leading `0x` with no hex digits degrades to the single `0`
//! digit. Only the produced value is observable in this program, so the number
//! of characters consumed is tracked but unused.

/// `isspace` in the C locale, which is what scanf's leading whitespace skip uses.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn make_inf(neg: bool) -> f32 {
    f32::from_bits(if neg { 0xff80_0000 } else { 0x7f80_0000 })
}

fn make_zero(neg: bool) -> f32 {
    f32::from_bits(if neg { 0x8000_0000 } else { 0x0000_0000 })
}

/// Perform the whole `scanf("%f")` conversion over the remaining stream bytes.
/// Returns `None` when the conversion fails (matching failure or EOF), in which
/// case C leaves the destination object unmodified.
pub fn scanf_float(input: &[u8]) -> Option<f32> {
    let mut i = 0usize;
    while i < input.len() && is_c_space(input[i]) {
        i += 1;
    }
    strtof(&input[i..]).map(|(v, _consumed)| v)
}

/// `strtof` restricted to what follows the whitespace skip. Returns the value
/// and the number of bytes consumed, or `None` for "no conversion performed".
fn strtof(s: &[u8]) -> Option<(f32, usize)> {
    let mut i = 0usize;
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    if i >= s.len() {
        return None;
    }

    // "inf" / "infinity": vfscanf consumes the longest prefix of "infinity" and
    // then demands that the match length be exactly 3 or exactly 8. Anything in
    // between (e.g. "infi") is a conversion failure.
    if (s[i] | 0x20) == b'i' {
        const INF_STR: &[u8; 8] = b"infinity";
        let mut cnt = 0usize;
        while cnt < 8 && i + cnt < s.len() && (s[i + cnt] | 0x20) == INF_STR[cnt] {
            cnt += 1;
        }
        if cnt != 3 && cnt != 8 {
            return None;
        }
        return Some((make_inf(neg), i + cnt));
    }

    // "nan": vfscanf matches only the three letters and never the optional
    // parenthesised payload sequence, so the result is always a plain quiet NaN.
    if (s[i] | 0x20) == b'n' {
        const NAN_STR: &[u8; 3] = b"nan";
        let mut cnt = 0usize;
        while cnt < 3 && i + cnt < s.len() && (s[i + cnt] | 0x20) == NAN_STR[cnt] {
            cnt += 1;
        }
        if cnt != 3 {
            return None;
        }
        let mut bits: u32 = 0x7fc0_0000;
        if neg {
            bits |= 0x8000_0000;
        }
        return Some((f32::from_bits(bits), i + 3));
    }

    if s[i] == b'0' && i + 1 < s.len() && (s[i + 1] == b'x' || s[i + 1] == b'X') {
        if let Some(r) = parse_hex(s, i + 2, neg) {
            return Some(r);
        }
        // `0x` with no hex digits: only the leading `0` is converted.
        return Some((make_zero(neg), i + 1));
    }

    parse_dec(s, i, neg)
}

/// Parse an optional `[eEpP][+-]?digits` exponent. On an incomplete exponent the
/// characters are not consumed, mirroring strtod's backtracking.
fn parse_exponent(s: &[u8], j: &mut usize, markers: [u8; 2]) -> i64 {
    if *j >= s.len() || (s[*j] != markers[0] && s[*j] != markers[1]) {
        return 0;
    }
    let mut k = *j + 1;
    let mut eneg = false;
    if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
        eneg = s[k] == b'-';
        k += 1;
    }
    if k >= s.len() || !s[k].is_ascii_digit() {
        return 0;
    }
    let mut e: i64 = 0;
    while k < s.len() && s[k].is_ascii_digit() {
        if e < 1_000_000 {
            e = e * 10 + i64::from(s[k] - b'0');
        }
        k += 1;
    }
    *j = k;
    if eneg {
        -e
    } else {
        e
    }
}

fn hex_val(b: u8) -> u64 {
    match b {
        b'0'..=b'9' => u64::from(b - b'0'),
        b'a'..=b'f' => u64::from(b - b'a') + 10,
        _ => u64::from(b - b'A') + 10,
    }
}

/// Hexadecimal floating form, with `i` positioned just after the `0x` prefix.
fn parse_hex(s: &[u8], i: usize, neg: bool) -> Option<(f32, usize)> {
    let mut digits: Vec<u64> = Vec::new();
    let mut frac_len: i64 = 0;
    let mut j = i;
    let mut any = false;

    while j < s.len() && s[j].is_ascii_hexdigit() {
        digits.push(hex_val(s[j]));
        j += 1;
        any = true;
    }
    if j < s.len() && s[j] == b'.' {
        let mut k = j + 1;
        let mut fany = false;
        while k < s.len() && s[k].is_ascii_hexdigit() {
            digits.push(hex_val(s[k]));
            frac_len += 1;
            k += 1;
            fany = true;
        }
        if any || fany {
            j = k;
            any = true;
        }
    }
    if !any {
        return None;
    }

    let exp = parse_exponent(s, &mut j, [b'p', b'P']);

    // Drop leading zero digits so the accumulator keeps significant bits.
    let first_sig = digits.iter().position(|&d| d != 0);
    let value = match first_sig {
        None => make_zero(neg),
        Some(pos) => {
            let mut m: u64 = 0;
            let mut sticky = false;
            let mut extra: i64 = 0;
            for &d in &digits[pos..] {
                if m < (1u64 << 59) {
                    m = (m << 4) | d;
                } else {
                    // Beyond 64 bits of precision only "are any bits set"
                    // matters for correct rounding.
                    if d != 0 {
                        sticky = true;
                    }
                    extra += 4;
                }
            }
            let exp2 = exp - 4 * frac_len + extra;
            round_to_f32(m, sticky, exp2, neg)
        }
    };

    Some((value, j))
}

/// Decimal floating form.
fn parse_dec(s: &[u8], i: usize, neg: bool) -> Option<(f32, usize)> {
    let mut int_digits: Vec<u8> = Vec::new();
    let mut frac_digits: Vec<u8> = Vec::new();
    let mut j = i;
    let mut any = false;

    while j < s.len() && s[j].is_ascii_digit() {
        int_digits.push(s[j]);
        j += 1;
        any = true;
    }
    if j < s.len() && s[j] == b'.' {
        let mut k = j + 1;
        while k < s.len() && s[k].is_ascii_digit() {
            frac_digits.push(s[k]);
            k += 1;
            any = true;
        }
        if any {
            j = k;
        }
    }
    if !any {
        return None;
    }

    let exp = parse_exponent(s, &mut j, [b'e', b'E']);

    let mut digits: Vec<u8> = int_digits;
    digits.extend_from_slice(&frac_digits);
    let exp10 = exp - frac_digits.len() as i64;

    // Strip leading zeros; an all-zero significand is a signed zero.
    let first_sig = digits.iter().position(|&d| d != b'0');
    let value = match first_sig {
        None => make_zero(neg),
        Some(pos) => {
            let sig = &digits[pos..];
            // Position of the decimal point relative to the significant digits:
            // the value lies in [10^(adj-1), 10^adj).
            let adj = exp10.saturating_add(sig.len() as i64);
            if adj > 60 {
                make_inf(neg)
            } else if adj < -60 {
                make_zero(neg)
            } else {
                // Safe to hand to Rust's correctly-rounded decimal parser, which
                // matches glibc's correctly-rounded strtof.
                let mut text = String::with_capacity(sig.len() + 24);
                text.push_str(core::str::from_utf8(sig).unwrap_or("0"));
                text.push('e');
                let e = exp10.clamp(-1_000_000_000, 1_000_000_000);
                text.push_str(&e.to_string());
                let v: f32 = text.parse().unwrap_or(0.0);
                if neg {
                    -v
                } else {
                    v
                }
            }
        }
    };

    Some((value, j))
}

/// Round `(m + tiny_if_sticky) * 2^e` to `f32` using round-to-nearest,
/// ties-to-even, handling subnormals and overflow to infinity.
fn round_to_f32(m: u64, sticky: bool, e: i64, neg: bool) -> f32 {
    if m == 0 {
        return make_zero(neg);
    }

    let bits_in_m = 64 - i64::from(m.leading_zeros());
    // Unbiased exponent of the most significant set bit.
    let top_exp = bits_in_m - 1 + e;

    if top_exp > 200 {
        return make_inf(neg);
    }
    if top_exp < -200 {
        return make_zero(neg);
    }

    // Weight of the least significant bit we can keep: 24 significant bits for
    // normals, clamped at 2^-149 for subnormals.
    let q = core::cmp::max(top_exp - 23, -149);
    let shift = q - e;

    let mut keep: u128;
    if shift <= 0 {
        keep = (m as u128) << ((-shift) as u32);
    } else {
        let sh = shift as u32;
        let mm = m as u128;
        keep = if sh >= 128 { 0 } else { mm >> sh };
        let round_bit = if sh >= 129 { 0 } else { (mm >> (sh - 1)) & 1 };
        let low_mask: u128 = if sh >= 129 {
            u128::MAX
        } else {
            (1u128 << (sh - 1)) - 1
        };
        let rest = (mm & low_mask) != 0 || sticky;
        if round_bit == 1 && (rest || (keep & 1) == 1) {
            keep += 1;
        }
    }

    let bits: u32 = if q == -149 {
        // The IEEE-754 binary32 encoding is contiguous across the
        // subnormal/normal boundary when the significand is scaled by 2^-149,
        // so the integer itself is the encoding.
        keep as u32
    } else {
        let mut kk = keep;
        let mut ee = q + 23;
        if kk >= (1u128 << 24) {
            kk >>= 1;
            ee += 1;
        }
        let exp_field = ee + 127;
        if exp_field >= 255 {
            return make_inf(neg);
        }
        ((exp_field as u32) << 23) | ((kk as u32) & 0x007f_ffff)
    };

    let out = f32::from_bits(bits);
    if neg {
        f32::from_bits(out.to_bits() | 0x8000_0000)
    } else {
        out
    }
}
