use std::ffi::{c_char, c_double, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const FORMAT: &[u8] = b"%llx %a %.4f\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    unsafe {
        printf(FORMAT.as_ptr().cast(), f.to_bits(), f, f);
    }
}
