use std::ffi::{c_char, c_int, c_uint};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    const HEX_FORMAT: &[u8] = b"%02x\0";
    const NEWLINE_FORMAT: &[u8] = b"\n\0";

    for &byte in bytes {
        unsafe {
            printf(HEX_FORMAT.as_ptr().cast::<c_char>(), byte as c_uint);
        }
    }

    unsafe {
        printf(NEWLINE_FORMAT.as_ptr().cast::<c_char>());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    print_hex(&x.to_ne_bytes());
}
