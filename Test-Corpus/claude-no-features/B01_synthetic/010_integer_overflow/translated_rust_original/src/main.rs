// Translation of c_src/src/main.c to Rust producing byte-identical output.
//
// Original C behavior:
//   char data = ' ';
//   fscanf(stdin, "%c", &data);   // reads one byte; on EOF leaves data unchanged
//   char result = data + 1;       // signed char wrap-around addition
//   printf("%02x\n", result);     // signed char promoted to int (sign-extended),
//                                 // then formatted as unsigned hex with min-width 2.

use std::io::{self, Read, Write};

fn print_hex_char_line(char_hex: i8) {
    // In C, when a `char` (signed by default) is passed through printf's varargs,
    // it is promoted to `int` with sign extension. The %x conversion then
    // reinterprets that int as `unsigned int`.
    let as_int: i32 = char_hex as i32;
    let as_uint: u32 = as_int as u32;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // %02x => lowercase hex, zero-padded to a minimum width of 2.
    write!(out, "{:02x}\n", as_uint).expect("write to stdout failed");
}

fn main() {
    // Initialize data to ' ' (0x20), matching the C source.
    let mut data: i8 = b' ' as i8;

    // fscanf(stdin, "%c", &data) reads exactly one byte. If stdin is at EOF
    // (or the read otherwise fails) the C code leaves `data` untouched,
    // so we mirror that behaviour: only update `data` if a byte was read.
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    match handle.read(&mut buf) {
        Ok(0) => {
            // EOF: leave data as-is.
        }
        Ok(_) => {
            data = buf[0] as i8;
        }
        Err(_) => {
            // Read error: leave data as-is, matching scanf failure behaviour.
        }
    }

    // result = data + 1, with signed char wrap-around (8-bit two's complement).
    let result: i8 = data.wrapping_add(1);
    print_hex_char_line(result);
}
