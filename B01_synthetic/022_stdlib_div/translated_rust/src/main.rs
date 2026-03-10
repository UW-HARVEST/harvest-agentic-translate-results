use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let mut iter = input.split_whitespace();
    let x: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let y: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let quot = x / y;
    let rem = x % y;
    println!("quotient: {}, remainder: {}", quot, rem);
}
