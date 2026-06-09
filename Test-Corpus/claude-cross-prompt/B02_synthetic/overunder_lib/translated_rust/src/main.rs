// Executable entry point. The C library exposes `overunder(int, int, int, int)`.
// Read four ints from stdin (scanf-style: whitespace-separated, may span newlines)
// and call overunder. The function itself prints all output to stdout.

use std::io::Read;

mod core;
use crate::core::overunder;

/// Mimic C's `scanf("%d", &x)`: skip leading whitespace, then parse an optional
/// sign and a sequence of decimal digits, stopping at the first non-digit char.
/// Returns None if no integer could be parsed (e.g., EOF reached after WS).
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (space, tab, newline, CR, vertical-tab, form-feed).
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

    let mut neg = false;
    if input[*pos] == b'+' {
        *pos += 1;
    } else if input[*pos] == b'-' {
        neg = true;
        *pos += 1;
    }

    let start = *pos;
    let mut value: i64 = 0;
    while *pos < input.len() {
        let c = input[*pos];
        if c.is_ascii_digit() {
            value = value
                .saturating_mul(10)
                .saturating_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos == start {
        // No digits consumed.
        return None;
    }

    // Mimic C's int truncation when the value overflows: clamp into i32 range.
    if neg {
        value = -value;
    }
    if value > i32::MAX as i64 {
        Some(i32::MAX)
    } else if value < i32::MIN as i64 {
        Some(i32::MIN)
    } else {
        Some(value as i32)
    }
}

fn main() {
    let mut buf = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut buf);

    let mut pos = 0usize;
    let a = scanf_int(&buf, &mut pos).unwrap_or(0);
    let b = scanf_int(&buf, &mut pos).unwrap_or(0);
    let c = scanf_int(&buf, &mut pos).unwrap_or(0);
    let d = scanf_int(&buf, &mut pos).unwrap_or(0);

    let _ = overunder(a, b, c, d);
}
