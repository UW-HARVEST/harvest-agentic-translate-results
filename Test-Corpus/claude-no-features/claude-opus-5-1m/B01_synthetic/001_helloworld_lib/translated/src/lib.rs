use std::ffi::c_int;

extern "C" {
    fn printf(format: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    unsafe {
        printf(b"Hello World!\n\0".as_ptr());
    }
    0
}
