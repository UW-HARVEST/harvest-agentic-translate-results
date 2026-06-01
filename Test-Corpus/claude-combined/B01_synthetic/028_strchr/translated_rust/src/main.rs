use std::io::{self, Read, Write};

fn foo(input: &[u8], c: u8) -> i32 {
    // Mimic strchr behavior: search a NUL-terminated string for `c`.
    // strchr stops at the first NUL byte. If c == 0, it would match the NUL.
    let mut res: i32 = 0;
    let mut i: usize = 0;
    loop {
        // Find next occurrence of c starting at i, up to (and including) the NUL terminator.
        let mut found: Option<usize> = None;
        let mut j = i;
        loop {
            if j >= input.len() {
                break;
            }
            let b = input[j];
            if b == c {
                found = Some(j);
                break;
            }
            if b == 0 {
                // NUL terminator: strchr returns NULL (unless c == 0, handled above)
                break;
            }
            j += 1;
        }
        match found {
            Some(pos) => {
                res += 1;
                i = pos + 1;
            }
            None => return res,
        }
    }
}

fn driver(input: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "A: {}\n", foo(input, b'A'));
    let _ = write!(out, "x: {}\n", foo(input, b'x'));
}

fn main() {
    // Equivalent to: char in[1000] = ""; fread(in, 1, sizeof(in), stdin);
    let mut buf = [0u8; 1000];
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // fread reads up to N bytes; we replicate by reading until full or EOF.
    let mut total = 0usize;
    while total < buf.len() {
        match handle.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
    }

    driver(&buf);
}
