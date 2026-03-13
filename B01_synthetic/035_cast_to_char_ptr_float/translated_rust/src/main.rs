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
    // scanf("%f", &x) skips whitespace then parses a float; on failure x stays 0.0
    let x: f32 = input.trim().parse().unwrap_or(0.0_f32);
    driver(x);
}
