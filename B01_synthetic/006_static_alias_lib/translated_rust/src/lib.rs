use std::ffi::c_int;

static mut INNER: c_int = 1;

#[unsafe(no_mangle)]
pub extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
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
pub extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut running_sum = initial_value;
    let mut ptr: *mut c_int = &mut running_sum;
    for _ in 0..iterations {
        ptr = static_alias(ptr);
        unsafe {
            libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, *ptr);
        }
    }
}
