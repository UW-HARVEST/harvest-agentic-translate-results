//! Translation of deprecated/zbuff_compress.c
#![allow(non_snake_case, dead_code, non_upper_case_globals, non_camel_case_types)]

use crate::error_private::*;
use crate::zstd_compress::{
    ZSTD_CCtx_loadDictionary, ZSTD_CCtx_reset, ZSTD_CCtx_setParameter,
    ZSTD_CCtx_setPledgedSrcSize, ZSTD_checkCParams,
};
use crate::zstd_compress_internal::{ZSTD_CCtx, ZSTD_CStream};
use crate::zstd_compress_p4::{
    ZSTD_CStreamInSize, ZSTD_CStreamOutSize, ZSTD_compressStream, ZSTD_createCStream,
    ZSTD_createCStream_advanced, ZSTD_endStream, ZSTD_flushStream, ZSTD_freeCStream,
    ZSTD_initCStream,
};
use crate::zstd_h::*;

pub type ZBUFF_CCtx = ZSTD_CStream;

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_createCCtx() -> *mut ZBUFF_CCtx {
    ZSTD_createCStream()
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_CCtx {
    ZSTD_createCStream_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeCCtx(zbc: *mut ZBUFF_CCtx) -> usize {
    ZSTD_freeCStream(zbc)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit_advanced(
    zbc: *mut ZBUFF_CCtx,
    dict: *const core::ffi::c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> usize {
    let mut pledgedSrcSize = pledgedSrcSize;
    if pledgedSrcSize == 0 {
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    {
        let e = ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setPledgedSrcSize(zbc, pledgedSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    {
        let e = ZSTD_checkCParams(params.cParams);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_windowLog, params.cParams.windowLog as core::ffi::c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_hashLog, params.cParams.hashLog as core::ffi::c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_chainLog, params.cParams.chainLog as core::ffi::c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_searchLog, params.cParams.searchLog as core::ffi::c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_minMatch, params.cParams.minMatch as core::ffi::c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_targetLength,
            params.cParams.targetLength as core::ffi::c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_strategy, params.cParams.strategy as core::ffi::c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    {
        let e = ZSTD_CCtx_setParameter(
            zbc,
            ZSTD_c_contentSizeFlag,
            params.fParams.contentSizeFlag,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_checksumFlag, params.fParams.checksumFlag);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_dictIDFlag, params.fParams.noDictIDFlag);
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    {
        let e = ZSTD_CCtx_loadDictionary(zbc, dict, dictSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInitDictionary(
    zbc: *mut ZBUFF_CCtx,
    dict: *const core::ffi::c_void,
    dictSize: usize,
    compressionLevel: core::ffi::c_int,
) -> usize {
    {
        let e = ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(zbc, ZSTD_c_compressionLevel, compressionLevel);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_loadDictionary(zbc, dict, dictSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit(
    zbc: *mut ZBUFF_CCtx,
    compressionLevel: core::ffi::c_int,
) -> usize {
    ZSTD_initCStream(zbc, compressionLevel)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressContinue(
    zbc: *mut ZBUFF_CCtx,
    dst: *mut core::ffi::c_void,
    dstCapacityPtr: *mut usize,
    src: *const core::ffi::c_void,
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressFlush(
    zbc: *mut ZBUFF_CCtx,
    dst: *mut core::ffi::c_void,
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
    dst: *mut core::ffi::c_void,
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

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_recommendedCInSize() -> usize {
    ZSTD_CStreamInSize()
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_recommendedCOutSize() -> usize {
    ZSTD_CStreamOutSize()
}

/* silence unused-import warning for ZSTD_CCtx */
#[allow(dead_code)]
type _UnusedCCtx = ZSTD_CCtx;
