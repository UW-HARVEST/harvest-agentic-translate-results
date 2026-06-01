use std::io::{self, Read, Write};

mod lib_strings;

const MAX_BUFFER_SIZE: usize = 1024;

/// Reads from stdin, splitting by ASCII whitespace (matching scanf %d/%u/%zu behavior
/// across newlines and spaces). Returns tokens lazily.
struct ScanfReader {
    data: Vec<u8>,
    pos: usize,
}

impl ScanfReader {
    fn new() -> io::Result<Self> {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        Ok(Self { data, pos: 0 })
    }

    fn next_token(&mut self) -> Option<&[u8]> {
        // Skip leading whitespace (matching C's isspace for scanf)
        while self.pos < self.data.len() && is_ascii_whitespace(self.data[self.pos]) {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.data.len() && !is_ascii_whitespace(self.data[self.pos]) {
            self.pos += 1;
        }
        Some(&self.data[start..self.pos])
    }

    fn next_i32(&mut self) -> Option<i32> {
        let tok = self.next_token()?;
        parse_signed_int(tok)
    }

    fn next_u32(&mut self) -> Option<u32> {
        let tok = self.next_token()?;
        parse_unsigned_int(tok).map(|v| v as u32)
    }

    fn next_usize(&mut self) -> Option<usize> {
        let tok = self.next_token()?;
        parse_unsigned_int(tok).map(|v| v as usize)
    }
}

fn is_ascii_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Parse signed integer permissively, similar to scanf %d.
fn parse_signed_int(tok: &[u8]) -> Option<i32> {
    let s = std::str::from_utf8(tok).ok()?;
    // Try a permissive parse that mimics scanf - accept leading +/-
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Parse digits with optional sign
    s.parse::<i64>().ok().map(|v| v as i32)
}

/// Parse unsigned integer permissively, similar to scanf %u/%zu.
fn parse_unsigned_int(tok: &[u8]) -> Option<u64> {
    let s = std::str::from_utf8(tok).ok()?;
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // %u/%zu accepts optional + and digits; signed values wrap (per C's scanf %u with negative).
    // Try i64 first to handle negative inputs that wrap to unsigned.
    if let Ok(v) = s.parse::<i64>() {
        return Some(v as u64);
    }
    if let Ok(v) = s.parse::<u64>() {
        return Some(v);
    }
    None
}

fn main() {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let mut reader = match ScanfReader::new() {
        Ok(r) => r,
        Err(_) => {
            let _ = writeln!(stderr, "Error reading operation");
            std::process::exit(1);
        }
    };

    // Read operation
    let operation: i32 = match reader.next_i32() {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error reading operation");
            std::process::exit(1);
        }
    };

    // Read flags
    let flags: u32 = match reader.next_u32() {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error reading flags");
            std::process::exit(1);
        }
    };

    // Read input length
    let input_len: usize = match reader.next_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error reading input length");
            std::process::exit(1);
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            stderr,
            "Error: input length {} exceeds maximum {}",
            input_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    // Read input buffer data. Use 1024-byte buffer like C.
    let mut input_buffer: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];
    for i in 0..input_len {
        match reader.next_u32() {
            Some(byte) => {
                input_buffer[i] = byte as u8;
            }
            None => {
                let _ = writeln!(stderr, "Error reading input byte {}", i);
                std::process::exit(1);
            }
        }
    }

    // Read reference length
    let ref_len: usize = match reader.next_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error reading reference length");
            std::process::exit(1);
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            stderr,
            "Error: reference length {} exceeds maximum {}",
            ref_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut ref_buffer: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];
    for i in 0..ref_len {
        match reader.next_u32() {
            Some(byte) => {
                ref_buffer[i] = byte as u8;
            }
            None => {
                let _ = writeln!(stderr, "Error reading reference byte {}", i);
                std::process::exit(1);
            }
        }
    }

    let result = lib_strings::process_strings(
        &mut input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    let _ = writeln!(stdout, "{}", result);
}
