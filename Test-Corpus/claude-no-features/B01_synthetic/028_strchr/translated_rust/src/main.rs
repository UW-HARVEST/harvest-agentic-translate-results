use std::io::{self, Read};

fn foo(input: &[u8], c: u8) -> i32 {
    // Mimic strchr: scan from start, stop at first null byte (unwritten buffer is zeroed)
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
    // Mirror C: char in[1000] = ""; fread(in, 1, sizeof(in), stdin);
    let mut buf: [u8; 1000] = [0; 1000];

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut total_read = 0usize;
    while total_read < buf.len() {
        match handle.read(&mut buf[total_read..]) {
            Ok(0) => break,
            Ok(n) => total_read += n,
            Err(_) => break,
        }
    }

    driver(&buf);
}
