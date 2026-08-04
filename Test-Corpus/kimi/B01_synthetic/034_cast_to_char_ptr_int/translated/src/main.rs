use std::io;

fn print_hex(p: &[u8]) {
    for byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

fn driver(x: i32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap();
    driver(x);
}