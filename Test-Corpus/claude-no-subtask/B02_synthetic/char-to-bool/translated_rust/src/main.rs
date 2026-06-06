use std::io::{self, Read, Write};
use std::process::ExitCode;

mod lib_c;

const MAX_INPUT_SIZE: usize = 1024;

/// Mimic C's fgets: read up to size-1 bytes, stopping after a newline (which is
/// retained), or at EOF. Returns None if no bytes could be read before EOF.
fn fgets<R: Read>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() < size.saturating_sub(1) {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Mimic C's atoi: skip leading whitespace, optional sign, then digits.
/// Stops at first non-digit. Returns 0 if no digits.
fn atoi(bytes: &[u8]) -> i32 {
    let mut i = 0;
    // Skip whitespace (matching isspace: space, \t, \n, \v, \f, \r)
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => i += 1,
            _ => break,
        }
    }
    if i >= bytes.len() {
        return 0;
    }
    let mut neg = false;
    if bytes[i] == b'-' {
        neg = true;
        i += 1;
    } else if bytes[i] == b'+' {
        i += 1;
    }
    let mut val: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i32;
        // Match C's wrapping behavior on overflow.
        val = val.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    if neg {
        val = val.wrapping_neg();
    }
    val
}

fn run() -> i32 {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let stderr = io::stderr();
    let mut err_handle = stderr.lock();

    // Read operation
    let buf = match fgets(&mut handle, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(err_handle, "Error reading operation");
            return 1;
        }
    };
    let operation = atoi(&buf);

    // Read parameter
    let buf = match fgets(&mut handle, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(err_handle, "Error reading parameter");
            return 1;
        }
    };
    let param = atoi(&buf);

    // Read decision string
    let mut buf = match fgets(&mut handle, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(err_handle, "Error reading decision string");
            return 1;
        }
    };

    // Strip trailing newline if present
    if buf.last() == Some(&b'\n') {
        buf.pop();
    }

    let len = buf.len();
    let result = lib_c::process_decisions(&mut buf, len, operation, param);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", result);

    0
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
