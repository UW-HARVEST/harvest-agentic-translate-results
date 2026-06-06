/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Translated to Rust from original C code.
 */

use std::io::{self, Read, Write};
use std::process::ExitCode;

mod lib_buffer;

/// Reads a single whitespace-delimited token from the byte iterator.
/// Mimics scanf's behavior of skipping leading whitespace (including newlines)
/// and reading non-whitespace characters until whitespace or EOF.
fn read_token(input: &[u8], pos: &mut usize) -> Option<String> {
    // Skip leading whitespace
    while *pos < input.len() && (input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    while *pos < input.len() && !(input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if start == *pos {
        return None;
    }
    Some(std::str::from_utf8(&input[start..*pos]).ok()?.to_string())
}

/// Parse a token as an unsigned 32-bit integer, mimicking scanf("%u").
/// scanf accepts optional sign and treats the result modulo 2^32 for negative.
fn parse_u32(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Ok(v) = s.parse::<u32>() {
        return Some(v);
    }
    // Try parsing as i64, then casting (handles negative inputs like scanf)
    if let Ok(v) = s.parse::<i64>() {
        return Some(v as u32);
    }
    None
}

/// Parse a token as a signed 32-bit integer, mimicking scanf("%d").
fn parse_i32(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Ok(v) = s.parse::<i32>() {
        return Some(v);
    }
    if let Ok(v) = s.parse::<i64>() {
        return Some(v as i32);
    }
    None
}

/// Parse a token as a size_t (usize), mimicking scanf("%zu").
fn parse_usize(s: &str) -> Option<usize> {
    let s = s.trim();
    if let Ok(v) = s.parse::<usize>() {
        return Some(v);
    }
    if let Ok(v) = s.parse::<i64>() {
        return Some(v as usize);
    }
    None
}

fn main() -> ExitCode {
    // Read all of stdin into a buffer for token parsing.
    let mut input_bytes = Vec::new();
    if io::stdin().read_to_end(&mut input_bytes).is_err() {
        eprintln!("Error reading stdin");
        return ExitCode::from(1);
    }
    let mut pos = 0usize;

    let stderr = io::stderr();
    let stdout = io::stdout();

    // Read flags (%u)
    let flags: u32 = match read_token(&input_bytes, &mut pos).and_then(|t| parse_u32(&t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr.lock(), "Error reading flags");
            return ExitCode::from(1);
        }
    };

    // Read param1 (%d)
    let param1: i32 = match read_token(&input_bytes, &mut pos).and_then(|t| parse_i32(&t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr.lock(), "Error reading param1");
            return ExitCode::from(1);
        }
    };

    // Read param2 (%d)
    let param2: i32 = match read_token(&input_bytes, &mut pos).and_then(|t| parse_i32(&t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr.lock(), "Error reading param2");
            return ExitCode::from(1);
        }
    };

    // Read length (%zu)
    let length: usize = match read_token(&input_bytes, &mut pos).and_then(|t| parse_usize(&t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr.lock(), "Error reading length");
            return ExitCode::from(1);
        }
    };

    if length > 256 {
        let _ = writeln!(
            stderr.lock(),
            "Error: length {} exceeds maximum 256",
            length
        );
        return ExitCode::from(1);
    }

    // Allocate buffer larger than 256 to accommodate any potential expansion in
    // compact_runs (which could double size with threshold == 1). The C code
    // technically has a fixed [256] stack buffer; we use a generous size to
    // avoid panics while remaining behaviorally equivalent for in-bounds cases.
    let mut buffer = vec![0u8; 1024];

    // Read buffer data: each as %u, then cast to uint8_t (truncate low 8 bits).
    for i in 0..length {
        match read_token(&input_bytes, &mut pos).and_then(|t| parse_u32(&t)) {
            Some(byte) => {
                buffer[i] = byte as u8;
            }
            None => {
                let _ = writeln!(stderr.lock(), "Error reading byte {}", i);
                return ExitCode::from(1);
            }
        }
    }

    // Process the buffer
    let new_length = lib_buffer::process_buffer(&mut buffer, length, flags, param1, param2);

    // Output: print new_length, then each byte as " %u", then newline.
    let mut out = stdout.lock();
    let _ = write!(out, "{}", new_length);
    for i in 0..new_length {
        let _ = write!(out, " {}", buffer[i]);
    }
    let _ = writeln!(out);

    ExitCode::from(0)
}
