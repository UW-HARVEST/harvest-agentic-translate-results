use std::io::{self, Read};

fn print_hex(p: &[u8]) {
    for b in p {
        print!("{:02x}", b);
    }
    println!();
}

pub fn driver(x: i32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x = input.split_whitespace().next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    driver(x);
}
