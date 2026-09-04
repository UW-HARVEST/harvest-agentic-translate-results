//! Emulation of the C library's `scanf` conversions used by `c_src/src/main.c`
//! (`%d` and `%f`).
//!
//! Semantics preserved:
//!   * each directive first skips *any* amount of whitespace, newlines
//!     included, so a single `scanf` call reads across lines;
//!   * on a matching failure or end of input the call stops immediately and
//!     every later argument is left untouched (the C `main` leaves them at
//!     their zero initialisers);
//!   * `%d` follows `strtol` and stores the value truncated to `int`, which
//!     reproduces glibc's overflow behaviour (`LONG_MAX` -> `-1`);
//!   * `%f` follows `strtof`, i.e. decimal and hexadecimal forms plus
//!     `inf`/`infinity`/`nan`, correctly rounded to `float`.

pub struct Scanner<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Scanner { buf, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn peek_at(&self, off: usize) -> Option<u8> {
        self.buf.get(self.pos + off).copied()
    }

    /// C `isspace` in the "C" locale.
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    /// `%d`
    pub fn scan_i32(&mut self) -> Option<i32> {
        self.skip_ws();
        let start = self.pos;
        let mut negative = false;
        match self.peek() {
            Some(b'+') => self.pos += 1,
            Some(b'-') => {
                negative = true;
                self.pos += 1;
            }
            _ => {}
        }
        let digits_start = self.pos;
        let mut acc: i64 = 0;
        let mut saturated = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            let d = (c - b'0') as i64;
            if !saturated {
                match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => acc = v,
                    None => saturated = true,
                }
            }
            self.pos += 1;
        }
        if self.pos == digits_start {
            // Matching failure: no digits.
            self.pos = start;
            return None;
        }
        // glibc stores `(int) strtol(...)`; on overflow strtol yields
        // LONG_MAX / LONG_MIN which then wrap when narrowed to `int`.
        let long_val: i64 = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            -acc
        } else {
            acc
        };
        Some(long_val as i32)
    }

    /// `%f`
    ///
    /// glibc's `scanf` is *not* `strtof`: it consumes input greedily with only
    /// a single character of pushback, so a partially matched special name
    /// (`infi`, `na`, `0x`) is a matching failure rather than a shorter match,
    /// a stray `e`/`p` exponent marker is swallowed, and `nan(...)` never
    /// consumes the parenthesised part. All of this is reproduced below; it is
    /// observable because a failure aborts the remaining directives.
    pub fn scan_f32(&mut self) -> Option<f32> {
        self.skip_ws();
        let mut negative = false;
        match self.peek() {
            Some(b'+') => self.pos += 1,
            Some(b'-') => {
                negative = true;
                self.pos += 1;
            }
            _ => {}
        }

        // "inf" / "infinity": once a fourth `i` shows up the whole word is
        // required.
        if matches!(self.peek(), Some(b'i') | Some(b'I')) {
            if !self.match_ci(b"inf") {
                return None;
            }
            if matches!(self.peek(), Some(b'i') | Some(b'I')) && !self.match_ci(b"inity") {
                return None;
            }
            return Some(apply_sign(f32::INFINITY, negative));
        }

        // "nan" — the optional `(n-char-sequence)` is not accepted here.
        if matches!(self.peek(), Some(b'n') | Some(b'N')) {
            if !self.match_ci(b"nan") {
                return None;
            }
            return Some(apply_sign(f32::NAN, negative));
        }

        // Hexadecimal form: 0x h* [. h*] [p [sign] d*]
        if self.peek() == Some(b'0') && matches!(self.peek_at(1), Some(b'x') | Some(b'X')) {
            self.pos += 2;
            let mut mantissa = 0.0f64;
            let mut bin_exp: i64 = 0;
            let mut ndigits = 0usize;
            while let Some(c) = self.peek() {
                if let Some(v) = hex_val(c) {
                    mantissa = mantissa * 16.0 + v as f64;
                    ndigits += 1;
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let mut has_point = false;
            if self.peek() == Some(b'.') {
                has_point = true;
                self.pos += 1;
                while let Some(c) = self.peek() {
                    if let Some(v) = hex_val(c) {
                        mantissa = mantissa * 16.0 + v as f64;
                        bin_exp -= 4;
                        ndigits += 1;
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            }
            if ndigits == 0 && !has_point {
                // "0x", "0xz", "0xp1": matching failure.
                return None;
            }
            // The binary exponent is only looked at when a digit was seen.
            if ndigits > 0 && matches!(self.peek(), Some(b'p') | Some(b'P')) {
                self.pos += 1;
                let mut eneg = false;
                match self.peek() {
                    Some(b'+') => self.pos += 1,
                    Some(b'-') => {
                        eneg = true;
                        self.pos += 1;
                    }
                    _ => {}
                }
                let ds = self.pos;
                let mut e: i64 = 0;
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        e = (e * 10 + (c - b'0') as i64).min(1 << 30);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                if self.pos > ds {
                    bin_exp += if eneg { -e } else { e };
                }
            }
            let v = ldexp(mantissa, bin_exp) as f32;
            return Some(apply_sign(v, negative));
        }

        // Decimal form: d* [. d*] [e [sign] d*], with at least one digit.
        let mut int_part = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                int_part.push(c as char);
                self.pos += 1;
            } else {
                break;
            }
        }
        let mut frac_part = String::new();
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    frac_part.push(c as char);
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        let mut exp_part = String::new();
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            // The marker and its sign are consumed even if no digit follows.
            self.pos += 1;
            let mut sign = String::new();
            match self.peek() {
                Some(b'+') => {
                    sign.push('+');
                    self.pos += 1;
                }
                Some(b'-') => {
                    sign.push('-');
                    self.pos += 1;
                }
                _ => {}
            }
            let ds = self.pos;
            let mut digits = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    digits.push(c as char);
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.pos > ds {
                exp_part = format!("e{}{}", sign, digits);
            }
        }

        let normalized = format!(
            "{}.{}{}",
            if int_part.is_empty() { "0" } else { &int_part },
            if frac_part.is_empty() { "0" } else { &frac_part },
            exp_part
        );
        // `strtof` is correctly rounded, and so is Rust's `f32` parser.
        let v: f32 = normalized.parse().unwrap_or(f32::INFINITY);
        Some(apply_sign(v, negative))
    }

    fn match_ci(&mut self, word: &[u8]) -> bool {
        for (i, w) in word.iter().enumerate() {
            match self.peek_at(i) {
                Some(c) if c.to_ascii_lowercase() == *w => {}
                _ => return false,
            }
        }
        self.pos += word.len();
        true
    }
}

fn apply_sign(v: f32, negative: bool) -> f32 {
    if negative {
        -v
    } else {
        v
    }
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a' + 10) as u32),
        b'A'..=b'F' => Some((c - b'A' + 10) as u32),
        _ => None,
    }
}

/// `x * 2^e`, computed in steps so that huge exponents cannot overflow the
/// intermediate power of two.
fn ldexp(mut x: f64, mut e: i64) -> f64 {
    while e > 1000 {
        x *= f64::from_bits(0x7FE0_0000_0000_0000); // 2^1023
        e -= 1023;
        if x.is_infinite() {
            return x;
        }
    }
    while e < -1000 {
        x *= f64::from_bits(0x0010_0000_0000_0000); // 2^-1022
        e += 1022;
        if x == 0.0 {
            return x;
        }
    }
    x * (2.0f64).powi(e as i32)
}
