//! Translation of `decompress/zstd_decompress.c`.
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::allocations::*;
use crate::bits::zstd_highbit32;
use crate::error::*;
use crate::mem::*;
use crate::xxhash::*;
use crate::zstd_decompress_internal::*;
use crate::zstd_internal::*;
use crate::zstd_public::*;

// ===== Cross-module dependencies (implemented elsewhere) =====

// module `crate::zstd_decompress_block`
extern "C" {
    /// `ZSTD_decompressBlock_internal()` (zstd_decompress_block.h)
    fn ZSTD_decompressBlock_internal(
        dctx: *mut ZSTD_DCtx,
        dst: *mut c_void,
        dst_capacity: usize,
        src: *const c_void,
        src_size: usize,
        streaming: streaming_operation,
    ) -> usize;
    /// `ZSTD_checkContinuity()` (zstd_decompress_block.c, published in zstd_internal.h)
    fn ZSTD_checkContinuity(dctx: *mut ZSTD_DCtx, dst: *const c_void, dst_size: usize);
    /// `ZSTD_getcBlockSize()` (published in zstd_internal.h)
    fn ZSTD_getcBlockSize(
        src: *const c_void,
        src_size: usize,
        bp_ptr: *mut blockProperties_t,
    ) -> usize;
    /// `ZSTD_buildFSETable()` (zstd_decompress_block.h)
    fn ZSTD_buildFSETable(
        dt: *mut ZSTD_seqSymbol,
        normalized_counter: *const i16,
        max_symbol_value: c_uint,
        base_value: *const U32,
        nb_additional_bits: *const u8,
        table_log: c_uint,
        wksp: *mut c_void,
        wksp_size: usize,
        bmi2: c_int,
    );
}

// module `crate::zstd_ddict`
extern "C" {
    fn ZSTD_DDict_dictContent(ddict: *const ZSTD_DDict) -> *const c_void;
    fn ZSTD_DDict_dictSize(ddict: *const ZSTD_DDict) -> usize;
    fn ZSTD_copyDDictParameters(dctx: *mut ZSTD_DCtx, ddict: *const ZSTD_DDict);
    fn ZSTD_createDDict_advanced(
        dict: *const c_void,
        dict_size: usize,
        dict_load_method: ZSTD_dictLoadMethod_e,
        dict_content_type: ZSTD_dictContentType_e,
        custom_mem: ZSTD_customMem,
    ) -> *mut ZSTD_DDict;
    fn ZSTD_freeDDict(ddict: *mut ZSTD_DDict) -> usize;
    fn ZSTD_sizeof_DDict(ddict: *const ZSTD_DDict) -> usize;
    fn ZSTD_getDictID_fromDDict(ddict: *const ZSTD_DDict) -> c_uint;
}

// `HUF_readDTableX2_wksp` / `FSE_readNCount` / error helpers live in sibling modules.
use crate::entropy_common::FSE_readNCount;
use crate::fse::{FSE_isError, HUF_isError};
use crate::huf::HUF_DTable;
use crate::huf_decompress::HUF_readDTableX2_wksp;

// ===== local constants from the .c tuning section =====

/// `ZSTD_HEAPMODE`
const ZSTD_HEAPMODE: c_int = 1;
/// `ZSTD_MAXWINDOWSIZE_DEFAULT`
const ZSTD_MAXWINDOWSIZE_DEFAULT: usize = (1usize << ZSTD_WINDOWLOG_LIMIT_DEFAULT) + 1;
/// `ZSTD_NO_FORWARD_PROGRESS_MAX`
const ZSTD_NO_FORWARD_PROGRESS_MAX: c_int = 16;

/// `ZSTD_WINDOWLOG_LIMIT_DEFAULT` (zstd.h)
const ZSTD_WINDOWLOG_LIMIT_DEFAULT: c_int = 27;
/// `ZSTD_BLOCKSIZE_MAX_MIN` (zstd.h)
const ZSTD_BLOCKSIZE_MAX_MIN: c_int = 1 << 10;

/// `streaming_operation` (zstd_decompress_block.h)
pub type streaming_operation = c_uint;
pub const not_streaming: streaming_operation = 0;
pub const is_streaming: streaming_operation = 1;

/// `ZSTD_nextInputType_e` (zstd.h)
pub type ZSTD_nextInputType_e = c_uint;
pub const ZSTDnit_frameHeader: ZSTD_nextInputType_e = 0;
pub const ZSTDnit_blockHeader: ZSTD_nextInputType_e = 1;
pub const ZSTDnit_block: ZSTD_nextInputType_e = 2;
pub const ZSTDnit_lastBlock: ZSTD_nextInputType_e = 3;
pub const ZSTDnit_checksum: ZSTD_nextInputType_e = 4;
pub const ZSTDnit_skippableFrame: ZSTD_nextInputType_e = 5;

/// dParameter aliases (zstd.h maps these onto experimental params).
const ZSTD_d_format: ZSTD_dParameter = ZSTD_d_experimentalParam1;
const ZSTD_d_stableOutBuffer: ZSTD_dParameter = ZSTD_d_experimentalParam2;
const ZSTD_d_forceIgnoreChecksum: ZSTD_dParameter = ZSTD_d_experimentalParam3;
const ZSTD_d_refMultipleDDicts: ZSTD_dParameter = ZSTD_d_experimentalParam4;
const ZSTD_d_disableHuffmanAssembly: ZSTD_dParameter = ZSTD_d_experimentalParam5;
const ZSTD_d_maxBlockSize: ZSTD_dParameter = ZSTD_d_experimentalParam6;

/*************************************
 * Multiple DDicts Hashset internals *
 *************************************/

const DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT: usize = 4;
const DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT: usize = 3;
const DDICT_HASHSET_TABLE_BASE_SIZE: usize = 64;
const DDICT_HASHSET_RESIZE_FACTOR: usize = 2;

/// `ZSTD_DDictHashSet_getIndex()`
unsafe fn ZSTD_DDictHashSet_getIndex(hash_set: *const ZSTD_DDictHashSet, dict_id: U32) -> usize {
    let hash: U64 = ZSTD_XXH64(
        &dict_id as *const U32 as *const c_void,
        core::mem::size_of::<U32>(),
        0,
    );
    (hash & ((*hash_set).ddictPtrTableSize as U64 - 1)) as usize
}

/// `ZSTD_DDictHashSet_emplaceDDict()`
unsafe fn ZSTD_DDictHashSet_emplaceDDict(
    hash_set: *mut ZSTD_DDictHashSet,
    ddict: *const ZSTD_DDict,
) -> usize {
    let dict_id: U32 = ZSTD_getDictID_fromDDict(ddict);
    let mut idx = ZSTD_DDictHashSet_getIndex(hash_set, dict_id);
    let idx_range_mask = (*hash_set).ddictPtrTableSize - 1;
    if (*hash_set).ddictPtrCount == (*hash_set).ddictPtrTableSize {
        return err_code(ZSTD_error_GENERIC);
    }
    while !(*(*hash_set).ddictPtrTable.add(idx)).is_null() {
        if ZSTD_getDictID_fromDDict(*(*hash_set).ddictPtrTable.add(idx)) == dict_id {
            *(*hash_set).ddictPtrTable.add(idx) = ddict;
            return 0;
        }
        idx &= idx_range_mask;
        idx += 1;
    }
    *(*hash_set).ddictPtrTable.add(idx) = ddict;
    (*hash_set).ddictPtrCount += 1;
    0
}

/// `ZSTD_DDictHashSet_expand()`
unsafe fn ZSTD_DDictHashSet_expand(
    hash_set: *mut ZSTD_DDictHashSet,
    custom_mem: ZSTD_customMem,
) -> usize {
    let new_table_size = (*hash_set).ddictPtrTableSize * DDICT_HASHSET_RESIZE_FACTOR;
    let new_table = zstd_custom_calloc(
        core::mem::size_of::<*const ZSTD_DDict>() * new_table_size,
        custom_mem,
    ) as *mut *const ZSTD_DDict;
    let old_table = (*hash_set).ddictPtrTable;
    let old_table_size = (*hash_set).ddictPtrTableSize;

    if new_table.is_null() {
        return err_code(ZSTD_error_memory_allocation);
    }
    (*hash_set).ddictPtrTable = new_table;
    (*hash_set).ddictPtrTableSize = new_table_size;
    (*hash_set).ddictPtrCount = 0;
    let mut i: usize = 0;
    while i < old_table_size {
        if !(*old_table.add(i)).is_null() {
            let err = ZSTD_DDictHashSet_emplaceDDict(hash_set, *old_table.add(i));
            if err_is_error(err) {
                return err;
            }
        }
        i += 1;
    }
    zstd_custom_free(old_table as *mut c_void, custom_mem);
    0
}

/// `ZSTD_DDictHashSet_getDDict()`
unsafe fn ZSTD_DDictHashSet_getDDict(
    hash_set: *mut ZSTD_DDictHashSet,
    dict_id: U32,
) -> *const ZSTD_DDict {
    let mut idx = ZSTD_DDictHashSet_getIndex(hash_set, dict_id);
    let idx_range_mask = (*hash_set).ddictPtrTableSize - 1;
    loop {
        let curr_dict_id = ZSTD_getDictID_fromDDict(*(*hash_set).ddictPtrTable.add(idx));
        if curr_dict_id == dict_id || curr_dict_id == 0 {
            break;
        } else {
            idx &= idx_range_mask;
            idx += 1;
        }
    }
    *(*hash_set).ddictPtrTable.add(idx)
}

/// `ZSTD_createDDictHashSet()`
unsafe fn ZSTD_createDDictHashSet(custom_mem: ZSTD_customMem) -> *mut ZSTD_DDictHashSet {
    let ret = zstd_custom_malloc(core::mem::size_of::<ZSTD_DDictHashSet>(), custom_mem)
        as *mut ZSTD_DDictHashSet;
    if ret.is_null() {
        return core::ptr::null_mut();
    }
    (*ret).ddictPtrTable = zstd_custom_calloc(
        DDICT_HASHSET_TABLE_BASE_SIZE * core::mem::size_of::<*const ZSTD_DDict>(),
        custom_mem,
    ) as *mut *const ZSTD_DDict;
    if (*ret).ddictPtrTable.is_null() {
        zstd_custom_free(ret as *mut c_void, custom_mem);
        return core::ptr::null_mut();
    }
    (*ret).ddictPtrTableSize = DDICT_HASHSET_TABLE_BASE_SIZE;
    (*ret).ddictPtrCount = 0;
    ret
}

/// `ZSTD_freeDDictHashSet()`
unsafe fn ZSTD_freeDDictHashSet(hash_set: *mut ZSTD_DDictHashSet, custom_mem: ZSTD_customMem) {
    if !hash_set.is_null() && !(*hash_set).ddictPtrTable.is_null() {
        zstd_custom_free((*hash_set).ddictPtrTable as *mut c_void, custom_mem);
    }
    if !hash_set.is_null() {
        zstd_custom_free(hash_set as *mut c_void, custom_mem);
    }
}

/// `ZSTD_DDictHashSet_addDDict()`
unsafe fn ZSTD_DDictHashSet_addDDict(
    hash_set: *mut ZSTD_DDictHashSet,
    ddict: *const ZSTD_DDict,
    custom_mem: ZSTD_customMem,
) -> usize {
    if (*hash_set).ddictPtrCount * DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT
        / (*hash_set).ddictPtrTableSize
        * DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT
        != 0
    {
        let err = ZSTD_DDictHashSet_expand(hash_set, custom_mem);
        if err_is_error(err) {
            return err;
        }
    }
    let err = ZSTD_DDictHashSet_emplaceDDict(hash_set, ddict);
    if err_is_error(err) {
        return err;
    }
    0
}

/*-*************************************************************
*   Context management
***************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DCtx(dctx: *const ZSTD_DCtx) -> usize {
    if dctx.is_null() {
        return 0;
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

/// `ZSTD_startingInputLength()`
fn ZSTD_startingInputLength(format: ZSTD_format_e) -> usize {
    let starting_input_length = zstd_frameheadersize_prefix(format);
    /* only supports formats ZSTD_f_zstd1 and ZSTD_f_zstd1_magicless */
    starting_input_length
}

/// `ZSTD_DCtx_resetParameters()`
unsafe fn ZSTD_DCtx_resetParameters(dctx: *mut ZSTD_DCtx) {
    (*dctx).format = ZSTD_f_zstd1;
    (*dctx).maxWindowSize = ZSTD_MAXWINDOWSIZE_DEFAULT;
    (*dctx).outBufferMode = ZSTD_bm_buffered;
    (*dctx).forceIgnoreChecksum = ZSTD_d_validateChecksum;
    (*dctx).refMultipleDDicts = ZSTD_rmd_refSingleDDict;
    (*dctx).disableHufAsm = 0;
    (*dctx).maxBlockSizeParam = 0;
}

/// `ZSTD_initDCtx_internal()`
unsafe fn ZSTD_initDCtx_internal(dctx: *mut ZSTD_DCtx) {
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
    // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.
    (*dctx).noForwardProgress = 0;
    (*dctx).oversizedDuration = 0;
    (*dctx).isFrameDecompression = 1;
    // NOTE: DYNAMIC_BMI2 == 0, so the `bmi2` field does not exist and is not set.
    (*dctx).ddictSet = core::ptr::null_mut();
    ZSTD_DCtx_resetParameters(dctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDCtx(
    workspace: *mut c_void,
    workspace_size: usize,
) -> *mut ZSTD_DCtx {
    let dctx = workspace as *mut ZSTD_DCtx;

    if (workspace as usize) & 7 != 0 {
        return core::ptr::null_mut();
    }
    if workspace_size < core::mem::size_of::<ZSTD_DCtx>() {
        return core::ptr::null_mut();
    }

    ZSTD_initDCtx_internal(dctx);
    (*dctx).staticSize = workspace_size;
    (*dctx).inBuff = dctx.add(1) as *mut core::ffi::c_char;
    dctx
}

/// `ZSTD_createDCtx_internal()`
unsafe fn ZSTD_createDCtx_internal(custom_mem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    if custom_mem.customAlloc.is_none() ^ custom_mem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    let dctx = zstd_custom_malloc(core::mem::size_of::<ZSTD_DCtx>(), custom_mem) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    (*dctx).customMem = custom_mem;
    ZSTD_initDCtx_internal(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDCtx_advanced(custom_mem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    ZSTD_createDCtx_internal(custom_mem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    ZSTD_createDCtx_internal(ZSTD_defaultCMem)
}

/// `ZSTD_clearDict()`
unsafe fn ZSTD_clearDict(dctx: *mut ZSTD_DCtx) {
    ZSTD_freeDDict((*dctx).ddictLocal);
    (*dctx).ddictLocal = core::ptr::null_mut();
    (*dctx).ddict = core::ptr::null();
    (*dctx).dictUses = ZSTD_dont_use;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    if dctx.is_null() {
        return 0;
    }
    if (*dctx).staticSize != 0 {
        return err_code(ZSTD_error_memory_allocation);
    }
    let c_mem = (*dctx).customMem;
    ZSTD_clearDict(dctx);
    zstd_custom_free((*dctx).inBuff as *mut c_void, c_mem);
    (*dctx).inBuff = core::ptr::null_mut();
    // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.
    if !(*dctx).ddictSet.is_null() {
        ZSTD_freeDDictHashSet((*dctx).ddictSet, c_mem);
        (*dctx).ddictSet = core::ptr::null_mut();
    }
    zstd_custom_free(dctx as *mut c_void, c_mem);
    0
}

/* no longer useful */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyDCtx(dst_dctx: *mut ZSTD_DCtx, src_dctx: *const ZSTD_DCtx) {
    let to_copy = (&(*dst_dctx).inBuff as *const _ as *const core::ffi::c_char as usize)
        .wrapping_sub(dst_dctx as *const core::ffi::c_char as usize);
    core::ptr::copy_nonoverlapping(
        src_dctx as *const u8,
        dst_dctx as *mut u8,
        to_copy,
    );
}

/// `ZSTD_DCtx_selectFrameDDict()`
unsafe fn ZSTD_DCtx_selectFrameDDict(dctx: *mut ZSTD_DCtx) {
    if !(*dctx).ddict.is_null() {
        let frame_ddict = ZSTD_DDictHashSet_getDDict((*dctx).ddictSet, (*dctx).fParams.dictID);
        if !frame_ddict.is_null() {
            ZSTD_clearDict(dctx);
            (*dctx).dictID = (*dctx).fParams.dictID;
            (*dctx).ddict = frame_ddict;
            (*dctx).dictUses = ZSTD_use_indefinitely;
        }
    }
}

/*-*************************************************************
 *   Frame header decoding
 ***************************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isFrame(buffer: *const c_void, size: usize) -> c_uint {
    if size < ZSTD_FRAMEIDSIZE {
        return 0;
    }
    let magic: U32 = mem_read_le32(buffer as *const u8);
    if magic == ZSTD_MAGICNUMBER {
        return 1;
    }
    if (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
        return 1;
    }
    // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isSkippableFrame(buffer: *const c_void, size: usize) -> c_uint {
    if size < ZSTD_FRAMEIDSIZE {
        return 0;
    }
    let magic: U32 = mem_read_le32(buffer as *const u8);
    if (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
        return 1;
    }
    0
}

/// `ZSTD_frameHeaderSize_internal()`
unsafe fn ZSTD_frameHeaderSize_internal(
    src: *const c_void,
    src_size: usize,
    format: ZSTD_format_e,
) -> usize {
    let min_input_size = ZSTD_startingInputLength(format);
    if src_size < min_input_size {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    let fhd: BYTE = *(src as *const BYTE).add(min_input_size - 1);
    let dict_id: U32 = (fhd & 3) as U32;
    let single_segment: U32 = ((fhd >> 5) & 1) as U32;
    let fcs_id: U32 = (fhd >> 6) as U32;
    // C: return minInputSize + !singleSegment + ZSTD_did_fieldSize[dictID]
    //          + ZSTD_fcs_fieldSize[fcsId] + (singleSegment && !fcsId);
    min_input_size
        + (single_segment == 0) as usize
        + ZSTD_did_fieldSize[dict_id as usize]
        + ZSTD_fcs_fieldSize[fcs_id as usize]
        + ((single_segment != 0) && (fcs_id == 0)) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_frameHeaderSize(src: *const c_void, src_size: usize) -> usize {
    ZSTD_frameHeaderSize_internal(src, src_size, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader_advanced(
    zfh_ptr: *mut ZSTD_FrameHeader,
    src: *const c_void,
    src_size: usize,
    format: ZSTD_format_e,
) -> usize {
    let ip = src as *const BYTE;
    let min_input_size = ZSTD_startingInputLength(format);

    if src_size > 0 {
        if src.is_null() {
            return err_code(ZSTD_error_GENERIC);
        }
    }
    if src_size < min_input_size {
        if src_size > 0 && format != ZSTD_f_zstd1_magicless {
            let to_copy = min_usize(4, src_size);
            let mut hbuf: [core::ffi::c_uchar; 4] = [0; 4];
            mem_write_le32(hbuf.as_mut_ptr(), ZSTD_MAGICNUMBER);
            core::ptr::copy_nonoverlapping(src as *const u8, hbuf.as_mut_ptr(), to_copy);
            if mem_read_le32(hbuf.as_ptr()) != ZSTD_MAGICNUMBER {
                mem_write_le32(hbuf.as_mut_ptr(), ZSTD_MAGIC_SKIPPABLE_START);
                core::ptr::copy_nonoverlapping(src as *const u8, hbuf.as_mut_ptr(), to_copy);
                if (mem_read_le32(hbuf.as_ptr()) & ZSTD_MAGIC_SKIPPABLE_MASK)
                    != ZSTD_MAGIC_SKIPPABLE_START
                {
                    return err_code(ZSTD_error_prefix_unknown);
                }
            }
        }
        return min_input_size;
    }

    core::ptr::write_bytes(zfh_ptr as *mut u8, 0, core::mem::size_of::<ZSTD_FrameHeader>());
    if format != ZSTD_f_zstd1_magicless && mem_read_le32(src as *const u8) != ZSTD_MAGICNUMBER {
        if (mem_read_le32(src as *const u8) & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START
        {
            /* skippable frame */
            if src_size < ZSTD_SKIPPABLEHEADERSIZE {
                return ZSTD_SKIPPABLEHEADERSIZE;
            }
            core::ptr::write_bytes(zfh_ptr as *mut u8, 0, core::mem::size_of::<ZSTD_FrameHeader>());
            (*zfh_ptr).frameType = ZSTD_skippableFrame;
            (*zfh_ptr).dictID = mem_read_le32(src as *const u8) - ZSTD_MAGIC_SKIPPABLE_START;
            (*zfh_ptr).headerSize = ZSTD_SKIPPABLEHEADERSIZE as c_uint;
            (*zfh_ptr).frameContentSize =
                mem_read_le32((src as *const core::ffi::c_char).add(ZSTD_FRAMEIDSIZE) as *const u8)
                    as u64;
            return 0;
        }
        return err_code(ZSTD_error_prefix_unknown);
    }

    /* ensure there is enough `srcSize` to fully read/decode frame header */
    {
        let fhsize = ZSTD_frameHeaderSize_internal(src, src_size, format);
        if src_size < fhsize {
            return fhsize;
        }
        (*zfh_ptr).headerSize = fhsize as U32;
    }

    {
        let fhd_byte: BYTE = *ip.add(min_input_size - 1);
        let mut pos = min_input_size;
        let dict_id_size_code: U32 = (fhd_byte & 3) as U32;
        let checksum_flag: U32 = ((fhd_byte >> 2) & 1) as U32;
        let single_segment: U32 = ((fhd_byte >> 5) & 1) as U32;
        let fcs_id: U32 = (fhd_byte >> 6) as U32;
        let mut window_size: U64 = 0;
        let mut dict_id: U32 = 0;
        let mut frame_content_size: U64 = ZSTD_CONTENTSIZE_UNKNOWN;
        if (fhd_byte & 0x08) != 0 {
            return err_code(ZSTD_error_frameParameter_unsupported);
        }

        if single_segment == 0 {
            let wl_byte: BYTE = *ip.add(pos);
            pos += 1;
            let window_log: U32 = (wl_byte >> 3) as U32 + ZSTD_WINDOWLOG_ABSOLUTEMIN;
            if window_log > ZSTD_WINDOWLOG_MAX {
                return err_code(ZSTD_error_frameParameter_windowTooLarge);
            }
            window_size = 1u64 << window_log;
            window_size += (window_size >> 3) * (wl_byte & 7) as U64;
        }
        match dict_id_size_code {
            1 => {
                dict_id = *ip.add(pos) as U32;
                pos += 1;
            }
            2 => {
                dict_id = mem_read_le16(ip.add(pos)) as U32;
                pos += 2;
            }
            3 => {
                dict_id = mem_read_le32(ip.add(pos));
                pos += 4;
            }
            _ => {}
        }
        match fcs_id {
            0 => {
                if single_segment != 0 {
                    frame_content_size = *ip.add(pos) as U64;
                }
            }
            1 => {
                frame_content_size = mem_read_le16(ip.add(pos)) as U64 + 256;
            }
            2 => {
                frame_content_size = mem_read_le32(ip.add(pos)) as U64;
            }
            3 => {
                frame_content_size = mem_read_le64(ip.add(pos));
            }
            _ => {}
        }
        if single_segment != 0 {
            window_size = frame_content_size;
        }

        (*zfh_ptr).frameType = ZSTD_frame;
        (*zfh_ptr).frameContentSize = frame_content_size;
        (*zfh_ptr).windowSize = window_size;
        (*zfh_ptr).blockSizeMax = min_u64(window_size, ZSTD_BLOCKSIZE_MAX as U64) as c_uint;
        (*zfh_ptr).dictID = dict_id;
        (*zfh_ptr).checksumFlag = checksum_flag;
    }
    0
}

#[inline(always)]
fn min_u64(a: U64, b: U64) -> U64 {
    if a < b {
        a
    } else {
        b
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader(
    zfh_ptr: *mut ZSTD_FrameHeader,
    src: *const c_void,
    src_size: usize,
) -> usize {
    ZSTD_getFrameHeader_advanced(zfh_ptr, src, src_size, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameContentSize(
    src: *const c_void,
    src_size: usize,
) -> core::ffi::c_ulonglong {
    // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.
    let mut zfh = ZSTD_FrameHeader::default();
    if ZSTD_getFrameHeader(&mut zfh, src, src_size) != 0 {
        return ZSTD_CONTENTSIZE_ERROR;
    }
    if zfh.frameType == ZSTD_skippableFrame {
        0
    } else {
        zfh.frameContentSize
    }
}

/// `readSkippableFrameSize()`
unsafe fn readSkippableFrameSize(src: *const c_void, src_size: usize) -> usize {
    let skippable_header_size = ZSTD_SKIPPABLEHEADERSIZE;

    if src_size < ZSTD_SKIPPABLEHEADERSIZE {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    let size_u32: U32 = mem_read_le32((src as *const BYTE).add(ZSTD_FRAMEIDSIZE));
    if (size_u32.wrapping_add(ZSTD_SKIPPABLEHEADERSIZE as U32)) < size_u32 {
        return err_code(ZSTD_error_frameParameter_unsupported);
    }
    let skippable_size = skippable_header_size + size_u32 as usize;
    if skippable_size > src_size {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    skippable_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_readSkippableFrame(
    dst: *mut c_void,
    dst_capacity: usize,
    magic_variant: *mut c_uint,
    src: *const c_void,
    src_size: usize,
) -> usize {
    if src_size < ZSTD_SKIPPABLEHEADERSIZE {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    let magic_number: U32 = mem_read_le32(src as *const u8);
    let skippable_frame_size = readSkippableFrameSize(src, src_size);
    let skippable_content_size = skippable_frame_size - ZSTD_SKIPPABLEHEADERSIZE;

    if ZSTD_isSkippableFrame(src, src_size) == 0 {
        return err_code(ZSTD_error_frameParameter_unsupported);
    }
    if skippable_frame_size < ZSTD_SKIPPABLEHEADERSIZE || skippable_frame_size > src_size {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    if skippable_content_size > dst_capacity {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }

    if skippable_content_size > 0 && !dst.is_null() {
        core::ptr::copy_nonoverlapping(
            (src as *const BYTE).add(ZSTD_SKIPPABLEHEADERSIZE),
            dst as *mut u8,
            skippable_content_size,
        );
    }
    if !magic_variant.is_null() {
        *magic_variant = magic_number - ZSTD_MAGIC_SKIPPABLE_START;
    }
    skippable_content_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findDecompressedSize(
    mut src: *const c_void,
    mut src_size: usize,
) -> core::ffi::c_ulonglong {
    let mut total_dst_size: core::ffi::c_ulonglong = 0;

    while src_size >= ZSTD_startingInputLength(ZSTD_f_zstd1) {
        let magic_number: U32 = mem_read_le32(src as *const u8);

        if (magic_number & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            let skippable_size = readSkippableFrameSize(src, src_size);
            if ZSTD_isError(skippable_size) != 0 {
                return ZSTD_CONTENTSIZE_ERROR;
            }
            src = (src as *const BYTE).add(skippable_size) as *const c_void;
            src_size -= skippable_size;
            continue;
        }

        let fcs = ZSTD_getFrameContentSize(src, src_size);
        if fcs >= ZSTD_CONTENTSIZE_ERROR {
            return fcs;
        }
        if total_dst_size + fcs < total_dst_size {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        total_dst_size += fcs;

        let frame_src_size = ZSTD_findFrameCompressedSize(src, src_size);
        if ZSTD_isError(frame_src_size) != 0 {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        src = (src as *const BYTE).add(frame_src_size) as *const c_void;
        src_size -= frame_src_size;
    }

    if src_size != 0 {
        return ZSTD_CONTENTSIZE_ERROR;
    }

    total_dst_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDecompressedSize(
    src: *const c_void,
    src_size: usize,
) -> core::ffi::c_ulonglong {
    let ret = ZSTD_getFrameContentSize(src, src_size);
    if ret >= ZSTD_CONTENTSIZE_ERROR {
        0
    } else {
        ret
    }
}

/// `ZSTD_decodeFrameHeader()`
unsafe fn ZSTD_decodeFrameHeader(dctx: *mut ZSTD_DCtx, src: *const c_void, header_size: usize) -> usize {
    let result = ZSTD_getFrameHeader_advanced(&mut (*dctx).fParams, src, header_size, (*dctx).format);
    if ZSTD_isError(result) != 0 {
        return result;
    }
    if result > 0 {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    if (*dctx).refMultipleDDicts == ZSTD_rmd_refMultipleDDicts && !(*dctx).ddictSet.is_null() {
        ZSTD_DCtx_selectFrameDDict(dctx);
    }

    if (*dctx).fParams.dictID != 0 && ((*dctx).dictID != (*dctx).fParams.dictID) {
        return err_code(ZSTD_error_dictionary_wrong);
    }
    (*dctx).validateChecksum =
        if (*dctx).fParams.checksumFlag != 0 && (*dctx).forceIgnoreChecksum == 0 {
            1
        } else {
            0
        };
    if (*dctx).validateChecksum != 0 {
        ZSTD_XXH64_reset(&mut (*dctx).xxhState, 0);
    }
    (*dctx).processedCSize += header_size as U64;
    0
}

/// `ZSTD_errorFrameSizeInfo()`
fn ZSTD_errorFrameSizeInfo(ret: usize) -> ZSTD_frameSizeInfo {
    ZSTD_frameSizeInfo {
        nbBlocks: 0,
        compressedSize: ret,
        decompressedBound: ZSTD_CONTENTSIZE_ERROR,
    }
}

/// `ZSTD_findFrameSizeInfo()`
unsafe fn ZSTD_findFrameSizeInfo(
    src: *const c_void,
    src_size: usize,
    format: ZSTD_format_e,
) -> ZSTD_frameSizeInfo {
    let mut frame_size_info = ZSTD_frameSizeInfo::default();

    // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.

    if format == ZSTD_f_zstd1
        && src_size >= ZSTD_SKIPPABLEHEADERSIZE
        && (mem_read_le32(src as *const u8) & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START
    {
        frame_size_info.compressedSize = readSkippableFrameSize(src, src_size);
        frame_size_info
    } else {
        let mut ip = src as *const BYTE;
        let ipstart = ip;
        let mut remaining_size = src_size;
        let mut nb_blocks: usize = 0;
        let mut zfh = ZSTD_FrameHeader::default();

        /* Extract Frame Header */
        {
            let ret = ZSTD_getFrameHeader_advanced(&mut zfh, src, src_size, format);
            if ZSTD_isError(ret) != 0 {
                return ZSTD_errorFrameSizeInfo(ret);
            }
            if ret > 0 {
                return ZSTD_errorFrameSizeInfo(err_code(ZSTD_error_srcSize_wrong));
            }
        }

        ip = ip.add(zfh.headerSize as usize);
        remaining_size -= zfh.headerSize as usize;

        /* Iterate over each block */
        loop {
            let mut block_properties = blockProperties_t::default();
            let c_block_size = ZSTD_getcBlockSize(ip as *const c_void, remaining_size, &mut block_properties);
            if ZSTD_isError(c_block_size) != 0 {
                return ZSTD_errorFrameSizeInfo(c_block_size);
            }

            if ZSTD_blockHeaderSize + c_block_size > remaining_size {
                return ZSTD_errorFrameSizeInfo(err_code(ZSTD_error_srcSize_wrong));
            }

            ip = ip.add(ZSTD_blockHeaderSize + c_block_size);
            remaining_size -= ZSTD_blockHeaderSize + c_block_size;
            nb_blocks += 1;

            if block_properties.lastBlock != 0 {
                break;
            }
        }

        /* Final frame content checksum */
        if zfh.checksumFlag != 0 {
            if remaining_size < 4 {
                return ZSTD_errorFrameSizeInfo(err_code(ZSTD_error_srcSize_wrong));
            }
            ip = ip.add(4);
        }

        frame_size_info.nbBlocks = nb_blocks;
        frame_size_info.compressedSize = ip as usize - ipstart as usize;
        frame_size_info.decompressedBound = if zfh.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
            zfh.frameContentSize
        } else {
            nb_blocks as core::ffi::c_ulonglong * zfh.blockSizeMax as core::ffi::c_ulonglong
        };
        frame_size_info
    }
}

/// `ZSTD_findFrameCompressedSize_advanced()`
unsafe fn ZSTD_findFrameCompressedSize_advanced(
    src: *const c_void,
    src_size: usize,
    format: ZSTD_format_e,
) -> usize {
    let frame_size_info = ZSTD_findFrameSizeInfo(src, src_size, format);
    frame_size_info.compressedSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findFrameCompressedSize(src: *const c_void, src_size: usize) -> usize {
    ZSTD_findFrameCompressedSize_advanced(src, src_size, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBound(
    mut src: *const c_void,
    mut src_size: usize,
) -> core::ffi::c_ulonglong {
    let mut bound: core::ffi::c_ulonglong = 0;
    while src_size > 0 {
        let frame_size_info = ZSTD_findFrameSizeInfo(src, src_size, ZSTD_f_zstd1);
        let compressed_size = frame_size_info.compressedSize;
        let decompressed_bound = frame_size_info.decompressedBound;
        if ZSTD_isError(compressed_size) != 0 || decompressed_bound == ZSTD_CONTENTSIZE_ERROR {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        src = (src as *const BYTE).add(compressed_size) as *const c_void;
        src_size -= compressed_size;
        bound += decompressed_bound;
    }
    bound
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressionMargin(mut src: *const c_void, mut src_size: usize) -> usize {
    let mut margin: usize = 0;
    let mut max_block_size: c_uint = 0;

    while src_size > 0 {
        let frame_size_info = ZSTD_findFrameSizeInfo(src, src_size, ZSTD_f_zstd1);
        let compressed_size = frame_size_info.compressedSize;
        let decompressed_bound = frame_size_info.decompressedBound;
        let mut zfh = ZSTD_FrameHeader::default();

        let e = ZSTD_getFrameHeader(&mut zfh, src, src_size);
        if err_is_error(e) {
            return e;
        }
        if ZSTD_isError(compressed_size) != 0 || decompressed_bound == ZSTD_CONTENTSIZE_ERROR {
            return err_code(ZSTD_error_corruption_detected);
        }

        if zfh.frameType == ZSTD_frame {
            margin += zfh.headerSize as usize;
            margin += if zfh.checksumFlag != 0 { 4 } else { 0 };
            margin += 3 * frame_size_info.nbBlocks;
            max_block_size = max_u32(max_block_size, zfh.blockSizeMax);
        } else {
            margin += compressed_size;
        }

        src = (src as *const BYTE).add(compressed_size) as *const c_void;
        src_size -= compressed_size;
    }

    margin += max_block_size as usize;

    margin
}

/*-*************************************************************
 *   Frame decoding
 ***************************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_insertBlock(
    dctx: *mut ZSTD_DCtx,
    block_start: *const c_void,
    block_size: usize,
) -> usize {
    ZSTD_checkContinuity(dctx, block_start, block_size);
    (*dctx).previousDstEnd = (block_start as *const core::ffi::c_char).add(block_size) as *const c_void;
    block_size
}

/// `ZSTD_copyRawBlock()`
unsafe fn ZSTD_copyRawBlock(
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    if src_size > dst_capacity {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    if dst.is_null() {
        if src_size == 0 {
            return 0;
        }
        return err_code(ZSTD_error_dstBuffer_null);
    }
    core::ptr::copy(src as *const u8, dst as *mut u8, src_size);
    src_size
}

/// `ZSTD_setRleBlock()`
unsafe fn ZSTD_setRleBlock(dst: *mut c_void, dst_capacity: usize, b: BYTE, regen_size: usize) -> usize {
    if regen_size > dst_capacity {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    if dst.is_null() {
        if regen_size == 0 {
            return 0;
        }
        return err_code(ZSTD_error_dstBuffer_null);
    }
    core::ptr::write_bytes(dst as *mut u8, b, regen_size);
    regen_size
}

/// `ZSTD_DCtx_trace_end()` — ZSTD_TRACE==1 but the weak hook is NULL, so this is a no-op.
unsafe fn ZSTD_DCtx_trace_end(
    _dctx: *const ZSTD_DCtx,
    _uncompressed_size: U64,
    _compressed_size: U64,
    _streaming: c_int,
) {
    // ZSTD_trace_decompress_end is a NULL weak symbol in this build → no-op.
}

/// `ZSTD_decompressFrame()`
unsafe fn ZSTD_decompressFrame(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    src_ptr: *mut *const c_void,
    src_size_ptr: *mut usize,
) -> usize {
    let istart = *src_ptr as *const BYTE;
    let mut ip = istart;
    let ostart = dst as *mut BYTE;
    let oend = if dst_capacity != 0 {
        ostart.add(dst_capacity)
    } else {
        ostart
    };
    let mut op = ostart;
    let mut remaining_src_size = *src_size_ptr;

    /* check */
    if remaining_src_size < zstd_frameheadersize_min((*dctx).format) + ZSTD_blockHeaderSize {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    /* Frame Header */
    {
        let frame_header_size = ZSTD_frameHeaderSize_internal(
            ip as *const c_void,
            zstd_frameheadersize_prefix((*dctx).format),
            (*dctx).format,
        );
        if ZSTD_isError(frame_header_size) != 0 {
            return frame_header_size;
        }
        if remaining_src_size < frame_header_size + ZSTD_blockHeaderSize {
            return err_code(ZSTD_error_srcSize_wrong);
        }
        let e = ZSTD_decodeFrameHeader(dctx, ip as *const c_void, frame_header_size);
        if err_is_error(e) {
            return e;
        }
        ip = ip.add(frame_header_size);
        remaining_src_size -= frame_header_size;
    }

    /* Shrink the blockSizeMax if enabled */
    if (*dctx).maxBlockSizeParam != 0 {
        (*dctx).fParams.blockSizeMax =
            min_u32((*dctx).fParams.blockSizeMax, (*dctx).maxBlockSizeParam as c_uint);
    }

    /* Loop on each block */
    loop {
        let mut o_block_end = oend;
        let decoded_size: usize;
        let mut block_properties = blockProperties_t::default();
        let c_block_size = ZSTD_getcBlockSize(ip as *const c_void, remaining_src_size, &mut block_properties);
        if ZSTD_isError(c_block_size) != 0 {
            return c_block_size;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remaining_src_size -= ZSTD_blockHeaderSize;
        if c_block_size > remaining_src_size {
            return err_code(ZSTD_error_srcSize_wrong);
        }

        if ip >= op && ip < o_block_end {
            o_block_end = op.add(ip as usize - op as usize);
        }

        match block_properties.blockType {
            x if x == bt_compressed => {
                decoded_size = ZSTD_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    o_block_end as usize - op as usize,
                    ip as *const c_void,
                    c_block_size,
                    not_streaming,
                );
            }
            x if x == bt_raw => {
                decoded_size = ZSTD_copyRawBlock(
                    op as *mut c_void,
                    oend as usize - op as usize,
                    ip as *const c_void,
                    c_block_size,
                );
            }
            x if x == bt_rle => {
                decoded_size = ZSTD_setRleBlock(
                    op as *mut c_void,
                    o_block_end as usize - op as usize,
                    *ip,
                    block_properties.origSize as usize,
                );
            }
            _ => {
                return err_code(ZSTD_error_corruption_detected);
            }
        }
        if err_is_error(decoded_size) {
            return decoded_size;
        }
        if (*dctx).validateChecksum != 0 {
            ZSTD_XXH64_update(&mut (*dctx).xxhState, op as *const c_void, decoded_size);
        }
        if decoded_size != 0 {
            op = op.add(decoded_size);
        }
        ip = ip.add(c_block_size);
        remaining_src_size -= c_block_size;
        if block_properties.lastBlock != 0 {
            break;
        }
    }

    if (*dctx).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
        if (op as usize - ostart as usize) as U64 != (*dctx).fParams.frameContentSize {
            return err_code(ZSTD_error_corruption_detected);
        }
    }
    if (*dctx).fParams.checksumFlag != 0 {
        if remaining_src_size < 4 {
            return err_code(ZSTD_error_checksum_wrong);
        }
        if (*dctx).forceIgnoreChecksum == 0 {
            let check_calc: U32 = ZSTD_XXH64_digest(&(*dctx).xxhState) as U32;
            let check_read: U32 = mem_read_le32(ip);
            if check_read != check_calc {
                return err_code(ZSTD_error_checksum_wrong);
            }
        }
        ip = ip.add(4);
        remaining_src_size -= 4;
    }
    ZSTD_DCtx_trace_end(
        dctx,
        (op as usize - ostart as usize) as U64,
        (ip as usize - istart as usize) as U64,
        0,
    );
    *src_ptr = ip as *const c_void;
    *src_size_ptr = remaining_src_size;
    op as usize - ostart as usize
}

/// `ZSTD_decompressMultiFrame()` (static)
unsafe fn ZSTD_decompressMultiFrame(
    dctx: *mut ZSTD_DCtx,
    mut dst: *mut c_void,
    mut dst_capacity: usize,
    mut src: *const c_void,
    mut src_size: usize,
    mut dict: *const c_void,
    mut dict_size: usize,
    ddict: *const ZSTD_DDict,
) -> usize {
    let dststart = dst;
    let mut more_than_1_frame: c_int = 0;

    if !ddict.is_null() {
        dict = ZSTD_DDict_dictContent(ddict);
        dict_size = ZSTD_DDict_dictSize(ddict);
    }

    while src_size >= ZSTD_startingInputLength((*dctx).format) {
        // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.

        if (*dctx).format == ZSTD_f_zstd1 && src_size >= 4 {
            let magic_number: U32 = mem_read_le32(src as *const u8);
            if (magic_number & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
                let skippable_size = readSkippableFrameSize(src, src_size);
                if err_is_error(skippable_size) {
                    return skippable_size;
                }
                src = (src as *const BYTE).add(skippable_size) as *const c_void;
                src_size -= skippable_size;
                continue;
            }
        }

        if !ddict.is_null() {
            let e = ZSTD_decompressBegin_usingDDict(dctx, ddict);
            if err_is_error(e) {
                return e;
            }
        } else {
            let e = ZSTD_decompressBegin_usingDict(dctx, dict, dict_size);
            if err_is_error(e) {
                return e;
            }
        }
        ZSTD_checkContinuity(dctx, dst, dst_capacity);

        {
            let res = ZSTD_decompressFrame(dctx, dst, dst_capacity, &mut src, &mut src_size);
            if ZSTD_getErrorCode(res) == ZSTD_error_prefix_unknown && more_than_1_frame == 1 {
                return err_code(ZSTD_error_srcSize_wrong);
            }
            if ZSTD_isError(res) != 0 {
                return res;
            }
            if res != 0 {
                dst = (dst as *mut BYTE).add(res) as *mut c_void;
            }
            dst_capacity -= res;
        }
        more_than_1_frame = 1;
    }

    if src_size != 0 {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    dst as usize - dststart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress_usingDict(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
    dict: *const c_void,
    dict_size: usize,
) -> usize {
    ZSTD_decompressMultiFrame(
        dctx,
        dst,
        dst_capacity,
        src,
        src_size,
        dict,
        dict_size,
        core::ptr::null(),
    )
}

/// `ZSTD_getDDict()`
unsafe fn ZSTD_getDDict(dctx: *mut ZSTD_DCtx) -> *const ZSTD_DDict {
    match (*dctx).dictUses {
        ZSTD_use_indefinitely => (*dctx).ddict,
        ZSTD_use_once => {
            (*dctx).dictUses = ZSTD_dont_use;
            (*dctx).ddict
        }
        // default and ZSTD_dont_use
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
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    ZSTD_decompress_usingDDict(dctx, dst, dst_capacity, src, src_size, ZSTD_getDDict(dctx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress(
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    let regen_size: usize;
    let dctx = ZSTD_createDCtx_internal(ZSTD_defaultCMem);
    if dctx.is_null() {
        return err_code(ZSTD_error_memory_allocation);
    }
    regen_size = ZSTD_decompressDCtx(dctx, dst, dst_capacity, src, src_size);
    ZSTD_freeDCtx(dctx);
    regen_size
}

/*-**************************************
*   Advanced Streaming Decompression API
*   Bufferless and synchronous
****************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected
}

/// `ZSTD_nextSrcSizeToDecompressWithInputSize()`
unsafe fn ZSTD_nextSrcSizeToDecompressWithInputSize(dctx: *mut ZSTD_DCtx, input_size: usize) -> usize {
    if !((*dctx).stage == ZSTDds_decompressBlock || (*dctx).stage == ZSTDds_decompressLastBlock) {
        return (*dctx).expected;
    }
    if (*dctx).bType != bt_raw {
        return (*dctx).expected;
    }
    // BOUNDED(1, inputSize, dctx->expected)
    max_usize(1, min_usize(input_size, (*dctx).expected))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_nextInputType(dctx: *mut ZSTD_DCtx) -> ZSTD_nextInputType_e {
    match (*dctx).stage {
        ZSTDds_getFrameHeaderSize | ZSTDds_decodeFrameHeader => ZSTDnit_frameHeader,
        ZSTDds_decodeBlockHeader => ZSTDnit_blockHeader,
        ZSTDds_decompressBlock => ZSTDnit_block,
        ZSTDds_decompressLastBlock => ZSTDnit_lastBlock,
        ZSTDds_checkChecksum => ZSTDnit_checksum,
        ZSTDds_decodeSkippableHeader | ZSTDds_skipFrame => ZSTDnit_skippableFrame,
        _ => ZSTDnit_frameHeader,
    }
}

/// `ZSTD_isSkipFrame()`
unsafe fn ZSTD_isSkipFrame(dctx: *mut ZSTD_DCtx) -> c_int {
    ((*dctx).stage == ZSTDds_skipFrame) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressContinue(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    /* Sanity check */
    if src_size != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, src_size) {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    ZSTD_checkContinuity(dctx, dst, dst_capacity);

    (*dctx).processedCSize += src_size as U64;

    match (*dctx).stage {
        ZSTDds_getFrameHeaderSize => {
            if (*dctx).format == ZSTD_f_zstd1 {
                if (mem_read_le32(src as *const u8) & ZSTD_MAGIC_SKIPPABLE_MASK)
                    == ZSTD_MAGIC_SKIPPABLE_START
                {
                    core::ptr::copy_nonoverlapping(
                        src as *const u8,
                        (*dctx).headerBuffer.as_mut_ptr(),
                        src_size,
                    );
                    (*dctx).expected = ZSTD_SKIPPABLEHEADERSIZE - src_size;
                    (*dctx).stage = ZSTDds_decodeSkippableHeader;
                    return 0;
                }
            }
            (*dctx).headerSize = ZSTD_frameHeaderSize_internal(src, src_size, (*dctx).format);
            if ZSTD_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            core::ptr::copy_nonoverlapping(
                src as *const u8,
                (*dctx).headerBuffer.as_mut_ptr(),
                src_size,
            );
            (*dctx).expected = (*dctx).headerSize - src_size;
            (*dctx).stage = ZSTDds_decodeFrameHeader;
            0
        }

        ZSTDds_decodeFrameHeader => {
            core::ptr::copy_nonoverlapping(
                src as *const u8,
                (*dctx).headerBuffer.as_mut_ptr().add((*dctx).headerSize - src_size),
                src_size,
            );
            let e = ZSTD_decodeFrameHeader(
                dctx,
                (*dctx).headerBuffer.as_ptr() as *const c_void,
                (*dctx).headerSize,
            );
            if err_is_error(e) {
                return e;
            }
            (*dctx).expected = ZSTD_blockHeaderSize;
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            0
        }

        ZSTDds_decodeBlockHeader => {
            let mut bp = blockProperties_t::default();
            let c_block_size = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
            if ZSTD_isError(c_block_size) != 0 {
                return c_block_size;
            }
            if c_block_size > (*dctx).fParams.blockSizeMax as usize {
                return err_code(ZSTD_error_corruption_detected);
            }
            (*dctx).expected = c_block_size;
            (*dctx).bType = bp.blockType;
            (*dctx).rleSize = bp.origSize as usize;
            if c_block_size != 0 {
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
                    (*dctx).expected = 0;
                    (*dctx).stage = ZSTDds_getFrameHeaderSize;
                }
            } else {
                (*dctx).expected = ZSTD_blockHeaderSize;
                (*dctx).stage = ZSTDds_decodeBlockHeader;
            }
            0
        }

        ZSTDds_decompressLastBlock | ZSTDds_decompressBlock => {
            let r_size: usize;
            match (*dctx).bType {
                x if x == bt_compressed => {
                    r_size = ZSTD_decompressBlock_internal(
                        dctx, dst, dst_capacity, src, src_size, is_streaming,
                    );
                    (*dctx).expected = 0;
                }
                x if x == bt_raw => {
                    r_size = ZSTD_copyRawBlock(dst, dst_capacity, src, src_size);
                    if err_is_error(r_size) {
                        return r_size;
                    }
                    (*dctx).expected -= r_size;
                }
                x if x == bt_rle => {
                    r_size = ZSTD_setRleBlock(dst, dst_capacity, *(src as *const BYTE), (*dctx).rleSize);
                    (*dctx).expected = 0;
                }
                _ => {
                    return err_code(ZSTD_error_corruption_detected);
                }
            }
            if err_is_error(r_size) {
                return r_size;
            }
            if r_size > (*dctx).fParams.blockSizeMax as usize {
                return err_code(ZSTD_error_corruption_detected);
            }
            (*dctx).decodedSize += r_size as U64;
            if (*dctx).validateChecksum != 0 {
                ZSTD_XXH64_update(&mut (*dctx).xxhState, dst as *const c_void, r_size);
            }
            (*dctx).previousDstEnd = (dst as *mut core::ffi::c_char).add(r_size) as *const c_void;

            /* Stay on the same stage until we are finished streaming the block. */
            if (*dctx).expected > 0 {
                return r_size;
            }

            if (*dctx).stage == ZSTDds_decompressLastBlock {
                if (*dctx).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
                    && (*dctx).decodedSize != (*dctx).fParams.frameContentSize
                {
                    return err_code(ZSTD_error_corruption_detected);
                }
                if (*dctx).fParams.checksumFlag != 0 {
                    (*dctx).expected = 4;
                    (*dctx).stage = ZSTDds_checkChecksum;
                } else {
                    ZSTD_DCtx_trace_end(dctx, (*dctx).decodedSize, (*dctx).processedCSize, 1);
                    (*dctx).expected = 0;
                    (*dctx).stage = ZSTDds_getFrameHeaderSize;
                }
            } else {
                (*dctx).stage = ZSTDds_decodeBlockHeader;
                (*dctx).expected = ZSTD_blockHeaderSize;
            }
            r_size
        }

        ZSTDds_checkChecksum => {
            if (*dctx).validateChecksum != 0 {
                let h32: U32 = ZSTD_XXH64_digest(&(*dctx).xxhState) as U32;
                let check32: U32 = mem_read_le32(src as *const u8);
                if check32 != h32 {
                    return err_code(ZSTD_error_checksum_wrong);
                }
            }
            ZSTD_DCtx_trace_end(dctx, (*dctx).decodedSize, (*dctx).processedCSize, 1);
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            0
        }

        ZSTDds_decodeSkippableHeader => {
            core::ptr::copy_nonoverlapping(
                src as *const u8,
                (*dctx)
                    .headerBuffer
                    .as_mut_ptr()
                    .add(ZSTD_SKIPPABLEHEADERSIZE - src_size),
                src_size,
            );
            (*dctx).expected =
                mem_read_le32((*dctx).headerBuffer.as_ptr().add(ZSTD_FRAMEIDSIZE)) as usize;
            (*dctx).stage = ZSTDds_skipFrame;
            0
        }

        ZSTDds_skipFrame => {
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            0
        }

        _ => err_code(ZSTD_error_GENERIC),
    }
}

/// `ZSTD_refDictContent()`
unsafe fn ZSTD_refDictContent(dctx: *mut ZSTD_DCtx, dict: *const c_void, dict_size: usize) -> usize {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).virtualStart = (dict as *const core::ffi::c_char)
        .offset(-(((*dctx).previousDstEnd as *const core::ffi::c_char as isize)
            - ((*dctx).prefixStart as *const core::ffi::c_char as isize))) as *const c_void;
    (*dctx).prefixStart = dict;
    (*dctx).previousDstEnd = (dict as *const core::ffi::c_char).add(dict_size) as *const c_void;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_loadDEntropy(
    entropy: *mut ZSTD_entropyDTables_t,
    dict: *const c_void,
    dict_size: usize,
) -> usize {
    let mut dict_ptr = dict as *const BYTE;
    let dict_end = dict_ptr.add(dict_size);

    if dict_size <= 8 {
        return err_code(ZSTD_error_dictionary_corrupted);
    }
    dict_ptr = dict_ptr.add(8); /* skip header = magic + dictID */

    {
        let workspace = core::ptr::addr_of_mut!((*entropy).LLTable) as *mut c_void;
        let workspace_size = core::mem::size_of_val(&(*entropy).LLTable)
            + core::mem::size_of_val(&(*entropy).OFTable)
            + core::mem::size_of_val(&(*entropy).MLTable);
        let h_size = HUF_readDTableX2_wksp(
            (*entropy).hufTable.as_mut_ptr(),
            dict_ptr as *const c_void,
            dict_end as usize - dict_ptr as usize,
            workspace,
            workspace_size,
            0,
        );
        if HUF_isError(h_size) != 0 {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        dict_ptr = dict_ptr.add(h_size);
    }

    {
        let mut offcode_n_count: [i16; (MaxOff + 1) as usize] = [0; (MaxOff + 1) as usize];
        let mut offcode_max_value: c_uint = MaxOff;
        let mut offcode_log: c_uint = 0;
        let offcode_header_size = FSE_readNCount(
            offcode_n_count.as_mut_ptr(),
            &mut offcode_max_value,
            &mut offcode_log,
            dict_ptr as *const c_void,
            dict_end as usize - dict_ptr as usize,
        );
        if FSE_isError(offcode_header_size) != 0 {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        if offcode_max_value > MaxOff {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        if offcode_log > OffFSELog {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        ZSTD_buildFSETable(
            (*entropy).OFTable.as_mut_ptr(),
            offcode_n_count.as_ptr(),
            offcode_max_value,
            OF_base.as_ptr(),
            OF_bits.as_ptr(),
            offcode_log,
            (*entropy).workspace.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*entropy).workspace),
            0,
        );
        dict_ptr = dict_ptr.add(offcode_header_size);
    }

    {
        let mut matchlength_n_count: [i16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut matchlength_max_value: c_uint = MaxML;
        let mut matchlength_log: c_uint = 0;
        let matchlength_header_size = FSE_readNCount(
            matchlength_n_count.as_mut_ptr(),
            &mut matchlength_max_value,
            &mut matchlength_log,
            dict_ptr as *const c_void,
            dict_end as usize - dict_ptr as usize,
        );
        if FSE_isError(matchlength_header_size) != 0 {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        if matchlength_max_value > MaxML {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        if matchlength_log > MLFSELog {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        ZSTD_buildFSETable(
            (*entropy).MLTable.as_mut_ptr(),
            matchlength_n_count.as_ptr(),
            matchlength_max_value,
            ML_base.as_ptr(),
            ML_bits.as_ptr(),
            matchlength_log,
            (*entropy).workspace.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*entropy).workspace),
            0,
        );
        dict_ptr = dict_ptr.add(matchlength_header_size);
    }

    {
        let mut litlength_n_count: [i16; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
        let mut litlength_max_value: c_uint = MaxLL;
        let mut litlength_log: c_uint = 0;
        let litlength_header_size = FSE_readNCount(
            litlength_n_count.as_mut_ptr(),
            &mut litlength_max_value,
            &mut litlength_log,
            dict_ptr as *const c_void,
            dict_end as usize - dict_ptr as usize,
        );
        if FSE_isError(litlength_header_size) != 0 {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        if litlength_max_value > MaxLL {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        if litlength_log > LLFSELog {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        ZSTD_buildFSETable(
            (*entropy).LLTable.as_mut_ptr(),
            litlength_n_count.as_ptr(),
            litlength_max_value,
            LL_base.as_ptr(),
            LL_bits.as_ptr(),
            litlength_log,
            (*entropy).workspace.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*entropy).workspace),
            0,
        );
        dict_ptr = dict_ptr.add(litlength_header_size);
    }

    if dict_ptr.add(12) > dict_end {
        return err_code(ZSTD_error_dictionary_corrupted);
    }
    {
        let dict_content_size = dict_end as usize - dict_ptr.add(12) as usize;
        for i in 0..3usize {
            let rep: U32 = mem_read_le32(dict_ptr);
            dict_ptr = dict_ptr.add(4);
            if rep == 0 || rep as usize > dict_content_size {
                return err_code(ZSTD_error_dictionary_corrupted);
            }
            (*entropy).rep[i] = rep;
        }
    }

    dict_ptr as usize - dict as usize
}

/// `ZSTD_decompress_insertDictionary()`
unsafe fn ZSTD_decompress_insertDictionary(
    dctx: *mut ZSTD_DCtx,
    mut dict: *const c_void,
    mut dict_size: usize,
) -> usize {
    if dict_size < 8 {
        return ZSTD_refDictContent(dctx, dict, dict_size);
    }
    {
        let magic: U32 = mem_read_le32(dict as *const u8);
        if magic != ZSTD_MAGIC_DICTIONARY {
            return ZSTD_refDictContent(dctx, dict, dict_size);
        }
    }
    (*dctx).dictID = mem_read_le32((dict as *const core::ffi::c_char).add(ZSTD_FRAMEIDSIZE) as *const u8);

    /* load entropy tables */
    {
        let e_size = ZSTD_loadDEntropy(&mut (*dctx).entropy, dict, dict_size);
        if ZSTD_isError(e_size) != 0 {
            return err_code(ZSTD_error_dictionary_corrupted);
        }
        dict = (dict as *const core::ffi::c_char).add(e_size) as *const c_void;
        dict_size -= e_size;
    }
    (*dctx).litEntropy = 1;
    (*dctx).fseEntropy = 1;

    ZSTD_refDictContent(dctx, dict, dict_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin(dctx: *mut ZSTD_DCtx) -> usize {
    // ZSTD_TRACE==1, but ZSTD_trace_decompress_begin is a NULL weak symbol → returns 0.
    (*dctx).traceCtx = 0;
    (*dctx).expected = ZSTD_startingInputLength((*dctx).format);
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).processedCSize = 0;
    (*dctx).decodedSize = 0;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).prefixStart = core::ptr::null();
    (*dctx).virtualStart = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).entropy.hufTable[0] =
        (ZSTD_HUFFDTABLE_CAPACITY_LOG.wrapping_mul(0x1000001)) as HUF_DTable;
    (*dctx).litEntropy = 0;
    (*dctx).fseEntropy = 0;
    (*dctx).dictID = 0;
    (*dctx).bType = bt_reserved;
    (*dctx).isFrameDecompression = 1;
    core::ptr::copy_nonoverlapping(
        repStartValue.as_ptr(),
        (*dctx).entropy.rep.as_mut_ptr(),
        ZSTD_REP_NUM,
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
    dict_size: usize,
) -> usize {
    let e = ZSTD_decompressBegin(dctx);
    if err_is_error(e) {
        return e;
    }
    if !dict.is_null() && dict_size != 0 {
        if ZSTD_isError(ZSTD_decompress_insertDictionary(dctx, dict, dict_size)) != 0 {
            return err_code(ZSTD_error_dictionary_corrupted);
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
        let dict_start = ZSTD_DDict_dictContent(ddict) as *const core::ffi::c_char;
        let dict_size = ZSTD_DDict_dictSize(ddict);
        let dict_end = dict_start.add(dict_size) as *const c_void;
        (*dctx).ddictIsCold = ((*dctx).dictEnd != dict_end) as c_int;
    }
    let e = ZSTD_decompressBegin(dctx);
    if err_is_error(e) {
        return e;
    }
    if !ddict.is_null() {
        ZSTD_copyDDictParameters(dctx, ddict);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDict(dict: *const c_void, dict_size: usize) -> c_uint {
    if dict_size < 8 {
        return 0;
    }
    if mem_read_le32(dict as *const u8) != ZSTD_MAGIC_DICTIONARY {
        return 0;
    }
    mem_read_le32((dict as *const core::ffi::c_char).add(ZSTD_FRAMEIDSIZE) as *const u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromFrame(src: *const c_void, src_size: usize) -> c_uint {
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
    let h_error = ZSTD_getFrameHeader(&mut zfp, src, src_size);
    if ZSTD_isError(h_error) != 0 {
        return 0;
    }
    zfp.dictID
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress_usingDDict(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
    ddict: *const ZSTD_DDict,
) -> usize {
    ZSTD_decompressMultiFrame(
        dctx,
        dst,
        dst_capacity,
        src,
        src_size,
        core::ptr::null(),
        0,
        ddict,
    )
}

/*=====================================
*   Streaming decompression
*====================================*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDStream() -> *mut ZSTD_DCtx {
    ZSTD_createDCtx_internal(ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDStream(
    workspace: *mut c_void,
    workspace_size: usize,
) -> *mut ZSTD_DCtx {
    ZSTD_initStaticDCtx(workspace, workspace_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDStream_advanced(custom_mem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    ZSTD_createDCtx_internal(custom_mem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDStream(zds: *mut ZSTD_DCtx) -> usize {
    ZSTD_freeDCtx(zds)
}

/* ***  Initialization  *** */

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_DStreamInSize() -> usize {
    ZSTD_BLOCKSIZE_MAX + ZSTD_blockHeaderSize
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_DStreamOutSize() -> usize {
    ZSTD_BLOCKSIZE_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary_advanced(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dict_size: usize,
    dict_load_method: ZSTD_dictLoadMethod_e,
    dict_content_type: ZSTD_dictContentType_e,
) -> usize {
    if (*dctx).streamStage != zdss_init {
        return err_code(ZSTD_error_stage_wrong);
    }
    ZSTD_clearDict(dctx);
    if !dict.is_null() && dict_size != 0 {
        (*dctx).ddictLocal = ZSTD_createDDict_advanced(
            dict,
            dict_size,
            dict_load_method,
            dict_content_type,
            (*dctx).customMem,
        );
        if (*dctx).ddictLocal.is_null() {
            return err_code(ZSTD_error_memory_allocation);
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
    dict_size: usize,
) -> usize {
    ZSTD_DCtx_loadDictionary_advanced(dctx, dict, dict_size, ZSTD_dlm_byRef, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dict_size: usize,
) -> usize {
    ZSTD_DCtx_loadDictionary_advanced(dctx, dict, dict_size, ZSTD_dlm_byCopy, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refPrefix_advanced(
    dctx: *mut ZSTD_DCtx,
    prefix: *const c_void,
    prefix_size: usize,
    dict_content_type: ZSTD_dictContentType_e,
) -> usize {
    let e = ZSTD_DCtx_loadDictionary_advanced(
        dctx,
        prefix,
        prefix_size,
        ZSTD_dlm_byRef,
        dict_content_type,
    );
    if err_is_error(e) {
        return e;
    }
    (*dctx).dictUses = ZSTD_use_once;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refPrefix(
    dctx: *mut ZSTD_DCtx,
    prefix: *const c_void,
    prefix_size: usize,
) -> usize {
    ZSTD_DCtx_refPrefix_advanced(dctx, prefix, prefix_size, ZSTD_dct_rawContent)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDict(
    zds: *mut ZSTD_DCtx,
    dict: *const c_void,
    dict_size: usize,
) -> usize {
    let e = ZSTD_DCtx_reset(zds, ZSTD_reset_session_only);
    if err_is_error(e) {
        return e;
    }
    let e = ZSTD_DCtx_loadDictionary(zds, dict, dict_size);
    if err_is_error(e) {
        return e;
    }
    ZSTD_startingInputLength((*zds).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream(zds: *mut ZSTD_DCtx) -> usize {
    let e = ZSTD_DCtx_reset(zds, ZSTD_reset_session_only);
    if err_is_error(e) {
        return e;
    }
    let e = ZSTD_DCtx_refDDict(zds, core::ptr::null());
    if err_is_error(e) {
        return e;
    }
    ZSTD_startingInputLength((*zds).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDDict(
    dctx: *mut ZSTD_DCtx,
    ddict: *const ZSTD_DDict,
) -> usize {
    let e = ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only);
    if err_is_error(e) {
        return e;
    }
    let e = ZSTD_DCtx_refDDict(dctx, ddict);
    if err_is_error(e) {
        return e;
    }
    ZSTD_startingInputLength((*dctx).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetDStream(dctx: *mut ZSTD_DCtx) -> usize {
    let e = ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only);
    if err_is_error(e) {
        return e;
    }
    ZSTD_startingInputLength((*dctx).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refDDict(
    dctx: *mut ZSTD_DCtx,
    ddict: *const ZSTD_DDict,
) -> usize {
    if (*dctx).streamStage != zdss_init {
        return err_code(ZSTD_error_stage_wrong);
    }
    ZSTD_clearDict(dctx);
    if !ddict.is_null() {
        (*dctx).ddict = ddict;
        (*dctx).dictUses = ZSTD_use_indefinitely;
        if (*dctx).refMultipleDDicts == ZSTD_rmd_refMultipleDDicts {
            if (*dctx).ddictSet.is_null() {
                (*dctx).ddictSet = ZSTD_createDDictHashSet((*dctx).customMem);
                if (*dctx).ddictSet.is_null() {
                    return err_code(ZSTD_error_memory_allocation);
                }
            }
            let e = ZSTD_DDictHashSet_addDDict((*dctx).ddictSet, ddict, (*dctx).customMem);
            if err_is_error(e) {
                return e;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setMaxWindowSize(dctx: *mut ZSTD_DCtx, max_window_size: usize) -> usize {
    let bounds = ZSTD_dParam_getBounds(ZSTD_d_windowLogMax);
    let min = 1usize << bounds.lowerBound;
    let max = 1usize << bounds.upperBound;
    if (*dctx).streamStage != zdss_init {
        return err_code(ZSTD_error_stage_wrong);
    }
    if max_window_size < min {
        return err_code(ZSTD_error_parameter_outOfBound);
    }
    if max_window_size > max {
        return err_code(ZSTD_error_parameter_outOfBound);
    }
    (*dctx).maxWindowSize = max_window_size;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setFormat(dctx: *mut ZSTD_DCtx, format: ZSTD_format_e) -> usize {
    ZSTD_DCtx_setParameter(dctx, ZSTD_d_format, format as c_int)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_dParam_getBounds(d_param: ZSTD_dParameter) -> ZSTD_bounds {
    let mut bounds = ZSTD_bounds {
        error: 0,
        lowerBound: 0,
        upperBound: 0,
    };
    match d_param {
        ZSTD_d_windowLogMax => {
            bounds.lowerBound = ZSTD_WINDOWLOG_ABSOLUTEMIN as c_int;
            bounds.upperBound = ZSTD_WINDOWLOG_MAX as c_int;
            return bounds;
        }
        ZSTD_d_experimentalParam1 => {
            // ZSTD_d_format
            bounds.lowerBound = ZSTD_f_zstd1 as c_int;
            bounds.upperBound = ZSTD_f_zstd1_magicless as c_int;
            return bounds;
        }
        ZSTD_d_experimentalParam2 => {
            // ZSTD_d_stableOutBuffer
            bounds.lowerBound = ZSTD_bm_buffered as c_int;
            bounds.upperBound = ZSTD_bm_stable as c_int;
            return bounds;
        }
        ZSTD_d_experimentalParam3 => {
            // ZSTD_d_forceIgnoreChecksum
            bounds.lowerBound = ZSTD_d_validateChecksum as c_int;
            bounds.upperBound = ZSTD_d_ignoreChecksum as c_int;
            return bounds;
        }
        ZSTD_d_experimentalParam4 => {
            // ZSTD_d_refMultipleDDicts
            bounds.lowerBound = ZSTD_rmd_refSingleDDict as c_int;
            bounds.upperBound = ZSTD_rmd_refMultipleDDicts as c_int;
            return bounds;
        }
        ZSTD_d_experimentalParam5 => {
            // ZSTD_d_disableHuffmanAssembly
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }
        ZSTD_d_experimentalParam6 => {
            // ZSTD_d_maxBlockSize
            bounds.lowerBound = ZSTD_BLOCKSIZE_MAX_MIN;
            bounds.upperBound = ZSTD_BLOCKSIZE_MAX as c_int;
            return bounds;
        }
        _ => {}
    }
    bounds.error = err_code(ZSTD_error_parameter_unsupported);
    bounds
}

/// `ZSTD_dParam_withinBounds()`
fn ZSTD_dParam_withinBounds(d_param: ZSTD_dParameter, value: c_int) -> c_int {
    let bounds = ZSTD_dParam_getBounds(d_param);
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
            *value = zstd_highbit32((*dctx).maxWindowSize as U32) as c_int;
            return 0;
        }
        ZSTD_d_experimentalParam1 => {
            *value = (*dctx).format as c_int;
            return 0;
        }
        ZSTD_d_experimentalParam2 => {
            *value = (*dctx).outBufferMode as c_int;
            return 0;
        }
        ZSTD_d_experimentalParam3 => {
            *value = (*dctx).forceIgnoreChecksum as c_int;
            return 0;
        }
        ZSTD_d_experimentalParam4 => {
            *value = (*dctx).refMultipleDDicts as c_int;
            return 0;
        }
        ZSTD_d_experimentalParam5 => {
            *value = (*dctx).disableHufAsm;
            return 0;
        }
        ZSTD_d_experimentalParam6 => {
            *value = (*dctx).maxBlockSizeParam;
            return 0;
        }
        _ => {}
    }
    err_code(ZSTD_error_parameter_unsupported)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setParameter(
    dctx: *mut ZSTD_DCtx,
    d_param: ZSTD_dParameter,
    mut value: c_int,
) -> usize {
    if (*dctx).streamStage != zdss_init {
        return err_code(ZSTD_error_stage_wrong);
    }
    match d_param {
        ZSTD_d_windowLogMax => {
            if value == 0 {
                value = ZSTD_WINDOWLOG_LIMIT_DEFAULT;
            }
            if ZSTD_dParam_withinBounds(ZSTD_d_windowLogMax, value) == 0 {
                return err_code(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).maxWindowSize = 1usize << value;
            return 0;
        }
        ZSTD_d_experimentalParam1 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_format, value) == 0 {
                return err_code(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).format = value as ZSTD_format_e;
            return 0;
        }
        ZSTD_d_experimentalParam2 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_stableOutBuffer, value) == 0 {
                return err_code(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).outBufferMode = value as c_uint;
            return 0;
        }
        ZSTD_d_experimentalParam3 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_forceIgnoreChecksum, value) == 0 {
                return err_code(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).forceIgnoreChecksum = value as ZSTD_forceIgnoreChecksum_e;
            return 0;
        }
        ZSTD_d_experimentalParam4 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_refMultipleDDicts, value) == 0 {
                return err_code(ZSTD_error_parameter_outOfBound);
            }
            if (*dctx).staticSize != 0 {
                return err_code(ZSTD_error_parameter_unsupported);
            }
            (*dctx).refMultipleDDicts = value as ZSTD_refMultipleDDicts_e;
            return 0;
        }
        ZSTD_d_experimentalParam5 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_disableHuffmanAssembly, value) == 0 {
                return err_code(ZSTD_error_parameter_outOfBound);
            }
            (*dctx).disableHufAsm = (value != 0) as c_int;
            return 0;
        }
        ZSTD_d_experimentalParam6 => {
            if value != 0 {
                if ZSTD_dParam_withinBounds(ZSTD_d_maxBlockSize, value) == 0 {
                    return err_code(ZSTD_error_parameter_outOfBound);
                }
            }
            (*dctx).maxBlockSizeParam = value;
            return 0;
        }
        _ => {}
    }
    err_code(ZSTD_error_parameter_unsupported)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_reset(dctx: *mut ZSTD_DCtx, reset: ZSTD_ResetDirective) -> usize {
    if reset == ZSTD_reset_session_only || reset == ZSTD_reset_session_and_parameters {
        (*dctx).streamStage = zdss_init;
        (*dctx).noForwardProgress = 0;
        (*dctx).isFrameDecompression = 1;
    }
    if reset == ZSTD_reset_parameters || reset == ZSTD_reset_session_and_parameters {
        if (*dctx).streamStage != zdss_init {
            return err_code(ZSTD_error_stage_wrong);
        }
        ZSTD_clearDict(dctx);
        ZSTD_DCtx_resetParameters(dctx);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DStream(dctx: *const ZSTD_DCtx) -> usize {
    ZSTD_sizeof_DCtx(dctx)
}

/// `ZSTD_decodingBufferSize_internal()`
fn ZSTD_decodingBufferSize_internal(
    window_size: core::ffi::c_ulonglong,
    frame_content_size: core::ffi::c_ulonglong,
    block_size_max: usize,
) -> usize {
    let block_size = min_usize(
        min_usize(window_size as usize, ZSTD_BLOCKSIZE_MAX),
        block_size_max,
    );
    let needed_rb_size: core::ffi::c_ulonglong =
        window_size + (block_size as u64 * 2) + (WILDCOPY_OVERLENGTH as u64 * 2);
    let needed_size: core::ffi::c_ulonglong = min_u64(frame_content_size, needed_rb_size);
    let min_rb_size = needed_size as usize;
    if min_rb_size as core::ffi::c_ulonglong != needed_size {
        return err_code(ZSTD_error_frameParameter_windowTooLarge);
    }
    min_rb_size
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_decodingBufferSize_min(
    window_size: core::ffi::c_ulonglong,
    frame_content_size: core::ffi::c_ulonglong,
) -> usize {
    ZSTD_decodingBufferSize_internal(window_size, frame_content_size, ZSTD_BLOCKSIZE_MAX)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_estimateDStreamSize(window_size: usize) -> usize {
    let block_size = min_usize(window_size, ZSTD_BLOCKSIZE_MAX);
    let in_buff_size = block_size;
    let out_buff_size =
        ZSTD_decodingBufferSize_min(window_size as u64, ZSTD_CONTENTSIZE_UNKNOWN);
    ZSTD_estimateDCtxSize() + in_buff_size + out_buff_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize_fromFrame(src: *const c_void, src_size: usize) -> usize {
    let window_size_max: U32 = 1u32 << ZSTD_WINDOWLOG_MAX;
    let mut zfh = ZSTD_FrameHeader::default();
    let err = ZSTD_getFrameHeader(&mut zfh, src, src_size);
    if ZSTD_isError(err) != 0 {
        return err;
    }
    if err > 0 {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    if zfh.windowSize > window_size_max as u64 {
        return err_code(ZSTD_error_frameParameter_windowTooLarge);
    }
    ZSTD_estimateDStreamSize(zfh.windowSize as usize)
}

/* *****   Decompression   ***** */

/// `ZSTD_DCtx_isOverflow()`
unsafe fn ZSTD_DCtx_isOverflow(
    zds: *mut ZSTD_DCtx,
    needed_in_buff_size: usize,
    needed_out_buff_size: usize,
) -> c_int {
    (((*zds).inBuffSize + (*zds).outBuffSize)
        >= (needed_in_buff_size + needed_out_buff_size) * ZSTD_WORKSPACETOOLARGE_FACTOR as usize)
        as c_int
}

/// `ZSTD_DCtx_updateOversizedDuration()`
unsafe fn ZSTD_DCtx_updateOversizedDuration(
    zds: *mut ZSTD_DCtx,
    needed_in_buff_size: usize,
    needed_out_buff_size: usize,
) {
    if ZSTD_DCtx_isOverflow(zds, needed_in_buff_size, needed_out_buff_size) != 0 {
        (*zds).oversizedDuration += 1;
    } else {
        (*zds).oversizedDuration = 0;
    }
}

/// `ZSTD_DCtx_isOversizedTooLong()`
unsafe fn ZSTD_DCtx_isOversizedTooLong(zds: *mut ZSTD_DCtx) -> c_int {
    ((*zds).oversizedDuration >= ZSTD_WORKSPACETOOLARGE_MAXDURATION as usize) as c_int
}

/// `ZSTD_checkOutBuffer()`
unsafe fn ZSTD_checkOutBuffer(zds: *const ZSTD_DCtx, output: *const ZSTD_outBuffer) -> usize {
    let expect = (*zds).expectedOutBuffer;
    if (*zds).outBufferMode != ZSTD_bm_stable {
        return 0;
    }
    if (*zds).streamStage == zdss_init {
        return 0;
    }
    if expect.dst == (*output).dst && expect.pos == (*output).pos && expect.size == (*output).size {
        return 0;
    }
    err_code(ZSTD_error_dstBuffer_wrong)
}

/// `ZSTD_decompressContinueStream()`
unsafe fn ZSTD_decompressContinueStream(
    zds: *mut ZSTD_DCtx,
    op: *mut *mut core::ffi::c_char,
    oend: *mut core::ffi::c_char,
    src: *const c_void,
    src_size: usize,
) -> usize {
    let is_skip_frame = ZSTD_isSkipFrame(zds);
    if (*zds).outBufferMode == ZSTD_bm_buffered {
        let dst_size = if is_skip_frame != 0 {
            0
        } else {
            (*zds).outBuffSize - (*zds).outStart
        };
        let decoded_size = ZSTD_decompressContinue(
            zds,
            (*zds).outBuff.add((*zds).outStart) as *mut c_void,
            dst_size,
            src,
            src_size,
        );
        if err_is_error(decoded_size) {
            return decoded_size;
        }
        if decoded_size == 0 && is_skip_frame == 0 {
            (*zds).streamStage = zdss_read;
        } else {
            (*zds).outEnd = (*zds).outStart + decoded_size;
            (*zds).streamStage = zdss_flush;
        }
    } else {
        let dst_size = if is_skip_frame != 0 {
            0
        } else {
            oend as usize - *op as usize
        };
        let decoded_size = ZSTD_decompressContinue(zds, *op as *mut c_void, dst_size, src, src_size);
        if err_is_error(decoded_size) {
            return decoded_size;
        }
        *op = (*op).add(decoded_size);
        (*zds).streamStage = zdss_read;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream(
    zds: *mut ZSTD_DCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    let src = (*input).src as *const core::ffi::c_char;
    let istart = if (*input).pos != 0 {
        src.add((*input).pos)
    } else {
        src
    };
    let iend = if (*input).size != 0 {
        src.add((*input).size)
    } else {
        src
    };
    let mut ip = istart;
    let dst = (*output).dst as *mut core::ffi::c_char;
    let ostart = if (*output).pos != 0 {
        dst.add((*output).pos)
    } else {
        dst
    };
    let oend = if (*output).size != 0 {
        dst.add((*output).size)
    } else {
        dst
    };
    let mut op = ostart;
    let mut some_more_work: U32 = 1;

    if (*input).pos > (*input).size {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    if (*output).pos > (*output).size {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    {
        let e = ZSTD_checkOutBuffer(zds, output);
        if err_is_error(e) {
            return e;
        }
    }

    while some_more_work != 0 {
        match (*zds).streamStage {
            zdss_init => {
                (*zds).streamStage = zdss_loadHeader;
                (*zds).lhSize = 0;
                (*zds).inPos = 0;
                (*zds).outStart = 0;
                (*zds).outEnd = 0;
                // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.
                (*zds).hostageByte = 0;
                (*zds).expectedOutBuffer = *output;
                // fallthrough to zdss_loadHeader
                let r = zdss_load_header(
                    zds, output, input, &mut ip, iend, istart, &mut op, oend, ostart,
                    &mut some_more_work,
                );
                if let Some(ret) = r {
                    return ret;
                }
            }
            zdss_loadHeader => {
                let r = zdss_load_header(
                    zds, output, input, &mut ip, iend, istart, &mut op, oend, ostart,
                    &mut some_more_work,
                );
                if let Some(ret) = r {
                    return ret;
                }
            }
            zdss_read => {
                let r = zdss_read_stage(zds, &mut ip, iend, &mut op, oend, &mut some_more_work);
                if let Some(ret) = r {
                    return ret;
                }
            }
            zdss_load => {
                let r = zdss_load_stage(zds, &mut ip, iend, &mut op, oend, &mut some_more_work);
                if let Some(ret) = r {
                    return ret;
                }
            }
            zdss_flush => {
                let to_flush_size = (*zds).outEnd - (*zds).outStart;
                let flushed_size = zstd_limit_copy(
                    op as *mut u8,
                    oend as usize - op as usize,
                    (*zds).outBuff.add((*zds).outStart) as *const u8,
                    to_flush_size,
                );
                op = op.add(flushed_size);
                (*zds).outStart += flushed_size;
                if flushed_size == to_flush_size {
                    (*zds).streamStage = zdss_read;
                    if ((*zds).outBuffSize as u64) < (*zds).fParams.frameContentSize
                        && (*zds).outStart + (*zds).fParams.blockSizeMax as usize > (*zds).outBuffSize
                    {
                        (*zds).outStart = 0;
                        (*zds).outEnd = 0;
                    }
                    // break
                } else {
                    some_more_work = 0;
                }
            }
            _ => {
                return err_code(ZSTD_error_GENERIC);
            }
        }
    }

    /* result */
    (*input).pos = ip as usize - (*input).src as *const core::ffi::c_char as usize;
    (*output).pos = op as usize - (*output).dst as *mut core::ffi::c_char as usize;

    (*zds).expectedOutBuffer = *output;

    if ip == istart && op == ostart {
        (*zds).noForwardProgress += 1;
        if (*zds).noForwardProgress >= ZSTD_NO_FORWARD_PROGRESS_MAX {
            if op == oend {
                return err_code(ZSTD_error_noForwardProgress_destFull);
            }
            if ip == iend {
                return err_code(ZSTD_error_noForwardProgress_inputEmpty);
            }
        }
    } else {
        (*zds).noForwardProgress = 0;
    }
    {
        let mut next_src_size_hint = ZSTD_nextSrcSizeToDecompress(zds);
        if next_src_size_hint == 0 {
            if (*zds).outEnd == (*zds).outStart {
                if (*zds).hostageByte != 0 {
                    if (*input).pos >= (*input).size {
                        (*zds).streamStage = zdss_read;
                        return 1;
                    }
                    (*input).pos += 1;
                }
                return 0;
            }
            if (*zds).hostageByte == 0 {
                (*input).pos -= 1;
                (*zds).hostageByte = 1;
            }
            return 1;
        }
        next_src_size_hint += ZSTD_blockHeaderSize
            * (ZSTD_nextInputType(zds) == ZSTDnit_block) as usize;
        next_src_size_hint -= (*zds).inPos;
        next_src_size_hint
    }
}

/// zdss_loadHeader body. Returns Some(ret) to return from ZSTD_decompressStream,
/// or None to continue the loop (the C `break`).
#[allow(clippy::too_many_arguments)]
unsafe fn zdss_load_header(
    zds: *mut ZSTD_DCtx,
    // Only the (untranslated) legacy branch of the C `zdss_loadHeader` case
    // reads `output`, so it is unused here.
    _output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    ip: &mut *const core::ffi::c_char,
    iend: *const core::ffi::c_char,
    istart: *const core::ffi::c_char,
    op: &mut *mut core::ffi::c_char,
    oend: *mut core::ffi::c_char,
    _ostart: *mut core::ffi::c_char,
    some_more_work: &mut U32,
) -> Option<usize> {
    // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.
    {
        let h_size = ZSTD_getFrameHeader_advanced(
            &mut (*zds).fParams,
            (*zds).headerBuffer.as_ptr() as *const c_void,
            (*zds).lhSize,
            (*zds).format,
        );
        if (*zds).refMultipleDDicts != 0 && !(*zds).ddictSet.is_null() {
            ZSTD_DCtx_selectFrameDDict(zds);
        }
        if ZSTD_isError(h_size) != 0 {
            // NOTE: legacy (v0.1-v0.7) frame support is not translated; the C build has ZSTD_LEGACY_SUPPORT=5.
            return Some(h_size);
        }
        if h_size != 0 {
            let to_load = h_size - (*zds).lhSize;
            let remaining_input = iend as usize - *ip as usize;
            if to_load > remaining_input {
                if remaining_input > 0 {
                    core::ptr::copy_nonoverlapping(
                        *ip as *const u8,
                        (*zds).headerBuffer.as_mut_ptr().add((*zds).lhSize),
                        remaining_input,
                    );
                    (*zds).lhSize += remaining_input;
                }
                (*input).pos = (*input).size;
                let e = ZSTD_getFrameHeader_advanced(
                    &mut (*zds).fParams,
                    (*zds).headerBuffer.as_ptr() as *const c_void,
                    (*zds).lhSize,
                    (*zds).format,
                );
                if err_is_error(e) {
                    return Some(e);
                }
                return Some(
                    (max_usize(zstd_frameheadersize_min((*zds).format), h_size) - (*zds).lhSize)
                        + ZSTD_blockHeaderSize,
                );
            }
            core::ptr::copy_nonoverlapping(
                *ip as *const u8,
                (*zds).headerBuffer.as_mut_ptr().add((*zds).lhSize),
                to_load,
            );
            (*zds).lhSize = h_size;
            *ip = (*ip).add(to_load);
            return None; // break
        }
    }

    /* check for single-pass mode opportunity */
    if (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
        && (*zds).fParams.frameType != ZSTD_skippableFrame
        && (oend as usize - *op as usize) as u64 >= (*zds).fParams.frameContentSize
    {
        let c_size = ZSTD_findFrameCompressedSize_advanced(
            istart as *const c_void,
            iend as usize - istart as usize,
            (*zds).format,
        );
        if c_size <= (iend as usize - istart as usize) {
            let decompressed_size = ZSTD_decompress_usingDDict(
                zds,
                *op as *mut c_void,
                oend as usize - *op as usize,
                istart as *const c_void,
                c_size,
                ZSTD_getDDict(zds),
            );
            if ZSTD_isError(decompressed_size) != 0 {
                return Some(decompressed_size);
            }
            *ip = istart.add(c_size);
            *op = if !(*op).is_null() {
                (*op).add(decompressed_size)
            } else {
                *op
            };
            (*zds).expected = 0;
            (*zds).streamStage = zdss_init;
            *some_more_work = 0;
            return None; // break
        }
    }

    /* Check output buffer is large enough for ZSTD_odm_stable. */
    if (*zds).outBufferMode == ZSTD_bm_stable
        && (*zds).fParams.frameType != ZSTD_skippableFrame
        && (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
        && ((oend as usize - *op as usize) as u64) < (*zds).fParams.frameContentSize
    {
        return Some(err_code(ZSTD_error_dstSize_tooSmall));
    }

    /* Consume header (see ZSTDds_decodeFrameHeader) */
    {
        let e = ZSTD_decompressBegin_usingDDict(zds, ZSTD_getDDict(zds));
        if err_is_error(e) {
            return Some(e);
        }
    }

    if (*zds).format == ZSTD_f_zstd1
        && (mem_read_le32((*zds).headerBuffer.as_ptr()) & ZSTD_MAGIC_SKIPPABLE_MASK)
            == ZSTD_MAGIC_SKIPPABLE_START
    {
        (*zds).expected =
            mem_read_le32((*zds).headerBuffer.as_ptr().add(ZSTD_FRAMEIDSIZE)) as usize;
        (*zds).stage = ZSTDds_skipFrame;
    } else {
        let e = ZSTD_decodeFrameHeader(
            zds,
            (*zds).headerBuffer.as_ptr() as *const c_void,
            (*zds).lhSize,
        );
        if err_is_error(e) {
            return Some(e);
        }
        (*zds).expected = ZSTD_blockHeaderSize;
        (*zds).stage = ZSTDds_decodeBlockHeader;
    }

    /* control buffer memory usage */
    (*zds).fParams.windowSize = max_u64((*zds).fParams.windowSize, 1u64 << ZSTD_WINDOWLOG_ABSOLUTEMIN);
    if (*zds).fParams.windowSize > (*zds).maxWindowSize as u64 {
        return Some(err_code(ZSTD_error_frameParameter_windowTooLarge));
    }
    if (*zds).maxBlockSizeParam != 0 {
        (*zds).fParams.blockSizeMax =
            min_u32((*zds).fParams.blockSizeMax, (*zds).maxBlockSizeParam as c_uint);
    }

    /* Adapt buffer sizes to frame header instructions */
    {
        let needed_in_buff_size = max_usize((*zds).fParams.blockSizeMax as usize, 4);
        let needed_out_buff_size = if (*zds).outBufferMode == ZSTD_bm_buffered {
            ZSTD_decodingBufferSize_internal(
                (*zds).fParams.windowSize,
                (*zds).fParams.frameContentSize,
                (*zds).fParams.blockSizeMax as usize,
            )
        } else {
            0
        };

        ZSTD_DCtx_updateOversizedDuration(zds, needed_in_buff_size, needed_out_buff_size);

        let too_small =
            ((*zds).inBuffSize < needed_in_buff_size) || ((*zds).outBuffSize < needed_out_buff_size);
        let too_large = ZSTD_DCtx_isOversizedTooLong(zds);

        if too_small || too_large != 0 {
            let buffer_size = needed_in_buff_size + needed_out_buff_size;
            if (*zds).staticSize != 0 {
                if buffer_size > (*zds).staticSize - core::mem::size_of::<ZSTD_DCtx>() {
                    return Some(err_code(ZSTD_error_memory_allocation));
                }
            } else {
                zstd_custom_free((*zds).inBuff as *mut c_void, (*zds).customMem);
                (*zds).inBuffSize = 0;
                (*zds).outBuffSize = 0;
                (*zds).inBuff = zstd_custom_malloc(buffer_size, (*zds).customMem) as *mut core::ffi::c_char;
                if (*zds).inBuff.is_null() {
                    return Some(err_code(ZSTD_error_memory_allocation));
                }
            }
            (*zds).inBuffSize = needed_in_buff_size;
            (*zds).outBuff = (*zds).inBuff.add((*zds).inBuffSize);
            (*zds).outBuffSize = needed_out_buff_size;
        }
    }
    (*zds).streamStage = zdss_read;
    // fallthrough to zdss_read
    zdss_read_stage(zds, ip, iend, op, oend, some_more_work)
}

/// zdss_read stage body.
unsafe fn zdss_read_stage(
    zds: *mut ZSTD_DCtx,
    ip: &mut *const core::ffi::c_char,
    iend: *const core::ffi::c_char,
    op: &mut *mut core::ffi::c_char,
    oend: *mut core::ffi::c_char,
    some_more_work: &mut U32,
) -> Option<usize> {
    {
        let needed_in_size =
            ZSTD_nextSrcSizeToDecompressWithInputSize(zds, iend as usize - *ip as usize);
        if needed_in_size == 0 {
            (*zds).streamStage = zdss_init;
            *some_more_work = 0;
            return None; // break
        }
        if (iend as usize - *ip as usize) >= needed_in_size {
            let e = ZSTD_decompressContinueStream(zds, op, oend, *ip as *const c_void, needed_in_size);
            if err_is_error(e) {
                return Some(e);
            }
            *ip = (*ip).add(needed_in_size);
            return None; // break
        }
    }
    if *ip == iend {
        *some_more_work = 0;
        return None; // break
    }
    (*zds).streamStage = zdss_load;
    // fallthrough to zdss_load
    zdss_load_stage(zds, ip, iend, op, oend, some_more_work)
}

/// zdss_load stage body.
unsafe fn zdss_load_stage(
    zds: *mut ZSTD_DCtx,
    ip: &mut *const core::ffi::c_char,
    iend: *const core::ffi::c_char,
    op: &mut *mut core::ffi::c_char,
    oend: *mut core::ffi::c_char,
    some_more_work: &mut U32,
) -> Option<usize> {
    let needed_in_size = ZSTD_nextSrcSizeToDecompress(zds);
    let to_load = needed_in_size - (*zds).inPos;
    let is_skip_frame = ZSTD_isSkipFrame(zds);
    let loaded_size;
    if is_skip_frame != 0 {
        loaded_size = min_usize(to_load, iend as usize - *ip as usize);
    } else {
        if to_load > (*zds).inBuffSize - (*zds).inPos {
            return Some(err_code(ZSTD_error_corruption_detected));
        }
        loaded_size = zstd_limit_copy(
            (*zds).inBuff.add((*zds).inPos) as *mut u8,
            to_load,
            *ip as *const u8,
            iend as usize - *ip as usize,
        );
    }
    if loaded_size != 0 {
        *ip = (*ip).add(loaded_size);
        (*zds).inPos += loaded_size;
    }
    if loaded_size < to_load {
        *some_more_work = 0;
        return None; // break
    }

    (*zds).inPos = 0;
    let e = ZSTD_decompressContinueStream(
        zds,
        op,
        oend,
        (*zds).inBuff as *const c_void,
        needed_in_size,
    );
    if err_is_error(e) {
        return Some(e);
    }
    None // break
}

#[inline(always)]
fn max_u64(a: U64, b: U64) -> U64 {
    if a > b {
        a
    } else {
        b
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream_simpleArgs(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    dst_pos: *mut usize,
    src: *const c_void,
    src_size: usize,
    src_pos: *mut usize,
) -> usize {
    let mut output = ZSTD_outBuffer {
        dst,
        size: dst_capacity,
        pos: *dst_pos,
    };
    let mut input = ZSTD_inBuffer {
        src,
        size: src_size,
        pos: *src_pos,
    };
    let c_err = ZSTD_decompressStream(dctx, &mut output, &mut input);
    *dst_pos = output.pos;
    *src_pos = input.pos;
    c_err
}
