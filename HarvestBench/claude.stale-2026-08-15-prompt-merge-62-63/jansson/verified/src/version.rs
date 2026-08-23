//! Translation of version.c
use crate::types::*;
use core::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jansson_version_str() -> *const c_char {
    JANSSON_VERSION.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jansson_version_cmp(major: c_int, minor: c_int, micro: c_int) -> c_int {
    // `wrapping_sub`, not `-`: the C computes plain `int` subtraction, which on
    // x86-64 gcc wraps. Rust's `-` panics on overflow when overflow-checks are
    // on (the default in debug builds), so a caller passing INT_MIN/INT_MAX
    // would abort the process instead of getting the C's wrapped value.
    let diff = JANSSON_MAJOR_VERSION.wrapping_sub(major);
    if diff != 0 {
        return diff;
    }

    let diff = JANSSON_MINOR_VERSION.wrapping_sub(minor);
    if diff != 0 {
        return diff;
    }

    JANSSON_MICRO_VERSION.wrapping_sub(micro)
}
