use std::io;

fn driver(f: f64) {
    let u = f.to_bits();
    println!("{:x} {:e} {:.4}", u, f, f);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let f: f64 = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    driver(f);
}
