// Translation of c_src/src/main.c to Rust.
// Goal: byte-identical stdout for the same inputs.

use std::io::{self, BufWriter, Read, Write};

/// Mimic C's fgets(buf, buf_size, stdin):
/// Reads up to buf_size - 1 bytes from stdin, stopping after a newline (inclusive)
/// or at EOF. Returns None if EOF is reached before any byte is read (i.e. NULL).
fn fgets(buf_size: usize) -> Option<Vec<u8>> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut result: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while result.len() < buf_size.saturating_sub(1) {
        match handle.read(&mut byte) {
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
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Mimic C's atoi: skip leading whitespace, optional +/-, parse decimal digits
/// until a non-digit is encountered. Returns 0 if no digits are parsed.
/// Uses wrapping arithmetic to match typical C behavior on overflow.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip whitespace as defined by C isspace() in the "C" locale.
    while i < s.len()
        && matches!(
            s[i],
            b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r'
        )
    {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut result: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((s[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        result.wrapping_neg()
    } else {
        result
    }
}

fn print_line<W: Write>(out: &mut W, line: &[u8]) {
    // The C code only prints when the pointer is non-NULL; we always have a slice here.
    out.write_all(line).unwrap();
    out.write_all(b"\n").unwrap();
}

fn main() {
    let stdout = io::stdout();
    // Use a BufWriter to mimic C's block-buffered stdout when piped,
    // so that on a "segfault" we can simply abandon the buffer and produce no output.
    let mut out = BufWriter::new(stdout.lock());

    let mut data: i32 = -1;

    {
        // char inputBuffer[14]
        match fgets(14) {
            Some(buf) => {
                data = atoi(&buf);
            }
            None => {
                print_line(&mut out, b"fgets() failed.");
            }
        }
    }

    // Replicate C UB: strncpy(dest, source, data) with data < 0 segfaults
    // because data is converted to a huge size_t. The buffered stdout is
    // never flushed, so any prior output is lost.
    if data < 0 {
        // Drop the BufWriter without flushing to match the unflushed-buffer behavior.
        std::mem::forget(out);
        std::process::exit(139);
    }

    {
        // char source[100]; memset(source, 'A', 99); source[99] = '\0';
        // char dest[100] = "";
        // if (data < 100) { strncpy(dest, source, data); dest[data] = '\0'; }
        // printLine(dest);
        let mut dest: Vec<u8> = Vec::new();
        if data < 100 {
            // For 0 <= data < 100, this copies `data` 'A's (no NUL in the first 99 source bytes).
            for _ in 0..data {
                dest.push(b'A');
            }
        }
        // If data >= 100, dest stays empty (the zero-initialized array's first byte is '\0').
        print_line(&mut out, &dest);
    }

    out.flush().unwrap();
}
