// Copyright 2025 MIT Lincoln Laboratory
// Driver executable that mirrors the C `driver` library function.

mod driver;

use std::io::{self, Read};

/// Reads all of stdin and parses two whitespace-separated integers,
/// matching how C's `scanf("%d %d", ...)` would consume input.
fn read_two_ints() -> Option<(i32, i32)> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    let mut it = buf.split_ascii_whitespace();
    let x: i32 = it.next()?.parse().ok()?;
    let y: i32 = it.next()?.parse().ok()?;
    Some((x, y))
}

fn main() {
    match read_two_ints() {
        Some((x, y)) => driver::driver(x, y),
        None => {
            // If we couldn't read two integers, exit silently — matching
            // the behavior of a C program where scanf failed to populate
            // the variables (which would then have indeterminate values).
            std::process::exit(1);
        }
    }
}
