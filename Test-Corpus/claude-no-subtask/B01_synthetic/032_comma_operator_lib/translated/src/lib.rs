use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let fmt = b"%d %d\n\0".as_ptr();
    let mut i: c_int = 0;
    let mut j: c_int = 0;
    while i < x {
        unsafe {
            printf(fmt, i, j);
        }
        i += 1;
        j += 2;
    }
}
