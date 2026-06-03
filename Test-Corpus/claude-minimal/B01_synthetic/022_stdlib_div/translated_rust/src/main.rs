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

struct DivT {
    quot: i32,
    rem: i32,
}

fn div(numer: i32, denom: i32) -> DivT {
    DivT {
        quot: numer / denom,
        rem: numer % denom,
    }
}

fn main() {
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    // Mimic scanf("%d %d", &x, &y): read whitespace-separated integers from stdin.
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read from stdin");

    let mut iter = input.split_ascii_whitespace();
    if let Some(tok) = iter.next() {
        if let Ok(v) = tok.parse::<i32>() {
            x = v;
        }
    }
    if let Some(tok) = iter.next() {
        if let Ok(v) = tok.parse::<i32>() {
            y = v;
        }
    }

    let result = div(x, y);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(
        handle,
        "quotient: {}, remainder: {}",
        result.quot, result.rem
    )
    .expect("failed to write to stdout");
}
