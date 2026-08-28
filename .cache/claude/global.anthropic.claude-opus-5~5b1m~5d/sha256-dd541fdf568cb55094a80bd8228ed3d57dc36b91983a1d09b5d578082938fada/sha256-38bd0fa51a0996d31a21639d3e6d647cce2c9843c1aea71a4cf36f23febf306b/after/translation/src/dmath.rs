//! Translation of `calculate_with_doubles` from `c_src/src/lib.c`.

use core::ffi::c_int;

use crate::ffi;

/// Shared implementation so that `doubleneg` observes exactly the same result
/// the exported entry point returns.
#[inline]
pub fn calculate(a: c_int, b: c_int, c: c_int) -> f64 {
    let mut result = 0.0_f64;

    // Guarding on `b != 0` leaves `result` at 0.0 for a zero divisor rather
    // than producing an infinity.
    if b != 0 {
        result = f64::from(a) / f64::from(b);
    }

    // `c % 10` truncates toward zero in C, so a negative `c` gives a negative
    // exponent. Rust's `%` matches. libm's `pow` is called directly to keep the
    // floating point result bit-identical.
    let exponent = f64::from(c % 10);
    result *= unsafe { ffi::pow(10.0, exponent) };

    result
}

/// C: `double calculate_with_doubles(int a, int b, int c)`
#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> f64 {
    calculate(a, b, c)
}
