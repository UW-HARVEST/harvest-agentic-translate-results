use std::io::{self, Read, Write};

fn foo(input: &[u8], c: u8) -> i32 {
    // Mirror C's strchr semantics: scan up to first NUL byte.
    let mut res: i32 = 0;
    for &b in input.iter() {
        if b == 0 {
            break;
        }
        if b == c {
            res = res.wrapping_add(1);
        }
    }
    res
}

fn driver(input: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "A: {}\n", foo(input, b'A'));
    let _ = write!(out, "x: {}\n", foo(input, b'x'));
}

fn main() {
    // Mirror C's `char in[1000] = "";` — a 1000-byte zero-initialized buffer.
    let mut buf = [0u8; 1000];
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // Mirror fread(in, 1, sizeof(in), stdin) — read up to 1000 bytes.
    let mut filled = 0usize;
    while filled < buf.len() {
        match handle.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(_) => break,
        }
    }

    driver(&buf);
}
