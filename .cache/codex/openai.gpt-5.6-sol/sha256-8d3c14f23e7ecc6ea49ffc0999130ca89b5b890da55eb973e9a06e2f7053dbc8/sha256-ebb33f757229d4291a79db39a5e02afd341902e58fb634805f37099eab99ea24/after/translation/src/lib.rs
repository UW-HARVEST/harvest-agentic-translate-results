use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut i: c_int = 0;

    while i < x {
        unsafe {
            printf(b"%d %d\n\0".as_ptr().cast(), i, i.wrapping_mul(2));
        }
        i += 1;
    }
}
