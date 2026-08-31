// Rust translation of c_src/src/main.c
//
// Original behavior:
//   int x = 0;
//   scanf("%d", &x);      // on matching failure / EOF, x keeps its value (0)
//   driver(x);            // memcpy the int into a char[4] and hex-dump it
//
// The hex dump uses the host byte order (memcpy of the raw object
// representation), so `to_ne_bytes` is used here.

use std::io::{self, Read, Write};

/// Reader that hands out one byte at a time and supports a single pushback,
/// mirroring the getc/ungetc pair that glibc's scanf uses.
struct ByteReader<R: Read> {
    inner: R,
    buf: [u8; 4096],
    pos: usize,
    len: usize,
    eof: bool,
    pushed_back: Option<u8>,
}

impl<R: Read> ByteReader<R> {
    fn new(inner: R) -> Self {
        ByteReader {
            inner,
            buf: [0u8; 4096],
            pos: 0,
            len: 0,
            eof: false,
            pushed_back: None,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushed_back.take() {
            return Some(b);
        }
        loop {
            if self.pos < self.len {
                let b = self.buf[self.pos];
                self.pos += 1;
                return Some(b);
            }
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
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn push_back(&mut self, b: u8) {
        self.pushed_back = Some(b);
    }
}

/// C's isspace() for the "C" locale, which is the set of characters that a
/// scanf conversion skips before an integer conversion.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion and `None` on a matching
/// failure or input failure (in which case the caller leaves `x` untouched,
/// exactly like C).
///
/// glibc converts the collected digit sequence with `strtol` (a 64-bit `long`
/// on this platform), which saturates at LONG_MAX / LONG_MIN on overflow, and
/// then truncates the result to `int`. That quirk is reproduced here rather
/// than "fixed".
fn scanf_i32<R: Read>(r: &mut ByteReader<R>) -> Option<i32> {
    // Skip leading whitespace (this crosses newlines, just like scanf).
    let mut b = loop {
        match r.next_byte() {
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
            None => return None, // input failure
        }
    };

    let mut negative = false;
    if b == b'+' || b == b'-' {
        negative = b == b'-';
        match r.next_byte() {
            Some(c) => b = c,
            None => return None, // sign then EOF: matching failure
        }
    }

    if !b.is_ascii_digit() {
        r.push_back(b);
        return None; // matching failure
    }

    let mut acc: i64 = 0;
    let mut overflow = false;
    loop {
        let digit = (b - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        match r.next_byte() {
            Some(c) if c.is_ascii_digit() => b = c,
            Some(c) => {
                r.push_back(c);
                break;
            }
            None => break,
        }
    }

    let as_long: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        acc.wrapping_neg()
    } else {
        acc
    };

    // strtol result assigned through an `int *`: plain truncation.
    Some(as_long as i32)
}

fn print_hex(p: &[u8], out: &mut impl Write) {
    let mut s = String::with_capacity(p.len() * 2 + 1);
    for &byte in p {
        s.push_str(&format!("{:02x}", byte));
    }
    s.push('\n');
    let _ = out.write_all(s.as_bytes());
}

fn driver(x: i32, out: &mut impl Write) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw = x.to_ne_bytes();
    print_hex(&raw, out);
}

fn main() {
    let mut x: i32 = 0;

    let stdin = io::stdin();
    let mut reader = ByteReader::new(stdin.lock());
    if let Some(v) = scanf_i32(&mut reader) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(x, &mut out);
    let _ = out.flush();
}
