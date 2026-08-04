// Translated from MIT Lincoln Laboratory C source.

use std::io::{self, Read, Write};
use std::process::ExitCode;

mod lib_decisions;

const MAX_INPUT_SIZE: usize = 1024;

/// Mimic C's `fgets`: reads at most `max - 1` bytes from `reader` into a
/// `Vec<u8>`, stopping at end-of-file or after a newline byte (which is kept
/// in the buffer when present). Returns `Ok(None)` if no bytes were read
/// before encountering EOF (mirroring `fgets` returning `NULL`).
fn fgets<R: Read>(reader: &mut R, max: usize) -> io::Result<Option<Vec<u8>>> {
    if max == 0 {
        return Ok(None);
    }
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() + 1 < max {
        let n = reader.read(&mut byte)?;
        if n == 0 {
            break;
        }
        buf.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    if buf.is_empty() {
        Ok(None)
    } else {
        Ok(Some(buf))
    }
}

/// Mimic C's `atoi`: skip leading whitespace, optional sign, then parse
/// decimal digits until the first non-digit byte.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < s.len()
        && matches!(
            s[i],
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c
        )
    {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        if s[i] == b'-' {
            neg = true;
        }
        i += 1;
    }
    let mut result: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    let result = if neg { result.wrapping_neg() } else { result };
    result as i32
}

fn main_impl() -> i32 {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // Read operation number
    let line1 = match fgets(&mut handle, MAX_INPUT_SIZE) {
        Ok(Some(v)) => v,
        _ => {
            let _ = writeln!(io::stderr(), "Error reading operation");
            return 1;
        }
    };
    let operation = atoi(&line1);

    // Read parameter
    let line2 = match fgets(&mut handle, MAX_INPUT_SIZE) {
        Ok(Some(v)) => v,
        _ => {
            let _ = writeln!(io::stderr(), "Error reading parameter");
            return 1;
        }
    };
    let param = atoi(&line2);

    // Read decision string
    let mut line3 = match fgets(&mut handle, MAX_INPUT_SIZE) {
        Ok(Some(v)) => v,
        _ => {
            let _ = writeln!(io::stderr(), "Error reading decision string");
            return 1;
        }
    };

    // Remove trailing newline if present
    if !line3.is_empty() && *line3.last().unwrap() == b'\n' {
        line3.pop();
    }
    let len = line3.len();

    // Call the library function
    let result = lib_decisions::process_decisions(&line3, len, operation, param);

    // Print result to stdout
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", result);

    0
}

fn main() -> ExitCode {
    ExitCode::from(main_impl() as u8)
}
