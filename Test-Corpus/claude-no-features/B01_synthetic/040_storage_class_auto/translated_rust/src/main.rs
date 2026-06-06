use std::io::{self, Read, Write};

fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", y);
}

/// Mimics scanf("%d", ...) behavior: skips leading whitespace, reads optional
/// sign, then reads digits. Returns Some(value) if at least one digit was read,
/// None otherwise (in which case the caller should leave the destination
/// unchanged, matching C scanf semantics).
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace
    while *pos < input.len() && (input[*pos] as char).is_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    let mut negative = false;
    if input[*pos] == b'-' {
        negative = true;
        *pos += 1;
    } else if input[*pos] == b'+' {
        *pos += 1;
    }
    let digits_start = *pos;
    let mut value: i64 = 0;
    while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
        value = value * 10 + (input[*pos] - b'0') as i64;
        *pos += 1;
    }
    if *pos == digits_start {
        // No digits parsed; revert position and report failure
        *pos = start;
        return None;
    }
    if negative {
        value = -value;
    }
    Some(value as i32)
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);
    let mut pos = 0usize;
    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&input, &mut pos) {
        x = v;
    }
    driver(x);
}
