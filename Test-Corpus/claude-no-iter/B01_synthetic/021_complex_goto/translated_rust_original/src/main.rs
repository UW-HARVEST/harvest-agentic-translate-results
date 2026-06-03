// Translated from c_src/src/main.c
// Reproduces the exact behavior of the original C program, including the
// goto-based control flow inside `foo`.

use std::io::{self, Read, Write};

fn foo(mut x: i32, mut y: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    'outer: while x > 0 || y > 0 {
        out.write_all(b"loop\n").unwrap();

        // Decide whether to jump straight to `label2` (skipping `label1`).
        let mut skip_label1 = x == 1 && y == 4;

        // Inner loop emulates `goto label1` re-entering the body.
        loop {
            // label1:
            if !skip_label1 {
                if x > 0 {
                    out.write_all(b"x\n").unwrap();
                    x -= 1;
                }
            }

            // label2:
            if y == 0 {
                continue 'outer;
            }
            out.write_all(b"y\n").unwrap();
            y -= 1;
            if x < 3 {
                // goto label1 — re-enter and do not skip label1 this time.
                skip_label1 = false;
                continue;
            }
            break;
        }
    }
}

/// Minimal scanf("%d %d", ...) emulation.
///
/// Reads all of stdin, then attempts to parse two whitespace-separated
/// integers (whitespace includes newlines, matching scanf). If parsing
/// fails for either value, that value is left unchanged (matching how
/// the C program leaves the corresponding variable at its initialized
/// value of 0).
fn read_two_ints(x: &mut i32, y: &mut i32) {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return;
    }

    let bytes = buf.as_bytes();
    let mut i = 0usize;

    // Parse one integer in the manner of scanf("%d", ...): skip leading
    // whitespace, optional sign, then one or more digits.
    fn parse_int(bytes: &[u8], i: &mut usize) -> Option<i32> {
        while *i < bytes.len() && (bytes[*i] as char).is_ascii_whitespace() {
            *i += 1;
        }
        if *i >= bytes.len() {
            return None;
        }

        let mut negative = false;
        if bytes[*i] == b'+' {
            *i += 1;
        } else if bytes[*i] == b'-' {
            negative = true;
            *i += 1;
        }

        let start = *i;
        let mut value: i64 = 0;
        while *i < bytes.len() && bytes[*i].is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add((bytes[*i] - b'0') as i64);
            *i += 1;
        }
        if *i == start {
            return None;
        }
        if negative {
            value = value.wrapping_neg();
        }
        // Truncate to i32 to mirror C's `int` storage.
        Some(value as i32)
    }

    if let Some(v) = parse_int(bytes, &mut i) {
        *x = v;
    } else {
        return;
    }
    if let Some(v) = parse_int(bytes, &mut i) {
        *y = v;
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    read_two_ints(&mut x, &mut y);
    foo(x, y);
    // Make sure all buffered output is flushed before exit.
    let _ = io::stdout().flush();
}
