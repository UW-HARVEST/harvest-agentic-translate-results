//! Translation of `convert_double_to_int` from `c_src/src/lib.c`.

use core::ffi::c_int;

/// Reproduces the x86-64 `cvttsd2si` instruction that GCC emits for the C cast
/// `(int)value`.
///
/// In C, converting a `double` whose truncated value does not fit in an `int`
/// (or which is NaN) is undefined behaviour. The original library deliberately
/// exercises that UB, and on x86-64 the hardware answers with the "integer
/// indefinite" value `0x80000000` (`INT_MIN`). The C library's own output
/// confirms this: `INFINITY`, `NAN`, `-2^40` and `1e300` all print
/// `-2147483648`.
///
/// This is *not* what Rust's `as i32` does -- Rust saturates, so `1e300 as i32`
/// would yield `INT_MAX` and `NAN as i32` would yield `0`. We therefore emulate
/// the hardware explicitly instead of fixing up the original behaviour.
#[inline]
pub fn convert(value: f64) -> c_int {
    // NaN never compares in-range, but check explicitly for clarity.
    if value.is_nan() {
        return c_int::MIN;
    }

    // `cvttsd2si` truncates toward zero, then reports the indefinite value if
    // the truncated result is outside the destination range. Both bounds are
    // exactly representable as `f64`, so these comparisons are exact.
    let truncated = value.trunc();
    if truncated >= -2_147_483_648.0_f64 && truncated <= 2_147_483_647.0_f64 {
        truncated as c_int
    } else {
        c_int::MIN
    }
}

/// C: `int convert_double_to_int(double value)`
#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: f64) -> c_int {
    convert(value)
}
