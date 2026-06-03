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

//! Rust translation of the C `driver` library.

/// Print a line of text, mirroring the C `printLine` helper.
///
/// In C the function accepts a pointer that may be `NULL`, so this Rust
/// counterpart accepts an `Option<&str>` and prints nothing for `None`.
pub fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

/// Print an integer followed by a newline, mirroring the C `printIntLine`
/// helper.
pub fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

/// Faithful translation of the C `bad()` function. The C version computes
/// `intOne + intTwo` but discards the result, so `intSum` remains `0`.
pub fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    // The original C code computes `intOne + intTwo;` and discards the
    // result. We replicate that behaviour by computing and ignoring the sum.
    let _ = int_one.wrapping_add(int_two);
    print_int_line(int_sum);
}

/// Faithful translation of the C `good()` function.
pub fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(int_sum);
    int_sum = int_one + int_two;
    print_int_line(int_sum);
}

/// Top-level driver that exercises both `good()` and `bad()`.
pub fn driver() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
}

/// C ABI entry point so this library can be used as a drop-in replacement
/// for the original C shared library.
#[no_mangle]
pub extern "C" fn driver_c() {
    driver();
}
