//! Translation of deprecated/zbuff_decompress.c
//! Deprecated buffered streaming decompression API, a thin wrapper over the
//! public ZSTD_DStream streaming API.

use core::ffi::c_void;

use crate::common::allocations::ZSTD_customMem;
use crate::zstd_h::{ZSTD_inBuffer, ZSTD_outBuffer};

/* ZBUFF_DCtx is a typedef of ZSTD_DStream */
pub type ZBUFF_DCtx = c_void;

extern "C" {
    fn ZSTD_createDStream() -> *mut c_void;
    fn ZSTD_createDStream_advanced(customMem: ZSTD_customMem) -> *mut c_void;
    fn ZSTD_freeDStream(zds: *mut c_void) -> usize;
    fn ZSTD_initDStream_usingDict(
        zds: *mut c_void,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZSTD_initDStream(zds: *mut c_void) -> usize;
    fn ZSTD_decompressStream(
        zds: *mut c_void,
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
    let result: usize;
    outBuff.dst = dst;
    outBuff.pos = 0;
    outBuff.size = *dstCapacityPtr;
    inBuff.src = src;
    inBuff.pos = 0;
    inBuff.size = *srcSizePtr;
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
