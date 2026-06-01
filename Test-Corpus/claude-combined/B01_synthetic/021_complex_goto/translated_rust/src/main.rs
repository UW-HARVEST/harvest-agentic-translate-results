use std::io::{self, Read};

fn foo(mut x: i32, mut y: i32) {
    while x > 0 || y > 0 {
        println!("loop");

        // If x == 1 && y == 4, goto label2 (skip label1 block on first inner iteration)
        let mut goto_label2 = x == 1 && y == 4;

        loop {
            if !goto_label2 {
                // label1:
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            // After this point we always enter label2 region; subsequent
            // iterations from `goto label1` should go through the label1 block.
            goto_label2 = false;

            // label2:
            if y == 0 {
                break; // continue (back to while header)
            }
            println!("y");
            y -= 1;
            if x < 3 {
                continue; // goto label1
            }
            break; // fall off end of while body
        }
    }
}

/// Mimic scanf("%d %d", &x, &y) with initial values 0, 0.
/// scanf skips leading whitespace, reads an optional sign and decimal digits,
/// stopping at the first non-matching character. If it fails to parse, the
/// variable retains its prior value.
fn parse_two_ints(input: &str) -> (i32, i32) {
    let bytes = input.as_bytes();
    let mut pos = 0;
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    fn skip_ws(bytes: &[u8], pos: &mut usize) {
        while *pos < bytes.len() {
            let c = bytes[*pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
                *pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_int(bytes: &[u8], pos: &mut usize) -> Option<i32> {
        let start = *pos;
        let mut neg = false;
        if *pos < bytes.len() && (bytes[*pos] == b'+' || bytes[*pos] == b'-') {
            neg = bytes[*pos] == b'-';
            *pos += 1;
        }
        let digit_start = *pos;
        while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
            *pos += 1;
        }
        if *pos == digit_start {
            // No digits parsed; rewind and fail
            *pos = start;
            return None;
        }
        // Use wrapping arithmetic to mirror typical scanf behavior on overflow
        // (undefined in C, but we just compute as i64 then truncate).
        let s = std::str::from_utf8(&bytes[digit_start..*pos]).unwrap();
        let mut acc: i64 = 0;
        for ch in s.bytes() {
            acc = acc.wrapping_mul(10).wrapping_add((ch - b'0') as i64);
        }
        if neg {
            acc = acc.wrapping_neg();
        }
        Some(acc as i32)
    }

    skip_ws(bytes, &mut pos);
    if let Some(v) = parse_int(bytes, &mut pos) {
        x = v;
        skip_ws(bytes, &mut pos);
        if let Some(v) = parse_int(bytes, &mut pos) {
            y = v;
        }
    }

    (x, y)
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let (x, y) = parse_two_ints(&input);
    foo(x, y);
}
