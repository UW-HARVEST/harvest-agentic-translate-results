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

use std::io::Read;

fn print_hex_char_line(char_hex: i8) {
    // Mimic C's printf("%02x\n", charHex) where char is sign-extended
    // to int when passed through varargs, then interpreted as unsigned.
    let promoted: i32 = char_hex as i32;
    let as_unsigned: u32 = promoted as u32;
    if as_unsigned <= 0xFF {
        println!("{:02x}", as_unsigned);
    } else {
        // Sign-extended negative values produce 8 hex digits in C.
        println!("{:02x}", as_unsigned);
    }
}

fn main() {
    let mut data: i8 = b' ' as i8;

    let mut buf = [0u8; 1];
    if std::io::stdin().read_exact(&mut buf).is_ok() {
        data = buf[0] as i8;
    }

    let result: i8 = data.wrapping_add(1);
    print_hex_char_line(result);
}
