use std::io::{self, Read, Write};

/// Mimic C's fgets: reads up to capacity-1 bytes, stops on newline (kept in buffer),
/// or EOF. Returns the number of bytes read (excluding the C-style NUL terminator).
/// If nothing was read (EOF immediately), returns 0 and the buffer is left as-is.
fn fgets(stdin: &mut impl Read, buf: &mut Vec<u8>, capacity: usize) -> usize {
    let max = capacity.saturating_sub(1);
    let mut byte = [0u8; 1];
    let mut count = 0usize;
    while count < max {
        match stdin.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                count += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    count
}

fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    // Length of the initial segment of s1 consisting of bytes not in s2.
    for (i, &c) in s1.iter().enumerate() {
        if s2.contains(&c) {
            return i;
        }
    }
    s1.len()
}

fn driver(s1: &[u8], s2: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Match printf("%zu\n", ...)
    let _ = write!(out, "{}\n", strcspn(s1, s2));
}

fn main() {
    // C: char s1[100] = "", s2[100] = "";
    // We track the contents up to the (eventual) NUL terminator as a Vec<u8>.
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    let mut s1: Vec<u8> = Vec::new();
    let mut s2: Vec<u8> = Vec::new();

    // fgets(s1, 100, stdin); fgets(s2, 100, stdin);
    fgets(&mut handle, &mut s1, 100);
    fgets(&mut handle, &mut s2, 100);

    // s1[strlen(s1)-1] = '\0';  -> drop the last byte (typically the newline).
    // If the buffer is empty, the C code is undefined behavior; we leave it empty.
    if !s1.is_empty() {
        s1.pop();
    }
    if !s2.is_empty() {
        s2.pop();
    }

    driver(&s1, &s2);
}
