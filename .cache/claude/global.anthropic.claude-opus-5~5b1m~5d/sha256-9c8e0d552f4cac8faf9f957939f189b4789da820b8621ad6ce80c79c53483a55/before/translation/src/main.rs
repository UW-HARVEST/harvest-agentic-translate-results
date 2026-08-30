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

use std::io::{Read, Write};

/// Mirrors `void printHexCharLine(char charHex)` which does
/// `printf("%02x\n", charHex);`.
///
/// In C, `charHex` is a (signed, on the usual x86/ARM-Linux ABI) `char` that is
/// promoted to `int` for the variadic call, and `%x` then reinterprets that
/// `int` as an `unsigned int`. So a negative char such as 0x80 (-128) prints as
/// "ffffff80", not "80". The `02` flag only sets a *minimum* field width.
fn print_hex_char_line(char_hex: i8) {
    let promoted_as_unsigned = char_hex as i32 as u32;
    print!("{:02x}\n", promoted_as_unsigned);
}

fn main() {
    // char data; data = ' ';
    let mut data: i8 = b' ' as i8;

    // fscanf(stdin, "%c", &data);
    // "%c" consumes exactly one byte with no leading-whitespace skipping and no
    // regard for newlines. On EOF / read failure the conversion does not happen
    // and `data` keeps its previous value (' ').
    let mut buf = [0u8; 1];
    match std::io::stdin().read(&mut buf) {
        Ok(1) => data = buf[0] as i8,
        _ => {}
    }

    {
        // char result = data + 1;
        // The addition happens in `int` and is then truncated back to `char`,
        // which wraps for data == 0x7f.
        let result: i8 = data.wrapping_add(1);
        print_hex_char_line(result);
    }

    let _ = std::io::stdout().flush();
}
