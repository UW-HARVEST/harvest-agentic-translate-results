use std::io::{self, Read, Write};
use std::process::ExitCode;

mod lib_proc;

/// Reads whitespace-separated tokens from stdin (similar to scanf with %u/%d/%zu).
struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> io::Result<Self> {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        Ok(Scanner { data, pos: 0 })
    }

    fn next_token(&mut self) -> Option<&[u8]> {
        // Skip whitespace
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.data.len() && !self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        Some(&self.data[start..self.pos])
    }

    /// Parse %u (unsigned int) from a token. C's scanf %u accepts an optional
    /// sign and parses digits; we accept unsigned decimal with optional + or -.
    fn next_u32(&mut self) -> Option<u32> {
        let tok = self.next_token()?;
        parse_uint_c::<u32>(tok)
    }

    fn next_i32(&mut self) -> Option<i32> {
        let tok = self.next_token()?;
        parse_int_c::<i32>(tok)
    }

    fn next_usize(&mut self) -> Option<usize> {
        let tok = self.next_token()?;
        parse_uint_c::<usize>(tok)
    }
}

trait UIntC: Copy {
    fn zero() -> Self;
    fn checked_mul10_add(self, d: u32) -> Option<Self>;
    fn wrapping_neg(self) -> Self;
}

impl UIntC for u32 {
    fn zero() -> Self { 0 }
    fn checked_mul10_add(self, d: u32) -> Option<Self> {
        self.checked_mul(10).and_then(|x| x.checked_add(d))
    }
    fn wrapping_neg(self) -> Self { u32::wrapping_neg(self) }
}

impl UIntC for usize {
    fn zero() -> Self { 0 }
    fn checked_mul10_add(self, d: u32) -> Option<Self> {
        self.checked_mul(10).and_then(|x| x.checked_add(d as usize))
    }
    fn wrapping_neg(self) -> Self { usize::wrapping_neg(self) }
}

fn parse_uint_c<T: UIntC>(tok: &[u8]) -> Option<T> {
    let mut i = 0;
    let mut negative = false;
    if i < tok.len() && (tok[i] == b'+' || tok[i] == b'-') {
        negative = tok[i] == b'-';
        i += 1;
    }
    if i >= tok.len() || !tok[i].is_ascii_digit() {
        return None;
    }
    let mut val = T::zero();
    while i < tok.len() && tok[i].is_ascii_digit() {
        let d = (tok[i] - b'0') as u32;
        val = val.checked_mul10_add(d)?;
        i += 1;
    }
    if negative {
        val = val.wrapping_neg();
    }
    Some(val)
}

fn parse_int_c<T>(tok: &[u8]) -> Option<T>
where
    T: Copy,
    T: TryFrom<i64>,
{
    let mut i = 0;
    let mut negative = false;
    if i < tok.len() && (tok[i] == b'+' || tok[i] == b'-') {
        negative = tok[i] == b'-';
        i += 1;
    }
    if i >= tok.len() || !tok[i].is_ascii_digit() {
        return None;
    }
    let mut val: i64 = 0;
    while i < tok.len() && tok[i].is_ascii_digit() {
        let d = (tok[i] - b'0') as i64;
        val = val.checked_mul(10)?.checked_add(d)?;
        i += 1;
    }
    if negative {
        val = val.checked_neg()?;
    }
    T::try_from(val).ok()
}

fn main() -> ExitCode {
    let mut scanner = match Scanner::new() {
        Ok(s) => s,
        Err(_) => {
            let _ = writeln!(io::stderr(), "Error reading input");
            return ExitCode::from(1);
        }
    };

    // Read flags
    let flags: u32 = match scanner.next_u32() {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading flags");
            return ExitCode::from(1);
        }
    };

    // Read param1
    let param1: i32 = match scanner.next_i32() {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading param1");
            return ExitCode::from(1);
        }
    };

    // Read param2
    let param2: i32 = match scanner.next_i32() {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading param2");
            return ExitCode::from(1);
        }
    };

    // Read buffer length
    let length: usize = match scanner.next_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading length");
            return ExitCode::from(1);
        }
    };

    if length > 256 {
        let _ = writeln!(io::stderr(), "Error: length {} exceeds maximum 256", length);
        return ExitCode::from(1);
    }

    // Allocate a buffer big enough to absorb the worst-case growth from
    // compact_runs (threshold == 1 can double the size). The C code wrote into
    // a fixed 256-byte stack buffer, which is technically undefined behavior in
    // those edge cases. We mirror the logical behavior with a larger backing
    // store while preserving the same observable outputs.
    let mut buffer: Vec<u8> = vec![0u8; 1024];

    for i in 0..length {
        match scanner.next_u32() {
            Some(byte) => {
                buffer[i] = byte as u8;
            }
            None => {
                let _ = writeln!(io::stderr(), "Error reading byte {}", i);
                return ExitCode::from(1);
            }
        }
    }

    // Process the buffer
    let new_length = lib_proc::process_buffer(&mut buffer, length, flags, param1, param2);

    // Output new length followed by buffer contents
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}", new_length);
    for i in 0..new_length {
        let _ = write!(out, " {}", buffer[i] as u32);
    }
    let _ = writeln!(out);

    ExitCode::from(0)
}
