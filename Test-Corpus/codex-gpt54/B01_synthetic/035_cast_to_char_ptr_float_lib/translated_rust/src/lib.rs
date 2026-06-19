use std::ffi::{c_char, c_int, c_uint};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
}

const PRINTF_HEX_FORMAT: &[u8] = b"%02x\0";

fn print_hex(bytes: &[u8]) {
    for &byte in bytes {
        unsafe {
            printf(PRINTF_HEX_FORMAT.as_ptr().cast(), c_uint::from(byte));
        }
    }

    unsafe {
        putchar('\n' as c_int);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}
