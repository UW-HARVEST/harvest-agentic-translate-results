use std::ffi::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> i32;
    fn strcspn(s: *const c_char, reject: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let fmt = b"%zu\n\0".as_ptr() as *const c_char;
    printf(fmt, strcspn(s1, s2));
}
