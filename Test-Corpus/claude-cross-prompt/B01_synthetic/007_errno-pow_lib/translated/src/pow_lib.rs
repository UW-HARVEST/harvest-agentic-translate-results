// Translated from c_src/src/pow.c

use std::io::Write;

/// Replicates C's `pow(base, exponent)` semantics, including the
/// errno-based domain/range error reporting from the original C code.
///
/// Takes two arguments, a base and an exponent, and returns base^exponent.
pub fn my_pow(base: f64, exponent: f64) -> f64 {
    // Compute power using libm-equivalent semantics.
    let result = base.powf(exponent);

    // Detect domain error: result is NaN due to invalid arguments
    // (e.g. negative base with non-integer exponent, or pow(0, negative)
    // when result would be infinite -- though that's typically ERANGE).
    let is_domain_error = result.is_nan()
        || (base < 0.0 && exponent.fract() != 0.0)
        || (base == 0.0 && exponent < 0.0 && exponent.fract() != 0.0);

    if is_domain_error {
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(
            handle,
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base, exponent
        );
        return -1.0;
    }

    // Detect range error: result is infinite (overflow) or underflow
    // (subnormal/zero when the true result should be nonzero).
    let is_range_error = result.is_infinite()
        || (result == 0.0 && base != 0.0 && exponent != 0.0 && !(base == 1.0 || base == -1.0));

    if is_range_error {
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(
            handle,
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        return -1.0;
    }

    result
}
