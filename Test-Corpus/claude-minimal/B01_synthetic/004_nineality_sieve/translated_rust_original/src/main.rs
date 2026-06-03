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

use std::env;
use std::process::ExitCode;

/*
Count from a starting point,
stopping when the count ends in 9 (base 10).
*/
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    // Mimic C's strtol behavior: parse leading integer, allow trailing junk.
    // If nothing is parsed, treat as error (like end == argv[1] in C).
    let input = &args[1];
    let bytes = input.as_bytes();
    let mut idx = 0;

    // Allow optional leading whitespace (strtol skips whitespace)
    while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
        idx += 1;
    }

    let sign_start = idx;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        idx += 1;
    }

    let digits_start = idx;
    while idx < bytes.len() && (bytes[idx] as char).is_ascii_digit() {
        idx += 1;
    }

    if digits_start == idx {
        // No digits parsed
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    let parsed_str = &input[sign_start..idx];
    let mut val: i32 = match parsed_str.parse::<i32>() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            return ExitCode::from(1);
        }
    };

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }

    ExitCode::from(0)
}
