use std::io::{self, Read, Write};
use std::process::ExitCode;

mod lib_rs;

fn main() -> ExitCode {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        let _ = writeln!(io::stderr(), "Error reading input");
        return ExitCode::from(1);
    }
    let mut tokens = input.split_ascii_whitespace();

    // Read flags (uint32_t, scanf %u)
    let flags: u32 = match tokens.next().and_then(|t| parse_u32(t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading flags");
            return ExitCode::from(1);
        }
    };

    // Read param1 (int, scanf %d)
    let param1: i32 = match tokens.next().and_then(|t| parse_i32(t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading param1");
            return ExitCode::from(1);
        }
    };

    // Read param2 (int, scanf %d)
    let param2: i32 = match tokens.next().and_then(|t| parse_i32(t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading param2");
            return ExitCode::from(1);
        }
    };

    // Read length (size_t, scanf %zu)
    let length: usize = match tokens.next().and_then(|t| parse_usize(t)) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading length");
            return ExitCode::from(1);
        }
    };

    if length > 256 {
        let _ = writeln!(
            io::stderr(),
            "Error: length {} exceeds maximum 256",
            length
        );
        return ExitCode::from(1);
    }

    let mut buffer: [u8; 256] = [0; 256];
    for i in 0..length {
        let byte_val: u32 = match tokens.next().and_then(|t| parse_u32(t)) {
            Some(v) => v,
            None => {
                let _ = writeln!(io::stderr(), "Error reading byte {}", i);
                return ExitCode::from(1);
            }
        };
        buffer[i] = byte_val as u8;
    }

    let new_length = lib_rs::process_buffer(&mut buffer, length, flags, param1, param2);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Output new length
    let _ = write!(out, "{}", new_length);
    for i in 0..new_length {
        let _ = write!(out, " {}", buffer[i] as u32);
    }
    let _ = writeln!(out);

    ExitCode::from(0)
}

/// Parse a u32 the way scanf("%u") does: optional leading sign, decimal digits.
/// Returns None if no valid match.
fn parse_u32(s: &str) -> Option<u32> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let (negative, rest) = match bytes[0] {
        b'+' => (false, &bytes[1..]),
        b'-' => (true, &bytes[1..]),
        _ => (false, bytes),
    };
    if rest.is_empty() || !rest.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Mimic strtoul: parse as u64 then truncate to u32, negate via wrap if needed
    let mut acc: u64 = 0;
    for &b in rest {
        acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as u64);
    }
    let val = acc as u32;
    Some(if negative { 0u32.wrapping_sub(val) } else { val })
}

/// Parse i32 the way scanf("%d") does.
fn parse_i32(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let (negative, rest) = match bytes[0] {
        b'+' => (false, &bytes[1..]),
        b'-' => (true, &bytes[1..]),
        _ => (false, bytes),
    };
    if rest.is_empty() || !rest.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut acc: i64 = 0;
    for &b in rest {
        acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as i64);
    }
    let val = if negative { acc.wrapping_neg() } else { acc };
    Some(val as i32)
}

/// Parse usize the way scanf("%zu") does.
fn parse_usize(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let (negative, rest) = match bytes[0] {
        b'+' => (false, &bytes[1..]),
        b'-' => (true, &bytes[1..]),
        _ => (false, bytes),
    };
    if rest.is_empty() || !rest.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut acc: u64 = 0;
    for &b in rest {
        acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as u64);
    }
    let val = acc as usize;
    Some(if negative { 0usize.wrapping_sub(val) } else { val })
}
