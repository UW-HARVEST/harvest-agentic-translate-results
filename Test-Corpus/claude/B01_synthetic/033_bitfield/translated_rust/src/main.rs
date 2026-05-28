// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from c_src/src/main.c

use std::io::{self, Read, Write, BufWriter};

/// Parse an integer token like C's scanf("%d") / scanf("%u").
/// Returns the parsed value as i64, or None on match failure.
/// Consumes characters from the input iterator: skips leading whitespace,
/// then reads optional sign and decimal digits.
fn read_int_token(input: &mut std::iter::Peekable<std::vec::IntoIter<u8>>) -> Option<i64> {
    // Skip leading whitespace
    while let Some(&c) = input.peek() {
        if (c as char).is_ascii_whitespace() {
            input.next();
        } else {
            break;
        }
    }
    let mut s = String::new();
    if let Some(&c) = input.peek() {
        if c == b'+' || c == b'-' {
            s.push(c as char);
            input.next();
        }
    }
    let mut had_digit = false;
    while let Some(&c) = input.peek() {
        if (c as char).is_ascii_digit() {
            s.push(c as char);
            input.next();
            had_digit = true;
        } else {
            break;
        }
    }
    if !had_digit {
        return None;
    }
    s.parse::<i64>().ok()
}

fn print_foo(out: &mut impl Write, x: u32, y: u32, b: bool, z: i32) {
    // Simulate C bitfields:
    //   unsigned int x : 2  -> mask to 2 bits
    //   unsigned int y : 3  -> mask to 3 bits
    //   bool b : 1          -> 0 or 1
    //   int z               -> full width int
    let fx = x & 0x3;
    let fy = y & 0x7;
    let fb: i32 = if b { 1 } else { 0 };
    writeln!(out, "{} {} {} {}", fx, fy, fb, z).unwrap();
}

fn driver(out: &mut impl Write, x: u32, y: u32, b: bool, z: i32) {
    print_foo(out, x, y, b, z);
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).expect("failed to read stdin");
    let mut iter = buf.into_iter().peekable();

    // Match C: variables initialized to 0; on scanf match failure, they stay 0.
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    if let Some(v) = read_int_token(&mut iter) {
        // %u — reinterpret as u32 (matches glibc behavior of accepting signed input)
        x = v as i64 as u32;
    }
    if let Some(v) = read_int_token(&mut iter) {
        y = v as i64 as u32;
    }
    if let Some(v) = read_int_token(&mut iter) {
        b = v as i32;
    }
    if let Some(v) = read_int_token(&mut iter) {
        z = v as i32;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    driver(&mut out, x, y, b != 0, z);

    out.flush().unwrap();
}
