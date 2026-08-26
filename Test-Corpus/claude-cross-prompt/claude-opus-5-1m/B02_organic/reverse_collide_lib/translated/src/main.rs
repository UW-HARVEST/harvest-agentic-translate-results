use std::io::{self, Read, Write};

mod lib_c;

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("failed to read stdin");

    // scanf("%f %f %f", ...) - reads three floats separated by whitespace (including newlines)
    let mut iter = input.split_ascii_whitespace();
    let x: f32 = iter
        .next()
        .expect("expected x")
        .parse()
        .expect("failed to parse x");
    let y: f32 = iter
        .next()
        .expect("expected y")
        .parse()
        .expect("failed to parse y");
    let r: f32 = iter
        .next()
        .expect("expected r")
        .parse()
        .expect("failed to parse r");

    let result = lib_c::reverse_collide(x, y, r);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "{}\n", result).expect("failed to write");
}
