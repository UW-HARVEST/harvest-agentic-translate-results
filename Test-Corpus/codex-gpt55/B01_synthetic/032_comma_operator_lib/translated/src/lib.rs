use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let format = b"%d %d\n\0";
    let mut i: c_int = 0;
    let mut j: c_int = 0;

    while i < x {
        unsafe {
            printf(format.as_ptr().cast::<c_char>(), i, j);
        }

        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}
