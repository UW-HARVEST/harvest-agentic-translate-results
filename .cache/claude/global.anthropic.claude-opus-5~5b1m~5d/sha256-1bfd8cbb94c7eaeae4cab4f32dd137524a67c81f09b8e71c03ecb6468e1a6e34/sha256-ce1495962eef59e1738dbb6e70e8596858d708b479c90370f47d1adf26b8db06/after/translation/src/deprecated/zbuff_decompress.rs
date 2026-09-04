//! Translation of `deprecated/zbuff_decompress.c`
#![allow(dead_code)]

use core::ffi::c_void;

use crate::zstd_h::*;

use crate::decompress::zstd_decompress_internal::ZSTD_DStream;

/// `typedef ZSTD_DStream ZBUFF_DCtx;`
pub type ZBUFF_DCtx = ZSTD_DStream;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createDCtx() -> *mut ZBUFF_DCtx {
    crate::decompress::zstd_decompress::ZSTD_createDStream()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createDCtx_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_DCtx {
    crate::decompress::zstd_decompress::ZSTD_createDStream_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeDCtx(zbd: *mut ZBUFF_DCtx) -> usize {
    crate::decompress::zstd_decompress::ZSTD_freeDStream(zbd)
}

/* *** Initialization *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInitDictionary(
    zbd: *mut ZBUFF_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    crate::decompress::zstd_decompress::ZSTD_initDStream_usingDict(zbd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInit(zbd: *mut ZBUFF_DCtx) -> usize {
    crate::decompress::zstd_decompress::ZSTD_initDStream(zbd)
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
        dst: core::ptr::null_mut(),
        size: 0,
        pos: 0,
    };
    let mut inBuff: ZSTD_inBuffer = ZSTD_inBuffer {
        src: core::ptr::null(),
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
    result =
        crate::decompress::zstd_decompress::ZSTD_decompressStream(zbd, &mut outBuff, &mut inBuff);
    *dstCapacityPtr = outBuff.pos;
    *srcSizePtr = inBuff.pos;
    result
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedDInSize() -> usize {
    crate::decompress::zstd_decompress::ZSTD_DStreamInSize()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedDOutSize() -> usize {
    crate::decompress::zstd_decompress::ZSTD_DStreamOutSize()
}
