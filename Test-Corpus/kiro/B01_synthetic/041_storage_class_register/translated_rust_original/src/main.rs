use std::io::{self, Read};

fn driver(x: i32) {
    let y: i32 = 2_i32.wrapping_mul(x).wrapping_add(300);
    println!("{}", y);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.split_whitespace().next().map(|s| s.parse().unwrap()).unwrap_or(0);
    driver(x);
}
