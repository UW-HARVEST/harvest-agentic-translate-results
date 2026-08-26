//! Translation of `deprecated/zbuff_decompress.c`
#![allow(dead_code)]

use crate::zstd_h::*;
use core::ffi::c_void;

/* `ZBUFF_DCtx` is `ZSTD_DStream` == `ZSTD_DCtx` */
type ZBUFF_DCtx = c_void;

extern "C" {
    fn ZSTD_createDStream() -> *mut ZBUFF_DCtx;
    fn ZSTD_createDStream_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_DCtx;
    fn ZSTD_freeDStream(zds: *mut ZBUFF_DCtx) -> usize;
    fn ZSTD_initDStream_usingDict(
        zds: *mut ZBUFF_DCtx,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZSTD_initDStream(zds: *mut ZBUFF_DCtx) -> usize;
    fn ZSTD_decompressStream(
        zds: *mut ZBUFF_DCtx,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> usize;
    fn ZSTD_DStreamInSize() -> usize;
    fn ZSTD_DStreamOutSize() -> usize;
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
pub unsafe extern "C" fn ZBUFF_freeDCtx(zbd: *mut ZBUFF_DCtx) -> usize {
    ZSTD_freeDStream(zbd)
}

/* *** Initialization *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInitDictionary(
    zbd: *mut ZBUFF_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTD_initDStream_usingDict(zbd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInit(zbd: *mut ZBUFF_DCtx) -> usize {
    ZSTD_initDStream(zbd)
}

/* *** Decompression *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressContinue(
    zbd: *mut ZBUFF_DCtx,
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
    result = ZSTD_decompressStream(zbd, &mut outBuff, &mut inBuff);
    *dstCapacityPtr = outBuff.pos;
    *srcSizePtr = inBuff.pos;
    result
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedDInSize() -> usize {
    ZSTD_DStreamInSize()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedDOutSize() -> usize {
    ZSTD_DStreamOutSize()
}
