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

// Translation of c_src/src/main.c
//
// The C `printLine` takes a `const char *` and guards against NULL. In Rust the
// equivalent parameter is an `Option<&str>`: `None` models the NULL pointer and
// results in no output, exactly as in C.

use std::io::{self, Write};

/// Mirrors C's `void printLine(const char *line)`.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        println!("{}", line);
    }
}

/// Mirrors C's `void printIntLine(int intNumber)`.
fn print_int_line(int_number: i32) {
    // printf("%d\n", intNumber);
    println!("{}", int_number);
}

/// Mirrors C's `void bad()`.
///
/// NOTE: the original C computes `intOne + intTwo;` as a statement whose result
/// is discarded, so `intSum` is never updated and `0` is printed twice. This
/// (buggy) behavior is reproduced verbatim rather than "fixed".
fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    let _ = int_one.wrapping_add(int_two); // discarded, just like in C
    print_int_line(int_sum);
}

/// Mirrors C's `void good()`.
fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(int_sum);
    int_sum = int_one.wrapping_add(int_two);
    print_int_line(int_sum);
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));

    // Flush explicitly so output is not lost, mirroring C's exit-time flush.
    let _ = io::stdout().flush();

    // return 0;
    std::process::exit(0);
}
