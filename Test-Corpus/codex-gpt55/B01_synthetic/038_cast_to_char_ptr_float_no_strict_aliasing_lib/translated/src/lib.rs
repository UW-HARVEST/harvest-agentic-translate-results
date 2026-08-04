use std::ffi::{c_char, c_float, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const HEX_FORMAT: &[u8] = b"%02x\0";
const NEWLINE_FORMAT: &[u8] = b"\n\0";

fn print_hex(bytes: &[u8]) {
    for &byte in bytes {
        unsafe {
            printf(HEX_FORMAT.as_ptr().cast(), byte as c_int);
        }
    }

    unsafe {
        printf(NEWLINE_FORMAT.as_ptr().cast());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}
