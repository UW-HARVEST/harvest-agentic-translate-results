//! Small helpers that reproduce the exact byte output of the `printf`
//! conversions used by jansson's error reporting code.

use core::ffi::c_char;

/// A fixed capacity buffer with `snprintf()` truncation semantics: at most
/// `N - 1` bytes are stored (room is left for the terminating NUL that the C
/// code relies on).
pub struct CBuf<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> CBuf<N> {
    pub fn new() -> Self {
        CBuf {
            buf: [0u8; N],
            len: 0,
        }
    }

    fn push_byte(&mut self, b: u8) {
        if self.len < N - 1 {
            self.buf[self.len] = b;
            self.len += 1;
        }
    }

    pub fn push(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.push_byte(b);
        }
    }

    /// `%c`
    pub fn push_char(&mut self, c: u8) {
        self.push_byte(c);
    }

    /// `%s` for a NUL terminated C string.
    pub unsafe fn push_cstr(&mut self, s: *const c_char) {
        let mut p = s as *const u8;
        loop {
            let b = *p;
            if b == 0 {
                break;
            }
            self.push_byte(b);
            p = p.add(1);
        }
    }

    /// `%.Ns` for a NUL terminated C string.
    pub unsafe fn push_cstr_prec(&mut self, s: *const c_char, prec: usize) {
        let mut p = s as *const u8;
        let mut i = 0;
        while i < prec {
            let b = *p;
            if b == 0 {
                break;
            }
            self.push_byte(b);
            p = p.add(1);
            i += 1;
        }
    }

    /// `%d` / `%li`
    pub fn push_dec_i64(&mut self, v: i64) {
        let mut tmp = [0u8; 24];
        let neg = v < 0;
        let mut uv = if neg {
            (v as i128).unsigned_abs() as u128
        } else {
            v as u128
        };
        let mut i = tmp.len();
        if uv == 0 {
            i -= 1;
            tmp[i] = b'0';
        }
        while uv > 0 {
            i -= 1;
            tmp[i] = b'0' + (uv % 10) as u8;
            uv /= 10;
        }
        if neg {
            self.push_byte(b'-');
        }
        self.push(&tmp[i..]);
    }

    /// `%u` / `%lu`
    pub fn push_dec_u64(&mut self, mut v: u64) {
        let mut tmp = [0u8; 24];
        let mut i = tmp.len();
        if v == 0 {
            i -= 1;
            tmp[i] = b'0';
        }
        while v > 0 {
            i -= 1;
            tmp[i] = b'0' + (v % 10) as u8;
            v /= 10;
        }
        self.push(&tmp[i..]);
    }

    /// `%x`
    pub fn push_hex_lower(&mut self, mut v: u32) {
        const D: &[u8; 16] = b"0123456789abcdef";
        let mut tmp = [0u8; 8];
        let mut i = tmp.len();
        if v == 0 {
            i -= 1;
            tmp[i] = b'0';
        }
        while v > 0 {
            i -= 1;
            tmp[i] = D[(v & 0xf) as usize];
            v >>= 4;
        }
        self.push(&tmp[i..]);
    }

    /// The bytes written so far, including any embedded NUL bytes produced by
    /// `%c` conversions.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// The bytes written so far up to (but not including) the first NUL byte,
    /// i.e. what a subsequent `%s` conversion of this buffer would produce.
    pub fn as_cstr_bytes(&self) -> &[u8] {
        let b = self.as_bytes();
        match b.iter().position(|&c| c == 0) {
            Some(i) => &b[..i],
            None => b,
        }
    }
}

/// `\uXXXX` style upper case, zero padded to 4 digits (`%04X`).
pub fn hex_upper_pad4(v: u32, out: &mut [u8; 8]) -> usize {
    const D: &[u8; 16] = b"0123456789ABCDEF";
    let mut tmp = [0u8; 8];
    let mut i = tmp.len();
    let mut v2 = v;
    if v2 == 0 {
        i -= 1;
        tmp[i] = b'0';
    }
    while v2 > 0 {
        i -= 1;
        tmp[i] = D[(v2 & 0xf) as usize];
        v2 >>= 4;
    }
    let digits = &tmp[i..];
    let mut n = 0;
    while n + digits.len() < 4 {
        out[n] = b'0';
        n += 1;
    }
    out[n..n + digits.len()].copy_from_slice(digits);
    n + digits.len()
}
