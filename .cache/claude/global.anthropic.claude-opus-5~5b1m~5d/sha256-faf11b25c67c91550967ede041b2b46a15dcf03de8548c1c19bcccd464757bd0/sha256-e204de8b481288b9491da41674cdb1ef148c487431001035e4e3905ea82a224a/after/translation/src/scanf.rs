//! Minimal emulation of the glibc `scanf` conversions used by
//! `c_src/src/main.c`: `%d`, `%u` and `%zu`.
//!
//! Like `scanf`, each conversion first skips whitespace, then accepts an
//! optional sign followed by decimal digits and leaves the first
//! non-matching character in the stream.  Overflowing values are clamped the
//! way `strtol`/`strtoul` do (`LONG_MAX`, `LONG_MIN`, `ULONG_MAX`) and then
//! truncated to the width of the destination object.

use std::io::Read;

pub struct Scanner<R: Read> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
    len: usize,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(inner: R) -> Self {
        Scanner {
            inner,
            buf: vec![0u8; 8192],
            pos: 0,
            len: 0,
            eof: false,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        if self.pos == self.len {
            if self.eof {
                return None;
            }
            match self.inner.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.len = n;
                }
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        Some(self.buf[self.pos])
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    /// C `isspace()` in the "C" locale.
    fn is_space(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    fn skip_space(&mut self) {
        while let Some(c) = self.peek() {
            if Self::is_space(c) {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// Read an optional sign plus a run of decimal digits.
    ///
    /// Returns `(negative, magnitude, overflowed)` or `None` on a matching
    /// failure / end of input.
    fn read_number(&mut self) -> Option<(bool, u64, bool)> {
        self.skip_space();

        let mut negative = false;
        match self.peek() {
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            Some(b'+') => {
                self.bump();
            }
            Some(_) => {}
            None => return None,
        }

        let mut digits = 0usize;
        let mut magnitude: u64 = 0;
        let mut overflowed = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            self.bump();
            digits += 1;
            let d = (c - b'0') as u64;
            match magnitude.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => magnitude = v,
                None => overflowed = true,
            }
        }

        if digits == 0 {
            /* Matching failure: no digits were converted. */
            return None;
        }

        Some((negative, magnitude, overflowed))
    }

    /// `scanf("%d", &int)`
    pub fn read_int(&mut self) -> Option<i32> {
        let (negative, magnitude, overflowed) = self.read_number()?;
        /* strtol() clamps to LONG_MAX / LONG_MIN on overflow, the result is
         * then stored into an `int`, i.e. truncated. */
        let value: i64 = if negative {
            if overflowed || magnitude > (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                (magnitude as i128).wrapping_neg() as i64
            }
        } else if overflowed || magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };
        Some(value as i32)
    }

    /// `scanf("%u", &unsigned int)`
    pub fn read_uint(&mut self) -> Option<u32> {
        Some(self.read_ulong()? as u32)
    }

    /// `scanf("%zu", &size_t)`
    pub fn read_usize(&mut self) -> Option<usize> {
        Some(self.read_ulong()? as usize)
    }

    /// strtoul() semantics: ULONG_MAX on overflow (whatever the sign),
    /// otherwise the (possibly negated) magnitude.
    fn read_ulong(&mut self) -> Option<u64> {
        let (negative, magnitude, overflowed) = self.read_number()?;
        if overflowed {
            return Some(u64::MAX);
        }
        Some(if negative {
            magnitude.wrapping_neg()
        } else {
            magnitude
        })
    }
}
