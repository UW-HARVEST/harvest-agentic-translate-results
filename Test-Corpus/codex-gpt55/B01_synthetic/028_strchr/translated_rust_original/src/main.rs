use std::io::{self, Read};

fn foo(input: &[u8], c: u8) -> i32 {
    let end = input.iter().position(|&b| b == 0).unwrap_or(input.len());
    input[..end].iter().filter(|&&b| b == c).count() as i32
}

fn driver(input: &[u8]) {
    println!("A: {}", foo(input, b'A'));
    println!("x: {}", foo(input, b'x'));
}

fn main() {
    let mut input = [0_u8; 1000];
    let mut stdin_bytes = Vec::new();
    let _ = io::stdin().lock().take(1000).read_to_end(&mut stdin_bytes);
    input[..stdin_bytes.len()].copy_from_slice(&stdin_bytes);
    driver(&input);
}
