//! Translation of `common/zstd_common.c`.
#![allow(dead_code)]

use core::ffi::{c_char, c_uint};

use crate::error_private::*;
use crate::zstd_h::*;

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionNumber() -> c_uint {
    ZSTD_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionString() -> *const c_char {
    ZSTD_VERSION_STRING.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorCode(code: usize) -> ZSTD_ErrorCode {
    ERR_getErrorCode(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorString(code: ZSTD_ErrorCode) -> *const c_char {
    ERR_getErrorString_impl(code)
}
