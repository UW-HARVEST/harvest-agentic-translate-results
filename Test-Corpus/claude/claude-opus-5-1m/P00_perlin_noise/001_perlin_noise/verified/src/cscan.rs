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

    /// Number of input bytes consumed so far, i.e. what C's `%n` would report.
    /// (Only the differential tests need this.)
    #[allow(dead_code)]
    pub fn consumed(&self) -> usize {
        self.pos
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

        // Hexadecimal floating point.
        //
        // glibc collects the subject sequence with its own state machine and
        // then hands the collected text to `strtof`; the conversion fails only
        // when nothing but the "0x" prefix was collected.  In particular "0x."
        // *succeeds* with the value 0 (`strtof("0x.")` converts the leading
        // "0"), while "0x" and "0xp1" fail.
        if self.peek() == Some(b'0') && matches!(self.peek_at(1), Some(b'x') | Some(b'X')) {
            self.bump();
            self.bump();
            return self.scan_hex_float_body().map(|v| apply_sign(v, negative));
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

    /// Consumes the body of a hex float (the "0x" prefix is already gone) and
    /// returns `None` for a matching failure.
    ///
    /// glibc's state machine (`stdio-common/vfscanf-internal.c`) accepts, after
    /// the prefix: hex digits and at most one radix point, and the `p`
    /// exponent marker *only* once at least one hex digit has been seen (which
    /// is why "0x.p1" stops before the `p` and converts to 0, while "0x1p"
    /// consumes the dangling `p` and converts to 1).  A sign is accepted only
    /// directly behind the marker, and the collected text is finally converted
    /// by `strtof`, which ignores an exponent without digits.  The conversion
    /// fails only when nothing at all followed the "0x".
    fn scan_hex_float_body(&mut self) -> Option<f32> {
        let mut mantissa: u128 = 0;
        let mut sticky = false;
        let mut binary_exponent: i64 = 0;
        let mut got_digit = false;
        let mut got_dot = false;
        let mut got_any = false;

        loop {
            match self.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    got_digit = true;
                    got_any = true;
                    let digit = u128::from(hex_value(c));
                    if mantissa <= (u128::MAX >> 8) {
                        mantissa = mantissa * 16 + digit;
                        if got_dot {
                            binary_exponent -= 4;
                        }
                    } else {
                        // Below the precision we track: remember that the value
                        // is not exact and keep the magnitude.
                        if digit != 0 {
                            sticky = true;
                        }
                        if !got_dot {
                            binary_exponent += 4;
                        }
                    }
                    self.bump();
                }
                Some(b'.') if !got_dot => {
                    got_dot = true;
                    got_any = true;
                    self.bump();
                }
                _ => break,
            }
        }

        if !got_any {
            // Only the "0x" prefix was present.
            return None;
        }

        // Binary exponent: 'p' with an optional sign and decimal digits.  The
        // marker is only part of the number when a hex digit preceded it.
        if got_digit && matches!(self.peek(), Some(b'p') | Some(b'P')) {
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

        Some(hex_to_f32(mantissa, sticky, binary_exponent))
    }
}

/// Rounds `mantissa * 2^exponent` (plus a non-zero remainder when `sticky`) to
/// `float` the way `strtof` does: once, to nearest, ties to even.
///
/// The mantissa is first reduced to 53 significant bits with the dropped bits
/// folded into its lowest bit, which makes the `f64` product exact enough that
/// the final `as f32` cannot round twice.
fn hex_to_f32(mantissa: u128, sticky: bool, exponent: i64) -> f32 {
    if mantissa == 0 {
        return 0.0;
    }
    let mut m = mantissa;
    let mut e = exponent;
    let mut s = sticky;
    let bits = 128 - m.leading_zeros() as i64;
    if bits > 53 {
        let shift = bits - 53;
        if m & ((1u128 << shift) - 1) != 0 {
            s = true;
        }
        m >>= shift;
        e += shift;
    }
    let mut m = m as u64;
    if s {
        m |= 1;
    }
    scale_by_pow2(m as f64, e) as f32
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

#[cfg(test)]
mod glibc_tests {
    //! Differential tests of the scanner against the very `scanf`
    //! implementation the C program uses (glibc), including the number of
    //! characters each conversion consumes.

    use super::Scanner;
    use std::ffi::CString;
    use std::os::raw::{c_char, c_float, c_int};

    extern "C" {
        fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    }

    /// glibc `sscanf(s, "%f%n")` -> (assignments, value bits, consumed).
    fn glibc_float(s: &str) -> (i32, Option<u32>, i32) {
        let cs = CString::new(s).unwrap();
        let fmt = CString::new("%f%n").unwrap();
        let mut v: c_float = -12345.0;
        let mut n: c_int = -1;
        let ret = unsafe { sscanf(cs.as_ptr(), fmt.as_ptr(), &mut v, &mut n) };
        (
            ret,
            if ret == 1 { Some(v.to_bits()) } else { None },
            n,
        )
    }

    /// glibc `sscanf(s, "%d%n")` -> (assignments, value, consumed).
    fn glibc_int(s: &str) -> (i32, Option<i32>, i32) {
        let cs = CString::new(s).unwrap();
        let fmt = CString::new("%d%n").unwrap();
        let mut v: c_int = -12345;
        let mut n: c_int = -1;
        let ret = unsafe { sscanf(cs.as_ptr(), fmt.as_ptr(), &mut v, &mut n) };
        (ret, if ret == 1 { Some(v) } else { None }, n)
    }

    fn rust_float(s: &str) -> (Option<u32>, usize) {
        let mut sc = Scanner::new(std::io::Cursor::new(s.as_bytes().to_vec()));
        let v = sc.scan_float();
        (v.map(|f| f.to_bits()), sc.consumed())
    }

    fn rust_int(s: &str) -> (Option<i32>, usize) {
        let mut sc = Scanner::new(std::io::Cursor::new(s.as_bytes().to_vec()));
        let v = sc.scan_int();
        (v, sc.consumed())
    }

    #[track_caller]
    fn check_float(s: &str) {
        let (ret, bits, consumed) = glibc_float(s);
        let (rbits, rconsumed) = rust_float(s);
        assert_eq!(
            bits, rbits,
            "%f value mismatch for {s:?} (glibc ret={ret})"
        );
        if ret == 1 {
            assert_eq!(
                consumed as usize, rconsumed,
                "%f consumed mismatch for {s:?}: glibc {consumed}, rust {rconsumed}"
            );
        }
    }

    #[track_caller]
    fn check_int(s: &str) {
        let (ret, val, consumed) = glibc_int(s);
        let (rval, rconsumed) = rust_int(s);
        assert_eq!(val, rval, "%d value mismatch for {s:?} (glibc ret={ret})");
        if ret == 1 {
            assert_eq!(
                consumed as usize, rconsumed,
                "%d consumed mismatch for {s:?}: glibc {consumed}, rust {rconsumed}"
            );
        }
    }

    /// Deterministic splitmix64.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
        fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
            &xs[self.below(xs.len() as u64) as usize]
        }
    }

    #[test]
    fn float_tokens_match_glibc() {
        for s in [
            "0x.", "0x", "0x.p1", "0xp1", "0x1p", "0x1p+", "0x1p-", "0x1p 3", "0x1.8.8", "0x.5",
            "0x.8p1", "0x8.p1", "0x1p--2", "0x.g", "0x.0", "0X.", "0X.A", "0x1e5", "0x1E5",
            "0X1P3", "0x1p3x", "0x1p2147483648", "0x1p-149", "0x1p-150", "0x1p128", "0x0",
            "0x00", "0.", "00", "1e", "1e+", "1e-", "1.5e", "1E5", "1e05", ".5e3", "5.e3", ".5",
            ".", "-.", "-.5", "1_5", "1.5.5", "--1", "1,5", "1.5f", "inf", "-inf", "INF",
            "infinity", "INFINITY", "infinit", "infinityx", "nan", "-nan", "NaN", "nan(abc)",
            "nanx", "in", "na", "i", "n", "1e2147483648", "1e-2147483648",
            "1e999999999999999999999", "340282350000000000000000000000000000000",
            "340282360000000000000000000000000000000", "1e38", "1e39", "1e-38", "1e-45", "1e-46",
            "7e-46", "0.0000000000000000000000000000000000000000000000001", "  \t\n1.5", "+.5",
            "-0", "-0.0", "0", "1", "16777217", "16777216.5", "0x1.fffffep127",
            "0x1.ffffffp127", "0x1.0000001p0", "0x1.00000009p0", "0x1.00000011p0",
            "0x1000000.800000000001p0", "0x1000001.8p0", "0x0.0000000000000000000001p100",
        ] {
            check_float(s);
        }
    }

    #[test]
    fn int_tokens_match_glibc() {
        for s in [
            "0", "-0", "+0", "1", "-1", "007", "0x10", "2147483647", "-2147483648", "2147483648",
            "-2147483649", "4294967296", "9223372036854775807", "9223372036854775808",
            "-9223372036854775808", "-9223372036854775809", "99999999999999999999999",
            "-99999999999999999999999", "1e3", "-", "+", "abc", "  42", "\t-7\n", "0000000000012",
            "12abc", "12.5", "-000000000000000000000000001",
        ] {
            check_int(s);
        }
    }

    #[test]
    fn randomised_float_tokens_match_glibc() {
        let mut rng = Rng(0x5CA1AB1E);
        let digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        let hexdigits = [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'A',
            'B', 'C', 'D', 'E', 'F',
        ];
        for _ in 0..20000 {
            let mut t = String::new();
            if rng.below(4) == 0 {
                t.push(*rng.pick(&['+', '-']));
            }
            if rng.below(3) == 0 {
                // hexadecimal
                t.push('0');
                t.push(*rng.pick(&['x', 'X']));
                for _ in 0..rng.below(22) {
                    t.push(*rng.pick(&hexdigits));
                }
                if rng.below(2) == 0 {
                    t.push('.');
                    for _ in 0..rng.below(22) {
                        t.push(*rng.pick(&hexdigits));
                    }
                }
                match rng.below(5) {
                    0 => {}
                    1 => t.push(*rng.pick(&['p', 'P'])),
                    2 => t.push_str(&format!("p{}", rng.below(400) as i64 - 200)),
                    3 => t.push_str(&format!("P+{}", rng.below(300))),
                    _ => t.push_str(&format!("p-{}", rng.below(300))),
                }
            } else {
                for _ in 0..rng.below(25) {
                    t.push(*rng.pick(&digits));
                }
                if rng.below(2) == 0 {
                    t.push('.');
                    for _ in 0..rng.below(25) {
                        t.push(*rng.pick(&digits));
                    }
                }
                match rng.below(5) {
                    0 => {}
                    1 => t.push(*rng.pick(&['e', 'E'])),
                    2 => t.push_str(&format!("e{}", rng.below(120) as i64 - 60)),
                    3 => t.push_str(&format!("E+{}", rng.below(400))),
                    _ => t.push_str(&format!("e-{}", rng.below(400))),
                }
            }
            // Occasionally append trailing junk.
            if rng.below(4) == 0 {
                t.push(*rng.pick(&['x', 'p', '.', 'e', '-', '+', ' ', 'z']));
            }
            check_float(&t);
        }
    }

    #[test]
    fn randomised_int_tokens_match_glibc() {
        let mut rng = Rng(0xBADC0FFE);
        let digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        for _ in 0..20000 {
            let mut t = String::new();
            for _ in 0..rng.below(3) {
                t.push(*rng.pick(&[' ', '\t', '\n']));
            }
            if rng.below(3) == 0 {
                t.push(*rng.pick(&['+', '-']));
            }
            for _ in 0..rng.below(26) {
                t.push(*rng.pick(&digits));
            }
            if rng.below(4) == 0 {
                t.push(*rng.pick(&['x', '.', 'e', '-', ' ', 'z']));
            }
            check_int(&t);
        }
    }
}
