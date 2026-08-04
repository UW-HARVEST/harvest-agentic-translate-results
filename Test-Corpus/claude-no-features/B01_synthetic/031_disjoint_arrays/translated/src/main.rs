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

/// Mimics C scanf("%d", ...) behavior:
/// - Skips leading whitespace (including newlines)
/// - Optional sign (+/-)
/// - Reads digits
/// - Returns None if no integer could be parsed (EOF or non-digit input)
struct ScanfReader {
    bytes: Vec<u8>,
    pos: usize,
}

impl ScanfReader {
    fn new() -> Self {
        let mut bytes = Vec::new();
        io::stdin().read_to_end(&mut bytes).ok();
        ScanfReader { bytes, pos: 0 }
    }

    fn read_int(&mut self) -> Option<i32> {
        // Skip whitespace
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos >= self.bytes.len() {
            return None;
        }

        let start = self.pos;
        let mut negative = false;

        // Optional sign
        if self.bytes[self.pos] == b'+' {
            self.pos += 1;
        } else if self.bytes[self.pos] == b'-' {
            negative = true;
            self.pos += 1;
        }

        let digits_start = self.pos;
        let mut value: i64 = 0;
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c.is_ascii_digit() {
                value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == digits_start {
            // No digits read; matching failure - rewind to start
            self.pos = start;
            return None;
        }

        if negative {
            value = -value;
        }

        Some(value as i32)
    }
}

fn main() {
    let mut reader = ScanfReader::new();
    let mut data = [0i32; 100];
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
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", result);
}
