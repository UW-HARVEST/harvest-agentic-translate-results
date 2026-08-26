//! Translation of `deprecated/zbuff_common.c`
#![allow(dead_code)]

use crate::common::error_private::*;
use core::ffi::c_char;

/* ZBUFF_isError() */
#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_isError(errorCode: usize) -> u32 {
    ERR_isError(errorCode)
}

/* ZBUFF_getErrorName() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}
