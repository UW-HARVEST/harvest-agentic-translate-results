//! Rust transliteration of the inline dispatch helpers in
//! `c_src/src/legacy/zstd_legacy.h`.
//!
//! Build configuration: `ZSTD_LEGACY_SUPPORT=5`. Therefore the preprocessor
//! guards `(ZSTD_LEGACY_SUPPORT <= 1)`, `<= 2`, `<= 3`, `<= 4` are all FALSE,
//! and `<= 5`, `<= 6`, `<= 7` are TRUE. This means:
//!   * `ZSTD_isLegacy` only recognises the v0.5, v0.6 and v0.7 magic numbers.
//!   * versions 1..4 fall through to the `default` cases (unsupported / 0).
//!   * versions 5, 6, 7 dispatch to the `ZSTDv0{5,6,7}_*` / `ZBUFFv0{5,6,7}_*`
//!     symbols, which are exported by the same cdylib (written by other agents),
//!     so they link through `unsafe extern "C"` declarations.
//!
//! DEBUGLEVEL 0 -> DEBUGLOG / assert dropped.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_uint, c_ulonglong, c_void};

use crate::common::error_private::*;
use crate::common::mem::{size_t, MEM_readLE32, U32, U64};
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_h::{ZSTD_inBuffer, ZSTD_outBuffer, ZSTD_BLOCKSIZE_MAX, ZSTD_CONTENTSIZE_ERROR};
use crate::common::zstd_internal::ZSTD_frameSizeInfo;

/*-*************************************
*  Legacy magic numbers (v0.5, v0.6, v0.7)
***************************************/
pub const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525; /* v0.5 */
pub const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526; /* v0.6 */
pub const ZSTDv07_MAGICNUMBER: U32 = 0xFD2FB527; /* v0.7 */

/*-*************************************
*  Legacy frame-param structures
***************************************/
#[repr(C)]
pub struct ZSTDv05_parameters {
    pub srcSize: U64,
    pub windowLog: U32,
    pub contentLog: U32,
    pub hashLog: U32,
    pub searchLog: U32,
    pub searchLength: U32,
    pub targetLength: U32,
    pub strategy: c_uint, /* ZSTDv05_strategy */
}

#[repr(C)]
pub struct ZSTDv06_frameParams {
    pub frameContentSize: c_ulonglong,
    pub windowLog: c_uint,
}

#[repr(C)]
pub struct ZSTDv07_frameParams {
    pub frameContentSize: c_ulonglong,
    pub windowSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
}

/*-*************************************
*  Externs to legacy decoders (same cdylib)
***************************************/
unsafe extern "C" {
    /* v0.5 */
    fn ZSTDv05_getFrameParams(params: *mut ZSTDv05_parameters, src: *const c_void, srcSize: size_t) -> size_t;
    fn ZSTDv05_findFrameSizeInfoLegacy(src: *const c_void, srcSize: size_t, cSize: *mut size_t, dBound: *mut c_ulonglong);
    fn ZSTDv05_createDCtx() -> *mut c_void;
    fn ZSTDv05_freeDCtx(dctx: *mut c_void) -> size_t;
    fn ZSTDv05_decompress_usingDict(dctx: *mut c_void, dst: *mut c_void, dstCapacity: size_t, src: *const c_void, srcSize: size_t, dict: *const c_void, dictSize: size_t) -> size_t;
    fn ZBUFFv05_createDCtx() -> *mut c_void;
    fn ZBUFFv05_freeDCtx(dctx: *mut c_void) -> size_t;
    fn ZBUFFv05_decompressInitDictionary(dctx: *mut c_void, dict: *const c_void, dictSize: size_t) -> size_t;
    fn ZBUFFv05_decompressContinue(dctx: *mut c_void, dst: *mut c_void, dstCapacityPtr: *mut size_t, src: *const c_void, srcSizePtr: *mut size_t) -> size_t;

    /* v0.6 */
    fn ZSTDv06_getFrameParams(fparamsPtr: *mut ZSTDv06_frameParams, src: *const c_void, srcSize: size_t) -> size_t;
    fn ZSTDv06_findFrameSizeInfoLegacy(src: *const c_void, srcSize: size_t, cSize: *mut size_t, dBound: *mut c_ulonglong);
    fn ZSTDv06_createDCtx() -> *mut c_void;
    fn ZSTDv06_freeDCtx(dctx: *mut c_void) -> size_t;
    fn ZSTDv06_decompress_usingDict(dctx: *mut c_void, dst: *mut c_void, dstCapacity: size_t, src: *const c_void, srcSize: size_t, dict: *const c_void, dictSize: size_t) -> size_t;
    fn ZBUFFv06_createDCtx() -> *mut c_void;
    fn ZBUFFv06_freeDCtx(dctx: *mut c_void) -> size_t;
    fn ZBUFFv06_decompressInitDictionary(dctx: *mut c_void, dict: *const c_void, dictSize: size_t) -> size_t;
    fn ZBUFFv06_decompressContinue(dctx: *mut c_void, dst: *mut c_void, dstCapacityPtr: *mut size_t, src: *const c_void, srcSizePtr: *mut size_t) -> size_t;

    /* v0.7 */
    fn ZSTDv07_getFrameParams(fparamsPtr: *mut ZSTDv07_frameParams, src: *const c_void, srcSize: size_t) -> size_t;
    fn ZSTDv07_findFrameSizeInfoLegacy(src: *const c_void, srcSize: size_t, cSize: *mut size_t, dBound: *mut c_ulonglong);
    fn ZSTDv07_createDCtx() -> *mut c_void;
    fn ZSTDv07_freeDCtx(dctx: *mut c_void) -> size_t;
    fn ZSTDv07_decompress_usingDict(dctx: *mut c_void, dst: *mut c_void, dstCapacity: size_t, src: *const c_void, srcSize: size_t, dict: *const c_void, dictSize: size_t) -> size_t;
    fn ZBUFFv07_createDCtx() -> *mut c_void;
    fn ZBUFFv07_freeDCtx(dctx: *mut c_void) -> size_t;
    fn ZBUFFv07_decompressInitDictionary(dctx: *mut c_void, dict: *const c_void, dictSize: size_t) -> size_t;
    fn ZBUFFv07_decompressContinue(dctx: *mut c_void, dst: *mut c_void, dstCapacityPtr: *mut size_t, src: *const c_void, srcSizePtr: *mut size_t) -> size_t;
}

/** ZSTD_isLegacy() :
 *  @return : > 0 if supported by legacy decoder. 0 otherwise.
 *            return value is the version. */
#[inline]
pub unsafe fn ZSTD_isLegacy(src: *const c_void, srcSize: size_t) -> c_uint {
    if srcSize < 4 {
        return 0;
    }
    let magicNumberLE: U32 = MEM_readLE32(src as *const u8);
    /* Only the (ZSTD_LEGACY_SUPPORT <= 5,6,7) cases are compiled. */
    match magicNumberLE {
        ZSTDv05_MAGICNUMBER => 5,
        ZSTDv06_MAGICNUMBER => 6,
        ZSTDv07_MAGICNUMBER => 7,
        _ => 0,
    }
}

#[inline]
pub unsafe fn ZSTD_getDecompressedSize_legacy(src: *const c_void, srcSize: size_t) -> c_ulonglong {
    let version: U32 = ZSTD_isLegacy(src, srcSize);
    if version < 5 {
        return 0; /* no decompressed size in frame header, or not a legacy format */
    }
    if version == 5 {
        let mut fParams: ZSTDv05_parameters = core::mem::zeroed();
        let frResult: size_t = ZSTDv05_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.srcSize;
    }
    if version == 6 {
        let mut fParams: ZSTDv06_frameParams = core::mem::zeroed();
        let frResult: size_t = ZSTDv06_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    if version == 7 {
        let mut fParams: ZSTDv07_frameParams = core::mem::zeroed();
        let frResult: size_t = ZSTDv07_getFrameParams(&mut fParams, src, srcSize);
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    0 /* should not be possible */
}

#[inline]
pub unsafe fn ZSTD_decompressLegacy(
    mut dst: *mut c_void,
    dstCapacity: size_t,
    mut src: *const c_void,
    compressedSize: size_t,
    mut dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    let version: U32 = ZSTD_isLegacy(src, compressedSize);
    let mut x: c_char = 0;
    /* Avoid passing NULL to legacy decoding. */
    if dst.is_null() {
        /* assert(dstCapacity == 0); dropped */
        dst = &mut x as *mut c_char as *mut c_void;
    }
    if src.is_null() {
        /* assert(compressedSize == 0); dropped */
        src = &x as *const c_char as *const c_void;
    }
    if dict.is_null() {
        /* assert(dictSize == 0); dropped */
        dict = &x as *const c_char as *const c_void;
    }
    let _ = dstCapacity;
    let _ = dictSize;
    match version {
        /* versions 1..4 : (ZSTD_LEGACY_SUPPORT <= n) FALSE -> not compiled */
        5 => {
            let zd: *mut c_void = ZSTDv05_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            let result = ZSTDv05_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv05_freeDCtx(zd);
            result
        }
        6 => {
            let zd: *mut c_void = ZSTDv06_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            let result = ZSTDv06_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv06_freeDCtx(zd);
            result
        }
        7 => {
            let zd: *mut c_void = ZSTDv07_createDCtx();
            if zd.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
            let result = ZSTDv07_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv07_freeDCtx(zd);
            result
        }
        _ => ERROR(ZSTD_error_prefix_unknown),
    }
}

#[inline]
pub unsafe fn ZSTD_findFrameSizeInfoLegacy(src: *const c_void, srcSize: size_t) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = core::mem::zeroed();
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
            frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
        }
    }
    if ZSTD_isError(frameSizeInfo.compressedSize) == 0 && frameSizeInfo.compressedSize > srcSize {
        frameSizeInfo.compressedSize = ERROR(ZSTD_error_srcSize_wrong);
        frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
    }
    /* decompressedBound == nbBlocks * ZSTD_BLOCKSIZE_MAX. */
    if frameSizeInfo.decompressedBound != ZSTD_CONTENTSIZE_ERROR {
        /* assert dropped */
        frameSizeInfo.nbBlocks =
            (frameSizeInfo.decompressedBound / ZSTD_BLOCKSIZE_MAX as u64) as size_t;
    }
    frameSizeInfo
}

#[inline]
pub unsafe fn ZSTD_findFrameCompressedSizeLegacy(src: *const c_void, srcSize: size_t) -> size_t {
    let frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    frameSizeInfo.compressedSize
}

#[inline]
pub unsafe fn ZSTD_freeLegacyStreamContext(legacyContext: *mut c_void, version: U32) -> size_t {
    match version {
        5 => ZBUFFv05_freeDCtx(legacyContext),
        6 => ZBUFFv06_freeDCtx(legacyContext),
        7 => ZBUFFv07_freeDCtx(legacyContext),
        /* default, 1, 2, 3, 4 : (void)legacyContext; return ERROR(version_unsupported); */
        _ => ERROR(ZSTD_error_version_unsupported),
    }
}

#[inline]
pub unsafe fn ZSTD_initLegacyStream(
    legacyContext: *mut *mut c_void,
    prevVersion: U32,
    newVersion: U32,
    mut dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    let x: c_char = 0;
    /* Avoid passing NULL to legacy decoding. */
    if dict.is_null() {
        /* assert(dictSize == 0); dropped */
        dict = &x as *const c_char as *const c_void;
    }
    /* DEBUGLOG dropped */
    if prevVersion != newVersion {
        ZSTD_freeLegacyStreamContext(*legacyContext, prevVersion);
    }
    match newVersion {
        5 => {
            let dctx: *mut c_void = if prevVersion != newVersion {
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
            let dctx: *mut c_void = if prevVersion != newVersion {
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
            let dctx: *mut c_void = if prevVersion != newVersion {
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
        /* default, 1, 2, 3, 4 : (void)dict; (void)dictSize; return 0; */
        _ => {
            let _ = dictSize;
            0
        }
    }
}

#[inline]
pub unsafe fn ZSTD_decompressLegacyStream(
    legacyContext: *mut c_void,
    version: U32,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> size_t {
    /* static char x; -- a mutable static shared across calls, matching C. */
    static mut X: c_char = 0;
    /* Avoid passing NULL to legacy decoding. */
    if (*output).dst.is_null() {
        /* assert(output->size == 0); dropped */
        (*output).dst = &raw mut X as *mut c_void;
    }
    if (*input).src.is_null() {
        /* assert(input->size == 0); dropped */
        (*input).src = &raw const X as *const c_void;
    }
    /* DEBUGLOG dropped */
    match version {
        5 => {
            let dctx: *mut c_void = legacyContext;
            let src: *const c_void =
                ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize: size_t = (*input).size - (*input).pos;
            let dst: *mut c_void = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize: size_t = (*output).size - (*output).pos;
            let hintSize = ZBUFFv05_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        6 => {
            let dctx: *mut c_void = legacyContext;
            let src: *const c_void =
                ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize: size_t = (*input).size - (*input).pos;
            let dst: *mut c_void = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize: size_t = (*output).size - (*output).pos;
            let hintSize = ZBUFFv06_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        7 => {
            let dctx: *mut c_void = legacyContext;
            let src: *const c_void =
                ((*input).src as *const c_char).add((*input).pos) as *const c_void;
            let mut readSize: size_t = (*input).size - (*input).pos;
            let dst: *mut c_void = ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize: size_t = (*output).size - (*output).pos;
            let hintSize = ZBUFFv07_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        /* default, 1, 2, 3, 4 */
        _ => ERROR(ZSTD_error_version_unsupported),
    }
}
