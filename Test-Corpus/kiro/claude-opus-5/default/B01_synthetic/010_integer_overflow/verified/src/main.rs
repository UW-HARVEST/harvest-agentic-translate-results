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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C program reads a single raw byte from stdin with `fscanf(stdin, "%c", &data)`
//! and prints `data + 1` via `printf("%02x\n", ...)`.
//!
//! Behaviors reproduced exactly (including the original's quirks):
//!
//! * `%c` never skips whitespace, so the very first byte on stdin is consumed
//!   verbatim -- a leading space or newline is data, not a separator.
//! * If the conversion fails (empty stdin / EOF, or a read error), `fscanf`
//!   leaves `data` untouched, so it retains its initialized value `' '` (0x20)
//!   and the program prints the increment of that instead.
//! * `char` is signed on the reference platform (x86-64 Linux). Passing a `char`
//!   through `...` promotes it to `int`, and `%x` then reinterprets that `int` as
//!   `unsigned int`. Bytes whose incremented value is negative are therefore
//!   sign-extended: input 0x7f prints `ffffff80`, not `80`.
//! * Signed overflow of `data + 1` is done in `int` in C, so no wraparound can
//!   occur there; the truncation back to `char` is what yields the negative value.

use std::io::{Read, Write};

/// Mirrors `void printHexCharLine(char charHex)`.
///
/// `charHex` is promoted to `int` for the variadic call, and `%02x` prints that
/// bit pattern as an `unsigned int` with a minimum field width of 2, zero padded.
fn print_hex_char_line(char_hex: i8) {
    let promoted_to_int = char_hex as i32;
    let as_unsigned = promoted_to_int as u32;
    print!("{:02x}\n", as_unsigned);
}

/// Mirrors `fscanf(stdin, "%c", &data)`.
///
/// Returns the byte read, or `None` when the conversion did not happen (EOF or
/// error), in which case the caller must leave its variable unmodified.
fn scanf_one_char() -> Option<u8> {
    let mut byte = [0u8; 1];
    let mut stdin = std::io::stdin();
    loop {
        match stdin.read(&mut byte) {
            Ok(0) => return None, // EOF: no assignment performed
            Ok(_) => return Some(byte[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None, // read error: no assignment performed
        }
    }
}

fn main() {
    // char data; data = ' ';
    let mut data: i8 = b' ' as i8;

    // fscanf(stdin, "%c", &data);
    if let Some(byte) = scanf_one_char() {
        data = byte as i8;
    }

    {
        // char result = data + 1;
        // The addition happens in `int`, then truncates back to `char`.
        let result: i8 = ((data as i32) + 1) as i8;
        print_hex_char_line(result);
    }

    // C's exit flushes stdout; Rust's line-buffered stdout is flushed here so
    // the byte stream is identical even when stdout is a pipe or file.
    let _ = std::io::stdout().flush();
}
