// Rust translation of c_src/src/main.c
//
// Behavior is preserved exactly, including the intentional defect in `bad()`
// where the result of `intOne + intTwo` is computed and discarded instead of
// being stored in `intSum` (CWE-482: Comparing/using an unused value).
//
// Original C copyright notice:
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

/// Mirrors `void printLine(const char *line)`.
///
/// The C version guards against a NULL pointer, so the Rust version takes an
/// `Option<&str>` and prints nothing for `None`.
fn print_line<W: Write>(out: &mut W, line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        let _ = writeln!(out, "{}", line);
    }
}

/// Mirrors `void printIntLine(int intNumber)`.
fn print_int_line<W: Write>(out: &mut W, int_number: i32) {
    // printf("%d\n", intNumber);
    let _ = writeln!(out, "{}", int_number);
}

/// Mirrors `void bad()`.
///
/// The addition result is intentionally discarded (as in the original C), so
/// `int_sum` stays 0 and is printed as 0 both times.
fn bad<W: Write>(out: &mut W) {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(out, int_sum);
    // `intOne + intTwo;` — value computed, never assigned. Reproduced verbatim.
    // wrapping_add keeps the semantics of C `int` arithmetic without panicking.
    let _ = int_one.wrapping_add(int_two);
    print_int_line(out, int_sum);
}

/// Mirrors `void good()`.
fn good<W: Write>(out: &mut W) {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(out, int_sum);
    int_sum = int_one.wrapping_add(int_two);
    print_int_line(out, int_sum);
}

fn main() {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    print_line(&mut out, Some("Calling good()..."));
    good(&mut out);
    print_line(&mut out, Some("Finished good()"));
    print_line(&mut out, Some("Calling bad()..."));
    bad(&mut out);
    print_line(&mut out, Some("Finished bad()"));

    // Flush explicitly; C's exit from main flushes stdout.
    let _ = out.flush();

    // return 0;
    std::process::exit(0);
}
