// Translated from C to Rust. Behavior matches the original.

use std::io::{self, Read, Write};

mod lib_buffer;

use lib_buffer::process_buffer;

/// Reads from stdin and yields whitespace-delimited tokens (similar to scanf).
struct TokenReader {
    data: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn new() -> Self {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data).ok();
        TokenReader { data, pos: 0 }
    }

    /// Returns the next whitespace-delimited token as a string slice, or None on EOF.
    fn next_token(&mut self) -> Option<String> {
        // Skip whitespace
        while self.pos < self.data.len() && (self.data[self.pos] as char).is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.data.len() && !(self.data[self.pos] as char).is_ascii_whitespace() {
            self.pos += 1;
        }
        Some(String::from_utf8_lossy(&self.data[start..self.pos]).into_owned())
    }
}

fn parse_u32(s: &str) -> Option<u32> {
    // Mimic scanf %u: optional leading sign (+ allowed), digits.
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let (sign_skip, _negative) = match bytes[0] {
        b'+' => (1, false),
        b'-' => (1, true),
        _ => (0, false),
    };
    let digits = &s[sign_skip..];
    if digits.is_empty() {
        return None;
    }
    // Find longest leading run of digits
    let mut end = 0;
    for (i, c) in digits.bytes().enumerate() {
        if !c.is_ascii_digit() {
            break;
        }
        end = i + 1;
    }
    if end == 0 {
        return None;
    }
    // scanf %u parses as unsigned with possible sign; negative wraps mod 2^32
    let num_str = &digits[..end];
    // Parse as u64 to allow overflow handling
    let parsed: u64 = num_str.parse().ok()?;
    let val = parsed as u32;
    if bytes[0] == b'-' {
        Some(0u32.wrapping_sub(val))
    } else {
        Some(val)
    }
}

fn parse_i32(s: &str) -> Option<i32> {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let (sign_skip, negative) = match bytes[0] {
        b'+' => (1, false),
        b'-' => (1, true),
        _ => (0, false),
    };
    let digits = &s[sign_skip..];
    if digits.is_empty() {
        return None;
    }
    let mut end = 0;
    for (i, c) in digits.bytes().enumerate() {
        if !c.is_ascii_digit() {
            break;
        }
        end = i + 1;
    }
    if end == 0 {
        return None;
    }
    let num_str = &digits[..end];
    let parsed: u64 = num_str.parse().ok()?;
    let val = parsed as u32 as i32;
    if negative {
        Some(0i32.wrapping_sub(val))
    } else {
        Some(val)
    }
}

fn parse_usize(s: &str) -> Option<usize> {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let sign_skip = match bytes[0] {
        b'+' => 1,
        b'-' => 1,
        _ => 0,
    };
    let digits = &s[sign_skip..];
    if digits.is_empty() {
        return None;
    }
    let mut end = 0;
    for (i, c) in digits.bytes().enumerate() {
        if !c.is_ascii_digit() {
            break;
        }
        end = i + 1;
    }
    if end == 0 {
        return None;
    }
    let num_str = &digits[..end];
    let parsed: u64 = num_str.parse().ok()?;
    if bytes[0] == b'-' {
        // wrap to usize size
        Some(0usize.wrapping_sub(parsed as usize))
    } else {
        Some(parsed as usize)
    }
}

fn main() {
    let mut reader = TokenReader::new();
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    // Read flags (uint32_t)
    let flags = match reader.next_token().and_then(|t| parse_u32(&t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error reading flags");
            std::process::exit(1);
        }
    };

    // Read param1 (int)
    let param1 = match reader.next_token().and_then(|t| parse_i32(&t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error reading param1");
            std::process::exit(1);
        }
    };

    // Read param2 (int)
    let param2 = match reader.next_token().and_then(|t| parse_i32(&t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error reading param2");
            std::process::exit(1);
        }
    };

    // Read length (size_t)
    let length = match reader.next_token().and_then(|t| parse_usize(&t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error reading length");
            std::process::exit(1);
        }
    };

    if length > 256 {
        let _ = writeln!(stderr, "Error: length {} exceeds maximum 256", length);
        std::process::exit(1);
    }

    let mut buffer = [0u8; 256];
    for i in 0..length {
        let byte = match reader.next_token().and_then(|t| parse_u32(&t)) {
            Some(v) => v,
            None => {
                let _ = writeln!(stderr, "Error reading byte {}", i);
                std::process::exit(1);
            }
        };
        buffer[i] = byte as u8;
    }

    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    // Output new length followed by buffer contents
    let _ = write!(stdout, "{}", new_length);
    for i in 0..new_length {
        let _ = write!(stdout, " {}", buffer[i]);
    }
    let _ = writeln!(stdout);
}
