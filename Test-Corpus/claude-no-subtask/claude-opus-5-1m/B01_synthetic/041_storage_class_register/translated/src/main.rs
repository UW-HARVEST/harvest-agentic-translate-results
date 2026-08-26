use std::io::{self, Read, Write};

fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", y);
}

/// Mimic C's `scanf("%d", &x)` for a single integer value.
/// Skips leading whitespace, then reads an optional sign followed by digits.
/// Returns the parsed integer, or `None` if no integer could be read.
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace (including newlines, tabs, spaces, etc.)
    while *pos < input.len() && (input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    if input[*pos] == b'+' || input[*pos] == b'-' {
        *pos += 1;
    }
    let digits_start = *pos;
    while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
        *pos += 1;
    }
    if *pos == digits_start {
        // No digits found; restore position to start
        *pos = start;
        return None;
    }
    let s = std::str::from_utf8(&input[start..*pos]).ok()?;
    // Mimic C scanf behavior: parse with wrapping on overflow is technically UB in C,
    // but here we just attempt parse; on failure return None and leave x as initialized 0.
    s.parse::<i32>().ok()
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
