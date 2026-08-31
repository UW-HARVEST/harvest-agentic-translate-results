//! Byte-level emulation of the C library's `scanf("%u", ...)` / `scanf("%d", ...)`
//! numeric conversions, as implemented by glibc on a 64-bit LP64 target.
//!
//! Behaviour reproduced here:
//!   * leading whitespace (including newlines) is skipped, so a conversion can
//!     span lines,
//!   * an optional `+`/`-` sign is accepted, even for `%u`,
//!   * at least one decimal digit is required, otherwise the conversion fails
//!     (matching failure) and the offending characters are pushed back,
//!   * the digit string is converted with `strtoul`/`strtol` semantics: on
//!     overflow the value saturates to `ULONG_MAX` / `LONG_MAX` / `LONG_MIN`
//!     (64-bit) and is then truncated to the 32-bit destination type,
//!   * a negative value for `%u` wraps modulo 2^64 before truncation,
//!   * on failure or EOF the caller's variable is left untouched.

use std::io::{BufReader, Read};

pub struct Scanner<R: Read> {
    reader: BufReader<R>,
    /// Characters pushed back into the stream, most recent last.
    pushback: Vec<u8>,
    /// Sticky end-of-file / error flag, like the FILE stream's EOF indicator.
    eof: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(inner: R) -> Self {
        Scanner {
            reader: BufReader::new(inner),
            pushback: Vec::new(),
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.pop() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.reader.read(&mut buf) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => Some(buf[0]),
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn unread(&mut self, b: u8) {
        self.pushback.push(b);
    }

    /// `isspace()` for the C locale.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Skips leading whitespace. Returns `false` if end-of-input was reached
    /// first (an input failure for the conversion).
    fn skip_whitespace(&mut self) -> bool {
        loop {
            match self.next_byte() {
                Some(b) if Self::is_space(b) => continue,
                Some(b) => {
                    self.unread(b);
                    return true;
                }
                None => return false,
            }
        }
    }

    /// Reads `[+-]?[0-9]+`. Returns `(negative, magnitude, overflowed)`.
    fn scan_decimal(&mut self) -> Option<(bool, u128, bool)> {
        if !self.skip_whitespace() {
            return None;
        }

        let mut negative = false;
        match self.next_byte() {
            Some(b @ b'+') | Some(b @ b'-') => {
                negative = b == b'-';
            }
            Some(b) => self.unread(b),
            None => return None,
        }

        // Guard so the accumulator can never overflow u128; anything past
        // 2^64 already saturates for both %u and %d.
        const LIMIT: u128 = 1u128 << 96;

        let mut magnitude: u128 = 0;
        let mut overflowed = false;
        let mut digits = 0usize;

        loop {
            match self.next_byte() {
                Some(b) if b.is_ascii_digit() => {
                    digits += 1;
                    if !overflowed {
                        magnitude = magnitude * 10 + u128::from(b - b'0');
                        if magnitude >= LIMIT {
                            overflowed = true;
                        }
                    }
                }
                Some(b) => {
                    self.unread(b);
                    break;
                }
                None => break,
            }
        }

        if digits == 0 {
            // Matching failure. glibc only pushes back the single offending
            // character (already done in the loop above); an accepted sign
            // character is consumed and lost.
            return None;
        }

        Some((negative, magnitude, overflowed))
    }

    /// `scanf("%u", &dest)` for an `unsigned int` destination.
    pub fn scan_u32(&mut self) -> Option<u32> {
        let (negative, magnitude, overflowed) = self.scan_decimal()?;

        let value: u64 = if overflowed || magnitude > u128::from(u64::MAX) {
            // strtoul() saturates to ULONG_MAX on overflow, sign included.
            u64::MAX
        } else {
            let m = magnitude as u64;
            if negative {
                m.wrapping_neg()
            } else {
                m
            }
        };

        Some(value as u32)
    }

    /// `scanf("%d", &dest)` for an `int` destination.
    pub fn scan_i32(&mut self) -> Option<i32> {
        let (negative, magnitude, overflowed) = self.scan_decimal()?;

        let value: i64 = if negative {
            let limit = u128::from(i64::MAX as u64) + 1; // 2^63
            if overflowed || magnitude >= limit {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if overflowed || magnitude > u128::from(i64::MAX as u64) {
            i64::MAX
        } else {
            magnitude as i64
        };

        Some(value as i32)
    }
}
