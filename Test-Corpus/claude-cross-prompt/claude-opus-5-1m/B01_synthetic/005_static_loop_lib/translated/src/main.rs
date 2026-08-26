// Rust translation of staticloop.c
// Original: Copyright 2025 MIT Lincoln Laboratory

use std::cell::Cell;
use std::io::{self, Read, Write};

thread_local! {
    static SUM: Cell<i32> = const { Cell::new(0) };
}

/// Mirror of static_sum from staticloop.c.
/// Maintains a running total in a thread-local static.
fn static_sum(update: i32) -> i32 {
    SUM.with(|sum| {
        let new_val = sum.get().wrapping_add(update);
        sum.set(new_val);
        new_val
    })
}

/// Mirror of driver from staticloop.c.
/// Maintain a running total using a static variable.
fn driver(stride: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for i in 0..10i32 {
        let v = static_sum(i.wrapping_mul(stride));
        // printf("%d\n", v) — match C exactly
        writeln!(out, "{}", v).unwrap();
    }
}

/// Reads the next signed-decimal integer from stdin, mimicking scanf("%d", ...).
/// Skips leading whitespace, then reads optional sign and digits.
/// Returns None if no integer could be parsed (EOF before any digits).
fn scanf_int<R: Read>(reader: &mut R) -> Option<i32> {
    let mut buf = [0u8; 1];
    // Skip whitespace
    let mut c;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => {
                c = buf[0];
                if !(c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
                    || c == 0x0b || c == 0x0c)
                {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    let mut sign: i64 = 1;
    if c == b'-' {
        sign = -1;
        match reader.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => c = buf[0],
            Err(_) => return None,
        }
    } else if c == b'+' {
        match reader.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => c = buf[0],
            Err(_) => return None,
        }
    }

    if !c.is_ascii_digit() {
        return None;
    }

    let mut value: i64 = 0;
    loop {
        if c.is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add((c - b'0') as i64);
        } else {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => c = buf[0],
            Err(_) => break,
        }
    }

    Some((value.wrapping_mul(sign)) as i32)
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let stride = scanf_int(&mut handle).unwrap_or(0);
    driver(stride);
}
