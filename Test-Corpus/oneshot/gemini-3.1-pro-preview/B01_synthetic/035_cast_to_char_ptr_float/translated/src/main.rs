use std::io;

fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    let x: f32 = input.trim().parse().unwrap_or(0.0);
    driver(x);
}