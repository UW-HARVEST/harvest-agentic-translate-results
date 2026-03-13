use std::ffi::c_double;

extern "C" {
    fn printf(fmt: *const std::ffi::c_char, ...) -> std::ffi::c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    let u: u64 = f.to_bits();
    unsafe {
        printf(b"%llx %a %.4f\n\0".as_ptr().cast(), u, f, f);
    }
}
