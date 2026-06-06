use std::io::{self, Read};

/// Mimic C's fgets: read up to `max - 1` bytes from `input`, stopping after a
/// newline (which is included in the result) or at EOF. The buffer is filled
/// with the bytes read; no NUL terminator is appended (Rust's Vec<u8> is
/// length-tracked).
fn fgets<R: Read>(buf: &mut Vec<u8>, max: usize, input: &mut R) {
    buf.clear();
    if max == 0 {
        return;
    }
    let mut byte = [0u8; 1];
    while buf.len() < max - 1 {
        match input.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Mimic C's strcspn: returns the length of the maximal initial segment of
/// `s1` that contains no bytes from `s2`.
fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    for (i, &c) in s1.iter().enumerate() {
        if s2.contains(&c) {
            return i;
        }
    }
    s1.len()
}

fn driver(s1: &[u8], s2: &[u8]) {
    // C: printf("%zu\n", strcspn(s1, s2));
    println!("{}", strcspn(s1, s2));
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    let mut s1: Vec<u8> = Vec::new();
    let mut s2: Vec<u8> = Vec::new();

    fgets(&mut s1, 100, &mut handle);
    fgets(&mut s2, 100, &mut handle);

    // Mimic: s1[strlen(s1)-1] = '\0';
    // s2[strlen(s2)-1] = '\0';
    // i.e. drop the trailing byte (typically the newline). If the buffer is
    // empty, the original C would invoke undefined behavior; we conservatively
    // do nothing in that case to avoid panicking.
    if !s1.is_empty() {
        s1.pop();
    }
    if !s2.is_empty() {
        s2.pop();
    }

    driver(&s1, &s2);
}
