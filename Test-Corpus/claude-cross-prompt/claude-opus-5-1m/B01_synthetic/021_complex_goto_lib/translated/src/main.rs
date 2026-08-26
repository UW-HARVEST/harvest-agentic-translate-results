// Translation of c_src/src/driver.c to Rust.
//
// The original C is a library exposing `void driver(int x, int y)`.
// The task specifies this is an executable, so a small main is provided
// that reads two integers from stdin (scanf("%d %d") semantics) and
// calls `driver` with them, producing byte-identical output to a C
// program that does the same.

use std::io::{self, Read, Write};

fn driver(mut x: i32, mut y: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    'outer: while x > 0 || y > 0 {
        let _ = out.write_all(b"loop\n");

        // `if (x == 1 && y == 4) goto label2;` skips the label1 block
        // for this iteration of the outer while loop only.
        let mut skip_label1 = x == 1 && y == 4;

        // Inner loop emulates `goto label1` (which is reachable only
        // from the bottom of the loop body via `if (x < 3) goto label1;`).
        loop {
            // label1:
            if !skip_label1 {
                if x > 0 {
                    let _ = out.write_all(b"x\n");
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2:
            if y == 0 {
                continue 'outer;
            }
            let _ = out.write_all(b"y\n");
            y -= 1;
            if x < 3 {
                // goto label1
                continue;
            }
            break;
        }
    }
}

/// Read two integers from stdin using scanf("%d %d") semantics:
/// skip leading whitespace (including newlines), parse an optional
/// sign followed by decimal digits.
fn read_two_ints() -> (i32, i32) {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut values: Vec<i32> = Vec::with_capacity(2);

    while values.len() < 2 {
        // Skip whitespace (matches C isspace for the common cases).
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // Optional sign.
        let mut neg = false;
        if bytes[i] == b'+' {
            i += 1;
        } else if bytes[i] == b'-' {
            neg = true;
            i += 1;
        }
        // Digits.
        let start = i;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }
        if start == i {
            // Not a valid integer; mimic scanf returning fewer matches
            // by leaving the value uninitialised. We default to 0.
            break;
        }
        // Parse with wrapping behaviour to mimic C's int overflow tolerance
        // for typical inputs (full UB-correct emulation is not required).
        let mut value: i32 = 0;
        for &b in &bytes[start..i] {
            let d = (b - b'0') as i32;
            value = value.wrapping_mul(10).wrapping_add(d);
        }
        if neg {
            value = value.wrapping_neg();
        }
        values.push(value);
    }

    while values.len() < 2 {
        values.push(0);
    }
    (values[0], values[1])
}

fn main() {
    let (x, y) = read_two_ints();
    driver(x, y);
    let _ = io::stdout().flush();
}
