// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Preserves byte-identical output behavior.

use std::ffi::c_char;
use std::ffi::c_int;

// CHAR_MAX on Linux/glibc x86_64 is 127 (signed char).
const CHAR_MAX: c_char = 127;

/// Print a NUL-terminated C string followed by a newline. Mirrors C's
/// `printf("%s\n", line)` exactly by delegating to libc::printf.
fn print_line(line: *const c_char) {
    if !line.is_null() {
        // SAFETY: caller guarantees `line` is a valid NUL-terminated C string
        // when non-null. This matches the C behavior precisely.
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

/// Print a `char` value as two-digit hex followed by newline. Mirrors C's
/// `printf("%02x\n", charHex)` where `charHex` (a `char`) is promoted to
/// `int` per C's default argument promotions before being read by `%x` as
/// unsigned int. We replicate this by passing `c_int` (sign-extended from
/// the signed `c_char`).
fn print_hex_char_line(char_hex: c_char) {
    let promoted: c_int = char_hex as c_int;
    // SAFETY: format string is a valid NUL-terminated literal.
    unsafe {
        libc::printf(b"%02x\n\0".as_ptr() as *const c_char, promoted);
    }
}

fn bad() {
    let data: c_char;
    data = CHAR_MAX;
    if data > 0 {
        // C: `char result = data * 2;`
        // Multiplication occurs in `int` (default integer promotions), then
        // is converted back to `char`. For data = 127, result is 254 which
        // wraps to -2 in signed char. We replicate by performing the
        // computation in c_int and truncating with `as c_char`.
        let result: c_char = (data as c_int * 2) as c_char;
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: c_char;
    data = 2;
    if data > 0 {
        let result: c_char = (data as c_int * 2) as c_char;
        print_hex_char_line(result);
    }
}

#[allow(unused_assignments)]
fn good_b2g() {
    let mut data: c_char;
    data = b' ' as c_char;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: c_char = (data as c_int * 2) as c_char;
            print_hex_char_line(result);
        } else {
            // The original C string literal — must be byte-identical.
            print_line(b"data value is too large to perform arithmetic safely.\0".as_ptr() as *const c_char);
        }
    }
    // Suppress unused-assignment warning for the initial `data = ' '`.
    let _ = data;
}

fn good() {
    good_g2b();
    good_b2g();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
