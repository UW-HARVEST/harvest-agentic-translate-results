use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

static mut INNER: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    unsafe {
        let inner_ptr: *mut c_int = &raw mut INNER;
        if *outer >= *inner_ptr {
            *inner_ptr += *outer;
            inner_ptr
        } else {
            *outer += *inner_ptr;
            outer
        }
    }
}

/*
  Maintain a sum leveraging multiple references to a static variable
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &raw mut initial_value;
    let mut i: c_int = 0;
    while i < iterations {
        unsafe {
            running_sum = static_alias(running_sum);
            printf(b"%d\n\0".as_ptr(), *running_sum);
        }
        i += 1;
    }
}
