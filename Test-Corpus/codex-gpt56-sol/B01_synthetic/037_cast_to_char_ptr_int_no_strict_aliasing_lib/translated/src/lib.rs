use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const HEX_FORMAT: &[u8] = b"%02x\0";
const NEWLINE_FORMAT: &[u8] = b"\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    for byte in x.to_ne_bytes() {
        unsafe {
            printf(HEX_FORMAT.as_ptr().cast(), byte as c_int);
        }
    }

    unsafe {
        printf(NEWLINE_FORMAT.as_ptr().cast());
    }
}
