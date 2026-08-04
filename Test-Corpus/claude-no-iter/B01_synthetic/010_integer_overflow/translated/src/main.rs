// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from C; reproduces byte-identical output.

use std::io::{self, Read, Write, BufWriter};

fn print_hex_char_line<W: Write>(out: &mut W, char_hex: i8) {
    // In C, the char argument undergoes default argument promotion to int
    // when passed to printf. A signed char is sign-extended, so negative
    // values become negative ints and `%02x` reinterprets them as unsigned
    // int, printing 8 hex digits like "ffffff80".
    let as_unsigned = (char_hex as i32) as u32;
    writeln!(out, "{:02x}", as_unsigned).unwrap();
}

fn main() {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    // char data; data = ' ';
    // In C this is `char`, which is signed on common Linux/x86 toolchains.
    let mut data: i8 = b' ' as i8;

    // fscanf(stdin, "%c", &data);
    // %c reads a single character (no whitespace skipping). On EOF/no-input,
    // fscanf returns EOF and `data` is left unchanged (still ' ').
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    match handle.read(&mut buf) {
        Ok(n) if n >= 1 => {
            data = buf[0] as i8;
        }
        _ => {
            // No bytes read: data remains as initialized.
        }
    }

    {
        // char result = data + 1;
        // C performs integer promotion to int for `data + 1` and then narrows
        // back to char when assigning. We model that with wrapping i8 add.
        let result: i8 = data.wrapping_add(1);
        print_hex_char_line(&mut out, result);
    }

    out.flush().unwrap();
}
