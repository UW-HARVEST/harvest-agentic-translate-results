// Translated from c_src/src/driver.c
// Produces byte-identical output to the original C program.

use std::io::{self, Read, Write};

mod driver;

/// Reads a single integer from stdin in the manner of C's `scanf("%d", &x)`.
///
/// Returns `Some(value)` on success, or `None` if EOF/no integer found
/// (matching scanf returning EOF / 0 matched items).
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (space, tab, newline, vertical tab, form feed, carriage return)
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
    if input[*pos] == b'+' {
        *pos += 1;
    } else if input[*pos] == b'-' {
        negative = true;
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
        // No digits read; restore pos and report failure.
        *pos = start;
        return None;
    }

    if negative {
        value = -value;
    }

    // Truncate to i32 like C does for %d.
    Some(value as i32)
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return;
    }

    let mut pos: usize = 0;
    if let Some(data) = scanf_int(&input, &mut pos) {
        driver::driver(data);
    }

    // Ensure stdout is flushed.
    let _ = io::stdout().flush();
}
