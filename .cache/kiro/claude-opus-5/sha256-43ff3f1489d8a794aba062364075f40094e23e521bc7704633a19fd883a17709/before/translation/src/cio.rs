//! C stdio input semantics: `fgets` and the `sscanf("%d", ...)` conversion.

use std::io::Read;

/// A byte-at-a-time reader over stdin, so that `fgets` consumes exactly the
/// bytes C would consume and leaves the rest in the stream.
pub struct Stdin {
    inner: std::io::Stdin,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl Stdin {
    pub fn new() -> Stdin {
        Stdin {
            inner: std::io::stdin(),
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if self.pos == self.buf.len() {
            if self.eof {
                return None;
            }
            self.buf.clear();
            self.pos = 0;
            let mut chunk = [0u8; 4096];
            loop {
                match self.inner.read(&mut chunk) {
                    Ok(0) => {
                        self.eof = true;
                        return None;
                    }
                    Ok(n) => {
                        self.buf.extend_from_slice(&chunk[..n]);
                        break;
                    }
                    // C's fgets retries on EINTR via the stdio layer.
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        self.eof = true;
                        return None;
                    }
                }
            }
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    /// `fgets(dst, size, stdin)`
    ///
    /// Reads at most `size - 1` bytes, stopping early after a `'\n'` (which is
    /// kept in the buffer). Returns `None` for the NULL return: end of input
    /// with nothing read. The returned `Vec` is the C string contents *without*
    /// the terminating NUL; `size` is the full array length including the NUL
    /// slot, matching `sizeof(input)` at the call site.
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let limit = size - 1;
        let mut out: Vec<u8> = Vec::new();
        while out.len() < limit {
            match self.next_byte() {
                Some(b) => {
                    out.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

/// `sscanf(s, "%d", &out)`
///
/// Returns the number of successful assignments (0 or 1). Mirrors glibc: skip
/// leading whitespace, take an optional sign and a run of decimal digits, then
/// convert with `strtol` semantics (saturating at `long` bounds) and assign the
/// result to an `int`, truncating the low 32 bits.
pub fn sscanf_int(s: &[u8]) -> (i32, i32) {
    // Only the bytes before the first NUL are part of the C string.
    let s = match s.iter().position(|&b| b == 0) {
        Some(i) => &s[..i],
        None => s,
    };

    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let start = i;
    // strtol saturation: accumulate in i64 and clamp.
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = i64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == start {
        // No digits consumed: matching failure, nothing assigned.
        return (0, 0);
    }

    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // `acc` cannot exceed i64::MAX here, so negation is representable.
        -acc
    } else {
        acc
    };

    (1, value as i32)
}

/// C's `isspace` for the default locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}
