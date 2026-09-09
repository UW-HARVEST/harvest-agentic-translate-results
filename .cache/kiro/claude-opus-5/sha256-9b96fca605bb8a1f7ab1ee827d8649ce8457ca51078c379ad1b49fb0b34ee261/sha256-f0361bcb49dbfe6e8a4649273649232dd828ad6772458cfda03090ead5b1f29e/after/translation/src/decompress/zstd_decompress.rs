//! Rust transliteration of `c_src/src/decompress/zstd_decompress.c`.
//!
//! Build configuration baked in:
//!   ZSTD_HEAPMODE 1, ZSTD_LEGACY_SUPPORT=5 (>=1), DYNAMIC_BMI2=0,
//!   ZSTD_TRACE=0, DEBUGLEVEL 0, no ZSTD_MULTITHREAD,
//!   no FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::common::error_private::*;
use crate::common::mem::*;
use crate::common::bits::ZSTD_highbit32;
use crate::common::huf::HUF_DTable;
use crate::common::xxhash::*;
use crate::common::zstd_common::{ZSTD_getErrorCode, ZSTD_isError};
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;
use crate::common::entropy_common::*;

use crate::decompress::zstd_ddict::*;
use crate::decompress::zstd_decompress_internal::*;
use crate::decompress::huf_decompress::*;

use crate::legacy::zstd_legacy::*;

/* ZSTD_customMem used by the DCtx is the zstd_internal definition. */
type ZSTD_customMem = crate::common::zstd_internal::ZSTD_customMem;

/*-*******************************************************
*  Tuning parameters (baked constants)
*********************************************************/
pub const ZSTD_MAXWINDOWSIZE_DEFAULT: size_t =
    ((1u32 << ZSTD_WINDOWLOG_LIMIT_DEFAULT) as size_t) + 1;
pub const ZSTD_NO_FORWARD_PROGRESS_MAX: c_int = 16;

/*-*******************************************************
*  streaming_operation enum (from zstd_decompress_block.h)
*********************************************************/
pub type streaming_operation = c_uint;
pub const not_streaming: streaming_operation = 0;
pub const is_streaming: streaming_operation = 1;

/*-*******************************************************
*  Functions provided by zstd_decompress_block.c (concurrent agent).
*  Declared extern; they link within the same cdylib.
*  ZSTD_getcBlockSize / ZSTD_decodeSeqHeaders are declared in zstd_internal.h,
*  ZSTD_decompressBlock_internal / ZSTD_buildFSETable / ZSTD_checkContinuity
*  in the block headers.
*********************************************************/
unsafe extern "C" {
    pub fn ZSTD_getcBlockSize(
        src: *const c_void,
        srcSize: size_t,
        bpPtr: *mut blockProperties_t,
    ) -> size_t;

    pub fn ZSTD_decompressBlock_internal(
        dctx: *mut ZSTD_DCtx,
        dst: *mut c_void,
        dstCapacity: size_t,
        src: *const c_void,
        srcSize: size_t,
        streaming: streaming_operation,
    ) -> size_t;

    pub fn ZSTD_buildFSETable(
        dt: *mut ZSTD_seqSymbol,
        normalizedCounter: *const core::ffi::c_short,
        maxSymbolValue: c_uint,
        baseValue: *const U32,
        nbAdditionalBits: *const U8,
        tableLog: c_uint,
        wksp: *mut c_void,
        wkspSize: size_t,
        bmi2: c_int,
    );

    pub fn ZSTD_checkContinuity(dctx: *mut ZSTD_DCtx, dst: *const c_void, dstSize: size_t);
}

/* RETURN_ERROR_IF / FORWARD_IF_ERROR / RETURN_ERROR helpers.
 * DEBUGLEVEL 0 -> no logging; the string args are discarded. */

/*************************************
 * Multiple DDicts Hashset internals *
 *************************************/

pub const DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT: size_t = 4;
pub const DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT: size_t = 3;
pub const DDICT_HASHSET_TABLE_BASE_SIZE: size_t = 64;
pub const DDICT_HASHSET_RESIZE_FACTOR: size_t = 2;

/* Hash function to determine starting position of dict insertion within the table */
pub unsafe fn ZSTD_DDictHashSet_getIndex(
    hashSet: *const ZSTD_DDictHashSet,
    dictID: U32,
) -> size_t {
    let hash: U64 = XXH64(&dictID as *const U32 as *const c_void, core::mem::size_of::<U32>(), 0);
    (hash & ((*hashSet).ddictPtrTableSize as U64 - 1)) as size_t
}

/* Adds DDict to a hashset without resizing it. */
pub unsafe fn ZSTD_DDictHashSet_emplaceDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    ddict: *const ZSTD_DDict,
) -> size_t {
    let dictID: U32 = ZSTD_getDictID_fromDDict(ddict);
    let mut idx: size_t = ZSTD_DDictHashSet_getIndex(hashSet, dictID);
    let idxRangeMask: size_t = (*hashSet).ddictPtrTableSize - 1;
    if (*hashSet).ddictPtrCount == (*hashSet).ddictPtrTableSize {
        return ERROR(ZSTD_error_GENERIC);
    }
    while !(*(*hashSet).ddictPtrTable.add(idx)).is_null() {
        /* Replace existing ddict if inserting ddict with same dictID */
        if ZSTD_getDictID_fromDDict(*(*hashSet).ddictPtrTable.add(idx)) == dictID {
            *((*hashSet).ddictPtrTable.add(idx) as *mut *const ZSTD_DDict) = ddict;
            return 0;
        }
        idx &= idxRangeMask;
        idx += 1;
    }
    *((*hashSet).ddictPtrTable.add(idx) as *mut *const ZSTD_DDict) = ddict;
    (*hashSet).ddictPtrCount += 1;
    0
}

/* Expands hash table by factor of DDICT_HASHSET_RESIZE_FACTOR and rehashes. */
pub unsafe fn ZSTD_DDictHashSet_expand(
    hashSet: *mut ZSTD_DDictHashSet,
    customMem: ZSTD_customMem,
) -> size_t {
    let newTableSize: size_t = (*hashSet).ddictPtrTableSize * DDICT_HASHSET_RESIZE_FACTOR;
    let newTable: *const *const ZSTD_DDict = ZSTD_customCalloc(
        core::mem::size_of::<*const ZSTD_DDict>() * newTableSize,
        customMem,
    ) as *const *const ZSTD_DDict;
    let oldTable: *const *const ZSTD_DDict = (*hashSet).ddictPtrTable;
    let oldTableSize: size_t = (*hashSet).ddictPtrTableSize;
    let mut i: size_t;

    if newTable.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    (*hashSet).ddictPtrTable = newTable;
    (*hashSet).ddictPtrTableSize = newTableSize;
    (*hashSet).ddictPtrCount = 0;
    i = 0;
    while i < oldTableSize {
        if !(*oldTable.add(i)).is_null() {
            let err_code = ZSTD_DDictHashSet_emplaceDDict(hashSet, *oldTable.add(i));
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        i += 1;
    }
    ZSTD_customFree(oldTable as *mut c_void, customMem);
    0
}

/* Fetches a DDict with the given dictID */
pub unsafe fn ZSTD_DDictHashSet_getDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    dictID: U32,
) -> *const ZSTD_DDict {
    let mut idx: size_t = ZSTD_DDictHashSet_getIndex(hashSet, dictID);
    let idxRangeMask: size_t = (*hashSet).ddictPtrTableSize - 1;
    loop {
        let currDictID: size_t =
            ZSTD_getDictID_fromDDict(*(*hashSet).ddictPtrTable.add(idx)) as size_t;
        if currDictID == dictID as size_t || currDictID == 0 {
            /* currDictID == 0 implies a NULL ddict entry */
            break;
        } else {
            idx &= idxRangeMask;
            idx += 1;
        }
    }
    *(*hashSet).ddictPtrTable.add(idx)
}

/* Allocates space for and returns a ddict hash set */
pub unsafe fn ZSTD_createDDictHashSet(customMem: ZSTD_customMem) -> *mut ZSTD_DDictHashSet {
    let ret: *mut ZSTD_DDictHashSet =
        ZSTD_customMalloc(core::mem::size_of::<ZSTD_DDictHashSet>(), customMem)
            as *mut ZSTD_DDictHashSet;
    if ret.is_null() {
        return core::ptr::null_mut();
    }
    (*ret).ddictPtrTable = ZSTD_customCalloc(
        DDICT_HASHSET_TABLE_BASE_SIZE * core::mem::size_of::<*const ZSTD_DDict>(),
        customMem,
    ) as *const *const ZSTD_DDict;
    if (*ret).ddictPtrTable.is_null() {
        ZSTD_customFree(ret as *mut c_void, customMem);
        return core::ptr::null_mut();
    }
    (*ret).ddictPtrTableSize = DDICT_HASHSET_TABLE_BASE_SIZE;
    (*ret).ddictPtrCount = 0;
    ret
}

/* Frees the table of ZSTD_DDict* within a hashset, then frees the hashset itself. */
pub unsafe fn ZSTD_freeDDictHashSet(hashSet: *mut ZSTD_DDictHashSet, customMem: ZSTD_customMem) {
    if !hashSet.is_null() && !(*hashSet).ddictPtrTable.is_null() {
        ZSTD_customFree((*hashSet).ddictPtrTable as *mut c_void, customMem);
    }
    if !hashSet.is_null() {
        ZSTD_customFree(hashSet as *mut c_void, customMem);
    }
}

/* Public function: Adds a DDict into the ZSTD_DDictHashSet, possibly triggering a resize. */
pub unsafe fn ZSTD_DDictHashSet_addDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    ddict: *const ZSTD_DDict,
    customMem: ZSTD_customMem,
) -> size_t {
    if (*hashSet).ddictPtrCount * DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT
        / (*hashSet).ddictPtrTableSize
        * DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT
        != 0
    {
        let err_code = ZSTD_DDictHashSet_expand(hashSet, customMem);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    let err_code = ZSTD_DDictHashSet_emplaceDDict(hashSet, ddict);
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    0
}

/*-*************************************************************
*   Context management
***************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DCtx(dctx: *const ZSTD_DCtx) -> size_t {
    if dctx.is_null() {
        return 0; /* support sizeof NULL */
    }
    core::mem::size_of::<ZSTD_DCtx>()
        + ZSTD_sizeof_DDict((*dctx).ddictLocal)
        + (*dctx).inBuffSize
        + (*dctx).outBuffSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDCtxSize() -> size_t {
    core::mem::size_of::<ZSTD_DCtx>()
}

pub unsafe fn ZSTD_startingInputLength(format: ZSTD_format_e) -> size_t {
    let startingInputLength: size_t = ZSTD_FRAMEHEADERSIZE_PREFIX(format);
    /* assert dropped */
    startingInputLength
}

pub unsafe fn ZSTD_DCtx_resetParameters(dctx: *mut ZSTD_DCtx) {
    /* assert(dctx->streamStage == zdss_init); dropped */
    (*dctx).format = ZSTD_f_zstd1;
    (*dctx).maxWindowSize = ZSTD_MAXWINDOWSIZE_DEFAULT;
    (*dctx).outBufferMode = ZSTD_bm_buffered;
    (*dctx).forceIgnoreChecksum = ZSTD_d_validateChecksum;
    (*dctx).refMultipleDDicts = ZSTD_rmd_refSingleDDict;
    (*dctx).disableHufAsm = 0;
    (*dctx).maxBlockSizeParam = 0;
}

pub unsafe fn ZSTD_initDCtx_internal(dctx: *mut ZSTD_DCtx) {
    (*dctx).staticSize = 0;
    (*dctx).ddict = core::ptr::null();
    (*dctx).ddictLocal = core::ptr::null_mut();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).ddictIsCold = 0;
    (*dctx).dictUses = ZSTD_dont_use;
    (*dctx).inBuff = core::ptr::null_mut();
    (*dctx).inBuffSize = 0;
    (*dctx).outBuffSize = 0;
    (*dctx).streamStage = zdss_init;
    /* ZSTD_LEGACY_SUPPORT >= 1 */
    (*dctx).legacyContext = core::ptr::null_mut();
    (*dctx).previousLegacyVersion = 0;
    (*dctx).noForwardProgress = 0;
    (*dctx).oversizedDuration = 0;
    (*dctx).isFrameDecompression = 1;
    /* DYNAMIC_BMI2 == 0 : dctx->bmi2 not set */
    (*dctx).ddictSet = core::ptr::null_mut();
    ZSTD_DCtx_resetParameters(dctx);
    /* FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION not defined */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDCtx(
    workspace: *mut c_void,
    workspaceSize: size_t,
) -> *mut ZSTD_DCtx {
    let dctx: *mut ZSTD_DCtx = workspace as *mut ZSTD_DCtx;

    if (workspace as size_t) & 7 != 0 {
        return core::ptr::null_mut(); /* 8-aligned */
    }
    if workspaceSize < core::mem::size_of::<ZSTD_DCtx>() {
        return core::ptr::null_mut(); /* minimum size */
    }

    ZSTD_initDCtx_internal(dctx);
    (*dctx).staticSize = workspaceSize;
    (*dctx).inBuff = dctx.add(1) as *mut c_char;
    dctx
}

pub unsafe fn ZSTD_createDCtx_internal(customMem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    if (customMem.customAlloc.is_none() as c_int) ^ (customMem.customFree.is_none() as c_int) != 0 {
        return core::ptr::null_mut();
    }

    {
        let dctx: *mut ZSTD_DCtx =
            ZSTD_customMalloc(core::mem::size_of::<ZSTD_DCtx>(), customMem) as *mut ZSTD_DCtx;
        if dctx.is_null() {
            return core::ptr::null_mut();
        }
        (*dctx).customMem = customMem;
        ZSTD_initDCtx_internal(dctx);
        dctx
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    ZSTD_createDCtx_internal(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    ZSTD_createDCtx_internal(ZSTD_defaultCMem)
}

pub unsafe fn ZSTD_clearDict(dctx: *mut ZSTD_DCtx) {
    ZSTD_freeDDict((*dctx).ddictLocal);
    (*dctx).ddictLocal = core::ptr::null_mut();
    (*dctx).ddict = core::ptr::null();
    (*dctx).dictUses = ZSTD_dont_use;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> size_t {
    if dctx.is_null() {
        return 0; /* support free on NULL */
    }
    if (*dctx).staticSize != 0 {
        return ERROR(ZSTD_error_memory_allocation); /* not compatible with static DCtx */
    }
    {
        let cMem: ZSTD_customMem = (*dctx).customMem;
        ZSTD_clearDict(dctx);
        ZSTD_customFree((*dctx).inBuff as *mut c_void, cMem);
        (*dctx).inBuff = core::ptr::null_mut();
        /* ZSTD_LEGACY_SUPPORT >= 1 */
        if !(*dctx).legacyContext.is_null() {
            ZSTD_freeLegacyStreamContext((*dctx).legacyContext, (*dctx).previousLegacyVersion);
        }
        if !(*dctx).ddictSet.is_null() {
            ZSTD_freeDDictHashSet((*dctx).ddictSet, cMem);
            (*dctx).ddictSet = core::ptr::null_mut();
        }
        ZSTD_customFree(dctx as *mut c_void, cMem);
        0
    }
}

/* no longer useful */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyDCtx(dstDCtx: *mut ZSTD_DCtx, srcDCtx: *const ZSTD_DCtx) {
    let toCopy: size_t =
        (&(*dstDCtx).inBuff as *const _ as *const c_char).offset_from(dstDCtx as *const c_char)
            as size_t;
    ZSTD_memcpy(dstDCtx as *mut u8, srcDCtx as *const u8, toCopy); /* no need to copy workspace */
}

pub unsafe fn ZSTD_DCtx_selectFrameDDict(dctx: *mut ZSTD_DCtx) {
    /* assert dropped */
    if !(*dctx).ddict.is_null() {
        let frameDDict: *const ZSTD_DDict =
            ZSTD_DDictHashSet_getDDict((*dctx).ddictSet, (*dctx).fParams.dictID);
        if !frameDDict.is_null() {
            ZSTD_clearDict(dctx);
            (*dctx).dictID = (*dctx).fParams.dictID;
            (*dctx).ddict = frameDDict;
            (*dctx).dictUses = ZSTD_use_indefinitely;
        }
    }
}

/*-*************************************************************
 *   Frame header decoding
 ***************************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isFrame(buffer: *const c_void, size: size_t) -> c_uint {
    if size < ZSTD_FRAMEIDSIZE {
        return 0;
    }
    {
        let magic: U32 = MEM_readLE32(buffer as *const u8);
        if magic == ZSTD_MAGICNUMBER {
            return 1;
        }
        if (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            return 1;
        }
    }
    /* ZSTD_LEGACY_SUPPORT >= 1 */
    if ZSTD_isLegacy(buffer, size) != 0 {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isSkippableFrame(buffer: *const c_void, size: size_t) -> c_uint {
    if size < ZSTD_FRAMEIDSIZE {
        return 0;
    }
    {
        let magic: U32 = MEM_readLE32(buffer as *const u8);
        if (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            return 1;
        }
    }
    0
}

pub unsafe fn ZSTD_frameHeaderSize_internal(
    src: *const c_void,
    srcSize: size_t,
    format: ZSTD_format_e,
) -> size_t {
    let minInputSize: size_t = ZSTD_startingInputLength(format);
    if srcSize < minInputSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    {
        let fhd: BYTE = *(src as *const BYTE).add(minInputSize - 1);
        let dictID: U32 = (fhd & 3) as U32;
        let singleSegment: U32 = ((fhd >> 5) & 1) as U32;
        let fcsId: U32 = (fhd >> 6) as U32;
        minInputSize
            + ((singleSegment == 0) as size_t)
            + ZSTD_did_fieldSize[dictID as usize]
            + ZSTD_fcs_fieldSize[fcsId as usize]
            + ((singleSegment != 0 && fcsId == 0) as size_t)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_frameHeaderSize(src: *const c_void, srcSize: size_t) -> size_t {
    ZSTD_frameHeaderSize_internal(src, srcSize, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader_advanced(
    zfhPtr: *mut ZSTD_FrameHeader,
    src: *const c_void,
    srcSize: size_t,
    format: ZSTD_format_e,
) -> size_t {
    let ip: *const BYTE = src as *const BYTE;
    let minInputSize: size_t = ZSTD_startingInputLength(format);

    if srcSize > 0 {
        /* note : technically could be considered an assert() */
        if src.is_null() {
            return ERROR(ZSTD_error_GENERIC);
        }
    }
    if srcSize < minInputSize {
        if srcSize > 0 && format != ZSTD_f_zstd1_magicless {
            let toCopy: size_t = MIN(4, srcSize);
            let mut hbuf: [core::ffi::c_uchar; 4] = [0; 4];
            MEM_writeLE32(hbuf.as_mut_ptr(), ZSTD_MAGICNUMBER);
            /* assert(src != NULL) dropped */
            ZSTD_memcpy(hbuf.as_mut_ptr(), src as *const u8, toCopy);
            if MEM_readLE32(hbuf.as_ptr()) != ZSTD_MAGICNUMBER {
                /* not a zstd frame : check if it's a skippable frame */
                MEM_writeLE32(hbuf.as_mut_ptr(), ZSTD_MAGIC_SKIPPABLE_START);
                ZSTD_memcpy(hbuf.as_mut_ptr(), src as *const u8, toCopy);
                if (MEM_readLE32(hbuf.as_ptr()) & ZSTD_MAGIC_SKIPPABLE_MASK)
                    != ZSTD_MAGIC_SKIPPABLE_START
                {
                    return ERROR(ZSTD_error_prefix_unknown);
                }
            }
        }
        return minInputSize;
    }

    ZSTD_memset(zfhPtr as *mut u8, 0, core::mem::size_of::<ZSTD_FrameHeader>());
    if (format != ZSTD_f_zstd1_magicless) && (MEM_readLE32(src as *const u8) != ZSTD_MAGICNUMBER) {
        if (MEM_readLE32(src as *const u8) & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START
        {
            /* skippable frame */
            if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
                return ZSTD_SKIPPABLEHEADERSIZE; /* magic number + frame length */
            }
            ZSTD_memset(zfhPtr as *mut u8, 0, core::mem::size_of::<ZSTD_FrameHeader>());
            (*zfhPtr).frameType = ZSTD_skippableFrame;
            (*zfhPtr).dictID = MEM_readLE32(src as *const u8) - ZSTD_MAGIC_SKIPPABLE_START;
            (*zfhPtr).headerSize = ZSTD_SKIPPABLEHEADERSIZE as U32;
            (*zfhPtr).frameContentSize =
                MEM_readLE32((src as *const c_char).add(ZSTD_FRAMEIDSIZE) as *const u8)
                    as c_ulonglong;
            return 0;
        }
        return ERROR(ZSTD_error_prefix_unknown);
    }

    /* ensure there is enough `srcSize` to fully read/decode frame header */
    {
        let fhsize: size_t = ZSTD_frameHeaderSize_internal(src, srcSize, format);
        if srcSize < fhsize {
            return fhsize;
        }
        (*zfhPtr).headerSize = fhsize as U32;
    }

    {
        let fhdByte: BYTE = *ip.add(minInputSize - 1);
        let mut pos: size_t = minInputSize;
        let dictIDSizeCode: U32 = (fhdByte & 3) as U32;
        let checksumFlag: U32 = ((fhdByte >> 2) & 1) as U32;
        let singleSegment: U32 = ((fhdByte >> 5) & 1) as U32;
        let fcsID: U32 = (fhdByte >> 6) as U32;
        let mut windowSize: U64 = 0;
        let mut dictID: U32 = 0;
        let mut frameContentSize: U64 = ZSTD_CONTENTSIZE_UNKNOWN;
        if (fhdByte & 0x08) != 0 {
            return ERROR(ZSTD_error_frameParameter_unsupported); /* reserved bits, must be zero */
        }

        if singleSegment == 0 {
            let wlByte: BYTE = *ip.add(pos);
            pos += 1;
            let windowLog: U32 = (wlByte >> 3) as U32 + ZSTD_WINDOWLOG_ABSOLUTEMIN;
            if windowLog > ZSTD_WINDOWLOG_MAX as U32 {
                return ERROR(ZSTD_error_frameParameter_windowTooLarge);
            }
            windowSize = 1u64 << windowLog;
            windowSize += (windowSize >> 3) * (wlByte & 7) as U64;
        }
        match dictIDSizeCode {
            /* default / 0 : break */
            1 => {
                dictID = *ip.add(pos) as U32;
                pos += 1;
            }
            2 => {
                dictID = MEM_readLE16(ip.add(pos)) as U32;
                pos += 2;
            }
            3 => {
                dictID = MEM_readLE32(ip.add(pos));
                pos += 4;
            }
            _ => {}
        }
        match fcsID {
            0 => {
                if singleSegment != 0 {
                    frameContentSize = *ip.add(pos) as U64;
                }
            }
            1 => {
                frameContentSize = MEM_readLE16(ip.add(pos)) as U64 + 256;
            }
            2 => {
                frameContentSize = MEM_readLE32(ip.add(pos)) as U64;
            }
            3 => {
                frameContentSize = MEM_readLE64(ip.add(pos));
            }
            _ => {}
        }
        if singleSegment != 0 {
            windowSize = frameContentSize;
        }

        (*zfhPtr).frameType = ZSTD_frame;
        (*zfhPtr).frameContentSize = frameContentSize;
        (*zfhPtr).windowSize = windowSize;
        (*zfhPtr).blockSizeMax = MIN(windowSize, ZSTD_BLOCKSIZE_MAX as U64) as c_uint;
        (*zfhPtr).dictID = dictID;
        (*zfhPtr).checksumFlag = checksumFlag;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader(
    zfhPtr: *mut ZSTD_FrameHeader,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_getFrameHeader_advanced(zfhPtr, src, srcSize, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameContentSize(
    src: *const c_void,
    srcSize: size_t,
) -> c_ulonglong {
    /* ZSTD_LEGACY_SUPPORT >= 1 */
    if ZSTD_isLegacy(src, srcSize) != 0 {
        let ret: c_ulonglong = ZSTD_getDecompressedSize_legacy(src, srcSize);
        return if ret == 0 { ZSTD_CONTENTSIZE_UNKNOWN } else { ret };
    }
    {
        let mut zfh: ZSTD_FrameHeader = core::mem::zeroed();
        if ZSTD_getFrameHeader(&mut zfh, src, srcSize) != 0 {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        if zfh.frameType == ZSTD_skippableFrame {
            0
        } else {
            zfh.frameContentSize
        }
    }
}

pub unsafe fn readSkippableFrameSize(src: *const c_void, srcSize: size_t) -> size_t {
    let skippableHeaderSize: size_t = ZSTD_SKIPPABLEHEADERSIZE;
    let sizeU32: U32;

    if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    sizeU32 = MEM_readLE32((src as *const BYTE).add(ZSTD_FRAMEIDSIZE));
    if (sizeU32.wrapping_add(ZSTD_SKIPPABLEHEADERSIZE as U32)) < sizeU32 {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    {
        let skippableSize: size_t = skippableHeaderSize + sizeU32 as size_t;
        if skippableSize > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        skippableSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_readSkippableFrame(
    dst: *mut c_void,
    dstCapacity: size_t,
    magicVariant: *mut c_uint,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    {
        let magicNumber: U32 = MEM_readLE32(src as *const u8);
        let skippableFrameSize: size_t = readSkippableFrameSize(src, srcSize);
        let skippableContentSize: size_t = skippableFrameSize - ZSTD_SKIPPABLEHEADERSIZE;

        /* check input validity */
        if ZSTD_isSkippableFrame(src, srcSize) == 0 {
            return ERROR(ZSTD_error_frameParameter_unsupported);
        }
        if skippableFrameSize < ZSTD_SKIPPABLEHEADERSIZE || skippableFrameSize > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if skippableContentSize > dstCapacity {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        /* deliver payload */
        if skippableContentSize > 0 && !dst.is_null() {
            ZSTD_memcpy(
                dst as *mut u8,
                (src as *const BYTE).add(ZSTD_SKIPPABLEHEADERSIZE),
                skippableContentSize,
            );
        }
        if !magicVariant.is_null() {
            *magicVariant = magicNumber - ZSTD_MAGIC_SKIPPABLE_START;
        }
        skippableContentSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findDecompressedSize(
    mut src: *const c_void,
    mut srcSize: size_t,
) -> c_ulonglong {
    let mut totalDstSize: c_ulonglong = 0;

    while srcSize >= ZSTD_startingInputLength(ZSTD_f_zstd1) {
        let magicNumber: U32 = MEM_readLE32(src as *const u8);

        if (magicNumber & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            let skippableSize: size_t = readSkippableFrameSize(src, srcSize);
            if ZSTD_isError(skippableSize) != 0 {
                return ZSTD_CONTENTSIZE_ERROR;
            }
            /* assert dropped */
            src = (src as *const BYTE).add(skippableSize) as *const c_void;
            srcSize -= skippableSize;
            continue;
        }

        {
            let fcs: c_ulonglong = ZSTD_getFrameContentSize(src, srcSize);
            if fcs >= ZSTD_CONTENTSIZE_ERROR {
                return fcs;
            }

            if totalDstSize + fcs < totalDstSize {
                return ZSTD_CONTENTSIZE_ERROR; /* check for overflow */
            }
            totalDstSize += fcs;
        }
        /* skip to next frame */
        {
            let frameSrcSize: size_t = ZSTD_findFrameCompressedSize(src, srcSize);
            if ZSTD_isError(frameSrcSize) != 0 {
                return ZSTD_CONTENTSIZE_ERROR;
            }
            /* assert dropped */
            src = (src as *const BYTE).add(frameSrcSize) as *const c_void;
            srcSize -= frameSrcSize;
        }
    }

    if srcSize != 0 {
        return ZSTD_CONTENTSIZE_ERROR;
    }

    totalDstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDecompressedSize(
    src: *const c_void,
    srcSize: size_t,
) -> c_ulonglong {
    let ret: c_ulonglong = ZSTD_getFrameContentSize(src, srcSize);
    /* ZSTD_STATIC_ASSERT dropped */
    if ret >= ZSTD_CONTENTSIZE_ERROR {
        0
    } else {
        ret
    }
}

pub unsafe fn ZSTD_decodeFrameHeader(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    headerSize: size_t,
) -> size_t {
    let result: size_t =
        ZSTD_getFrameHeader_advanced(&mut (*dctx).fParams, src, headerSize, (*dctx).format);
    if ZSTD_isError(result) != 0 {
        return result; /* invalid header */
    }
    if result > 0 {
        return ERROR(ZSTD_error_srcSize_wrong); /* headerSize too small */
    }

    /* Reference DDict requested by frame if dctx references multiple ddicts */
    if (*dctx).refMultipleDDicts == ZSTD_rmd_refMultipleDDicts && !(*dctx).ddictSet.is_null() {
        ZSTD_DCtx_selectFrameDDict(dctx);
    }

    /* not FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION */
    if (*dctx).fParams.dictID != 0 && ((*dctx).dictID != (*dctx).fParams.dictID) {
        return ERROR(ZSTD_error_dictionary_wrong);
    }
    (*dctx).validateChecksum =
        if (*dctx).fParams.checksumFlag != 0 && (*dctx).forceIgnoreChecksum == 0 {
            1
        } else {
            0
        };
    if (*dctx).validateChecksum != 0 {
        XXH64_reset(&mut (*dctx).xxhState, 0);
    }
    (*dctx).processedCSize += headerSize as U64;
    0
}

pub unsafe fn ZSTD_errorFrameSizeInfo(ret: size_t) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = core::mem::zeroed();
    frameSizeInfo.compressedSize = ret;
    frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
    frameSizeInfo
}

pub unsafe fn ZSTD_findFrameSizeInfo(
    src: *const c_void,
    srcSize: size_t,
    format: ZSTD_format_e,
) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = core::mem::zeroed();
    ZSTD_memset(
        &mut frameSizeInfo as *mut ZSTD_frameSizeInfo as *mut u8,
        0,
        core::mem::size_of::<ZSTD_frameSizeInfo>(),
    );

    /* ZSTD_LEGACY_SUPPORT >= 1 */
    if format == ZSTD_f_zstd1 && ZSTD_isLegacy(src, srcSize) != 0 {
        return ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    }

    if format == ZSTD_f_zstd1
        && (srcSize >= ZSTD_SKIPPABLEHEADERSIZE)
        && (MEM_readLE32(src as *const u8) & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START
    {
        frameSizeInfo.compressedSize = readSkippableFrameSize(src, srcSize);
        /* assert dropped */
        frameSizeInfo
    } else {
        let mut ip: *const BYTE = src as *const BYTE;
        let ipstart: *const BYTE = ip;
        let mut remainingSize: size_t = srcSize;
        let mut nbBlocks: size_t = 0;
        let mut zfh: ZSTD_FrameHeader = core::mem::zeroed();

        /* Extract Frame Header */
        {
            let ret: size_t = ZSTD_getFrameHeader_advanced(&mut zfh, src, srcSize, format);
            if ZSTD_isError(ret) != 0 {
                return ZSTD_errorFrameSizeInfo(ret);
            }
            if ret > 0 {
                return ZSTD_errorFrameSizeInfo(ERROR(ZSTD_error_srcSize_wrong));
            }
        }

        ip = ip.add(zfh.headerSize as size_t);
        remainingSize -= zfh.headerSize as size_t;

        /* Iterate over each block */
        loop {
            let mut blockProperties: blockProperties_t = core::mem::zeroed();
            let cBlockSize: size_t =
                ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
            if ZSTD_isError(cBlockSize) != 0 {
                return ZSTD_errorFrameSizeInfo(cBlockSize);
            }

            if ZSTD_blockHeaderSize + cBlockSize > remainingSize {
                return ZSTD_errorFrameSizeInfo(ERROR(ZSTD_error_srcSize_wrong));
            }

            ip = ip.add(ZSTD_blockHeaderSize + cBlockSize);
            remainingSize -= ZSTD_blockHeaderSize + cBlockSize;
            nbBlocks += 1;

            if blockProperties.lastBlock != 0 {
                break;
            }
        }

        /* Final frame content checksum */
        if zfh.checksumFlag != 0 {
            if remainingSize < 4 {
                return ZSTD_errorFrameSizeInfo(ERROR(ZSTD_error_srcSize_wrong));
            }
            ip = ip.add(4);
        }

        frameSizeInfo.nbBlocks = nbBlocks;
        frameSizeInfo.compressedSize = ip.offset_from(ipstart) as size_t;
        frameSizeInfo.decompressedBound = if zfh.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
            zfh.frameContentSize
        } else {
            nbBlocks as c_ulonglong * zfh.blockSizeMax as c_ulonglong
        };
        frameSizeInfo
    }
}

pub unsafe fn ZSTD_findFrameCompressedSize_advanced(
    src: *const c_void,
    srcSize: size_t,
    format: ZSTD_format_e,
) -> size_t {
    let frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_findFrameSizeInfo(src, srcSize, format);
    frameSizeInfo.compressedSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findFrameCompressedSize(
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_findFrameCompressedSize_advanced(src, srcSize, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBound(
    mut src: *const c_void,
    mut srcSize: size_t,
) -> c_ulonglong {
    let mut bound: c_ulonglong = 0;
    /* Iterate over each frame */
    while srcSize > 0 {
        let frameSizeInfo: ZSTD_frameSizeInfo =
            ZSTD_findFrameSizeInfo(src, srcSize, ZSTD_f_zstd1);
        let compressedSize: size_t = frameSizeInfo.compressedSize;
        let decompressedBound: c_ulonglong = frameSizeInfo.decompressedBound;
        if ZSTD_isError(compressedSize) != 0 || decompressedBound == ZSTD_CONTENTSIZE_ERROR {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        /* assert dropped */
        src = (src as *const BYTE).add(compressedSize) as *const c_void;
        srcSize -= compressedSize;
        bound += decompressedBound;
    }
    bound
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressionMargin(
    mut src: *const c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut margin: size_t = 0;
    let mut maxBlockSize: c_uint = 0;

    /* Iterate over each frame */
    while srcSize > 0 {
        let frameSizeInfo: ZSTD_frameSizeInfo =
            ZSTD_findFrameSizeInfo(src, srcSize, ZSTD_f_zstd1);
        let compressedSize: size_t = frameSizeInfo.compressedSize;
        let decompressedBound: c_ulonglong = frameSizeInfo.decompressedBound;
        let mut zfh: ZSTD_FrameHeader = core::mem::zeroed();

        {
            let err_code = ZSTD_getFrameHeader(&mut zfh, src, srcSize);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if ZSTD_isError(compressedSize) != 0 || decompressedBound == ZSTD_CONTENTSIZE_ERROR {
            return ERROR(ZSTD_error_corruption_detected);
        }

        if zfh.frameType == ZSTD_frame {
            /* Add the frame header to our margin */
            margin += zfh.headerSize as size_t;
            /* Add the checksum to our margin */
            margin += if zfh.checksumFlag != 0 { 4 } else { 0 };
            /* Add 3 bytes per block */
            margin += 3 * frameSizeInfo.nbBlocks;

            /* Compute the max block size */
            maxBlockSize = MAX(maxBlockSize, zfh.blockSizeMax);
        } else {
            /* assert(zfh.frameType == ZSTD_skippableFrame); dropped */
            margin += compressedSize;
        }

        /* assert dropped */
        src = (src as *const BYTE).add(compressedSize) as *const c_void;
        srcSize -= compressedSize;
    }

    /* Add the max block size back to the margin. */
    margin += maxBlockSize as size_t;

    margin
}

/*-*************************************************************
 *   Frame decoding
 ***************************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_insertBlock(
    dctx: *mut ZSTD_DCtx,
    blockStart: *const c_void,
    blockSize: size_t,
) -> size_t {
    ZSTD_checkContinuity(dctx, blockStart, blockSize);
    (*dctx).previousDstEnd = (blockStart as *const c_char).add(blockSize) as *const c_void;
    blockSize
}

pub unsafe fn ZSTD_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    if srcSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if dst.is_null() {
        if srcSize == 0 {
            return 0;
        }
        return ERROR(ZSTD_error_dstBuffer_null);
    }
    ZSTD_memmove(dst as *mut u8, src as *const u8, srcSize);
    srcSize
}

pub unsafe fn ZSTD_setRleBlock(
    dst: *mut c_void,
    dstCapacity: size_t,
    b: BYTE,
    regenSize: size_t,
) -> size_t {
    if regenSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if dst.is_null() {
        if regenSize == 0 {
            return 0;
        }
        return ERROR(ZSTD_error_dstBuffer_null);
    }
    ZSTD_memset(dst as *mut u8, b as c_int, regenSize);
    regenSize
}

pub unsafe fn ZSTD_DCtx_trace_end(
    dctx: *const ZSTD_DCtx,
    uncompressedSize: U64,
    compressedSize: U64,
    streaming: c_int,
) {
    /* ZSTD_TRACE == 0 : body is empty */
    let _ = dctx;
    let _ = uncompressedSize;
    let _ = compressedSize;
    let _ = streaming;
}

pub unsafe fn ZSTD_decompressFrame(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    srcPtr: *mut *const c_void,
    srcSizePtr: *mut size_t,
) -> size_t {
    let istart: *const BYTE = *srcPtr as *const BYTE;
    let mut ip: *const BYTE = istart;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = if dstCapacity != 0 {
        ostart.add(dstCapacity)
    } else {
        ostart
    };
    let mut op: *mut BYTE = ostart;
    let mut remainingSrcSize: size_t = *srcSizePtr;

    /* check */
    if remainingSrcSize < ZSTD_FRAMEHEADERSIZE_MIN((*dctx).format) + ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Frame Header */
    {
        let frameHeaderSize: size_t = ZSTD_frameHeaderSize_internal(
            ip as *const c_void,
            ZSTD_FRAMEHEADERSIZE_PREFIX((*dctx).format),
            (*dctx).format,
        );
        if ZSTD_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if remainingSrcSize < frameHeaderSize + ZSTD_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        {
            let err_code = ZSTD_decodeFrameHeader(dctx, ip as *const c_void, frameHeaderSize);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        ip = ip.add(frameHeaderSize);
        remainingSrcSize -= frameHeaderSize;
    }

    /* Shrink the blockSizeMax if enabled */
    if (*dctx).maxBlockSizeParam != 0 {
        (*dctx).fParams.blockSizeMax =
            MIN((*dctx).fParams.blockSizeMax, (*dctx).maxBlockSizeParam as c_uint);
    }

    /* Loop on each block */
    loop {
        let mut oBlockEnd: *mut BYTE = oend;
        let decodedSize: size_t;
        let mut blockProperties: blockProperties_t = core::mem::zeroed();
        let cBlockSize: size_t =
            ZSTD_getcBlockSize(ip as *const c_void, remainingSrcSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSrcSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSrcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        if ip >= op as *const BYTE && ip < oBlockEnd as *const BYTE {
            /* We are decompressing in-place. */
            oBlockEnd = op.offset((ip as *const BYTE).offset_from(op as *const BYTE));
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                /* assert(dctx->isFrameDecompression == 1); dropped */
                decodedSize = ZSTD_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    oBlockEnd.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                    not_streaming,
                );
            }
            x if x == bt_raw => {
                /* Use oend instead of oBlockEnd because this function is safe to overlap. */
                decodedSize = ZSTD_copyRawBlock(
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_rle => {
                decodedSize = ZSTD_setRleBlock(
                    op as *mut c_void,
                    oBlockEnd.offset_from(op) as size_t,
                    *ip,
                    blockProperties.origSize as size_t,
                );
            }
            /* bt_reserved and default */
            _ => {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }
        {
            let err_code = decodedSize;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if (*dctx).validateChecksum != 0 {
            XXH64_update(&mut (*dctx).xxhState, op as *const c_void, decodedSize);
        }
        if decodedSize != 0 {
            /* support dst = NULL,0 */
            op = op.add(decodedSize);
        }
        /* assert(ip != NULL); dropped */
        ip = ip.add(cBlockSize);
        remainingSrcSize -= cBlockSize;
        if blockProperties.lastBlock != 0 {
            break;
        }
    }

    if (*dctx).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
        if (op.offset_from(ostart) as U64) != (*dctx).fParams.frameContentSize {
            return ERROR(ZSTD_error_corruption_detected);
        }
    }
    if (*dctx).fParams.checksumFlag != 0 {
        /* Frame content checksum verification */
        if remainingSrcSize < 4 {
            return ERROR(ZSTD_error_checksum_wrong);
        }
        if (*dctx).forceIgnoreChecksum == 0 {
            let checkCalc: U32 = XXH64_digest(&(*dctx).xxhState) as U32;
            let checkRead: U32;
            checkRead = MEM_readLE32(ip);
            if checkRead != checkCalc {
                return ERROR(ZSTD_error_checksum_wrong);
            }
        }
        ip = ip.add(4);
        remainingSrcSize -= 4;
    }
    ZSTD_DCtx_trace_end(
        dctx,
        op.offset_from(ostart) as U64,
        ip.offset_from(istart) as U64,
        0, /* streaming */
    );
    /* Allow caller to get size read */
    *srcPtr = ip as *const c_void;
    *srcSizePtr = remainingSrcSize;
    op.offset_from(ostart) as size_t
}

pub unsafe fn ZSTD_decompressMultiFrame(
    dctx: *mut ZSTD_DCtx,
    mut dst: *mut c_void,
    mut dstCapacity: size_t,
    mut src: *const c_void,
    mut srcSize: size_t,
    mut dict: *const c_void,
    mut dictSize: size_t,
    ddict: *const ZSTD_DDict,
) -> size_t {
    let dststart: *mut c_void = dst;
    let mut moreThan1Frame: c_int = 0;

    /* assert(dict==NULL || ddict==NULL); dropped */

    if !ddict.is_null() {
        dict = ZSTD_DDict_dictContent(ddict);
        dictSize = ZSTD_DDict_dictSize(ddict);
    }

    while srcSize >= ZSTD_startingInputLength((*dctx).format) {
        /* ZSTD_LEGACY_SUPPORT >= 1 */
        if (*dctx).format == ZSTD_f_zstd1 && ZSTD_isLegacy(src, srcSize) != 0 {
            let decodedSize: size_t;
            let frameSize: size_t = ZSTD_findFrameCompressedSizeLegacy(src, srcSize);
            if ZSTD_isError(frameSize) != 0 {
                return frameSize;
            }
            if (*dctx).staticSize != 0 {
                return ERROR(ZSTD_error_memory_allocation); /* legacy not compatible with static dctx */
            }

            decodedSize = ZSTD_decompressLegacy(dst, dstCapacity, src, frameSize, dict, dictSize);
            if ZSTD_isError(decodedSize) != 0 {
                return decodedSize;
            }

            {
                let expectedSize: c_ulonglong = ZSTD_getFrameContentSize(src, srcSize);
                if expectedSize == ZSTD_CONTENTSIZE_ERROR {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                if expectedSize != ZSTD_CONTENTSIZE_UNKNOWN {
                    if expectedSize != decodedSize as c_ulonglong {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                }
            }

            /* assert dropped */
            dst = (dst as *mut BYTE).add(decodedSize) as *mut c_void;
            dstCapacity -= decodedSize;

            src = (src as *const BYTE).add(frameSize) as *const c_void;
            srcSize -= frameSize;

            continue;
        }

        if (*dctx).format == ZSTD_f_zstd1 && srcSize >= 4 {
            let magicNumber: U32 = MEM_readLE32(src as *const u8);
            if (magicNumber & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
                /* skippable frame detected : skip it */
                let skippableSize: size_t = readSkippableFrameSize(src, srcSize);
                {
                    let err_code = skippableSize;
                    if ERR_isError(err_code) != 0 {
                        return err_code;
                    }
                }
                /* assert dropped */
                src = (src as *const BYTE).add(skippableSize) as *const c_void;
                srcSize -= skippableSize;
                continue; /* check next frame */
            }
        }

        if !ddict.is_null() {
            /* we were called from ZSTD_decompress_usingDDict */
            let err_code = ZSTD_decompressBegin_usingDDict(dctx, ddict);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        } else {
            /* initialize correctly with no dict if dict == NULL */
            let err_code = ZSTD_decompressBegin_usingDict(dctx, dict, dictSize);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        ZSTD_checkContinuity(dctx, dst, dstCapacity);

        {
            let res: size_t = ZSTD_decompressFrame(dctx, dst, dstCapacity, &mut src, &mut srcSize);
            if (ZSTD_getErrorCode(res) == ZSTD_error_prefix_unknown) && (moreThan1Frame == 1) {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if ZSTD_isError(res) != 0 {
                return res;
            }
            /* assert dropped */
            if res != 0 {
                dst = (dst as *mut BYTE).add(res) as *mut c_void;
            }
            dstCapacity -= res;
        }
        moreThan1Frame = 1;
    }

    if srcSize != 0 {
        return ERROR(ZSTD_error_srcSize_wrong); /* input not entirely consumed */
    }

    (dst as *mut BYTE).offset_from(dststart as *mut BYTE) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress_usingDict(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    ZSTD_decompressMultiFrame(
        dctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        dict,
        dictSize,
        core::ptr::null(),
    )
}

pub unsafe fn ZSTD_getDDict(dctx: *mut ZSTD_DCtx) -> *const ZSTD_DDict {
    match (*dctx).dictUses {
        ZSTD_use_indefinitely => (*dctx).ddict,
        ZSTD_use_once => {
            (*dctx).dictUses = ZSTD_dont_use;
            (*dctx).ddict
        }
        /* ZSTD_dont_use and default */
        _ => {
            ZSTD_clearDict(dctx);
            core::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressDCtx(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_decompress_usingDDict(dctx, dst, dstCapacity, src, srcSize, ZSTD_getDDict(dctx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    /* ZSTD_HEAPMODE >= 1 */
    let regenSize: size_t;
    let dctx: *mut ZSTD_DCtx = ZSTD_createDCtx_internal(ZSTD_defaultCMem);
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTD_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTD_freeDCtx(dctx);
    regenSize
}

/*-**************************************
*   Advanced Streaming Decompression API
*   Bufferless and synchronous
****************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> size_t {
    (*dctx).expected
}

pub unsafe fn ZSTD_nextSrcSizeToDecompressWithInputSize(
    dctx: *mut ZSTD_DCtx,
    inputSize: size_t,
) -> size_t {
    if !((*dctx).stage == ZSTDds_decompressBlock || (*dctx).stage == ZSTDds_decompressLastBlock) {
        return (*dctx).expected;
    }
    if (*dctx).bType != bt_raw {
        return (*dctx).expected;
    }
    BOUNDED(1, inputSize, (*dctx).expected)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_nextInputType(dctx: *mut ZSTD_DCtx) -> ZSTD_nextInputType_e {
    match (*dctx).stage {
        /* default : should not happen -> falls through to getFrameHeaderSize */
        ZSTDds_getFrameHeaderSize => ZSTDnit_frameHeader,
        ZSTDds_decodeFrameHeader => ZSTDnit_frameHeader,
        ZSTDds_decodeBlockHeader => ZSTDnit_blockHeader,
        ZSTDds_decompressBlock => ZSTDnit_block,
        ZSTDds_decompressLastBlock => ZSTDnit_lastBlock,
        ZSTDds_checkChecksum => ZSTDnit_checksum,
        ZSTDds_decodeSkippableHeader => ZSTDnit_skippableFrame,
        ZSTDds_skipFrame => ZSTDnit_skippableFrame,
        _ => ZSTDnit_frameHeader,
    }
}

pub unsafe fn ZSTD_isSkipFrame(dctx: *mut ZSTD_DCtx) -> c_int {
    ((*dctx).stage == ZSTDds_skipFrame) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressContinue(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    /* Sanity check */
    if srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ZSTD_checkContinuity(dctx, dst, dstCapacity);

    (*dctx).processedCSize += srcSize as U64;

    match (*dctx).stage {
        ZSTDds_getFrameHeaderSize => {
            /* assert(src != NULL); dropped */
            if (*dctx).format == ZSTD_f_zstd1 {
                /* allows header */
                /* assert(srcSize >= ZSTD_FRAMEIDSIZE); dropped */
                if (MEM_readLE32(src as *const u8) & ZSTD_MAGIC_SKIPPABLE_MASK)
                    == ZSTD_MAGIC_SKIPPABLE_START
                {
                    /* skippable frame */
                    ZSTD_memcpy((*dctx).headerBuffer.as_mut_ptr(), src as *const u8, srcSize);
                    (*dctx).expected = ZSTD_SKIPPABLEHEADERSIZE - srcSize;
                    (*dctx).stage = ZSTDds_decodeSkippableHeader;
                    return 0;
                }
            }
            (*dctx).headerSize = ZSTD_frameHeaderSize_internal(src, srcSize, (*dctx).format);
            if ZSTD_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            ZSTD_memcpy((*dctx).headerBuffer.as_mut_ptr(), src as *const u8, srcSize);
            (*dctx).expected = (*dctx).headerSize - srcSize;
            (*dctx).stage = ZSTDds_decodeFrameHeader;
            0
        }

        ZSTDds_decodeFrameHeader => {
            /* assert(src != NULL); dropped */
            ZSTD_memcpy(
                (*dctx).headerBuffer.as_mut_ptr().add((*dctx).headerSize - srcSize),
                src as *const u8,
                srcSize,
            );
            {
                let err_code = ZSTD_decodeFrameHeader(
                    dctx,
                    (*dctx).headerBuffer.as_ptr() as *const c_void,
                    (*dctx).headerSize,
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            (*dctx).expected = ZSTD_blockHeaderSize;
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            0
        }

        ZSTDds_decodeBlockHeader => {
            let mut bp: blockProperties_t = core::mem::zeroed();
            let cBlockSize: size_t =
                ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
            if ZSTD_isError(cBlockSize) != 0 {
                return cBlockSize;
            }
            if cBlockSize > (*dctx).fParams.blockSizeMax as size_t {
                return ERROR(ZSTD_error_corruption_detected); /* Block Size Exceeds Maximum */
            }
            (*dctx).expected = cBlockSize;
            (*dctx).bType = bp.blockType;
            (*dctx).rleSize = bp.origSize as size_t;
            if cBlockSize != 0 {
                (*dctx).stage = if bp.lastBlock != 0 {
                    ZSTDds_decompressLastBlock
                } else {
                    ZSTDds_decompressBlock
                };
                return 0;
            }
            /* empty block */
            if bp.lastBlock != 0 {
                if (*dctx).fParams.checksumFlag != 0 {
                    (*dctx).expected = 4;
                    (*dctx).stage = ZSTDds_checkChecksum;
                } else {
                    (*dctx).expected = 0; /* end of frame */
                    (*dctx).stage = ZSTDds_getFrameHeaderSize;
                }
            } else {
                (*dctx).expected = ZSTD_blockHeaderSize; /* jump to next header */
                (*dctx).stage = ZSTDds_decodeBlockHeader;
            }
            0
        }

        ZSTDds_decompressLastBlock | ZSTDds_decompressBlock => {
            let rSize: size_t;
            match (*dctx).bType {
                x if x == bt_compressed => {
                    /* assert(dctx->isFrameDecompression == 1); dropped */
                    rSize = ZSTD_decompressBlock_internal(
                        dctx, dst, dstCapacity, src, srcSize, is_streaming,
                    );
                    (*dctx).expected = 0; /* Streaming not supported */
                }
                x if x == bt_raw => {
                    /* assert(srcSize <= dctx->expected); dropped */
                    rSize = ZSTD_copyRawBlock(dst, dstCapacity, src, srcSize);
                    {
                        let err_code = rSize;
                        if ERR_isError(err_code) != 0 {
                            return err_code;
                        }
                    }
                    /* assert(rSize == srcSize); dropped */
                    (*dctx).expected -= rSize;
                }
                x if x == bt_rle => {
                    rSize = ZSTD_setRleBlock(
                        dst,
                        dstCapacity,
                        *(src as *const BYTE),
                        (*dctx).rleSize,
                    );
                    (*dctx).expected = 0; /* Streaming not supported */
                }
                /* bt_reserved and default */
                _ => {
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            {
                let err_code = rSize;
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            if rSize > (*dctx).fParams.blockSizeMax as size_t {
                return ERROR(ZSTD_error_corruption_detected); /* Decompressed Block Size Exceeds Maximum */
            }
            (*dctx).decodedSize += rSize as U64;
            if (*dctx).validateChecksum != 0 {
                XXH64_update(&mut (*dctx).xxhState, dst as *const c_void, rSize);
            }
            (*dctx).previousDstEnd = (dst as *const c_char).add(rSize) as *const c_void;

            /* Stay on the same stage until we are finished streaming the block. */
            if (*dctx).expected > 0 {
                return rSize;
            }

            if (*dctx).stage == ZSTDds_decompressLastBlock {
                /* end of frame */
                if (*dctx).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
                    && (*dctx).decodedSize != (*dctx).fParams.frameContentSize
                {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                if (*dctx).fParams.checksumFlag != 0 {
                    /* another round for frame checksum */
                    (*dctx).expected = 4;
                    (*dctx).stage = ZSTDds_checkChecksum;
                } else {
                    ZSTD_DCtx_trace_end(
                        dctx,
                        (*dctx).decodedSize,
                        (*dctx).processedCSize,
                        1, /* streaming */
                    );
                    (*dctx).expected = 0; /* ends here */
                    (*dctx).stage = ZSTDds_getFrameHeaderSize;
                }
            } else {
                (*dctx).stage = ZSTDds_decodeBlockHeader;
                (*dctx).expected = ZSTD_blockHeaderSize;
            }
            rSize
        }

        ZSTDds_checkChecksum => {
            /* assert(srcSize == 4); dropped */
            if (*dctx).validateChecksum != 0 {
                let h32: U32 = XXH64_digest(&(*dctx).xxhState) as U32;
                let check32: U32 = MEM_readLE32(src as *const u8);
                if check32 != h32 {
                    return ERROR(ZSTD_error_checksum_wrong);
                }
            }
            ZSTD_DCtx_trace_end(dctx, (*dctx).decodedSize, (*dctx).processedCSize, 1);
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            0
        }

        ZSTDds_decodeSkippableHeader => {
            /* assert(src != NULL); assert(srcSize <= ZSTD_SKIPPABLEHEADERSIZE); dropped */
            /* assert(dctx->format != ZSTD_f_zstd1_magicless); dropped */
            ZSTD_memcpy(
                (*dctx).headerBuffer.as_mut_ptr().add(ZSTD_SKIPPABLEHEADERSIZE - srcSize),
                src as *const u8,
                srcSize,
            );
            (*dctx).expected = MEM_readLE32(
                (*dctx).headerBuffer.as_ptr().add(ZSTD_FRAMEIDSIZE),
            ) as size_t;
            (*dctx).stage = ZSTDds_skipFrame;
            0
        }

        ZSTDds_skipFrame => {
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            0
        }

        _ => {
            /* default : impossible */
            ERROR(ZSTD_error_GENERIC)
        }
    }
}

pub unsafe fn ZSTD_refDictContent(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).virtualStart = (dict as *const c_char).offset(
        -(((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).prefixStart as *const c_char)),
    ) as *const c_void;
    (*dctx).prefixStart = dict;
    (*dctx).previousDstEnd = (dict as *const c_char).add(dictSize) as *const c_void;
    /* FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION not defined */
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_loadDEntropy(
    entropy: *mut ZSTD_entropyDTables_t,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    let mut dictPtr: *const BYTE = dict as *const BYTE;
    let dictEnd: *const BYTE = dictPtr.add(dictSize);

    if dictSize <= 8 {
        return ERROR(ZSTD_error_dictionary_corrupted); /* dict is too small */
    }
    /* assert(MEM_readLE32(dict) == ZSTD_MAGIC_DICTIONARY); dropped */
    dictPtr = dictPtr.add(8); /* skip header = magic + dictID */

    /* ZSTD_STATIC_ASSERTs dropped */
    {
        let workspace: *mut c_void = (*entropy).LLTable.as_mut_ptr() as *mut c_void;
        let workspaceSize: size_t = core::mem::size_of_val(&(*entropy).LLTable)
            + core::mem::size_of_val(&(*entropy).OFTable)
            + core::mem::size_of_val(&(*entropy).MLTable);
        /* not HUF_FORCE_DECOMPRESS_X1 : use X2 variant */
        let hSize: size_t = HUF_readDTableX2_wksp(
            (*entropy).hufTable.as_mut_ptr(),
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
            workspace,
            workspaceSize,
            0, /* flags */
        );
        if HUF_isError(hSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dictPtr = dictPtr.add(hSize);
    }

    {
        let mut offcodeNCount: [core::ffi::c_short; (MaxOff + 1) as usize] =
            [0; (MaxOff + 1) as usize];
        let mut offcodeMaxValue: c_uint = MaxOff;
        let mut offcodeLog: c_uint = 0;
        let offcodeHeaderSize: size_t = FSE_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
        );
        if FSE_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeMaxValue > MaxOff {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        ZSTD_buildFSETable(
            (*entropy).OFTable.as_mut_ptr(),
            offcodeNCount.as_ptr(),
            offcodeMaxValue,
            OF_base.as_ptr(),
            OF_bits.as_ptr(),
            offcodeLog,
            (*entropy).workspace.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*entropy).workspace),
            0, /* bmi2 */
        );
        dictPtr = dictPtr.add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount: [core::ffi::c_short; (MaxML + 1) as usize] =
            [0; (MaxML + 1) as usize];
        let mut matchlengthMaxValue: c_uint = MaxML;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize: size_t = FSE_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
        );
        if FSE_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthMaxValue > MaxML {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        ZSTD_buildFSETable(
            (*entropy).MLTable.as_mut_ptr(),
            matchlengthNCount.as_ptr(),
            matchlengthMaxValue,
            ML_base.as_ptr(),
            ML_bits.as_ptr(),
            matchlengthLog,
            (*entropy).workspace.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*entropy).workspace),
            0, /* bmi2 */
        );
        dictPtr = dictPtr.add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount: [core::ffi::c_short; (MaxLL + 1) as usize] =
            [0; (MaxLL + 1) as usize];
        let mut litlengthMaxValue: c_uint = MaxLL;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize: size_t = FSE_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
        );
        if FSE_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthMaxValue > MaxLL {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        ZSTD_buildFSETable(
            (*entropy).LLTable.as_mut_ptr(),
            litlengthNCount.as_ptr(),
            litlengthMaxValue,
            LL_base.as_ptr(),
            LL_bits.as_ptr(),
            litlengthLog,
            (*entropy).workspace.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*entropy).workspace),
            0, /* bmi2 */
        );
        dictPtr = dictPtr.add(litlengthHeaderSize);
    }

    if dictPtr.add(12) > dictEnd {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    {
        let mut i: c_int;
        let dictContentSize: size_t = dictEnd.offset_from(dictPtr.add(12)) as size_t;
        i = 0;
        while i < 3 {
            let rep: U32 = MEM_readLE32(dictPtr);
            dictPtr = dictPtr.add(4);
            if rep == 0 || rep as size_t > dictContentSize {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
            (*entropy).rep[i as usize] = rep;
            i += 1;
        }
    }

    dictPtr.offset_from(dict as *const BYTE) as size_t
}

pub unsafe fn ZSTD_decompress_insertDictionary(
    dctx: *mut ZSTD_DCtx,
    mut dict: *const c_void,
    mut dictSize: size_t,
) -> size_t {
    if dictSize < 8 {
        return ZSTD_refDictContent(dctx, dict, dictSize);
    }
    {
        let magic: U32 = MEM_readLE32(dict as *const u8);
        if magic != ZSTD_MAGIC_DICTIONARY {
            return ZSTD_refDictContent(dctx, dict, dictSize); /* pure content mode */
        }
    }
    (*dctx).dictID = MEM_readLE32((dict as *const c_char).add(ZSTD_FRAMEIDSIZE) as *const u8);

    /* load entropy tables */
    {
        let eSize: size_t = ZSTD_loadDEntropy(&mut (*dctx).entropy, dict, dictSize);
        if ZSTD_isError(eSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dict = (dict as *const c_char).add(eSize) as *const c_void;
        dictSize -= eSize;
    }
    (*dctx).litEntropy = 1;
    (*dctx).fseEntropy = 1;

    /* reference dictionary content */
    ZSTD_refDictContent(dctx, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin(dctx: *mut ZSTD_DCtx) -> size_t {
    /* assert(dctx != NULL); dropped */
    /* ZSTD_TRACE == 0 */
    (*dctx).expected = ZSTD_startingInputLength((*dctx).format);
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).processedCSize = 0;
    (*dctx).decodedSize = 0;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).prefixStart = core::ptr::null();
    (*dctx).virtualStart = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).entropy.hufTable[0] =
        ((ZSTD_HUFFDTABLE_CAPACITY_LOG).wrapping_mul(0x1000001)) as HUF_DTable;
    (*dctx).litEntropy = 0;
    (*dctx).fseEntropy = 0;
    (*dctx).dictID = 0;
    (*dctx).bType = bt_reserved;
    (*dctx).isFrameDecompression = 1;
    /* ZSTD_STATIC_ASSERT dropped */
    ZSTD_memcpy(
        (*dctx).entropy.rep.as_mut_ptr() as *mut u8,
        repStartValue.as_ptr() as *const u8,
        core::mem::size_of_val(&repStartValue),
    );
    (*dctx).LLTptr = (*dctx).entropy.LLTable.as_ptr();
    (*dctx).MLTptr = (*dctx).entropy.MLTable.as_ptr();
    (*dctx).OFTptr = (*dctx).entropy.OFTable.as_ptr();
    (*dctx).HUFptr = (*dctx).entropy.hufTable.as_ptr();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin_usingDict(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    {
        let err_code = ZSTD_decompressBegin(dctx);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    if !dict.is_null() && dictSize != 0 {
        if ZSTD_isError(ZSTD_decompress_insertDictionary(dctx, dict, dictSize)) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin_usingDDict(
    dctx: *mut ZSTD_DCtx,
    ddict: *const ZSTD_DDict,
) -> size_t {
    /* assert(dctx != NULL); dropped */
    if !ddict.is_null() {
        let dictStart: *const c_char = ZSTD_DDict_dictContent(ddict) as *const c_char;
        let dictSize: size_t = ZSTD_DDict_dictSize(ddict);
        let dictEnd: *const c_void = dictStart.add(dictSize) as *const c_void;
        (*dctx).ddictIsCold = ((*dctx).dictEnd != dictEnd) as c_int;
    }
    {
        let err_code = ZSTD_decompressBegin(dctx);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    if !ddict.is_null() {
        /* NULL ddict is equivalent to no dictionary */
        ZSTD_copyDDictParameters(dctx, ddict);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDict(dict: *const c_void, dictSize: size_t) -> c_uint {
    if dictSize < 8 {
        return 0;
    }
    if MEM_readLE32(dict as *const u8) != ZSTD_MAGIC_DICTIONARY {
        return 0;
    }
    MEM_readLE32((dict as *const c_char).add(ZSTD_FRAMEIDSIZE) as *const u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromFrame(src: *const c_void, srcSize: size_t) -> c_uint {
    let mut zfp: ZSTD_FrameHeader = ZSTD_FrameHeader {
        frameContentSize: 0,
        windowSize: 0,
        blockSizeMax: 0,
        frameType: ZSTD_frame,
        headerSize: 0,
        dictID: 0,
        checksumFlag: 0,
        _reserved1: 0,
        _reserved2: 0,
    };
    let hError: size_t = ZSTD_getFrameHeader(&mut zfp, src, srcSize);
    if ZSTD_isError(hError) != 0 {
        return 0;
    }
    zfp.dictID
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress_usingDDict(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    ddict: *const ZSTD_DDict,
) -> size_t {
    /* pass content and size in case legacy frames are encountered */
    ZSTD_decompressMultiFrame(
        dctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        core::ptr::null(),
        0,
        ddict,
    )
}

/*=====================================
*   Streaming decompression
*====================================*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDStream() -> *mut ZSTD_DStream {
    ZSTD_createDCtx_internal(ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDStream(
    workspace: *mut c_void,
    workspaceSize: size_t,
) -> *mut ZSTD_DStream {
    ZSTD_initStaticDCtx(workspace, workspaceSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDStream_advanced(
    customMem: ZSTD_customMem,
) -> *mut ZSTD_DStream {
    ZSTD_createDCtx_internal(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDStream(zds: *mut ZSTD_DStream) -> size_t {
    ZSTD_freeDCtx(zds)
}

/* ***  Initialization  *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DStreamInSize() -> size_t {
    ZSTD_BLOCKSIZE_MAX as size_t + ZSTD_blockHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DStreamOutSize() -> size_t {
    ZSTD_BLOCKSIZE_MAX as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary_advanced(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> size_t {
    if (*dctx).streamStage != zdss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    ZSTD_clearDict(dctx);
    if !dict.is_null() && dictSize != 0 {
        (*dctx).ddictLocal = ZSTD_createDDict_advanced(
            dict,
            dictSize,
            dictLoadMethod,
            dictContentType,
            (*dctx).customMem,
        );
        if (*dctx).ddictLocal.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        (*dctx).ddict = (*dctx).ddictLocal;
        (*dctx).dictUses = ZSTD_use_indefinitely;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary_byReference(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    ZSTD_DCtx_loadDictionary_advanced(dctx, dict, dictSize, ZSTD_dlm_byRef, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    ZSTD_DCtx_loadDictionary_advanced(dctx, dict, dictSize, ZSTD_dlm_byCopy, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refPrefix_advanced(
    dctx: *mut ZSTD_DCtx,
    prefix: *const c_void,
    prefixSize: size_t,
    dictContentType: ZSTD_dictContentType_e,
) -> size_t {
    {
        let err_code =
            ZSTD_DCtx_loadDictionary_advanced(dctx, prefix, prefixSize, ZSTD_dlm_byRef, dictContentType);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    (*dctx).dictUses = ZSTD_use_once;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refPrefix(
    dctx: *mut ZSTD_DCtx,
    prefix: *const c_void,
    prefixSize: size_t,
) -> size_t {
    ZSTD_DCtx_refPrefix_advanced(dctx, prefix, prefixSize, ZSTD_dct_rawContent)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDict(
    zds: *mut ZSTD_DStream,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    {
        let err_code = ZSTD_DCtx_reset(zds, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_DCtx_loadDictionary(zds, dict, dictSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_startingInputLength((*zds).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream(zds: *mut ZSTD_DStream) -> size_t {
    {
        let err_code = ZSTD_DCtx_reset(zds, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_DCtx_refDDict(zds, core::ptr::null());
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_startingInputLength((*zds).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDDict(
    dctx: *mut ZSTD_DStream,
    ddict: *const ZSTD_DDict,
) -> size_t {
    {
        let err_code = ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_DCtx_refDDict(dctx, ddict);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_startingInputLength((*dctx).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetDStream(dctx: *mut ZSTD_DStream) -> size_t {
    {
        let err_code = ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_startingInputLength((*dctx).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refDDict(
    dctx: *mut ZSTD_DCtx,
    ddict: *const ZSTD_DDict,
) -> size_t {
    if (*dctx).streamStage != zdss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    ZSTD_clearDict(dctx);
    if !ddict.is_null() {
        (*dctx).ddict = ddict;
        (*dctx).dictUses = ZSTD_use_indefinitely;
        if (*dctx).refMultipleDDicts == ZSTD_rmd_refMultipleDDicts {
            if (*dctx).ddictSet.is_null() {
                (*dctx).ddictSet = ZSTD_createDDictHashSet((*dctx).customMem);
                if (*dctx).ddictSet.is_null() {
                    return ERROR(ZSTD_error_memory_allocation);
                }
            }
            /* assert(!dctx->staticSize); dropped */
            let err_code =
                ZSTD_DDictHashSet_addDDict((*dctx).ddictSet, ddict, (*dctx).customMem);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setMaxWindowSize(
    dctx: *mut ZSTD_DCtx,
    maxWindowSize: size_t,
) -> size_t {
    let bounds: ZSTD_bounds = ZSTD_dParam_getBounds(ZSTD_d_windowLogMax);
    let min: size_t = 1usize << bounds.lowerBound;
    let max: size_t = 1usize << bounds.upperBound;
    if (*dctx).streamStage != zdss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    if maxWindowSize < min {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if maxWindowSize > max {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    (*dctx).maxWindowSize = maxWindowSize;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setFormat(
    dctx: *mut ZSTD_DCtx,
    format: ZSTD_format_e,
) -> size_t {
    ZSTD_DCtx_setParameter(dctx, ZSTD_d_format, format as c_int)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_dParam_getBounds(dParam: ZSTD_dParameter) -> ZSTD_bounds {
    let mut bounds: ZSTD_bounds = ZSTD_bounds {
        error: 0,
        lowerBound: 0,
        upperBound: 0,
    };
    match dParam {
        ZSTD_d_windowLogMax => {
            bounds.lowerBound = ZSTD_WINDOWLOG_ABSOLUTEMIN as c_int;
            bounds.upperBound = ZSTD_WINDOWLOG_MAX;
            return bounds;
        }
        ZSTD_d_format => {
            bounds.lowerBound = ZSTD_f_zstd1 as c_int;
            bounds.upperBound = ZSTD_f_zstd1_magicless as c_int;
            /* ZSTD_STATIC_ASSERT dropped */
            return bounds;
        }
        ZSTD_d_stableOutBuffer => {
            bounds.lowerBound = ZSTD_bm_buffered as c_int;
            bounds.upperBound = ZSTD_bm_stable as c_int;
            return bounds;
        }
        ZSTD_d_forceIgnoreChecksum => {
            bounds.lowerBound = ZSTD_d_validateChecksum as c_int;
            bounds.upperBound = ZSTD_d_ignoreChecksum as c_int;
            return bounds;
        }
        ZSTD_d_refMultipleDDicts => {
            bounds.lowerBound = ZSTD_rmd_refSingleDDict as c_int;
            bounds.upperBound = ZSTD_rmd_refMultipleDDicts as c_int;
            return bounds;
        }
        ZSTD_d_disableHuffmanAssembly => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }
        ZSTD_d_maxBlockSize => {
            bounds.lowerBound = ZSTD_BLOCKSIZE_MAX_MIN;
            bounds.upperBound = ZSTD_BLOCKSIZE_MAX as c_int;
            return bounds;
        }
        _ => {}
    }
    bounds.error = ERROR(ZSTD_error_parameter_unsupported);
    bounds
}

pub unsafe fn ZSTD_dParam_withinBounds(dParam: ZSTD_dParameter, value: c_int) -> c_int {
    let bounds: ZSTD_bounds = ZSTD_dParam_getBounds(dParam);
    if ZSTD_isError(bounds.error) != 0 {
        return 0;
    }
    if value < bounds.lowerBound {
        return 0;
    }
    if value > bounds.upperBound {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_getParameter(
    dctx: *mut ZSTD_DCtx,
    param: ZSTD_dParameter,
    value: *mut c_int,
) -> size_t {
    match param {
        ZSTD_d_windowLogMax => {
            *value = ZSTD_highbit32((*dctx).maxWindowSize as U32) as c_int;
            return 0;
        }
        ZSTD_d_format => {
            *value = (*dctx).format as c_int;
            return 0;
        }
        ZSTD_d_stableOutBuffer => {
            *value = (*dctx).outBufferMode as c_int;
            return 0;
        }
        ZSTD_d_forceIgnoreChecksum => {
            *value = (*dctx).forceIgnoreChecksum as c_int;
            return 0;
        }
        ZSTD_d_refMultipleDDicts => {
            *value = (*dctx).refMultipleDDicts as c_int;
            return 0;
        }
        ZSTD_d_disableHuffmanAssembly => {
            *value = (*dctx).disableHufAsm as c_int;
            return 0;
        }
        ZSTD_d_maxBlockSize => {
            *value = (*dctx).maxBlockSizeParam;
            return 0;
        }
        _ => {}
    }
    ERROR(ZSTD_error_parameter_unsupported)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setParameter(
    dctx: *mut ZSTD_DCtx,
    dParam: ZSTD_dParameter,
    mut value: c_int,
) -> size_t {
    if (*dctx).streamStage != zdss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    match dParam {
        ZSTD_d_windowLogMax => {
            if value == 0 {
                value = ZSTD_WINDOWLOG_LIMIT_DEFAULT;
            }
            if ZSTD_dParam_withinBounds(ZSTD_d_windowLogMax, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).maxWindowSize = 1usize << value;
            return 0;
        }
        ZSTD_d_format => {
            if ZSTD_dParam_withinBounds(ZSTD_d_format, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).format = value as ZSTD_format_e;
            return 0;
        }
        ZSTD_d_stableOutBuffer => {
            if ZSTD_dParam_withinBounds(ZSTD_d_stableOutBuffer, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).outBufferMode = value as ZSTD_bufferMode_e;
            return 0;
        }
        ZSTD_d_forceIgnoreChecksum => {
            if ZSTD_dParam_withinBounds(ZSTD_d_forceIgnoreChecksum, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).forceIgnoreChecksum = value as ZSTD_forceIgnoreChecksum_e;
            return 0;
        }
        ZSTD_d_refMultipleDDicts => {
            if ZSTD_dParam_withinBounds(ZSTD_d_refMultipleDDicts, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            if (*dctx).staticSize != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            (*dctx).refMultipleDDicts = value as ZSTD_refMultipleDDicts_e;
            return 0;
        }
        ZSTD_d_disableHuffmanAssembly => {
            if ZSTD_dParam_withinBounds(ZSTD_d_disableHuffmanAssembly, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).disableHufAsm = (value != 0) as c_int;
            return 0;
        }
        ZSTD_d_maxBlockSize => {
            if value != 0 && ZSTD_dParam_withinBounds(ZSTD_d_maxBlockSize, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).maxBlockSizeParam = value;
            return 0;
        }
        _ => {}
    }
    ERROR(ZSTD_error_parameter_unsupported)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_reset(
    dctx: *mut ZSTD_DCtx,
    reset: ZSTD_ResetDirective,
) -> size_t {
    if (reset == ZSTD_reset_session_only) || (reset == ZSTD_reset_session_and_parameters) {
        (*dctx).streamStage = zdss_init;
        (*dctx).noForwardProgress = 0;
        (*dctx).isFrameDecompression = 1;
    }
    if (reset == ZSTD_reset_parameters) || (reset == ZSTD_reset_session_and_parameters) {
        if (*dctx).streamStage != zdss_init {
            return ERROR(ZSTD_error_stage_wrong);
        }
        ZSTD_clearDict(dctx);
        ZSTD_DCtx_resetParameters(dctx);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DStream(dctx: *const ZSTD_DStream) -> size_t {
    ZSTD_sizeof_DCtx(dctx)
}

pub unsafe fn ZSTD_decodingBufferSize_internal(
    windowSize: c_ulonglong,
    frameContentSize: c_ulonglong,
    blockSizeMax: size_t,
) -> size_t {
    let blockSize: size_t = MIN(
        MIN(windowSize, ZSTD_BLOCKSIZE_MAX as c_ulonglong) as size_t,
        blockSizeMax,
    );
    let neededRBSize: c_ulonglong = windowSize
        + (blockSize as c_ulonglong * 2)
        + (WILDCOPY_OVERLENGTH as c_ulonglong * 2);
    let neededSize: c_ulonglong = MIN(frameContentSize, neededRBSize);
    let minRBSize: size_t = neededSize as size_t;
    if minRBSize as c_ulonglong != neededSize {
        return ERROR(ZSTD_error_frameParameter_windowTooLarge);
    }
    minRBSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodingBufferSize_min(
    windowSize: c_ulonglong,
    frameContentSize: c_ulonglong,
) -> size_t {
    ZSTD_decodingBufferSize_internal(windowSize, frameContentSize, ZSTD_BLOCKSIZE_MAX as size_t)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize(windowSize: size_t) -> size_t {
    let blockSize: size_t = MIN(windowSize, ZSTD_BLOCKSIZE_MAX as size_t);
    let inBuffSize: size_t = blockSize; /* no block can be larger */
    let outBuffSize: size_t =
        ZSTD_decodingBufferSize_min(windowSize as c_ulonglong, ZSTD_CONTENTSIZE_UNKNOWN);
    ZSTD_estimateDCtxSize() + inBuffSize + outBuffSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize_fromFrame(
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let windowSizeMax: U32 = 1u32 << ZSTD_WINDOWLOG_MAX;
    let mut zfh: ZSTD_FrameHeader = core::mem::zeroed();
    let err: size_t = ZSTD_getFrameHeader(&mut zfh, src, srcSize);
    if ZSTD_isError(err) != 0 {
        return err;
    }
    if err > 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if zfh.windowSize > windowSizeMax as c_ulonglong {
        return ERROR(ZSTD_error_frameParameter_windowTooLarge);
    }
    ZSTD_estimateDStreamSize(zfh.windowSize as size_t)
}

/* *****   Decompression   ***** */

pub unsafe fn ZSTD_DCtx_isOverflow(
    zds: *mut ZSTD_DStream,
    neededInBuffSize: size_t,
    neededOutBuffSize: size_t,
) -> c_int {
    (((*zds).inBuffSize + (*zds).outBuffSize)
        >= (neededInBuffSize + neededOutBuffSize) * ZSTD_WORKSPACETOOLARGE_FACTOR as size_t)
        as c_int
}

pub unsafe fn ZSTD_DCtx_updateOversizedDuration(
    zds: *mut ZSTD_DStream,
    neededInBuffSize: size_t,
    neededOutBuffSize: size_t,
) {
    if ZSTD_DCtx_isOverflow(zds, neededInBuffSize, neededOutBuffSize) != 0 {
        (*zds).oversizedDuration += 1;
    } else {
        (*zds).oversizedDuration = 0;
    }
}

pub unsafe fn ZSTD_DCtx_isOversizedTooLong(zds: *mut ZSTD_DStream) -> c_int {
    ((*zds).oversizedDuration >= ZSTD_WORKSPACETOOLARGE_MAXDURATION as size_t) as c_int
}

pub unsafe fn ZSTD_checkOutBuffer(
    zds: *const ZSTD_DStream,
    output: *const ZSTD_outBuffer,
) -> size_t {
    let expect: &ZSTD_outBuffer = &(*zds).expectedOutBuffer;
    /* No requirement when ZSTD_obm_stable is not enabled. */
    if (*zds).outBufferMode != ZSTD_bm_stable {
        return 0;
    }
    /* Any buffer is allowed in zdss_init */
    if (*zds).streamStage == zdss_init {
        return 0;
    }
    /* The buffer must match our expectation exactly. */
    if expect.dst == (*output).dst && expect.pos == (*output).pos && expect.size == (*output).size {
        return 0;
    }
    ERROR(ZSTD_error_dstBuffer_wrong)
}

pub unsafe fn ZSTD_decompressContinueStream(
    zds: *mut ZSTD_DStream,
    op: *mut *mut c_char,
    oend: *mut c_char,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let isSkipFrame: c_int = ZSTD_isSkipFrame(zds);
    if (*zds).outBufferMode == ZSTD_bm_buffered {
        let dstSize: size_t = if isSkipFrame != 0 {
            0
        } else {
            (*zds).outBuffSize - (*zds).outStart
        };
        let decodedSize: size_t = ZSTD_decompressContinue(
            zds,
            (*zds).outBuff.add((*zds).outStart) as *mut c_void,
            dstSize,
            src,
            srcSize,
        );
        {
            let err_code = decodedSize;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if decodedSize == 0 && isSkipFrame == 0 {
            (*zds).streamStage = zdss_read;
        } else {
            (*zds).outEnd = (*zds).outStart + decodedSize;
            (*zds).streamStage = zdss_flush;
        }
    } else {
        /* Write directly into the output buffer */
        let dstSize: size_t = if isSkipFrame != 0 {
            0
        } else {
            (oend as *const c_char).offset_from(*op) as size_t
        };
        let decodedSize: size_t =
            ZSTD_decompressContinue(zds, *op as *mut c_void, dstSize, src, srcSize);
        {
            let err_code = decodedSize;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        *op = (*op).add(decodedSize);
        /* Flushing is not needed. */
        (*zds).streamStage = zdss_read;
        /* asserts dropped */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream(
    zds: *mut ZSTD_DStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> size_t {
    let src: *const c_char = (*input).src as *const c_char;
    let istart: *const c_char = if (*input).pos != 0 {
        src.add((*input).pos)
    } else {
        src
    };
    let iend: *const c_char = if (*input).size != 0 {
        src.add((*input).size)
    } else {
        src
    };
    let mut ip: *const c_char = istart;
    let dst: *mut c_char = (*output).dst as *mut c_char;
    let ostart: *mut c_char = if (*output).pos != 0 {
        dst.add((*output).pos)
    } else {
        dst
    };
    let oend: *mut c_char = if (*output).size != 0 {
        dst.add((*output).size)
    } else {
        dst
    };
    let mut op: *mut c_char = ostart;
    let mut someMoreWork: U32 = 1;

    /* assert(zds != NULL); dropped */
    if (*input).pos > (*input).size {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if (*output).pos > (*output).size {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let err_code = ZSTD_checkOutBuffer(zds, output);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    while someMoreWork != 0 {
        match (*zds).streamStage {
            zdss_init => {
                (*zds).streamStage = zdss_loadHeader;
                (*zds).lhSize = 0;
                (*zds).inPos = 0;
                (*zds).outStart = 0;
                (*zds).outEnd = 0;
                /* ZSTD_LEGACY_SUPPORT >= 1 */
                (*zds).legacyVersion = 0;
                (*zds).hostageByte = 0;
                (*zds).expectedOutBuffer = core::ptr::read(output);
                /* fallthrough */

                /* zdss_loadHeader */
                let r = ZSTD_decompressStream_loadHeader(
                    zds,
                    &mut ip,
                    iend,
                    istart,
                    &mut op,
                    oend,
                    &mut someMoreWork,
                    output,
                    input,
                );
                if let Some(ret) = r {
                    return ret;
                }
            }

            zdss_loadHeader => {
                let r = ZSTD_decompressStream_loadHeader(
                    zds,
                    &mut ip,
                    iend,
                    istart,
                    &mut op,
                    oend,
                    &mut someMoreWork,
                    output,
                    input,
                );
                if let Some(ret) = r {
                    return ret;
                }
            }

            zdss_read => {
                {
                    let neededInSize: size_t = ZSTD_nextSrcSizeToDecompressWithInputSize(
                        zds,
                        iend.offset_from(ip) as size_t,
                    );
                    if neededInSize == 0 {
                        /* end of frame */
                        (*zds).streamStage = zdss_init;
                        someMoreWork = 0;
                        continue;
                    }
                    if (iend.offset_from(ip) as size_t) >= neededInSize {
                        /* decode directly from src */
                        {
                            let err_code = ZSTD_decompressContinueStream(
                                zds,
                                &mut op,
                                oend,
                                ip as *const c_void,
                                neededInSize,
                            );
                            if ERR_isError(err_code) != 0 {
                                return err_code;
                            }
                        }
                        /* assert(ip != NULL); dropped */
                        ip = ip.add(neededInSize);
                        /* Function modifies the stage so we must break */
                        continue;
                    }
                }
                if ip == iend {
                    someMoreWork = 0;
                    continue;
                } /* no more input */
                (*zds).streamStage = zdss_load;
                /* fallthrough */
                let r = ZSTD_decompressStream_load(zds, &mut ip, iend, &mut op, oend, &mut someMoreWork);
                if let Some(ret) = r {
                    return ret;
                }
            }

            zdss_load => {
                let r = ZSTD_decompressStream_load(zds, &mut ip, iend, &mut op, oend, &mut someMoreWork);
                if let Some(ret) = r {
                    return ret;
                }
            }

            zdss_flush => {
                {
                    let toFlushSize: size_t = (*zds).outEnd - (*zds).outStart;
                    let flushedSize: size_t = ZSTD_limitCopy(
                        op as *mut u8,
                        oend.offset_from(op) as size_t,
                        (*zds).outBuff.add((*zds).outStart) as *const u8,
                        toFlushSize,
                    );

                    op = op.add(flushedSize);

                    (*zds).outStart += flushedSize;
                    if flushedSize == toFlushSize {
                        /* flush completed */
                        (*zds).streamStage = zdss_read;
                        if ((*zds).outBuffSize < (*zds).fParams.frameContentSize as size_t)
                            && ((*zds).outStart + (*zds).fParams.blockSizeMax as size_t
                                > (*zds).outBuffSize)
                        {
                            (*zds).outStart = 0;
                            (*zds).outEnd = 0;
                        }
                        continue;
                    }
                }
                /* cannot complete flush */
                someMoreWork = 0;
                continue;
            }

            _ => {
                /* default : impossible */
                return ERROR(ZSTD_error_GENERIC);
            }
        }
    }

    /* result */
    (*input).pos = (ip as *const c_char).offset_from((*input).src as *const c_char) as size_t;
    (*output).pos = (op as *const c_char).offset_from((*output).dst as *const c_char) as size_t;

    /* Update the expected output buffer for ZSTD_obm_stable. */
    (*zds).expectedOutBuffer = core::ptr::read(output);

    if (ip == istart) && (op == ostart) {
        /* no forward progress */
        (*zds).noForwardProgress += 1;
        if (*zds).noForwardProgress >= ZSTD_NO_FORWARD_PROGRESS_MAX {
            if op == oend {
                return ERROR(ZSTD_error_noForwardProgress_destFull);
            }
            if ip == iend {
                return ERROR(ZSTD_error_noForwardProgress_inputEmpty);
            }
            /* assert(0); dropped */
        }
    } else {
        (*zds).noForwardProgress = 0;
    }
    {
        let mut nextSrcSizeHint: size_t = ZSTD_nextSrcSizeToDecompress(zds);
        if nextSrcSizeHint == 0 {
            /* frame fully decoded */
            if (*zds).outEnd == (*zds).outStart {
                /* output fully flushed */
                if (*zds).hostageByte != 0 {
                    if (*input).pos >= (*input).size {
                        /* can't release hostage (not present) */
                        (*zds).streamStage = zdss_read;
                        return 1;
                    }
                    (*input).pos += 1; /* release hostage */
                }
                return 0;
            }
            if (*zds).hostageByte == 0 {
                /* output not fully flushed; keep last byte as hostage */
                (*input).pos -= 1;
                (*zds).hostageByte = 1;
            }
            return 1;
        }
        nextSrcSizeHint += ZSTD_blockHeaderSize
            * (ZSTD_nextInputType(zds) == ZSTDnit_block) as size_t; /* preload header of next block */
        /* assert(zds->inPos <= nextSrcSizeHint); dropped */
        nextSrcSizeHint -= (*zds).inPos; /* part already loaded*/
        nextSrcSizeHint
    }
}

/* Helper for the zdss_loadHeader arm (with its zdss_init fallthrough).
 * Returns Some(ret) if the outer function should return `ret`,
 * or None to continue the outer while-loop (i.e., a `break` in C). */
unsafe fn ZSTD_decompressStream_loadHeader(
    zds: *mut ZSTD_DStream,
    ip_ref: &mut *const c_char,
    iend: *const c_char,
    istart: *const c_char,
    op_ref: &mut *mut c_char,
    oend: *mut c_char,
    someMoreWork: &mut U32,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> Option<size_t> {
    let mut ip = *ip_ref;
    let mut op = *op_ref;
    /* ZSTD_LEGACY_SUPPORT >= 1 */
    if (*zds).legacyVersion != 0 {
        if (*zds).staticSize != 0 {
            *ip_ref = ip;
            *op_ref = op;
            return Some(ERROR(ZSTD_error_memory_allocation));
        }
        {
            let hint: size_t = ZSTD_decompressLegacyStream(
                (*zds).legacyContext,
                (*zds).legacyVersion,
                output,
                input,
            );
            if hint == 0 {
                (*zds).streamStage = zdss_init;
            }
            *ip_ref = ip;
            *op_ref = op;
            return Some(hint);
        }
    }
    {
        let hSize: size_t = ZSTD_getFrameHeader_advanced(
            &mut (*zds).fParams,
            (*zds).headerBuffer.as_ptr() as *const c_void,
            (*zds).lhSize,
            (*zds).format,
        );
        if (*zds).refMultipleDDicts != 0 && !(*zds).ddictSet.is_null() {
            ZSTD_DCtx_selectFrameDDict(zds);
        }
        if ZSTD_isError(hSize) != 0 {
            /* ZSTD_LEGACY_SUPPORT >= 1 */
            let legacyVersion: U32 = ZSTD_isLegacy(istart as *const c_void, iend.offset_from(istart) as size_t);
            if legacyVersion != 0 {
                let ddict: *const ZSTD_DDict = ZSTD_getDDict(zds);
                let dict: *const c_void = if !ddict.is_null() {
                    ZSTD_DDict_dictContent(ddict)
                } else {
                    core::ptr::null()
                };
                let dictSize: size_t = if !ddict.is_null() {
                    ZSTD_DDict_dictSize(ddict)
                } else {
                    0
                };
                if (*zds).staticSize != 0 {
                    *ip_ref = ip;
                    *op_ref = op;
                    return Some(ERROR(ZSTD_error_memory_allocation));
                }
                {
                    let err_code = ZSTD_initLegacyStream(
                        &mut (*zds).legacyContext,
                        (*zds).previousLegacyVersion,
                        legacyVersion,
                        dict,
                        dictSize,
                    );
                    if ERR_isError(err_code) != 0 {
                        *ip_ref = ip;
                        *op_ref = op;
                        return Some(err_code);
                    }
                }
                (*zds).legacyVersion = legacyVersion;
                (*zds).previousLegacyVersion = legacyVersion;
                {
                    let hint: size_t = ZSTD_decompressLegacyStream(
                        (*zds).legacyContext,
                        legacyVersion,
                        output,
                        input,
                    );
                    if hint == 0 {
                        (*zds).streamStage = zdss_init;
                    }
                    *ip_ref = ip;
                    *op_ref = op;
                    return Some(hint);
                }
            }
            *ip_ref = ip;
            *op_ref = op;
            return Some(hSize); /* error */
        }
        if hSize != 0 {
            /* need more input */
            let toLoad: size_t = hSize - (*zds).lhSize;
            let remainingInput: size_t = iend.offset_from(ip) as size_t;
            /* assert(iend >= ip); dropped */
            if toLoad > remainingInput {
                /* not enough input to load full header */
                if remainingInput > 0 {
                    ZSTD_memcpy(
                        (*zds).headerBuffer.as_mut_ptr().add((*zds).lhSize),
                        ip as *const u8,
                        remainingInput,
                    );
                    (*zds).lhSize += remainingInput;
                }
                (*input).pos = (*input).size;
                /* check first few bytes */
                {
                    let err_code = ZSTD_getFrameHeader_advanced(
                        &mut (*zds).fParams,
                        (*zds).headerBuffer.as_ptr() as *const c_void,
                        (*zds).lhSize,
                        (*zds).format,
                    );
                    if ERR_isError(err_code) != 0 {
                        *ip_ref = ip;
                        *op_ref = op;
                        return Some(err_code);
                    }
                }
                /* return hint input size */
                *ip_ref = ip;
                *op_ref = op;
                return Some(
                    (MAX(ZSTD_FRAMEHEADERSIZE_MIN((*zds).format), hSize) - (*zds).lhSize)
                        + ZSTD_blockHeaderSize,
                );
            }
            /* assert(ip != NULL); dropped */
            ZSTD_memcpy(
                (*zds).headerBuffer.as_mut_ptr().add((*zds).lhSize),
                ip as *const u8,
                toLoad,
            );
            (*zds).lhSize = hSize;
            ip = ip.add(toLoad);
            *ip_ref = ip;
            *op_ref = op;
            return None; /* break */
        }
    }

    /* check for single-pass mode opportunity */
    if (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
        && (*zds).fParams.frameType != ZSTD_skippableFrame
        && (oend.offset_from(op) as size_t) as c_ulonglong >= (*zds).fParams.frameContentSize
    {
        let cSize: size_t = ZSTD_findFrameCompressedSize_advanced(
            istart as *const c_void,
            iend.offset_from(istart) as size_t,
            (*zds).format,
        );
        if cSize <= (iend.offset_from(istart) as size_t) {
            /* shortcut : using single-pass mode */
            let decompressedSize: size_t = ZSTD_decompress_usingDDict(
                zds,
                op as *mut c_void,
                oend.offset_from(op) as size_t,
                istart as *const c_void,
                cSize,
                ZSTD_getDDict(zds),
            );
            if ZSTD_isError(decompressedSize) != 0 {
                *ip_ref = ip;
                *op_ref = op;
                return Some(decompressedSize);
            }
            /* assert(istart != NULL); dropped */
            ip = istart.add(cSize);
            op = if !op.is_null() {
                op.add(decompressedSize)
            } else {
                op
            };
            (*zds).expected = 0;
            (*zds).streamStage = zdss_init;
            *someMoreWork = 0;
            *ip_ref = ip;
            *op_ref = op;
            return None; /* break */
        }
    }

    /* Check output buffer is large enough for ZSTD_odm_stable. */
    if (*zds).outBufferMode == ZSTD_bm_stable
        && (*zds).fParams.frameType != ZSTD_skippableFrame
        && (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
        && ((oend.offset_from(op) as size_t) as c_ulonglong) < (*zds).fParams.frameContentSize
    {
        *ip_ref = ip;
        *op_ref = op;
        return Some(ERROR(ZSTD_error_dstSize_tooSmall));
    }

    /* Consume header (see ZSTDds_decodeFrameHeader) */
    {
        let err_code = ZSTD_decompressBegin_usingDDict(zds, ZSTD_getDDict(zds));
        if ERR_isError(err_code) != 0 {
            *ip_ref = ip;
            *op_ref = op;
            return Some(err_code);
        }
    }

    if (*zds).format == ZSTD_f_zstd1
        && (MEM_readLE32((*zds).headerBuffer.as_ptr()) & ZSTD_MAGIC_SKIPPABLE_MASK)
            == ZSTD_MAGIC_SKIPPABLE_START
    {
        /* skippable frame */
        (*zds).expected =
            MEM_readLE32((*zds).headerBuffer.as_ptr().add(ZSTD_FRAMEIDSIZE)) as size_t;
        (*zds).stage = ZSTDds_skipFrame;
    } else {
        {
            let err_code = ZSTD_decodeFrameHeader(
                zds,
                (*zds).headerBuffer.as_ptr() as *const c_void,
                (*zds).lhSize,
            );
            if ERR_isError(err_code) != 0 {
                *ip_ref = ip;
                *op_ref = op;
                return Some(err_code);
            }
        }
        (*zds).expected = ZSTD_blockHeaderSize;
        (*zds).stage = ZSTDds_decodeBlockHeader;
    }

    /* control buffer memory usage */
    (*zds).fParams.windowSize = MAX((*zds).fParams.windowSize, 1u64 << ZSTD_WINDOWLOG_ABSOLUTEMIN);
    if (*zds).fParams.windowSize > (*zds).maxWindowSize as c_ulonglong {
        *ip_ref = ip;
        *op_ref = op;
        return Some(ERROR(ZSTD_error_frameParameter_windowTooLarge));
    }
    if (*zds).maxBlockSizeParam != 0 {
        (*zds).fParams.blockSizeMax =
            MIN((*zds).fParams.blockSizeMax, (*zds).maxBlockSizeParam as c_uint);
    }

    /* Adapt buffer sizes to frame header instructions */
    {
        let neededInBuffSize: size_t = MAX((*zds).fParams.blockSizeMax as size_t, 4 /* frame checksum */);
        let neededOutBuffSize: size_t = if (*zds).outBufferMode == ZSTD_bm_buffered {
            ZSTD_decodingBufferSize_internal(
                (*zds).fParams.windowSize,
                (*zds).fParams.frameContentSize,
                (*zds).fParams.blockSizeMax as size_t,
            )
        } else {
            0
        };

        ZSTD_DCtx_updateOversizedDuration(zds, neededInBuffSize, neededOutBuffSize);

        {
            let tooSmall: c_int = (((*zds).inBuffSize < neededInBuffSize)
                || ((*zds).outBuffSize < neededOutBuffSize))
                as c_int;
            let tooLarge: c_int = ZSTD_DCtx_isOversizedTooLong(zds);

            if tooSmall != 0 || tooLarge != 0 {
                let bufferSize: size_t = neededInBuffSize + neededOutBuffSize;
                if (*zds).staticSize != 0 {
                    /* static DCtx */
                    /* assert(zds->staticSize >= sizeof(ZSTD_DCtx)); dropped */
                    if bufferSize > (*zds).staticSize - core::mem::size_of::<ZSTD_DCtx>() {
                        *ip_ref = ip;
                        *op_ref = op;
                        return Some(ERROR(ZSTD_error_memory_allocation));
                    }
                } else {
                    ZSTD_customFree((*zds).inBuff as *mut c_void, (*zds).customMem);
                    (*zds).inBuffSize = 0;
                    (*zds).outBuffSize = 0;
                    (*zds).inBuff = ZSTD_customMalloc(bufferSize, (*zds).customMem) as *mut c_char;
                    if (*zds).inBuff.is_null() {
                        *ip_ref = ip;
                        *op_ref = op;
                        return Some(ERROR(ZSTD_error_memory_allocation));
                    }
                }
                (*zds).inBuffSize = neededInBuffSize;
                (*zds).outBuff = (*zds).inBuff.add((*zds).inBuffSize);
                (*zds).outBuffSize = neededOutBuffSize;
            }
        }
    }
    (*zds).streamStage = zdss_read;
    /* fallthrough to zdss_read */
    *ip_ref = ip;
    *op_ref = op;
    let r = ZSTD_decompressStream_read(zds, ip_ref, iend, op_ref, oend, someMoreWork);
    r
}

/* Helper for the zdss_read arm (with fallthrough to zdss_load). */
unsafe fn ZSTD_decompressStream_read(
    zds: *mut ZSTD_DStream,
    ip_ref: &mut *const c_char,
    iend: *const c_char,
    op_ref: &mut *mut c_char,
    oend: *mut c_char,
    someMoreWork: &mut U32,
) -> Option<size_t> {
    let mut ip = *ip_ref;
    {
        let neededInSize: size_t =
            ZSTD_nextSrcSizeToDecompressWithInputSize(zds, iend.offset_from(ip) as size_t);
        if neededInSize == 0 {
            /* end of frame */
            (*zds).streamStage = zdss_init;
            *someMoreWork = 0;
            *ip_ref = ip;
            return None;
        }
        if (iend.offset_from(ip) as size_t) >= neededInSize {
            /* decode directly from src */
            {
                let err_code = ZSTD_decompressContinueStream(
                    zds,
                    op_ref,
                    oend,
                    ip as *const c_void,
                    neededInSize,
                );
                if ERR_isError(err_code) != 0 {
                    *ip_ref = ip;
                    return Some(err_code);
                }
            }
            ip = ip.add(neededInSize);
            *ip_ref = ip;
            return None; /* break */
        }
    }
    if ip == iend {
        *someMoreWork = 0;
        *ip_ref = ip;
        return None;
    }
    (*zds).streamStage = zdss_load;
    *ip_ref = ip;
    /* fallthrough to zdss_load */
    ZSTD_decompressStream_load(zds, ip_ref, iend, op_ref, oend, someMoreWork)
}

/* Helper for the zdss_load arm. */
unsafe fn ZSTD_decompressStream_load(
    zds: *mut ZSTD_DStream,
    ip_ref: &mut *const c_char,
    iend: *const c_char,
    op_ref: &mut *mut c_char,
    oend: *mut c_char,
    someMoreWork: &mut U32,
) -> Option<size_t> {
    let mut ip = *ip_ref;
    {
        let neededInSize: size_t = ZSTD_nextSrcSizeToDecompress(zds);
        let toLoad: size_t = neededInSize - (*zds).inPos;
        let isSkipFrame: c_int = ZSTD_isSkipFrame(zds);
        let loadedSize: size_t;
        /* assert dropped */
        if isSkipFrame != 0 {
            loadedSize = MIN(toLoad, iend.offset_from(ip) as size_t);
        } else {
            if toLoad > (*zds).inBuffSize - (*zds).inPos {
                *ip_ref = ip;
                return Some(ERROR(ZSTD_error_corruption_detected)); /* should never happen */
            }
            loadedSize = ZSTD_limitCopy(
                (*zds).inBuff.add((*zds).inPos) as *mut u8,
                toLoad,
                ip as *const u8,
                iend.offset_from(ip) as size_t,
            );
        }
        if loadedSize != 0 {
            /* ip may be NULL */
            ip = ip.add(loadedSize);
            (*zds).inPos += loadedSize;
        }
        if loadedSize < toLoad {
            *someMoreWork = 0;
            *ip_ref = ip;
            return None;
        } /* not enough input, wait for more */

        /* decode loaded input */
        (*zds).inPos = 0; /* input is consumed */
        {
            let err_code = ZSTD_decompressContinueStream(
                zds,
                op_ref,
                oend,
                (*zds).inBuff as *const c_void,
                neededInSize,
            );
            if ERR_isError(err_code) != 0 {
                *ip_ref = ip;
                return Some(err_code);
            }
        }
        *ip_ref = ip;
        None /* break */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream_simpleArgs(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    dstPos: *mut size_t,
    src: *const c_void,
    srcSize: size_t,
    srcPos: *mut size_t,
) -> size_t {
    let mut output: ZSTD_outBuffer = ZSTD_outBuffer {
        dst,
        size: dstCapacity,
        pos: *dstPos,
    };
    let mut input: ZSTD_inBuffer = ZSTD_inBuffer {
        src,
        size: srcSize,
        pos: *srcPos,
    };
    {
        let cErr: size_t = ZSTD_decompressStream(dctx, &mut output, &mut input);
        *dstPos = output.pos;
        *srcPos = input.pos;
        cErr
    }
}
