use libc::printf;
use std::os::raw::{c_char, c_double};

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    let x = f.to_bits();
    unsafe {
        printf(b"%llx %a %.4f\n\0".as_ptr() as *const c_char, x, f, f);
    }
}