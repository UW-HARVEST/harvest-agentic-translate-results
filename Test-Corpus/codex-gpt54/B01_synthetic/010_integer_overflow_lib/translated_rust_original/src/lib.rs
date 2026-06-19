#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static PRINT_HEX_CHAR_LINE_FORMAT: &[u8] = b"%02x\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(charHex: c_char) {
    unsafe {
        // Mirror C default argument promotion from char to int for printf varargs.
        printf(
            PRINT_HEX_CHAR_LINE_FORMAT.as_ptr().cast(),
            c_int::from(charHex),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result = data.wrapping_add(1);
    printHexCharLine(result);
}
