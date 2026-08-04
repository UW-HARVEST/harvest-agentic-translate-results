// Translated from C - reproduces byte-identical output

use std::io::{self, Read, Write};

fn print_hex_char_line(char_hex: i8) {
    // C: printf("%02x\n", charHex);
    // In C, `char` (here signed on most platforms like x86 Linux) is promoted to
    // `int` when passed as a variadic argument. `%x` then reinterprets that
    // `int` bit pattern as `unsigned int`. So for negative `char` values, the
    // sign extension produces a large unsigned int, printed as e.g. "ffffffff".
    // %02x means "at least 2 hex digits, zero-padded"; longer values print in
    // full.
    let as_int: i32 = char_hex as i32;
    let as_uint: u32 = as_int as u32;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if as_uint <= 0xFF {
        // fits in 2 hex digits (or is small) — apply %02x padding
        write!(out, "{:02x}\n", as_uint).unwrap();
    } else {
        // larger than 2 hex digits — just print full hex with no extra padding
        write!(out, "{:x}\n", as_uint).unwrap();
    }
}

fn main() {
    // C: char data; data = ' '; fscanf(stdin, "%c", &data);
    // On most platforms (e.g. Linux/x86) `char` is signed.
    let mut data: i8 = b' ' as i8;

    // fscanf(stdin, "%c", &data) reads exactly one byte (no whitespace skip
    // for %c). If EOF is hit before reading, `data` is unchanged.
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    if let Ok(n) = handle.read(&mut buf) {
        if n == 1 {
            data = buf[0] as i8;
        }
    }

    // C: char result = data + 1;
    // In C, this is `(int)data + 1` (integer promotion), then implicit
    // conversion back to char. The conversion to signed char of out-of-range
    // values is implementation-defined, but on every common platform it's
    // a simple bitwise truncation (two's complement wrap).
    let result: i8 = data.wrapping_add(1);

    print_hex_char_line(result);

    // Ensure all output is flushed before exit (matches C's stdout flush at
    // program exit).
    io::stdout().flush().unwrap();
}
