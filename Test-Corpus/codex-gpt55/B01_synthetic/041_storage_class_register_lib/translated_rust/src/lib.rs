use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y = x.wrapping_mul(2);
    y = y.wrapping_add(300);

    unsafe {
        printf(c"%d\n".as_ptr(), y);
    }
}
