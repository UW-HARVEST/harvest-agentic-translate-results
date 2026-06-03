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

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

/// Mimics C's `fgets(buffer, 14, stdin)` semantics:
/// reads up to 13 bytes or until a newline (whichever comes first),
/// returning Some(string) on success, or None on EOF/error before any byte read.
fn fgets_like(max_bytes: usize) -> Option<String> {
    // The C code reads up to (size - 1) bytes, then null-terminates.
    let limit = max_bytes.saturating_sub(1);
    let mut buf: Vec<u8> = Vec::with_capacity(limit);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];

    loop {
        if buf.len() >= limit {
            break;
        }
        match handle.read(&mut byte) {
            Ok(0) => {
                // EOF
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

    // Convert bytes to string lossily (preserves behavior similar to atoi which
    // only inspects leading digits).
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Mimics C's `atoi`: parse leading optional whitespace, optional sign,
/// and digits. Returns 0 if no valid conversion.
fn atoi_like(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }

    let mut result: i32 = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i32;
        // Use wrapping arithmetic to mirror C's overflow behavior.
        result = result.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }

    result.wrapping_mul(sign)
}

fn flush_stdout() {
    let _ = io::stdout().flush();
}

fn bad() {
    let mut data: i32 = -1;
    {
        match fgets_like(14) {
            Some(input_buffer) => {
                data = atoi_like(&input_buffer);
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            // Mirrors the C vulnerability: out-of-bounds write if data >= 10.
            // We use unsafe pointer arithmetic to preserve the original
            // (vulnerable) semantics exactly.
            unsafe {
                let ptr = buffer.as_mut_ptr();
                *ptr.offset(data as isize) = 1;
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
    #[allow(unused_assignments)]
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

/// goodB2G uses the BadSource with the GoodSink
fn good_b2g() {
    let mut data: i32 = -1;
    {
        match fgets_like(14) {
            Some(input_buffer) => {
                data = atoi_like(&input_buffer);
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

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
    flush_stdout();
}
