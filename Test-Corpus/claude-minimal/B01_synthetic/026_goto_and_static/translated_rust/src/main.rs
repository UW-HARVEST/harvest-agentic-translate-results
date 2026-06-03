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

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result: i32 = 0;
    'fail: {
        if x != 1 {
            println!("Error: x != 1");
            result = 1;
            break 'fail;
        }

        // SAFETY: single-threaded program reading the static mutable global Y,
        // matching the C source semantics.
        let y_val = unsafe { Y };
        if y_val != 2 {
            println!("Error: x == 1 but y != 2");
            result = 2;
            break 'fail;
        }

        if z != 3 {
            println!("Error: x == 1 and y == 2, but z != 3");
            result = 3;
            break 'fail;
        }

        println!("Ok!");
        return result;
    }

    println!("Operation failed");
    result
}

fn read_three_ints() -> (i32, i32, i32) {
    // Mimic C's scanf("%d %d %d", ...) by reading whitespace-separated integers
    // from standard input until three have been parsed.
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read from stdin");

    let mut iter = input.split_ascii_whitespace();
    let parse_next = |it: &mut std::str::SplitAsciiWhitespace| -> i32 {
        match it.next() {
            Some(tok) => tok.parse::<i32>().unwrap_or(0),
            None => 0,
        }
    };

    let x = parse_next(&mut iter);
    let y = parse_next(&mut iter);
    let z = parse_next(&mut iter);
    (x, y, z)
}

fn main() {
    // Match the C source which initializes x and z to 0 before scanf overwrites them.
    let (x, ry, z) = read_three_ints();
    // SAFETY: single-threaded program writing to the static mutable global Y,
    // matching the C source semantics.
    unsafe {
        Y = ry;
    }

    let result = multi_stage(x, z);
    println!("Result: {}", result);

    // Make sure stdout is flushed before exit.
    let _ = io::stdout().flush();
}
