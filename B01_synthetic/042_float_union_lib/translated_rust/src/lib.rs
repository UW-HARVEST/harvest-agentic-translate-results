use std::ffi::c_double;

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    let x = f.to_bits();
    unsafe {
        libc::printf(b"%llx %a %.4f\n\0".as_ptr() as *const libc::c_char, x, f, f);
    }
}
