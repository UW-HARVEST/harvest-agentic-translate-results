use std::io::{self, ErrorKind, Read, Write};

fn foo(input: &[u8], needle: u8) -> i32 {
    input
        .iter()
        .take_while(|&&byte| byte != 0)
        .filter(|&&byte| byte == needle)
        .count() as i32
}

fn main() {
    let mut input = [0_u8; 1000];
    let mut bytes_read = 0;
    let mut stdin = io::stdin().lock();

    while bytes_read < input.len() {
        match stdin.read(&mut input[bytes_read..]) {
            Ok(0) => break,
            Ok(count) => bytes_read += count,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    let output = format!("A: {}\nx: {}\n", foo(&input, b'A'), foo(&input, b'x'));
    let _ = io::stdout().lock().write_all(output.as_bytes());
}
