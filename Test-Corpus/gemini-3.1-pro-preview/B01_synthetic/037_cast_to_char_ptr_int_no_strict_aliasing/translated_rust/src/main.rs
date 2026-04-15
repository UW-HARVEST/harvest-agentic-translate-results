use std::io;

fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

fn driver(x: i32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    let x: i32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
    driver(x);
}
