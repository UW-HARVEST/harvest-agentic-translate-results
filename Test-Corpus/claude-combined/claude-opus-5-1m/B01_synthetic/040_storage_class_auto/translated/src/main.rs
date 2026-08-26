use std::io::{self, Read, Write};

fn driver(x: i32) {
    let mut y: i32 = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", y);
}

/// Mimic C's `scanf("%d", &x)` behavior:
/// - Skips leading whitespace (matches isspace).
/// - Optional leading '+' or '-'.
/// - Consumes consecutive ASCII digits.
/// - Returns the parsed value (saturating into i32 on overflow is UB in C;
///   we use wrapping arithmetic to mimic typical x86_64 GCC behavior).
/// - Returns None if no digits could be read (in which case the caller
///   should leave the destination variable at its prior value, matching
///   scanf's behavior of not modifying the destination on conversion
///   failure).
fn scanf_int(input: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    // Skip whitespace as per C isspace (' ', '\t', '\n', '\v', '\f', '\r').
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

    if i >= input.len() || !input[i].is_ascii_digit() {
        return None;
    }

    let mut value: i32 = 0;
    while i < input.len() && input[i].is_ascii_digit() {
        let digit = (input[i] - b'0') as i32;
        // Mimic C overflow behavior using wrapping arithmetic.
        value = value.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }

    if negative {
        value = value.wrapping_neg();
    }

    Some(value)
}

fn main() {
    let mut buf = Vec::new();
    let _ = io::stdin().read_to_end(&mut buf);

    let mut x: i32 = 0;
    if let Some(parsed) = scanf_int(&buf) {
        x = parsed;
    }
    driver(x);
}
