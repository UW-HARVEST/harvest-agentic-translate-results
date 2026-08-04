use std::io;

fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

pub fn driver(x: i32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut x = 0;
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<i32>() {
                x = parsed;
            }
        }
    }
    driver(x);
}