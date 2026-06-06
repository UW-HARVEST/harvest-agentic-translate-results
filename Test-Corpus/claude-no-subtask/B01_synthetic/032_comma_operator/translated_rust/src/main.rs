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

/// Mimic scanf("%d", &x): skip leading whitespace, then parse optional sign
/// and digits. Returns the parsed value, or None if no integer was found.
fn scan_int(bytes: &[u8]) -> Option<i32> {
    let mut idx = 0;
    // Skip leading whitespace (matches isspace in C locale: space, tab, \n, \v, \f, \r)
    while idx < bytes.len() {
        let b = bytes[idx];
        if b == b' ' || b == b'\t' || b == b'\n' || b == 0x0B || b == 0x0C || b == b'\r' {
            idx += 1;
        } else {
            break;
        }
    }
    if idx >= bytes.len() {
        return None;
    }

    let mut negative = false;
    if bytes[idx] == b'+' {
        idx += 1;
    } else if bytes[idx] == b'-' {
        negative = true;
        idx += 1;
    }

    let start = idx;
    let mut value: i64 = 0;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b.is_ascii_digit() {
            value = value.wrapping_mul(10).wrapping_add((b - b'0') as i64);
            idx += 1;
        } else {
            break;
        }
    }

    if idx == start {
        return None;
    }

    if negative {
        value = value.wrapping_neg();
    }
    Some(value as i32)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let x = scan_int(&input).unwrap_or(0);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(x, &mut out);
}
