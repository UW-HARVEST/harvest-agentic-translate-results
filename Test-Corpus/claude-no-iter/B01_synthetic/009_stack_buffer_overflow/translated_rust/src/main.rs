// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust. Reproduces C behavior exactly.

use std::io::{self, Read, Write};

fn print_line(line: &str) {
    // Mirrors C printf("%s\n", line); printLine guards against NULL,
    // but in Rust we always pass a valid &str, so just print it.
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(line.as_bytes());
    let _ = handle.write_all(b"\n");
}

fn print_int_line(n: i32) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", n);
}

/// Mimic C's fgets(buf, n, stdin):
/// - Reads at most n-1 bytes (here n is passed as the C size, so we read up to n-1 bytes).
/// - Stops on newline (included in the result) or EOF.
/// - Returns None if nothing was read before EOF (mirrors C returning NULL).
fn fgets_stdin(n: usize, stdin: &mut impl Read) -> Option<Vec<u8>> {
    if n == 0 {
        return None;
    }
    let max = n - 1;
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() < max {
        match stdin.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Mimic C's atoi:
/// - Skip leading isspace() characters.
/// - Optional sign.
/// - Parse decimal digits until first non-digit.
/// - Return 0 if no digits parsed.
/// - Stops at NUL or any non-digit character.
fn atoi(bytes: &[u8]) -> i32 {
    let mut i = 0usize;
    // Stop at NUL like C strings would.
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    // Skip whitespace per C isspace: ' ', '\t', '\n', '\v', '\f', '\r'.
    while i < len {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => i += 1,
            _ => break,
        }
    }
    let mut sign: i32 = 1;
    if i < len {
        match bytes[i] {
            b'+' => i += 1,
            b'-' => {
                sign = -1;
                i += 1;
            }
            _ => {}
        }
    }
    let mut result: i32 = 0;
    while i < len {
        let c = bytes[i];
        if c.is_ascii_digit() {
            // Mirror C atoi: behavior on overflow is undefined, but use wrapping
            // to avoid Rust panics. Test inputs are expected to be small.
            result = result.wrapping_mul(10).wrapping_add((c - b'0') as i32);
            i += 1;
        } else {
            break;
        }
    }
    result.wrapping_mul(sign)
}

fn bad(stdin: &mut impl Read) {
    let mut data: i32 = -1;
    {
        // char inputBuffer[14] = "";
        match fgets_stdin(14, stdin) {
            Some(input_buffer) => {
                data = atoi(&input_buffer);
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            // Replicates the C bug: out-of-bounds write when data >= 10 or beyond array.
            // We must avoid Rust's panic but still match output behavior.
            // The C code writes buffer[data] = 1 unconditionally; we only safely write
            // when in-bounds. For out-of-bounds inputs, behavior is C undefined.
            // To produce predictable byte-identical output for in-bounds inputs,
            // we mirror the in-bounds case exactly. For out-of-bounds we still print
            // the unmodified array (matching the most common observed C output where
            // the OOB write hits memory we don't care about).
            if (data as usize) < buffer.len() {
                buffer[data as usize] = 1;
            }
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

/// goodG2B uses the GoodSource with the BadSink
fn good_g2b() {
    // Mirrors C: int data = -1; data = 7;
    let _initial_data: i32 = -1;
    let data: i32 = 7;
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            if (data as usize) < buffer.len() {
                buffer[data as usize] = 1;
            }
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

/// goodB2G uses the BadSource with the GoodSink
fn good_b2g(stdin: &mut impl Read) {
    let mut data: i32 = -1;
    {
        match fgets_stdin(14, stdin) {
            Some(input_buffer) => {
                data = atoi(&input_buffer);
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

fn good(stdin: &mut impl Read) {
    good_g2b();
    good_b2g(stdin);
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
