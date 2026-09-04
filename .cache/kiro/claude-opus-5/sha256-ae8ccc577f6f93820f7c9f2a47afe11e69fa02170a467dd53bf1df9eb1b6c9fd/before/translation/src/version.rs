//! Translation of `src/version.c`.

use crate::jtypes::{
    JANSSON_MAJOR_VERSION, JANSSON_MICRO_VERSION, JANSSON_MINOR_VERSION, JANSSON_VERSION,
};
use core::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn jansson_version_str() -> *const c_char {
    JANSSON_VERSION.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn jansson_version_cmp(major: c_int, minor: c_int, micro: c_int) -> c_int {
    let mut diff = JANSSON_MAJOR_VERSION - major;
    if diff != 0 {
        return diff;
    }

    diff = JANSSON_MINOR_VERSION - minor;
    if diff != 0 {
        return diff;
    }

    JANSSON_MICRO_VERSION - micro
}
