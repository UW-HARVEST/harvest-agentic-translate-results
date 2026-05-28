use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut j: c_int = 0;
    let mut i: c_int = 0;
    while i < x {
        unsafe {
            libc::printf(b"%d %d\n\0".as_ptr() as *const libc::c_char, i, j);
        }
        i += 1;
        j += 2;
    }
}
