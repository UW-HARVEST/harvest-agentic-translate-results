// Translation of c_src/src/main.c and c_src/src/lib.c to Rust.
// This is an executable that reads operation parameters from stdin
// (using scanf-style whitespace-separated tokens) and produces output
// matching the original C program byte-for-byte.

use std::io::{self, Read, Write};

mod process;

const MAX_BUFFER_SIZE: usize = 1024;

/// Token reader that mimics scanf's whitespace-skipping behavior.
/// scanf("%d"), scanf("%u"), scanf("%zu") all skip leading whitespace
/// (including newlines) and read a token of digits.
struct TokenReader {
    buf: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn new() -> Self {
        let mut buf = Vec::new();
        // Read all of stdin up front. scanf reads lazily, but for our
        // purposes (whitespace-separated tokens) reading everything is
        // equivalent.
        let _ = io::stdin().read_to_end(&mut buf);
        TokenReader { buf, pos: 0 }
    }

    /// Skip ASCII whitespace (matches isspace in the C locale for
    /// scanf's purposes: space, tab, newline, vertical tab, form feed,
    /// carriage return).
    fn skip_ws(&mut self) {
        while self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
                || c == 0x0b || c == 0x0c
            {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Read the next non-whitespace token as a string.
    fn next_token(&mut self) -> Option<&[u8]> {
        self.skip_ws();
        if self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
                || c == 0x0b || c == 0x0c
            {
                break;
            }
            self.pos += 1;
        }
        if start == self.pos {
            None
        } else {
            Some(&self.buf[start..self.pos])
        }
    }

    /// Read a signed integer, scanf("%d"): optional sign, then digits.
    fn read_i32(&mut self) -> Option<i32> {
        let tok = self.next_token()?;
        let s = std::str::from_utf8(tok).ok()?;
        // scanf("%d") accepts leading +/- and decimal digits.
        s.parse::<i32>().ok()
    }

    /// Read an unsigned 32-bit integer, scanf("%u"): digits only
    /// (or with optional sign per C standard, which we handle the same
    /// way the typical glibc impl does).
    fn read_u32(&mut self) -> Option<u32> {
        let tok = self.next_token()?;
        let s = std::str::from_utf8(tok).ok()?;
        s.parse::<u32>().ok()
    }

    /// Read a size_t, scanf("%zu"): unsigned digits.
    fn read_usize(&mut self) -> Option<usize> {
        let tok = self.next_token()?;
        let s = std::str::from_utf8(tok).ok()?;
        s.parse::<usize>().ok()
    }
}

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    let stderr = io::stderr();
    let mut err = stderr.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut reader = TokenReader::new();

    // Read operation
    let operation = match reader.read_i32() {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading operation");
            return 1;
        }
    };

    // Read flags
    let flags = match reader.read_u32() {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading flags");
            return 1;
        }
    };

    // Read input length
    let input_len = match reader.read_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading input length");
            return 1;
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            err,
            "Error: input length {} exceeds maximum {}",
            input_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    // Allocate the buffer at full MAX_BUFFER_SIZE so that out-of-bounds
    // reads (as performed by strcmp/strlen on possibly non-null-terminated
    // input in the original C code) read defined zero bytes rather than
    // truly uninitialized stack memory. Zero-initialization gives a
    // well-defined (and benign) interpretation of the original UB.
    let mut input_buffer: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];

    // Read input buffer data
    for i in 0..input_len {
        match reader.read_u32() {
            Some(byte) => {
                // Truncate to a single byte (matches C's `(char)byte`).
                input_buffer[i] = byte as u8;
            }
            None => {
                let _ = writeln!(err, "Error reading input byte {}", i);
                return 1;
            }
        }
    }

    // Read reference length
    let ref_len = match reader.read_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading reference length");
            return 1;
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            err,
            "Error: reference length {} exceeds maximum {}",
            ref_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    let mut ref_buffer: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];

    for i in 0..ref_len {
        match reader.read_u32() {
            Some(byte) => {
                ref_buffer[i] = byte as u8;
            }
            None => {
                let _ = writeln!(err, "Error reading reference byte {}", i);
                return 1;
            }
        }
    }

    // Call the library function
    let result = process::process_strings(
        &mut input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    // Print result to stdout
    let _ = writeln!(out, "{}", result);

    0
}
