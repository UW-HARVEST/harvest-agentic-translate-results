// Rust translation of c_src/src/main.c
//
// Original C:
//     static void print_hex(unsigned char *p, int len);
//     void driver(int x) { print_hex((unsigned char *)&x, sizeof(x)); }
//     int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
//
// Behavior reproduced:
//   * `scanf("%d", &x)` skips leading whitespace (including newlines), accepts an
//     optional sign and one or more decimal digits. On a matching failure or EOF
//     the C variable is left untouched, so `x` stays 0.
//   * glibc implements `%d` by collecting the digit run and handing it to
//     `strtol`, so the value saturates at `long` range and is then truncated to
//     `int`. That truncation is reproduced here (e.g. "4294967296" -> 0).
//   * The bytes of the `int` are printed in native (host) order, two lowercase
//     hex digits each, followed by a single newline.

use std::io::{self, Read, Write};

/// Whitespace set used by C's `isspace` in the default "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Byte-at-a-time reader over stdin so that parsing consumes exactly the bytes
/// `scanf` would consume (and can "unread" one lookahead byte).
struct ByteReader<R: Read> {
    inner: R,
    buf: [u8; 4096],
    pos: usize,
    len: usize,
    eof: bool,
    peeked: Option<u8>,
}

impl<R: Read> ByteReader<R> {
    fn new(inner: R) -> Self {
        ByteReader {
            inner,
            buf: [0u8; 4096],
            pos: 0,
            len: 0,
            eof: false,
            peeked: None,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
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

    /// Push a byte back so the next read returns it, mirroring `ungetc`.
    fn unread(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// Emulates `scanf("%d", out)`. Returns true when a value was assigned.
fn scan_int<R: Read>(r: &mut ByteReader<R>, out: &mut i32) -> bool {
    // Skip leading whitespace; whitespace directives cross newlines.
    let mut cur = loop {
        match r.next_byte() {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return false, // input failure
        }
    };

    let mut negative = false;
    if cur == b'+' || cur == b'-' {
        negative = cur == b'-';
        match r.next_byte() {
            Some(b) => cur = b,
            None => return false, // sign with no digits: matching failure
        }
    }

    if !cur.is_ascii_digit() {
        // Matching failure: glibc pushes the offending character back and the
        // argument is left unmodified.
        r.unread(cur);
        return false;
    }

    // Accumulate the magnitude with saturation, as `strtol` does.
    let mut magnitude: u128 = 0;
    let mut saturated = false;
    loop {
        let digit = (cur - b'0') as u128;
        if !saturated {
            magnitude = magnitude * 10 + digit;
            // Once past the `long` range there is no way back; clamp.
            if magnitude > u128::from(i64::MAX as u64) + 1 {
                saturated = true;
            }
        }
        match r.next_byte() {
            Some(b) if b.is_ascii_digit() => cur = b,
            Some(b) => {
                r.unread(b);
                break;
            }
            None => break,
        }
    }

    // strtol clamps to [LONG_MIN, LONG_MAX]; the result is then converted to int.
    let as_long: i64 = if negative {
        if saturated || magnitude > u128::from(i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            (magnitude as i64).wrapping_neg()
        }
    } else if saturated || magnitude > i64::MAX as u128 {
        i64::MAX
    } else {
        magnitude as i64
    };

    *out = as_long as i32; // implicit narrowing conversion, as in glibc
    true
}

fn print_hex(out: &mut impl Write, p: &[u8]) -> io::Result<()> {
    for &byte in p {
        write!(out, "{:02x}", byte)?;
    }
    writeln!(out)
}

fn driver(out: &mut impl Write, x: i32) -> io::Result<()> {
    // Reinterprets the int's storage bytes in host order, like the C cast.
    print_hex(out, &x.to_ne_bytes())
}

fn main() {
    let mut x: i32 = 0;

    let stdin = io::stdin();
    let mut reader = ByteReader::new(stdin.lock());
    let _ = scan_int(&mut reader, &mut x);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = driver(&mut out, x);
    let _ = out.flush();
}
