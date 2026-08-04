// Translated from C. The C source provides only a `driver(int x)` function.
// This main reads an integer from stdin (mirroring `scanf("%d", &x)` semantics)
// and invokes driver(x), so the program operates as an executable.

mod driver;

use std::io::Read;

/// Read a single integer from stdin using scanf("%d", ...) semantics:
/// - Skip leading whitespace (space, tab, newline, vertical tab, form feed, carriage return)
/// - Optional sign (+ or -)
/// - One or more decimal digits
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0B || c == 0x0C || c == b'\r' {
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
    if input[*pos] == b'-' {
        negative = true;
        *pos += 1;
    } else if input[*pos] == b'+' {
        *pos += 1;
    }

    let digit_start = *pos;
    let mut value: i64 = 0;
    while *pos < input.len() {
        let c = input[*pos];
        if c.is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos == digit_start {
        // No digits consumed — match scanf's failure: rewind.
        *pos = start;
        return None;
    }

    if negative {
        value = -value;
    }
    Some(value as i32)
}

fn main() {
    let mut buf = Vec::new();
    if std::io::stdin().read_to_end(&mut buf).is_err() {
        return;
    }
    let mut pos = 0usize;
    let x = match scanf_int(&buf, &mut pos) {
        Some(v) => v,
        None => return,
    };
    driver::driver(x);
}
