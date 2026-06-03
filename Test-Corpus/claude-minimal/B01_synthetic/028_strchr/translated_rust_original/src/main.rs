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

use std::io::Read;

/// Count occurrences of byte `c` in `input` until the first null byte.
/// Mirrors the C implementation that uses `strchr` over a C string.
fn foo(input: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    for &b in input {
        if b == 0 {
            break;
        }
        if b == c {
            res += 1;
        }
    }
    res
}

fn driver(input: &[u8]) {
    println!("A: {}", foo(input, b'A'));
    println!("x: {}", foo(input, b'x'));
}

fn main() {
    // Match the C buffer: 1000 bytes, zero-initialized.
    let mut buf = [0u8; 1000];
    // fread(in, 1, sizeof(in), stdin): read up to 1000 bytes from stdin.
    let mut stdin = std::io::stdin();
    let mut total_read = 0usize;
    while total_read < buf.len() {
        match stdin.read(&mut buf[total_read..]) {
            Ok(0) => break,
            Ok(n) => total_read += n,
            Err(_) => break,
        }
    }
    driver(&buf);
}
