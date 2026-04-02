use std::ffi::c_char;

unsafe extern "C" {
    fn strcspn(s1: *const c_char, s2: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    unsafe {
        printf(b"%zu\n\0".as_ptr() as *const c_char, strcspn(s1, s2));
    }
}
