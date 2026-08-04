// Translation of c_src/src/driver.c to Rust.
//
// The original C source is a shared library exposing a single function:
//     void driver(char data)
// which prints `(char)(data + 1)` formatted with C's `printf("%02x\n", c)`.
//
// In C, when a `char` (which is `signed char` on most platforms, including
// Linux x86_64 where this code is intended to run) is passed to a variadic
// function such as `printf`, it is promoted to `int`. The `%02x` conversion
// then interprets the promoted `int` as `unsigned int`. Therefore a negative
// `char` value such as -128 (0x80 as a bit pattern) is sign-extended to the
// 32-bit value 0xFFFFFF80 and printed as "ffffff80\n".
//
// This binary wraps the library by reading bytes from standard input one at
// a time (matching how the function would naturally be invoked on each input
// byte) and invoking `driver` on each. Any C "stdin reading behavior" simply
// reads raw bytes here -- no scanf/fgets is involved in the original code.

use std::io::{self, Read, Write, BufWriter};

fn print_hex_char_line<W: Write>(out: &mut W, char_hex: i8) {
    // Mimic C's `printf("%02x\n", charHex)` where `charHex` is a signed char.
    // The signed char is promoted to int, then reinterpreted as unsigned int
    // for the `%x` conversion. The minimum field width is 2 with zero-padding.
    let promoted: i32 = char_hex as i32; // sign-extending promotion
    let as_unsigned: u32 = promoted as u32; // reinterpret as unsigned int
    // `%02x` => lowercase hex, zero-padded to a minimum width of 2.
    // `format!("{:02x}", value)` produces identical output for u32.
    writeln!(out, "{:02x}", as_unsigned).expect("write failed");
}

fn driver<W: Write>(out: &mut W, data: i8) {
    // C: `char result = data + 1;`
    // For signed char arithmetic, the addition is performed in `int` and then
    // implicitly converted back to `char`. Conversion of an out-of-range int
    // to a signed char is implementation-defined in C, but on all common
    // platforms (gcc/clang on x86_64) it is a two's-complement wrap, which
    // matches Rust's `wrapping_add` on `i8`.
    let result: i8 = data.wrapping_add(1);
    print_hex_char_line(out, result);
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut input = Vec::new();
    // Read all bytes from stdin. The original library has no input behavior
    // of its own; this wrapper feeds each input byte to `driver` in order.
    if let Err(e) = stdin.lock().read_to_end(&mut input) {
        eprintln!("error reading stdin: {}", e);
        std::process::exit(1);
    }

    for byte in input.iter() {
        // `byte` is u8; reinterpret as signed char to match C's `char` type
        // on platforms where `char` is signed (the typical case).
        let data = *byte as i8;
        driver(&mut out, data);
    }
}
