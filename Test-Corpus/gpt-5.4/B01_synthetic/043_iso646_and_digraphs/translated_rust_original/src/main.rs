use std::io::{self, Read};

pub fn driver(x: i32, y: i32) {
    let result = x | !y;
    println!("{}", result);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut it = input.split_whitespace();
    let x: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let y: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    driver(x, y);
}
