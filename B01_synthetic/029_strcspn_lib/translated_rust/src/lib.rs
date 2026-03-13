use std::ffi::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    unsafe {
        libc::printf(b"%zu\n\0".as_ptr() as *const c_char, libc::strcspn(s1, s2));
    }
}
