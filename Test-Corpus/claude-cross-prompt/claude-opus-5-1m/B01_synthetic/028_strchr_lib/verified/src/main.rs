use std::io::Read;

fn foo(input: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    for &b in input {
        if b == 0 {
            break;
        }
        if b == c {
            res += 1;
        }
    }
    res
}

fn driver(input: &[u8]) {
    println!("A: {}", foo(input, b'A'));
    println!("x: {}", foo(input, b'x'));
}

fn main() {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).ok();
    // Ensure null termination behavior matches a C string
    if !buf.contains(&0) {
        buf.push(0);
    }
    driver(&buf);
}
