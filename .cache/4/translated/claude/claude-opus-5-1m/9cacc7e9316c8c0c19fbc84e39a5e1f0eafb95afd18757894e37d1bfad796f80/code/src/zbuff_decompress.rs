//! Translation of deprecated/zbuff_decompress.c
#![allow(non_snake_case, dead_code, non_upper_case_globals, non_camel_case_types)]

use crate::zstd_decompress::{
    ZSTD_DStreamInSize, ZSTD_DStreamOutSize, ZSTD_createDStream, ZSTD_createDStream_advanced,
    ZSTD_decompressStream, ZSTD_freeDStream, ZSTD_initDStream, ZSTD_initDStream_usingDict,
};
use crate::zstd_decompress_internal::ZSTD_DStream;
use crate::zstd_h::*;

pub type ZBUFF_DCtx = ZSTD_DStream;

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_createDCtx() -> *mut ZBUFF_DCtx {
    ZSTD_createDStream()
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_createDCtx_advanced(customMem: ZSTD_customMem) -> *mut ZBUFF_DCtx {
    ZSTD_createDStream_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeDCtx(zbd: *mut ZBUFF_DCtx) -> usize {
    ZSTD_freeDStream(zbd)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInitDictionary(
    zbd: *mut ZBUFF_DCtx,
    dict: *const core::ffi::c_void,
    dictSize: usize,
) -> usize {
    ZSTD_initDStream_usingDict(zbd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInit(zbd: *mut ZBUFF_DCtx) -> usize {
    ZSTD_initDStream(zbd)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressContinue(
    zbd: *mut ZBUFF_DCtx,
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
    result = ZSTD_decompressStream(zbd, &mut outBuff, &mut inBuff);
    *dstCapacityPtr = outBuff.pos;
    *srcSizePtr = inBuff.pos;
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_recommendedDInSize() -> usize {
    ZSTD_DStreamInSize()
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFF_recommendedDOutSize() -> usize {
    ZSTD_DStreamOutSize()
}
