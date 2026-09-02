/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

//! Rust translation of `c_src/src/deprecated/zbuff_compress.c`.
//!
//! `#define ZBUFF_STATIC_LINKING_ONLY`

use crate::common::error_private::ERR_isError;
use crate::common::mem::size_t;
use crate::common::zstd_h::{
    ZSTD_CONTENTSIZE_UNKNOWN, ZSTD_c_chainLog, ZSTD_c_checksumFlag, ZSTD_c_compressionLevel,
    ZSTD_c_contentSizeFlag, ZSTD_c_dictIDFlag, ZSTD_c_hashLog, ZSTD_c_minMatch, ZSTD_c_searchLog,
    ZSTD_c_strategy, ZSTD_c_targetLength, ZSTD_c_windowLog, ZSTD_inBuffer, ZSTD_outBuffer,
    ZSTD_parameters, ZSTD_reset_session_only,
};
use crate::common::zstd_internal::ZSTD_customMem;

use core::ffi::{c_int, c_ulonglong, c_void};

/* ZBUFF_CCtx is a typedef of ZSTD_CStream (== ZSTD_CCtx). We keep it opaque via
 * an incomplete-like unit type so the ABI matches C's `ZSTD_CStream*`. */
#[repr(C)]
pub struct ZBUFF_CCtx {
    _private: [u8; 0],
}

/* ======  Forward declarations of symbols owned by other translation units ======
 * These are all exported symbols of the same cdylib (defined in zstd_compress.c,
 * being written by a concurrent agent), so they will link once available. */
unsafe extern "C" {
    fn ZSTD_createCStream() -> *mut ZBUFF_CCtx;
    fn ZSTD_createCStream_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_CCtx;
    fn ZSTD_freeCStream(zcs: *mut ZBUFF_CCtx) -> size_t;
    fn ZSTD_initCStream(zcs: *mut ZBUFF_CCtx, compressionLevel: c_int) -> size_t;
    fn ZSTD_compressStream(
        zcs: *mut ZBUFF_CCtx,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> size_t;
    fn ZSTD_flushStream(zcs: *mut ZBUFF_CCtx, output: *mut ZSTD_outBuffer) -> size_t;
    fn ZSTD_endStream(zcs: *mut ZBUFF_CCtx, output: *mut ZSTD_outBuffer) -> size_t;
    fn ZSTD_CStreamInSize() -> size_t;
    fn ZSTD_CStreamOutSize() -> size_t;

    fn ZSTD_CCtx_reset(cctx: *mut ZBUFF_CCtx, reset: c_uint) -> size_t;
    fn ZSTD_CCtx_setPledgedSrcSize(cctx: *mut ZBUFF_CCtx, pledgedSrcSize: c_ulonglong) -> size_t;
    fn ZSTD_checkCParams(params: crate::common::zstd_h::ZSTD_compressionParameters) -> size_t;
    fn ZSTD_CCtx_setParameter(cctx: *mut ZBUFF_CCtx, param: c_int, value: c_int) -> size_t;
    fn ZSTD_CCtx_loadDictionary(
        cctx: *mut ZBUFF_CCtx,
        dict: *const c_void,
        dictSize: size_t,
    ) -> size_t;
}

use core::ffi::c_uint;

/* FORWARD_IF_ERROR(err, "") with DEBUGLEVEL==0 : evaluate err, if error return it. */
macro_rules! FORWARD_IF_ERROR {
    ($err:expr) => {{
        let err_code: size_t = ($err);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }};
}

/* ***********************************************************
*  Streaming compression
* ***********************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createCCtx() -> *mut ZBUFF_CCtx {
    ZSTD_createCStream()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_CCtx {
    ZSTD_createCStream_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeCCtx(zbc: *mut ZBUFF_CCtx) -> size_t {
    ZSTD_freeCStream(zbc)
}

/* ======   Initialization   ====== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit_advanced(
    zbc: *mut ZBUFF_CCtx,
    dict: *const c_void,
    dictSize: size_t,
    params: ZSTD_parameters,
    mut pledgedSrcSize: c_ulonglong,
) -> size_t {
    if pledgedSrcSize == 0 {
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    } /* preserve "0 == unknown" behavior */
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setPledgedSrcSize(zbc, pledgedSrcSize));

    FORWARD_IF_ERROR!(ZSTD_checkCParams(params.cParams));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_windowLog as c_int,
        params.cParams.windowLog as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_hashLog as c_int,
        params.cParams.hashLog as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_chainLog as c_int,
        params.cParams.chainLog as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_searchLog as c_int,
        params.cParams.searchLog as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_minMatch as c_int,
        params.cParams.minMatch as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_targetLength as c_int,
        params.cParams.targetLength as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_strategy as c_int,
        params.cParams.strategy as c_int
    ));

    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_contentSizeFlag as c_int,
        params.fParams.contentSizeFlag
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_checksumFlag as c_int,
        params.fParams.checksumFlag
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_dictIDFlag as c_int,
        params.fParams.noDictIDFlag
    ));

    FORWARD_IF_ERROR!(ZSTD_CCtx_loadDictionary(zbc, dict, dictSize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInitDictionary(
    zbc: *mut ZBUFF_CCtx,
    dict: *const c_void,
    dictSize: size_t,
    compressionLevel: c_int,
) -> size_t {
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_compressionLevel as c_int,
        compressionLevel
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_loadDictionary(zbc, dict, dictSize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit(zbc: *mut ZBUFF_CCtx, compressionLevel: c_int) -> size_t {
    ZSTD_initCStream(zbc, compressionLevel)
}

/* ======   Compression   ====== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressContinue(
    zbc: *mut ZBUFF_CCtx,
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
    result = ZSTD_compressStream(zbc, &mut outBuff, &mut inBuff);
    *dstCapacityPtr = outBuff.pos;
    *srcSizePtr = inBuff.pos;
    result
}

/* ======   Finalize   ====== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressFlush(
    zbc: *mut ZBUFF_CCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut size_t,
) -> size_t {
    let result: size_t;
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst,
        size: *dstCapacityPtr,
        pos: 0,
    };
    result = ZSTD_flushStream(zbc, &mut outBuff);
    *dstCapacityPtr = outBuff.pos;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressEnd(
    zbc: *mut ZBUFF_CCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut size_t,
) -> size_t {
    let result: size_t;
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst,
        size: *dstCapacityPtr,
        pos: 0,
    };
    result = ZSTD_endStream(zbc, &mut outBuff);
    *dstCapacityPtr = outBuff.pos;
    result
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedCInSize() -> size_t {
    ZSTD_CStreamInSize()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedCOutSize() -> size_t {
    ZSTD_CStreamOutSize()
}
