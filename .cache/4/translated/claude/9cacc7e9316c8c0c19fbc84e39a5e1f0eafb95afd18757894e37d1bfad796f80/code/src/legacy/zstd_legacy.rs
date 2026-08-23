//! Translation of legacy/zstd_legacy.h  (ZSTD_LEGACY_SUPPORT == 5)
//!
//! With `ZSTD_LEGACY_SUPPORT=5`, only the v0.5, v0.6 and v0.7 decoders are
//! reachable through this dispatcher.
#![allow(
    non_snake_case,
    dead_code,
    non_upper_case_globals,
    non_camel_case_types,
    unused_variables
)]

use crate::error_private::*;
use crate::mem::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

use crate::legacy::v05;
use crate::legacy::v06;
use crate::legacy::v07;

pub const ZSTD_LEGACY_SUPPORT: u32 = 5;

pub const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525;
pub const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526;
pub const ZSTDv07_MAGICNUMBER: U32 = 0xFD2FB527;

#[inline]
pub unsafe fn ZSTD_isLegacy(src: *const core::ffi::c_void, srcSize: usize) -> core::ffi::c_uint {
    let magicNumberLE: U32;
    if srcSize < 4 {
        return 0;
    }
    magicNumberLE = MEM_readLE32(src as *const u8);
    match magicNumberLE {
        ZSTDv05_MAGICNUMBER => 5,
        ZSTDv06_MAGICNUMBER => 6,
        ZSTDv07_MAGICNUMBER => 7,
        _ => 0,
    }
}

#[inline]
pub unsafe fn ZSTD_getDecompressedSize_legacy(
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> core::ffi::c_ulonglong {
    let version: U32 = ZSTD_isLegacy(src, srcSize);
    if version < 5 {
        return 0;
    }
    if version == 5 {
        let mut fParams = v05::ZSTDv05_parameters::default();
        let frResult = v05::ZSTDv05_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.srcSize as core::ffi::c_ulonglong;
    }
    if version == 6 {
        let mut fParams = v06::ZSTDv06_frameParams::default();
        let frResult = v06::ZSTDv06_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    if version == 7 {
        let mut fParams = v07::ZSTDv07_frameParams::default();
        let frResult = v07::ZSTDv07_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    0
}

#[inline]
pub unsafe fn ZSTD_decompressLegacy(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    src: *const core::ffi::c_void,
    compressedSize: usize,
    dict: *const core::ffi::c_void,
    dictSize: usize,
) -> usize {
    let version: U32 = ZSTD_isLegacy(src, compressedSize);
    let mut x: core::ffi::c_char = 0;
    let mut dst = dst;
    let mut src = src;
    let mut dict = dict;
    if dst.is_null() {
        dst = &mut x as *mut core::ffi::c_char as *mut core::ffi::c_void;
    }
    if src.is_null() {
        src = &x as *const core::ffi::c_char as *const core::ffi::c_void;
    }
    if dict.is_null() {
        dict = &x as *const core::ffi::c_char as *const core::ffi::c_void;
    }
    match version {
        5 => {
            let result: usize;
            let zd = v05::ZSTDv05_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result = v05::ZSTDv05_decompress_usingDict(
                zd,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            v05::ZSTDv05_freeDCtx(zd);
            result
        }
        6 => {
            let result: usize;
            let zd = v06::ZSTDv06_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result = v06::ZSTDv06_decompress_usingDict(
                zd,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            v06::ZSTDv06_freeDCtx(zd);
            result
        }
        7 => {
            let result: usize;
            let zd = v07::ZSTDv07_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result = v07::ZSTDv07_decompress_usingDict(
                zd,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            v07::ZSTDv07_freeDCtx(zd);
            result
        }
        _ => ERROR(ZSTD_error_prefix_unknown),
    }
}

#[inline]
pub unsafe fn ZSTD_findFrameSizeInfoLegacy(
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo = ZSTD_frameSizeInfo {
        nbBlocks: 0,
        compressedSize: 0,
        decompressedBound: 0,
    };
    let version: U32 = ZSTD_isLegacy(src, srcSize);
    match version {
        5 => {
            v05::ZSTDv05_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &mut frameSizeInfo.compressedSize,
                &mut frameSizeInfo.decompressedBound,
            );
        }
        6 => {
            v06::ZSTDv06_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &mut frameSizeInfo.compressedSize,
                &mut frameSizeInfo.decompressedBound,
            );
        }
        7 => {
            v07::ZSTDv07_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &mut frameSizeInfo.compressedSize,
                &mut frameSizeInfo.decompressedBound,
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

#[inline]
pub unsafe fn ZSTD_findFrameCompressedSizeLegacy(
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    let frameSizeInfo = ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    frameSizeInfo.compressedSize
}

#[inline]
pub unsafe fn ZSTD_freeLegacyStreamContext(
    legacyContext: *mut core::ffi::c_void,
    version: U32,
) -> usize {
    match version {
        5 => v05::ZBUFFv05_freeDCtx(legacyContext as *mut v05::ZBUFFv05_DCtx),
        6 => v06::ZBUFFv06_freeDCtx(legacyContext as *mut v06::ZBUFFv06_DCtx),
        7 => v07::ZBUFFv07_freeDCtx(legacyContext as *mut v07::ZBUFFv07_DCtx),
        _ => ERROR(ZSTD_error_version_unsupported),
    }
}

#[inline]
pub unsafe fn ZSTD_initLegacyStream(
    legacyContext: *mut *mut core::ffi::c_void,
    prevVersion: U32,
    newVersion: U32,
    dict: *const core::ffi::c_void,
    dictSize: usize,
) -> usize {
    let x: core::ffi::c_char = 0;
    let mut dict = dict;
    if dict.is_null() {
        dict = &x as *const core::ffi::c_char as *const core::ffi::c_void;
    }
    if prevVersion != newVersion {
        ZSTD_freeLegacyStreamContext(*legacyContext, prevVersion);
    }
    match newVersion {
        5 => {
            let dctx: *mut v05::ZBUFFv05_DCtx = if prevVersion != newVersion {
                v05::ZBUFFv05_createDCtx()
            } else {
                *legacyContext as *mut v05::ZBUFFv05_DCtx
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            v05::ZBUFFv05_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx as *mut core::ffi::c_void;
            0
        }
        6 => {
            let dctx: *mut v06::ZBUFFv06_DCtx = if prevVersion != newVersion {
                v06::ZBUFFv06_createDCtx()
            } else {
                *legacyContext as *mut v06::ZBUFFv06_DCtx
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            v06::ZBUFFv06_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx as *mut core::ffi::c_void;
            0
        }
        7 => {
            let dctx: *mut v07::ZBUFFv07_DCtx = if prevVersion != newVersion {
                v07::ZBUFFv07_createDCtx()
            } else {
                *legacyContext as *mut v07::ZBUFFv07_DCtx
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            v07::ZBUFFv07_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx as *mut core::ffi::c_void;
            0
        }
        _ => 0,
    }
}

static mut ZSTD_decompressLegacyStream_x: core::ffi::c_char = 0;

#[inline]
pub unsafe fn ZSTD_decompressLegacyStream(
    legacyContext: *mut core::ffi::c_void,
    version: U32,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    if (*output).dst.is_null() {
        (*output).dst =
            core::ptr::addr_of_mut!(ZSTD_decompressLegacyStream_x) as *mut core::ffi::c_void;
    }
    if (*input).src.is_null() {
        (*input).src =
            core::ptr::addr_of_mut!(ZSTD_decompressLegacyStream_x) as *const core::ffi::c_void;
    }
    match version {
        5 => {
            let dctx = legacyContext as *mut v05::ZBUFFv05_DCtx;
            let src = ((*input).src as *const core::ffi::c_char).wrapping_add((*input).pos)
                as *const core::ffi::c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut core::ffi::c_char).wrapping_add((*output).pos)
                as *mut core::ffi::c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize = v05::ZBUFFv05_decompressContinue(
                dctx,
                dst,
                &mut decodedSize,
                src,
                &mut readSize,
            );
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        6 => {
            let dctx = legacyContext as *mut v06::ZBUFFv06_DCtx;
            let src = ((*input).src as *const core::ffi::c_char).wrapping_add((*input).pos)
                as *const core::ffi::c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut core::ffi::c_char).wrapping_add((*output).pos)
                as *mut core::ffi::c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize = v06::ZBUFFv06_decompressContinue(
                dctx,
                dst,
                &mut decodedSize,
                src,
                &mut readSize,
            );
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        7 => {
            let dctx = legacyContext as *mut v07::ZBUFFv07_DCtx;
            let src = ((*input).src as *const core::ffi::c_char).wrapping_add((*input).pos)
                as *const core::ffi::c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut core::ffi::c_char).wrapping_add((*output).pos)
                as *mut core::ffi::c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize = v07::ZBUFFv07_decompressContinue(
                dctx,
                dst,
                &mut decodedSize,
                src,
                &mut readSize,
            );
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        _ => ERROR(ZSTD_error_version_unsupported),
    }
}
