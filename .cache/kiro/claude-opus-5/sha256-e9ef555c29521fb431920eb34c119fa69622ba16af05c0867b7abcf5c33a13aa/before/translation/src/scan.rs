//! Emulation of the subset of C `scanf` behavior used by the original program.
//!
//! Only the `%d` conversion is needed. The important properties that must be
//! preserved to stay byte-identical with the C original:
//!
//! * Leading whitespace (including newlines) is skipped, so a conversion may
//!   span line boundaries.
//! * A conversion returns "1 item assigned" only when at least one digit was
//!   consumed; EOF-before-any-input and a matching failure are both reported as
//!   "not 1", which is all the caller ever checks.
//! * On a matching failure the offending byte is *not* consumed (glibc pushes it
//!   back), so every later conversion fails as well.
//! * Out-of-range values follow glibc: the magnitude saturates at the `long`
//!   limits and the result is then truncated to `int`.

use std::io::Read;

pub struct Scanner<R: Read> {
    inner: R,
    buf: Box<[u8; 8192]>,
    pos: usize,
    len: usize,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(inner: R) -> Self {
        Scanner {
            inner,
            buf: Box::new([0u8; 8192]),
            pos: 0,
            len: 0,
            eof: false,
        }
    }

    /// Look at the next byte without consuming it.
    fn peek(&mut self) -> Option<u8> {
        while self.pos >= self.len {
            if self.eof {
                return None;
            }
            match self.inner.read(&mut self.buf[..]) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.len = n;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
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

    /// Matches the `isspace()` set that `scanf` skips.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    }

    /// `scanf("%d", &out)`: returns the number of items assigned (0 or 1).
    pub fn scan_int(&mut self, out: &mut i32) -> i32 {
        loop {
            match self.peek() {
                Some(b) if Self::is_space(b) => self.bump(),
                _ => break,
            }
        }

        let mut negative = false;
        match self.peek() {
            None => return 0, // input failure (EOF): C reports EOF, caller only checks != 1
            Some(b'+') => {
                self.bump();
            }
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            Some(_) => {}
        }

        // At least one digit is required for the conversion to succeed.
        let mut saw_digit = false;
        // Magnitude is accumulated with a cap; anything beyond `long` range
        // saturates exactly like glibc's internal `strtol`.
        let mut mag: u128 = 0;
        const CAP: u128 = 1u128 << 80;
        while let Some(b) = self.peek() {
            if !b.is_ascii_digit() {
                break;
            }
            saw_digit = true;
            if mag < CAP {
                mag = mag * 10 + u128::from(b - b'0');
            }
            self.bump();
        }

        if !saw_digit {
            // Matching failure. The offending byte stays in the stream.
            return 0;
        }

        let as_long: i64 = if negative {
            if mag > (i64::MAX as u128) + 1 {
                i64::MIN
            } else if mag == (i64::MAX as u128) + 1 {
                i64::MIN
            } else {
                -(mag as i64)
            }
        } else if mag > i64::MAX as u128 {
            i64::MAX
        } else {
            mag as i64
        };

        *out = as_long as i32;
        1
    }
}
