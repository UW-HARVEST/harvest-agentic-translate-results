// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output.

use std::io::{self, Read, Write};

#[path = "decisions.rs"]
mod decisions;

const MAX_INPUT_SIZE: usize = 1024;

/// Mirror of C `fgets` behavior:
/// - Reads up to `buf.len() - 1` bytes, or until a newline (newline included if present),
///   or until EOF.
/// - Returns `None` if EOF is encountered before any byte is read.
/// - Returns the number of bytes read (excluding the implicit null terminator).
/// The buffer is *not* required to be null-terminated by the caller; we don't store
/// a null since Rust slices carry their length.
fn fgets(buf: &mut [u8], stdin: &mut impl Read) -> Option<usize> {
    let max = buf.len().saturating_sub(1);
    let mut count = 0usize;
    let mut single = [0u8; 1];
    while count < max {
        match stdin.read(&mut single) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf[count] = single[0];
                count += 1;
                if single[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    if count == 0 {
        return None;
    }
    Some(count)
}

/// Mirror of C `atoi`:
/// - Skips leading whitespace (matches isspace: space, \t, \n, \v, \f, \r).
/// - Reads optional '+' or '-'.
/// - Reads ASCII digits until a non-digit is encountered.
/// - Returns 0 if no conversion can be performed.
/// - Uses wrapping arithmetic (C's atoi is UB on overflow; we choose a deterministic behavior).
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < s.len()
        && matches!(
            s[i],
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r'
        )
    {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        if s[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        result = result
            .wrapping_mul(10)
            .wrapping_add((s[i] - b'0') as i32);
        i += 1;
    }
    result.wrapping_mul(sign)
}

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    let mut input_buffer = [0u8; MAX_INPUT_SIZE];

    // Read operation number
    let n = match fgets(&mut input_buffer, &mut handle) {
        Some(n) => n,
        None => {
            let _ = writeln!(io::stderr(), "Error reading operation");
            return 1;
        }
    };
    let operation = atoi(&input_buffer[..n]);

    // Read parameter
    let n = match fgets(&mut input_buffer, &mut handle) {
        Some(n) => n,
        None => {
            let _ = writeln!(io::stderr(), "Error reading parameter");
            return 1;
        }
    };
    let param = atoi(&input_buffer[..n]);

    // Read decision string
    let n = match fgets(&mut input_buffer, &mut handle) {
        Some(n) => n,
        None => {
            let _ = writeln!(io::stderr(), "Error reading decision string");
            return 1;
        }
    };

    // Strip trailing newline if present
    let mut len = n;
    if len > 0 && input_buffer[len - 1] == b'\n' {
        input_buffer[len - 1] = 0;
        len -= 1;
    }

    let result = decisions::process_decisions(&mut input_buffer[..len], len, operation, param);

    // Match printf("%d\n", result)
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", result);
    let _ = out.flush();

    0
}
