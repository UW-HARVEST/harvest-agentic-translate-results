//! Translation of `legacy/zstd_legacy.h` (all `MEM_STATIC` inline helpers).
//!
//! This build uses `ZSTD_LEGACY_SUPPORT = 5`, so only the
//! `ZSTD_LEGACY_SUPPORT <= 5 / <= 6 / <= 7` branches are live: versions 5, 6
//! and 7 are wired up, versions 1..4 fall into the `default`/unsupported arms
//! (their object files still export their own symbols, they are just never
//! referenced from here).
#![allow(dead_code)]

use crate::common::error_private::*;
use crate::common::mem::*;
use crate::common::zstd_internal::*;
use core::ffi::{c_char, c_void};

pub const ZSTD_LEGACY_SUPPORT: u32 = 5;

pub const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525;
pub const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526;
pub const ZSTDv07_MAGICNUMBER: U32 = 0xFD2FB527;

/* legacy frame-parameter structs, from the vXX headers */
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTDv05_parameters {
    pub srcSize: U64,
    pub windowLog: U32,
    pub contentLog: U32,
    pub hashLog: U32,
    pub searchLog: U32,
    pub searchLength: U32,
    pub targetLength: U32,
    pub strategy: core::ffi::c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTDv06_frameParams {
    pub frameContentSize: u64,
    pub windowLog: core::ffi::c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTDv07_frameParams {
    pub frameContentSize: u64,
    pub windowSize: core::ffi::c_uint,
    pub dictID: core::ffi::c_uint,
    pub checksumFlag: core::ffi::c_uint,
}

extern "C" {
    /* v05 */
    fn ZSTDv05_getFrameParams(
        params: *mut ZSTDv05_parameters,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    fn ZSTDv05_createDCtx() -> *mut c_void;
    fn ZSTDv05_freeDCtx(dctx: *mut c_void) -> usize;
    fn ZSTDv05_decompress_usingDict(
        dctx: *mut c_void,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZSTDv05_findFrameSizeInfoLegacy(
        src: *const c_void,
        srcSize: usize,
        cSize: *mut usize,
        dBound: *mut u64,
    );
    fn ZBUFFv05_createDCtx() -> *mut c_void;
    fn ZBUFFv05_freeDCtx(dctx: *mut c_void) -> usize;
    fn ZBUFFv05_decompressInitDictionary(
        dctx: *mut c_void,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZBUFFv05_decompressContinue(
        dctx: *mut c_void,
        dst: *mut c_void,
        dstCapacityPtr: *mut usize,
        src: *const c_void,
        srcSizePtr: *mut usize,
    ) -> usize;

    /* v06 */
    fn ZSTDv06_getFrameParams(
        params: *mut ZSTDv06_frameParams,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    fn ZSTDv06_createDCtx() -> *mut c_void;
    fn ZSTDv06_freeDCtx(dctx: *mut c_void) -> usize;
    fn ZSTDv06_decompress_usingDict(
        dctx: *mut c_void,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZSTDv06_findFrameSizeInfoLegacy(
        src: *const c_void,
        srcSize: usize,
        cSize: *mut usize,
        dBound: *mut u64,
    );
    fn ZBUFFv06_createDCtx() -> *mut c_void;
    fn ZBUFFv06_freeDCtx(dctx: *mut c_void) -> usize;
    fn ZBUFFv06_decompressInitDictionary(
        dctx: *mut c_void,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZBUFFv06_decompressContinue(
        dctx: *mut c_void,
        dst: *mut c_void,
        dstCapacityPtr: *mut usize,
        src: *const c_void,
        srcSizePtr: *mut usize,
    ) -> usize;

    /* v07 */
    fn ZSTDv07_getFrameParams(
        params: *mut ZSTDv07_frameParams,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    fn ZSTDv07_createDCtx() -> *mut c_void;
    fn ZSTDv07_freeDCtx(dctx: *mut c_void) -> usize;
    fn ZSTDv07_decompress_usingDict(
        dctx: *mut c_void,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZSTDv07_findFrameSizeInfoLegacy(
        src: *const c_void,
        srcSize: usize,
        cSize: *mut usize,
        dBound: *mut u64,
    );
    fn ZBUFFv07_createDCtx() -> *mut c_void;
    fn ZBUFFv07_freeDCtx(dctx: *mut c_void) -> usize;
    fn ZBUFFv07_decompressInitDictionary(
        dctx: *mut c_void,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZBUFFv07_decompressContinue(
        dctx: *mut c_void,
        dst: *mut c_void,
        dstCapacityPtr: *mut usize,
        src: *const c_void,
        srcSizePtr: *mut usize,
    ) -> usize;
}

/// `ZSTD_isLegacy()` : @return : > 0 if supported by legacy decoder, else 0.
#[inline(always)]
pub unsafe fn ZSTD_isLegacy(src: *const c_void, srcSize: usize) -> u32 {
    let magicNumberLE: U32;
    if srcSize < 4 {
        return 0;
    }
    magicNumberLE = MEM_readLE32(src);
    match magicNumberLE {
        ZSTDv05_MAGICNUMBER => 5,
        ZSTDv06_MAGICNUMBER => 6,
        ZSTDv07_MAGICNUMBER => 7,
        _ => 0,
    }
}

#[inline(always)]
pub unsafe fn ZSTD_getDecompressedSize_legacy(src: *const c_void, srcSize: usize) -> u64 {
    let version = ZSTD_isLegacy(src, srcSize);
    if version < 5 {
        return 0;
    }
    if version == 5 {
        let mut fParams = ZSTDv05_parameters::default();
        let frResult = ZSTDv05_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.srcSize;
    }
    if version == 6 {
        let mut fParams = ZSTDv06_frameParams::default();
        let frResult = ZSTDv06_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    if version == 7 {
        let mut fParams = ZSTDv07_frameParams::default();
        let frResult = ZSTDv07_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    0
}

#[inline(always)]
pub unsafe fn ZSTD_decompressLegacy(
    mut dst: *mut c_void,
    dstCapacity: usize,
    mut src: *const c_void,
    compressedSize: usize,
    mut dict: *const c_void,
    dictSize: usize,
) -> usize {
    let version = ZSTD_isLegacy(src, compressedSize);
    let mut x: c_char = 0;
    /* Avoid passing NULL to legacy decoding. */
    if dst.is_null() {
        dst = core::ptr::addr_of_mut!(x) as *mut c_void;
    }
    if src.is_null() {
        src = core::ptr::addr_of!(x) as *const c_void;
    }
    if dict.is_null() {
        dict = core::ptr::addr_of!(x) as *const c_void;
    }
    match version {
        5 => {
            let result: usize;
            let zd = ZSTDv05_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result =
                ZSTDv05_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv05_freeDCtx(zd);
            result
        }
        6 => {
            let result: usize;
            let zd = ZSTDv06_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result =
                ZSTDv06_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv06_freeDCtx(zd);
            result
        }
        7 => {
            let result: usize;
            let zd = ZSTDv07_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result =
                ZSTDv07_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv07_freeDCtx(zd);
            result
        }
        _ => ERROR(ZSTD_error_prefix_unknown),
    }
}

#[inline(always)]
pub unsafe fn ZSTD_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo = ZSTD_frameSizeInfo {
        nbBlocks: 0,
        compressedSize: 0,
        decompressedBound: 0,
    };
    let version = ZSTD_isLegacy(src, srcSize);
    match version {
        5 => {
            ZSTDv05_findFrameSizeInfoLegacy(
                src,
                srcSize,
                core::ptr::addr_of_mut!(frameSizeInfo.compressedSize),
                core::ptr::addr_of_mut!(frameSizeInfo.decompressedBound),
            );
        }
        6 => {
            ZSTDv06_findFrameSizeInfoLegacy(
                src,
                srcSize,
                core::ptr::addr_of_mut!(frameSizeInfo.compressedSize),
                core::ptr::addr_of_mut!(frameSizeInfo.decompressedBound),
            );
        }
        7 => {
            ZSTDv07_findFrameSizeInfoLegacy(
                src,
                srcSize,
                core::ptr::addr_of_mut!(frameSizeInfo.compressedSize),
                core::ptr::addr_of_mut!(frameSizeInfo.decompressedBound),
            );
        }
        _ => {
            frameSizeInfo.compressedSize = ERROR(ZSTD_error_prefix_unknown);
            frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
        }
    }
    if ERR_isError(frameSizeInfo.compressedSize) == 0 && frameSizeInfo.compressedSize > srcSize {
        frameSizeInfo.compressedSize = ERROR(ZSTD_error_srcSize_wrong);
        frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
    }
    if frameSizeInfo.decompressedBound != ZSTD_CONTENTSIZE_ERROR {
        frameSizeInfo.nbBlocks =
            (frameSizeInfo.decompressedBound / ZSTD_BLOCKSIZE_MAX as u64) as usize;
    }
    frameSizeInfo
}

#[inline(always)]
pub unsafe fn ZSTD_findFrameCompressedSizeLegacy(src: *const c_void, srcSize: usize) -> usize {
    let frameSizeInfo = ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    frameSizeInfo.compressedSize
}

#[inline(always)]
pub unsafe fn ZSTD_freeLegacyStreamContext(legacyContext: *mut c_void, version: U32) -> usize {
    match version {
        5 => ZBUFFv05_freeDCtx(legacyContext),
        6 => ZBUFFv06_freeDCtx(legacyContext),
        7 => ZBUFFv07_freeDCtx(legacyContext),
        _ => ERROR(ZSTD_error_version_unsupported),
    }
}

#[inline(always)]
pub unsafe fn ZSTD_initLegacyStream(
    legacyContext: *mut *mut c_void,
    prevVersion: U32,
    newVersion: U32,
    mut dict: *const c_void,
    dictSize: usize,
) -> usize {
    let x: c_char = 0;
    /* Avoid passing NULL to legacy decoding. */
    if dict.is_null() {
        dict = core::ptr::addr_of!(x) as *const c_void;
    }
    if prevVersion != newVersion {
        ZSTD_freeLegacyStreamContext(*legacyContext, prevVersion);
    }
    match newVersion {
        5 => {
            let dctx = if prevVersion != newVersion {
                ZBUFFv05_createDCtx()
            } else {
                *legacyContext
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            ZBUFFv05_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx;
            0
        }
        6 => {
            let dctx = if prevVersion != newVersion {
                ZBUFFv06_createDCtx()
            } else {
                *legacyContext
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            ZBUFFv06_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx;
            0
        }
        7 => {
            let dctx = if prevVersion != newVersion {
                ZBUFFv07_createDCtx()
            } else {
                *legacyContext
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            ZBUFFv07_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx;
            0
        }
        _ => 0,
    }
}

static mut ZSTD_decompressLegacyStream_x: c_char = 0;

#[inline(always)]
pub unsafe fn ZSTD_decompressLegacyStream(
    legacyContext: *mut c_void,
    version: U32,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    /* Avoid passing NULL to legacy decoding. */
    if (*output).dst.is_null() {
        (*output).dst = core::ptr::addr_of_mut!(ZSTD_decompressLegacyStream_x) as *mut c_void;
    }
    if (*input).src.is_null() {
        (*input).src = core::ptr::addr_of!(ZSTD_decompressLegacyStream_x) as *const c_void;
    }
    match version {
        5 => {
            let dctx = legacyContext;
            let src = ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize =
                ZBUFFv05_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        6 => {
            let dctx = legacyContext;
            let src = ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize =
                ZBUFFv06_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        7 => {
            let dctx = legacyContext;
            let src = ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize =
                ZBUFFv07_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        _ => ERROR(ZSTD_error_version_unsupported),
    }
}
