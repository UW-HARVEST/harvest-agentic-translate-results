//! Minimal emulation of glibc `scanf` for the `%d`, `%u` and `%zu`
//! conversions used by `main.c`.
//!
//! Like `scanf`, leading whitespace (including newlines) is skipped, so tokens
//! are read across line boundaries.  Conversion follows glibc: the digits are
//! converted with `strtol`/`strtoul` semantics (saturating on overflow, and
//! wrapping for a negated unsigned value) and the result is then truncated to
//! the destination type.

use std::io::Read;

/// Values larger than this are certainly out of range for every destination
/// type, so accumulation can stop there.
const CAP: u128 = 1u128 << 70;

pub struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    pub fn from_stdin() -> Scanner {
        let mut data = Vec::new();
        // Ignore read errors the same way C would simply see end of input.
        let _ = std::io::stdin().read_to_end(&mut data);
        Scanner { data, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    /// Parse `[+-]?[0-9]+`, returning the sign, the (clamped) magnitude and
    /// whether the magnitude exceeded what any integer type can hold.
    /// `None` means a matching failure / end of input (i.e. `scanf` != 1).
    fn scan_number(&mut self) -> Option<(bool, u128, bool)> {
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
        let mut value: u128 = 0;
        let mut huge = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            self.pos += 1;
            if !huge {
                value = value * 10 + u128::from(c - b'0');
                if value >= CAP {
                    huge = true;
                }
            }
        }
        if self.pos == digits_start {
            // No digits consumed: matching failure, input stays unconsumed.
            self.pos = start;
            return None;
        }
        Some((negative, value, huge))
    }

    /// `strtol` result (saturating at LONG_MIN / LONG_MAX).
    fn scan_long(&mut self) -> Option<i64> {
        let (negative, value, huge) = self.scan_number()?;
        let long_min_mag: u128 = 1u128 << 63;
        let result = if negative {
            if huge || value > long_min_mag {
                i64::MIN
            } else if value == long_min_mag {
                i64::MIN
            } else {
                -(value as i64)
            }
        } else if huge || value > i64::MAX as u128 {
            i64::MAX
        } else {
            value as i64
        };
        Some(result)
    }

    /// `strtoul` result (saturating at ULONG_MAX, wrapping a negated value).
    fn scan_ulong(&mut self) -> Option<u64> {
        let (negative, value, huge) = self.scan_number()?;
        if huge || value > u64::MAX as u128 {
            return Some(u64::MAX);
        }
        let magnitude = value as u64;
        Some(if negative {
            magnitude.wrapping_neg()
        } else {
            magnitude
        })
    }

    /// `scanf("%d", &int)`
    pub fn scan_int(&mut self) -> Option<i32> {
        self.scan_long().map(|v| v as i32)
    }

    /// `scanf("%u", &unsigned)`
    pub fn scan_uint(&mut self) -> Option<u32> {
        self.scan_ulong().map(|v| v as u32)
    }

    /// `scanf("%zu", &size_t)`
    pub fn scan_size(&mut self) -> Option<u64> {
        self.scan_ulong()
    }
}
