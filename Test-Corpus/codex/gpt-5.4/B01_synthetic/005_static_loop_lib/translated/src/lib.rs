use std::ffi::c_int;

unsafe extern "C" {
    fn printf(format: *const i8, ...) -> c_int;
}

static mut SUM: c_int = 0;
static FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_sum(update: c_int) -> c_int {
    unsafe {
        SUM = SUM.wrapping_add(update);
        SUM
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(stride: c_int) {
    for i in 0..10 {
        let sum = unsafe { static_sum((i as c_int).wrapping_mul(stride)) };
        let _ = unsafe { printf(FORMAT.as_ptr().cast(), sum) };
    }
}
