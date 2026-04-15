use std::io::Read;

fn foo(in_bytes: &[u8], c: u8) -> usize {
    in_bytes.iter().filter(|&&b| b == c).count()
}

fn driver(in_bytes: &[u8]) {
    println!("A: {}", foo(in_bytes, b'A'));
    println!("x: {}", foo(in_bytes, b'x'));
}

fn main() {
    let mut buffer = [0u8; 1000];
    let _ = std::io::stdin().read(&mut buffer);
    let null_pos = buffer.iter().position(|&b| b == 0).unwrap_or(1000);
    driver(&buffer[..null_pos]);
}
