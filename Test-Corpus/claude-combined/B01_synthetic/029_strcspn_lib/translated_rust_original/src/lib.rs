use std::ffi::c_char;

extern "C" {
    fn strcspn(s1: *const c_char, s2: *const c_char) -> libc::size_t;
    fn printf(format: *const c_char, ...) -> libc::c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let fmt = b"%zu\n\0".as_ptr() as *const c_char;
    let n = strcspn(s1, s2);
    printf(fmt, n);
}
