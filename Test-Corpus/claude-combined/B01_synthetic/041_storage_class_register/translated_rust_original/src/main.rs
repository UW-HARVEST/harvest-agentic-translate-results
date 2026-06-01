use std::io::{self, Read, Write};

fn driver(x: i32) {
    // C: register int y = 2*x; y += 300; printf("%d\n", y);
    // Use wrapping arithmetic to mirror C int overflow semantics.
    let mut y: i32 = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", y);
}

/// Mimics scanf("%d", &x). On failure or EOF, x is left as the initial value (0).
/// scanf "%d" skips leading whitespace, then reads an optional sign and digits.
fn scan_int(input: &[u8]) -> Option<(i32, usize)> {
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace: space, \t, \n, \v, \f, \r)
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
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
    let mut value: i32 = 0;
    let mut any_digit = false;
    while i < input.len() {
        let c = input[i];
        if c.is_ascii_digit() {
            any_digit = true;
            let d = (c - b'0') as i32;
            // C scanf with %d on overflow has undefined behavior; mimic with wrapping.
            if negative {
                value = value.wrapping_mul(10).wrapping_sub(d);
            } else {
                value = value.wrapping_mul(10).wrapping_add(d);
            }
            i += 1;
        } else {
            break;
        }
    }

    if !any_digit {
        // Restore position to before sign so that conversion fails
        let _ = start;
        return None;
    }
    Some((value, i))
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        // On read error, behave like scanf failure: x stays 0
        driver(0);
        return;
    }

    let x = match scan_int(&input) {
        Some((v, _)) => v,
        None => 0,
    };

    driver(x);
}
