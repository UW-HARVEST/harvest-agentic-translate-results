use std::ffi::{c_char, c_double, c_int, c_ulonglong};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    let bits = f.to_bits() as c_ulonglong;
    let format = b"%llx %a %.4f\n\0".as_ptr().cast::<c_char>();

    unsafe {
        printf(format, bits, f, f);
    }
}
