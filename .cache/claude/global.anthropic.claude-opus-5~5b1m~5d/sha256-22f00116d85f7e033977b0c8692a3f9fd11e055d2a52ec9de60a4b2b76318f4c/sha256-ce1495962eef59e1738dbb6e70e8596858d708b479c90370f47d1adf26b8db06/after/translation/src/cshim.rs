//! Declarations of the C standard library entry points that the original C
//! sources rely upon.  Re-using the platform C library (instead of a Rust
//! re-implementation) is what guarantees byte identical formatting of numbers,
//! identical `strtod` rounding, identical locale handling and identical stdio
//! buffering behaviour.

use core::ffi::{c_char, c_int, c_void};

/// Minimal view of `struct lconv`.  Only the first member (`decimal_point`) is
/// ever read by cJSON, and `repr(C)` guarantees it lives at offset 0.
#[repr(C)]
pub struct lconv {
    pub decimal_point: *mut c_char,
}

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;

    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    pub fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    pub fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64;
    pub fn tolower(c: c_int) -> c_int;
    pub fn localeconv() -> *mut lconv;

    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    pub fn printf(fmt: *const c_char, ...) -> c_int;

    pub fn exit(code: c_int) -> !;
}

/// `(int)double` with the semantics of the C cast as implemented on the
/// platforms the C library targets (x86-64 `cvttsd2si`): out-of-range values and
/// NaN produce `INT_MIN`.  Rust's `as` conversion saturates instead, so the
/// difference is made explicit here.
#[inline]
pub fn double_to_int(value: f64) -> c_int {
    if value.is_nan() || value >= 2147483648.0f64 || value < -2147483648.0f64 {
        c_int::MIN
    } else {
        value as c_int
    }
}
