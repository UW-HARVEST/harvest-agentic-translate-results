/*
 * Translated from C to Rust.
 * Original: Copyright 2025 MIT Lincoln Laboratory
 */

use std::io::{self, Read, Write};
use std::process::ExitCode;

use driver::process_strings;

const MAX_BUFFER_SIZE: usize = 1024;

/// Reader that streams whitespace-separated tokens from a source.
struct TokenReader<R: Read> {
    reader: R,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl<R: Read> TokenReader<R> {
    fn new(reader: R) -> Self {
        TokenReader {
            reader,
            buf: Vec::with_capacity(4096),
            pos: 0,
            eof: false,
        }
    }

    fn fill(&mut self) -> io::Result<()> {
        if self.eof {
            return Ok(());
        }
        let mut tmp = [0u8; 4096];
        let n = self.reader.read(&mut tmp)?;
        if n == 0 {
            self.eof = true;
        } else {
            self.buf.extend_from_slice(&tmp[..n]);
        }
        Ok(())
    }

    fn next_byte(&mut self) -> io::Result<Option<u8>> {
        if self.pos >= self.buf.len() {
            self.buf.clear();
            self.pos = 0;
            self.fill()?;
            if self.pos >= self.buf.len() {
                return Ok(None);
            }
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(Some(b))
    }

    fn next_token(&mut self) -> io::Result<Option<String>> {
        // Skip whitespace
        loop {
            match self.next_byte()? {
                Some(b) if (b as char).is_whitespace() => continue,
                Some(b) => {
                    let mut tok = vec![b];
                    loop {
                        match self.next_byte()? {
                            Some(c) if (c as char).is_whitespace() => break,
                            Some(c) => tok.push(c),
                            None => break,
                        }
                    }
                    return Ok(Some(String::from_utf8_lossy(&tok).into_owned()));
                }
                None => return Ok(None),
            }
        }
    }
}

fn read_i32<R: Read>(r: &mut TokenReader<R>) -> Option<i32> {
    r.next_token().ok().flatten()?.parse::<i32>().ok()
}

fn read_u32<R: Read>(r: &mut TokenReader<R>) -> Option<u32> {
    r.next_token().ok().flatten()?.parse::<u32>().ok()
}

fn read_usize<R: Read>(r: &mut TokenReader<R>) -> Option<usize> {
    r.next_token().ok().flatten()?.parse::<usize>().ok()
}

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut reader = TokenReader::new(stdin.lock());
    let stderr = io::stderr();
    let mut err = stderr.lock();

    let operation = match read_i32(&mut reader) {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading operation");
            return ExitCode::from(1);
        }
    };

    let flags = match read_u32(&mut reader) {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading flags");
            return ExitCode::from(1);
        }
    };

    let input_len = match read_usize(&mut reader) {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading input length");
            return ExitCode::from(1);
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            err,
            "Error: input length {} exceeds maximum {}",
            input_len, MAX_BUFFER_SIZE
        );
        return ExitCode::from(1);
    }

    let mut input_buffer = [0u8; MAX_BUFFER_SIZE];
    for i in 0..input_len {
        match read_u32(&mut reader) {
            Some(byte) => input_buffer[i] = byte as u8,
            None => {
                let _ = writeln!(err, "Error reading input byte {}", i);
                return ExitCode::from(1);
            }
        }
    }

    let ref_len = match read_usize(&mut reader) {
        Some(v) => v,
        None => {
            let _ = writeln!(err, "Error reading reference length");
            return ExitCode::from(1);
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            err,
            "Error: reference length {} exceeds maximum {}",
            ref_len, MAX_BUFFER_SIZE
        );
        return ExitCode::from(1);
    }

    let mut ref_buffer = [0u8; MAX_BUFFER_SIZE];
    for i in 0..ref_len {
        match read_u32(&mut reader) {
            Some(byte) => ref_buffer[i] = byte as u8,
            None => {
                let _ = writeln!(err, "Error reading reference byte {}", i);
                return ExitCode::from(1);
            }
        }
    }

    let result = process_strings(
        &input_buffer,
        input_len,
        Some(&ref_buffer),
        ref_len,
        operation,
        flags,
    );

    println!("{}", result);
    ExitCode::from(0)
}
