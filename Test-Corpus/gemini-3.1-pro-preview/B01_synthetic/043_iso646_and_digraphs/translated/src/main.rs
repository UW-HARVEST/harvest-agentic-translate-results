use std::io::{self, Read};

fn driver(x: i32, y: i32) {
    let result = x | !y;
    println!("{}", result);
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        let mut tokens = input.split_whitespace();
        let x: i32 = tokens.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let y: i32 = tokens.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        driver(x, y);
    }
}
