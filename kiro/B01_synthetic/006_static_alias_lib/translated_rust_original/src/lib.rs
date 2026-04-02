use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

static mut INNER: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    unsafe {
        if *outer >= INNER {
            INNER += *outer;
            &raw mut INNER
        } else {
            *outer += INNER;
            outer
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    unsafe {
        let mut val = initial_value;
        let mut running_sum: *mut c_int = &raw mut val;
        for _ in 0..iterations {
            running_sum = static_alias(running_sum);
            printf(b"%d\n\0".as_ptr(), *running_sum);
        }
    }
}
