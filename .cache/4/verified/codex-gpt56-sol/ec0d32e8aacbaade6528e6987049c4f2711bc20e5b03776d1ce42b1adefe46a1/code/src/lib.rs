use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const HELLO_WORLD: &[u8] = b"Hello World!\n\0";

#[no_mangle]
pub extern "C" fn helloworld() -> c_int {
    unsafe {
        printf(HELLO_WORLD.as_ptr().cast());
    }
    0
}

#[no_mangle]
pub extern "C" fn main() -> c_int {
    helloworld()
}
