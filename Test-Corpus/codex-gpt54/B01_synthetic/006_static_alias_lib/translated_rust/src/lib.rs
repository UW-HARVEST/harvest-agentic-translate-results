use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static mut INNER: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    let inner = &raw mut INNER;

    unsafe {
        if *outer >= *inner {
            *inner += *outer;
            inner
        } else {
            *outer += *inner;
            outer
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &mut initial_value;

    for _ in 0..iterations {
        unsafe {
            running_sum = static_alias(running_sum);
            printf(c"%d\n".as_ptr(), *running_sum);
        }
    }
}
