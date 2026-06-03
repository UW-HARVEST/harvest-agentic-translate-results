// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::io::{self, Read, Write, BufWriter};

/// Mimics C `scanf("%d", &x)` as implemented by glibc:
///   - skips leading whitespace (space, tab, \n, \r, \v, \f)
///   - reads optional '+' or '-'
///   - consumes consecutive ASCII digits
///   - parses through `strtoll` (saturating to LLONG_MAX/LLONG_MIN on overflow),
///     then truncates the resulting `long long` to `int`
///   - if no digits found, leaves x unchanged
fn scanf_int<R: Read>(reader: &mut R) -> Option<i32> {
    let mut buf = [0u8; 1];

    let mut read_byte = |reader: &mut R| -> Option<u8> {
        match reader.read(&mut buf) {
            Ok(0) => None,
            Ok(_) => Some(buf[0]),
            Err(_) => None,
        }
    };

    // Skip leading whitespace as defined by C isspace() in the C locale.
    let mut c = loop {
        match read_byte(reader) {
            Some(b) if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) => continue,
            Some(b) => break b,
            None => return None,
        }
    };

    // Optional sign.
    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match read_byte(reader) {
            Some(b) => c = b,
            None => return None,
        }
    }

    if !c.is_ascii_digit() {
        // No digits; nothing matched.
        return None;
    }

    // Accumulate digits in i64 with saturation (matches glibc strtoll-on-overflow).
    // Build as a negative number to handle i64::MIN cleanly.
    let mut value: i64 = 0;
    let mut saturated = false;
    loop {
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as i64;
        if !saturated {
            match value.checked_mul(10).and_then(|v| v.checked_sub(d)) {
                Some(v) => value = v,
                None => {
                    saturated = true;
                    value = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        match read_byte(reader) {
            Some(b) => c = b,
            None => break,
        }
    }

    let final_i64 = if saturated {
        value
    } else if negative {
        value
    } else {
        // Accumulated as negative. For a positive input, hitting i64::MIN
        // means the magnitude exceeds i64::MAX, which is overflow.
        if value == i64::MIN {
            i64::MAX
        } else {
            -value
        }
    };

    // Truncate long long → int (matches what glibc does after strtoll).
    Some(final_i64 as i32)
}

fn print_hex<W: Write>(out: &mut W, bytes: &[u8]) {
    for b in bytes {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: i32) {
    // Match C: print the raw bytes of `int x` in native (little-endian on x86) order.
    let bytes = x.to_ne_bytes();
    print_hex(out, &bytes);
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = BufWriter::new(stdout.lock());

    let x: i32 = scanf_int(&mut input).unwrap_or(0);
    driver(&mut output, x);
}
