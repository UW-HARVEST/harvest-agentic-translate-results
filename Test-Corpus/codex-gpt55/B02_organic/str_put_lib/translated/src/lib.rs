use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn str_put(num: c_int) {
    unsafe {
        printf(c"a %d\n".as_ptr(), num);
    }
}
