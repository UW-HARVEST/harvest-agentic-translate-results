//! Translation of `src/version.c`.

use core::ffi::{c_char, c_int};

pub const JANSSON_MAJOR_VERSION: c_int = 2;
pub const JANSSON_MINOR_VERSION: c_int = 15;
pub const JANSSON_MICRO_VERSION: c_int = 0;

const JANSSON_VERSION: &[u8; 8] = b"2.15.0\0\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jansson_version_str() -> *const c_char {
    JANSSON_VERSION.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jansson_version_cmp(major: c_int, minor: c_int, micro: c_int) -> c_int {
    let mut diff: c_int;

    diff = JANSSON_MAJOR_VERSION - major;
    if diff != 0 {
        return diff;
    }

    diff = JANSSON_MINOR_VERSION - minor;
    if diff != 0 {
        return diff;
    }

    JANSSON_MICRO_VERSION - micro
}
