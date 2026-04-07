use std::ffi::c_int;

static mut SUM: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    unsafe {
        SUM += update;
        SUM
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    for i in 0..10 {
        let result = static_sum(i * stride);
        unsafe {
            libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, result);
        }
    }
}
