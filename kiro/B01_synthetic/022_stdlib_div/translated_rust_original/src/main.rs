use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let mut nums = input.split_whitespace();
    let x: i32 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let y: i32 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(1);

    let quot = x / y;
    let rem = x % y;
    println!("quotient: {}, remainder: {}", quot, rem);
}
