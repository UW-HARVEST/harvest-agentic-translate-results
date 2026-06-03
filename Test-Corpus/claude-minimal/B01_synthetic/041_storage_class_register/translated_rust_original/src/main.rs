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

use std::io::{self, BufRead, Write};

fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    println!("{}", y);
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.flush();

    let mut x: i32 = 0;

    // Mimic scanf("%d", &x): read whitespace-separated tokens until we get an integer.
    // If parsing fails or input ends, x remains 0 (matching uninitialized-then-failed-scanf semantics
    // is undefined in C; this initializes to 0 as the C code does prior to scanf).
    let mut buf = String::new();
    let mut handle = stdin.lock();

    'outer: loop {
        buf.clear();
        match handle.read_line(&mut buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                for tok in buf.split_whitespace() {
                    if let Ok(v) = tok.parse::<i32>() {
                        x = v;
                        break 'outer;
                    }
                }
            }
            Err(_) => break,
        }
    }

    driver(x);
}
