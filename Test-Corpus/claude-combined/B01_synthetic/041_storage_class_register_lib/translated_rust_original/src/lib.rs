use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const libc::c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = 2 * x;
    y += 300;
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const libc::c_char, y);
    }
}
