//! Translation of version.c
use crate::types::*;
use core::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jansson_version_str() -> *const c_char {
    JANSSON_VERSION.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jansson_version_cmp(major: c_int, minor: c_int, micro: c_int) -> c_int {
    let diff = JANSSON_MAJOR_VERSION - major;
    if diff != 0 {
        return diff;
    }

    let diff = JANSSON_MINOR_VERSION - minor;
    if diff != 0 {
        return diff;
    }

    JANSSON_MICRO_VERSION - micro
}
