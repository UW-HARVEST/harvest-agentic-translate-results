use std::ffi::c_double;

extern "C" {
    fn printf(fmt: *const i8, ...) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    let x: u64 = f.to_bits();
    unsafe {
        printf(b"%llx %a %.4f\n\0".as_ptr() as *const i8, x, f, f);
    }
}
