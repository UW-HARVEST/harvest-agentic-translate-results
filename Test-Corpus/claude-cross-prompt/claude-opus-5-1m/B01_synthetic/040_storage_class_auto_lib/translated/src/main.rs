use std::io::{self, Read, Write};

fn driver(x: i32) {
    // Mirrors C: int y = 2*x; y += 300; printf("%d\n", y);
    let mut y: i32 = (2i32).wrapping_mul(x);
    y = y.wrapping_add(300);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "{}", y).expect("write failed");
}

/// Reads an i32 from stdin in a way that mirrors C's `scanf("%d", &x)`:
/// - Skips leading whitespace (including newlines)
/// - Optional leading sign
/// - Reads decimal digits until a non-digit/EOF
/// Returns None if no integer could be parsed (matches scanf returning != 1).
fn read_int_scanf(input: &[u8], pos: &mut usize) -> Option<i32> {
    // skip whitespace
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    let mut sign: i64 = 1;
    if input[*pos] == b'+' {
        *pos += 1;
    } else if input[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }
    let digits_start = *pos;
    let mut value: i64 = 0;
    while *pos < input.len() {
        let c = input[*pos];
        if c.is_ascii_digit() {
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos == digits_start {
        // No digits — restore position and fail
        *pos = start;
        return None;
    }
    let result = (value.wrapping_mul(sign)) as i32;
    Some(result)
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return;
    }
    let mut pos: usize = 0;
    if let Some(x) = read_int_scanf(&input, &mut pos) {
        driver(x);
    }
}
