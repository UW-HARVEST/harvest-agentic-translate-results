use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    for byte in x.to_ne_bytes() {
        unsafe {
            printf(c"%02x".as_ptr(), c_int::from(byte));
        }
    }

    unsafe {
        printf(c"\n".as_ptr());
    }
}
