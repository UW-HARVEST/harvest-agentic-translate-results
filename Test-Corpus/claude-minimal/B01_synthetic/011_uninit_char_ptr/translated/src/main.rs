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

use std::io::{self, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn bad() {
    // The original C code declares an uninitialized pointer `char *data;`
    // and passes it to printLine, which is undefined behavior in C.
    // In Rust we represent this as `None` (modeling the case where the
    // uninitialized pointer happens to be NULL, which printLine checks for).
    let data: Option<&str> = None;
    print_line(data);
}

fn good() {
    let data: Option<&str> = Some("string");
    print_line(data);
}

fn main() {
    let mut x: i32 = 0;
    let mut input = String::new();

    // Mimic scanf("%d", &x): read an integer from stdin.
    io::stdout().flush().ok();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Ok(parsed) = input.trim().parse::<i32>() {
            x = parsed;
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
