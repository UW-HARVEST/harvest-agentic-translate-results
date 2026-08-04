// Translated from c_src/src/driver.c
// Original: Copyright 2025 MIT Lincoln Laboratory

use std::io::Read;

// CHAR_MAX in C, assuming `char` is signed (the common case on x86/x86_64
// where the original code's behavior is reproducible).
const CHAR_MAX: i8 = i8::MAX; // 127

fn print_line(line: &str) {
    // Mirrors: if(line != NULL) printf("%s\n", line);
    println!("{}", line);
}

fn print_hex_char_line(char_hex: i8) {
    // C: printf("%02x\n", charHex);
    // `charHex` is `char` which on this platform is signed; default
    // argument promotion converts it to `int` (sign-extended). `%x`
    // then reinterprets that `int` as `unsigned int`.
    let as_int: i32 = char_hex as i32;
    let as_uint: u32 = as_int as u32;
    // %02x means: hex, lowercase, minimum width 2 padded with zeros.
    println!("{:02x}", as_uint);
}

fn bad() {
    let data: i8;
    data = CHAR_MAX;
    if data > 0 {
        // result = data * 2;  (signed char arithmetic via int promotion,
        // then truncated back to signed char)
        let product: i32 = (data as i32) * 2;
        let result: i8 = product as i8; // wrap-around like C's truncation
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8;
    data = 2;
    if data > 0 {
        let product: i32 = (data as i32) * 2;
        let result: i8 = product as i8;
        print_hex_char_line(result);
    }
}

#[allow(unused_assignments)]
fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let product: i32 = (data as i32) * 2;
            let result: i8 = product as i8;
            print_hex_char_line(result);
        } else {
            print_line("data value is too large to perform arithmetic safely.");
        }
    }
    // suppress unused mut warning if compilers ever complain; keep semantics
    let _ = &mut data;
}

fn good() {
    good_g2b();
    good_b2g();
}

fn driver(use_good: i32) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

/// Mimic scanf("%d", &x): skip leading whitespace, read optional sign,
/// then read decimal digits, stopping at the first non-digit. If no
/// integer can be parsed, returns 0 (matching what an uninitialized-but-
/// zeroed int in `main` would typically be on this platform when the
/// scanf assignment fails to occur).
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < input.len() && (input[i] as char).is_whitespace() {
        i += 1;
    }
    if i >= input.len() {
        return 0;
    }
    let mut negative = false;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        negative = true;
        i += 1;
    }
    let start = i;
    while i < input.len() && (input[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return 0;
    }
    let s = std::str::from_utf8(&input[start..i]).unwrap_or("");
    let v: i64 = s.parse::<i64>().unwrap_or(0);
    let signed: i64 = if negative { -v } else { v };
    // Truncate to i32 like C would for %d into an int.
    signed as i32
}

fn main() {
    let mut buf = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut buf);
    let use_good = scanf_int(&buf);
    driver(use_good);
}
