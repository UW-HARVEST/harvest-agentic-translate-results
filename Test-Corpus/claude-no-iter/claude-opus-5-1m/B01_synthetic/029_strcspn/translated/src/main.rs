// Rust translation of c_src/src/main.c
// Reads two lines from stdin (fgets-style, max 99 bytes each),
// strips the last byte of each, and prints strcspn(s1, s2).

use std::io::{self, Read, Write};

/// Mimics C's fgets(buf, max, stdin): reads up to `max - 1` bytes or until a
/// newline (inclusive), whichever comes first. Stops on EOF. The bytes read
/// are appended to `buf`. We do not store an explicit NUL terminator; the
/// length of `buf` plays that role.
fn fgets_like<R: Read>(reader: &mut R, buf: &mut Vec<u8>, max: usize) {
    let limit = max.saturating_sub(1);
    let mut byte = [0u8; 1];
    while buf.len() < limit {
        match reader.read(&mut byte) {
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

/// Returns the length of the longest initial prefix of `s1` that contains no
/// byte from `s2`. In C, both strings are NUL-terminated; we treat them as
/// already-trimmed byte slices and additionally stop at any embedded NUL byte
/// to mirror C's strcspn semantics.
fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    // Truncate at first embedded NUL (C string semantics).
    let s1 = match s1.iter().position(|&b| b == 0) {
        Some(i) => &s1[..i],
        None => s1,
    };
    let s2_end = s2.iter().position(|&b| b == 0).unwrap_or(s2.len());
    let s2 = &s2[..s2_end];

    for (i, &b) in s1.iter().enumerate() {
        if s2.contains(&b) {
            return i;
        }
    }
    s1.len()
}

fn driver(s1: &[u8], s2: &[u8]) {
    // C: printf("%zu\n", strcspn(s1, s2));
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", strcspn(s1, s2));
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // Mirror C's `char s1[100] = "", s2[100] = "";` plus two fgets calls.
    // Note: the C source passes `sizeof(s1)` to *both* fgets calls, so both
    // buffers use the same 100-byte limit.
    let mut s1: Vec<u8> = Vec::new();
    let mut s2: Vec<u8> = Vec::new();
    fgets_like(&mut handle, &mut s1, 100);
    fgets_like(&mut handle, &mut s2, 100);

    // C: s1[strlen(s1)-1] = '\0';
    //    s2[strlen(s2)-1] = '\0';
    // Strip the last byte (typically the trailing newline). If the string is
    // empty, the C code invokes undefined behavior; we simply do nothing in
    // that case to remain well-defined while matching the common path.
    if !s1.is_empty() {
        s1.pop();
    }
    if !s2.is_empty() {
        s2.pop();
    }

    driver(&s1, &s2);
}
