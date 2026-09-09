//! Translation of `deprecated/zbuff_common.c`
#![allow(dead_code)]

use core::ffi::{c_char, c_uint};

use crate::error_private::*;

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}
