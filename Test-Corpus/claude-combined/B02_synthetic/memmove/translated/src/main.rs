use std::io::{self, Read, Write};

mod process;

struct Scanner<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && (self.bytes[self.pos] as char).is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    /// Reads an optionally-signed decimal integer, mimicking scanf %d/%u/%zu.
    /// Returns None if no digits are available.
    fn read_int(&mut self) -> Option<i128> {
        self.skip_whitespace();
        let start = self.pos;
        let mut neg = false;
        if self.pos < self.bytes.len()
            && (self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-')
        {
            if self.bytes[self.pos] == b'-' {
                neg = true;
            }
            self.pos += 1;
        }
        let digits_start = self.pos;
        while self.pos < self.bytes.len() && (self.bytes[self.pos] as char).is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == digits_start {
            // No digits were consumed. Restore position so a second attempt sees same input.
            self.pos = start;
            return None;
        }
        let s = std::str::from_utf8(&self.bytes[digits_start..self.pos]).ok()?;
        let mut num: i128 = 0;
        for c in s.bytes() {
            num = num.wrapping_mul(10).wrapping_add((c - b'0') as i128);
        }
        if neg {
            num = num.wrapping_neg();
        }
        Some(num)
    }

    fn read_u32(&mut self) -> Option<u32> {
        self.read_int().map(|v| v as u32)
    }

    fn read_i32(&mut self) -> Option<i32> {
        self.read_int().map(|v| v as i32)
    }

    fn read_usize(&mut self) -> Option<usize> {
        self.read_int().map(|v| v as usize)
    }
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        std::process::exit(1);
    }
    let mut scanner = Scanner::new(&input);

    let stderr = io::stderr();
    let mut err = stderr.lock();

    // Read flags
    let flags: u32 = match scanner.read_u32() {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading flags");
            std::process::exit(1);
        }
    };

    // Read param1
    let param1: i32 = match scanner.read_i32() {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading param1");
            std::process::exit(1);
        }
    };

    // Read param2
    let param2: i32 = match scanner.read_i32() {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading param2");
            std::process::exit(1);
        }
    };

    // Read buffer length
    let length: usize = match scanner.read_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading length");
            std::process::exit(1);
        }
    };

    if length > 256 {
        let _ = writeln!(err, "Error: length {} exceeds maximum 256", length);
        std::process::exit(1);
    }

    let mut buffer = [0u8; 256];

    // Read buffer data
    for i in 0..length {
        let byte_val = match scanner.read_u32() {
            Some(v) => v,
            None => {
                let _ = writeln!(err, "Error reading byte {}", i);
                std::process::exit(1);
            }
        };
        buffer[i] = byte_val as u8;
    }

    // Process the buffer
    let new_length = process::process_buffer(&mut buffer, length, flags, param1, param2);

    // Output new length
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}", new_length);

    // Output buffer contents
    for i in 0..new_length {
        let _ = write!(out, " {}", buffer[i]);
    }
    let _ = writeln!(out);
}
