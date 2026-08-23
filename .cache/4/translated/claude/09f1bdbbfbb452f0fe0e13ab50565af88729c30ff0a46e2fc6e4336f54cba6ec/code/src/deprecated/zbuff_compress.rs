//! Translation of `deprecated/zbuff_compress.c`
#![allow(dead_code)]

use crate::common::error_private::*;
use crate::zstd_h::*;
use core::ffi::{c_int, c_void};

/* `ZBUFF_CCtx` is `ZSTD_CStream` == `ZSTD_CCtx` */
type ZBUFF_CCtx = c_void;

extern "C" {
    fn ZSTD_createCStream() -> *mut ZBUFF_CCtx;
    fn ZSTD_createCStream_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_CCtx;
    fn ZSTD_freeCStream(zcs: *mut ZBUFF_CCtx) -> usize;
    fn ZSTD_CCtx_reset(cctx: *mut ZBUFF_CCtx, reset: ZSTD_ResetDirective) -> usize;
    fn ZSTD_CCtx_setPledgedSrcSize(cctx: *mut ZBUFF_CCtx, pledgedSrcSize: u64) -> usize;
    fn ZSTD_checkCParams(cParams: ZSTD_compressionParameters) -> usize;
    fn ZSTD_CCtx_setParameter(
        cctx: *mut ZBUFF_CCtx,
        param: ZSTD_cParameter,
        value: c_int,
    ) -> usize;
    fn ZSTD_CCtx_loadDictionary(
        cctx: *mut ZBUFF_CCtx,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZSTD_initCStream(zcs: *mut ZBUFF_CCtx, compressionLevel: c_int) -> usize;
    fn ZSTD_compressStream(
        zcs: *mut ZBUFF_CCtx,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> usize;
    fn ZSTD_flushStream(zcs: *mut ZBUFF_CCtx, output: *mut ZSTD_outBuffer) -> usize;
    fn ZSTD_endStream(zcs: *mut ZBUFF_CCtx, output: *mut ZSTD_outBuffer) -> usize;
    fn ZSTD_CStreamInSize() -> usize;
    fn ZSTD_CStreamOutSize() -> usize;
}

macro_rules! FORWARD_IF_ERROR {
    ($e:expr) => {{
        let err_code: usize = $e;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }};
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createCCtx() -> *mut ZBUFF_CCtx {
    ZSTD_createCStream()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_CCtx {
    ZSTD_createCStream_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeCCtx(zbc: *mut ZBUFF_CCtx) -> usize {
    ZSTD_freeCStream(zbc)
}

/* ======   Initialization   ====== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit_advanced(
    zbc: *mut ZBUFF_CCtx,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    mut pledgedSrcSize: u64,
) -> usize {
    if pledgedSrcSize == 0 {
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setPledgedSrcSize(zbc, pledgedSrcSize));

    FORWARD_IF_ERROR!(ZSTD_checkCParams(params.cParams));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_windowLog,
        params.cParams.windowLog as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_hashLog,
        params.cParams.hashLog as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_chainLog,
        params.cParams.chainLog as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_searchLog,
        params.cParams.searchLog as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_minMatch,
        params.cParams.minMatch as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_targetLength,
        params.cParams.targetLength as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_strategy,
        params.cParams.strategy as c_int
    ));

    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_contentSizeFlag,
        params.fParams.contentSizeFlag
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_checksumFlag,
        params.fParams.checksumFlag
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_dictIDFlag,
        params.fParams.noDictIDFlag
    ));

    FORWARD_IF_ERROR!(ZSTD_CCtx_loadDictionary(zbc, dict, dictSize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInitDictionary(
    zbc: *mut ZBUFF_CCtx,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        zbc,
        ZSTD_c_compressionLevel,
        compressionLevel
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_loadDictionary(zbc, dict, dictSize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit(
    zbc: *mut ZBUFF_CCtx,
    compressionLevel: c_int,
) -> usize {
    ZSTD_initCStream(zbc, compressionLevel)
}

/* ======   Compression   ====== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressContinue(
    zbc: *mut ZBUFF_CCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let result: usize;
    let mut outBuff = ZSTD_outBuffer {
        dst,
        size: *dstCapacityPtr,
        pos: 0,
    };
    let mut inBuff = ZSTD_inBuffer {
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
    dstCapacityPtr: *mut usize,
) -> usize {
    let result: usize;
    let mut outBuff = ZSTD_outBuffer {
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
    dstCapacityPtr: *mut usize,
) -> usize {
    let result: usize;
    let mut outBuff = ZSTD_outBuffer {
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
pub unsafe extern "C" fn ZBUFF_recommendedCInSize() -> usize {
    ZSTD_CStreamInSize()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedCOutSize() -> usize {
    ZSTD_CStreamOutSize()
}
