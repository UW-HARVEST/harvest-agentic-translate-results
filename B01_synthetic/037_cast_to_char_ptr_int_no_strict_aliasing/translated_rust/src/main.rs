use std::io::{self, Read};

fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

fn driver(x: i32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let token = input.split_whitespace().next().unwrap_or("");
    let x: i32 = token.parse().unwrap_or(0);
    driver(x);
}
