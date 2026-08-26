use std::io::{self, Read, Write};
use std::process::ExitCode;

mod lib_impl;

const MAX_BUFFER_SIZE: usize = 1024;

/// Token reader that mimics C's scanf whitespace-skipping behavior.
struct TokenReader {
    data: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn new() -> io::Result<Self> {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        Ok(TokenReader { data, pos: 0 })
    }

    /// Read the next whitespace-delimited token.
    /// Returns None if no token is found before EOF.
    fn next_token(&mut self) -> Option<&[u8]> {
        // Skip leading whitespace.
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

    /// Parse the next token as a signed 32-bit integer (C int / scanf "%d").
    fn next_i32(&mut self) -> Option<i32> {
        let tok = self.next_token()?;
        let s = std::str::from_utf8(tok).ok()?;
        s.parse::<i32>().ok()
    }

    /// Parse the next token as an unsigned 32-bit integer (C unsigned / scanf "%u").
    /// Matches C's behavior of accepting a leading '-' as wrapping conversion.
    fn next_u32(&mut self) -> Option<u32> {
        let tok = self.next_token()?;
        let s = std::str::from_utf8(tok).ok()?;
        if let Some(stripped) = s.strip_prefix('-') {
            // C scanf "%u" with negative input: parse as i64 then wrap.
            let v = stripped.parse::<u64>().ok()?;
            Some((0u32).wrapping_sub(v as u32))
        } else if let Some(stripped) = s.strip_prefix('+') {
            stripped.parse::<u32>().ok()
        } else {
            s.parse::<u32>().ok()
        }
    }

    /// Parse the next token as a size_t (C size_t / scanf "%zu").
    fn next_usize(&mut self) -> Option<usize> {
        let tok = self.next_token()?;
        let s = std::str::from_utf8(tok).ok()?;
        s.parse::<usize>().ok()
    }
}

fn run() -> i32 {
    let mut reader = match TokenReader::new() {
        Ok(r) => r,
        Err(_) => {
            let _ = writeln!(io::stderr(), "Error reading stdin");
            return 1;
        }
    };

    let stderr = io::stderr();

    // Read operation
    let operation = match reader.next_i32() {
        Some(v) => v,
        None => {
            let _ = writeln!(&mut stderr.lock(), "Error reading operation");
            return 1;
        }
    };

    // Read flags
    let flags = match reader.next_u32() {
        Some(v) => v,
        None => {
            let _ = writeln!(&mut stderr.lock(), "Error reading flags");
            return 1;
        }
    };

    // Read input length
    let input_len = match reader.next_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(&mut stderr.lock(), "Error reading input length");
            return 1;
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            &mut stderr.lock(),
            "Error: input length {} exceeds maximum {}",
            input_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    // Read input buffer data — mirrors C's stack-allocated buffer of MAX_BUFFER_SIZE bytes.
    // Although C leaves bytes beyond input_len uninitialized, we zero-initialize so the
    // string-handling helpers can safely treat the buffer as a C string when callers
    // include the trailing null byte within input_len.
    let mut input_buffer = vec![0u8; MAX_BUFFER_SIZE];
    for i in 0..input_len {
        let byte = match reader.next_u32() {
            Some(v) => v,
            None => {
                let _ = writeln!(&mut stderr.lock(), "Error reading input byte {}", i);
                return 1;
            }
        };
        input_buffer[i] = byte as u8;
    }

    // Read reference length
    let ref_len = match reader.next_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(&mut stderr.lock(), "Error reading reference length");
            return 1;
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            &mut stderr.lock(),
            "Error: reference length {} exceeds maximum {}",
            ref_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    // Read reference buffer data
    let mut ref_buffer = vec![0u8; MAX_BUFFER_SIZE];
    for i in 0..ref_len {
        let byte = match reader.next_u32() {
            Some(v) => v,
            None => {
                let _ = writeln!(&mut stderr.lock(), "Error reading reference byte {}", i);
                return 1;
            }
        };
        ref_buffer[i] = byte as u8;
    }

    // Call the library function
    let result = lib_impl::process_strings(
        &mut input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    // Print result to stdout
    let stdout = io::stdout();
    let _ = writeln!(&mut stdout.lock(), "{}", result);

    0
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
