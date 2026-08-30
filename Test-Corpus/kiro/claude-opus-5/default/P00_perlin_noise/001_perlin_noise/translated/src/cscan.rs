//! Minimal reimplementation of the `scanf` conversions used by `main.c`
//! (`%d` and `%f`), matching C semantics:
//!
//! * leading whitespace (including newlines) is skipped before each conversion,
//!   so a conversion happily reads across line boundaries;
//! * on a matching failure the conversion stops and no further assignments are
//!   made, leaving the remaining variables at their initial values;
//! * `%d` overflow follows glibc (`strtol` saturates to `LONG_{MAX,MIN}`, then
//!   the value is truncated to `int`).

pub struct Scanner {
    buf: Vec<u8>,
    pos: usize,
}

#[inline]
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

#[inline]
fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Correctly rounded hexadecimal-float to `f32`.
///
/// `digits` holds the hex digit values of the integer part followed by the
/// fraction part, `frac_len` is how many of them are fractional, and `p_exp` is
/// the binary exponent, i.e. the value is
/// `digits * 2^(p_exp - 4*frac_len)`.
fn hex_to_f32(digits: &[u8], frac_len: usize, p_exp: i64, neg: bool) -> f32 {
    // Accumulate the significand; digits that no longer fit only contribute a
    // sticky bit (they are strictly below the rounding position).
    let mut mant: u128 = 0;
    let mut dropped_exp: i64 = 0;
    let mut sticky = false;
    for &d in digits {
        if mant < (1u128 << 124) {
            mant = (mant << 4) | d as u128;
        } else {
            dropped_exp += 4;
            if d != 0 {
                sticky = true;
            }
        }
    }
    let sign = if neg { -1.0f32 } else { 1.0f32 };
    if mant == 0 {
        return sign * 0.0;
    }

    // value == mant * 2^exp2 (plus the sticky remainder)
    let exp2: i64 = dropped_exp - 4 * frac_len as i64 + p_exp;
    let nbits = 128 - mant.leading_zeros() as i64;
    let e = exp2 + nbits - 1; // floor(log2(value))
    if e > 200 {
        return sign * f32::INFINITY;
    }
    if e < -400 {
        return sign * 0.0;
    }

    // Exponent of the least significant bit of an f32 with this magnitude.
    let target = std::cmp::max(e - 23, -149);
    let shift = target - exp2;
    let q: u128 = if shift <= 0 {
        let s = (-shift) as u32;
        if s >= 104 {
            return sign * f32::INFINITY;
        }
        mant << s
    } else if shift > 128 {
        return sign * 0.0;
    } else {
        let s = shift as u32;
        let (q0, rem, half) = if s == 128 {
            (0u128, mant, 1u128 << 127)
        } else {
            (mant >> s, mant & ((1u128 << s) - 1), 1u128 << (s - 1))
        };
        if rem > half || (rem == half && (sticky || (q0 & 1) == 1)) {
            q0 + 1
        } else {
            q0
        }
    };

    // `q` needs at most 25 bits and `target >= -149`, so both factors and their
    // product are exact in f64; the cast only overflows to infinity.
    let v = (q as f64) * 2f64.powi(target as i32);
    sign * (v as f32)
}

impl Scanner {
    pub fn new(buf: Vec<u8>) -> Self {
        Scanner { buf, pos: 0 }
    }

    #[inline]
    fn peek(&self, off: usize) -> Option<u8> {
        self.buf.get(self.pos + off).copied()
    }

    #[inline]
    fn at(&self, i: usize) -> Option<u8> {
        self.buf.get(i).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek(0) {
            if is_space(c) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// `%d`
    pub fn scan_i32(&mut self) -> Option<i32> {
        self.skip_ws();
        let mut i = self.pos;
        let neg = match self.at(i) {
            Some(b'-') => {
                i += 1;
                true
            }
            Some(b'+') => {
                i += 1;
                false
            }
            Some(_) => false,
            None => return None,
        };

        let digits_start = i;
        let mut acc: i64 = 0;
        let mut overflow = false;
        while let Some(c) = self.at(i) {
            if !c.is_ascii_digit() {
                break;
            }
            let d = (c - b'0') as i64;
            if !overflow {
                match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => acc = v,
                    None => overflow = true,
                }
            }
            i += 1;
        }
        if i == digits_start {
            // Matching failure: nothing was assigned.
            return None;
        }
        self.pos = i;

        let val: i64 = if overflow {
            if neg {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if neg {
            -acc
        } else {
            acc
        };
        Some(val as i32)
    }

    /// `%f`
    ///
    /// Matches glibc's `__vfscanf_internal` float conversion, which differs from
    /// a plain `strtod` in a few observable ways (all confirmed against glibc):
    ///
    /// * an incomplete exponent is *consumed* rather than pushed back:
    ///   `1e` and `1e-` both convert to `1` and leave the `e`/sign behind;
    /// * `inf` and `infinity` are accepted, but any partial prefix of
    ///   `infinity` (`in`, `infi`, `infinit`) is a matching failure;
    /// * `nan` is accepted but the optional `nan(n-char-sequence)` form is not:
    ///   `nan(x)` converts to NaN and leaves `(x)` unread;
    /// * hexadecimal floats (`0x1.8p+1`) are accepted, while a `0x` with no hex
    ///   digits is a matching failure.
    ///
    /// Note that the stream position after a *failed* conversion is irrelevant
    /// here: `main.c` never reads again once a conversion fails.
    pub fn scan_f32(&mut self) -> Option<f32> {
        self.skip_ws();
        let mut i = self.pos;
        let neg = match self.at(i) {
            Some(b'-') => {
                i += 1;
                true
            }
            Some(b'+') => {
                i += 1;
                false
            }
            Some(_) => false,
            None => return None,
        };

        // "inf" / "infinity", but nothing in between.
        if self.match_ci(i, b"inf") {
            i += 3;
            let mut extra = 0;
            while extra < 5 && self.match_ci(i + extra, &b"inity"[extra..extra + 1]) {
                extra += 1;
            }
            if extra != 0 && extra != 5 {
                return None;
            }
            self.pos = i + extra;
            return Some(if neg {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            });
        }
        if self.match_ci(i, b"nan") {
            self.pos = i + 3;
            return Some(if neg { -f32::NAN } else { f32::NAN });
        }

        // Hexadecimal form: 0x / 0X prefix.
        if self.at(i) == Some(b'0') && matches!(self.at(i + 1), Some(b'x') | Some(b'X')) {
            i += 2;
            let mut digits: Vec<u8> = Vec::new();
            while let Some(c) = self.at(i) {
                match hex_val(c) {
                    Some(d) => {
                        digits.push(d);
                        i += 1;
                    }
                    None => break,
                }
            }
            let mut frac_len = 0usize;
            if self.at(i) == Some(b'.') {
                i += 1;
                while let Some(c) = self.at(i) {
                    match hex_val(c) {
                        Some(d) => {
                            digits.push(d);
                            frac_len += 1;
                            i += 1;
                        }
                        None => break,
                    }
                }
            }
            if digits.is_empty() {
                return None;
            }
            let mut p_exp: i64 = 0;
            if matches!(self.at(i), Some(b'p') | Some(b'P')) {
                i += 1;
                let mut exp_neg = false;
                match self.at(i) {
                    Some(b'-') => {
                        exp_neg = true;
                        i += 1;
                    }
                    Some(b'+') => i += 1,
                    _ => {}
                }
                let mut n: i64 = 0;
                let mut any = false;
                while let Some(c) = self.at(i) {
                    if !c.is_ascii_digit() {
                        break;
                    }
                    any = true;
                    n = (n * 10 + (c - b'0') as i64).min(1_000_000);
                    i += 1;
                }
                if any {
                    p_exp = if exp_neg { -n } else { n };
                }
                // An incomplete 'p' exponent is consumed, exponent stays 0.
            }
            self.pos = i;
            return Some(hex_to_f32(&digits, frac_len, p_exp, neg));
        }

        let mut int_digits: Vec<u8> = Vec::new();
        let mut frac_digits: Vec<u8> = Vec::new();
        while let Some(c) = self.at(i) {
            if !c.is_ascii_digit() {
                break;
            }
            int_digits.push(c);
            i += 1;
        }
        if self.at(i) == Some(b'.') {
            i += 1;
            while let Some(c) = self.at(i) {
                if !c.is_ascii_digit() {
                    break;
                }
                frac_digits.push(c);
                i += 1;
            }
        }
        if int_digits.is_empty() && frac_digits.is_empty() {
            // Matching failure.
            return None;
        }

        let mut exp_digits: Vec<u8> = Vec::new();
        let mut exp_neg = false;
        if matches!(self.at(i), Some(b'e') | Some(b'E')) {
            i += 1;
            let mut sign_neg = false;
            match self.at(i) {
                Some(b'-') => {
                    sign_neg = true;
                    i += 1;
                }
                Some(b'+') => i += 1,
                _ => {}
            }
            while let Some(c) = self.at(i) {
                if !c.is_ascii_digit() {
                    break;
                }
                exp_digits.push(c);
                i += 1;
            }
            exp_neg = sign_neg;
            // An incomplete exponent leaves `exp_digits` empty (exponent 0) but
            // the 'e' and sign stay consumed, as glibc does.
        }
        self.pos = i;

        // Rebuild a canonical literal so Rust's parser accepts it, then let it
        // do the correctly-rounded decimal -> f32 conversion (no intermediate
        // f64, to avoid double rounding).
        let mut lit = String::new();
        if neg {
            lit.push('-');
        }
        if int_digits.is_empty() {
            lit.push('0');
        } else {
            lit.push_str(std::str::from_utf8(&int_digits).unwrap());
        }
        lit.push('.');
        if frac_digits.is_empty() {
            lit.push('0');
        } else {
            lit.push_str(std::str::from_utf8(&frac_digits).unwrap());
        }
        lit.push('e');
        if exp_neg {
            lit.push('-');
        }
        if exp_digits.is_empty() {
            lit.push('0');
        } else if exp_digits.len() > 9 {
            // Absurd exponents only need to keep their sign and magnitude.
            lit.push_str("999999999");
        } else {
            lit.push_str(std::str::from_utf8(&exp_digits).unwrap());
        }

        Some(lit.parse::<f32>().unwrap_or(0.0))
    }

    fn match_ci(&self, at: usize, pat: &[u8]) -> bool {
        for (k, p) in pat.iter().enumerate() {
            match self.at(at + k) {
                Some(c) if c.to_ascii_lowercase() == *p => {}
                _ => return false,
            }
        }
        true
    }
}
