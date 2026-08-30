//! `scanf`-compatible tokenised reader for stdin.
//!
//! Every conversion `main` uses (`%d`, `%u`, `%zu`) skips leading whitespace,
//! newlines included, then consumes the longest valid numeric prefix and leaves
//! the first non-matching character in the stream.  Nothing here is line
//! oriented: `scanf` reads straight across newlines, so the shape of the input
//! is irrelevant and only the token sequence matters.
//!
//! Out-of-range values follow glibc, which converts the collected digits with
//! `strtol` / `strtoul` and then stores the result into the (narrower)
//! destination:
//!
//! * `strtol` saturates at `LONG_MIN` / `LONG_MAX`;
//! * `strtoul` saturates at `ULONG_MAX` **whether or not a minus sign was
//!   present** - the sign is only applied to a magnitude that fitted;
//! * the saturated value is then truncated to `int`, `unsigned int` or `size_t`.

use std::io::{self, Read};

pub struct Scanner<R: Read> {
    reader: R,
    buf: Vec<u8>,
    pos: usize,
    filled: usize,
    eof: bool,
}

/// One parsed integer token: sign, magnitude, and whether the magnitude itself
/// exceeded 64 bits (in which case `magnitude` has been clamped).
struct Token {
    negative: bool,
    magnitude: u64,
    overflow: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(reader: R) -> Self {
        Scanner {
            reader,
            buf: vec![0u8; 8192],
            pos: 0,
            filled: 0,
            eof: false,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        while self.pos == self.filled {
            if self.eof {
                return None;
            }
            match self.reader.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.filled = n;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        Some(self.buf[self.pos])
    }

    fn bump(&mut self) {
        if self.pos < self.filled {
            self.pos += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => self.bump(),
                _ => break,
            }
        }
    }

    /// Reads one integer token, or `None` on a matching / input failure - which
    /// is what makes `scanf` return less than the number of conversions asked
    /// for.
    fn read_integer(&mut self) -> Option<Token> {
        self.skip_whitespace();

        let mut negative = false;
        match self.peek() {
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            Some(b'+') => self.bump(),
            Some(_) => {}
            None => return None,
        }

        let mut digits = 0usize;
        let mut magnitude: u64 = 0;
        let mut overflow = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            self.bump();
            digits += 1;
            let d = u64::from(c - b'0');
            match magnitude.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => magnitude = v,
                None => overflow = true,
            }
        }

        if digits == 0 {
            return None;
        }
        if overflow {
            magnitude = u64::MAX;
        }
        Some(Token {
            negative,
            magnitude,
            overflow,
        })
    }

    /// `scanf("%d", ...)`: `strtol` saturated to `long`, stored into an `int`.
    pub fn scan_int(&mut self) -> Option<i32> {
        let t = self.read_integer()?;
        const LONG_MAX: u64 = i64::MAX as u64;
        let as_long: i64 = if t.negative {
            if t.overflow || t.magnitude > LONG_MAX + 1 {
                i64::MIN
            } else if t.magnitude == LONG_MAX + 1 {
                i64::MIN
            } else {
                -(t.magnitude as i64)
            }
        } else if t.overflow || t.magnitude > LONG_MAX {
            i64::MAX
        } else {
            t.magnitude as i64
        };
        Some(as_long as i32)
    }

    /// `scanf("%u", ...)`: `strtoul` saturated to `unsigned long`, stored into
    /// an `unsigned int`.
    pub fn scan_uint(&mut self) -> Option<u32> {
        Some(self.scan_unsigned_long()? as u32)
    }

    /// `scanf("%zu", ...)`: `strtoul` saturated to `unsigned long`, stored into
    /// a `size_t` (the same width here).
    pub fn scan_size(&mut self) -> Option<usize> {
        Some(self.scan_unsigned_long()? as usize)
    }

    /// `strtoul`: a magnitude that does not fit becomes `ULONG_MAX` regardless
    /// of sign; otherwise a minus sign negates modulo 2^64.
    fn scan_unsigned_long(&mut self) -> Option<u64> {
        let t = self.read_integer()?;
        Some(if t.overflow {
            u64::MAX
        } else if t.negative {
            t.magnitude.wrapping_neg()
        } else {
            t.magnitude
        })
    }
}
