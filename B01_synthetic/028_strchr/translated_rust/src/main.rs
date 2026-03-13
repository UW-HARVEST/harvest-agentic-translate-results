use std::io::Read;

fn foo(input: &[u8], c: u8) -> i32 {
    // Replicate strchr: only search up to first null byte
    let len = input.iter().position(|&b| b == 0).unwrap_or(input.len());
    input[..len].iter().filter(|&&b| b == c).count() as i32
}

fn driver(input: &[u8]) {
    println!("A: {}", foo(input, b'A'));
    println!("x: {}", foo(input, b'x'));
}

fn main() {
    let mut buf = [0u8; 1000];
    let _ = std::io::stdin().read(&mut buf);
    driver(&buf);
}
