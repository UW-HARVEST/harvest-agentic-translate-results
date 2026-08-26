use std::io::{self, Read, Write};

fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", y);
}

/// Mimic C's `scanf("%d", &x)`: skip leading whitespace, then parse an
/// optionally-signed decimal integer. If no valid integer can be parsed,
/// the destination variable retains its prior value (here, 0).
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (matches isspace in C locale "C").
    while *pos < input.len() && is_c_whitespace(input[*pos]) {
        *pos += 1;
    }

    if *pos >= input.len() {
        return None;
    }

    let start = *pos;
    let mut idx = *pos;

    // Optional sign
    if input[idx] == b'+' || input[idx] == b'-' {
        idx += 1;
    }

    let digits_start = idx;
    while idx < input.len() && input[idx].is_ascii_digit() {
        idx += 1;
    }

    if idx == digits_start {
        // No digits parsed; per scanf, the conversion fails and the
        // input position is left at the start of the would-be number
        // (after consumed whitespace). We don't advance pos in that case.
        *pos = start;
        return None;
    }

    let s = std::str::from_utf8(&input[start..idx]).ok()?;
    // C's scanf with %d on overflow has undefined behavior; emulate by
    // saturating using wrapping parse via i64 then casting.
    let parsed: i64 = match s.parse::<i64>() {
        Ok(v) => v,
        Err(_) => {
            *pos = idx;
            return None;
        }
    };
    *pos = idx;
    Some(parsed as i32)
}

fn is_c_whitespace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);
    let mut pos = 0usize;

    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&input, &mut pos) {
        x = v;
    }
    driver(x);
}
