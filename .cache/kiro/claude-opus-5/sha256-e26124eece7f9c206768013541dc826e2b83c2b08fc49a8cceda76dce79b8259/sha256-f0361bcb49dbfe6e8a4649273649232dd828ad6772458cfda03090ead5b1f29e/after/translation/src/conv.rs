//! Translation of `convert_double_to_int`.

use core::ffi::c_int;

/// Reproduce the exact behaviour of a C `(int)` cast from `double` on x86-64.
///
/// In C, converting a `double` whose truncated value does not fit in `int`
/// (including infinities and NaN) is undefined behaviour. The original library
/// deliberately exercises that case, and on x86-64 the compiler emits
/// `cvttsd2si`, which yields the "integer indefinite" value `0x80000000`
/// (`INT_MIN`) for every out-of-range or NaN input.
///
/// Rust's `as` cast saturates instead, so the C behaviour is emulated
/// explicitly here. Do not "fix" this: the observable output of the library
/// depends on it.
#[inline]
pub fn c_double_to_int(value: f64) -> c_int {
    if value.is_nan() {
        return c_int::MIN;
    }

    let truncated = value.trunc();

    // Both bounds are exactly representable as `f64`, so the comparison is
    // precise.
    if truncated >= -2147483648.0_f64 && truncated <= 2147483647.0_f64 {
        truncated as c_int
    } else {
        c_int::MIN
    }
}

/// ```c
/// int convert_double_to_int(double value) {
///     return (int)value;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: f64) -> c_int {
    c_double_to_int(value)
}
