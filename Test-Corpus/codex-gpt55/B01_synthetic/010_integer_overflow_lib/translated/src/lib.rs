use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const HEX_LINE_FORMAT: &[u8] = b"%02x\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    unsafe {
        printf(HEX_LINE_FORMAT.as_ptr().cast(), char_hex as c_int);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result = data.wrapping_add(1);
    printHexCharLine(result);
}
