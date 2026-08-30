// Translation of c_src/src/main.c to Rust.
//
// Original copyright notice from the C source:
//
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

/// Mirrors `void printLine(const char * line)`.
///
/// The C version guards against a NULL pointer; `Option<&str>` models that,
/// and `None` (NULL) prints nothing.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        print!("{}\n", line);
    }
}

/// Mirrors `void printIntLine(int intNumber)` -> printf("%d\n", intNumber);
fn print_int_line(int_number: i32) {
    print!("{}\n", int_number);
}

/// Mirrors `void bad()`.
///
/// NOTE: the original C contains the statement `intOne + intTwo;`, whose value
/// is discarded, so `intSum` is never assigned and stays 0. This bug is
/// reproduced verbatim: both printed values are 0.
fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    // Discarded expression, exactly as in the C source (no assignment).
    let _ = int_one + int_two;
    print_int_line(int_sum);
}

/// Mirrors `void good()`.
fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(int_sum);
    int_sum = int_one + int_two;
    print_int_line(int_sum);
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));

    // C's exit flushes stdout; make that explicit here.
    let _ = io::stdout().flush();
}
