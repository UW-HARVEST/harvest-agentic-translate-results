use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    // Use libc::printf to match C's stdio buffering behavior
    // and produce byte-identical output.
    let fmt = b"Hello World!\n\0".as_ptr() as *const i8;
    unsafe {
        libc::printf(fmt);
    }
    0
}
