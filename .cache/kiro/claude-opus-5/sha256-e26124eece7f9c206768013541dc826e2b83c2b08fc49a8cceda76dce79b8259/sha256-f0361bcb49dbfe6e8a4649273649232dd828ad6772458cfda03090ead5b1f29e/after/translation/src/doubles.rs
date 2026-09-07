//! Translation of `calculate_with_doubles`.

use core::ffi::c_int;

use crate::ffi;

/// ```c
/// double calculate_with_doubles(int a, int b, int c) {
///     double result = 0.0;
///
///     if (b != 0) {
///         result = (double)a / (double)b;
///     }
///
///     result *= pow(10.0, c % 10);
///
///     return result;
/// }
/// ```
///
/// Note that the multiplication happens unconditionally, so a zero `b` still
/// yields `0.0 * pow(...)`. `c % 10` truncates toward zero in C, so a negative
/// `c` produces a negative exponent.
#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> f64 {
    let mut result: f64 = 0.0;

    if b != 0 {
        result = f64::from(a) / f64::from(b);
    }

    let exponent = f64::from(c.wrapping_rem(10));
    result *= unsafe { ffi::pow(10.0, exponent) };

    result
}
