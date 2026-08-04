use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static DECIMAL_LINE_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn sieve(mut val: c_int) {
    loop {
        unsafe {
            printf(DECIMAL_LINE_FORMAT.as_ptr().cast(), val);
        }

        if val % 10 == 9 {
            break;
        }

        val += 1;
    }
}
