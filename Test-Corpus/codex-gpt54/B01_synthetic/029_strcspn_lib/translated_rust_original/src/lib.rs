use std::ffi::c_char;

static PRINTF_FMT: &[u8] = b"%zu\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let result = unsafe { libc::strcspn(s1, s2) };
    unsafe {
        libc::printf(PRINTF_FMT.as_ptr().cast(), result);
    }
}
