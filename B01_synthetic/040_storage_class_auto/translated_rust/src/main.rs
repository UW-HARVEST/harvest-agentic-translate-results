use std::io::{self, Read};

fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    println!("{}", y);
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    // scanf("%d", &x) skips whitespace, parses one integer; x stays 0 on failure
    let x: i32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
    driver(x);
}
