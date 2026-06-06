use std::io::{self, Read, Write};

fn driver(x: i32) {
    let mut y: i32 = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", y);
}

/// Mimic C's `scanf("%d", &x)` behavior:
/// - skip leading whitespace
/// - optional '+' or '-' sign
/// - one or more decimal digits
/// - stop at first non-digit (which is left in the stream)
/// Returns Some(value) on success, None on failure (no digits or EOF).
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < input.len() && (input[*pos] as char).is_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None;
    }
    let mut negative = false;
    if input[*pos] == b'+' {
        *pos += 1;
    } else if input[*pos] == b'-' {
        negative = true;
        *pos += 1;
    }
    let start = *pos;
    let mut value: i32 = 0;
    while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
        let digit = (input[*pos] - b'0') as i32;
        // C int overflow is undefined; use wrapping to match common compiler behavior
        value = value.wrapping_mul(10);
        if negative {
            value = value.wrapping_sub(digit);
        } else {
            value = value.wrapping_add(digit);
        }
        *pos += 1;
    }
    if *pos == start {
        // No digits matched
        return None;
    }
    Some(value)
}

fn main() {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        // On read error, x remains 0
        driver(0);
        return;
    }
    let mut pos: usize = 0;
    let x = scanf_int(&buf, &mut pos).unwrap_or(0);
    driver(x);
}
