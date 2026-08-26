// Translated from c_src/src/main.c.
// Reproduces byte-identical output for the same inputs.

use std::io::{self, Read, Write};

fn print_line(line: &str) {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(line.as_bytes());
    let _ = h.write_all(b"\n");
}

fn print_int_line(n: i32) {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = write!(h, "{}\n", n);
}

/// Emulates C's `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes from `reader`, stopping early if a `\n`
/// is read (which is included in the returned data) or EOF is reached.
/// Returns `None` if no bytes could be read before EOF (which mirrors
/// fgets returning NULL), otherwise returns the bytes that were read.
fn fgets_bytes<R: Read>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let max = size - 1;
    let mut out: Vec<u8> = Vec::with_capacity(max);
    let mut byte = [0u8; 1];
    while out.len() < max {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Emulates C's `atoi`. Skips leading whitespace, parses an optional sign,
/// then consumes digits until the first non-digit. Returns 0 if no digits.
fn c_atoi(bytes: &[u8]) -> i32 {
    let mut i = 0usize;
    // C's isspace: ' ', '\t', '\n', '\v', '\f', '\r'
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < bytes.len() {
        match bytes[i] {
            b'-' => {
                sign = -1;
                i += 1;
            }
            b'+' => {
                i += 1;
            }
            _ => {}
        }
    }
    let mut result: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    let signed = result.wrapping_mul(sign);
    // Mimic atoi behavior on out-of-range: clamp to i32 range matches glibc.
    if signed > i32::MAX as i64 {
        i32::MAX
    } else if signed < i32::MIN as i64 {
        i32::MIN
    } else {
        signed as i32
    }
}

fn bad<R: Read>(reader: &mut R) {
    let mut data: i32 = -1;
    {
        // char inputBuffer[14] = "";
        match fgets_bytes(reader, 14) {
            Some(buf) => {
                data = c_atoi(&buf);
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            // Reproduce C behavior exactly: out-of-bounds write would be UB
            // in C, but in Rust we must panic instead of silently corrupting
            // memory. Within bounds [0,9], behavior is identical.
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

fn good_g2b() {
    let mut data: i32 = -1;
    data = 7;
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

fn good_b2g<R: Read>(reader: &mut R) {
    let mut data: i32 = -1;
    {
        match fgets_bytes(reader, 14) {
            Some(buf) => {
                data = c_atoi(&buf);
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 && data < 10 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is out-of-bounds");
        }
    }
}

fn good<R: Read>(reader: &mut R) {
    good_g2b();
    good_b2g(reader);
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    print_line("Calling good()...");
    good(&mut handle);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut handle);
    print_line("Finished bad()");
}
