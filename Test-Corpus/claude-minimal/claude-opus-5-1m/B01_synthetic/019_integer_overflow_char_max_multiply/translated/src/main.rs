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

// Equivalent of CHAR_MAX for signed char (i8) on typical platforms.
const CHAR_MAX: i8 = i8::MAX;

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn print_hex_char_line(char_hex: i8) {
    // Mirror C's `printf("%02x\n", charHex)` semantics: the `char` argument is
    // promoted to `int` via default argument promotions. On platforms where
    // `char` is signed, a negative value is sign-extended to int and then
    // reinterpreted as `unsigned int` by `%x`, yielding e.g. "fffffffe".
    let promoted: i32 = char_hex as i32;
    let unsigned: u32 = promoted as u32;
    println!("{:02x}", unsigned);
}

fn bad() {
    let data: i8;
    data = CHAR_MAX;
    if data > 0 {
        // Use wrapping_mul to emulate C's wraparound on signed overflow
        // (which is technically UB in C but commonly wraps in practice).
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8;
    data = 2;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

#[allow(unused_assignments)]
fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: i8 = data.wrapping_mul(2);
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

fn read_int_from_stdin() -> i32 {
    // Emulate `scanf("%d", &x)` minimally: skip leading whitespace and parse
    // an optional sign followed by digits. If parsing fails, leave x at 0
    // (matching the initial value used by main).
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return 0;
    }

    let bytes = input.as_bytes();
    let mut i = 0;
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

    let start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }

    if start == i {
        return 0;
    }

    let digits = &input[start..i];
    match digits.parse::<i32>() {
        Ok(n) => sign * n,
        Err(_) => 0,
    }
}

fn main() {
    // Make sure stdout is flushed at the end.
    let x: i32 = read_int_from_stdin();

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = io::stdout().flush();
}
