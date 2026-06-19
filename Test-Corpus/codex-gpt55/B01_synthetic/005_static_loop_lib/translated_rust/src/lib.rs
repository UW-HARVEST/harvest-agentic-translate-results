use std::ffi::{c_char, c_int};

static mut SUM: c_int = 0;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    unsafe {
        SUM = SUM.wrapping_add(update);
        SUM
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    for i in 0..10 {
        let update = (i as c_int).wrapping_mul(stride);
        let value = static_sum(update);
        unsafe {
            printf(c"%d\n".as_ptr(), value);
        }
    }
}
