use std::io::{self, Read, Write};

/// Mimics C fgets: reads up to size-1 bytes from `input` into `buf` starting at offset 0.
/// Reading stops on newline (which is included) or EOF.
/// On success, a trailing 0 byte is written (we represent the "string" as the bytes
/// before the first 0 byte).
/// Returns the number of bytes written into `buf` (excluding the null terminator).
/// If nothing was read (immediate EOF), buf is left untouched and 0 is returned.
fn fgets(buf: &mut [u8], input: &mut dyn Read) -> usize {
    if buf.len() < 2 {
        return 0;
    }
    let max = buf.len() - 1;
    let mut written = 0usize;
    let mut byte = [0u8; 1];
    while written < max {
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf[written] = byte[0];
                written += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if written > 0 {
        buf[written] = 0;
    }
    written
}

fn strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    // Initial segment of s1 (up to first 0) that contains no character from s2 (up to first 0).
    let s1_len = strlen(s1);
    let s2_len = strlen(s2);
    let s2_set = &s2[..s2_len];
    let mut count = 0usize;
    while count < s1_len {
        let c = s1[count];
        if s2_set.contains(&c) {
            break;
        }
        count += 1;
    }
    count
}

fn driver(s1: &[u8], s2: &[u8], out: &mut dyn Write) {
    let n = strcspn(s1, s2);
    write!(out, "{}\n", n).unwrap();
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut s1 = [0u8; 100];
    let mut s2 = [0u8; 100];

    // fgets(s1, sizeof(s1), stdin);
    let _ = fgets(&mut s1, &mut handle);
    // fgets(s2, sizeof(s1), stdin);  -- note: sizeof(s1) is used but s2 is also size 100
    let _ = fgets(&mut s2, &mut handle);

    // s1[strlen(s1)-1] = '\0';
    // Reproduce C behavior: if strlen is 0, this writes to index (usize)-1 which is UB in C.
    // We'll only zero the last character if strlen > 0; otherwise leave alone (any choice
    // here is undefined but matches a common compiler outcome).
    let l1 = strlen(&s1);
    if l1 > 0 {
        s1[l1 - 1] = 0;
    }
    let l2 = strlen(&s2);
    if l2 > 0 {
        s2[l2 - 1] = 0;
    }

    driver(&s1, &s2, &mut out);
}
