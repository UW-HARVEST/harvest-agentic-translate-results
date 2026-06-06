use std::io::{self, Read, Write};

fn driver(x: i32, y: i32) {
    // x bitor compl y => x | !y (bitwise NOT in C is ~ which Rust represents with !)
    let result = x | !y;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "{}", result).unwrap();
    // puts("") prints "\n"
    writeln!(out).unwrap();
}

/// Read all of stdin into a String once.
fn read_all_stdin() -> String {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).ok();
    s
}

/// Mimic scanf("%d", ...) which skips leading whitespace (including newlines)
/// and then reads an optional sign followed by decimal digits.
/// Returns the parsed integer and advances `pos` to just after the number.
/// Returns None if no integer could be parsed (EOF or invalid input).
fn scanf_int(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= bytes.len() {
        return None;
    }

    let start = *pos;
    // Optional sign
    if bytes[*pos] == b'+' || bytes[*pos] == b'-' {
        *pos += 1;
    }
    let digits_start = *pos;
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_digit() {
        *pos += 1;
    }
    if *pos == digits_start {
        // No digits found; restore pos and fail
        *pos = start;
        return None;
    }
    let num_str = std::str::from_utf8(&bytes[start..*pos]).ok()?;
    // Use wrapping parse semantics matching scanf's int (i32) - on overflow, scanf
    // technically has undefined behavior; we'll just parse and on failure, return None.
    num_str.parse::<i32>().ok()
}

fn main() {
    let input = read_all_stdin();
    let bytes = input.as_bytes();
    let mut pos = 0usize;

    // int x = 0, y = 0;
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    if let Some(v) = scanf_int(bytes, &mut pos) {
        x = v;
    }
    if let Some(v) = scanf_int(bytes, &mut pos) {
        y = v;
    }

    driver(x, y);
}
