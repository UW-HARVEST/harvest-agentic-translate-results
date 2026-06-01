use std::io::{self, Read};

fn driver(x: i32, y: i32) {
    // C code: x bitor compl y  ==  x | ~y
    let result = x | !y;
    print!("{}", result);
    println!();
}

/// Read the next whitespace-separated integer token from `iter`.
/// Mimics C's `scanf("%d", ...)` behavior: skips leading whitespace,
/// then reads an optional sign followed by decimal digits, stopping at
/// the first non-digit. Wraps on overflow like C does for signed ints.
fn scan_int<I: Iterator<Item = u8>>(iter: &mut std::iter::Peekable<I>) -> Option<i32> {
    // Skip whitespace
    while let Some(&b) = iter.peek() {
        if (b as char).is_ascii_whitespace() {
            iter.next();
        } else {
            break;
        }
    }
    let mut sign: i32 = 1;
    match iter.peek() {
        Some(&b'+') => {
            iter.next();
        }
        Some(&b'-') => {
            sign = -1;
            iter.next();
        }
        _ => {}
    }
    let mut any = false;
    let mut val: i32 = 0;
    while let Some(&b) = iter.peek() {
        if b.is_ascii_digit() {
            any = true;
            val = val.wrapping_mul(10).wrapping_add((b - b'0') as i32);
            iter.next();
        } else {
            break;
        }
    }
    if !any {
        return None;
    }
    Some(val.wrapping_mul(sign))
}

fn main() {
    let mut buf = Vec::new();
    // Read all stdin; scanf reads across newlines until it finds the
    // requested number of integers (or EOF).
    io::stdin().read_to_end(&mut buf).ok();
    let mut iter = buf.into_iter().peekable();

    // Match C: variables are zero-initialized; if scanf fails to match,
    // they remain at 0.
    let x = scan_int(&mut iter).unwrap_or(0);
    let y = scan_int(&mut iter).unwrap_or(0);
    driver(x, y);
}
