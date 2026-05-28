use std::io::{self, Read, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: i32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}

/// Read a single integer from stdin in a manner similar to C's `scanf("%d", ...)`.
/// Skips leading whitespace, then reads an optional sign followed by decimal digits.
/// If parsing fails, returns 0 (matching the initial value of `x` in the C code).
fn scanf_int<R: Read>(reader: &mut R) -> i32 {
    let mut byte = [0u8; 1];

    // Skip leading whitespace
    loop {
        match reader.read(&mut byte) {
            Ok(0) => return 0,
            Ok(_) => {
                if !byte[0].is_ascii_whitespace() {
                    break;
                }
            }
            Err(_) => return 0,
        }
    }

    let mut buf: Vec<u8> = Vec::new();

    // Optional sign
    if byte[0] == b'+' || byte[0] == b'-' {
        buf.push(byte[0]);
        match reader.read(&mut byte) {
            Ok(0) => {
                // No digits after sign — scanf would not assign; return 0.
                return 0;
            }
            Ok(_) => {}
            Err(_) => return 0,
        }
    }

    // Must have at least one digit
    if !byte[0].is_ascii_digit() {
        return 0;
    }

    buf.push(byte[0]);

    // Read remaining digits
    loop {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0].is_ascii_digit() {
                    buf.push(byte[0]);
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let s = std::str::from_utf8(&buf).unwrap_or("");
    // Use wrapping parse to mimic C's behavior on overflow as best we can.
    s.parse::<i32>().unwrap_or_else(|_| {
        // On overflow, scanf has undefined behavior; fall back to wrapping i64 parse.
        s.parse::<i64>().map(|v| v as i32).unwrap_or(0)
    })
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let x = scanf_int(&mut handle);
    driver(x);
}
