// Translation of C driver to Rust producing byte-identical output.
// Original C exposes a library function driver(int x) that prints the
// raw bytes of `x` (an `int`, treated as unsigned char) in hex, followed
// by a newline. To make this an executable we read an int from stdin
// (scanf-style) and invoke driver().

use std::io::{self, Read, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: i32) {
    // Match C's `print_hex((unsigned char *)&x, sizeof(x))`. On the
    // reference C build (x86_64 Linux) `sizeof(int) == 4` and the host
    // is little-endian, so produce the same 4-byte little-endian image.
    let bytes = x.to_le_bytes();
    print_hex(&bytes);
}

fn main() {
    // Read the entire stdin and parse the first whitespace-delimited
    // integer, mirroring scanf("%d", ...) semantics (reads across
    // newlines, skips leading whitespace).
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let token = input.split_ascii_whitespace().next();
    let x: i32 = match token {
        Some(t) => match t.parse::<i32>() {
            Ok(v) => v,
            Err(_) => return,
        },
        None => return,
    };

    driver(x);
}
