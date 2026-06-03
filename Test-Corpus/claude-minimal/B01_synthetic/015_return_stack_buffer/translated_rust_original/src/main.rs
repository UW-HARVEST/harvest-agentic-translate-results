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

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

// Mirrors the C `helperBad()` which returns a pointer to a stack-allocated
// buffer. In Rust, returning a reference to a local would not compile, so we
// preserve the behavior of producing the string by returning an owned String.
fn helper_bad() -> String {
    let char_string: String = String::from("helperBad string");
    char_string
}

fn bad() {
    let s = helper_bad();
    print_line(Some(s.as_str()));
}

fn helper_good1() -> &'static str {
    static CHAR_STRING: &str = "helperGood1 string";
    CHAR_STRING
}

fn good() {
    print_line(Some(helper_good1()));
}

fn main() {
    let mut x: i32 = 0;

    // Mimic scanf("%d", &x): read whitespace-separated integer tokens from stdin.
    let stdin = io::stdin();
    let mut input = String::new();
    // Read one line (best-effort match for scanf behavior in this simple program).
    let _ = io::stdout().flush();
    if stdin.lock().read_line(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<i32>() {
                x = parsed;
            }
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
