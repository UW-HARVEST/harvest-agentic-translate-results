// Translated from c_src/src/main.c
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
//
//! Core translation of `c_src/src/main.c`.
//!
//! This module holds the whole behaviour of the C translation unit so that the
//! `driver` executable (`src/main.rs`) and the `cdylib` FFI surface
//! (`src/lib.rs`, which re-exports the same symbols the C shared library
//! exports) are guaranteed to run *identical* code.

use std::io::{Read, Write};

/// Mirrors:
///     void printHexCharLine (char charHex) { printf("%02x\n", charHex); }
///
/// `char` is signed on the reference platform (x86-64 Linux), and the default
/// argument promotions widen it to `int` before printf consumes it.  `%02x`
/// then reinterprets that `int` as an `unsigned int`, so negative characters
/// print as eight hex digits (e.g. -128 -> "ffffff80") while non-negative ones
/// print zero-padded to a minimum width of two (e.g. 5 -> "05").
pub fn print_hex_char_line(char_hex: i8, out: &mut impl Write) {
    let promoted = char_hex as i32; // default argument promotion (sign extend)
    let as_unsigned = promoted as u32; // %x reinterpretation
    let _ = write!(out, "{:02x}\n", as_unsigned);
}

/// `printHexCharLine` writing to the process' standard output, exactly like
/// `printf` does in C.
pub fn print_hex_char_line_stdout(char_hex: i8) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    print_hex_char_line(char_hex, &mut out);
    let _ = out.flush();
}

/// Mirrors C's `int main()`.
///
/// ```c
/// int main()
/// {
///     char data;
///     data = ' ';
///     fscanf (stdin, "%c", &data);
///     {
///         char result = data + 1;
///         printHexCharLine(result);
///     }
///
///     return 0;
/// }
/// ```
pub fn c_main() -> i32 {
    // char data; data = ' ';
    let mut data: i8 = b' ' as i8;

    // fscanf(stdin, "%c", &data);
    // "%c" consumes exactly one byte with no leading-whitespace skipping.  On
    // EOF (or a read failure) the conversion never happens, so `data` keeps its
    // previous value of ' '.
    let mut buf = [0u8; 1];
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    if let Ok(1) = handle.read(&mut buf) {
        data = buf[0] as i8;
    }
    drop(handle);

    {
        // char result = data + 1;
        // The addition happens in `int` and is truncated back to `char`,
        // wrapping on overflow as gcc does.
        let result: i8 = data.wrapping_add(1);
        print_hex_char_line_stdout(result);
    }

    // return 0;
    0
}
