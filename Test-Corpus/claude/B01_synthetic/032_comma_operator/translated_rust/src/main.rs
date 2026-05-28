use std::io::{self, Read, Write, BufWriter};

fn driver<W: Write>(out: &mut W, x: i32) {
    let mut j: i32 = 0;
    let mut i: i32 = 0;
    while i < x {
        // Use wrapping arithmetic to mirror C's int overflow behavior
        writeln!(out, "{} {}", i, j).unwrap();
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

/// Mimic C's `scanf("%d", &x)` for a single integer.
/// Returns Some(value) if a value was parsed, None otherwise.
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (matches scanf %d behavior)
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0b || c == 0x0c
        {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= input.len() {
        return None;
    }

    let start = *pos;
    // Optional sign
    if input[*pos] == b'+' || input[*pos] == b'-' {
        *pos += 1;
    }

    let digit_start = *pos;
    while *pos < input.len() && input[*pos].is_ascii_digit() {
        *pos += 1;
    }

    if *pos == digit_start {
        // No digits found
        *pos = start;
        return None;
    }

    let s = std::str::from_utf8(&input[start..*pos]).ok()?;
    // Mirror scanf's behavior: on overflow, behavior is undefined; use wrapping parse via i64 then cast
    match s.parse::<i64>() {
        Ok(v) => Some(v as i32),
        Err(_) => None,
    }
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut pos = 0usize;
    let x = scanf_int(&input, &mut pos).unwrap_or(0);
    driver(&mut out, x);
}
