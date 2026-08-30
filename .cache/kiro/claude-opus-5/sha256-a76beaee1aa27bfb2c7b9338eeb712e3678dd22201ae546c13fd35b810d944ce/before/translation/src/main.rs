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

/// `char` on the reference platform (x86-64 Linux gcc) is signed.
type CChar = i8;

/// Equivalent of `CHAR_MAX` for a signed 8-bit `char`.
const CHAR_MAX: CChar = CChar::MAX; // 127

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        println!("{}", line);
    }
}

fn print_hex_char_line(char_hex: CChar) {
    // printf("%02x\n", charHex);
    //
    // `charHex` undergoes the default argument promotion to `int`, and `%x`
    // then reinterprets that `int` as `unsigned int`. A negative char thus
    // prints as eight hex digits (e.g. -2 -> "fffffffe").
    println!("{:02x}", (char_hex as i32) as u32);
}

fn bad() {
    let data: CChar;
    data = CHAR_MAX;
    if data > 0 {
        // char result = data * 2;
        //
        // The operands are promoted to `int`, so 127 * 2 == 254, and the
        // conversion back to `char` wraps to -2 on this platform.
        let result: CChar = ((data as i32) * 2) as CChar;
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: CChar;
    data = 2;
    if data > 0 {
        let result: CChar = ((data as i32) * 2) as CChar;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: CChar;
    data = b' ' as CChar;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: CChar = ((data as i32) * 2) as CChar;
            print_hex_char_line(result);
        } else {
            print_line(Some("data value is too large to perform arithmetic safely."));
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

/// Byte-oriented view of stdin with a single byte of push-back, mirroring the
/// `getc`/`ungetc` pair that a C `scanf` implementation uses.
struct Scanner {
    stdin: io::Stdin,
    pushed_back: Option<u8>,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            stdin: io::stdin(),
            pushed_back: None,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushed_back.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.stdin.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    fn push_back(&mut self, b: u8) {
        self.pushed_back = Some(b);
    }

    /// Emulates `scanf("%d", &out)`: leading whitespace (including newlines)
    /// is skipped, an optional sign is accepted, and one or more decimal
    /// digits are consumed. Returns `true` when a value was assigned.
    fn scan_i32(&mut self, out: &mut i32) -> bool {
        // Skip whitespace, spanning newlines just like C's scanf.
        let mut c = loop {
            match self.next_byte() {
                None => return false,
                Some(b) if b.is_ascii_whitespace() || b == 0x0b => continue,
                Some(b) => break b,
            }
        };

        let negative = match c {
            b'-' => {
                c = match self.next_byte() {
                    Some(b) => b,
                    None => return false,
                };
                true
            }
            b'+' => {
                c = match self.next_byte() {
                    Some(b) => b,
                    None => return false,
                };
                false
            }
            _ => false,
        };

        if !c.is_ascii_digit() {
            // Matching failure: the offending byte stays in the stream and the
            // output object is left untouched.
            self.push_back(c);
            return false;
        }

        // glibc parses the digit run with strtol (saturating at long range)
        // and then narrows the result to `int`.
        let mut acc: i64 = 0;
        let mut saturated = false;
        loop {
            let digit = (c - b'0') as i64;
            if !saturated {
                match acc
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(digit))
                {
                    Some(v) => acc = v,
                    None => saturated = true,
                }
            }
            match self.next_byte() {
                Some(b) if b.is_ascii_digit() => c = b,
                Some(b) => {
                    self.push_back(b);
                    break;
                }
                None => break,
            }
        }

        let value: i64 = if saturated {
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

        *out = value as i32;
        true
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut scanner = Scanner::new();
    // The return value of scanf is discarded in the original program, so a
    // matching failure simply leaves `x` at 0.
    let _ = scanner.scan_i32(&mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = io::stdout().flush();
}
