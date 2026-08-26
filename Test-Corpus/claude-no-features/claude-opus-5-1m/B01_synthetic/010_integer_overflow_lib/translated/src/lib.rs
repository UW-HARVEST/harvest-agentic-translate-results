// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output.

use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// Format string `"%02x\n\0"` — using a byte literal so it's nul-terminated.
const FMT: &[u8] = b"%02x\n\0";

fn print_hex_char_line(char_hex: c_char) {
    // In C, the `char` argument to printf gets default-promoted to `int` via
    // varargs. On platforms where `char` is signed (e.g. x86_64 Linux), this
    // sign-extends, so values 0x80..=0xff are printed as 8 hex digits
    // ("ffffff80".."ffffffff"). Replicate that exactly by passing the
    // sign-extended c_int to printf.
    let promoted: c_int = char_hex as c_int;
    unsafe {
        printf(FMT.as_ptr() as *const c_char, promoted);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    // C: `char result = data + 1;` — narrowing addition wraps as i8.
    let result: c_char = data.wrapping_add(1);
    print_hex_char_line(result);
}
