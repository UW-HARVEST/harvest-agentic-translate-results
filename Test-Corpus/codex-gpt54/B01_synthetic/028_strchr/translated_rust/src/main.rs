use std::io::{self, Read, Write};

fn foo(input: &[u8], needle: u8) -> i32 {
    let end = input.iter().position(|&b| b == 0).unwrap_or(input.len());
    let mut res = 0;
    let mut start = 0;

    while start < end {
        match input[start..end].iter().position(|&b| b == needle) {
            Some(pos) => {
                res += 1;
                start += pos + 1;
            }
            None => break,
        }
    }

    res
}

fn driver(input: &[u8]) {
    let mut stdout = io::stdout().lock();
    write!(stdout, "A: {}\n", foo(input, b'A')).unwrap();
    write!(stdout, "x: {}\n", foo(input, b'x')).unwrap();
}

fn main() {
    let mut input = [0u8; 1001];
    let _ = io::stdin().lock().read(&mut input[..1000]);
    driver(&input[..]);
}
