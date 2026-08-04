use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static HELLO_WORLD: &[u8] = b"Hello World!\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    unsafe {
        printf(HELLO_WORLD.as_ptr().cast());
    }
    0
}
