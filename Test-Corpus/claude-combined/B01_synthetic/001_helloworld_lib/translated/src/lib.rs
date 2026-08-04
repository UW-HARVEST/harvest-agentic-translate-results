use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    // Use libc::printf to match the C implementation byte-for-byte (including
    // the same stdout buffering behavior).
    unsafe {
        libc::printf(b"Hello World!\n\0".as_ptr() as *const libc::c_char);
    }
    0
}
