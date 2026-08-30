// Rust translation of c_src/src/main.c
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

/// Mirror of the C `printLine`. Kept for structural fidelity with the original
/// translation unit; the C program never calls it.
#[allow(dead_code)]
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

/// `printf("%d\n", intNumber);`
fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

/// CWE-787-style undersized allocation: the C allocates only 10 *bytes* via
/// `alloca(10)` but then writes 10 `int`s (40 bytes) through the pointer.
///
/// This is a bug in the original C and is deliberately NOT fixed here. The
/// out-of-bounds writes have no effect on the program's observable output
/// (`source` is zero-initialized, so `data[0]` is 0 either way), so the
/// behavior is reproduced in safe Rust by using a buffer large enough for the
/// writes that the C actually performs.
fn bad() {
    // `alloca(10)` -- 10 bytes, i.e. room for only 2 full ints in the C.
    let mut data = [0i32; 10];
    {
        let source = [0i32; 10];
        for i in 0..10usize {
            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

/// Correctly sized allocation: `alloca(10 * sizeof(int))`.
fn good() {
    let mut data = [0i32; 10];
    {
        let source = [0i32; 10];
        for i in 0..10usize {
            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

/// Byte-at-a-time reader over stdin, so that the scanf emulation consumes
/// exactly the bytes `scanf` would consume (and no more), including reading
/// across newlines while skipping leading whitespace.
struct Stdin {
    inner: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn getc(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => Some(buf[0]),
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn ungetc(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Emulates `scanf("%d", &x)`: skip leading whitespace, then parse an
    /// optional sign followed by decimal digits. Returns the parsed value, or
    /// `None` on matching failure / EOF (in which case the C leaves `x`
    /// untouched). Overflow wraps the same way glibc's conversion truncates
    /// into an `int`.
    fn scan_i32(&mut self) -> Option<i32> {
        // Skip whitespace (space, \t, \n, \v, \f, \r) -- crosses newlines.
        let mut c = loop {
            match self.getc() {
                None => return None,
                Some(c) if c.is_ascii_whitespace() || c == 0x0b => continue,
                Some(c) => break c,
            }
        };

        let mut negative = false;
        if c == b'-' || c == b'+' {
            negative = c == b'-';
            match self.getc() {
                None => return None,
                Some(next) => c = next,
            }
        }

        if !c.is_ascii_digit() {
            // Matching failure: push the offending character back.
            self.ungetc(c);
            return None;
        }

        let mut value: i64 = 0;
        loop {
            value = value
                .wrapping_mul(10)
                .wrapping_add((c - b'0') as i64);
            match self.getc() {
                Some(next) if next.is_ascii_digit() => c = next,
                Some(next) => {
                    self.ungetc(next);
                    break;
                }
                None => break,
            }
        }

        if negative {
            value = value.wrapping_neg();
        }
        Some(value as i32)
    }
}

fn main() {
    let mut x: i32 = 0;

    let mut stdin = Stdin::new();
    if let Some(v) = stdin.scan_i32() {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = io::stdout().flush();
    std::process::exit(0);
}
