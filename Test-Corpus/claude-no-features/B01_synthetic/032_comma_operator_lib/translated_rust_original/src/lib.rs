use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let fmt = b"%d %d\n\0".as_ptr() as *const std::ffi::c_char;
    let mut i: c_int = 0;
    let mut j: c_int = 0;
    while i < x {
        unsafe {
            libc::printf(fmt, i, j);
        }
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}
