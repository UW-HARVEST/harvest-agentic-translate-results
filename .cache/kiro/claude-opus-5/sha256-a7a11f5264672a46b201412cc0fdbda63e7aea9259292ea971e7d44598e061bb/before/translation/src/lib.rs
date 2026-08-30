// Rust translation of c_src/src/pow.c
//
// Original C:
//   double my_pow(double base, double exponent);
//
// Behavior is reproduced exactly, including:
//   * clearing errno before the call to the C library's pow()
//   * checking EDOM first, then ERANGE (in that order)
//   * returning -1 on either error, after writing the diagnostic to stderr
//   * the diagnostic text, formatted with "%.2f"
//
// The math itself and the stderr formatting are delegated to the platform C
// library so that both the returned values and the emitted bytes are identical
// to the original program.

use std::ffi::{c_char, c_int, c_void};

// <errno.h> values on Linux.
const EDOM: c_int = 33;
const ERANGE: c_int = 34;

unsafe extern "C" {
    /// libm's pow(3); it is the one that sets errno the way the C code expects.
    #[link_name = "pow"]
    safe fn c_pow(base: f64, exponent: f64) -> f64;

    /// glibc's thread-local errno accessor (what the `errno` macro expands to).
    fn __errno_location() -> *mut c_int;

    /// The standard error stream object, as used by `fprintf(stderr, ...)`.
    static mut stderr: *mut c_void;

    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
}

#[inline]
fn errno() -> c_int {
    unsafe { *__errno_location() }
}

#[inline]
fn set_errno(value: c_int) {
    unsafe { *__errno_location() = value };
}

/// Writes `format` (a NUL-terminated C format string taking two doubles) to
/// stderr through the C library, so the bytes match the original exactly.
fn eprint_c(format: &[u8], base: f64, exponent: f64) {
    unsafe {
        let stream = stderr;
        fprintf(stream, format.as_ptr() as *const c_char, base, exponent);
    }
}

// Takes two arguments, a base and an exponent, and returns base^exponent
#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: f64, exponent: f64) -> f64 {
    // Calculate power
    set_errno(0);
    let result = c_pow(base, exponent);
    if errno() == EDOM {
        eprint_c(
            b"Domain error: pow(%.2f, %.2f) is undefined in the real number \
              domain.\n\0",
            base,
            exponent,
        );
        return -1.0;
    } else if errno() == ERANGE {
        eprint_c(
            b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0",
            base,
            exponent,
        );
        return -1.0;
    }

    result
}
