#![allow(non_snake_case)]

use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

/// Count from a starting point,
/// stopping when the count ends in 9 (base 10).
#[unsafe(no_mangle)]
pub extern "C" fn sieve(val: c_int) {
    let mut val = val;
    loop {
        unsafe {
            printf(b"%d\n\0".as_ptr(), val);
        }
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }
}
