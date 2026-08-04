use std::io::{self, Read, Write};

/// Read the next whitespace-separated token from `data` starting at index `*pos`,
/// then attempt to parse it as a C-style `int` (`i32`). Returns `Some(value)`
/// on a successful parse and `None` if there are no more tokens or the token
/// does not start with a valid integer.
///
/// This mimics `scanf("%d", ...)`: it skips leading whitespace, then reads as
/// many characters as form a valid integer (optional sign followed by digits),
/// stopping at the first non-matching character (which is left in the stream).
fn scan_int(data: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace.
    while *pos < data.len() && (data[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= data.len() {
        return None;
    }

    let start = *pos;
    let mut idx = *pos;

    // Optional sign.
    if idx < data.len() && (data[idx] == b'+' || data[idx] == b'-') {
        idx += 1;
    }

    let digits_start = idx;
    while idx < data.len() && (data[idx] as char).is_ascii_digit() {
        idx += 1;
    }

    // No digits read -> not a valid integer; leave position at `start` so the
    // caller knows nothing was consumed (matches scanf returning a short count).
    if idx == digits_start {
        // scanf would leave the non-digit character in the stream.
        *pos = start;
        return None;
    }

    let token = std::str::from_utf8(&data[start..idx]).ok()?;
    *pos = idx;

    // Parse with i32 wrapping behavior similar to C scanf overflow handling
    // (scanf overflow is undefined; we use saturating parse via i64 fallback).
    match token.parse::<i32>() {
        Ok(v) => Some(v),
        Err(_) => match token.parse::<i64>() {
            Ok(v) => Some(v as i32),
            Err(_) => Some(0),
        },
    }
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        // If stdin can't be read, fall through with empty input.
    }

    let mut x: i32 = 1;
    let mut y: i32 = 1;

    let mut pos: usize = 0;
    if let Some(v) = scan_int(&input, &mut pos) {
        x = v;
        if let Some(v2) = scan_int(&input, &mut pos) {
            y = v2;
        }
    }

    // C's div() computes quotient and remainder with truncation toward zero,
    // which is exactly what Rust's `/` and `%` do for signed integers.
    let quot = x / y;
    let rem = x % y;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "quotient: {}, remainder: {}\n", quot, rem);
    let _ = out.flush();
}
