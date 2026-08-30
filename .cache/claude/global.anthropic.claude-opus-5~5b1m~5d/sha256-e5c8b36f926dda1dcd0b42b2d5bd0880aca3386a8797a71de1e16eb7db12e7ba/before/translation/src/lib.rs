// Rust translation of the C library in c_src/.
//
// Original copyright 2025 MIT Lincoln Laboratory (see c_src/ for the full
// permission notice). This file reproduces the observable behavior of
// c_src/src/driver.c exactly, including its use of C stdio for output so that
// buffering and byte output match the original library.

use std::ffi::{c_char, c_float, c_int, c_uchar};

extern "C" {
    // Variadic C stdio printf, used so that output goes through the very same
    // FILE* stream (and hence the same buffering) as the original C code.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Translation of the C `static void print_hex(unsigned char *p, int len)`.
///
/// Not exported (it was `static` in C), but kept as a separate function to
/// mirror the original structure.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        printf(b"%02x\0".as_ptr() as *const c_char, *p.offset(i as isize) as c_int);
        i += 1;
    }
    printf(b"\n\0".as_ptr() as *const c_char);
}

/// Translation of the C `void driver(float x)`.
///
/// Copies the raw object representation of `x` into a local buffer and prints
/// it as lowercase hexadecimal, one byte at a time, followed by a newline.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [u8; std::mem::size_of::<c_float>()] = x.to_ne_bytes();

    // print_hex((unsigned char *)raw, sizeof(raw));
    unsafe {
        print_hex(raw.as_ptr() as *const c_uchar, raw.len() as c_int);
    }
}
