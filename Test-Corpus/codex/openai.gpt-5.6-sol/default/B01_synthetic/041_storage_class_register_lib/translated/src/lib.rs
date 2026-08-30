use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const DECIMAL_LINE_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let y = x.wrapping_mul(2).wrapping_add(300);

    // Match the C library's formatting and stdio buffering exactly.
    unsafe {
        printf(DECIMAL_LINE_FORMAT.as_ptr().cast(), y);
    }
}
