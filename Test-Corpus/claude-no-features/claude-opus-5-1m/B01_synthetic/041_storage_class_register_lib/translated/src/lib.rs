use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    unsafe {
        let fmt = b"%d\n\0".as_ptr() as *const std::ffi::c_char;
        libc::printf(fmt, y);
    }
}
