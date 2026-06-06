// Translation of c_src/src/main.c to Rust.
// Produces byte-identical output for the same inputs.

use std::io::{self, Read, Write};

fn print_line(line: &[u8]) {
    // Mirrors C's: printf("%s\n", line);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(line);
    let _ = out.write_all(b"\n");
}

/// Reads up to `max_size - 1` bytes from stdin, stopping after a newline
/// (which is included). Returns None on immediate EOF (matches C's fgets
/// returning NULL when no characters were read).
fn c_fgets(max_size: usize) -> Option<Vec<u8>> {
    if max_size <= 1 {
        return Some(Vec::new());
    }
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() < max_size - 1 {
        match handle.read(&mut byte) {
            Ok(0) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
        }
    }
    Some(buf)
}

/// Mimics C's atoi: optional leading whitespace, optional sign, then digits.
/// Stops at the first non-digit. Overflow uses wrapping arithmetic (C is UB).
fn c_atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip whitespace per isspace()
    while i < s.len() {
        let c = s[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut n: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i32;
        n = n.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    if neg { n.wrapping_neg() } else { n }
}

fn main() {
    // int data; data = -1;
    let mut data: i32 = -1;

    // First block: fgets into a 14-char buffer, then atoi.
    {
        match c_fgets(14) {
            Some(input_buffer) => {
                data = c_atoi(&input_buffer);
            }
            None => {
                print_line(b"fgets() failed.");
            }
        }
    }

    // Second block: build source/dest, conditionally copy and print.
    {
        // char source[100]; memset(source, 'A', 99); source[99] = '\0';
        // Logically: 99 'A's followed by a NUL terminator at index 99.
        let mut source = [0u8; 100];
        for b in &mut source[..99] {
            *b = b'A';
        }
        source[99] = 0;

        // char dest[100] = "";  (all zeros)
        let mut dest = [0u8; 100];

        if data < 100 {
            // strncpy(dest, source, data); dest[data] = '\0';
            // For safe Rust we only emulate the well-defined range 0..=99.
            // In C, negative `data` is reinterpreted as a huge size_t (UB).
            if data >= 0 {
                let n = data as usize;
                // strncpy stops at NUL in source (index 99 here) and pads with NULs.
                let copy_len = n.min(99);
                for i in 0..copy_len {
                    dest[i] = source[i];
                }
                // (Padding remainder with NULs is already done because dest was zeroed.)
                // Then: dest[data] = '\0';  (n is in 0..=99 here, so in-bounds)
                if n <= 99 {
                    dest[n] = 0;
                }
            }
            // else: undefined behavior in C; we leave dest as-is.
        }

        // printLine(dest) — print up to first NUL.
        let end = dest.iter().position(|&b| b == 0).unwrap_or(dest.len());
        print_line(&dest[..end]);
    }
}
