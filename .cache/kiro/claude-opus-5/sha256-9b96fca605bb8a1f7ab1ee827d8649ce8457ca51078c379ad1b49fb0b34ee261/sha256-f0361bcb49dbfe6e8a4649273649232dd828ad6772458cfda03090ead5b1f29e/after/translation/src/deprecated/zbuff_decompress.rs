/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

//! Rust translation of `c_src/src/deprecated/zbuff_decompress.c`.
//!
//! `#define ZSTD_DISABLE_DEPRECATE_WARNINGS`
//! `#define ZBUFF_STATIC_LINKING_ONLY`

use crate::common::mem::size_t;
use crate::common::zstd_h::{ZSTD_inBuffer, ZSTD_outBuffer};
use crate::common::zstd_internal::ZSTD_customMem;

use core::ffi::c_void;

/* ZBUFF_DCtx is a typedef of ZSTD_DStream (== ZSTD_DCtx). Opaque, matching the
 * C ABI `ZSTD_DStream*`. */
#[repr(C)]
pub struct ZBUFF_DCtx {
    _private: [u8; 0],
}

/* ======  Forward declarations of externally-defined exported symbols  ======
 * ZSTD_createDStream / ZSTD_freeDStream / ZSTD_initDStream / ZSTD_DStreamInSize /
 * ZSTD_DStreamOutSize / ZSTD_decompressStream are defined in the decompress
 * translation unit; ZSTD_createDStream_advanced / ZSTD_initDStream_usingDict too.
 * All are cdylib-exported symbols and link within the same library. */
unsafe extern "C" {
    fn ZSTD_createDStream() -> *mut ZBUFF_DCtx;
    fn ZSTD_createDStream_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_DCtx;
    fn ZSTD_freeDStream(zds: *mut ZBUFF_DCtx) -> size_t;
    fn ZSTD_initDStream(zds: *mut ZBUFF_DCtx) -> size_t;
    fn ZSTD_initDStream_usingDict(
        zds: *mut ZBUFF_DCtx,
        dict: *const c_void,
        dictSize: size_t,
    ) -> size_t;
    fn ZSTD_decompressStream(
        zds: *mut ZBUFF_DCtx,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> size_t;
    fn ZSTD_DStreamInSize() -> size_t;
    fn ZSTD_DStreamOutSize() -> size_t;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createDCtx() -> *mut ZBUFF_DCtx {
    ZSTD_createDStream()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createDCtx_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_DCtx {
    ZSTD_createDStream_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeDCtx(zbd: *mut ZBUFF_DCtx) -> size_t {
    ZSTD_freeDStream(zbd)
}

/* *** Initialization *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInitDictionary(
    zbd: *mut ZBUFF_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    ZSTD_initDStream_usingDict(zbd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInit(zbd: *mut ZBUFF_DCtx) -> size_t {
    ZSTD_initDStream(zbd)
}

/* *** Decompression *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressContinue(
    zbd: *mut ZBUFF_DCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut size_t,
    src: *const c_void,
    srcSizePtr: *mut size_t,
) -> size_t {
    let result: size_t;
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst,
        size: *dstCapacityPtr,
        pos: 0,
    };
    let mut inBuff: ZSTD_inBuffer = ZSTD_inBuffer {
        src,
        size: *srcSizePtr,
        pos: 0,
    };
    result = ZSTD_decompressStream(zbd, &mut outBuff, &mut inBuff);
    *dstCapacityPtr = outBuff.pos;
    *srcSizePtr = inBuff.pos;
    result
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedDInSize() -> size_t {
    ZSTD_DStreamInSize()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedDOutSize() -> size_t {
    ZSTD_DStreamOutSize()
}
