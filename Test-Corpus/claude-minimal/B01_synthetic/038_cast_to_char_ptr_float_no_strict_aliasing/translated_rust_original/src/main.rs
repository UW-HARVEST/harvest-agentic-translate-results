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

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for byte in p {
        write!(handle, "{:02x}", byte).unwrap();
    }
    writeln!(handle).unwrap();
}

fn driver(x: f32) {
    // Mimic memcpy of float bytes into a raw buffer, then print as hex.
    let raw: [u8; std::mem::size_of::<f32>()] = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    // Read a float from stdin, mirroring scanf("%f", &x) behavior:
    // read whitespace-separated token and parse as f32.
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");

    let x: f32 = match input.split_whitespace().next() {
        Some(token) => token.parse().unwrap_or(0.0),
        None => 0.0,
    };

    driver(x);
}
