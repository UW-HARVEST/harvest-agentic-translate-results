use std::ffi::{c_char, c_float, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn putchar(character: c_int) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    const HEX_FORMAT: &[u8] = b"%02x\0";

    for byte in x.to_ne_bytes() {
        unsafe {
            printf(HEX_FORMAT.as_ptr().cast(), c_int::from(byte));
        }
    }

    unsafe {
        putchar(c_int::from(b'\n'));
    }
}
