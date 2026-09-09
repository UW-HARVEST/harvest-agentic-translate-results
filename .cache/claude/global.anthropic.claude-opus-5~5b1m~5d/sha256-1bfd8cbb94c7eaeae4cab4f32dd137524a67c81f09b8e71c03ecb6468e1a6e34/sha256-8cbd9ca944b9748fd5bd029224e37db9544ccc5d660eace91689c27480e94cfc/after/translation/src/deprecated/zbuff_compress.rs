//! Translation of `deprecated/zbuff_compress.c`
#![allow(dead_code)]

use core::ffi::{c_int, c_void};

use crate::error_private::*;
use crate::zstd_h::*;

use crate::compress::zstd_compress_internal::ZSTD_CStream;

/// `typedef ZSTD_CStream ZBUFF_CCtx;`
pub type ZBUFF_CCtx = ZSTD_CStream;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createCCtx() -> *mut ZBUFF_CCtx {
    crate::compress::zstd_compress::ZSTD_createCStream()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_CCtx {
    crate::compress::zstd_compress::ZSTD_createCStream_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeCCtx(zbc: *mut ZBUFF_CCtx) -> usize {
    crate::compress::zstd_compress::ZSTD_freeCStream(zbc)
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
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN; /* preserve "0 == unknown" behavior */
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e =
            crate::compress::zstd_compress::ZSTD_CCtx_setPledgedSrcSize(zbc, pledgedSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    {
        let e = crate::compress::zstd_compress::ZSTD_checkCParams(params.cParams);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_windowLog,
            params.cParams.windowLog as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_hashLog,
            params.cParams.hashLog as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_chainLog,
            params.cParams.chainLog as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_searchLog,
            params.cParams.searchLog as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_minMatch,
            params.cParams.minMatch as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_targetLength,
            params.cParams.targetLength as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_strategy,
            params.cParams.strategy as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_contentSizeFlag,
            params.fParams.contentSizeFlag,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_checksumFlag,
            params.fParams.checksumFlag,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_dictIDFlag,
            params.fParams.noDictIDFlag,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_loadDictionary(zbc, dict, dictSize);
        if ERR_isError(e) != 0 {
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
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_compressionLevel,
            compressionLevel,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_loadDictionary(zbc, dict, dictSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit(
    zbc: *mut ZBUFF_CCtx,
    compressionLevel: c_int,
) -> usize {
    crate::compress::zstd_compress::ZSTD_initCStream(zbc, compressionLevel)
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
        dst: core::ptr::null_mut(),
        size: 0,
        pos: 0,
    };
    let mut inBuff: ZSTD_inBuffer = ZSTD_inBuffer {
        src: core::ptr::null(),
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0;
    outBuff.size = *dstCapacityPtr;
    inBuff.src = src;
    inBuff.pos = 0;
    inBuff.size = *srcSizePtr;
    result = crate::compress::zstd_compress::ZSTD_compressStream(zbc, &mut outBuff, &mut inBuff);
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
        dst: core::ptr::null_mut(),
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0;
    outBuff.size = *dstCapacityPtr;
    result = crate::compress::zstd_compress::ZSTD_flushStream(zbc, &mut outBuff);
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
        dst: core::ptr::null_mut(),
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0;
    outBuff.size = *dstCapacityPtr;
    result = crate::compress::zstd_compress::ZSTD_endStream(zbc, &mut outBuff);
    *dstCapacityPtr = outBuff.pos;
    result
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedCInSize() -> usize {
    crate::compress::zstd_compress::ZSTD_CStreamInSize()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedCOutSize() -> usize {
    crate::compress::zstd_compress::ZSTD_CStreamOutSize()
}
