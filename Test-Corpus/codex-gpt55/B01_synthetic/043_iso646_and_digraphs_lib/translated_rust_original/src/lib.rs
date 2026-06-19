use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(s: *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result = x | !y;

    unsafe {
        printf(c"%d".as_ptr(), result);
        puts(c"".as_ptr());
    }
}
