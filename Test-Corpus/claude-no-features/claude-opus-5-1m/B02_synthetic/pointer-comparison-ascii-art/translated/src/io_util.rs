// Stdin reading helpers that mimic C's scanf/fgets/getchar behavior.

use std::io::{self, Read, Write};

pub struct StdinReader {
    stdin: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl StdinReader {
    pub fn new() -> Self {
        StdinReader {
            stdin: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.stdin.lock().read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    fn peek_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
            return Some(b);
        }
        let b = self.read_byte()?;
        self.peeked = Some(b);
        Some(b)
    }

    /// Mimic C's fgets(buf, max, stdin): read up to max-1 bytes, or until '\n'
    /// inclusive, or EOF. Returns None if no bytes were read (i.e. EOF hit
    /// before any input).
    pub fn fgets(&mut self, max: usize) -> Option<Vec<u8>> {
        let limit = max.saturating_sub(1);
        let mut buf: Vec<u8> = Vec::new();
        for _ in 0..limit {
            match self.read_byte() {
                Some(b) => {
                    buf.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        if buf.is_empty() {
            None
        } else {
            Some(buf)
        }
    }

    /// Mimic `while (getchar() != '\n');` loop. Reads bytes until '\n' or EOF.
    pub fn consume_until_newline(&mut self) {
        loop {
            match self.read_byte() {
                Some(b'\n') => break,
                Some(_) => continue,
                None => break,
            }
        }
    }

    /// Mimic scanf("%d", &x). Skip leading whitespace, parse signed int.
    /// Returns None if no integer could be parsed (i.e., scanf would have
    /// returned 0 or EOF).
    pub fn scanf_int(&mut self) -> Option<i32> {
        // Skip leading whitespace (space, tab, \n, \r, \v, \f)
        loop {
            match self.peek_byte() {
                Some(b) if is_c_whitespace(b) => {
                    self.read_byte();
                }
                Some(_) => break,
                None => return None,
            }
        }

        let mut digits: Vec<u8> = Vec::new();
        let mut sign_consumed = false;

        match self.peek_byte() {
            Some(b'-') | Some(b'+') => {
                digits.push(self.read_byte().unwrap());
                sign_consumed = true;
            }
            _ => {}
        }

        let mut digit_count = 0;
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_digit() {
                digits.push(b);
                self.read_byte();
                digit_count += 1;
            } else {
                break;
            }
        }

        if digit_count == 0 {
            // Note: in C, sign chars consumed by scanf would already have been
            // taken from the buffer. We mirror this.
            let _ = sign_consumed;
            return None;
        }

        let s = std::str::from_utf8(&digits).ok()?;
        match s.parse::<i64>() {
            Ok(n) => Some(n as i32),
            Err(_) => None,
        }
    }
}

fn is_c_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Convert a C-style fgets buffer into a Rust string with the trailing newline
/// (if any) stripped (mirrors `name[strcspn(name, "\n")] = 0;`).
pub fn strip_newline(buf: &[u8]) -> &[u8] {
    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        &buf[..pos]
    } else {
        buf
    }
}

/// Flush stdout (used to mirror C's flushing semantics around interactive
/// prompts and stderr writes).
pub fn flush_stdout() {
    let _ = io::stdout().flush();
}
