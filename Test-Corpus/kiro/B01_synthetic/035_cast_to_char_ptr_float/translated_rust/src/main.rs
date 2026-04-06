use std::io::{self, Read};

fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let mut x: f32 = 0.0;
    if let Some(token) = input.split_whitespace().next() {
        if let Ok(v) = token.parse::<f32>() {
            x = v;
        }
    }
    driver(x);
}
