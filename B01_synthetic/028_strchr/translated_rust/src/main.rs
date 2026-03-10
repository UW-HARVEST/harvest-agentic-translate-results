use std::io::Read;

fn foo(input: &[u8], c: u8) -> i32 {
    let mut res = 0i32;
    // strchr stops at the first null byte, so truncate at null
    let end = input.iter().position(|&b| b == 0).unwrap_or(input.len());
    let slice = &input[..end];
    let mut i = 0;
    while i < slice.len() {
        if let Some(pos) = slice[i..].iter().position(|&b| b == c) {
            res += 1;
            i += pos + 1;
        } else {
            break;
        }
    }
    res
}

fn driver(input: &[u8]) {
    print!("A: {}\n", foo(input, b'A'));
    print!("x: {}\n", foo(input, b'x'));
}

fn main() {
    let mut buf = [0u8; 1000];
    let _ = std::io::stdin().read(&mut buf);
    driver(&buf);
}
