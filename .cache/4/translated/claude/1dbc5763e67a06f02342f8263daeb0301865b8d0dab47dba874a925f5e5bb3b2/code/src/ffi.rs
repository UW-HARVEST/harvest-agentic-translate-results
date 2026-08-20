// Raw C library bindings used to guarantee byte-identical formatting,
// identical stdio buffering, and an identical allocator to the original C
// implementation (pointers returned by `create_state` must be releasable by
// `free`, and vice-versa).

use core::ffi::{c_char, c_int, c_void};

extern "C" {
    pub fn printf(format: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn strlen(s: *const c_char) -> usize;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

/// Emulates the x86-64 `cvttss2si` instruction that gcc emits for a C
/// `(int)` cast of a `float`.
///
/// A C cast of an out-of-range / NaN float to `int` is undefined behaviour;
/// gcc on x86-64 lowers it to `cvttss2si`, which yields the "integer
/// indefinite" value `INT_MIN` when the truncated result is not representable.
/// Rust's `as` cast saturates instead, so it cannot be used directly.
#[inline]
pub fn cvttss2si(value: f32) -> c_int {
    if value.is_nan() || value >= 2147483648.0f32 || value < -2147483648.0f32 {
        c_int::MIN
    } else {
        value as c_int
    }
}
