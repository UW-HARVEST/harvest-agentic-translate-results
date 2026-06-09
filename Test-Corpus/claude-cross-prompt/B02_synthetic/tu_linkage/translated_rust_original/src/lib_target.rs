// Translated from c_src/src/lib.c — defines the public `target` symbol.

use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn target(code: c_int) -> c_int {
    if code < 0 {
        return 7;
    }
    let m = code % 10;
    if m == 0 {
        return 0;
    }
    if m <= 3 {
        return 1;
    }
    if m <= 6 {
        return 2;
    }
    if m == 7 {
        return 3;
    }
    4
}
