//! Byte level re-implementation of the `scanf` conversions used by the program.
//!
//! Only the `%d` and `%f` directives are needed.  Both skip leading whitespace,
//! consume the longest matching subject sequence and report a "matching
//! failure" (`None`) when no valid sequence is present.  As in C, the caller
//! stops at the first failure and leaves the remaining variables untouched.

use std::io::Read;

/// Buffered input, filled on demand in `BUFSIZ` sized chunks just like C's
/// stdio, so the program never waits for more input than `scanf` would.
pub struct Scanner<R: Read> {
    reader: R,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(reader: R) -> Self {
        Scanner {
            reader,
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    /// Makes sure `self.buf` holds at least `count` bytes past `self.pos`.
    fn fill(&mut self, count: usize) {
        while !self.eof && self.buf.len() < self.pos + count {
            let mut chunk = [0u8; 4096];
            match self.reader.read(&mut chunk) {
                Ok(0) => self.eof = true,
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => self.eof = true,
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.peek_at(0)
    }

    fn peek_at(&mut self, offset: usize) -> Option<u8> {
        self.fill(offset + 1);
        self.buf.get(self.pos + offset).copied()
    }

    fn bump(&mut self) {
        if self.pos < self.buf.len() {
            self.pos += 1;
        }
    }

    /// Skips whitespace, matching the "C" locale `isspace()`.
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => self.bump(),
                _ => break,
            }
        }
    }

    /// Consumes an optional '+'/'-' sign, returning true when negative.
    fn scan_sign(&mut self) -> bool {
        match self.peek() {
            Some(b'+') => {
                self.bump();
                false
            }
            Some(b'-') => {
                self.bump();
                true
            }
            _ => false,
        }
    }

    /// Case insensitive match of `word` against the input.
    ///
    /// Returns the number of leading characters that matched; the matched
    /// characters are consumed.  This mirrors glibc's greedy behaviour for
    /// "inf"/"infinity"/"nan", where a partial match consumes the characters
    /// and then reports a matching failure.
    fn scan_word_prefix(&mut self, word: &str) -> usize {
        let mut matched = 0usize;
        for expected in word.bytes() {
            match self.peek() {
                Some(c) if c.eq_ignore_ascii_case(&expected) => {
                    self.bump();
                    matched += 1;
                }
                _ => break,
            }
        }
        matched
    }

    /// `scanf("%d", ...)`
    ///
    /// glibc converts the digits with `strtol` and then assigns the (possibly
    /// truncated) `long` to the `int` object, so out-of-range values saturate
    /// to `LONG_MAX`/`LONG_MIN` first and are then cut down to 32 bits.
    pub fn scan_int(&mut self) -> Option<i32> {
        self.skip_ws();
        let negative = self.scan_sign();

        let mut any = false;
        let mut value: i64 = 0;
        let mut overflow = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            any = true;
            let digit = i64::from(c - b'0');
            if !overflow {
                match value
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(if negative { -digit } else { digit }))
                {
                    Some(v) => value = v,
                    None => overflow = true,
                }
            }
            self.bump();
        }
        if !any {
            return None;
        }
        if overflow {
            value = if negative { i64::MIN } else { i64::MAX };
        }
        Some(value as i32)
    }

    /// `scanf("%f", ...)`
    pub fn scan_float(&mut self) -> Option<f32> {
        self.skip_ws();
        let negative = self.scan_sign();

        // "nan" / "inf" / "infinity"
        match self.peek().map(|c| c.to_ascii_lowercase()) {
            Some(b'n') => {
                if self.scan_word_prefix("nan") != 3 {
                    return None;
                }
                // glibc's scanf does not consume the optional "(chars)" suffix.
                return Some(apply_sign(f32::NAN, negative));
            }
            Some(b'i') => {
                if self.scan_word_prefix("inf") != 3 {
                    return None;
                }
                match self.scan_word_prefix("inity") {
                    0 | 5 => {}
                    _ => return None,
                }
                return Some(apply_sign(f32::INFINITY, negative));
            }
            _ => {}
        }

        // Hexadecimal floating point: "0x" must be followed by a hex digit
        // (possibly after the radix point), otherwise glibc reports a failure
        // after having consumed the "0x".
        if self.peek() == Some(b'0') && matches!(self.peek_at(1), Some(b'x') | Some(b'X')) {
            let hex_follows = match self.peek_at(2) {
                Some(c) if c.is_ascii_hexdigit() => true,
                Some(b'.') => matches!(self.peek_at(3), Some(c) if c.is_ascii_hexdigit()),
                _ => false,
            };
            self.bump();
            self.bump();
            if !hex_follows {
                return None;
            }
            return Some(apply_sign(self.scan_hex_float_body(), negative));
        }

        // Decimal floating point.
        let mut int_digits = String::new();
        let mut frac_digits = String::new();
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            int_digits.push(char::from(c));
            self.bump();
        }
        if self.peek() == Some(b'.') {
            self.bump();
            while let Some(c) = self.peek() {
                if !c.is_ascii_digit() {
                    break;
                }
                frac_digits.push(char::from(c));
                self.bump();
            }
        }
        if int_digits.is_empty() && frac_digits.is_empty() {
            return None;
        }

        let exponent = self.scan_decimal_exponent();

        let text = format!(
            "{}.{}e{}",
            if int_digits.is_empty() {
                "0"
            } else {
                int_digits.as_str()
            },
            if frac_digits.is_empty() {
                "0"
            } else {
                frac_digits.as_str()
            },
            exponent
        );
        let magnitude: f32 = text.parse().unwrap_or(0.0);
        Some(apply_sign(magnitude, negative))
    }

    /// Consumes a decimal exponent part, returning its value.
    ///
    /// glibc consumes the 'e' and an optional sign even when no digit follows,
    /// and in that case simply behaves as if no exponent had been given.
    fn scan_decimal_exponent(&mut self) -> i32 {
        if !matches!(self.peek(), Some(b'e') | Some(b'E')) {
            return 0;
        }
        self.bump();
        let negative = self.scan_sign();
        let mut any = false;
        let mut value: i64 = 0;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            any = true;
            if value < 1_000_000 {
                value = value * 10 + i64::from(c - b'0');
            }
            self.bump();
        }
        if !any {
            return 0;
        }
        let clamped = value.min(1_000_000) as i32;
        if negative {
            -clamped
        } else {
            clamped
        }
    }

    /// Consumes the digits of a hex float (the "0x" prefix is already gone).
    fn scan_hex_float_body(&mut self) -> f32 {
        let mut mantissa: u128 = 0;
        let mut binary_exponent: i64 = 0;
        let mut saturated = false;

        while let Some(c) = self.peek() {
            if !c.is_ascii_hexdigit() {
                break;
            }
            let digit = u128::from(hex_value(c));
            if saturated {
                binary_exponent += 4;
            } else if mantissa <= (u128::MAX >> 8) {
                mantissa = mantissa * 16 + digit;
            } else {
                saturated = true;
                binary_exponent += 4;
            }
            self.bump();
        }
        if self.peek() == Some(b'.') {
            self.bump();
            while let Some(c) = self.peek() {
                if !c.is_ascii_hexdigit() {
                    break;
                }
                let digit = u128::from(hex_value(c));
                if !saturated {
                    if mantissa <= (u128::MAX >> 8) {
                        mantissa = mantissa * 16 + digit;
                        binary_exponent -= 4;
                    } else {
                        saturated = true;
                    }
                }
                self.bump();
            }
        }

        // Binary exponent: 'p' with an optional sign and decimal digits.
        if matches!(self.peek(), Some(b'p') | Some(b'P')) {
            self.bump();
            let negative = self.scan_sign();
            let mut any = false;
            let mut value: i64 = 0;
            while let Some(c) = self.peek() {
                if !c.is_ascii_digit() {
                    break;
                }
                any = true;
                if value < 1_000_000 {
                    value = value * 10 + i64::from(c - b'0');
                }
                self.bump();
            }
            if any {
                let clamped = value.min(1_000_000);
                binary_exponent += if negative { -clamped } else { clamped };
            }
        }

        scale_by_pow2(mantissa as f64, binary_exponent) as f32
    }
}

fn hex_value(c: u8) -> u32 {
    match c {
        b'0'..=b'9' => u32::from(c - b'0'),
        b'a'..=b'f' => u32::from(c - b'a') + 10,
        _ => u32::from(c - b'A') + 10,
    }
}

/// `value * 2^exponent`, computed in steps so huge exponents behave sanely.
fn scale_by_pow2(value: f64, exponent: i64) -> f64 {
    let mut result = value;
    let mut remaining = exponent.clamp(-4096, 4096);
    while remaining > 0 {
        let step = remaining.min(512) as i32;
        result *= 2f64.powi(step);
        remaining -= i64::from(step);
    }
    while remaining < 0 {
        let step = (-remaining).min(512) as i32;
        result /= 2f64.powi(step);
        remaining += i64::from(step);
    }
    result
}

fn apply_sign(value: f32, negative: bool) -> f32 {
    if negative {
        -value
    } else {
        value
    }
}
