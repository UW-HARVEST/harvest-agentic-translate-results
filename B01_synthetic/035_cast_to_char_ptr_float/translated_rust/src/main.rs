use std::io;

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
    io::stdin().read_line(&mut input).unwrap();
    let x: f32 = input.trim().parse().unwrap_or(0.0);
    driver(x);
}
