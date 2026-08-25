use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    const MESSAGE: &[u8] = b"Hello World!\n\0";

    unsafe {
        printf(MESSAGE.as_ptr().cast());
    }

    0
}
