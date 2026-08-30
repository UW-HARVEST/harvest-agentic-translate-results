use std::io::{self, Read, Write};

fn foo(input: &[u8], needle: u8) -> i32 {
    input.iter().filter(|&&byte| byte == needle).count() as i32
}

fn driver(input: &[u8]) {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let _ = writeln!(output, "A: {}", foo(input, b'A'));
    let _ = writeln!(output, "x: {}", foo(input, b'x'));
}

fn main() {
    let mut input = Vec::with_capacity(1000);
    let _ = io::stdin().take(1000).read_to_end(&mut input);
    let string_end = input.iter().position(|&byte| byte == 0).unwrap_or(input.len());
    driver(&input[..string_end]);
}
