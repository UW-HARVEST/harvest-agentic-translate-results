//! Translation of `c_src/src/pow.c`.

use std::ffi::c_double;

use crate::ffi::{self, EDOM, ERANGE};

/// Takes two arguments, a base and an exponent, and returns base^exponent.
///
/// Mirrors the C original exactly, including the order of the `errno` checks
/// and the `-1` returned on either error.
#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    // Calculate power
    ffi::set_errno(0);
    ffi::barrier();

    // SAFETY: calling libm's `pow` with two `double` arguments.
    let result = unsafe { ffi::pow(base, exponent) };

    ffi::barrier();

    if ffi::errno() == EDOM {
        // SAFETY: `stderr` is a valid stream and the variadic arguments match
        // the two `%.2f` conversions in the format string.
        unsafe {
            ffi::fprintf(
                ffi::stderr,
                c"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n"
                    .as_ptr(),
                base,
                exponent,
            );
        }
        return -1.0;
    } else if ffi::errno() == ERANGE {
        // SAFETY: as above.
        unsafe {
            ffi::fprintf(
                ffi::stderr,
                c"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n".as_ptr(),
                base,
                exponent,
            );
        }
        return -1.0;
    }

    result
}
