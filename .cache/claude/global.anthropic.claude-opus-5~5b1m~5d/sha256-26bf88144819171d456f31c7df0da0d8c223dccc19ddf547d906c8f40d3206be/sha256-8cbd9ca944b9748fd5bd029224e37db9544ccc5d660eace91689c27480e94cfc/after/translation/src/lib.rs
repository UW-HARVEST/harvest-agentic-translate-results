// Rust translation of the MIT Lincoln Laboratory "Sieve" C library.
//
// Original C library (c_src/):
//   include/sieve.h : void sieve(int start);
//   src/sieve.c     : counts up from `val`, printing each value with
//                     printf("%d\n", val), stopping once the value ends in 9
//                     (base 10, i.e. val % 10 == 9 using C truncated modulo).
//
// Public ABI exported by the C shared library (nm -D libSieve.so): `sieve`.

// The library name mirrors the C target (libSieve.so).
#![allow(non_snake_case)]

use std::ffi::c_int;

unsafe extern "C" {
    // Use the platform C printf so that stdout buffering / flushing behaviour
    // (and therefore the exact byte stream produced) matches the C library.
    fn printf(fmt: *const std::ffi::c_char, ...) -> c_int;
}

/// Count from a starting point, stopping when the count ends in 9 (base 10).
///
/// Faithful translation of the C original, including its behaviour for negative
/// inputs (C's `%` truncates toward zero, so e.g. -9 % 10 == -9 != 9 and the
/// loop keeps counting up until it reaches 9).
#[unsafe(no_mangle)]
pub extern "C" fn sieve(val: c_int) {
    let mut val = val;
    loop {
        unsafe {
            printf(b"%d\n\0".as_ptr() as *const std::ffi::c_char, val);
        }
        if val % 10 == 9 {
            break;
        }
        // The C code performs `val++`; on signed overflow the C behaviour is
        // undefined but in practice wraps on the target platforms.
        val = val.wrapping_add(1);
    }
}
