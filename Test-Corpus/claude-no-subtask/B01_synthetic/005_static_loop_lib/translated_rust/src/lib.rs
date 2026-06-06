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
    let fmt = b"%d\n\0".as_ptr() as *const i8;
    for i in 0..10 {
        let val = static_sum((i as c_int).wrapping_mul(stride));
        unsafe {
            libc::printf(fmt, val);
        }
    }
}
