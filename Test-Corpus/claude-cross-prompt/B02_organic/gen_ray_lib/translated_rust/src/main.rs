// Driver for the gen_ray function from the C library.
// Reads 16 whitespace-separated floats from stdin (mimicking scanf("%f", ...))
// then calls gen_ray and prints the same fields the C code would expose.

use std::io::{self, Read, Write};

use translated_rust::{c2Raycast, gen_ray};

fn read_all_stdin() -> String {
    let mut s = String::new();
    let _ = io::stdin().read_to_string(&mut s);
    s
}

/// Parse a floating point value the way C's `scanf("%f")` does:
/// skip leading whitespace, then consume the longest valid prefix.
/// Returns (value, remaining_input). On failure, returns None.
fn scanf_float(input: &str) -> Option<(f32, &str)> {
    // Skip leading whitespace (space, tab, newline, etc.)
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let mut idx = 0usize;

    // Optional sign
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        idx += 1;
    }

    let start_digits = idx;
    // Integer part
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    let mut had_digits = idx > start_digits;
    // Fraction part
    if idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
            had_digits = true;
        }
    }
    if !had_digits {
        return None;
    }
    // Exponent
    if idx < bytes.len() && (bytes[idx] == b'e' || bytes[idx] == b'E') {
        let exp_start = idx;
        idx += 1;
        if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
            idx += 1;
        }
        let exp_digits_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx == exp_digits_start {
            // No exponent digits — back up
            idx = exp_start;
        }
    }

    let token = &trimmed[..idx];
    let value: f32 = token.parse().ok()?;
    Some((value, &trimmed[idx..]))
}

fn main() {
    let input = read_all_stdin();
    let mut rest: &str = &input;

    let mut vals: [f32; 16] = [0.0; 16];
    for v in vals.iter_mut() {
        match scanf_float(rest) {
            Some((x, r)) => {
                *v = x;
                rest = r;
            }
            None => {
                // Mimic scanf reading fewer items than expected: leave default 0.0.
                break;
            }
        }
    }

    let mut cast1 = c2Raycast::default();
    let mut cast2 = c2Raycast::default();
    let mut cast3 = c2Raycast::default();

    let hit = gen_ray(
        &mut cast1, &mut cast2, &mut cast3,
        vals[0], vals[1], vals[2], vals[3],
        vals[4], vals[5], vals[6],
        vals[7], vals[8], vals[9], vals[10], vals[11],
        vals[12], vals[13], vals[14], vals[15],
    );

    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Print in a stable, deterministic format that mirrors the C struct field
    // order: hit mask, then each c2Raycast's t and n.
    let _ = writeln!(out, "hit={}", hit);
    let _ = writeln!(
        out,
        "cast1: t={:.6} n=({:.6}, {:.6})",
        cast1.t, cast1.n.x, cast1.n.y
    );
    let _ = writeln!(
        out,
        "cast2: t={:.6} n=({:.6}, {:.6})",
        cast2.t, cast2.n.x, cast2.n.y
    );
    let _ = writeln!(
        out,
        "cast3: t={:.6} n=({:.6}, {:.6})",
        cast3.t, cast3.n.x, cast3.n.y
    );
}
