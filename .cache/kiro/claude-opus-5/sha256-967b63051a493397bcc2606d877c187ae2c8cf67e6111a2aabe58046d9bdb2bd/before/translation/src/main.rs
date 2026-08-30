// Rust translation of c_src/src/main.c
//
// Original C:
//
//     void printHexCharLine (char charHex) { printf("%02x\n", charHex); }
//
//     int main() {
//         char data;
//         data = ' ';
//         fscanf (stdin, "%c", &data);
//         { char result = data + 1; printHexCharLine(result); }
//         return 0;
//     }
//
// Behavior notes that must be reproduced exactly:
//
//  * `%c` in scanf does NOT skip leading whitespace and reads exactly one byte,
//    including '\n'. If the read fails (EOF / empty stdin), `data` is left at
//    its initialized value of ' ' (0x20).
//  * `char` is signed on the reference platform (x86-64 Linux). `data + 1` is
//    computed in `int` and then truncated back into a `char`, so 0x7F wraps to
//    -128 (0x80).
//  * `printf("%02x", charHex)` promotes the signed `char` to `int` (sign
//    extension) and then reinterprets those bits as `unsigned int`. Negative
//    values therefore print as eight hex digits, e.g. -128 -> "ffffff80".
//    The "02" is a minimum field width, so it never truncates.

use std::io::{self, Read, Write};

fn print_hex_char_line<W: Write>(out: &mut W, char_hex: i8) {
    // %02x on an `int` produced by integer promotion of a signed char.
    let promoted = char_hex as i32;
    let as_unsigned = promoted as u32;
    let _ = write!(out, "{:02x}\n", as_unsigned);
}

fn main() {
    // char data; data = ' ';
    let mut data: i8 = b' ' as i8;

    // fscanf(stdin, "%c", &data);
    let mut byte = [0u8; 1];
    let mut stdin = io::stdin().lock();
    // read_exact retries on EINTR, matching stdio's behavior, and leaves `data`
    // untouched when no byte is available (the EOF / matching-failure case).
    if stdin.read_exact(&mut byte).is_ok() {
        data = byte[0] as i8;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    {
        // char result = data + 1;  (computed in int, truncated to char)
        let result: i8 = ((data as i32) + 1) as i8;
        print_hex_char_line(&mut out, result);
    }

    let _ = out.flush();
}
