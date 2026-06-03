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
    let mut y: i32 = 2 * x;
    y += 300;
    println!("{}", y);
}

fn main() {
    let mut x: i32 = 0;
    let stdin = io::stdin();
    let mut input = String::new();
    // Read a line and parse the first integer token, mimicking scanf("%d", &x).
    loop {
        input.clear();
        let bytes = stdin.lock().read_line(&mut input).expect("failed to read");
        if bytes == 0 {
            break;
        }
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(token) = trimmed.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<i32>() {
                x = parsed;
            }
        }
        break;
    }
    let _ = io::stdout().flush();
    driver(x);
}
