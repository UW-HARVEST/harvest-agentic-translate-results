//! Translation of decompress/zstd_decompress.c
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens
)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::bits::ZSTD_highbit32;
use crate::entropy_common::{FSE_isError, FSE_readNCount, HUF_isError};
use crate::error_private::*;
use crate::huf::*;
use crate::huf_decompress::HUF_readDTableX2_wksp;
use crate::legacy::zstd_legacy::{
    ZSTD_decompressLegacy, ZSTD_decompressLegacyStream, ZSTD_findFrameCompressedSizeLegacy,
    ZSTD_findFrameSizeInfoLegacy, ZSTD_freeLegacyStreamContext, ZSTD_getDecompressedSize_legacy,
    ZSTD_initLegacyStream, ZSTD_isLegacy,
};
use crate::mem::*;
use crate::xxhash::{ZSTD_XXH64, ZSTD_XXH64_digest, ZSTD_XXH64_reset, ZSTD_XXH64_update};
use crate::zstd_common::{ZSTD_getErrorCode, ZSTD_isError};
use crate::zstd_ddict::{
    ZSTD_DDict_dictContent, ZSTD_DDict_dictSize, ZSTD_copyDDictParameters,
    ZSTD_createDDict_advanced, ZSTD_freeDDict, ZSTD_getDictID_fromDDict, ZSTD_sizeof_DDict,
};
use crate::zstd_decompress_block::{
    ZSTD_buildFSETable, ZSTD_checkContinuity, ZSTD_decompressBlock_internal, ZSTD_getcBlockSize,
    is_streaming, not_streaming,
};
use crate::zstd_decompress_internal::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* ***************************************************************
*  Tuning parameters
*****************************************************************/
pub const ZSTD_HEAPMODE: u32 = 1;

/*
 *  MAXWINDOWSIZE_DEFAULT :
 *  maximum window size accepted by DStream __by default__.
 */
pub const ZSTD_MAXWINDOWSIZE_DEFAULT: usize =
    (((1u32) << ZSTD_WINDOWLOG_LIMIT_DEFAULT) + 1) as usize;

/*
 *  NO_FORWARD_PROGRESS_MAX :
 *  maximum allowed nb of calls to ZSTD_decompressStream()
 *  without any forward progress
 */
pub const ZSTD_NO_FORWARD_PROGRESS_MAX: c_int = 16;

/************************************
 * Multiple DDicts Hashset internals *
 *************************************/

pub const DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT: usize = 4;
pub const DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT: usize = 3;

pub const DDICT_HASHSET_TABLE_BASE_SIZE: usize = 64;
pub const DDICT_HASHSET_RESIZE_FACTOR: usize = 2;

/* Hash function to determine starting position of dict insertion within the table
 * Returns an index between [0, hashSet->ddictPtrTableSize]
 */
pub unsafe fn ZSTD_DDictHashSet_getIndex(
    hashSet: *const ZSTD_DDictHashSet,
    dictID: U32,
) -> usize {
    let dictID = dictID;
    let hash: U64 = ZSTD_XXH64(
        core::ptr::addr_of!(dictID) as *const c_void,
        core::mem::size_of::<U32>(),
        0,
    );
    /* DDict ptr table size is a multiple of 2, use size - 1 as mask to get index within [0, hashSet->ddictPtrTableSize) */
    (hash & ((*hashSet).ddictPtrTableSize as U64).wrapping_sub(1)) as usize
}

/* Adds DDict to a hashset without resizing it.
 * If inserting a DDict with a dictID that already exists in the set, replaces the one in the set.
 * Returns 0 if successful, or a zstd error code if something went wrong.
 */
pub unsafe fn ZSTD_DDictHashSet_emplaceDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    ddict: *const ZSTD_DDict,
) -> usize {
    let dictID: U32 = ZSTD_getDictID_fromDDict(ddict);
    let mut idx: usize = ZSTD_DDictHashSet_getIndex(hashSet, dictID);
    let idxRangeMask: usize = (*hashSet).ddictPtrTableSize.wrapping_sub(1);
    if (*hashSet).ddictPtrCount == (*hashSet).ddictPtrTableSize {
        return ERROR(ZSTD_error_GENERIC);
    }
    while !(*(*hashSet).ddictPtrTable.add(idx)).is_null() {
        /* Replace existing ddict if inserting ddict with same dictID */
        if ZSTD_getDictID_fromDDict(*(*hashSet).ddictPtrTable.add(idx)) == dictID {
            *(*hashSet).ddictPtrTable.add(idx) = ddict;
            return 0;
        }
        idx &= idxRangeMask;
        idx = idx.wrapping_add(1);
    }
    *(*hashSet).ddictPtrTable.add(idx) = ddict;
    (*hashSet).ddictPtrCount = (*hashSet).ddictPtrCount.wrapping_add(1);
    0
}

/* Expands hash table by factor of DDICT_HASHSET_RESIZE_FACTOR and
 * rehashes all values, allocates new table, frees old table.
 * Returns 0 on success, otherwise a zstd error code.
 */
pub unsafe fn ZSTD_DDictHashSet_expand(
    hashSet: *mut ZSTD_DDictHashSet,
    customMem: ZSTD_customMem,
) -> usize {
    let newTableSize: usize = (*hashSet)
        .ddictPtrTableSize
        .wrapping_mul(DDICT_HASHSET_RESIZE_FACTOR);
    let newTable: *mut *const ZSTD_DDict = ZSTD_customCalloc(
        core::mem::size_of::<*const ZSTD_DDict>().wrapping_mul(newTableSize),
        customMem,
    ) as *mut *const ZSTD_DDict;
    let oldTable: *mut *const ZSTD_DDict = (*hashSet).ddictPtrTable;
    let oldTableSize: usize = (*hashSet).ddictPtrTableSize;
    let mut i: usize;

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
        i = i.wrapping_add(1);
    }
    ZSTD_customFree(oldTable as *mut u8, customMem);
    0
}

/* Fetches a DDict with the given dictID
 * Returns the ZSTD_DDict* with the requested dictID. If it doesn't exist, then returns NULL.
 */
pub unsafe fn ZSTD_DDictHashSet_getDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    dictID: U32,
) -> *const ZSTD_DDict {
    let mut idx: usize = ZSTD_DDictHashSet_getIndex(hashSet, dictID);
    let idxRangeMask: usize = (*hashSet).ddictPtrTableSize.wrapping_sub(1);
    loop {
        let currDictID: usize =
            ZSTD_getDictID_fromDDict(*(*hashSet).ddictPtrTable.add(idx)) as usize;
        if currDictID == dictID as usize || currDictID == 0 {
            /* currDictID == 0 implies a NULL ddict entry */
            break;
        } else {
            idx &= idxRangeMask; /* Goes to start of table when we reach the end */
            idx = idx.wrapping_add(1);
        }
    }
    *(*hashSet).ddictPtrTable.add(idx)
}

/* Allocates space for and returns a ddict hash set
 * The hash set's ZSTD_DDict* table has all values automatically set to NULL to begin with.
 * Returns NULL if allocation failed.
 */
pub unsafe fn ZSTD_createDDictHashSet(customMem: ZSTD_customMem) -> *mut ZSTD_DDictHashSet {
    let ret: *mut ZSTD_DDictHashSet = ZSTD_customMalloc(
        core::mem::size_of::<ZSTD_DDictHashSet>(),
        customMem,
    ) as *mut ZSTD_DDictHashSet;
    if ret.is_null() {
        return core::ptr::null_mut();
    }
    (*ret).ddictPtrTable = ZSTD_customCalloc(
        DDICT_HASHSET_TABLE_BASE_SIZE.wrapping_mul(core::mem::size_of::<*const ZSTD_DDict>()),
        customMem,
    ) as *mut *const ZSTD_DDict;
    if (*ret).ddictPtrTable.is_null() {
        ZSTD_customFree(ret as *mut u8, customMem);
        return core::ptr::null_mut();
    }
    (*ret).ddictPtrTableSize = DDICT_HASHSET_TABLE_BASE_SIZE;
    (*ret).ddictPtrCount = 0;
    ret
}

/* Frees the table of ZSTD_DDict* within a hashset, then frees the hashset itself.
 * Note: The ZSTD_DDict* within the table are NOT freed.
 */
pub unsafe fn ZSTD_freeDDictHashSet(
    hashSet: *mut ZSTD_DDictHashSet,
    customMem: ZSTD_customMem,
) {
    if !hashSet.is_null() && !(*hashSet).ddictPtrTable.is_null() {
        ZSTD_customFree((*hashSet).ddictPtrTable as *mut u8, customMem);
    }
    if !hashSet.is_null() {
        ZSTD_customFree(hashSet as *mut u8, customMem);
    }
}

/* Public function: Adds a DDict into the ZSTD_DDictHashSet, possibly triggering a resize of the hash set.
 * Returns 0 on success, or a ZSTD error.
 */
pub unsafe fn ZSTD_DDictHashSet_addDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    ddict: *const ZSTD_DDict,
    customMem: ZSTD_customMem,
) -> usize {
    if (*hashSet)
        .ddictPtrCount
        .wrapping_mul(DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT)
        / (*hashSet).ddictPtrTableSize
        * DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT
        != 0
    {
        let err_code = ZSTD_DDictHashSet_expand(hashSet, customMem);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_DDictHashSet_emplaceDDict(hashSet, ddict);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    0
}

/*-*************************************************************
*   Context management
***************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DCtx(dctx: *const ZSTD_DCtx) -> usize {
    if dctx.is_null() {
        return 0; /* support sizeof NULL */
    }
    core::mem::size_of::<ZSTD_DCtx>()
        + ZSTD_sizeof_DDict((*dctx).ddictLocal)
        + (*dctx).inBuffSize
        + (*dctx).outBuffSize
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_estimateDCtxSize() -> usize {
    core::mem::size_of::<ZSTD_DCtx>()
}

pub unsafe fn ZSTD_startingInputLength(format: ZSTD_format_e) -> usize {
    let startingInputLength: usize = ZSTD_FRAMEHEADERSIZE_PREFIX(format);
    /* only supports formats ZSTD_f_zstd1 and ZSTD_f_zstd1_magicless */
    startingInputLength
}

pub unsafe fn ZSTD_DCtx_resetParameters(dctx: *mut ZSTD_DCtx) {
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
    (*dctx).legacyContext = core::ptr::null_mut();
    (*dctx).previousLegacyVersion = 0;
    (*dctx).noForwardProgress = 0;
    (*dctx).oversizedDuration = 0;
    (*dctx).isFrameDecompression = 1;
    (*dctx).ddictSet = core::ptr::null_mut();
    ZSTD_DCtx_resetParameters(dctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDCtx(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DCtx {
    let dctx: *mut ZSTD_DCtx = workspace as *mut ZSTD_DCtx;

    if ((workspace as usize) & 7) != 0 {
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
    if ((customMem.customAlloc.is_none() as c_int) ^ (customMem.customFree.is_none() as c_int)) != 0
    {
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
        return dctx;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_createDCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    unsafe { ZSTD_createDCtx_internal(customMem) }
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    unsafe { ZSTD_createDCtx_internal(ZSTD_defaultCMem) }
}

pub unsafe fn ZSTD_clearDict(dctx: *mut ZSTD_DCtx) {
    ZSTD_freeDDict((*dctx).ddictLocal);
    (*dctx).ddictLocal = core::ptr::null_mut();
    (*dctx).ddict = core::ptr::null();
    (*dctx).dictUses = ZSTD_dont_use;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    if dctx.is_null() {
        return 0; /* support free on NULL */
    }
    if (*dctx).staticSize != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }
    {
        let cMem: ZSTD_customMem = (*dctx).customMem;
        ZSTD_clearDict(dctx);
        ZSTD_customFree((*dctx).inBuff as *mut u8, cMem);
        (*dctx).inBuff = core::ptr::null_mut();
        if !(*dctx).legacyContext.is_null() {
            ZSTD_freeLegacyStreamContext((*dctx).legacyContext, (*dctx).previousLegacyVersion);
        }
        if !(*dctx).ddictSet.is_null() {
            ZSTD_freeDDictHashSet((*dctx).ddictSet, cMem);
            (*dctx).ddictSet = core::ptr::null_mut();
        }
        ZSTD_customFree(dctx as *mut u8, cMem);
        return 0;
    }
}

/* no longer useful */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyDCtx(dstDCtx: *mut ZSTD_DCtx, srcDCtx: *const ZSTD_DCtx) {
    let toCopy: usize = (core::ptr::addr_of!((*dstDCtx).inBuff) as *const c_char as usize)
        .wrapping_sub(dstDCtx as *const c_char as usize);
    ZSTD_memcpy(dstDCtx as *mut u8, srcDCtx as *const u8, toCopy); /* no need to copy workspace */
}

/* Given a dctx with a digested frame params, re-selects the correct ZSTD_DDict based on
 * the requested dict ID from the frame. If there exists a reference to the correct ZSTD_DDict, then
 * accordingly sets the ddict to be used to decompress the frame.
 *
 * If no DDict is found, then no action is taken, and the ZSTD_DCtx::ddict remains as-is.
 *
 * ZSTD_d_refMultipleDDicts must be enabled for this function to be called.
 */
pub unsafe fn ZSTD_DCtx_selectFrameDDict(dctx: *mut ZSTD_DCtx) {
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

/* ZSTD_isFrame() :
 *  Tells if the content of `buffer` starts with a valid Frame Identifier. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isFrame(buffer: *const c_void, size: usize) -> c_uint {
    if size < ZSTD_FRAMEIDSIZE {
        return 0;
    }
    {
        let magic: U32 = MEM_readLE32(buffer as *const BYTE);
        if magic == ZSTD_MAGICNUMBER {
            return 1;
        }
        if (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            return 1;
        }
    }
    if ZSTD_isLegacy(buffer, size) != 0 {
        return 1;
    }
    0
}

/* ZSTD_isSkippableFrame() :
 *  Tells if the content of `buffer` starts with a valid Frame Identifier for a skippable frame.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isSkippableFrame(buffer: *const c_void, size: usize) -> c_uint {
    if size < ZSTD_FRAMEIDSIZE {
        return 0;
    }
    {
        let magic: U32 = MEM_readLE32(buffer as *const BYTE);
        if (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            return 1;
        }
    }
    0
}

/* ZSTD_frameHeaderSize_internal() :
 *  srcSize must be large enough to reach header size fields.
 *  note : only works for formats ZSTD_f_zstd1 and ZSTD_f_zstd1_magicless.
 * @return : size of the Frame Header
 *           or an error code, which can be tested with ZSTD_isError() */
pub unsafe fn ZSTD_frameHeaderSize_internal(
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> usize {
    let minInputSize: usize = ZSTD_startingInputLength(format);
    if srcSize < minInputSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    {
        let fhd: BYTE = *(src as *const BYTE).add(minInputSize - 1);
        let dictID: U32 = (fhd & 3) as U32;
        let singleSegment: U32 = ((fhd >> 5) & 1) as U32;
        let fcsId: U32 = (fhd >> 6) as U32;
        return minInputSize
            + ((singleSegment == 0) as usize)
            + ZSTD_did_fieldSize[dictID as usize]
            + ZSTD_fcs_fieldSize[fcsId as usize]
            + ((singleSegment != 0 && fcsId == 0) as usize);
    }
}

/* ZSTD_frameHeaderSize() :
 *  srcSize must be >= ZSTD_frameHeaderSize_prefix.
 * @return : size of the Frame Header,
 *           or an error code (if srcSize is too small) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    ZSTD_frameHeaderSize_internal(src, srcSize, ZSTD_f_zstd1)
}

/* ZSTD_getFrameHeader_advanced() :
 *  decode Frame Header, or require larger `srcSize`.
 *  note : only works for formats ZSTD_f_zstd1 and ZSTD_f_zstd1_magicless
 * @return : 0, `zfhPtr` is correctly filled,
 *          >0, `srcSize` is too small, value is wanted `srcSize` amount,
 *           or an error code, which can be tested using ZSTD_isError() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader_advanced(
    zfhPtr: *mut ZSTD_FrameHeader,
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> usize {
    let ip: *const BYTE = src as *const BYTE;
    let minInputSize: usize = ZSTD_startingInputLength(format);

    if srcSize > 0 {
        /* note : technically could be considered an assert(), since it's an invalid entry */
        if src.is_null() {
            return ERROR(ZSTD_error_GENERIC);
        }
    }
    if srcSize < minInputSize {
        if srcSize > 0 && format != ZSTD_f_zstd1_magicless {
            /* when receiving less than @minInputSize bytes,
             * control these bytes at least correspond to a supported magic number
             * in order to error out early if they don't.
             */
            let toCopy: usize = MIN(4, srcSize);
            let mut hbuf: [u8; 4] = [0; 4];
            MEM_writeLE32(hbuf.as_mut_ptr(), ZSTD_MAGICNUMBER);
            ZSTD_memcpy(hbuf.as_mut_ptr(), src as *const u8, toCopy);
            if MEM_readLE32(hbuf.as_ptr()) != ZSTD_MAGICNUMBER {
                /* not a zstd frame : let's check if it's a skippable frame */
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

    /* not strictly necessary, but static analyzers may not understand that zfhPtr
     * will be read only if return value is zero, since they are 2 different signals */
    ZSTD_memset(
        zfhPtr as *mut u8,
        0,
        core::mem::size_of::<ZSTD_FrameHeader>(),
    );
    if (format != ZSTD_f_zstd1_magicless) && (MEM_readLE32(src as *const BYTE) != ZSTD_MAGICNUMBER)
    {
        if (MEM_readLE32(src as *const BYTE) & ZSTD_MAGIC_SKIPPABLE_MASK)
            == ZSTD_MAGIC_SKIPPABLE_START
        {
            /* skippable frame */
            if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
                return ZSTD_SKIPPABLEHEADERSIZE; /* magic number + frame length */
            }
            ZSTD_memset(
                zfhPtr as *mut u8,
                0,
                core::mem::size_of::<ZSTD_FrameHeader>(),
            );
            (*zfhPtr).frameType = ZSTD_skippableFrame;
            (*zfhPtr).dictID =
                MEM_readLE32(src as *const BYTE).wrapping_sub(ZSTD_MAGIC_SKIPPABLE_START);
            (*zfhPtr).headerSize = ZSTD_SKIPPABLEHEADERSIZE as c_uint;
            (*zfhPtr).frameContentSize = MEM_readLE32(
                (src as *const c_char).wrapping_add(ZSTD_FRAMEIDSIZE) as *const BYTE,
            ) as c_ulonglong;
            return 0;
        }
        return ERROR(ZSTD_error_prefix_unknown);
    }

    /* ensure there is enough `srcSize` to fully read/decode frame header */
    {
        let fhsize: usize = ZSTD_frameHeaderSize_internal(src, srcSize, format);
        if srcSize < fhsize {
            return fhsize;
        }
        (*zfhPtr).headerSize = fhsize as U32;
    }

    {
        let fhdByte: BYTE = *ip.add(minInputSize - 1);
        let mut pos: usize = minInputSize;
        let dictIDSizeCode: U32 = (fhdByte & 3) as U32;
        let checksumFlag: U32 = ((fhdByte >> 2) & 1) as U32;
        let singleSegment: U32 = ((fhdByte >> 5) & 1) as U32;
        let fcsID: U32 = (fhdByte >> 6) as U32;
        let mut windowSize: U64 = 0;
        let mut dictID: U32 = 0;
        let mut frameContentSize: U64 = ZSTD_CONTENTSIZE_UNKNOWN;
        if (fhdByte & 0x08) != 0 {
            return ERROR(ZSTD_error_frameParameter_unsupported);
        }

        if singleSegment == 0 {
            let wlByte: BYTE = *ip.add(pos);
            pos += 1;
            let windowLog: U32 = ((wlByte >> 3) as U32) + ZSTD_WINDOWLOG_ABSOLUTEMIN;
            if windowLog > ZSTD_WINDOWLOG_MAX as U32 {
                return ERROR(ZSTD_error_frameParameter_windowTooLarge);
            }
            windowSize = 1u64 << windowLog;
            windowSize = windowSize.wrapping_add((windowSize >> 3) * ((wlByte & 7) as U64));
        }
        match dictIDSizeCode {
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
            1 => {
                frameContentSize = (MEM_readLE16(ip.add(pos)) as U64).wrapping_add(256);
            }
            2 => {
                frameContentSize = MEM_readLE32(ip.add(pos)) as U64;
            }
            3 => {
                frameContentSize = MEM_readLE64(ip.add(pos));
            }
            _ => {
                if singleSegment != 0 {
                    frameContentSize = *ip.add(pos) as U64;
                }
            }
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

/* ZSTD_getFrameHeader() :
 *  decode Frame Header, or require larger `srcSize`.
 *  note : this function does not consume input, it only reads it. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader(
    zfhPtr: *mut ZSTD_FrameHeader,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_getFrameHeader_advanced(zfhPtr, src, srcSize, ZSTD_f_zstd1)
}

/* ZSTD_getFrameContentSize() :
 *  compatible with legacy mode */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameContentSize(
    src: *const c_void,
    srcSize: usize,
) -> c_ulonglong {
    if ZSTD_isLegacy(src, srcSize) != 0 {
        let ret: c_ulonglong = ZSTD_getDecompressedSize_legacy(src, srcSize);
        return if ret == 0 {
            ZSTD_CONTENTSIZE_UNKNOWN
        } else {
            ret
        };
    }
    {
        let mut zfh = ZSTD_FrameHeader::default();
        if ZSTD_getFrameHeader(&mut zfh, src, srcSize) != 0 {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        if zfh.frameType == ZSTD_skippableFrame {
            return 0;
        } else {
            return zfh.frameContentSize;
        }
    }
}

pub unsafe fn readSkippableFrameSize(src: *const c_void, srcSize: usize) -> usize {
    let skippableHeaderSize: usize = ZSTD_SKIPPABLEHEADERSIZE;
    let sizeU32: U32;

    if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    sizeU32 = MEM_readLE32((src as *const BYTE).add(ZSTD_FRAMEIDSIZE));
    if sizeU32.wrapping_add(ZSTD_SKIPPABLEHEADERSIZE as U32) < sizeU32 {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    {
        let skippableSize: usize = skippableHeaderSize.wrapping_add(sizeU32 as usize);
        if skippableSize > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        return skippableSize;
    }
}

/* ZSTD_readSkippableFrame() :
 * Retrieves content of a skippable frame, and writes it to dst buffer.
 *
 * @return : number of bytes written or a ZSTD error.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_readSkippableFrame(
    dst: *mut c_void,
    dstCapacity: usize,
    magicVariant: *mut c_uint,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    {
        let magicNumber: U32 = MEM_readLE32(src as *const BYTE);
        let skippableFrameSize: usize = readSkippableFrameSize(src, srcSize);
        let skippableContentSize: usize = skippableFrameSize.wrapping_sub(ZSTD_SKIPPABLEHEADERSIZE);

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
            *magicVariant = magicNumber.wrapping_sub(ZSTD_MAGIC_SKIPPABLE_START);
        }
        return skippableContentSize;
    }
}

/* ZSTD_findDecompressedSize() :
 *  `srcSize` must be the exact length of some number of ZSTD compressed and/or
 *      skippable frames
 *  note: compatible with legacy mode
 * @return : decompressed size of the frames contained */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findDecompressedSize(
    mut src: *const c_void,
    mut srcSize: usize,
) -> c_ulonglong {
    let mut totalDstSize: c_ulonglong = 0;

    while srcSize >= ZSTD_startingInputLength(ZSTD_f_zstd1) {
        let magicNumber: U32 = MEM_readLE32(src as *const BYTE);

        if (magicNumber & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            let skippableSize: usize = readSkippableFrameSize(src, srcSize);
            if ZSTD_isError(skippableSize) != 0 {
                return ZSTD_CONTENTSIZE_ERROR;
            }

            src = (src as *const BYTE).wrapping_add(skippableSize) as *const c_void;
            srcSize -= skippableSize;
            continue;
        }

        {
            let fcs: c_ulonglong = ZSTD_getFrameContentSize(src, srcSize);
            if fcs >= ZSTD_CONTENTSIZE_ERROR {
                return fcs;
            }

            if totalDstSize.wrapping_add(fcs) < totalDstSize {
                return ZSTD_CONTENTSIZE_ERROR; /* check for overflow */
            }
            totalDstSize = totalDstSize.wrapping_add(fcs);
        }
        /* skip to next frame */
        {
            let frameSrcSize: usize = ZSTD_findFrameCompressedSize(src, srcSize);
            if ZSTD_isError(frameSrcSize) != 0 {
                return ZSTD_CONTENTSIZE_ERROR;
            }

            src = (src as *const BYTE).wrapping_add(frameSrcSize) as *const c_void;
            srcSize -= frameSrcSize;
        }
    } /* while (srcSize >= ZSTD_frameHeaderSize_prefix) */

    if srcSize != 0 {
        return ZSTD_CONTENTSIZE_ERROR;
    }

    totalDstSize
}

/* ZSTD_getDecompressedSize() :
 *  compatible with legacy mode
 * @return : decompressed size if known, 0 otherwise */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDecompressedSize(
    src: *const c_void,
    srcSize: usize,
) -> c_ulonglong {
    let ret: c_ulonglong = ZSTD_getFrameContentSize(src, srcSize);
    if ret >= ZSTD_CONTENTSIZE_ERROR {
        0
    } else {
        ret
    }
}

/* ZSTD_decodeFrameHeader() :
 * `headerSize` must be the size provided by ZSTD_frameHeaderSize().
 * If multiple DDict references are enabled, also will choose the correct DDict to use.
 * @return : 0 if success, or an error code, which can be tested using ZSTD_isError() */
pub unsafe fn ZSTD_decodeFrameHeader(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    headerSize: usize,
) -> usize {
    let result: usize = ZSTD_getFrameHeader_advanced(
        core::ptr::addr_of_mut!((*dctx).fParams),
        src,
        headerSize,
        (*dctx).format,
    );
    if ZSTD_isError(result) != 0 {
        return result; /* invalid header */
    }
    if result > 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Reference DDict requested by frame if dctx references multiple ddicts */
    if (*dctx).refMultipleDDicts == ZSTD_rmd_refMultipleDDicts && !(*dctx).ddictSet.is_null() {
        ZSTD_DCtx_selectFrameDDict(dctx);
    }

    /* Skip the dictID check in fuzzing mode, because it makes the search harder. */
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
        ZSTD_XXH64_reset(core::ptr::addr_of_mut!((*dctx).xxhState), 0);
    }
    (*dctx).processedCSize = (*dctx).processedCSize.wrapping_add(headerSize as U64);
    0
}

pub unsafe fn ZSTD_errorFrameSizeInfo(ret: usize) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo = ZSTD_frameSizeInfo::default();
    frameSizeInfo.compressedSize = ret;
    frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
    frameSizeInfo
}

pub unsafe fn ZSTD_findFrameSizeInfo(
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo = ZSTD_frameSizeInfo::default();
    ZSTD_memset(
        core::ptr::addr_of_mut!(frameSizeInfo) as *mut u8,
        0,
        core::mem::size_of::<ZSTD_frameSizeInfo>(),
    );

    if format == ZSTD_f_zstd1 && ZSTD_isLegacy(src, srcSize) != 0 {
        return ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    }

    if format == ZSTD_f_zstd1
        && (srcSize >= ZSTD_SKIPPABLEHEADERSIZE)
        && (MEM_readLE32(src as *const BYTE) & ZSTD_MAGIC_SKIPPABLE_MASK)
            == ZSTD_MAGIC_SKIPPABLE_START
    {
        frameSizeInfo.compressedSize = readSkippableFrameSize(src, srcSize);
        return frameSizeInfo;
    } else {
        let mut ip: *const BYTE = src as *const BYTE;
        let ipstart: *const BYTE = ip;
        let mut remainingSize: usize = srcSize;
        let mut nbBlocks: usize = 0;
        let mut zfh = ZSTD_FrameHeader::default();

        /* Extract Frame Header */
        {
            let ret: usize = ZSTD_getFrameHeader_advanced(&mut zfh, src, srcSize, format);
            if ZSTD_isError(ret) != 0 {
                return ZSTD_errorFrameSizeInfo(ret);
            }
            if ret > 0 {
                return ZSTD_errorFrameSizeInfo(ERROR(ZSTD_error_srcSize_wrong));
            }
        }

        ip = ip.wrapping_add(zfh.headerSize as usize);
        remainingSize -= zfh.headerSize as usize;

        /* Iterate over each block */
        loop {
            let mut blockProperties = blockProperties_t::default();
            let cBlockSize: usize =
                ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
            if ZSTD_isError(cBlockSize) != 0 {
                return ZSTD_errorFrameSizeInfo(cBlockSize);
            }

            if ZSTD_blockHeaderSize + cBlockSize > remainingSize {
                return ZSTD_errorFrameSizeInfo(ERROR(ZSTD_error_srcSize_wrong));
            }

            ip = ip.wrapping_add(ZSTD_blockHeaderSize + cBlockSize);
            remainingSize -= ZSTD_blockHeaderSize + cBlockSize;
            nbBlocks = nbBlocks.wrapping_add(1);

            if blockProperties.lastBlock != 0 {
                break;
            }
        }

        /* Final frame content checksum */
        if zfh.checksumFlag != 0 {
            if remainingSize < 4 {
                return ZSTD_errorFrameSizeInfo(ERROR(ZSTD_error_srcSize_wrong));
            }
            ip = ip.wrapping_add(4);
        }

        frameSizeInfo.nbBlocks = nbBlocks;
        frameSizeInfo.compressedSize = (ip as usize).wrapping_sub(ipstart as usize);
        frameSizeInfo.decompressedBound = if zfh.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
            zfh.frameContentSize
        } else {
            (nbBlocks as c_ulonglong).wrapping_mul(zfh.blockSizeMax as c_ulonglong)
        };
        return frameSizeInfo;
    }
}

pub unsafe fn ZSTD_findFrameCompressedSize_advanced(
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> usize {
    let frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_findFrameSizeInfo(src, srcSize, format);
    frameSizeInfo.compressedSize
}

/* ZSTD_findFrameCompressedSize() :
 * See docs in zstd.h
 * Note: compatible with legacy mode */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findFrameCompressedSize(
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_findFrameCompressedSize_advanced(src, srcSize, ZSTD_f_zstd1)
}

/* ZSTD_decompressBound() :
 *  compatible with legacy mode
 *  @return : the maximum decompressed size of the compressed source
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBound(
    mut src: *const c_void,
    mut srcSize: usize,
) -> c_ulonglong {
    let mut bound: c_ulonglong = 0;
    /* Iterate over each frame */
    while srcSize > 0 {
        let frameSizeInfo: ZSTD_frameSizeInfo =
            ZSTD_findFrameSizeInfo(src, srcSize, ZSTD_f_zstd1);
        let compressedSize: usize = frameSizeInfo.compressedSize;
        let decompressedBound: c_ulonglong = frameSizeInfo.decompressedBound;
        if ZSTD_isError(compressedSize) != 0 || decompressedBound == ZSTD_CONTENTSIZE_ERROR {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        src = (src as *const BYTE).wrapping_add(compressedSize) as *const c_void;
        srcSize -= compressedSize;
        bound = bound.wrapping_add(decompressedBound);
    }
    bound
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressionMargin(
    mut src: *const c_void,
    mut srcSize: usize,
) -> usize {
    let mut margin: usize = 0;
    let mut maxBlockSize: c_uint = 0;

    /* Iterate over each frame */
    while srcSize > 0 {
        let frameSizeInfo: ZSTD_frameSizeInfo =
            ZSTD_findFrameSizeInfo(src, srcSize, ZSTD_f_zstd1);
        let compressedSize: usize = frameSizeInfo.compressedSize;
        let decompressedBound: c_ulonglong = frameSizeInfo.decompressedBound;
        let mut zfh = ZSTD_FrameHeader::default();

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
            margin = margin.wrapping_add(zfh.headerSize as usize);
            /* Add the checksum to our margin */
            margin = margin.wrapping_add(if zfh.checksumFlag != 0 { 4 } else { 0 });
            /* Add 3 bytes per block */
            margin = margin.wrapping_add(3usize.wrapping_mul(frameSizeInfo.nbBlocks));

            /* Compute the max block size */
            maxBlockSize = MAX(maxBlockSize, zfh.blockSizeMax);
        } else {
            /* Add the entire skippable frame size to our margin. */
            margin = margin.wrapping_add(compressedSize);
        }

        src = (src as *const BYTE).wrapping_add(compressedSize) as *const c_void;
        srcSize -= compressedSize;
    }

    /* Add the max block size back to the margin. */
    margin = margin.wrapping_add(maxBlockSize as usize);

    margin
}

/*-*************************************************************
 *   Frame decoding
 ***************************************************************/

/* ZSTD_insertBlock() :
 *  insert `src` block into `dctx` history. Useful to track uncompressed blocks. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_insertBlock(
    dctx: *mut ZSTD_DCtx,
    blockStart: *const c_void,
    blockSize: usize,
) -> usize {
    ZSTD_checkContinuity(dctx, blockStart, blockSize);
    (*dctx).previousDstEnd =
        (blockStart as *const c_char).wrapping_add(blockSize) as *const c_void;
    blockSize
}

pub unsafe fn ZSTD_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
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
    dstCapacity: usize,
    b: BYTE,
    regenSize: usize,
) -> usize {
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

/* ZSTD_TRACE == 0 : this is a no-op */
pub unsafe fn ZSTD_DCtx_trace_end(
    dctx: *const ZSTD_DCtx,
    uncompressedSize: U64,
    compressedSize: U64,
    streaming: c_int,
) {
}

/* ZSTD_decompressFrame() :
 * @dctx must be properly initialized
 *  will update *srcPtr and *srcSizePtr,
 *  to make *srcPtr progress by one frame. */
pub unsafe fn ZSTD_decompressFrame(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    srcPtr: *mut *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart: *const BYTE = *srcPtr as *const BYTE;
    let mut ip: *const BYTE = istart;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = if dstCapacity != 0 {
        ostart.wrapping_add(dstCapacity)
    } else {
        ostart
    };
    let mut op: *mut BYTE = ostart;
    let mut remainingSrcSize: usize = *srcSizePtr;

    /* check */
    if remainingSrcSize < ZSTD_FRAMEHEADERSIZE_MIN((*dctx).format) + ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Frame Header */
    {
        let frameHeaderSize: usize = ZSTD_frameHeaderSize_internal(
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
        ip = ip.wrapping_add(frameHeaderSize);
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
        let decodedSize: usize;
        let mut blockProperties = blockProperties_t::default();
        let cBlockSize: usize =
            ZSTD_getcBlockSize(ip as *const c_void, remainingSrcSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.wrapping_add(ZSTD_blockHeaderSize);
        remainingSrcSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSrcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        if (ip as usize) >= (op as usize) && (ip as usize) < (oBlockEnd as usize) {
            /* We are decompressing in-place. Limit the output pointer so that we
             * don't overwrite the block that we are currently reading.
             */
            oBlockEnd = op.wrapping_offset((ip as isize).wrapping_sub(op as isize));
        }

        match blockProperties.blockType {
            bt_compressed => {
                decodedSize = ZSTD_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    (oBlockEnd as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                    not_streaming,
                );
            }
            bt_raw => {
                /* Use oend instead of oBlockEnd because this function is safe to overlap. It uses memmove. */
                decodedSize = ZSTD_copyRawBlock(
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_rle => {
                decodedSize = ZSTD_setRleBlock(
                    op as *mut c_void,
                    (oBlockEnd as usize).wrapping_sub(op as usize),
                    *ip,
                    blockProperties.origSize as usize,
                );
            }
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
            ZSTD_XXH64_update(
                core::ptr::addr_of_mut!((*dctx).xxhState),
                op as *const c_void,
                decodedSize,
            );
        }
        if decodedSize != 0 {
            /* support dst = NULL,0 */
            op = op.wrapping_add(decodedSize);
        }
        ip = ip.wrapping_add(cBlockSize);
        remainingSrcSize -= cBlockSize;
        if blockProperties.lastBlock != 0 {
            break;
        }
    }

    if (*dctx).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
        if ((op as usize).wrapping_sub(ostart as usize) as U64) != (*dctx).fParams.frameContentSize
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
    }
    if (*dctx).fParams.checksumFlag != 0 {
        /* Frame content checksum verification */
        if remainingSrcSize < 4 {
            return ERROR(ZSTD_error_checksum_wrong);
        }
        if (*dctx).forceIgnoreChecksum == 0 {
            let checkCalc: U32 =
                ZSTD_XXH64_digest(core::ptr::addr_of!((*dctx).xxhState)) as U32;
            let mut checkRead: U32;
            checkRead = MEM_readLE32(ip);
            if checkRead != checkCalc {
                return ERROR(ZSTD_error_checksum_wrong);
            }
        }
        ip = ip.wrapping_add(4);
        remainingSrcSize -= 4;
    }
    ZSTD_DCtx_trace_end(
        dctx,
        (op as usize).wrapping_sub(ostart as usize) as U64,
        (ip as usize).wrapping_sub(istart as usize) as U64,
        0,
    );
    /* Allow caller to get size read */
    *srcPtr = ip as *const c_void;
    *srcSizePtr = remainingSrcSize;
    (op as usize).wrapping_sub(ostart as usize)
}

pub unsafe fn ZSTD_decompressMultiFrame(
    dctx: *mut ZSTD_DCtx,
    mut dst: *mut c_void,
    mut dstCapacity: usize,
    mut src: *const c_void,
    mut srcSize: usize,
    mut dict: *const c_void,
    mut dictSize: usize,
    ddict: *const ZSTD_DDict,
) -> usize {
    let dststart: *mut c_void = dst;
    let mut moreThan1Frame: c_int = 0;

    if !ddict.is_null() {
        dict = ZSTD_DDict_dictContent(ddict);
        dictSize = ZSTD_DDict_dictSize(ddict);
    }

    while srcSize >= ZSTD_startingInputLength((*dctx).format) {
        if (*dctx).format == ZSTD_f_zstd1 && ZSTD_isLegacy(src, srcSize) != 0 {
            let decodedSize: usize;
            let frameSize: usize = ZSTD_findFrameCompressedSizeLegacy(src, srcSize);
            if ZSTD_isError(frameSize) != 0 {
                return frameSize;
            }
            if (*dctx).staticSize != 0 {
                return ERROR(ZSTD_error_memory_allocation);
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

            dst = (dst as *mut BYTE).wrapping_add(decodedSize) as *mut c_void;
            dstCapacity -= decodedSize;

            src = (src as *const BYTE).wrapping_add(frameSize) as *const c_void;
            srcSize -= frameSize;

            continue;
        }

        if (*dctx).format == ZSTD_f_zstd1 && srcSize >= 4 {
            let magicNumber: U32 = MEM_readLE32(src as *const BYTE);
            if (magicNumber & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
                /* skippable frame detected : skip it */
                let skippableSize: usize = readSkippableFrameSize(src, srcSize);
                {
                    let err_code = skippableSize;
                    if ERR_isError(err_code) != 0 {
                        return err_code;
                    }
                }

                src = (src as *const BYTE).wrapping_add(skippableSize) as *const c_void;
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
            /* this will initialize correctly with no dict if dict == NULL, so
             * use this in all cases but ddict */
            let err_code = ZSTD_decompressBegin_usingDict(dctx, dict, dictSize);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        ZSTD_checkContinuity(dctx, dst, dstCapacity);

        {
            let res: usize =
                ZSTD_decompressFrame(dctx, dst, dstCapacity, &mut src, &mut srcSize);
            if (ZSTD_getErrorCode(res) == ZSTD_error_prefix_unknown) && (moreThan1Frame == 1) {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if ZSTD_isError(res) != 0 {
                return res;
            }
            if res != 0 {
                dst = (dst as *mut BYTE).wrapping_add(res) as *mut c_void;
            }
            dstCapacity -= res;
        }
        moreThan1Frame = 1;
    } /* while (srcSize >= ZSTD_frameHeaderSize_prefix) */

    if srcSize != 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (dst as usize).wrapping_sub(dststart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress_usingDict(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
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
        _ => {
            /* ZSTD_dont_use, and the impossible default */
            ZSTD_clearDict(dctx);
            core::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressDCtx(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompress_usingDDict(dctx, dst, dstCapacity, src, srcSize, ZSTD_getDDict(dctx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let regenSize: usize;
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
pub unsafe extern "C" fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected
}

/*
 * Similar to ZSTD_nextSrcSizeToDecompress(), but when a block input can be streamed, we
 * allow taking a partial block as the input.
 */
pub unsafe fn ZSTD_nextSrcSizeToDecompressWithInputSize(
    dctx: *mut ZSTD_DCtx,
    inputSize: usize,
) -> usize {
    if !((*dctx).stage == ZSTDds_decompressBlock || (*dctx).stage == ZSTDds_decompressLastBlock) {
        return (*dctx).expected;
    }
    if (*dctx).bType != bt_raw {
        return (*dctx).expected;
    }
    /* BOUNDED(1, inputSize, dctx->expected) */
    MAX(1, MIN(inputSize, (*dctx).expected))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_nextInputType(dctx: *mut ZSTD_DCtx) -> ZSTD_nextInputType_e {
    match (*dctx).stage {
        ZSTDds_decodeBlockHeader => ZSTDnit_blockHeader,
        ZSTDds_decompressBlock => ZSTDnit_block,
        ZSTDds_decompressLastBlock => ZSTDnit_lastBlock,
        ZSTDds_checkChecksum => ZSTDnit_checksum,
        ZSTDds_decodeSkippableHeader | ZSTDds_skipFrame => ZSTDnit_skippableFrame,
        /* ZSTDds_getFrameHeaderSize, ZSTDds_decodeFrameHeader, and default */
        _ => ZSTDnit_frameHeader,
    }
}

pub unsafe fn ZSTD_isSkipFrame(dctx: *mut ZSTD_DCtx) -> c_int {
    ((*dctx).stage == ZSTDds_skipFrame) as c_int
}

/* ZSTD_decompressContinue() :
 *  srcSize : must be the exact nb of bytes expected (see ZSTD_nextSrcSizeToDecompress())
 *  @return : nb of bytes generated into `dst` (necessarily <= `dstCapacity)
 *            or an error code, which can be tested using ZSTD_isError() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressContinue(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* Sanity check */
    if srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ZSTD_checkContinuity(dctx, dst, dstCapacity);

    (*dctx).processedCSize = (*dctx).processedCSize.wrapping_add(srcSize as U64);

    match (*dctx).stage {
        ZSTDds_getFrameHeaderSize => {
            if (*dctx).format == ZSTD_f_zstd1 {
                /* allows header */
                if (MEM_readLE32(src as *const BYTE) & ZSTD_MAGIC_SKIPPABLE_MASK)
                    == ZSTD_MAGIC_SKIPPABLE_START
                {
                    /* skippable frame */
                    ZSTD_memcpy(
                        core::ptr::addr_of_mut!((*dctx).headerBuffer) as *mut u8,
                        src as *const u8,
                        srcSize,
                    );
                    (*dctx).expected = ZSTD_SKIPPABLEHEADERSIZE.wrapping_sub(srcSize); /* remaining to load to get full skippable frame header */
                    (*dctx).stage = ZSTDds_decodeSkippableHeader;
                    return 0;
                }
            }
            (*dctx).headerSize = ZSTD_frameHeaderSize_internal(src, srcSize, (*dctx).format);
            if ZSTD_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            ZSTD_memcpy(
                core::ptr::addr_of_mut!((*dctx).headerBuffer) as *mut u8,
                src as *const u8,
                srcSize,
            );
            (*dctx).expected = (*dctx).headerSize.wrapping_sub(srcSize);
            (*dctx).stage = ZSTDds_decodeFrameHeader;
            return 0;
        }

        ZSTDds_decodeFrameHeader => {
            ZSTD_memcpy(
                (core::ptr::addr_of_mut!((*dctx).headerBuffer) as *mut u8)
                    .wrapping_add((*dctx).headerSize.wrapping_sub(srcSize)),
                src as *const u8,
                srcSize,
            );
            {
                let err_code = ZSTD_decodeFrameHeader(
                    dctx,
                    core::ptr::addr_of!((*dctx).headerBuffer) as *const c_void,
                    (*dctx).headerSize,
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            (*dctx).expected = ZSTD_blockHeaderSize;
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            return 0;
        }

        ZSTDds_decodeBlockHeader => {
            let mut bp = blockProperties_t::default();
            let cBlockSize: usize = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
            if ZSTD_isError(cBlockSize) != 0 {
                return cBlockSize;
            }
            if cBlockSize > (*dctx).fParams.blockSizeMax as usize {
                return ERROR(ZSTD_error_corruption_detected);
            }
            (*dctx).expected = cBlockSize;
            (*dctx).bType = bp.blockType;
            (*dctx).rleSize = bp.origSize as usize;
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
            return 0;
        }

        ZSTDds_decompressLastBlock | ZSTDds_decompressBlock => {
            let rSize: usize;
            match (*dctx).bType {
                bt_compressed => {
                    rSize = ZSTD_decompressBlock_internal(
                        dctx,
                        dst,
                        dstCapacity,
                        src,
                        srcSize,
                        is_streaming,
                    );
                    (*dctx).expected = 0; /* Streaming not supported */
                }
                bt_raw => {
                    rSize = ZSTD_copyRawBlock(dst, dstCapacity, src, srcSize);
                    {
                        let err_code = rSize;
                        if ERR_isError(err_code) != 0 {
                            return err_code;
                        }
                    }
                    (*dctx).expected = (*dctx).expected.wrapping_sub(rSize);
                }
                bt_rle => {
                    rSize =
                        ZSTD_setRleBlock(dst, dstCapacity, *(src as *const BYTE), (*dctx).rleSize);
                    (*dctx).expected = 0; /* Streaming not supported */
                }
                _ => {
                    /* bt_reserved : should never happen */
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            {
                let err_code = rSize;
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            if rSize > (*dctx).fParams.blockSizeMax as usize {
                return ERROR(ZSTD_error_corruption_detected);
            }
            (*dctx).decodedSize = (*dctx).decodedSize.wrapping_add(rSize as U64);
            if (*dctx).validateChecksum != 0 {
                ZSTD_XXH64_update(
                    core::ptr::addr_of_mut!((*dctx).xxhState),
                    dst as *const c_void,
                    rSize,
                );
            }
            (*dctx).previousDstEnd =
                (dst as *mut c_char).wrapping_add(rSize) as *const c_void;

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
                        1,
                    );
                    (*dctx).expected = 0; /* ends here */
                    (*dctx).stage = ZSTDds_getFrameHeaderSize;
                }
            } else {
                (*dctx).stage = ZSTDds_decodeBlockHeader;
                (*dctx).expected = ZSTD_blockHeaderSize;
            }
            return rSize;
        }

        ZSTDds_checkChecksum => {
            if (*dctx).validateChecksum != 0 {
                let h32: U32 = ZSTD_XXH64_digest(core::ptr::addr_of!((*dctx).xxhState)) as U32;
                let check32: U32 = MEM_readLE32(src as *const BYTE);
                if check32 != h32 {
                    return ERROR(ZSTD_error_checksum_wrong);
                }
            }
            ZSTD_DCtx_trace_end(dctx, (*dctx).decodedSize, (*dctx).processedCSize, 1);
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            return 0;
        }

        ZSTDds_decodeSkippableHeader => {
            ZSTD_memcpy(
                (core::ptr::addr_of_mut!((*dctx).headerBuffer) as *mut u8)
                    .wrapping_add(ZSTD_SKIPPABLEHEADERSIZE.wrapping_sub(srcSize)),
                src as *const u8,
                srcSize,
            ); /* complete skippable header */
            /* note : dctx->expected can grow seriously large, beyond local buffer size */
            (*dctx).expected = MEM_readLE32(
                (core::ptr::addr_of!((*dctx).headerBuffer) as *const u8)
                    .wrapping_add(ZSTD_FRAMEIDSIZE),
            ) as usize;
            (*dctx).stage = ZSTDds_skipFrame;
            return 0;
        }

        ZSTDds_skipFrame => {
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            return 0;
        }

        _ => {
            /* impossible */
            return ERROR(ZSTD_error_GENERIC);
        }
    }
}

pub unsafe fn ZSTD_refDictContent(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).virtualStart = (dict as usize).wrapping_sub(
        ((*dctx).previousDstEnd as usize).wrapping_sub((*dctx).prefixStart as usize),
    ) as *const c_void;
    (*dctx).prefixStart = dict;
    (*dctx).previousDstEnd =
        (dict as *const c_char).wrapping_add(dictSize) as *const c_void;
    0
}

/* ZSTD_loadDEntropy() :
 *  dict : must point at beginning of a valid zstd dictionary.
 * @return : size of entropy tables read */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_loadDEntropy(
    entropy: *mut ZSTD_entropyDTables_t,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut dictPtr: *const BYTE = dict as *const BYTE;
    let dictEnd: *const BYTE = dictPtr.wrapping_add(dictSize);

    if dictSize <= 8 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dictPtr = dictPtr.wrapping_add(8); /* skip header = magic + dictID */

    {
        /* use fse tables as temporary workspace; implies fse tables are grouped together */
        let workspace: *mut c_void = core::ptr::addr_of_mut!((*entropy).LLTable) as *mut c_void;
        let workspaceSize: usize = core::mem::size_of::<[ZSTD_seqSymbol; LLTABLE_SIZE]>()
            + core::mem::size_of::<[ZSTD_seqSymbol; OFTABLE_SIZE]>()
            + core::mem::size_of::<[ZSTD_seqSymbol; MLTABLE_SIZE]>();
        let hSize: usize = HUF_readDTableX2_wksp(
            core::ptr::addr_of_mut!((*entropy).hufTable) as *mut HUF_DTable,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
            workspace,
            workspaceSize,
            0,
        );
        if HUF_isError(hSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dictPtr = dictPtr.wrapping_add(hSize);
    }

    {
        let mut offcodeNCount: [i16; (MaxOff + 1) as usize] = [0; (MaxOff + 1) as usize];
        let mut offcodeMaxValue: c_uint = MaxOff;
        let mut offcodeLog: c_uint = 0;
        let offcodeHeaderSize: usize = FSE_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
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
            core::ptr::addr_of_mut!((*entropy).OFTable) as *mut ZSTD_seqSymbol,
            offcodeNCount.as_ptr(),
            offcodeMaxValue,
            OF_base.as_ptr(),
            OF_bits.as_ptr(),
            offcodeLog,
            core::ptr::addr_of_mut!((*entropy).workspace) as *mut c_void,
            core::mem::size_of::<[U32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32]>(),
            0,
        );
        dictPtr = dictPtr.wrapping_add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount: [i16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut matchlengthMaxValue: c_uint = MaxML;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize: usize = FSE_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
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
            core::ptr::addr_of_mut!((*entropy).MLTable) as *mut ZSTD_seqSymbol,
            matchlengthNCount.as_ptr(),
            matchlengthMaxValue,
            ML_base.as_ptr(),
            ML_bits.as_ptr(),
            matchlengthLog,
            core::ptr::addr_of_mut!((*entropy).workspace) as *mut c_void,
            core::mem::size_of::<[U32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32]>(),
            0,
        );
        dictPtr = dictPtr.wrapping_add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount: [i16; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
        let mut litlengthMaxValue: c_uint = MaxLL;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize: usize = FSE_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
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
            core::ptr::addr_of_mut!((*entropy).LLTable) as *mut ZSTD_seqSymbol,
            litlengthNCount.as_ptr(),
            litlengthMaxValue,
            LL_base.as_ptr(),
            LL_bits.as_ptr(),
            litlengthLog,
            core::ptr::addr_of_mut!((*entropy).workspace) as *mut c_void,
            core::mem::size_of::<[U32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32]>(),
            0,
        );
        dictPtr = dictPtr.wrapping_add(litlengthHeaderSize);
    }

    if (dictPtr.wrapping_add(12) as usize) > (dictEnd as usize) {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    {
        let mut i: c_int;
        let dictContentSize: usize =
            (dictEnd as usize).wrapping_sub(dictPtr.wrapping_add(12) as usize);
        i = 0;
        while i < 3 {
            let rep: U32 = MEM_readLE32(dictPtr);
            dictPtr = dictPtr.wrapping_add(4);
            if rep == 0 || rep as usize > dictContentSize {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
            (*entropy).rep[i as usize] = rep;
            i += 1;
        }
    }

    (dictPtr as usize).wrapping_sub(dict as usize)
}

pub unsafe fn ZSTD_decompress_insertDictionary(
    dctx: *mut ZSTD_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    if dictSize < 8 {
        return ZSTD_refDictContent(dctx, dict, dictSize);
    }
    {
        let magic: U32 = MEM_readLE32(dict as *const BYTE);
        if magic != ZSTD_MAGIC_DICTIONARY {
            return ZSTD_refDictContent(dctx, dict, dictSize); /* pure content mode */
        }
    }
    (*dctx).dictID = MEM_readLE32(
        (dict as *const c_char).wrapping_add(ZSTD_FRAMEIDSIZE) as *const BYTE,
    );

    /* load entropy tables */
    {
        let eSize: usize =
            ZSTD_loadDEntropy(core::ptr::addr_of_mut!((*dctx).entropy), dict, dictSize);
        if ZSTD_isError(eSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dict = (dict as *const c_char).wrapping_add(eSize) as *const c_void;
        dictSize -= eSize;
    }
    (*dctx).litEntropy = 1;
    (*dctx).fseEntropy = 1;

    /* reference dictionary content */
    ZSTD_refDictContent(dctx, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected = ZSTD_startingInputLength((*dctx).format); /* dctx->format must be properly set */
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).processedCSize = 0;
    (*dctx).decodedSize = 0;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).prefixStart = core::ptr::null();
    (*dctx).virtualStart = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    /* cover both little and big endian */
    (*dctx).entropy.hufTable[0] =
        (ZSTD_HUFFDTABLE_CAPACITY_LOG).wrapping_mul(0x1000001) as HUF_DTable;
    (*dctx).litEntropy = 0;
    (*dctx).fseEntropy = 0;
    (*dctx).dictID = 0;
    (*dctx).bType = bt_reserved;
    (*dctx).isFrameDecompression = 1;
    ZSTD_memcpy(
        core::ptr::addr_of_mut!((*dctx).entropy.rep) as *mut u8,
        repStartValue.as_ptr() as *const u8,
        core::mem::size_of::<[U32; ZSTD_REP_NUM]>(),
    ); /* initial repcodes */
    (*dctx).LLTptr = core::ptr::addr_of!((*dctx).entropy.LLTable) as *const ZSTD_seqSymbol;
    (*dctx).MLTptr = core::ptr::addr_of!((*dctx).entropy.MLTable) as *const ZSTD_seqSymbol;
    (*dctx).OFTptr = core::ptr::addr_of!((*dctx).entropy.OFTable) as *const ZSTD_seqSymbol;
    (*dctx).HUFptr = core::ptr::addr_of!((*dctx).entropy.hufTable) as *const HUF_DTable;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin_usingDict(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
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

/* ======   ZSTD_DDict   ====== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin_usingDDict(
    dctx: *mut ZSTD_DCtx,
    ddict: *const ZSTD_DDict,
) -> usize {
    if !ddict.is_null() {
        let dictStart: *const c_char = ZSTD_DDict_dictContent(ddict) as *const c_char;
        let dictSize: usize = ZSTD_DDict_dictSize(ddict);
        let dictEnd: *const c_void = dictStart.wrapping_add(dictSize) as *const c_void;
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

/* ZSTD_getDictID_fromDict() :
 *  Provides the dictID stored within dictionary. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDict(
    dict: *const c_void,
    dictSize: usize,
) -> c_uint {
    if dictSize < 8 {
        return 0;
    }
    if MEM_readLE32(dict as *const BYTE) != ZSTD_MAGIC_DICTIONARY {
        return 0;
    }
    MEM_readLE32((dict as *const c_char).wrapping_add(ZSTD_FRAMEIDSIZE) as *const BYTE)
}

/* ZSTD_getDictID_fromFrame() :
 *  Provides the dictID required to decompress frame stored within `src`. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromFrame(
    src: *const c_void,
    srcSize: usize,
) -> c_uint {
    let mut zfp = ZSTD_FrameHeader {
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
    let hError: usize = ZSTD_getFrameHeader(&mut zfp, src, srcSize);
    if ZSTD_isError(hError) != 0 {
        return 0;
    }
    zfp.dictID
}

/* ZSTD_decompress_usingDDict() :
*   Decompression using a pre-digested Dictionary
*   Use dictionary without significant overhead. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress_usingDDict(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    ddict: *const ZSTD_DDict,
) -> usize {
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
pub extern "C" fn ZSTD_createDStream() -> *mut ZSTD_DStream {
    unsafe { ZSTD_createDCtx_internal(ZSTD_defaultCMem) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DStream {
    ZSTD_initStaticDCtx(workspace, workspaceSize)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_createDStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_DStream {
    unsafe { ZSTD_createDCtx_internal(customMem) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDStream(zds: *mut ZSTD_DStream) -> usize {
    ZSTD_freeDCtx(zds)
}

/* ***  Initialization  *** */

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_DStreamInSize() -> usize {
    ZSTD_BLOCKSIZE_MAX as usize + ZSTD_blockHeaderSize
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_DStreamOutSize() -> usize {
    ZSTD_BLOCKSIZE_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary_advanced(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
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
    dictSize: usize,
) -> usize {
    ZSTD_DCtx_loadDictionary_advanced(dctx, dict, dictSize, ZSTD_dlm_byRef, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTD_DCtx_loadDictionary_advanced(dctx, dict, dictSize, ZSTD_dlm_byCopy, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refPrefix_advanced(
    dctx: *mut ZSTD_DCtx,
    prefix: *const c_void,
    prefixSize: usize,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    {
        let err_code = ZSTD_DCtx_loadDictionary_advanced(
            dctx,
            prefix,
            prefixSize,
            ZSTD_dlm_byRef,
            dictContentType,
        );
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
    prefixSize: usize,
) -> usize {
    ZSTD_DCtx_refPrefix_advanced(dctx, prefix, prefixSize, ZSTD_dct_rawContent)
}

/* ZSTD_initDStream_usingDict() :
 * return : expected size, aka ZSTD_startingInputLength().
 * this function cannot fail */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDict(
    zds: *mut ZSTD_DStream,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
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

/* note : this variant can't fail */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream(zds: *mut ZSTD_DStream) -> usize {
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

/* ZSTD_initDStream_usingDDict() :
 * ddict will just be referenced, and must outlive decompression session
 * this function cannot fail */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDDict(
    dctx: *mut ZSTD_DStream,
    ddict: *const ZSTD_DDict,
) -> usize {
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

/* ZSTD_resetDStream() :
 * return : expected size, aka ZSTD_startingInputLength().
 * this function cannot fail */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetDStream(dctx: *mut ZSTD_DStream) -> usize {
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
) -> usize {
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
            {
                let err_code =
                    ZSTD_DDictHashSet_addDDict((*dctx).ddictSet, ddict, (*dctx).customMem);
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
        }
    }
    0
}

/* ZSTD_DCtx_setMaxWindowSize() :
 * note : no direct equivalence in ZSTD_DCtx_setParameter,
 * since this version sets windowSize, and the other sets windowLog */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setMaxWindowSize(
    dctx: *mut ZSTD_DCtx,
    maxWindowSize: usize,
) -> usize {
    let bounds: ZSTD_bounds = ZSTD_dParam_getBounds(ZSTD_d_windowLogMax);
    let min: usize = (1usize) << bounds.lowerBound;
    let max: usize = (1usize) << bounds.upperBound;
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
) -> usize {
    ZSTD_DCtx_setParameter(dctx, ZSTD_d_format, format as c_int)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_dParam_getBounds(dParam: ZSTD_dParameter) -> ZSTD_bounds {
    let mut bounds = ZSTD_bounds {
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

/* ZSTD_dParam_withinBounds:
 * @return 1 if value is within dParam bounds,
 * 0 otherwise */
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
) -> usize {
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
) -> usize {
    if (*dctx).streamStage != zdss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    match dParam {
        ZSTD_d_windowLogMax => {
            if value == 0 {
                value = ZSTD_WINDOWLOG_LIMIT_DEFAULT;
            }
            /* CHECK_DBOUNDS */
            if ZSTD_dParam_withinBounds(ZSTD_d_windowLogMax, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).maxWindowSize = (1usize) << value;
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
            if value != 0 {
                if ZSTD_dParam_withinBounds(ZSTD_d_maxBlockSize, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
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
) -> usize {
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
pub unsafe extern "C" fn ZSTD_sizeof_DStream(dctx: *const ZSTD_DStream) -> usize {
    ZSTD_sizeof_DCtx(dctx)
}

pub unsafe fn ZSTD_decodingBufferSize_internal(
    windowSize: c_ulonglong,
    frameContentSize: c_ulonglong,
    blockSizeMax: usize,
) -> usize {
    let blockSize: usize = MIN(
        MIN(windowSize, ZSTD_BLOCKSIZE_MAX as c_ulonglong) as usize,
        blockSizeMax,
    );
    /* We need blockSize + WILDCOPY_OVERLENGTH worth of buffer so that if a block
     * ends at windowSize + WILDCOPY_OVERLENGTH + 1 bytes, we can start writing
     * the block at the beginning of the output buffer, and maintain a full window.
     *
     * We need another blockSize worth of buffer so that we can store split
     * literals at the end of the block without overwriting the extDict window.
     */
    let neededRBSize: c_ulonglong = windowSize
        .wrapping_add(blockSize.wrapping_mul(2) as c_ulonglong)
        .wrapping_add((WILDCOPY_OVERLENGTH * 2) as c_ulonglong);
    let neededSize: c_ulonglong = MIN(frameContentSize, neededRBSize);
    let minRBSize: usize = neededSize as usize;
    if (minRBSize as c_ulonglong) != neededSize {
        return ERROR(ZSTD_error_frameParameter_windowTooLarge);
    }
    minRBSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodingBufferSize_min(
    windowSize: c_ulonglong,
    frameContentSize: c_ulonglong,
) -> usize {
    ZSTD_decodingBufferSize_internal(windowSize, frameContentSize, ZSTD_BLOCKSIZE_MAX as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize(windowSize: usize) -> usize {
    let blockSize: usize = MIN(windowSize, ZSTD_BLOCKSIZE_MAX as usize);
    let inBuffSize: usize = blockSize; /* no block can be larger */
    let outBuffSize: usize =
        ZSTD_decodingBufferSize_min(windowSize as c_ulonglong, ZSTD_CONTENTSIZE_UNKNOWN);
    ZSTD_estimateDCtxSize() + inBuffSize + outBuffSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize_fromFrame(
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* note : should be user-selectable, but requires an additional parameter (or a dctx) */
    let windowSizeMax: U32 = 1u32 << ZSTD_WINDOWLOG_MAX;
    let mut zfh = ZSTD_FrameHeader::default();
    let err: usize = ZSTD_getFrameHeader(&mut zfh, src, srcSize);
    if ZSTD_isError(err) != 0 {
        return err;
    }
    if err > 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if zfh.windowSize > windowSizeMax as c_ulonglong {
        return ERROR(ZSTD_error_frameParameter_windowTooLarge);
    }
    ZSTD_estimateDStreamSize(zfh.windowSize as usize)
}

/* *****   Decompression   ***** */

pub unsafe fn ZSTD_DCtx_isOverflow(
    zds: *mut ZSTD_DStream,
    neededInBuffSize: usize,
    neededOutBuffSize: usize,
) -> c_int {
    (((*zds).inBuffSize.wrapping_add((*zds).outBuffSize))
        >= (neededInBuffSize.wrapping_add(neededOutBuffSize))
            .wrapping_mul(ZSTD_WORKSPACETOOLARGE_FACTOR)) as c_int
}

pub unsafe fn ZSTD_DCtx_updateOversizedDuration(
    zds: *mut ZSTD_DStream,
    neededInBuffSize: usize,
    neededOutBuffSize: usize,
) {
    if ZSTD_DCtx_isOverflow(zds, neededInBuffSize, neededOutBuffSize) != 0 {
        (*zds).oversizedDuration = (*zds).oversizedDuration.wrapping_add(1);
    } else {
        (*zds).oversizedDuration = 0;
    }
}

pub unsafe fn ZSTD_DCtx_isOversizedTooLong(zds: *mut ZSTD_DStream) -> c_int {
    ((*zds).oversizedDuration >= ZSTD_WORKSPACETOOLARGE_MAXDURATION as usize) as c_int
}

/* Checks that the output buffer hasn't changed if ZSTD_obm_stable is used. */
pub unsafe fn ZSTD_checkOutBuffer(
    zds: *const ZSTD_DStream,
    output: *const ZSTD_outBuffer,
) -> usize {
    let expect: ZSTD_outBuffer = (*zds).expectedOutBuffer;
    /* No requirement when ZSTD_obm_stable is not enabled. */
    if (*zds).outBufferMode != ZSTD_bm_stable {
        return 0;
    }
    /* Any buffer is allowed in zdss_init, this must be the same for every other call until
     * the context is reset.
     */
    if (*zds).streamStage == zdss_init {
        return 0;
    }
    /* The buffer must match our expectation exactly. */
    if expect.dst == (*output).dst && expect.pos == (*output).pos && expect.size == (*output).size {
        return 0;
    }
    ERROR(ZSTD_error_dstBuffer_wrong)
}

/* Calls ZSTD_decompressContinue() with the right parameters for ZSTD_decompressStream()
 * and updates the stage and the output buffer state. This call is extracted so it can be
 * used both when reading directly from the ZSTD_inBuffer, and in buffered input mode.
 * NOTE: You must break after calling this function since the streamStage is modified.
 */
pub unsafe fn ZSTD_decompressContinueStream(
    zds: *mut ZSTD_DStream,
    op: *mut *mut c_char,
    oend: *mut c_char,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let isSkipFrame: c_int = ZSTD_isSkipFrame(zds);
    if (*zds).outBufferMode == ZSTD_bm_buffered {
        let dstSize: usize = if isSkipFrame != 0 {
            0
        } else {
            (*zds).outBuffSize.wrapping_sub((*zds).outStart)
        };
        let decodedSize: usize = ZSTD_decompressContinue(
            zds,
            (*zds).outBuff.wrapping_add((*zds).outStart) as *mut c_void,
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
            (*zds).outEnd = (*zds).outStart.wrapping_add(decodedSize);
            (*zds).streamStage = zdss_flush;
        }
    } else {
        /* Write directly into the output buffer */
        let dstSize: usize = if isSkipFrame != 0 {
            0
        } else {
            (oend as usize).wrapping_sub(*op as usize)
        };
        let decodedSize: usize =
            ZSTD_decompressContinue(zds, *op as *mut c_void, dstSize, src, srcSize);
        {
            let err_code = decodedSize;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        *op = (*op).wrapping_add(decodedSize);
        /* Flushing is not needed. */
        (*zds).streamStage = zdss_read;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream(
    zds: *mut ZSTD_DStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    let src: *const c_char = (*input).src as *const c_char;
    let istart: *const c_char = if (*input).pos != 0 {
        src.wrapping_add((*input).pos)
    } else {
        src
    };
    let iend: *const c_char = if (*input).size != 0 {
        src.wrapping_add((*input).size)
    } else {
        src
    };
    let mut ip: *const c_char = istart;
    let dst: *mut c_char = (*output).dst as *mut c_char;
    let ostart: *mut c_char = if (*output).pos != 0 {
        dst.wrapping_add((*output).pos)
    } else {
        dst
    };
    let oend: *mut c_char = if (*output).size != 0 {
        dst.wrapping_add((*output).size)
    } else {
        dst
    };
    let mut op: *mut c_char = ostart;
    let mut someMoreWork: U32 = 1;

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
        let mut stage: ZSTD_dStreamStage = (*zds).streamStage;
        'switch_stage: loop {
            if stage == zdss_init {
                (*zds).streamStage = zdss_loadHeader;
                (*zds).lhSize = 0;
                (*zds).inPos = 0;
                (*zds).outStart = 0;
                (*zds).outEnd = 0;
                (*zds).legacyVersion = 0;
                (*zds).hostageByte = 0;
                (*zds).expectedOutBuffer = *output;
                stage = zdss_loadHeader;
                continue 'switch_stage;
                /* ZSTD_FALLTHROUGH */
            }

            if stage == zdss_loadHeader {
                if (*zds).legacyVersion != 0 {
                    if (*zds).staticSize != 0 {
                        return ERROR(ZSTD_error_memory_allocation);
                    }
                    {
                        let hint: usize = ZSTD_decompressLegacyStream(
                            (*zds).legacyContext,
                            (*zds).legacyVersion,
                            output,
                            input,
                        );
                        if hint == 0 {
                            (*zds).streamStage = zdss_init;
                        }
                        return hint;
                    }
                }
                {
                    let hSize: usize = ZSTD_getFrameHeader_advanced(
                        core::ptr::addr_of_mut!((*zds).fParams),
                        core::ptr::addr_of!((*zds).headerBuffer) as *const c_void,
                        (*zds).lhSize,
                        (*zds).format,
                    );
                    if (*zds).refMultipleDDicts != 0 && !(*zds).ddictSet.is_null() {
                        ZSTD_DCtx_selectFrameDDict(zds);
                    }
                    if ZSTD_isError(hSize) != 0 {
                        let legacyVersion: U32 = ZSTD_isLegacy(
                            istart as *const c_void,
                            (iend as usize).wrapping_sub(istart as usize),
                        );
                        if legacyVersion != 0 {
                            let ddict: *const ZSTD_DDict = ZSTD_getDDict(zds);
                            let dict: *const c_void = if !ddict.is_null() {
                                ZSTD_DDict_dictContent(ddict)
                            } else {
                                core::ptr::null()
                            };
                            let dictSize: usize = if !ddict.is_null() {
                                ZSTD_DDict_dictSize(ddict)
                            } else {
                                0
                            };
                            if (*zds).staticSize != 0 {
                                return ERROR(ZSTD_error_memory_allocation);
                            }
                            {
                                let err_code = ZSTD_initLegacyStream(
                                    core::ptr::addr_of_mut!((*zds).legacyContext),
                                    (*zds).previousLegacyVersion,
                                    legacyVersion,
                                    dict,
                                    dictSize,
                                );
                                if ERR_isError(err_code) != 0 {
                                    return err_code;
                                }
                            }
                            (*zds).previousLegacyVersion = legacyVersion;
                            (*zds).legacyVersion = legacyVersion;
                            {
                                let hint: usize = ZSTD_decompressLegacyStream(
                                    (*zds).legacyContext,
                                    legacyVersion,
                                    output,
                                    input,
                                );
                                if hint == 0 {
                                    (*zds).streamStage = zdss_init; /* or stay in stage zdss_loadHeader */
                                }
                                return hint;
                            }
                        }
                        return hSize; /* error */
                    }
                    if hSize != 0 {
                        /* need more input */
                        let toLoad: usize = hSize.wrapping_sub((*zds).lhSize); /* if hSize!=0, hSize > zds->lhSize */
                        let remainingInput: usize =
                            (iend as usize).wrapping_sub(ip as usize);
                        if toLoad > remainingInput {
                            /* not enough input to load full header */
                            if remainingInput > 0 {
                                ZSTD_memcpy(
                                    (core::ptr::addr_of_mut!((*zds).headerBuffer) as *mut u8)
                                        .wrapping_add((*zds).lhSize),
                                    ip as *const u8,
                                    remainingInput,
                                );
                                (*zds).lhSize =
                                    (*zds).lhSize.wrapping_add(remainingInput);
                            }
                            (*input).pos = (*input).size;
                            /* check first few bytes */
                            {
                                let err_code = ZSTD_getFrameHeader_advanced(
                                    core::ptr::addr_of_mut!((*zds).fParams),
                                    core::ptr::addr_of!((*zds).headerBuffer) as *const c_void,
                                    (*zds).lhSize,
                                    (*zds).format,
                                );
                                if ERR_isError(err_code) != 0 {
                                    return err_code;
                                }
                            }
                            /* return hint input size */
                            /* remaining header bytes + next block header */
                            return (MAX(ZSTD_FRAMEHEADERSIZE_MIN((*zds).format), hSize)
                                .wrapping_sub((*zds).lhSize))
                                + ZSTD_blockHeaderSize;
                        }
                        ZSTD_memcpy(
                            (core::ptr::addr_of_mut!((*zds).headerBuffer) as *mut u8)
                                .wrapping_add((*zds).lhSize),
                            ip as *const u8,
                            toLoad,
                        );
                        (*zds).lhSize = hSize;
                        ip = ip.wrapping_add(toLoad);
                        break 'switch_stage;
                    }
                }

                /* check for single-pass mode opportunity */
                if (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
                    && (*zds).fParams.frameType != ZSTD_skippableFrame
                    && ((oend as usize).wrapping_sub(op as usize) as U64)
                        >= (*zds).fParams.frameContentSize
                {
                    let cSize: usize = ZSTD_findFrameCompressedSize_advanced(
                        istart as *const c_void,
                        (iend as usize).wrapping_sub(istart as usize),
                        (*zds).format,
                    );
                    if cSize <= (iend as usize).wrapping_sub(istart as usize) {
                        /* shortcut : using single-pass mode */
                        let decompressedSize: usize = ZSTD_decompress_usingDDict(
                            zds,
                            op as *mut c_void,
                            (oend as usize).wrapping_sub(op as usize),
                            istart as *const c_void,
                            cSize,
                            ZSTD_getDDict(zds),
                        );
                        if ZSTD_isError(decompressedSize) != 0 {
                            return decompressedSize;
                        }
                        ip = istart.wrapping_add(cSize);
                        /* can occur if frameContentSize = 0 (empty frame) */
                        op = if !op.is_null() {
                            op.wrapping_add(decompressedSize)
                        } else {
                            op
                        };
                        (*zds).expected = 0;
                        (*zds).streamStage = zdss_init;
                        someMoreWork = 0;
                        break 'switch_stage;
                    }
                }

                /* Check output buffer is large enough for ZSTD_odm_stable. */
                if (*zds).outBufferMode == ZSTD_bm_stable
                    && (*zds).fParams.frameType != ZSTD_skippableFrame
                    && (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
                    && ((oend as usize).wrapping_sub(op as usize) as U64)
                        < (*zds).fParams.frameContentSize
                {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                }

                /* Consume header (see ZSTDds_decodeFrameHeader) */
                {
                    let err_code = ZSTD_decompressBegin_usingDDict(zds, ZSTD_getDDict(zds));
                    if ERR_isError(err_code) != 0 {
                        return err_code;
                    }
                }

                if (*zds).format == ZSTD_f_zstd1
                    && (MEM_readLE32(core::ptr::addr_of!((*zds).headerBuffer) as *const BYTE)
                        & ZSTD_MAGIC_SKIPPABLE_MASK)
                        == ZSTD_MAGIC_SKIPPABLE_START
                {
                    /* skippable frame */
                    (*zds).expected = MEM_readLE32(
                        (core::ptr::addr_of!((*zds).headerBuffer) as *const u8)
                            .wrapping_add(ZSTD_FRAMEIDSIZE),
                    ) as usize;
                    (*zds).stage = ZSTDds_skipFrame;
                } else {
                    {
                        let err_code = ZSTD_decodeFrameHeader(
                            zds,
                            core::ptr::addr_of!((*zds).headerBuffer) as *const c_void,
                            (*zds).lhSize,
                        );
                        if ERR_isError(err_code) != 0 {
                            return err_code;
                        }
                    }
                    (*zds).expected = ZSTD_blockHeaderSize;
                    (*zds).stage = ZSTDds_decodeBlockHeader;
                }

                /* control buffer memory usage */
                (*zds).fParams.windowSize = MAX(
                    (*zds).fParams.windowSize,
                    (1u32 << ZSTD_WINDOWLOG_ABSOLUTEMIN) as c_ulonglong,
                );
                if (*zds).fParams.windowSize > (*zds).maxWindowSize as c_ulonglong {
                    return ERROR(ZSTD_error_frameParameter_windowTooLarge);
                }
                if (*zds).maxBlockSizeParam != 0 {
                    (*zds).fParams.blockSizeMax = MIN(
                        (*zds).fParams.blockSizeMax,
                        (*zds).maxBlockSizeParam as c_uint,
                    );
                }

                /* Adapt buffer sizes to frame header instructions */
                {
                    let neededInBuffSize: usize =
                        MAX((*zds).fParams.blockSizeMax, 4 /* frame checksum */) as usize;
                    let neededOutBuffSize: usize = if (*zds).outBufferMode == ZSTD_bm_buffered {
                        ZSTD_decodingBufferSize_internal(
                            (*zds).fParams.windowSize,
                            (*zds).fParams.frameContentSize,
                            (*zds).fParams.blockSizeMax as usize,
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
                            let bufferSize: usize =
                                neededInBuffSize.wrapping_add(neededOutBuffSize);
                            if (*zds).staticSize != 0 {
                                /* static DCtx */
                                if bufferSize
                                    > (*zds)
                                        .staticSize
                                        .wrapping_sub(core::mem::size_of::<ZSTD_DCtx>())
                                {
                                    return ERROR(ZSTD_error_memory_allocation);
                                }
                            } else {
                                ZSTD_customFree((*zds).inBuff as *mut u8, (*zds).customMem);
                                (*zds).inBuffSize = 0;
                                (*zds).outBuffSize = 0;
                                (*zds).inBuff =
                                    ZSTD_customMalloc(bufferSize, (*zds).customMem) as *mut c_char;
                                if (*zds).inBuff.is_null() {
                                    return ERROR(ZSTD_error_memory_allocation);
                                }
                            }
                            (*zds).inBuffSize = neededInBuffSize;
                            (*zds).outBuff = (*zds).inBuff.wrapping_add((*zds).inBuffSize);
                            (*zds).outBuffSize = neededOutBuffSize;
                        }
                    }
                }
                (*zds).streamStage = zdss_read;
                stage = zdss_read;
                continue 'switch_stage;
                /* ZSTD_FALLTHROUGH */
            }

            if stage == zdss_read {
                {
                    let neededInSize: usize = ZSTD_nextSrcSizeToDecompressWithInputSize(
                        zds,
                        (iend as usize).wrapping_sub(ip as usize),
                    );
                    if neededInSize == 0 {
                        /* end of frame */
                        (*zds).streamStage = zdss_init;
                        someMoreWork = 0;
                        break 'switch_stage;
                    }
                    if (iend as usize).wrapping_sub(ip as usize) >= neededInSize {
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
                        ip = ip.wrapping_add(neededInSize);
                        /* Function modifies the stage so we must break */
                        break 'switch_stage;
                    }
                }
                if ip == iend {
                    someMoreWork = 0;
                    break 'switch_stage;
                } /* no more input */
                (*zds).streamStage = zdss_load;
                stage = zdss_load;
                continue 'switch_stage;
                /* ZSTD_FALLTHROUGH */
            }

            if stage == zdss_load {
                {
                    let neededInSize: usize = ZSTD_nextSrcSizeToDecompress(zds);
                    let toLoad: usize = neededInSize.wrapping_sub((*zds).inPos);
                    let isSkipFrame: c_int = ZSTD_isSkipFrame(zds);
                    let loadedSize: usize;
                    /* At this point we shouldn't be decompressing a block that we can stream. */
                    if isSkipFrame != 0 {
                        loadedSize = MIN(toLoad, (iend as usize).wrapping_sub(ip as usize));
                    } else {
                        if toLoad > (*zds).inBuffSize.wrapping_sub((*zds).inPos) {
                            return ERROR(ZSTD_error_corruption_detected);
                        }
                        loadedSize = ZSTD_limitCopy(
                            (*zds).inBuff.wrapping_add((*zds).inPos) as *mut u8,
                            toLoad,
                            ip as *const u8,
                            (iend as usize).wrapping_sub(ip as usize),
                        );
                    }
                    if loadedSize != 0 {
                        /* ip may be NULL */
                        ip = ip.wrapping_add(loadedSize);
                        (*zds).inPos = (*zds).inPos.wrapping_add(loadedSize);
                    }
                    if loadedSize < toLoad {
                        someMoreWork = 0;
                        break 'switch_stage;
                    } /* not enough input, wait for more */

                    /* decode loaded input */
                    (*zds).inPos = 0; /* input is consumed */
                    {
                        let err_code = ZSTD_decompressContinueStream(
                            zds,
                            &mut op,
                            oend,
                            (*zds).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ERR_isError(err_code) != 0 {
                            return err_code;
                        }
                    }
                    /* Function modifies the stage so we must break */
                    break 'switch_stage;
                }
            }

            if stage == zdss_flush {
                {
                    let toFlushSize: usize = (*zds).outEnd.wrapping_sub((*zds).outStart);
                    let flushedSize: usize = ZSTD_limitCopy(
                        op as *mut u8,
                        (oend as usize).wrapping_sub(op as usize),
                        (*zds).outBuff.wrapping_add((*zds).outStart) as *const u8,
                        toFlushSize,
                    );

                    op = if !op.is_null() {
                        op.wrapping_add(flushedSize)
                    } else {
                        op
                    };

                    (*zds).outStart = (*zds).outStart.wrapping_add(flushedSize);
                    if flushedSize == toFlushSize {
                        /* flush completed */
                        (*zds).streamStage = zdss_read;
                        if ((*zds).outBuffSize as c_ulonglong)
                            < (*zds).fParams.frameContentSize
                            && ((*zds)
                                .outStart
                                .wrapping_add((*zds).fParams.blockSizeMax as usize)
                                > (*zds).outBuffSize)
                        {
                            (*zds).outStart = 0;
                            (*zds).outEnd = 0;
                        }
                        break 'switch_stage;
                    }
                }
                /* cannot complete flush */
                someMoreWork = 0;
                break 'switch_stage;
            }

            /* default : impossible */
            return ERROR(ZSTD_error_GENERIC);
        }
    }

    /* result */
    (*input).pos = (ip as usize).wrapping_sub((*input).src as usize);
    (*output).pos = (op as usize).wrapping_sub((*output).dst as usize);

    /* Update the expected output buffer for ZSTD_obm_stable. */
    (*zds).expectedOutBuffer = *output;

    if (ip == istart) && (op == ostart) {
        /* no forward progress */
        (*zds).noForwardProgress = (*zds).noForwardProgress.wrapping_add(1);
        if (*zds).noForwardProgress >= ZSTD_NO_FORWARD_PROGRESS_MAX {
            if op == oend {
                return ERROR(ZSTD_error_noForwardProgress_destFull);
            }
            if ip == iend {
                return ERROR(ZSTD_error_noForwardProgress_inputEmpty);
            }
        }
    } else {
        (*zds).noForwardProgress = 0;
    }
    {
        let mut nextSrcSizeHint: usize = ZSTD_nextSrcSizeToDecompress(zds);
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
                } /* zds->hostageByte */
                return 0;
            } /* zds->outEnd == zds->outStart */
            if (*zds).hostageByte == 0 {
                /* output not fully flushed; keep last byte as hostage; will be released when all output is flushed */
                (*input).pos -= 1; /* note : pos > 0, otherwise, impossible to finish reading last block */
                (*zds).hostageByte = 1;
            }
            return 1;
        } /* nextSrcSizeHint==0 */
        /* preload header of next block */
        nextSrcSizeHint = nextSrcSizeHint.wrapping_add(
            ZSTD_blockHeaderSize.wrapping_mul(
                ((ZSTD_nextInputType(zds) == ZSTDnit_block) as usize),
            ),
        );
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zds).inPos); /* part already loaded*/
        return nextSrcSizeHint;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream_simpleArgs(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    dstPos: *mut usize,
    src: *const c_void,
    srcSize: usize,
    srcPos: *mut usize,
) -> usize {
    let mut output = ZSTD_outBuffer {
        dst: core::ptr::null_mut(),
        size: 0,
        pos: 0,
    };
    let mut input = ZSTD_inBuffer {
        src: core::ptr::null(),
        size: 0,
        pos: 0,
    };
    output.dst = dst;
    output.size = dstCapacity;
    output.pos = *dstPos;
    input.src = src;
    input.size = srcSize;
    input.pos = *srcPos;
    {
        let cErr: usize = ZSTD_decompressStream(dctx, &mut output, &mut input);
        *dstPos = output.pos;
        *srcPos = input.pos;
        return cErr;
    }
}
