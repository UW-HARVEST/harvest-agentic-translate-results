// Translation of c_src/main.c and c_src/lib.c
// Produces byte-identical output for the same inputs.

use std::io::{self, Read, Write};
use std::process::ExitCode;

mod lib_translated;

const MAX_INPUT_SIZE: usize = 1024;

/// Mimics C's `fgets(buf, MAX_INPUT_SIZE, stdin)`.
/// Reads up to `max_size - 1` bytes or until a newline (inclusive) into a Vec<u8>.
/// Returns Ok(Some(bytes)) if any byte was read, Ok(None) on immediate EOF.
fn fgets<R: Read>(reader: &mut R, max_size: usize) -> io::Result<Option<Vec<u8>>> {
    if max_size <= 1 {
        return Ok(None);
    }
    let mut buf: Vec<u8> = Vec::new();
    let limit = max_size - 1;
    let mut byte = [0u8; 1];
    while buf.len() < limit {
        let n = reader.read(&mut byte)?;
        if n == 0 {
            // EOF
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

/// Mimics C's `atoi`:
/// - skips leading whitespace
/// - optional '+' or '-' sign
/// - reads digits until first non-digit
/// - returns 0 if no digits
/// - overflow is undefined behavior in C; here we use wrapping arithmetic to be safe.
fn atoi(buf: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < buf.len() {
        let c = buf[i];
        // C's isspace includes ' ', '\t', '\n', '\v', '\f', '\r'
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }
    let mut sign: i32 = 1;
    if i < buf.len() && (buf[i] == b'+' || buf[i] == b'-') {
        if buf[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < buf.len() {
        let c = buf[i];
        if c.is_ascii_digit() {
            result = result.wrapping_mul(10).wrapping_add((c - b'0') as i32);
            i += 1;
        } else {
            break;
        }
    }
    result.wrapping_mul(sign)
}

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();

    // Read operation number
    let buf1 = match fgets(&mut stdin_lock, MAX_INPUT_SIZE) {
        Ok(Some(b)) => b,
        _ => {
            let _ = writeln!(io::stderr(), "Error reading operation");
            return ExitCode::from(1);
        }
    };
    let operation = atoi(&buf1);

    // Read parameter
    let buf2 = match fgets(&mut stdin_lock, MAX_INPUT_SIZE) {
        Ok(Some(b)) => b,
        _ => {
            let _ = writeln!(io::stderr(), "Error reading parameter");
            return ExitCode::from(1);
        }
    };
    let param = atoi(&buf2);

    // Read decision string
    let mut buf3 = match fgets(&mut stdin_lock, MAX_INPUT_SIZE) {
        Ok(Some(b)) => b,
        _ => {
            let _ = writeln!(io::stderr(), "Error reading decision string");
            return ExitCode::from(1);
        }
    };

    // Remove trailing newline if present
    if let Some(&last) = buf3.last() {
        if last == b'\n' {
            buf3.pop();
        }
    }

    // Call the library function
    let result = lib_translated::process_decisions(&mut buf3, operation, param);

    // Print result to stdout
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", result);
    let _ = out.flush();

    ExitCode::from(0)
}
