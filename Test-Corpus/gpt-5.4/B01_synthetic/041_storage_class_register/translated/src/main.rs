use std::io::{self, Read};

pub fn driver(x: i32) {
    let mut y = 2 * x;
    y += 300;
    println!("{}", y);
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    driver(x);
}
