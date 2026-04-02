use std::io::Read;

fn foo(input: &[u8], c: u8) -> i32 {
    let mut count: i32 = 0;
    for &b in input {
        if b == 0 {
            break;
        }
        if b == c {
            count += 1;
        }
    }
    count
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
