#![allow(non_snake_case)]

use std::ffi::c_int;
use std::os::raw::c_char;
use std::sync::Mutex;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

static SUM: Mutex<c_int> = Mutex::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    let mut sum = SUM.lock().unwrap();
    *sum = sum.wrapping_add(update);
    *sum
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    for i in 0..10i32 {
        let value = static_sum(i.wrapping_mul(stride));
        unsafe {
            printf(fmt, value);
        }
    }
}
