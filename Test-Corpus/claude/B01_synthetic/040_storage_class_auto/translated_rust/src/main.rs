use std::io::{self, Read, Write};

fn driver(x: i32) {
    // Use wrapping arithmetic to match C's int overflow behavior.
    let mut y: i32 = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    println!("{}", y);
}

/// Mimic scanf("%d", &x):
/// - Skip leading whitespace (including newlines).
/// - Optionally read a leading '+' or '-'.
/// - Read decimal digits until a non-digit byte or EOF.
/// - On match failure (no digits), the variable is unchanged.
/// Returns Some(value) on success, None on match failure.
fn scanf_int(stdin_bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < stdin_bytes.len() {
        let c = stdin_bytes[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= stdin_bytes.len() {
        return None;
    }

    let start = *pos;
    let mut sign: i64 = 1;
    if stdin_bytes[*pos] == b'+' {
        *pos += 1;
    } else if stdin_bytes[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }

    let digit_start = *pos;
    let mut value: i64 = 0;
    while *pos < stdin_bytes.len() {
        let c = stdin_bytes[*pos];
        if c.is_ascii_digit() {
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos == digit_start {
        // No digits matched; rewind to the start so caller knows nothing was consumed.
        *pos = start;
        return None;
    }

    let signed = sign.wrapping_mul(value);
    Some(signed as i32)
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        // If reading fails, fall back to default behavior
    }

    let mut pos: usize = 0;
    let x: i32 = scanf_int(&input, &mut pos).unwrap_or(0);
    driver(x);

    // Ensure output is flushed
    let _ = io::stdout().flush();
}
