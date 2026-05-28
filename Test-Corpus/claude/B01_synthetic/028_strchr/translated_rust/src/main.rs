use std::io::{self, Read, Write};

fn foo(input: &[u8], c: u8) -> i32 {
    // Count occurrences of `c` up to the first null byte (mimics strchr behavior
    // on a null-terminated C string).
    let mut res: i32 = 0;
    for &b in input.iter() {
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
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    // Match C's printf("A: %d\n", ...) and printf("x: %d\n", ...).
    write!(handle, "A: {}\n", foo(input, b'A')).unwrap();
    write!(handle, "x: {}\n", foo(input, b'x')).unwrap();
}

fn main() {
    // char in[1000] = ""; -> 1000-byte buffer initialized to zeros.
    let mut buf = [0u8; 1000];

    // fread(in, 1, sizeof(in), stdin); -> read up to 1000 bytes from stdin.
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut total = 0usize;
    while total < buf.len() {
        match handle.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    driver(&buf);
}
