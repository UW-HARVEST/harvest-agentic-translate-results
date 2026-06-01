use std::io::{self, Read};

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

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

/// Reads integers from stdin in scanf("%d", ...) style:
/// - skips leading whitespace (including newlines)
/// - parses an optional sign followed by decimal digits
/// - returns parsed value or None if no integer could be read
struct ScanfReader {
    buf: Vec<u8>,
    pos: usize,
}

impl ScanfReader {
    fn new() -> io::Result<Self> {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        Ok(Self { buf, pos: 0 })
    }

    fn read_int(&mut self) -> Option<i32> {
        // Skip whitespace (matches isspace: space, \t, \n, \v, \f, \r)
        while self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\x0b' || c == b'\x0c' || c == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
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
        while self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == digits_start {
            // No digits found; rewind
            self.pos = start;
            return None;
        }

        let mut value: i64 = 0;
        for &c in &self.buf[digits_start..self.pos] {
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
        }
        value = value.wrapping_mul(sign);
        Some(value as i32)
    }
}

fn main() {
    let mut reader = match ScanfReader::new() {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut data: [i32; 100] = [0; 100];
    let mut i: usize = 0;
    while i < 100 {
        match reader.read_int() {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    let result = call_fma(&data, i);
    println!("{}", result);
}
