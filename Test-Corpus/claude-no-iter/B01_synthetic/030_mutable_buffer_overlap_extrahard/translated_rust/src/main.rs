// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to match the original C implementation byte-for-byte.

use std::io::{self, BufRead, BufReader, Read, Write, BufWriter};

/// Reader that mimics C's scanf %d behavior: skip leading whitespace,
/// read optional sign + digits, and "push back" the first non-digit byte.
struct ScanfReader<R: Read> {
    inner: BufReader<R>,
    pushback: Option<u8>,
    eof: bool,
}

impl<R: Read> ScanfReader<R> {
    fn new(inner: R) -> Self {
        ScanfReader {
            inner: BufReader::new(inner),
            pushback: None,
            eof: false,
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let buf = match self.inner.fill_buf() {
            Ok(b) => b,
            Err(_) => {
                self.eof = true;
                return None;
            }
        };
        if buf.is_empty() {
            self.eof = true;
            return None;
        }
        let byte = buf[0];
        self.inner.consume(1);
        Some(byte)
    }

    fn unread(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    /// Read an integer in the style of scanf("%d", ...).
    /// Returns Some(value) on success, None on failure (matching whitespace
    /// skip behavior).
    fn scan_i32(&mut self) -> Option<i32> {
        // Skip whitespace (matches C isspace: space, \t, \n, \v, \f, \r)
        let mut b;
        loop {
            b = self.read_byte()?;
            if !is_c_space(b) {
                break;
            }
        }

        let mut negative = false;
        if b == b'-' {
            negative = true;
            b = match self.read_byte() {
                Some(x) => x,
                None => {
                    return None;
                }
            };
        } else if b == b'+' {
            b = match self.read_byte() {
                Some(x) => x,
                None => {
                    return None;
                }
            };
        }

        if !b.is_ascii_digit() {
            // No digits read. Push back the offending byte (so it can stop
            // future scans the same way C's scanf does).
            self.unread(b);
            return None;
        }

        // Match C's int width: use i64 internally to compute, then cast
        // (C scanf with overflow is technically undefined, but in practice
        // it wraps on common platforms). We use wrapping cast.
        let mut acc: i64 = 0;
        loop {
            acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as i64);
            match self.read_byte() {
                Some(next) => {
                    if next.is_ascii_digit() {
                        b = next;
                    } else {
                        self.unread(next);
                        break;
                    }
                }
                None => break,
            }
        }

        if negative {
            acc = acc.wrapping_neg();
        }

        Some(acc as i32)
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn driver<W: Write>(out: &mut [i32], len: usize, writer: &mut W) {
    // The C code calls fma_array(out, out, out, out, len) — all four
    // pointers alias. Each element becomes out[i]*out[i] + out[i].
    // Replicate that aliasing semantics exactly.
    for i in 0..len {
        let v = out[i];
        out[i] = v.wrapping_mul(v).wrapping_add(v);
    }
    // The unused parameter shape preserved for fidelity.
    let _ = fma_array;

    for i in 0..len {
        writeln!(writer, "{}", out[i]).expect("write failed");
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    let mut reader = ScanfReader::new(stdin.lock());
    let mut data: [i32; 100] = [0; 100];
    let mut i: usize = 0;
    while i < 100 {
        match reader.scan_i32() {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    driver(&mut data, i, &mut writer);
}
