/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

//! Rust translation of `c_src/src/deprecated/zbuff_common.c`.

use crate::common::error_private::{ERR_getErrorName, ERR_isError};
use crate::common::mem::size_t;

use core::ffi::{c_char, c_uint};

/*-****************************************
*  ZBUFF Error Management  (deprecated)
******************************************/

/* ZBUFF_isError() :
*   tells if a return value is an error code */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_isError(errorCode: size_t) -> c_uint {
    ERR_isError(errorCode)
}

/* ZBUFF_getErrorName() :
*   provides error code string from function result (useful for debugging) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_getErrorName(errorCode: size_t) -> *const c_char {
    ERR_getErrorName(errorCode)
}
