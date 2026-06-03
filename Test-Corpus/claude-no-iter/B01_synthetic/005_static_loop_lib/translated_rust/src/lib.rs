#![allow(non_snake_case)]

use std::ffi::c_int;

static mut SUM: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    unsafe {
        SUM = SUM.wrapping_add(update);
        SUM
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    extern "C" {
        fn printf(fmt: *const std::ffi::c_char, ...) -> c_int;
    }
    let fmt = b"%d\n\0".as_ptr() as *const std::ffi::c_char;
    for i in 0..10i32 {
        let v = static_sum(i.wrapping_mul(stride));
        unsafe {
            printf(fmt, v);
        }
    }
}
