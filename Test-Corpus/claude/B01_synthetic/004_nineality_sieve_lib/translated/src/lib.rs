// Rust translation of the C library in c_src/.
//
// Public ABI (from `nm -D` on the C shared library):
//   T sieve
//
// The C implementation (c_src/src/sieve.c):
//
//   void sieve(int val) {
//       while (1) {
//           printf("%d\n", val);
//           if (val % 10 == 9) {
//               break;
//           }
//           val++;
//       }
//   }
//
// Behaviour notes preserved verbatim:
//   * The value is printed *before* the terminating condition is checked, so at
//     least one line is always emitted.
//   * C's `%` truncates toward zero, hence for negative values `val % 10` is
//     negative or zero and can never equal 9. Negative starting points
//     therefore keep counting up until they reach 9 itself.
//   * Incrementing past INT_MAX is undefined behaviour in C; on the usual
//     two's-complement targets it wraps to INT_MIN, which is what
//     `wrapping_add` reproduces here (and it must not panic).
//   * Output goes through C `printf` so that stdout buffering, flushing and the
//     exact byte stream match the C library.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `void sieve(int val);`
///
/// Count from a starting point, stopping when the count ends in 9 (base 10).
#[unsafe(no_mangle)]
pub extern "C" fn sieve(val: c_int) {
    let mut val: c_int = val;
    loop {
        // printf("%d\n", val);
        unsafe {
            printf(b"%d\n\0".as_ptr() as *const c_char, val);
        }
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }
}
