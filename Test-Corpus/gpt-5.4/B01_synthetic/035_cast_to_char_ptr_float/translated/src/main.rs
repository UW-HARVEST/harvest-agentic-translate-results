use std::io::{self, Read};

fn print_hex(p: &[u8]) {
    for b in p {
        print!("{:02x}", b);
    }
    println!();
}

pub fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x: f32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    driver(x);
}