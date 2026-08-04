use std::ffi::c_int;

const DECIMAL_NEWLINE_NUL: &[u8] = b"%d\n\0";

unsafe extern "C" {
    fn printf(format: *const i8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y = 2 * x;
    y += 300;

    // Match the C implementation's formatting and stdout behavior.
    unsafe {
        printf(DECIMAL_NEWLINE_NUL.as_ptr().cast(), y);
    }
}
