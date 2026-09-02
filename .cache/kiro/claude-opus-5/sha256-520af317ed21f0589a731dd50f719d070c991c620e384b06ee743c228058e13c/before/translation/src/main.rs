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

//! Rust translation of c_src/src/main.c
//!
//! Behavior is preserved exactly, including the intentional defect in `bad()`
//! where the result of `intOne + intTwo` is computed and discarded rather than
//! assigned to `intSum`, so `bad()` prints `0` twice.

use std::io::{self, Write};

/// Mirrors the C `printLine(const char *line)`.
///
/// The C function guards against a NULL pointer before printing; the
/// equivalent guard here is the `Option` being `Some`. Callers in this program
/// always pass a non-null literal, so the guard never rejects anything, but it
/// is retained to keep the control flow identical.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // C: printf("%s\n", line);
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = writeln!(out, "{}", line);
    }
}

/// Mirrors the C `printIntLine(int intNumber)`.
fn print_int_line(int_number: i32) {
    // C: printf("%d\n", intNumber);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", int_number);
}

/// Mirrors the C `bad()`.
///
/// The original C body is:
///
/// ```c
/// int intOne = 1, intTwo = 1, intSum = 0;
/// printIntLine(intSum);
/// intOne + intTwo;   /* result discarded -- the defect */
/// printIntLine(intSum);
/// ```
///
/// `intSum` is never updated, so both prints emit `0`. This is reproduced
/// faithfully and deliberately not "fixed".
fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    // The unused sum, exactly as in the C source: computed, then discarded.
    let _ = int_one + int_two;
    print_int_line(int_sum);
}

/// Mirrors the C `good()`, which correctly assigns the sum to `intSum`.
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

    // C `main` returns 0; flush so output ordering matches C's stdio at exit.
    let _ = io::stdout().flush();
    std::process::exit(0);
}
