use core::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static DRIVER_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y = x.wrapping_mul(2);
    y = y.wrapping_add(300);

    unsafe {
        printf(DRIVER_FORMAT.as_ptr().cast::<c_char>(), y);
    }
}
