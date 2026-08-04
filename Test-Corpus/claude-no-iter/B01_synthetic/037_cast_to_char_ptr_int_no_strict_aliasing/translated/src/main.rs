// Translated from C: reads an integer with scanf("%d") then prints its
// raw little-endian bytes as hex.

use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(out: &mut W, p: &[u8]) {
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: i32) {
    // Mimic: char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    // On the target platform (x86_64 Linux), int is 32-bit little-endian.
    let raw = x.to_ne_bytes();
    print_hex(out, &raw);
}

/// Replicates scanf("%d", &x) semantics:
/// - Skip leading whitespace (per isspace: space, \t, \n, \v, \f, \r)
/// - Accept optional '+' or '-' sign
/// - Read decimal digits; stop at first non-digit
/// - Returns the parsed value, or None if no digits were read
fn scanf_int(input: &[u8]) -> Option<i32> {
    let mut i = 0;
    // Skip whitespace
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0B || c == 0x0C || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    if i >= input.len() {
        return None;
    }

    let mut negative = false;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        negative = true;
        i += 1;
    }

    let start = i;
    let mut value: i64 = 0;
    while i < input.len() && input[i].is_ascii_digit() {
        value = value.wrapping_mul(10);
        value = value.wrapping_add((input[i] - b'0') as i64);
        i += 1;
    }

    if i == start {
        return None;
    }

    if negative {
        value = value.wrapping_neg();
    }

    // Truncate to i32 like C would for a 32-bit int
    Some(value as i32)
}

fn main() {
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer).unwrap();

    let x: i32 = scanf_int(&buffer).unwrap_or(0);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
}
