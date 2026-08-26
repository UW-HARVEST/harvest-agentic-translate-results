use std::io::{self, Read};

fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

pub fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    let mut x = 0.0f32;
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<f32>() {
                x = parsed;
            }
        }
    }
    driver(x);
}