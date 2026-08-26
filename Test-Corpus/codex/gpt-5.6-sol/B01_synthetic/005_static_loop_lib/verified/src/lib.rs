use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static mut SUM: c_int = 0;
const DECIMAL_LINE: &[u8] = b"%d\n\0";

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    unsafe {
        SUM = SUM.wrapping_add(update);
        SUM
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    for i in (0 as c_int)..10 {
        let sum = static_sum(i.wrapping_mul(stride));
        unsafe {
            printf(DECIMAL_LINE.as_ptr().cast(), sum);
        }
    }
}
