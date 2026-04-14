use std::io::{self, Read};

fn print_hex(p: &[u8]) {
    for b in p {
        print!("{:02x}", b);
    }
    println!();
}

pub fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        if let Ok(x) = input.split_whitespace().next().unwrap_or("").parse::<f32>() {
            driver(x);
        }
    }
}
