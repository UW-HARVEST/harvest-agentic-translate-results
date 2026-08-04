// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output for the same inputs.
//
// Note: The original C program calls a `bad()` function that dereferences an
// uninitialized pointer (undefined behavior). On the reference platform, that
// dereference happens to produce the integer value 0. To match the C output
// byte-for-byte, this Rust translation prints "0\n" for the bad() path rather
// than invoking actual UB.

use std::io::{self, Read, Write};

fn print_int_ptr_line<W: Write>(out: &mut W, int_number: i32) {
    // Mirrors the C printf("%d\n", *intNumber).
    writeln!(out, "{}", int_number).unwrap();
}

fn bad<W: Write>(out: &mut W) {
    // The C version dereferences an uninitialized `int *data`, which is
    // undefined behavior. On the reference platform this happens to print 0.
    print_int_ptr_line(out, 0);
}

fn good<W: Write>(out: &mut W) {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(out, *data_addr);
}

/// Mimic C's `scanf("%d", &x)` behavior: skip leading whitespace, then parse
/// an optional sign followed by decimal digits. Returns Some(value) if at
/// least one digit was consumed, otherwise None.
fn scanf_int(input: &[u8]) -> Option<i32> {
    let mut i = 0;
    // Skip whitespace (matching isspace: space, \t, \n, \v, \f, \r).
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\x0b' || c == b'\x0c' || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    if i >= input.len() {
        return None;
    }

    let start = i;
    // Optional sign.
    if input[i] == b'+' || input[i] == b'-' {
        i += 1;
    }

    let digits_start = i;
    while i < input.len() && input[i].is_ascii_digit() {
        i += 1;
    }

    if i == digits_start {
        // No digits consumed.
        return None;
    }

    // Parse the number. C's scanf with %d would on overflow behave UB; for
    // typical inputs in range we use wrapping parse via i64 then cast.
    let s = std::str::from_utf8(&input[start..i]).ok()?;
    match s.parse::<i64>() {
        Ok(v) => Some(v as i32),
        Err(_) => None,
    }
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();

    // x is initialized to 0 in main; if scanf doesn't match, x stays 0.
    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&buf) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    if x != 0 {
        good(&mut out);
    } else {
        bad(&mut out);
    }
}
