use std::io::{self, Read, Write, BufWriter};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn driver(out: &mut [i32], len: usize, w: &mut impl Write) {
    // fma_array(out, out, out, out, len) - same buffer for all
    // Replicate aliasing behavior: out[i] = out[i] * out[i] + out[i]
    for i in 0..len {
        out[i] = out[i].wrapping_mul(out[i]).wrapping_add(out[i]);
    }
    let _ = fma_array; // silence unused warning if optimized
    for i in 0..len {
        writeln!(w, "{}", out[i]).unwrap();
    }
}

/// Mimic scanf("%d", ...) behavior:
/// - Skip leading whitespace
/// - Read optional sign
/// - Read digits
/// - Returns Some(value) on success, None on EOF or no digits
/// - Parses with C-style int wrap (i32)
struct Scanner<R: Read> {
    reader: R,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(reader: R) -> Self {
        Scanner { reader, buf: Vec::new(), pos: 0, eof: false }
    }

    fn fill(&mut self) -> bool {
        if self.pos < self.buf.len() {
            return true;
        }
        if self.eof {
            return false;
        }
        let mut tmp = [0u8; 4096];
        match self.reader.read(&mut tmp) {
            Ok(0) => {
                self.eof = true;
                false
            }
            Ok(n) => {
                self.buf.clear();
                self.buf.extend_from_slice(&tmp[..n]);
                self.pos = 0;
                true
            }
            Err(_) => {
                self.eof = true;
                false
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        if !self.fill() {
            return None;
        }
        Some(self.buf[self.pos])
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn scan_i32(&mut self) -> Option<i32> {
        // Skip whitespace
        loop {
            match self.peek() {
                Some(b) if b.is_ascii_whitespace() => self.advance(),
                Some(_) => break,
                None => return None,
            }
        }

        let mut sign: i64 = 1;
        match self.peek() {
            Some(b'+') => { self.advance(); }
            Some(b'-') => { sign = -1; self.advance(); }
            _ => {}
        }

        let mut have_digit = false;
        let mut value: i64 = 0;
        loop {
            match self.peek() {
                Some(b) if b.is_ascii_digit() => {
                    have_digit = true;
                    value = value.wrapping_mul(10).wrapping_add((b - b'0') as i64);
                    self.advance();
                }
                _ => break,
            }
        }

        if !have_digit {
            return None;
        }

        let result = (value.wrapping_mul(sign)) as i32;
        Some(result)
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut scanner = Scanner::new(stdin.lock());
    let mut data: [i32; 100] = [0; 100];
    let mut i: usize = 0;
    while i < 100 {
        match scanner.scan_i32() {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    driver(&mut data, i, &mut out);
}
