// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output for the same inputs.

use std::io::{self, Read, Write, BufWriter};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out: Vec<i32> = vec![0; len];
    let mut ones: Vec<i32> = vec![0; len];
    let mut zeros: Vec<i32> = vec![0; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

/// A minimal `scanf("%d", ...)`-like reader that reads a single signed integer
/// from a byte stream, returning the integer on success or `None` on EOF/parse
/// failure. Skips leading whitespace (matching C's `scanf %d` behavior).
struct Scanner<R: Read> {
    inner: R,
    peeked: Option<u8>,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(inner: R) -> Self {
        Scanner { inner, peeked: None, eof: false }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
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

    fn unread_byte(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Reads a `%d` integer. Returns `Some(value)` on success (i.e. scanf
    /// would have returned 1), or `None` on matching failure / EOF.
    fn read_int(&mut self) -> Option<i32> {
        // Skip leading whitespace.
        let mut b = loop {
            match self.read_byte() {
                Some(c) if (c as char).is_ascii_whitespace() => continue,
                Some(c) => break c,
                None => return None,
            }
        };

        let mut negative = false;
        if b == b'+' || b == b'-' {
            negative = b == b'-';
            match self.read_byte() {
                Some(c) => b = c,
                None => return None, // matching failure
            }
        }

        if !b.is_ascii_digit() {
            // Matching failure — push back the offending byte.
            self.unread_byte(b);
            return None;
        }

        // Accumulate digits using i64 to avoid panic on overflow; truncate to
        // i32 with wrapping behavior to mirror typical C undefined-on-overflow
        // results in a deterministic way.
        let mut acc: i64 = 0;
        loop {
            acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as i64);
            match self.read_byte() {
                Some(c) if c.is_ascii_digit() => b = c,
                Some(c) => {
                    self.unread_byte(c);
                    break;
                }
                None => break,
            }
        }

        let value = if negative { acc.wrapping_neg() } else { acc };
        Some(value as i32)
    }
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    let mut data = [0i32; 100];
    let mut i: usize = 0;
    while i < 100 {
        match scanner.read_int() {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    let result = call_fma(&data, i);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    writeln!(out, "{}", result).unwrap();
}
