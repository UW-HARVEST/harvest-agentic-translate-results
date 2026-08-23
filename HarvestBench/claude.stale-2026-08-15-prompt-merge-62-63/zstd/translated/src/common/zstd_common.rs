//! Translation of common/zstd_common.c, debug.c, threading.c version/error glue.
#![allow(non_upper_case_globals)]

use super::error;
use core::ffi::{c_char, c_int, c_uint};

pub const ZSTD_VERSION_MAJOR: u32 = 1;
pub const ZSTD_VERSION_MINOR: u32 = 5;
pub const ZSTD_VERSION_RELEASE: u32 = 7;
pub const ZSTD_VERSION_NUMBER: u32 =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;

pub const FSE_VERSION_NUMBER: u32 = 0 * 100 * 100 + 9 * 100 + 0;

// debug.c global
#[unsafe(no_mangle)]
pub static mut g_debuglevel: c_int = 0;

// threading.c fake symbol
#[unsafe(no_mangle)]
pub static mut g_ZSTD_threading_useless_symbol: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionNumber() -> c_uint {
    ZSTD_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionString() -> *const c_char {
    b"1.5.7\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_isError(code: usize) -> c_uint {
    error::err_is_error(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorName(code: usize) -> *const c_char {
    error::err_get_error_name(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorCode(code: usize) -> c_int {
    error::err_get_error_code(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorString(code: c_int) -> *const c_char {
    error::ERR_getErrorString(code)
}
