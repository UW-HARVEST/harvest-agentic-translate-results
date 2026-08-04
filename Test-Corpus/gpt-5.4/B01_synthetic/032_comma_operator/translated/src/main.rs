use std::io::{self, Read};

fn driver(x: i32) {
    let mut j = 0;
    for i in 0..x {
        println!("{} {}", i, j);
        j += 2;
    }
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
