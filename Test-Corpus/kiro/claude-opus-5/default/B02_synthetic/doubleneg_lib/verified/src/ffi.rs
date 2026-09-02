//! Raw declarations of the C runtime functions used by the original library.
//!
//! The C sources rely on `printf` (stdio), `memchr` (string.h) and `pow`
//! (math.h). We bind to the very same libc/libm entry points instead of
//! re-implementing them so that formatting and floating point results are
//! bit-for-bit identical to the C build.

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...)`
    pub fn printf(format: *const c_char, ...) -> c_int;

    /// `void *memchr(const void *s, int c, size_t n)`
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;

    /// `double pow(double x, double y)`
    pub fn pow(x: f64, y: f64) -> f64;
}
