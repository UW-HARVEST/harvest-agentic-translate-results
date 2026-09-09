//! Translation of `legacy/zstd_legacy.h`
//!
//! Build configuration: `ZSTD_LEGACY_SUPPORT == 5`, therefore only the
//! `#if (ZSTD_LEGACY_SUPPORT <= 5)`, `<= 6` and `<= 7` branches are compiled.
//! Every `MEM_STATIC` function of the header becomes a `pub(crate) unsafe fn`.
#![allow(dead_code)]

use core::ffi::{c_char, c_uint, c_void};

use crate::cmem::*;
use crate::error_private::*;
use crate::zstd_common::ZSTD_isError;
use crate::zstd_h::*;
use crate::zstd_internal::ZSTD_frameSizeInfo;

use crate::legacy::v05::{
    ZBUFFv05_createDCtx, ZBUFFv05_decompressContinue, ZBUFFv05_decompressInitDictionary,
    ZBUFFv05_freeDCtx, ZSTDv05_createDCtx, ZSTDv05_decompress_usingDict,
    ZSTDv05_findFrameSizeInfoLegacy, ZSTDv05_freeDCtx, ZSTDv05_getFrameParams, ZSTDv05_parameters,
};
use crate::legacy::v06::{
    ZBUFFv06_createDCtx, ZBUFFv06_decompressContinue, ZBUFFv06_decompressInitDictionary,
    ZBUFFv06_freeDCtx, ZSTDv06_createDCtx, ZSTDv06_decompress_usingDict,
    ZSTDv06_findFrameSizeInfoLegacy, ZSTDv06_freeDCtx, ZSTDv06_frameParams,
    ZSTDv06_getFrameParams,
};
use crate::legacy::v07::{
    ZBUFFv07_createDCtx, ZBUFFv07_decompressContinue, ZBUFFv07_decompressInitDictionary,
    ZBUFFv07_freeDCtx, ZSTDv07_createDCtx, ZSTDv07_decompress_usingDict,
    ZSTDv07_findFrameSizeInfoLegacy, ZSTDv07_freeDCtx, ZSTDv07_frameParams,
    ZSTDv07_getFrameParams,
};

/* `ZSTDv0X_MAGICNUMBER` come from the respective legacy headers
 * (`zstd_v05.h`, `zstd_v06.h`, `zstd_v07.h`). */
const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525;
const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526;
const ZSTDv07_MAGICNUMBER: U32 = 0xFD2FB527;

/** ZSTD_isLegacy() :
    @return : > 0 if supported by legacy decoder. 0 otherwise.
              return value is the version.
*/
pub(crate) unsafe fn ZSTD_isLegacy(src: *const c_void, srcSize: usize) -> c_uint {
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

pub(crate) unsafe fn ZSTD_getDecompressedSize_legacy(src: *const c_void, srcSize: usize) -> u64 {
    let version: U32 = ZSTD_isLegacy(src, srcSize);
    if version < 5 {
        return 0; /* no decompressed size in frame header, or not a legacy format */
    }
    if version == 5 {
        let mut fParams: ZSTDv05_parameters = core::mem::zeroed();
        let frResult: usize = ZSTDv05_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.srcSize;
    }
    if version == 6 {
        let mut fParams: ZSTDv06_frameParams = core::mem::zeroed();
        let frResult: usize = ZSTDv06_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    if version == 7 {
        let mut fParams: ZSTDv07_frameParams = core::mem::zeroed();
        let frResult: usize = ZSTDv07_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    0 /* should not be possible */
}

pub(crate) unsafe fn ZSTD_decompressLegacy(
    mut dst: *mut c_void,
    dstCapacity: usize,
    mut src: *const c_void,
    compressedSize: usize,
    mut dict: *const c_void,
    dictSize: usize,
) -> usize {
    let version: U32 = ZSTD_isLegacy(src, compressedSize);
    let mut x: c_char = 0;
    /* Avoid passing NULL to legacy decoding. */
    if dst.is_null() {
        dst = (&mut x) as *mut c_char as *mut c_void;
    }
    if src.is_null() {
        src = (&x) as *const c_char as *const c_void;
    }
    if dict.is_null() {
        dict = (&x) as *const c_char as *const c_void;
    }
    match version {
        5 => {
            let result: usize;
            let zd = ZSTDv05_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result = ZSTDv05_decompress_usingDict(
                zd,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            ZSTDv05_freeDCtx(zd);
            result
        }
        6 => {
            let result: usize;
            let zd = ZSTDv06_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result = ZSTDv06_decompress_usingDict(
                zd,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            ZSTDv06_freeDCtx(zd);
            result
        }
        7 => {
            let result: usize;
            let zd = ZSTDv07_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            result = ZSTDv07_decompress_usingDict(
                zd,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            ZSTDv07_freeDCtx(zd);
            result
        }
        _ => ERROR(ZSTD_error_prefix_unknown),
    }
}

pub(crate) unsafe fn ZSTD_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_frameSizeInfo::default();
    let version: U32 = ZSTD_isLegacy(src, srcSize);
    match version {
        5 => {
            ZSTDv05_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &mut frameSizeInfo.compressedSize,
                &mut frameSizeInfo.decompressedBound,
            );
        }
        6 => {
            ZSTDv06_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &mut frameSizeInfo.compressedSize,
                &mut frameSizeInfo.decompressedBound,
            );
        }
        7 => {
            ZSTDv07_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &mut frameSizeInfo.compressedSize,
                &mut frameSizeInfo.decompressedBound,
            );
        }
        _ => {
            frameSizeInfo.compressedSize = ERROR(ZSTD_error_prefix_unknown);
            frameSizeInfo.decompressedBound = crate::zstd_h::ZSTD_CONTENTSIZE_ERROR;
        }
    }
    if ZSTD_isError(frameSizeInfo.compressedSize) == 0 && frameSizeInfo.compressedSize > srcSize {
        frameSizeInfo.compressedSize = ERROR(ZSTD_error_srcSize_wrong);
        frameSizeInfo.decompressedBound = crate::zstd_h::ZSTD_CONTENTSIZE_ERROR;
    }
    /* In all cases, decompressedBound == nbBlocks * ZSTD_BLOCKSIZE_MAX.
     * So we can compute nbBlocks without having to change every function.
     */
    if frameSizeInfo.decompressedBound != crate::zstd_h::ZSTD_CONTENTSIZE_ERROR {
        frameSizeInfo.nbBlocks =
            (frameSizeInfo.decompressedBound / ZSTD_BLOCKSIZE_MAX as u64) as usize;
    }
    frameSizeInfo
}

pub(crate) unsafe fn ZSTD_findFrameCompressedSizeLegacy(
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    frameSizeInfo.compressedSize
}

pub(crate) unsafe fn ZSTD_freeLegacyStreamContext(legacyContext: *mut c_void, version: U32) -> usize {
    match version {
        5 => ZBUFFv05_freeDCtx(legacyContext as *mut _),
        6 => ZBUFFv06_freeDCtx(legacyContext as *mut _),
        7 => ZBUFFv07_freeDCtx(legacyContext as *mut _),
        _ => ERROR(ZSTD_error_version_unsupported),
    }
}

pub(crate) unsafe fn ZSTD_initLegacyStream(
    legacyContext: *mut *mut c_void,
    prevVersion: U32,
    newVersion: U32,
    mut dict: *const c_void,
    dictSize: usize,
) -> usize {
    let x: c_char = 0;
    /* Avoid passing NULL to legacy decoding. */
    if dict.is_null() {
        dict = (&x) as *const c_char as *const c_void;
    }
    if prevVersion != newVersion {
        ZSTD_freeLegacyStreamContext(*legacyContext, prevVersion);
    }
    match newVersion {
        5 => {
            let dctx = if prevVersion != newVersion {
                ZBUFFv05_createDCtx()
            } else {
                *legacyContext as *mut _
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            ZBUFFv05_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx as *mut c_void;
            0
        }
        6 => {
            let dctx = if prevVersion != newVersion {
                ZBUFFv06_createDCtx()
            } else {
                *legacyContext as *mut _
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            ZBUFFv06_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx as *mut c_void;
            0
        }
        7 => {
            let dctx = if prevVersion != newVersion {
                ZBUFFv07_createDCtx()
            } else {
                *legacyContext as *mut _
            };
            if dctx.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            ZBUFFv07_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx as *mut c_void;
            0
        }
        _ => 0,
    }
}

/* `static char x;` of `ZSTD_decompressLegacyStream()` */
static mut ZSTD_decompressLegacyStream_x: c_char = 0;

pub(crate) unsafe fn ZSTD_decompressLegacyStream(
    legacyContext: *mut c_void,
    version: U32,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    /* Avoid passing NULL to legacy decoding. */
    if (*output).dst.is_null() {
        (*output).dst = (&raw mut ZSTD_decompressLegacyStream_x) as *mut c_void;
    }
    if (*input).src.is_null() {
        (*input).src = (&raw mut ZSTD_decompressLegacyStream_x) as *const c_void;
    }
    match version {
        5 => {
            let dctx = legacyContext as *mut _;
            let src = ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize: usize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize: usize = (*output).size - (*output).pos;
            let hintSize: usize =
                ZBUFFv05_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        6 => {
            let dctx = legacyContext as *mut _;
            let src = ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize: usize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize: usize = (*output).size - (*output).pos;
            let hintSize: usize =
                ZBUFFv06_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        7 => {
            let dctx = legacyContext as *mut _;
            let src = ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize: usize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize: usize = (*output).size - (*output).pos;
            let hintSize: usize =
                ZBUFFv07_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        _ => ERROR(ZSTD_error_version_unsupported),
    }
}
