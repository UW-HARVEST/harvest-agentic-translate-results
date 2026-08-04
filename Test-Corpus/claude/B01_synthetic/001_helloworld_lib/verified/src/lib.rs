use std::ffi::c_int;
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    unsafe {
        printf(b"Hello World!\n\0".as_ptr() as *const c_char);
    }
    0
}
