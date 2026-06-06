use std::io::{self, Read};

/// Mimic C's fgets: read bytes from stdin up to (buf_size - 1) characters,
/// stopping early at and including a newline byte. Returns the bytes read
/// (without an appended null terminator). An empty return means EOF before
/// any bytes were read (equivalent to fgets returning NULL).
fn fgets_like<R: Read>(reader: &mut R, buf_size: usize) -> Vec<u8> {
    let max_chars = buf_size.saturating_sub(1);
    let mut result = Vec::new();
    let mut byte = [0u8; 1];
    while result.len() < max_chars {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                result.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    result
}

/// C strcspn: returns length of the initial segment of s1 that contains
/// no characters from s2.
fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    for (i, c) in s1.iter().enumerate() {
        if s2.contains(c) {
            return i;
        }
    }
    s1.len()
}

fn driver(s1: &[u8], s2: &[u8]) {
    println!("{}", strcspn(s1, s2));
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // C declares: char s1[100] = "", s2[100] = "";
    // Then fgets(s1, sizeof(s1), stdin); fgets(s2, sizeof(s1), stdin);
    // sizeof(s1) is 100 in both calls.
    let mut s1 = fgets_like(&mut handle, 100);
    let mut s2 = fgets_like(&mut handle, 100);

    // C: s1[strlen(s1)-1] = '\0';  -- removes the last byte (typically the
    // trailing newline). When the buffer is non-empty we pop the last byte;
    // when empty this would be undefined behavior in C, but we leave it empty.
    if !s1.is_empty() {
        s1.pop();
    }
    if !s2.is_empty() {
        s2.pop();
    }

    driver(&s1, &s2);
}
