use std::io::{self, BufRead, Write};

/// Equivalent to C's `strcspn`: returns the length of the initial segment of
/// `s1` which consists entirely of bytes not in `s2`.
fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    for (i, b) in s1.iter().enumerate() {
        if s2.contains(b) {
            return i;
        }
    }
    s1.len()
}

fn driver(s1: &[u8], s2: &[u8]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{}", strcspn(s1, s2)).unwrap();
}

/// Read up to (capacity - 1) bytes (plus a terminating newline if encountered)
/// from `reader`, mirroring C's `fgets(buf, capacity, stdin)`.
fn fgets<R: BufRead>(reader: &mut R, capacity: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    let max_read = capacity.saturating_sub(1);
    let mut byte = [0u8; 1];
    while buf.len() < max_read {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    let mut s1 = fgets(&mut handle, 100);
    let mut s2 = fgets(&mut handle, 100);

    // Mirror the C code's behavior: replace the last character (typically '\n')
    // with a NUL terminator. In Rust we simply pop the trailing byte.
    if !s1.is_empty() {
        s1.pop();
    }
    if !s2.is_empty() {
        s2.pop();
    }

    driver(&s1, &s2);
}
