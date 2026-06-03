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

const CHAR_ARRAY_SIZE: usize = 20;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

/// Mimics C's fgets: reads up to size-1 bytes from stdin into a buffer,
/// stopping at newline (which is included) or EOF. Returns Some(string) on
/// success, None if no characters were read (EOF before any data).
fn fgets_like(size: usize) -> Option<String> {
    if size == 0 {
        return None;
    }
    let mut buf = Vec::with_capacity(size - 1);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() < size - 1 {
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
        Some(String::from_utf8_lossy(&buf).into_owned())
    }
}

/// Mimics C's atof: parses a leading floating-point number from the string.
/// Returns 0.0 if no conversion can be performed.
fn atof_like(s: &str) -> f64 {
    let trimmed = s.trim_start();
    let bytes = trimmed.as_bytes();
    let mut i = 0;

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let mut has_digits = false;

    // Integer part
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        has_digits = true;
        i += 1;
    }

    // Fractional part
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            has_digits = true;
            i += 1;
        }
    }

    if !has_digits {
        return 0.0;
    }
    let mut end = i;

    // Exponent part
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            end = j;
        }
    }

    trimmed[..end].parse::<f64>().unwrap_or(0.0)
}

fn bad() {
    let mut data: f32 = 0.0;
    {
        match fgets_like(CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = atof_like(&input_buffer) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let result = (100.0 / data as f64) as i32;
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = (100.0 / data as f64) as i32;
    print_int_line(result);
}

fn good_b2g() {
    let mut data: f32 = 0.0;
    {
        match fgets_like(CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = atof_like(&input_buffer) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = (100.0 / data as f64) as i32;
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
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
    // Ensure output is flushed before exit.
    let _ = io::stdout().flush();
}
