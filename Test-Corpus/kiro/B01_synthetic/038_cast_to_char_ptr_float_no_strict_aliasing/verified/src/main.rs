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
    // scanf("%f", &x) skips leading whitespace then parses a float.
    // On parse failure, x remains 0.0.
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let x: f32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    driver(x);
}
