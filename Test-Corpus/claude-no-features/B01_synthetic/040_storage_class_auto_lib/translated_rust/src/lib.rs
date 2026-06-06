use std::ffi::c_int;
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = 2 * x;
    y += 300;
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, y);
    }
}
