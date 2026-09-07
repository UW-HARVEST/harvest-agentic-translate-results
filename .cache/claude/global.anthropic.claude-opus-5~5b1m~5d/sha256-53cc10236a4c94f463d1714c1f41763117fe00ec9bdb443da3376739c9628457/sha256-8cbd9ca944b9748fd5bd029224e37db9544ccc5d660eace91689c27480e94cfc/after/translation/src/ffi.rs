//! Direct bindings to the C runtime routines that the original library relies
//! on.
//!
//! `lib.c` links against libc and libm (`target_link_libraries(... m)`), and it
//! produces all of its observable output through `printf`. Re-using the very
//! same `printf` implementation is what guarantees byte-identical output: the
//! `%e`/`%d`/`%ld` conversions, the handling of `-0.0`, `inf` and `nan`, and the
//! stdout buffering behaviour are then literally the same code that the C
//! library ran.
//!
//! Likewise `pow` is taken from libm rather than re-implemented so that the
//! floating point results are bit-for-bit what the C code produced.

use core::ffi::c_char;
use core::ffi::c_int;

extern "C" {
    /// C `int printf(const char *restrict format, ...)`.
    pub fn printf(format: *const c_char, ...) -> c_int;

    /// C `double pow(double x, double y)` from libm.
    pub fn pow(x: f64, y: f64) -> f64;
}

/// Helper for building the NUL-terminated format strings that get handed to
/// [`printf`]. The trailing `\0` is part of the literal so no allocation and no
/// runtime work is required.
macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const core::ffi::c_char
    };
}

pub(crate) use cstr;
