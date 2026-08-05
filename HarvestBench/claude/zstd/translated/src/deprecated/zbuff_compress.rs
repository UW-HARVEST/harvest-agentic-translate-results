//! Translation of deprecated/zbuff_compress.c
//! Deprecated buffered streaming compression API, a thin wrapper over the
//! public ZSTD_CStream (== ZSTD_CCtx) streaming API.

use core::ffi::{c_int, c_void};

use crate::common::allocations::ZSTD_customMem;
use crate::common::error::err_is_error;
use crate::zstd_h::{
    ZSTD_compressionParameters, ZSTD_inBuffer, ZSTD_outBuffer, ZSTD_parameters,
    ZSTD_reset_session_only, ZSTD_CONTENTSIZE_UNKNOWN,
};

/* ZBUFF_CCtx is a typedef of ZSTD_CStream (== ZSTD_CCtx) */
pub type ZBUFF_CCtx = c_void;

/* ZSTD_cParameter enum values (from public zstd.h) */
const ZSTD_c_compressionLevel: c_int = 100;
const ZSTD_c_windowLog: c_int = 101;
const ZSTD_c_hashLog: c_int = 102;
const ZSTD_c_chainLog: c_int = 103;
const ZSTD_c_searchLog: c_int = 104;
const ZSTD_c_minMatch: c_int = 105;
const ZSTD_c_targetLength: c_int = 106;
const ZSTD_c_strategy: c_int = 107;
const ZSTD_c_contentSizeFlag: c_int = 200;
const ZSTD_c_checksumFlag: c_int = 201;
const ZSTD_c_dictIDFlag: c_int = 202;

extern "C" {
    fn ZSTD_createCStream() -> *mut c_void;
    fn ZSTD_createCStream_advanced(customMem: ZSTD_customMem) -> *mut c_void;
    fn ZSTD_freeCStream(zcs: *mut c_void) -> usize;
    fn ZSTD_CCtx_reset(cctx: *mut c_void, reset: u32) -> usize;
    fn ZSTD_CCtx_setPledgedSrcSize(cctx: *mut c_void, pledgedSrcSize: u64) -> usize;
    fn ZSTD_checkCParams(params: ZSTD_compressionParameters) -> usize;
    fn ZSTD_CCtx_setParameter(cctx: *mut c_void, param: c_int, value: c_int) -> usize;
    fn ZSTD_CCtx_loadDictionary(cctx: *mut c_void, dict: *const c_void, dictSize: usize) -> usize;
    fn ZSTD_initCStream(zcs: *mut c_void, compressionLevel: c_int) -> usize;
    fn ZSTD_compressStream(
        zcs: *mut c_void,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> usize;
    fn ZSTD_flushStream(zcs: *mut c_void, output: *mut ZSTD_outBuffer) -> usize;
    fn ZSTD_endStream(zcs: *mut c_void, output: *mut ZSTD_outBuffer) -> usize;
    fn ZSTD_CStreamInSize() -> usize;
    fn ZSTD_CStreamOutSize() -> usize;
}

/* ======   Resource management   ====== */

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
    mut pledgedSrcSize: core::ffi::c_ulonglong,
) -> usize {
    if pledgedSrcSize == 0 {
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN; /* preserve "0 == unknown" behavior */
    }
    {
        let e = ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setPledgedSrcSize(zbc, pledgedSrcSize);
        if err_is_error(e) != 0 {
            return e;
        }
    }

    {
        let e = ZSTD_checkCParams(params.cParams);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_windowLog, params.cParams.windowLog as c_int);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_hashLog, params.cParams.hashLog as c_int);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_chainLog, params.cParams.chainLog as c_int);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_searchLog, params.cParams.searchLog as c_int);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_minMatch, params.cParams.minMatch as c_int);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_targetLength,
            params.cParams.targetLength as c_int,
        );
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_strategy, params.cParams.strategy as c_int);
        if err_is_error(e) != 0 {
            return e;
        }
    }

    {
        let e = ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_contentSizeFlag,
            params.fParams.contentSizeFlag as c_int,
        );
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_checksumFlag,
            params.fParams.checksumFlag as c_int,
        );
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_dictIDFlag, params.fParams.noDictIDFlag as c_int);
        if err_is_error(e) != 0 {
            return e;
        }
    }

    {
        let e = ZSTD_CCtx_loadDictionary(zbc, dict, dictSize);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInitDictionary(
    zbc: *mut ZBUFF_CCtx,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    {
        let e = ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_compressionLevel, compressionLevel);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_loadDictionary(zbc, dict, dictSize);
        if err_is_error(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit(zbc: *mut ZBUFF_CCtx, compressionLevel: c_int) -> usize {
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
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst,
        size: 0,
        pos: 0,
    };
    let mut inBuff: ZSTD_inBuffer = ZSTD_inBuffer {
        src,
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0;
    outBuff.size = *dstCapacityPtr;
    inBuff.src = src;
    inBuff.pos = 0;
    inBuff.size = *srcSizePtr;
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
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst,
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0;
    outBuff.size = *dstCapacityPtr;
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
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst,
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0;
    outBuff.size = *dstCapacityPtr;
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
