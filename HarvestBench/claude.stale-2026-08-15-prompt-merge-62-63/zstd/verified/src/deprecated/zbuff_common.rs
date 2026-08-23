//! Translation of deprecated/zbuff_common.c
//! ZBUFF Error Management (deprecated).

use core::ffi::c_char;

use crate::common::error::{err_get_error_name, err_is_error};

/// ZBUFF_isError() :
/// tells if a return value is an error code
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_isError(errorCode: usize) -> core::ffi::c_uint {
    err_is_error(errorCode)
}

/// ZBUFF_getErrorName() :
/// provides error code string from function result (useful for debugging)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_getErrorName(errorCode: usize) -> *const c_char {
    err_get_error_name(errorCode)
}
