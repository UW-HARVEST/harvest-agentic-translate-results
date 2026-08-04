use std::io::{self, Read};

fn driver(f: f64) {
    let x = f.to_bits();
    println!("{:x} {:a} {:.4}", x, f, f);
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let f = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    driver(f);
}
