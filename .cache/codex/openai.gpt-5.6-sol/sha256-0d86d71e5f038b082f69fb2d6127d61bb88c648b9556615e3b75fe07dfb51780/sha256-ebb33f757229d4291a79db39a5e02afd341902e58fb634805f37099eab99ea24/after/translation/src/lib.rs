use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn sieve(mut val: c_int) {
    loop {
        unsafe {
            printf(c"%d\n".as_ptr(), val);
        }

        if val % 10 == 9 {
            break;
        }

        val = val.wrapping_add(1);
    }
}
