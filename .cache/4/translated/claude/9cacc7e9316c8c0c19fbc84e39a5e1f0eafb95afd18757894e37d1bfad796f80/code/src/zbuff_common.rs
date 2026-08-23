//! Translation of deprecated/zbuff_common.c
#![allow(non_snake_case)]

use crate::error_private::*;

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_isError(errorCode: usize) -> core::ffi::c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_getErrorName(errorCode: usize) -> *const core::ffi::c_char {
    ERR_getErrorName(errorCode)
}
