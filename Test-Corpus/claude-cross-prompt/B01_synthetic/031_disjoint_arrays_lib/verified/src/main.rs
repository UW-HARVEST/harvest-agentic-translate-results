// Copyright 2025 MIT Lincoln Laboratory
//
// Rust translation of c_src/src/driver.c

use std::io::{self, Read, Write};

/// Match C's `isspace` in the C locale: space, tab, LF, VT, FF, CR.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// Parse a single integer from `s` using behavior equivalent to `sscanf("%d%zn", ...)`.
///
/// Returns `Some((value, bytes_consumed))` on success, mirroring sscanf returning 1
/// (with `%n` then storing the number of characters processed). Returns `None` on a
/// matching or input failure (sscanf returns 0 / EOF).
fn scanf_int(s: &[u8]) -> Option<(i32, usize)> {
    let mut i = 0usize;

    // Skip leading whitespace, just like %d does.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }
    if i >= s.len() {
        return None;
    }

    // Optional sign.
    let mut negative = false;
    if s[i] == b'-' {
        negative = true;
        i += 1;
    } else if s[i] == b'+' {
        i += 1;
    }

    // Digits.
    let digits_start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        // No digits => matching failure.
        return None;
    }

    // Build the value using wrapping arithmetic. C's signed-overflow on sscanf is
    // undefined, but wrapping is a reasonable, deterministic choice.
    let mut val: i32 = 0;
    for &b in &s[digits_start..i] {
        let d = (b - b'0') as i32;
        val = val.wrapping_mul(10).wrapping_add(d);
    }
    if negative {
        val = val.wrapping_neg();
    }

    Some((val, i))
}

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out = vec![0i32; len];
    let ones = vec![1i32; len];
    let zeros = vec![0i32; len];

    out[0] = 0;
    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

fn driver(input: &[u8]) {
    let mut data: [i32; 100] = [0; 100];
    let mut count: usize = 0;
    let mut cursor: &[u8] = input;

    for _ in 0..100 {
        match scanf_int(cursor) {
            Some((value, nb)) => {
                data[count] = value;
                cursor = &cursor[nb..];
                count += 1;
            }
            None => break,
        }
    }

    let result = call_fma(&data, count);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    // Match printf("%d\n", result)
    writeln!(handle, "{}", result).expect("failed to write to stdout");
}

fn main() {
    // C's sscanf operates on a single buffer. Read all of stdin into memory and pass
    // it through, preserving the C behavior where %d skips arbitrary whitespace
    // (including newlines) between integers.
    let mut buf = Vec::new();
    io::stdin()
        .read_to_end(&mut buf)
        .expect("failed to read stdin");
    driver(&buf);
}
