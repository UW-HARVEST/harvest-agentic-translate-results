// Translated from c_src/src/main.c
//
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

/// Byte-oriented stdin reader with one byte of pushback, mimicking the way the
/// C library's `scanf` consumes characters (it reads across newlines and pushes
/// back the first character that does not belong to the conversion).
struct Scanner {
    stdin: io::Stdin,
    pushed: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            stdin: io::stdin(),
            pushed: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushed.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.stdin.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    fn push_back(&mut self, b: u8) {
        self.pushed = Some(b);
    }

    /// Emulates `scanf("%d", &out)`.
    ///
    /// Returns the number of successfully assigned items (1 on success,
    /// 0 on matching failure, -1 (EOF) if input ends before any conversion).
    /// `out` is left untouched unless a value is assigned, exactly like scanf.
    fn scan_int(&mut self, out: &mut i32) -> i32 {
        // Skip leading whitespace.
        let mut cur;
        loop {
            match self.next_byte() {
                Some(b) if is_space(b) => continue,
                Some(b) => {
                    cur = Some(b);
                    break;
                }
                None => return -1, // EOF before any input
            }
        }

        let mut negative = false;
        if let Some(b) = cur {
            if b == b'+' || b == b'-' {
                negative = b == b'-';
                cur = self.next_byte();
            }
        }

        let mut digits = 0usize;
        // Accumulate in i64 with saturation, then truncate to int, which is how
        // glibc behaves (strtol saturates at LONG_MAX/LONG_MIN, the result is
        // then stored into an int).
        let mut acc: i128 = 0;
        let saturated_lo: i128 = i64::MIN as i128;
        let saturated_hi: i128 = i64::MAX as i128;

        while let Some(b) = cur {
            if !b.is_ascii_digit() {
                break;
            }
            digits += 1;
            if acc <= saturated_hi {
                acc = acc * 10 + i128::from(b - b'0');
                if acc > saturated_hi + 1 {
                    acc = saturated_hi + 1;
                }
            }
            cur = self.next_byte();
        }

        // Push back the character that terminated the conversion.
        if let Some(b) = cur {
            self.push_back(b);
        }

        if digits == 0 {
            // Matching failure: nothing assigned.
            return 0;
        }

        let value: i128 = if negative { -acc } else { acc };
        let clamped: i64 = if value > saturated_hi {
            i64::MAX
        } else if value < saturated_lo {
            i64::MIN
        } else {
            value as i64
        };

        *out = clamped as i32; // truncating conversion, as in C
        1
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

#[allow(dead_code)]
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    // The C code allocates only 10 bytes but writes 10 ints (40 bytes),
    // overflowing the buffer. The values copied are all zeros and only
    // data[0] is read back, so the observable output is unchanged; it is
    // modeled here with a properly sized safe buffer.
    let mut data = vec![0i32; 10];
    {
        let source = [0i32; 10];
        for i in 0..10usize {
            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

fn good() {
    let mut data = vec![0i32; 10];
    {
        let source = [0i32; 10];
        for i in 0..10usize {
            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

fn main() {
    let mut scanner = Scanner::new();
    let mut x: i32 = 0;
    let _ = scanner.scan_int(&mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    io::stdout().flush().ok();
}
