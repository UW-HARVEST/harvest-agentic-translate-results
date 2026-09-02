// Translation of c_src/src/main.c to Rust.
//
// Original copyright notice from the C source is reproduced below.
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

mod cfloat;

use std::io::{self, Read, Write};

/// `static void print_hex(unsigned char *p, int len)`
///
/// Writes each byte as two lowercase hex digits, then a newline, exactly as
/// `printf("%02x")` followed by `printf("\n")` would.
fn print_hex(p: &[u8], len: usize) {
    let mut buf = Vec::with_capacity(len * 2 + 1);
    for i in 0..len {
        let b = p[i];
        buf.push(hex_digit(b >> 4));
        buf.push(hex_digit(b & 0x0f));
    }
    buf.push(b'\n');

    let stdout = io::stdout();
    let mut lock = stdout.lock();
    // Ignore write errors the same way the C code ignores printf's return value.
    let _ = lock.write_all(&buf);
    let _ = lock.flush();
}

fn hex_digit(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    }
}

/// `void driver(float x)`
///
/// Reinterprets the object representation of `x` as bytes, matching
/// `(unsigned char *)&x` with `sizeof(x)` == 4. `to_ne_bytes` reproduces the
/// host byte order that the C cast exposes.
fn driver(x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes, core::mem::size_of::<f32>());
}

fn main() {
    // `float x = 0.f;`
    let mut x: f32 = 0.0;

    // `scanf("%f", &x);`
    //
    // scanf skips arbitrary leading whitespace (including newlines) and then
    // consumes the longest prefix that forms a floating constant. On a matching
    // failure or EOF, the object is left untouched, so `x` keeps its 0.0 value.
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);
    if let Some(v) = cfloat::scanf_float(&input) {
        x = v;
    }

    // `driver(x);`
    driver(x);

    // `return 0;`
}
