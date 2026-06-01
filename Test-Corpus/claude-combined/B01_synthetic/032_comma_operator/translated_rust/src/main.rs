use std::io::{self, Read, Write, BufWriter};

fn driver<W: Write>(out: &mut W, x: i32) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        writeln!(out, "{} {}", i, j).unwrap();
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

/// Read an integer from stdin in the same way scanf("%d", ...) does:
/// - skip leading whitespace (including newlines)
/// - read optional sign
/// - read decimal digits
/// Returns None if no valid integer was read (in which case x stays 0).
fn scanf_int(input: &[u8]) -> Option<i32> {
    let mut idx = 0;
    // Skip whitespace
    while idx < input.len() && (input[idx] as char).is_ascii_whitespace() {
        idx += 1;
    }
    if idx >= input.len() {
        return None;
    }
    let mut sign: i64 = 1;
    if input[idx] == b'+' {
        idx += 1;
    } else if input[idx] == b'-' {
        sign = -1;
        idx += 1;
    }
    let start = idx;
    let mut value: i64 = 0;
    while idx < input.len() && (input[idx] as u8).is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((input[idx] - b'0') as i64);
        idx += 1;
    }
    if idx == start {
        return None;
    }
    Some((value.wrapping_mul(sign)) as i32)
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();

    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&buf) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
    out.flush().unwrap();
}
