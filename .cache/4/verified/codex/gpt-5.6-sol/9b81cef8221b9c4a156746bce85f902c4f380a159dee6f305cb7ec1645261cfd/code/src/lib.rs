use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strcspn(string: *const c_char, rejected: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let length = unsafe { strcspn(s1, s2) };
    unsafe {
        printf(c"%zu\n".as_ptr(), length);
    }
}
