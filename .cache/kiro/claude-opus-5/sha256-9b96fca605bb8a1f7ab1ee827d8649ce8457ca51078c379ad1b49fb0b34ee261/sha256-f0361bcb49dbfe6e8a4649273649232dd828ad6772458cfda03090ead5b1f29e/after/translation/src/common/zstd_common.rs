//! Translation of `common/zstd_common.c`.
#![allow(dead_code)]

use super::error_private::*;
use super::mem::size_t;
use super::zstd_internal::{ZSTD_VERSION_NUMBER, ZSTD_VERSION_STRING};
use core::ffi::{c_char, c_uint};

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionNumber() -> c_uint {
    ZSTD_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionString() -> *const c_char {
    ZSTD_VERSION_STRING.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_isError(code: size_t) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorCode(code: size_t) -> ZSTD_ErrorCode {
    ERR_getErrorCode(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorString(code: ZSTD_ErrorCode) -> *const c_char {
    ERR_getErrorString(code)
}
