use std::io::{self, Read};

fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    for (i, &b) in s1.iter().enumerate() {
        if s2.contains(&b) {
            return i;
        }
    }
    s1.len()
}

fn driver(s1: &[u8], s2: &[u8]) {
    println!("{}", strcspn(s1, s2));
}

/// Emulate C fgets: read up to `max_len - 1` bytes, stopping after '\n' or at EOF.
/// Returns the bytes read (including '\n' if present).
fn fgets(input: &mut impl Read, max_len: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    let limit = max_len - 1; // fgets reads at most size-1 chars
    for _ in 0..limit {
        let mut byte = [0u8; 1];
        if input.read(&mut byte).unwrap_or(0) == 0 {
            break;
        }
        buf.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    buf
}

fn main() {
    let mut stdin = io::stdin().lock();

    let mut s1 = fgets(&mut stdin, 100);
    let mut s2 = fgets(&mut stdin, 100);

    // C code: s1[strlen(s1)-1] = '\0'  — strips last byte unconditionally
    if !s1.is_empty() {
        s1.pop();
    }
    if !s2.is_empty() {
        s2.pop();
    }

    driver(&s1, &s2);
}
