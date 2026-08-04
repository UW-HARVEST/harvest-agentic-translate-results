// Translation of c_src/src/driver.c to Rust.
// Produces byte-identical stdout to the C original.

#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(charHex: c_char) {
    // The C code does: printf("%02x\n", charHex)
    // In C, when a `char` is passed via varargs to printf, it undergoes the
    // default argument promotions: it is promoted to `int`. If the platform's
    // plain `char` is signed (the common case for x86_64 Linux), values with
    // the high bit set are sign-extended. The `%x` conversion specifier then
    // prints the resulting `int` reinterpreted as `unsigned int`, producing
    // long hex sequences like "ffffff80" for negative chars.
    //
    // We reproduce this exactly by promoting `c_char` (which is i8 on
    // platforms where C's `char` is signed) to `c_int` via `as`, which
    // performs sign-extension, then passing it through varargs to printf.
    let promoted: c_int = charHex as c_int;
    let fmt = b"%02x\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, promoted);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    // C: char result = data + 1;
    // In C, `data + 1` is computed in `int` (after integer promotion of data),
    // then the assignment to `char` truncates back. This is equivalent to a
    // wrapping add in the underlying char's representation.
    let result: c_char = data.wrapping_add(1);
    printHexCharLine(result);
}
