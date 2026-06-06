use std::io::{self, Read, Write, BufWriter};

fn driver<W: Write>(x: i32, out: &mut W) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        writeln!(out, "{} {}", i, j).unwrap();
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

/// Mimics C's scanf("%d", &x): skip whitespace, read optional sign, then digits.
/// Returns the parsed integer, or 0 if parsing fails (since x is initialized to 0
/// and scanf won't modify it on failure).
fn scanf_int(input: &[u8]) -> i32 {
    let mut idx = 0;
    // skip whitespace
    while idx < input.len() {
        let c = input[idx];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0b || c == 0x0c {
            idx += 1;
        } else {
            break;
        }
    }
    if idx >= input.len() {
        return 0;
    }
    let mut negative = false;
    if input[idx] == b'-' {
        negative = true;
        idx += 1;
    } else if input[idx] == b'+' {
        idx += 1;
    }
    let start = idx;
    let mut value: i64 = 0;
    while idx < input.len() && input[idx].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((input[idx] - b'0') as i64);
        idx += 1;
    }
    if idx == start {
        // No digits read, scanf would not modify x, leave as 0
        return 0;
    }
    if negative {
        value = -value;
    }
    value as i32
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();
    let x = scanf_int(&buf);
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(x, &mut out);
}
