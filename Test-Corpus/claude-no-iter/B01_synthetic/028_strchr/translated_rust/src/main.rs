// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::io::{self, Read};

/// Counts occurrences of byte `c` in `input` up to (but not including)
/// the first NUL byte. This mirrors the C `strchr`-based loop, which
/// scans the C string only as far as the NUL terminator.
fn foo(input: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    for &b in input.iter() {
        if b == 0 {
            break;
        }
        if b == c {
            res += 1;
        }
    }
    res
}

fn driver(input: &[u8]) {
    println!("A: {}", foo(input, b'A'));
    println!("x: {}", foo(input, b'x'));
}

fn main() {
    // Mimic `char in[1000] = "";` which zero-initializes a 1000-byte buffer.
    let mut buf: [u8; 1000] = [0; 1000];

    // Mimic `fread(in, 1, sizeof(in), stdin)`: read up to 1000 bytes from
    // stdin into the buffer. We read until EOF or the buffer fills up,
    // ignoring the byte count return value (as the C code does).
    let mut stdin = io::stdin().lock();
    let mut total = 0usize;
    while total < buf.len() {
        match stdin.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
    }

    driver(&buf);
}
