// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

/// Mimic C's atoi: parse a leading optional sign followed by digits;
/// return 0 if no valid conversion can be performed.
fn atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace (atoi behavior).
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    let mut sign: i32 = 1;
    if i < bytes.len() {
        match bytes[i] as char {
            '+' => {
                i += 1;
            }
            '-' => {
                sign = -1;
                i += 1;
            }
            _ => {}
        }
    }

    let mut result: i32 = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if !c.is_ascii_digit() {
            break;
        }
        let digit = (bytes[i] - b'0') as i32;
        // Use wrapping arithmetic to behave like C on overflow.
        result = result.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }

    result.wrapping_mul(sign)
}

/// Mimic C's fgets reading from stdin into a buffer of `size` bytes.
/// Reads at most size-1 bytes, stopping at newline (which is included).
/// Returns Some(string) on success, None on immediate EOF/error.
fn fgets_stdin(size: usize) -> Option<String> {
    if size == 0 {
        return None;
    }
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let max = size - 1;
    let mut buf: Vec<u8> = Vec::with_capacity(max);
    let mut byte = [0u8; 1];

    loop {
        if buf.len() >= max {
            break;
        }
        match handle.read(&mut byte) {
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
        return None;
    }

    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn main() {
    // Ensure stdout is flushed at exit.
    let _ = io::stdout().flush();

    let mut data: i32 = -1;

    // Mirror the C block: char inputBuffer[14] = "";
    {
        match fgets_stdin(14) {
            Some(input_buffer) => {
                // Convert to int
                data = atoi(&input_buffer);
            }
            None => {
                print_line(Some("fgets() failed."));
            }
        }
    }

    // Mirror the C block:
    //   char source[100];
    //   char dest[100] = "";
    //   memset(source, 'A', 100-1);
    //   source[100-1] = '\0';
    //   if (data < 100) {
    //       strncpy(dest, source, data);
    //       dest[data] = '\0';
    //   }
    //   printLine(dest);
    {
        // source is 99 'A' characters terminated by '\0' (we model it as &str).
        let source: String = std::iter::repeat('A').take(99).collect();
        let mut dest = String::new();

        if data < 100 {
            // In the original C, a negative `data` is reinterpreted as a huge
            // size_t and triggers a buffer overflow (the CWE this example
            // illustrates). In safe Rust we copy 0 bytes when data is
            // negative. For 0 <= data <= 99, copy that many bytes from source.
            if data >= 0 {
                let n = std::cmp::min(data as usize, source.len());
                dest.push_str(&source[..n]);
            }
            // Else: leave dest empty (no UB).
            // The C code unconditionally writes dest[data] = '\0', but in
            // Rust we already have a properly terminated owned String.
        }

        print_line(Some(&dest));
    }

    std::process::exit(0);
}
