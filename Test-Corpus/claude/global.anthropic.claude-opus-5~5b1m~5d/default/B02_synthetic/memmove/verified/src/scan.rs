//! Minimal emulation of the `scanf` numeric conversions used by `main.c`.
//!
//! Only what the C program needs is modelled, but it is modelled precisely:
//!
//! * leading whitespace (the C `isspace` set) is skipped and may contain any
//!   number of newlines, so a conversion happily crosses line boundaries;
//! * an optional `+`/`-` sign is accepted, followed by one or more decimal
//!   digits - anything else is a matching failure (return value `0`) and an
//!   exhausted stream is an input failure (`EOF`).  `main.c` treats both the
//!   same way (`!= 1`), so a single `None` covers them;
//! * out-of-range values behave like glibc, which converts through
//!   `strtol`/`strtoul`: the result saturates at `LONG_MAX`/`LONG_MIN`
//!   (or `ULONG_MAX`) and is then truncated to the destination type.

/// The characters `isspace()` reports as whitespace in the "C" locale.
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Outcome of scanning a numeric token.
struct Token {
    negative: bool,
    /// Magnitude of the digit sequence, clamped while parsing.
    magnitude: u128,
    /// Set when the digit sequence no longer fits in `magnitude`.
    huge: bool,
}

pub struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    pub fn new(data: Vec<u8>) -> Self {
        Scanner { data, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if is_c_space(c) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Scan `[+-]?[0-9]+`, leaving the stream just past the digits.
    fn scan_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        let start = self.pos;
        let mut negative = false;
        match self.peek() {
            Some(b'+') => {
                self.pos += 1;
            }
            Some(b'-') => {
                negative = true;
                self.pos += 1;
            }
            _ => {}
        }

        let mut magnitude: u128 = 0;
        let mut huge = false;
        let mut digits = 0usize;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            digits += 1;
            self.pos += 1;
            if huge {
                continue;
            }
            let digit = u128::from(c - b'0');
            match magnitude
                .checked_mul(10)
                .and_then(|m| m.checked_add(digit))
            {
                Some(m) => magnitude = m,
                None => huge = true,
            }
        }

        if digits == 0 {
            /* Matching failure: C pushes the offending character back.  No
             * further conversion happens after a failure in this program, but
             * rewinding keeps the stream state faithful. */
            self.pos = start;
            return None;
        }

        Some(Token {
            negative,
            magnitude,
            huge,
        })
    }

    /// `scanf("%lu"/"%zu")` semantics: `strtoul` saturating at `ULONG_MAX`.
    fn scan_u64(&mut self) -> Option<u64> {
        let t = self.scan_token()?;
        if t.huge || t.magnitude > u128::from(u64::MAX) {
            return Some(u64::MAX);
        }
        let v = t.magnitude as u64;
        Some(if t.negative { v.wrapping_neg() } else { v })
    }

    /// `scanf("%ld")` semantics: `strtol` saturating at `LONG_MAX`/`LONG_MIN`.
    fn scan_i64(&mut self) -> Option<i64> {
        let t = self.scan_token()?;
        let limit = 1u128 << 63; /* |LONG_MIN| */
        if t.negative {
            if t.huge || t.magnitude > limit {
                return Some(i64::MIN);
            }
            if t.magnitude == limit {
                return Some(i64::MIN);
            }
            Some(-(t.magnitude as i64))
        } else {
            if t.huge || t.magnitude > u128::from(i64::MAX as u64) {
                return Some(i64::MAX);
            }
            Some(t.magnitude as i64)
        }
    }

    /// `scanf("%u", &unsigned_int)`
    pub fn scan_u32(&mut self) -> Option<u32> {
        self.scan_u64().map(|v| v as u32)
    }

    /// `scanf("%d", &int)`
    pub fn scan_i32(&mut self) -> Option<i32> {
        self.scan_i64().map(|v| v as i32)
    }

    /// `scanf("%zu", &size_t)`
    pub fn scan_usize(&mut self) -> Option<usize> {
        self.scan_u64().map(|v| v as usize)
    }
}
