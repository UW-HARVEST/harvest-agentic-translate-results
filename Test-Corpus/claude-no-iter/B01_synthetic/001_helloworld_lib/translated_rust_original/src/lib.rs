use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    // Use libc::printf to match the C implementation byte-for-byte,
    // including stdout buffering behavior.
    let fmt = b"Hello World!\n\0".as_ptr() as *const std::ffi::c_char;
    unsafe {
        libc::printf(fmt);
    }
    0
}
