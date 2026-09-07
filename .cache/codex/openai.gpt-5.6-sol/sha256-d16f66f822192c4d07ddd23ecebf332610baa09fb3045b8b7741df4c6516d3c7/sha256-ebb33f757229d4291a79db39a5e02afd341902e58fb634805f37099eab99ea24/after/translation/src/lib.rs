use std::ffi::{c_char, c_float, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const HEX_FORMAT: &[u8] = b"%02x\0";
const NEWLINE: &[u8] = b"\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    for byte in x.to_ne_bytes() {
        unsafe {
            printf(HEX_FORMAT.as_ptr().cast(), c_int::from(byte));
        }
    }

    unsafe {
        printf(NEWLINE.as_ptr().cast());
    }
}
