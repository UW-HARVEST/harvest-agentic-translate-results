//! Translation of `common/zstd_common.c`
#![allow(dead_code)]

use super::error_private::*;
use crate::zstd_h::*;
use core::ffi::c_char;

/*-****************************************
*  Version
******************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionNumber() -> u32 {
    ZSTD_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionString() -> *const c_char {
    ZSTD_VERSION_STRING.as_ptr() as *const c_char
}

/*-****************************************
*  ZSTD Error Management
******************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_isError(code: usize) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorCode(code: usize) -> i32 {
    ERR_getErrorCode(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorString(code: i32) -> *const c_char {
    ERR_getErrorString(code)
}
