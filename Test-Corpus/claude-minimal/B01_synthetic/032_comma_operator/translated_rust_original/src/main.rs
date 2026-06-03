use std::io::{self, Read, Write, BufWriter};

fn driver(x: i32) {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut j: i32 = 0;
    let mut i: i32 = 0;
    while i < x {
        let _ = writeln!(out, "{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");

    // Mimic scanf("%d", &x): parse the first integer token from input.
    // If parsing fails, x remains 0 (matching the C initialization).
    let x: i32 = input
        .split_ascii_whitespace()
        .next()
        .and_then(|tok| tok.parse::<i32>().ok())
        .unwrap_or(0);

    driver(x);
}
