//! `scanf`-compatible tokenised reader for stdin.
//!
//! `scanf` conversions skip leading whitespace (including newlines) and then
//! consume the longest valid numeric prefix, leaving the first non-matching
//! character in the stream.  Overflowing values behave like glibc: the
//! underlying `strtol`/`strtoul` result saturates at `long`/`unsigned long`
//! range and is then truncated to the destination type.

use std::io::{self, Read};

pub struct Scanner<R: Read> {
    reader: R,
    buf: Vec<u8>,
    pos: usize,
    filled: usize,
    eof: bool,
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
        if self.pos == self.filled {
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
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => return self.peek(),
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

    /// Reads one integer token.  Returns the magnitude saturated to
    /// `unsigned long` range together with the sign, or `None` on a matching /
    /// input failure (which is what makes `scanf` return less than 1).
    fn read_integer(&mut self) -> Option<(bool, u64)> {
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
        let mut value: u64 = 0;
        let mut overflow = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            self.bump();
            digits += 1;
            let d = u64::from(c - b'0');
            match value.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => value = v,
                None => overflow = true,
            }
        }

        if digits == 0 {
            return None;
        }
        if overflow {
            value = u64::MAX;
        }
        Some((negative, value))
    }

    /// `scanf("%d", ...)`
    pub fn scan_int(&mut self) -> Option<i32> {
        let (negative, magnitude) = self.read_integer()?;
        // strtol saturates at LONG_MIN / LONG_MAX, the result is then stored
        // into an int (truncating the upper bits).
        let as_long: i64 = if negative {
            if magnitude > (i64::MAX as u64) + 1 {
                i64::MIN
            } else if magnitude == (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };
        Some(as_long as i32)
    }

    /// `scanf("%u", ...)`
    pub fn scan_uint(&mut self) -> Option<u32> {
        let value = self.scan_unsigned_long()?;
        Some(value as u32)
    }

    /// `scanf("%zu", ...)`
    pub fn scan_size(&mut self) -> Option<usize> {
        let value = self.scan_unsigned_long()?;
        Some(value as usize)
    }

    /// strtoul semantics: saturate at ULONG_MAX, negate modulo 2^64.
    fn scan_unsigned_long(&mut self) -> Option<u64> {
        let (negative, magnitude) = self.read_integer()?;
        Some(if negative {
            magnitude.wrapping_neg()
        } else {
            magnitude
        })
    }
}
