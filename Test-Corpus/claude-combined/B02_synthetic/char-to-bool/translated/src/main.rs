use std::io::{self, Read, Write};

mod lib_decisions;

const MAX_INPUT_SIZE: usize = 1024;

/// Mimics C's fgets: reads up to size-1 bytes, stopping at newline (which is included)
/// or EOF. Returns None if EOF is reached before any byte is read.
fn fgets(stdin_bytes: &mut std::vec::IntoIter<u8>, size: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let max = size.saturating_sub(1);
    let mut got_any = false;
    while buf.len() < max {
        match stdin_bytes.next() {
            Some(b) => {
                got_any = true;
                buf.push(b);
                if b == b'\n' {
                    break;
                }
            }
            None => break,
        }
    }
    if !got_any {
        return None;
    }
    Some(buf)
}

/// Mimics C's atoi on a null-terminated/length-bounded buffer of bytes.
/// Skips leading whitespace, optional +/-, then parses digits.
fn c_atoi(bytes: &[u8]) -> i32 {
    let mut i = 0;
    // Skip whitespace (matches C's isspace for typical inputs: space, tab, \n, \r, \v, \f)
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C => i += 1,
            _ => break,
        }
    }
    let mut sign: i32 = 1;
    if i < bytes.len() {
        if bytes[i] == b'-' {
            sign = -1;
            i += 1;
        } else if bytes[i] == b'+' {
            i += 1;
        }
    }
    let mut result: i32 = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            // Use wrapping arithmetic to mimic C overflow behavior
            result = result.wrapping_mul(10).wrapping_add((c - b'0') as i32);
            i += 1;
        } else {
            break;
        }
    }
    result.wrapping_mul(sign)
}

fn main() {
    // Read all of stdin once, then dispense via fgets
    let mut all = Vec::new();
    if io::stdin().read_to_end(&mut all).is_err() {
        // Treat as if no input available
    }
    let mut iter = all.into_iter();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let stderr = io::stderr();
    let mut err = stderr.lock();

    // Read operation number
    let buf1 = match fgets(&mut iter, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(err, "Error reading operation");
            std::process::exit(1);
        }
    };
    let operation = c_atoi(&buf1);

    // Read parameter
    let buf2 = match fgets(&mut iter, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(err, "Error reading parameter");
            std::process::exit(1);
        }
    };
    let param = c_atoi(&buf2);

    // Read decision string
    let mut buf3 = match fgets(&mut iter, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(err, "Error reading decision string");
            std::process::exit(1);
        }
    };

    // Remove trailing newline if present
    if let Some(&last) = buf3.last() {
        if last == b'\n' {
            buf3.pop();
        }
    }
    let len = buf3.len();

    // Call the library function
    let result = lib_decisions::process_decisions(&mut buf3, len, operation, param);

    // Print result
    let _ = writeln!(out, "{}", result);
}
