use std::ffi::c_int;

static mut INNER: c_int = 1;

/// # Safety
/// Caller must ensure `outer` is a valid, aligned pointer to a mutable c_int.
/// The returned pointer is valid until the next call that may invalidate it.
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
    let mut running_sum_val = initial_value;
    let mut running_sum: *mut c_int = &mut running_sum_val;
    for _ in 0..iterations {
        running_sum = static_alias(running_sum);
        unsafe {
            libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, *running_sum);
        }
    }
}
