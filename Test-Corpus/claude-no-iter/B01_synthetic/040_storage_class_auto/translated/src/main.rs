use std::io::{self, Read};

fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    println!("{}", y);
}

/// Reads a single integer from stdin in a manner similar to C's scanf("%d", ...).
/// Skips leading whitespace, then reads an optional sign and a sequence of digits.
/// Returns Some(value) on success, None on no conversion.
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (space, tab, newline, carriage return, vertical tab, form feed)
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= input.len() {
        return None;
    }

    let start = *pos;
    let mut negative = false;
    let c = input[*pos];
    if c == b'+' {
        *pos += 1;
    } else if c == b'-' {
        negative = true;
        *pos += 1;
    }

    let digits_start = *pos;
    while *pos < input.len() {
        let c = input[*pos];
        if c.is_ascii_digit() {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos == digits_start {
        // No digits read, restore position
        *pos = start;
        return None;
    }

    // Parse the digits with wrapping to mimic C's behavior on overflow.
    let mut value: i32 = 0;
    for &b in &input[digits_start..*pos] {
        let d = (b - b'0') as i32;
        value = value.wrapping_mul(10);
        if negative {
            value = value.wrapping_sub(d);
        } else {
            value = value.wrapping_add(d);
        }
    }
    Some(value)
}

fn main() {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        // Continue; x stays 0 if we can't read.
    }

    let mut x: i32 = 0;
    let mut pos = 0usize;
    if let Some(v) = scanf_int(&buf, &mut pos) {
        x = v;
    }
    driver(x);
}
