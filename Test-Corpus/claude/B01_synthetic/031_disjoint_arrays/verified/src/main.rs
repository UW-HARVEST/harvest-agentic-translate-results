use std::io::{self, Read, Write};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out = vec![0i32; len];
    let mut ones = vec![0i32; len];
    let mut zeros = vec![0i32; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, &data[..len], &zeros, len);
    out[len - 1]
}

/// A simple scanf-like reader for `%d` format.
/// Reads all of stdin and returns an iterator of byte positions.
struct Scanner {
    buf: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).ok();
        Scanner { buf, pos: 0 }
    }

    /// Mimics `scanf("%d", &x)`. Returns Some(value) on success, None on EOF/match failure.
    fn read_i32(&mut self) -> Option<i32> {
        // Skip leading whitespace (matches isspace: space, tab, newline, vertical tab, form feed, carriage return)
        while self.pos < self.buf.len() && is_space(self.buf[self.pos]) {
            self.pos += 1;
        }
        if self.pos >= self.buf.len() {
            return None;
        }

        let start = self.pos;
        let mut sign: i64 = 1;

        if self.buf[self.pos] == b'+' {
            self.pos += 1;
        } else if self.buf[self.pos] == b'-' {
            sign = -1;
            self.pos += 1;
        }

        let digits_start = self.pos;
        while self.pos < self.buf.len() && self.buf[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        if self.pos == digits_start {
            // No digits read; matching failure. Reset position to start.
            self.pos = start;
            return None;
        }

        let mut value: i64 = 0;
        for &b in &self.buf[digits_start..self.pos] {
            value = value.wrapping_mul(10).wrapping_add((b - b'0') as i64);
        }
        value = value.wrapping_mul(sign);
        Some(value as i32)
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn main() {
    let mut scanner = Scanner::new();
    let mut data = [0i32; 100];
    let mut i: usize = 0;
    while i < 100 {
        match scanner.read_i32() {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    let result = call_fma(&data, i);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{}", result).unwrap();
}
