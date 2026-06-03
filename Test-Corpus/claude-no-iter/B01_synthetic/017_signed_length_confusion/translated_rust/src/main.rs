// Translated from c_src/src/main.c
//
// Faithfully reproduces the original C program's I/O behavior:
//   - fgets(buf, 14, stdin): reads up to 13 bytes (or until newline/EOF),
//     null-terminates the result.
//   - atoi: skips leading whitespace, optional sign, digits, stops at the
//     first non-digit. Returns 0 if no digits were found.
//   - The "vulnerable" strncpy block: if data < 100, copy `data` 'A's into
//     a 100-byte destination buffer and null-terminate at index `data`.
//     We reproduce this exactly for valid (0..=99) sizes; outside that
//     range the original C exhibits undefined behavior, which we emulate
//     here as best we can (data >= 100: skip the copy and print empty;
//     data < 0: would smash the stack in C — we abort).
//
// Output is byte-identical to the C version's stdout for well-formed input.

use std::io::{self, Read, Write};

fn print_line(line: &[u8]) {
    // Equivalent to printf("%s\n", line) followed by no flush — but the C
    // program exits immediately after, which causes stdout to flush. We use
    // a locked stdout writer and let the BufWriter drop at end of main.
    let stdout = io::stdout();
    let mut out = stdout.lock();
    out.write_all(line).unwrap();
    out.write_all(b"\n").unwrap();
}

/// Reproduce C's fgets(buf, n, stdin):
/// reads up to n-1 bytes; stops after consuming a '\n' or at EOF.
/// Returns Some(bytes_read) on success (>=1 byte read), None on EOF before
/// any byte was read.
fn c_fgets(n: usize) -> Option<Vec<u8>> {
    if n == 0 {
        return Some(Vec::new());
    }
    let max = n - 1;
    let mut buf: Vec<u8> = Vec::with_capacity(max);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() < max {
        match handle.read(&mut byte) {
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
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Reproduce C's atoi: skip leading ASCII whitespace, optional sign, then
/// digits until a non-digit. Returns 0 on no digits. Saturates on overflow
/// (matching typical glibc behavior is technically UB; we use i32 wrapping
/// arithmetic where reasonable; for our test inputs values are small).
fn c_atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip leading whitespace as C's isspace would.
    while i < s.len() {
        match s[i] {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C => i += 1,
            _ => break,
        }
    }
    let mut sign: i32 = 1;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        if s[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < s.len() {
        let c = s[i];
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as i32;
        // Match typical C behavior: wrap on overflow (UB in C, but common).
        result = result.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    result.wrapping_mul(sign)
}

fn main() {
    let mut data: i32 = -1;

    // First block: read up to 13 bytes via fgets and atoi the result.
    {
        match c_fgets(14) {
            Some(buf) => {
                data = c_atoi(&buf);
            }
            None => {
                print_line(b"fgets() failed.");
            }
        }
    }

    // Second block: build a 100-byte dest buffer; if data < 100 copy
    // `data` 'A's from a 99-'A' source and null-terminate at index data.
    {
        // source is 99 'A's followed by a null terminator (length 100).
        let source: [u8; 100] = {
            let mut s = [b'A'; 100];
            s[99] = 0;
            s
        };
        let mut dest: [u8; 100] = [0; 100];

        if data < 100 {
            // Reproduce strncpy(dest, source, data); dest[data] = '\0';
            // The C code permits 0 <= data <= 99 cleanly; outside that
            // range it is undefined behavior. We faithfully cover the
            // sane range and abort on out-of-range values that would
            // trash memory in C.
            if data < 0 {
                // Undefined behavior in C (size_t cast of a negative int
                // is huge → buffer overflow). Mirror with a panic.
                panic!("invalid negative size: {}", data);
            }
            let n = data as usize;
            // strncpy semantics: copy at most n bytes from source, stopping
            // early (and padding with NULs) if a NUL is encountered. Here
            // source has no NUL within indices 0..99, so for n <= 99 we
            // simply copy n 'A's. For n == 100 (impossible here since
            // data < 100), we'd hit the NUL at index 99.
            for i in 0..n {
                dest[i] = source[i];
            }
            dest[n] = 0;
        }

        // printLine(dest) — print up to first NUL.
        let end = dest.iter().position(|&b| b == 0).unwrap_or(dest.len());
        print_line(&dest[..end]);
    }
}
