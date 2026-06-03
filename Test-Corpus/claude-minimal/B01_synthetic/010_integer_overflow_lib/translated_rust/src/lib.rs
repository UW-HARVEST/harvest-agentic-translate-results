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

use std::os::raw::c_char;

/// Prints the given char value as a zero-padded hex string followed by a newline.
///
/// Mirrors the C implementation `printf("%02x\n", charHex)` where the `char` is
/// promoted to `int` before formatting. On platforms where `char` is signed,
/// negative values are sign-extended to `int`, which produces an 8-hex-digit
/// representation (e.g. `ffffff80`).
pub fn print_hex_char_line(char_hex: c_char) {
    // Promote the char to int the same way C does.
    let promoted: i32 = char_hex as i32;
    // Reinterpret the int as unsigned for the %x conversion.
    let as_unsigned: u32 = promoted as u32;
    if promoted < 0 {
        // Negative values get sign-extended; print all the hex digits.
        println!("{:02x}", as_unsigned);
    } else {
        println!("{:02x}", as_unsigned);
    }
}

/// Adds 1 to the input data byte (with wrapping semantics matching typical C
/// signed-overflow behavior on two's complement platforms) and prints the
/// result as a hex value.
pub fn driver(data: c_char) {
    let result: c_char = data.wrapping_add(1);
    print_hex_char_line(result);
}

/// C-compatible export of `driver` so this library can be used as a drop-in
/// replacement for the C shared library.
#[no_mangle]
pub extern "C" fn driver_c(data: c_char) {
    driver(data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_does_not_panic() {
        driver(0);
        driver(1);
        driver(-1);
        driver(127);
    }
}
