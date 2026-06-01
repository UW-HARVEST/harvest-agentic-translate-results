// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces the original behavior byte-for-byte,
// including sign-extension of `signed char` when promoted to `int` for
// printf("%02x", ...).

use std::io::{self, Read, Write};

fn print_hex_char_line<W: Write>(out: &mut W, char_hex: i8) {
    // In C, `printf("%02x\n", char_hex)` triggers default argument promotion:
    // the signed char is promoted to int (sign-extended), then reinterpreted
    // as unsigned int by the %x conversion specifier. The 02 specifies a
    // minimum field width of 2; values whose hex representation is wider
    // are printed in full (e.g. 0xffffff80).
    let promoted: i32 = char_hex as i32;
    let as_unsigned: u32 = promoted as u32;
    // Mimic %02x: lowercase hex, minimum width 2, zero-padded.
    write!(out, "{:02x}\n", as_unsigned).unwrap();
}

fn main() {
    // C: char data; data = ' ';
    // `char` signedness is implementation-defined; on the platforms targeted
    // by the C build (gcc on x86_64 Linux) `char` is signed. We model it as
    // i8 here so the addition uses signed semantics.
    let mut data: i8 = b' ' as i8;

    // C: fscanf(stdin, "%c", &data);
    // The "%c" conversion reads a single byte. If no byte is available
    // (EOF before any input is consumed), the assignment is not performed
    // and `data` retains its initialized value of ' '. Note that "%c"
    // does NOT skip whitespace, so a leading space or newline counts as
    // the character read.
    let mut buf = [0u8; 1];
    match io::stdin().read(&mut buf) {
        Ok(0) => {
            // EOF - leave `data` as ' '.
        }
        Ok(_) => {
            data = buf[0] as i8;
        }
        Err(_) => {
            // I/O error - leave `data` as ' ' (matches fscanf returning EOF).
        }
    }

    // C: char result = data + 1;
    // The C expression `data + 1` triggers integer promotion to int, the
    // addition happens in int, and the result is converted back to char.
    // For values where the int sum doesn't fit in char, this is
    // implementation-defined; gcc effectively truncates (wrapping). We use
    // wrapping_add on i8 to match.
    let result: i8 = data.wrapping_add(1);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    print_hex_char_line(&mut out, result);
}
