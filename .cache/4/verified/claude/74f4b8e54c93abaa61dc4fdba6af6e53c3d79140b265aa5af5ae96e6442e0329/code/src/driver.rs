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

//! Translation of `c_src/src/main.c`.
//!
//! This module holds the pure-Rust translation; it is shared by both the
//! `driver` binary (`src/main.rs`) and the `libdriver.so` cdylib
//! (`src/lib.rs`, which adds the `#[no_mangle] extern "C"` export wrappers).
//!
//! Fidelity notes:
//! * `printLine` receives a `const char *` in C, i.e. an arbitrary NUL
//!   terminated byte string that may not be valid UTF-8.  The Rust signature is
//!   therefore `Option<&[u8]>`: `None` models the NULL pointer (no output, just
//!   like C's `if (line != NULL)` guard) and the bytes are written verbatim.
//! * `printf("%s\n", line)` treats `line` as *data*, never as a format string,
//!   so `%` characters inside `line` are emitted literally.

use std::io::Write;

/// Mirrors C's `void printLine(const char *line)`.
///
/// ```c
/// void printLine (const char * line)
/// {
///     if(line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
pub fn print_line(line: Option<&[u8]>) {
    if let Some(line) = line {
        // printf("%s\n", line);  -- the bytes of `line` followed by '\n'.
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(line);
        let _ = out.write_all(b"\n");
        let _ = out.flush();
    }
}

/// Mirrors C's `void printIntLine(int intNumber)`.
///
/// ```c
/// void printIntLine (int intNumber)
/// {
///     printf("%d\n", intNumber);
/// }
/// ```
pub fn print_int_line(int_number: i32) {
    // printf("%d\n", intNumber);  -- `{}` on i32 matches glibc's "%d" for every
    // value of the range, including i32::MIN ("-2147483648").
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", int_number);
    let _ = out.flush();
}

/// Mirrors C's `void bad()`.
///
/// NOTE: the original C computes `intOne + intTwo;` as an expression statement
/// whose result is discarded, so `intSum` is never updated and `0` is printed
/// twice.  This (buggy) behavior is reproduced verbatim rather than "fixed".
pub fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    let _ = int_one.wrapping_add(int_two); // discarded, just like in C
    print_int_line(int_sum);
}

/// Mirrors C's `void good()`.
pub fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(int_sum);
    int_sum = int_one.wrapping_add(int_two);
    print_int_line(int_sum);
}

/// Mirrors C's `int main(int argc, char *argv[])`.
///
/// `argc` / `argv` are unused by the C implementation, so they are unused here
/// as well.  Returns C's exit status (`return 0;`).
pub fn program_main() -> i32 {
    print_line(Some(b"Calling good()..."));
    good();
    print_line(Some(b"Finished good()"));
    print_line(Some(b"Calling bad()..."));
    bad();
    print_line(Some(b"Finished bad()"));
    // return 0;
    0
}
