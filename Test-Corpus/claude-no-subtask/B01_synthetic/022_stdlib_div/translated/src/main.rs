use std::io::{self, Read, Write};

fn parse_int(bytes: &[u8], mut pos: usize) -> Option<(i32, usize)> {
    // Skip whitespace (matches scanf %d which skips leading whitespace)
    while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= bytes.len() {
        return None;
    }
    let start = pos;
    if bytes[pos] == b'-' || bytes[pos] == b'+' {
        pos += 1;
    }
    let digits_start = pos;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos == digits_start {
        return None;
    }
    let s = std::str::from_utf8(&bytes[start..pos]).ok()?;
    // Use i64 first then cast to mimic typical scanf %d behavior (no overflow check)
    let val: i64 = s.parse().ok()?;
    Some((val as i32, pos))
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok();

    let mut x: i32 = 1;
    let mut y: i32 = 1;

    let bytes = input.as_bytes();
    let mut pos = 0;

    if let Some((val, new_pos)) = parse_int(bytes, pos) {
        x = val;
        pos = new_pos;
        if let Some((val2, _new_pos)) = parse_int(bytes, pos) {
            y = val2;
        }
    }

    // Replicates C's div(x, y) which truncates toward zero.
    // Rust's i32 / and % also truncate toward zero, matching C99 div behavior.
    let quot = x / y;
    let rem = x % y;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "quotient: {}, remainder: {}\n", quot, rem).unwrap();
}
