use std::io::Read;

fn foo(input: &[u8], c: u8) -> i32 {
    input.iter().filter(|&&b| b == c).count() as i32
}

fn driver(input: &[u8]) {
    println!("A: {}", foo(input, b'A'));
    println!("x: {}", foo(input, b'x'));
}

fn main() {
    let mut buf = [0u8; 1000];
    let n = std::io::stdin().read(&mut buf).unwrap_or(0);
    driver(&buf[..n]);
}
