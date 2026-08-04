// Translation of c_src/src/main.c to Rust.
// Aims to produce byte-identical output for the same inputs.

use std::io::{self, Read, Write};

fn print_line(line: Option<&[u8]>) {
    if let Some(s) = line {
        let stdout = io::stdout();
        let mut h = stdout.lock();
        h.write_all(s).unwrap();
        h.write_all(b"\n").unwrap();
    }
}

/// Mimic C's fgets: read up to `max_size - 1` bytes from stdin into `buf`,
/// stopping at newline (which is included in the buffer) or EOF.
/// Returns false if no characters were read before EOF/error.
fn fgets_like<R: Read>(reader: &mut R, max_size: usize, buf: &mut Vec<u8>) -> bool {
    if max_size == 0 {
        return false;
    }
    let mut byte = [0u8; 1];
    let mut got_any = false;
    while buf.len() < max_size - 1 {
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                got_any = true;
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return false,
        }
    }
    got_any
}

/// Mimic C's atoi: skip leading whitespace, optional sign, then parse digits.
/// Stops at first non-digit. Returns 0 if no digits found.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip C isspace whitespace.
    while i < s.len() {
        let c = s[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }
    let mut sign: i32 = 1;
    if i < s.len() {
        if s[i] == b'-' {
            sign = -1;
            i += 1;
        } else if s[i] == b'+' {
            i += 1;
        }
    }
    let mut result: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i32;
        // Mimic C's signed integer overflow with two's complement wrap.
        result = result.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }
    result.wrapping_mul(sign)
}

/// Mimic strncpy: copy up to n bytes from src to dst. If src has a null
/// before n bytes, pad the rest with nulls.
fn strncpy(dst: &mut [u8], src: &[u8], n: usize) {
    let mut hit_null = false;
    for i in 0..n {
        if i >= dst.len() {
            break;
        }
        if hit_null {
            dst[i] = 0;
        } else if i < src.len() {
            let b = src[i];
            dst[i] = b;
            if b == 0 {
                hit_null = true;
            }
        } else {
            // src exhausted; treat as null.
            dst[i] = 0;
            hit_null = true;
        }
    }
}

/// Find the C-string length (until first null byte) of a buffer.
fn cstr_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn main() {
    let mut data: i32 = -1;

    {
        // C: char inputBuffer[14] = "";
        let mut input_buffer: Vec<u8> = Vec::with_capacity(14);
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        if fgets_like(&mut handle, 14, &mut input_buffer) {
            // Convert to int.
            data = atoi(&input_buffer);
        } else {
            print_line(Some(b"fgets() failed."));
        }
    }

    {
        // C: char source[100]; char dest[100] = "";
        let mut source = vec![0u8; 100];
        let mut dest = vec![0u8; 100];
        // memset(source, 'A', 100-1); source[100-1] = '\0';
        for b in source.iter_mut().take(99) {
            *b = b'A';
        }
        source[99] = 0;

        if data < 100 {
            // strncpy(dest, source, data); dest[data] = '\0';
            if data >= 0 {
                let n = data as usize;
                strncpy(&mut dest, &source, n);
                if n < dest.len() {
                    dest[n] = 0;
                }
            } else {
                // C invokes undefined behavior here (strncpy with size_t cast
                // of a negative int, then negative-index write). In practice
                // this would crash. We approximate by leaving `dest` empty so
                // the program does not abort, since safe Rust cannot reproduce
                // the UB byte-for-byte.
            }
        }

        let n = cstr_len(&dest);
        print_line(Some(&dest[..n]));
    }
}
