use std::ffi::{c_char, c_int};

static mut INNER: c_int = 1;
const INTEGER_LINE_FORMAT: &[u8] = b"%d\n\0";

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    let inner = &raw mut INNER;

    if unsafe { *outer >= *inner } {
        unsafe {
            *inner += *outer;
        }
        inner
    } else {
        unsafe {
            *outer += *inner;
        }
        outer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut initial_value = initial_value;
    let mut running_sum = &raw mut initial_value;
    let mut i = 0;

    while i < iterations {
        running_sum = unsafe { static_alias(running_sum) };
        unsafe {
            printf(INTEGER_LINE_FORMAT.as_ptr().cast::<c_char>(), *running_sum);
        }
        i += 1;
    }
}
