use std::ffi::c_int;

unsafe extern "C" {
    fn printf(format: *const i8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let raw = x.to_ne_bytes();

    for byte in raw {
        unsafe {
            printf(c"%02x".as_ptr(), c_int::from(byte));
        }
    }
    unsafe {
        printf(c"\n".as_ptr());
    }
}
