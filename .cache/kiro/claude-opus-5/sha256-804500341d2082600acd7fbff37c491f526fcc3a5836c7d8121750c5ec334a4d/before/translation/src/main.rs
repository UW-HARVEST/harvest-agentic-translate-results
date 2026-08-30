// Rust translation of c_src/src/main.c
//
// Original C:
//     void printHexCharLine (char charHex) { printf("%02x\n", charHex); }
//     int main() {
//         char data;
//         data = ' ';
//         fscanf (stdin, "%c", &data);
//         { char result = data + 1; printHexCharLine(result); }
//         return 0;
//     }
//
// Behavioral details that are reproduced exactly:
//   * `data` is pre-initialized to ' ' (0x20). `fscanf(stdin, "%c", &data)`
//     reads exactly one byte with NO whitespace skipping (so '\n', ' ', '\t'
//     are all consumed as data). If the read fails (EOF / empty stdin), the
//     return value is ignored by the C code and `data` keeps its old value
//     ' ', so the program prints 0x21.
//   * `char` is signed on the reference platform (x86-64 Linux / gcc), and
//     `char result = data + 1;` truncates back to a signed char, so 0x7f
//     becomes -128 and 0xff becomes 0.
//   * `printf("%02x", charHex)` default-promotes the signed char to `int` and
//     then reinterprets it as `unsigned int`. For negative values that yields
//     the sign-extended 32-bit pattern, e.g. -128 prints as "ffffff80"
//     (8 digits, so the "02" minimum width has no effect). Non-negative
//     values print as at least two zero-padded hex digits.

use std::io::{self, Read, Write};

/// Equivalent of the C `printHexCharLine`: default-promote the `char` to `int`,
/// reinterpret as `unsigned int` for `%x`, minimum field width 2, zero padded.
fn print_hex_char_line(char_hex: i8) {
    let promoted = char_hex as i32 as u32;
    print!("{:02x}\n", promoted);
}

/// `fscanf(stdin, "%c", &data)`: read a single raw byte, returning `None` on
/// EOF or error (in which case the C code leaves `data` unmodified).
fn scanf_one_char() -> Option<u8> {
    let mut buf = [0u8; 1];
    let mut stdin = io::stdin();
    loop {
        match stdin.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => return Some(buf[0]),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

fn main() {
    let mut data: i8 = b' ' as i8;

    if let Some(byte) = scanf_one_char() {
        data = byte as i8;
    }

    {
        let result: i8 = data.wrapping_add(1);
        print_hex_char_line(result);
    }

    let _ = io::stdout().flush();
}
