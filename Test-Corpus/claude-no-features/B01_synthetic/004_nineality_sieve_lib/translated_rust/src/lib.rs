// Translated from c_src/src/sieve.c

use std::ffi::c_int;

/// Count from a starting point,
/// stopping when the count ends in 9 (base 10).
#[unsafe(no_mangle)]
pub extern "C" fn sieve(val: c_int) {
    let mut val = val;
    loop {
        // Use libc::printf to ensure byte-identical output to the C version.
        unsafe {
            libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, val);
        }
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }
}
