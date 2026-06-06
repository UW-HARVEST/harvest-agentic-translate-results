// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust - reproduces byte-identical output of original C library.

use std::ffi::c_char;
use std::ffi::c_int;

// CHAR_MAX for signed char on x86_64 Linux (where C `char` is signed).
const CHAR_MAX: i8 = i8::MAX;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// Format strings as C-style nul-terminated strings.
// "%s\n\0"
const FMT_LINE: &[u8] = b"%s\n\0";
// "%02x\n\0"
const FMT_HEX: &[u8] = b"%02x\n\0";

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_LINE.as_ptr() as *const c_char, line);
        }
    }
}

fn print_hex_char_line(char_hex: i8) {
    // In C, `char` passed to printf via varargs is promoted to int via the
    // default argument promotions. On x86_64 Linux `char` is signed, so the
    // value is sign-extended to int. Reproduce that exact behavior here by
    // sign-extending to c_int before passing as the variadic argument.
    let promoted: c_int = char_hex as c_int;
    unsafe {
        printf(FMT_HEX.as_ptr() as *const c_char, promoted);
    }
}

fn bad() {
    let data: i8;
    data = CHAR_MAX;
    if data > 0 {
        // `char result = data * 2;` -- C promotes `data` to int for the
        // multiplication then truncates back to `char` on assignment.
        // With CHAR_MAX = 127, 127 * 2 = 254, truncated to signed char = -2.
        let result: i8 = (data as c_int).wrapping_mul(2) as i8;
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8;
    data = 2;
    if data > 0 {
        let result: i8 = (data as c_int).wrapping_mul(2) as i8;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: i8;
    #[allow(unused_assignments)]
    {
        data = b' ' as i8;
    }
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: i8 = (data as c_int).wrapping_mul(2) as i8;
            print_hex_char_line(result);
        } else {
            let msg = b"data value is too large to perform arithmetic safely.\0";
            print_line(msg.as_ptr() as *const c_char);
        }
    }
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
