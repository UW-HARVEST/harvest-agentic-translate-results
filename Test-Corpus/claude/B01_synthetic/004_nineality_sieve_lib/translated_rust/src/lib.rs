// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from C source.

use std::ffi::c_int;

extern "C" {
    fn printf(format: *const u8, ...) -> c_int;
}

/// Count from a starting point,
/// stopping when the count ends in 9 (base 10).
#[unsafe(no_mangle)]
pub extern "C" fn sieve(val: c_int) {
    let mut val = val;
    let fmt = b"%d\n\0".as_ptr();
    loop {
        unsafe {
            printf(fmt, val);
        }
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }
}
