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

/// `char` on the reference platform (x86-64 Linux) is signed.
const CHAR_MAX: i8 = i8::MAX;

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        print!("{}\n", line);
    }
}

fn print_hex_char_line(char_hex: i8) {
    // printf("%02x\n", charHex);
    // The char argument is promoted to int, then reinterpreted as unsigned int
    // by the %x conversion (so negative values print as 8 hex digits).
    print!("{:02x}\n", (char_hex as i32) as u32);
}

fn bad() {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        // char result = data * 2;  (int arithmetic, truncated back to char)
        let result: i8 = ((data as i32) * 2) as i8;
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = ((data as i32) * 2) as i8;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: i8 = ((data as i32) * 2) as i8;
            print_hex_char_line(result);
        } else {
            print_line(Some(
                "data value is too large to perform arithmetic safely.",
            ));
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

/// Byte-at-a-time stdin reader with one byte of push-back, mirroring the way
/// `scanf` consumes exactly the characters it needs from the stream.
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

    fn peek(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
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
            Ok(_) => {
                self.peeked = Some(buf[0]);
                Some(buf[0])
            }
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let b = self.peek();
        self.peeked = None;
        b
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)`: returns `Some(value)` when a conversion took
/// place, `None` on matching failure or input failure (in which case the C code
/// leaves `x` untouched).
fn scanf_d(input: &mut Stdin) -> Option<i32> {
    // Skip leading whitespace (this is why scanf reads across newlines).
    loop {
        match input.peek() {
            Some(b) if is_c_space(b) => {
                input.next_byte();
            }
            _ => break,
        }
    }

    let mut negative = false;
    match input.peek() {
        Some(b'+') => {
            input.next_byte();
        }
        Some(b'-') => {
            negative = true;
            input.next_byte();
        }
        _ => {}
    }

    let mut saw_digit = false;
    // glibc converts via strtol, which saturates at LONG_MIN/LONG_MAX; the
    // resulting long is then stored through an `int *`.
    let mut acc: i64 = 0;
    let mut overflow = false;
    while let Some(b) = input.peek() {
        if !b.is_ascii_digit() {
            break;
        }
        input.next_byte();
        saw_digit = true;
        let digit = (b - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
    }

    if !saw_digit {
        return None;
    }

    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    Some(value as i32)
}

fn main() {
    let mut input = Stdin::new();

    let mut x: i32 = 0;
    if let Some(v) = scanf_d(&mut input) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = io::stdout().flush();
}
