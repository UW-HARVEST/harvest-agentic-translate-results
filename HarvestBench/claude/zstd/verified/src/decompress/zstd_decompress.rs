/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

//! Translation of c_src/src/decompress/zstd_decompress.c
//!
//! Build configuration:
//!   ZSTD_HEAPMODE = 1
//!   ZSTD_LEGACY_SUPPORT = 5   (legacy v05, v06, v07 dispatch enabled)
//!   DYNAMIC_BMI2 = 0
//!   ZSTD_TRACE = 0
//!   single-threaded, no ASM.
//! Target: little-endian 64-bit.

#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use core::ffi::{c_int, c_uint, c_ulonglong, c_void};

use crate::common::allocations::{
    zstd_custom_calloc, zstd_custom_free, zstd_custom_malloc, ZSTD_customMem, ZSTD_defaultCMem,
};
use crate::common::bits::highbit32;
use crate::common::error::{code, err_get_error_code, err_is_error, error};
use crate::common::mem::{mem_read_le16, mem_read_le32, mem_read_le64, mem_write_le32};
use crate::common::xxhash::{ZSTD_XXH64, ZSTD_XXH64_digest, ZSTD_XXH64_reset, ZSTD_XXH64_update};
use crate::common::zstd_internal::{
    blockProperties_t, bt_compressed, bt_raw, bt_reserved, bt_rle, repStartValue,
    ZSTD_blockHeaderSize, ZSTD_bm_buffered, ZSTD_bm_stable, ZSTD_did_fieldSize, ZSTD_fcs_fieldSize,
    ZSTD_frameSizeInfo, ZSTD_FRAMEIDSIZE, ZSTD_WINDOWLOG_ABSOLUTEMIN,
    ZSTD_WORKSPACETOOLARGE_FACTOR, ZSTD_WORKSPACETOOLARGE_MAXDURATION, WILDCOPY_OVERLENGTH,
};
use crate::zstd_h::{
    ZSTD_bounds, ZSTD_BLOCKSIZE_MAX, ZSTD_CONTENTSIZE_ERROR,
    ZSTD_CONTENTSIZE_UNKNOWN, ZSTD_d_ignoreChecksum, ZSTD_d_validateChecksum, ZSTD_dct_auto,
    ZSTD_dct_rawContent, ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e, ZSTD_dlm_byCopy,
    ZSTD_dlm_byRef, ZSTD_f_zstd1, ZSTD_f_zstd1_magicless, ZSTD_format_e, ZSTD_inBuffer,
    ZSTD_MAGIC_DICTIONARY, ZSTD_MAGIC_SKIPPABLE_MASK, ZSTD_MAGIC_SKIPPABLE_START, ZSTD_MAGICNUMBER,
    ZSTD_nextInputType_e, ZSTD_outBuffer, ZSTD_reset_parameters, ZSTD_reset_session_and_parameters,
    ZSTD_reset_session_only, ZSTD_ResetDirective, ZSTD_rmd_refMultipleDDicts,
    ZSTD_rmd_refSingleDDict, ZSTDnit_block, ZSTDnit_blockHeader, ZSTDnit_checksum,
    ZSTDnit_frameHeader, ZSTDnit_lastBlock, ZSTDnit_skippableFrame,
};

use crate::decompress::zstd_decompress_internal::*;

use crate::decompress::zstd_ddict::{
    ZSTD_copyDDictParameters, ZSTD_createDDict_advanced, ZSTD_DDict_dictContent,
    ZSTD_DDict_dictSize, ZSTD_freeDDict, ZSTD_getDictID_fromDDict, ZSTD_sizeof_DDict,
};

use crate::decompress::zstd_decompress_block::{
    is_streaming, not_streaming, streaming_operation, ZSTD_getcBlockSize,
};

/* ZSTD_checkContinuity and ZSTD_decompressBlock_internal are defined in the
 * block module (zstd_decompress_block) as exported C symbols. */
extern "C" {
    fn ZSTD_checkContinuity(dctx: *mut ZSTD_DCtx, dst: *const c_void, dstSize: usize);
    fn ZSTD_decompressBlock_internal(
        dctx: *mut ZSTD_DCtx,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        streaming: streaming_operation,
    ) -> usize;
    fn ZSTD_buildFSETable(
        dt: *mut ZSTD_seqSymbol,
        normalizedCounter: *const i16,
        maxSymbolValue: u32,
        baseValue: *const u32,
        nbAdditionalBits: *const u8,
        tableLog: u32,
        wksp: *mut c_void,
        wkspSize: usize,
        bmi2: c_int,
    ) -> usize;
    fn HUF_readDTableX2_wksp(
        DTable: *mut HUF_DTable,
        src: *const c_void,
        srcSize: usize,
        workSpace: *mut c_void,
        wkspSize: usize,
        flags: c_int,
    ) -> usize;
}

/* ZSTD_loadDEntropy() : translation of the definition in zstd_decompress.c.
 *  dict : must point at beginning of a valid zstd dictionary.
 * @return : size of entropy tables read */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_loadDEntropy(
    entropy: *mut ZSTD_entropyDTables_t,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    use crate::common::fse::{FSE_isError, FSE_readNCount};
    use crate::common::huf_common::HUF_isError;
    use crate::common::mem::mem_read_le32 as MEM_readLE32;
    use crate::common::zstd_internal::{
        LL_bits, ML_bits, MaxLL, MaxML, MaxOff,
    };

    let mut dictPtr = dict as *const u8;
    let dictEnd = dictPtr.add(dictSize);

    if dictSize <= 8 {
        return error(code::DICTIONARY_CORRUPTED);
    }
    dictPtr = dictPtr.add(8); /* skip header = magic + dictID */

    {
        let workspace = core::ptr::addr_of_mut!((*entropy).LLTable) as *mut c_void;
        let workspaceSize = core::mem::size_of_val(&(*entropy).LLTable)
            + core::mem::size_of_val(&(*entropy).OFTable)
            + core::mem::size_of_val(&(*entropy).MLTable);
        let hSize = HUF_readDTableX2_wksp(
            (*entropy).hufTable.as_mut_ptr(),
            dictPtr as *const c_void,
            (dictEnd as usize) - (dictPtr as usize),
            workspace,
            workspaceSize,
            0,
        );
        if HUF_isError(hSize) != 0 {
            return error(code::DICTIONARY_CORRUPTED);
        }
        dictPtr = dictPtr.add(hSize);
    }

    {
        let mut offcodeNCount = [0i16; (MaxOff as usize) + 1];
        let mut offcodeMaxValue: u32 = MaxOff;
        let mut offcodeLog: u32 = 0;
        let offcodeHeaderSize = FSE_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            (dictEnd as usize) - (dictPtr as usize),
        );
        if FSE_isError(offcodeHeaderSize) != 0 {
            return error(code::DICTIONARY_CORRUPTED);
        }
        if offcodeMaxValue > MaxOff {
            return error(code::DICTIONARY_CORRUPTED);
        }
        if offcodeLog > OffFSELog {
            return error(code::DICTIONARY_CORRUPTED);
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
            0,
        );
        dictPtr = dictPtr.add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount = [0i16; (MaxML as usize) + 1];
        let mut matchlengthMaxValue: u32 = MaxML;
        let mut matchlengthLog: u32 = 0;
        let matchlengthHeaderSize = FSE_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize) - (dictPtr as usize),
        );
        if FSE_isError(matchlengthHeaderSize) != 0 {
            return error(code::DICTIONARY_CORRUPTED);
        }
        if matchlengthMaxValue > MaxML {
            return error(code::DICTIONARY_CORRUPTED);
        }
        if matchlengthLog > MLFSELog {
            return error(code::DICTIONARY_CORRUPTED);
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
            0,
        );
        dictPtr = dictPtr.add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount = [0i16; (MaxLL as usize) + 1];
        let mut litlengthMaxValue: u32 = MaxLL;
        let mut litlengthLog: u32 = 0;
        let litlengthHeaderSize = FSE_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize) - (dictPtr as usize),
        );
        if FSE_isError(litlengthHeaderSize) != 0 {
            return error(code::DICTIONARY_CORRUPTED);
        }
        if litlengthMaxValue > MaxLL {
            return error(code::DICTIONARY_CORRUPTED);
        }
        if litlengthLog > LLFSELog {
            return error(code::DICTIONARY_CORRUPTED);
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
            0,
        );
        dictPtr = dictPtr.add(litlengthHeaderSize);
    }

    if dictPtr.add(12) > dictEnd {
        return error(code::DICTIONARY_CORRUPTED);
    }
    {
        let dictContentSize = (dictEnd as usize) - ((dictPtr as usize) + 12);
        for i in 0..3 {
            let rep = MEM_readLE32(dictPtr as *const c_void);
            dictPtr = dictPtr.add(4);
            if rep == 0 || (rep as usize) > dictContentSize {
                return error(code::DICTIONARY_CORRUPTED);
            }
            (*entropy).rep[i] = rep;
        }
    }

    (dictPtr as usize) - (dict as usize)
}

/* Legacy decoders (ZSTD_LEGACY_SUPPORT == 5 => versions 5, 6, 7 active).
 * These functions reference module-private DCtx/params types in their real
 * signatures, so we declare them here with opaque pointers matching the C ABI. */
extern "C" {
    fn ZSTDv05_getFrameParams(params: *mut c_void, src: *const c_void, srcSize: usize) -> usize;
    fn ZSTDv05_createDCtx() -> *mut c_void;
    fn ZSTDv05_freeDCtx(dctx: *mut c_void) -> usize;
    fn ZSTDv05_decompress_usingDict(
        dctx: *mut c_void,
        dst: *mut c_void,
        maxDstSize: usize,
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
    fn ZBUFFv05_freeDCtx(zbc: *mut c_void) -> usize;
    fn ZBUFFv05_decompressInitDictionary(
        zbc: *mut c_void,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZBUFFv05_decompressContinue(
        zbc: *mut c_void,
        dst: *mut c_void,
        maxDstSizePtr: *mut usize,
        src: *const c_void,
        srcSizePtr: *mut usize,
    ) -> usize;

    fn ZSTDv06_getFrameParams(fparamsPtr: *mut c_void, src: *const c_void, srcSize: usize) -> usize;
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
    fn ZBUFFv06_freeDCtx(zbd: *mut c_void) -> usize;
    fn ZBUFFv06_decompressInitDictionary(
        zbd: *mut c_void,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZBUFFv06_decompressContinue(
        zbd: *mut c_void,
        dst: *mut c_void,
        dstCapacityPtr: *mut usize,
        src: *const c_void,
        srcSizePtr: *mut usize,
    ) -> usize;

    fn ZSTDv07_getFrameParams(fparamsPtr: *mut c_void, src: *const c_void, srcSize: usize) -> usize;
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
        dBound: *mut c_ulonglong,
    );
    fn ZBUFFv07_createDCtx() -> *mut c_void;
    fn ZBUFFv07_freeDCtx(zbd: *mut c_void) -> usize;
    fn ZBUFFv07_decompressInitDictionary(
        zbd: *mut c_void,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZBUFFv07_decompressContinue(
        zbd: *mut c_void,
        dst: *mut c_void,
        dstCapacityPtr: *mut usize,
        src: *const c_void,
        srcSizePtr: *mut usize,
    ) -> usize;
}

/* Type aliases matching C */
type U32 = u32;
type U64 = u64;
type BYTE = u8;

/* ZSTD_DStream is a typedef of ZSTD_DCtx */
pub type ZSTD_DStream = ZSTD_DCtx;

/* ===== Tuning parameters ===== */
const ZSTD_NO_FORWARD_PROGRESS_MAX: i32 = 16;

/* ===== Constants not (yet) in the shared crate modules ===== */
const ZSTD_WINDOWLOG_MAX: u32 = 31; /* LE 64-bit */
const ZSTD_WINDOWLOG_LIMIT_DEFAULT: u32 = 27;
const ZSTD_MAXWINDOWSIZE_DEFAULT: usize = (1usize << ZSTD_WINDOWLOG_LIMIT_DEFAULT) + 1;
const ZSTD_SKIPPABLEHEADERSIZE: usize = 8;
const ZSTD_BLOCKSIZE_MAX_MIN: i32 = 1 << 10;

/* ZSTD_dParameter enum values (from zstd.h) */
const ZSTD_d_windowLogMax: c_int = 100;
const ZSTD_d_format: c_int = 1000; /* experimentalParam1 */
const ZSTD_d_stableOutBuffer: c_int = 1001; /* experimentalParam2 */
const ZSTD_d_forceIgnoreChecksum: c_int = 1002; /* experimentalParam3 */
const ZSTD_d_refMultipleDDicts: c_int = 1003; /* experimentalParam4 */
const ZSTD_d_disableHuffmanAssembly: c_int = 1004; /* experimentalParam5 */
const ZSTD_d_maxBlockSize: c_int = 1005; /* experimentalParam6 */

/* Multiple DDicts hashset load-factor / sizing constants */
const DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT: usize = 4;
const DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT: usize = 3;
const DDICT_HASHSET_TABLE_BASE_SIZE: usize = 64;
const DDICT_HASHSET_RESIZE_FACTOR: usize = 2;

/* ZSTD_error_prefix_unknown, used by ZSTD_getErrorCode comparison */
const ZSTD_error_prefix_unknown: i32 = code::PREFIX_UNKNOWN;

/* ===== small helpers ===== */
#[inline]
fn ZSTD_isError(code: usize) -> u32 {
    err_is_error(code)
}

#[inline]
fn ZSTD_getErrorCode(code: usize) -> i32 {
    err_get_error_code(code)
}

#[inline]
fn MIN_usize(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}
#[inline]
fn MAX_usize(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}
#[inline]
fn MIN_u64(a: u64, b: u64) -> u64 {
    if a < b { a } else { b }
}
#[inline]
fn MAX_u64(a: u64, b: u64) -> u64 {
    if a > b { a } else { b }
}

/* ZSTD_FRAMEHEADERSIZE_PREFIX(format) : minimum input size to query header size */
#[inline]
fn ZSTD_FRAMEHEADERSIZE_PREFIX(format: ZSTD_format_e) -> usize {
    if format == ZSTD_f_zstd1 { 5 } else { 1 }
}
/* ZSTD_FRAMEHEADERSIZE_MIN(format) */
#[inline]
fn ZSTD_FRAMEHEADERSIZE_MIN(format: ZSTD_format_e) -> usize {
    if format == ZSTD_f_zstd1 { 6 } else { 2 }
}

/* ZSTD_limitCopy (from zstd_internal.h) */
#[inline]
unsafe fn ZSTD_limitCopy(dst: *mut c_void, dstCapacity: usize, src: *const c_void, srcSize: usize) -> usize {
    let length = MIN_usize(dstCapacity, srcSize);
    if length > 0 {
        core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, length);
    }
    length
}

/* BOUNDED(min, val, max) */
#[inline]
fn BOUNDED(min: usize, val: usize, max: usize) -> usize {
    MIN_usize(MAX_usize(min, val), max)
}

/*************************************
 * Multiple DDicts Hashset internals *
 *************************************/

unsafe fn ZSTD_DDictHashSet_getIndex(hashSet: *const ZSTD_DDictHashSet, dictID: U32) -> usize {
    let hash: U64 = ZSTD_XXH64(
        &dictID as *const U32 as *const c_void,
        core::mem::size_of::<U32>(),
        0,
    );
    (hash as usize) & ((*hashSet).ddictPtrTableSize - 1)
}

unsafe fn ZSTD_DDictHashSet_emplaceDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    ddict: *const ZSTD_DDict,
) -> usize {
    let dictID: U32 = ZSTD_getDictID_fromDDict(ddict);
    let mut idx = ZSTD_DDictHashSet_getIndex(hashSet, dictID);
    let idxRangeMask = (*hashSet).ddictPtrTableSize - 1;
    if (*hashSet).ddictPtrCount == (*hashSet).ddictPtrTableSize {
        return error(code::GENERIC);
    }
    let table = (*hashSet).ddictPtrTable as *mut *const ZSTD_DDict;
    while !(*table.add(idx)).is_null() {
        /* Replace existing ddict if inserting ddict with same dictID */
        if ZSTD_getDictID_fromDDict(*table.add(idx)) == dictID {
            *table.add(idx) = ddict;
            return 0;
        }
        idx &= idxRangeMask;
        idx += 1;
    }
    *table.add(idx) = ddict;
    (*hashSet).ddictPtrCount += 1;
    0
}

unsafe fn ZSTD_DDictHashSet_expand(
    hashSet: *mut ZSTD_DDictHashSet,
    customMem: ZSTD_customMem,
) -> usize {
    let newTableSize = (*hashSet).ddictPtrTableSize * DDICT_HASHSET_RESIZE_FACTOR;
    let newTable = zstd_custom_calloc(
        core::mem::size_of::<*const ZSTD_DDict>() * newTableSize,
        customMem,
    ) as *const *const ZSTD_DDict;
    let oldTable = (*hashSet).ddictPtrTable;
    let oldTableSize = (*hashSet).ddictPtrTableSize;

    if newTable.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    (*hashSet).ddictPtrTable = newTable;
    (*hashSet).ddictPtrTableSize = newTableSize;
    (*hashSet).ddictPtrCount = 0;
    let mut i: usize = 0;
    while i < oldTableSize {
        if !(*oldTable.add(i)).is_null() {
            let e = ZSTD_DDictHashSet_emplaceDDict(hashSet, *oldTable.add(i));
            if ZSTD_isError(e) != 0 {
                return e;
            }
        }
        i += 1;
    }
    zstd_custom_free(oldTable as *mut c_void, customMem);
    0
}

unsafe fn ZSTD_DDictHashSet_getDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    dictID: U32,
) -> *const ZSTD_DDict {
    let mut idx = ZSTD_DDictHashSet_getIndex(hashSet, dictID);
    let idxRangeMask = (*hashSet).ddictPtrTableSize - 1;
    let table = (*hashSet).ddictPtrTable;
    loop {
        let currDictID = ZSTD_getDictID_fromDDict(*table.add(idx));
        if currDictID == dictID || currDictID == 0 {
            /* currDictID == 0 implies a NULL ddict entry */
            break;
        } else {
            idx &= idxRangeMask; /* Goes to start of table when we reach the end */
            idx += 1;
        }
    }
    *table.add(idx)
}

unsafe fn ZSTD_createDDictHashSet(customMem: ZSTD_customMem) -> *mut ZSTD_DDictHashSet {
    let ret = zstd_custom_malloc(core::mem::size_of::<ZSTD_DDictHashSet>(), customMem)
        as *mut ZSTD_DDictHashSet;
    if ret.is_null() {
        return core::ptr::null_mut();
    }
    (*ret).ddictPtrTable = zstd_custom_calloc(
        DDICT_HASHSET_TABLE_BASE_SIZE * core::mem::size_of::<*const ZSTD_DDict>(),
        customMem,
    ) as *const *const ZSTD_DDict;
    if (*ret).ddictPtrTable.is_null() {
        zstd_custom_free(ret as *mut c_void, customMem);
        return core::ptr::null_mut();
    }
    (*ret).ddictPtrTableSize = DDICT_HASHSET_TABLE_BASE_SIZE;
    (*ret).ddictPtrCount = 0;
    ret
}

unsafe fn ZSTD_freeDDictHashSet(hashSet: *mut ZSTD_DDictHashSet, customMem: ZSTD_customMem) {
    if !hashSet.is_null() && !(*hashSet).ddictPtrTable.is_null() {
        zstd_custom_free((*hashSet).ddictPtrTable as *mut c_void, customMem);
    }
    if !hashSet.is_null() {
        zstd_custom_free(hashSet as *mut c_void, customMem);
    }
}

unsafe fn ZSTD_DDictHashSet_addDDict(
    hashSet: *mut ZSTD_DDictHashSet,
    ddict: *const ZSTD_DDict,
    customMem: ZSTD_customMem,
) -> usize {
    if (*hashSet).ddictPtrCount * DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT
        / (*hashSet).ddictPtrTableSize
        * DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT
        != 0
    {
        let e = ZSTD_DDictHashSet_expand(hashSet, customMem);
        if ZSTD_isError(e) != 0 {
            return e;
        }
    }
    let e = ZSTD_DDictHashSet_emplaceDDict(hashSet, ddict);
    if ZSTD_isError(e) != 0 {
        return e;
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
pub unsafe extern "C" fn ZSTD_estimateDCtxSize() -> usize {
    core::mem::size_of::<ZSTD_DCtx>()
}

fn ZSTD_startingInputLength(format: ZSTD_format_e) -> usize {
    let startingInputLength = ZSTD_FRAMEHEADERSIZE_PREFIX(format);
    /* only supports formats ZSTD_f_zstd1 and ZSTD_f_zstd1_magicless */
    debug_assert!(format == ZSTD_f_zstd1 || format == ZSTD_f_zstd1_magicless);
    startingInputLength
}

unsafe fn ZSTD_DCtx_resetParameters(dctx: *mut ZSTD_DCtx) {
    debug_assert!((*dctx).streamStage == zdss_init);
    (*dctx).format = ZSTD_f_zstd1;
    (*dctx).maxWindowSize = ZSTD_MAXWINDOWSIZE_DEFAULT;
    (*dctx).outBufferMode = ZSTD_bm_buffered;
    (*dctx).forceIgnoreChecksum = ZSTD_d_validateChecksum;
    (*dctx).refMultipleDDicts = ZSTD_rmd_refSingleDDict;
    (*dctx).disableHufAsm = 0;
    (*dctx).maxBlockSizeParam = 0;
}

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
    let dctx = workspace as *mut ZSTD_DCtx;

    if (workspace as usize) & 7 != 0 {
        return core::ptr::null_mut();
    } /* 8-aligned */
    if workspaceSize < core::mem::size_of::<ZSTD_DCtx>() {
        return core::ptr::null_mut();
    } /* minimum size */

    ZSTD_initDCtx_internal(dctx);
    (*dctx).staticSize = workspaceSize;
    (*dctx).inBuff = dctx.add(1) as *mut u8;
    dctx
}

unsafe fn ZSTD_createDCtx_internal(customMem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    if (customMem.customAlloc.is_none()) ^ (customMem.customFree.is_none()) {
        return core::ptr::null_mut();
    }

    let dctx = zstd_custom_malloc(core::mem::size_of::<ZSTD_DCtx>(), customMem) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    (*dctx).customMem = customMem;
    ZSTD_initDCtx_internal(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    ZSTD_createDCtx_internal(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    ZSTD_createDCtx_internal(ZSTD_defaultCMem)
}

unsafe fn ZSTD_clearDict(dctx: *mut ZSTD_DCtx) {
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
        return error(code::MEMORY_ALLOCATION);
    }
    let cMem = (*dctx).customMem;
    ZSTD_clearDict(dctx);
    zstd_custom_free((*dctx).inBuff as *mut c_void, cMem);
    (*dctx).inBuff = core::ptr::null_mut();
    if !(*dctx).legacyContext.is_null() {
        ZSTD_freeLegacyStreamContext((*dctx).legacyContext, (*dctx).previousLegacyVersion);
    }
    if !(*dctx).ddictSet.is_null() {
        ZSTD_freeDDictHashSet((*dctx).ddictSet, cMem);
        (*dctx).ddictSet = core::ptr::null_mut();
    }
    zstd_custom_free(dctx as *mut c_void, cMem);
    0
}

/* no longer useful */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyDCtx(dstDCtx: *mut ZSTD_DCtx, srcDCtx: *const ZSTD_DCtx) {
    let toCopy = (&(*dstDCtx).inBuff as *const *mut u8 as *const u8)
        .offset_from(dstDCtx as *const u8) as usize;
    core::ptr::copy_nonoverlapping(srcDCtx as *const u8, dstDCtx as *mut u8, toCopy);
}

unsafe fn ZSTD_DCtx_selectFrameDDict(dctx: *mut ZSTD_DCtx) {
    debug_assert!((*dctx).refMultipleDDicts != 0 && !(*dctx).ddictSet.is_null());
    if !(*dctx).ddict.is_null() {
        let frameDDict = ZSTD_DDictHashSet_getDDict((*dctx).ddictSet, (*dctx).fParams.dictID);
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
pub unsafe extern "C" fn ZSTD_isFrame(buffer: *const c_void, size: usize) -> c_uint {
    if size < ZSTD_FRAMEIDSIZE {
        return 0;
    }
    let magic: U32 = mem_read_le32(buffer);
    if magic == ZSTD_MAGICNUMBER {
        return 1;
    }
    if (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
        return 1;
    }
    if ZSTD_isLegacy(buffer, size) != 0 {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isSkippableFrame(buffer: *const c_void, size: usize) -> c_uint {
    if size < ZSTD_FRAMEIDSIZE {
        return 0;
    }
    let magic: U32 = mem_read_le32(buffer);
    if (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
        return 1;
    }
    0
}

unsafe fn ZSTD_frameHeaderSize_internal(
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> usize {
    let minInputSize = ZSTD_startingInputLength(format);
    if srcSize < minInputSize {
        return error(code::SRCSIZE_WRONG);
    }

    let fhd: BYTE = *(src as *const BYTE).add(minInputSize - 1);
    let dictID: U32 = (fhd & 3) as U32;
    let singleSegment: U32 = ((fhd >> 5) & 1) as U32;
    let fcsId: U32 = (fhd >> 6) as U32;
    minInputSize
        + (singleSegment == 0) as usize
        + ZSTD_did_fieldSize[dictID as usize]
        + ZSTD_fcs_fieldSize[fcsId as usize]
        + ((singleSegment != 0 && fcsId == 0) as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    ZSTD_frameHeaderSize_internal(src, srcSize, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader_advanced(
    zfhPtr: *mut ZSTD_FrameHeader,
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> usize {
    let ip = src as *const BYTE;
    let minInputSize = ZSTD_startingInputLength(format);

    if srcSize > 0 {
        /* invalid entry */
        if src.is_null() {
            return error(code::GENERIC);
        }
    }
    if srcSize < minInputSize {
        if srcSize > 0 && format != ZSTD_f_zstd1_magicless {
            let toCopy = MIN_usize(4, srcSize);
            let mut hbuf: [u8; 4] = [0; 4];
            mem_write_le32(hbuf.as_mut_ptr() as *mut c_void, ZSTD_MAGICNUMBER);
            core::ptr::copy_nonoverlapping(src as *const u8, hbuf.as_mut_ptr(), toCopy);
            if mem_read_le32(hbuf.as_ptr() as *const c_void) != ZSTD_MAGICNUMBER {
                /* not a zstd frame : let's check if it's a skippable frame */
                mem_write_le32(hbuf.as_mut_ptr() as *mut c_void, ZSTD_MAGIC_SKIPPABLE_START);
                core::ptr::copy_nonoverlapping(src as *const u8, hbuf.as_mut_ptr(), toCopy);
                if (mem_read_le32(hbuf.as_ptr() as *const c_void) & ZSTD_MAGIC_SKIPPABLE_MASK)
                    != ZSTD_MAGIC_SKIPPABLE_START
                {
                    return error(code::PREFIX_UNKNOWN);
                }
            }
        }
        return minInputSize;
    }

    core::ptr::write_bytes(zfhPtr as *mut u8, 0, core::mem::size_of::<ZSTD_FrameHeader>());
    if format != ZSTD_f_zstd1_magicless && mem_read_le32(src) != ZSTD_MAGICNUMBER {
        if (mem_read_le32(src) & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            /* skippable frame */
            if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
                return ZSTD_SKIPPABLEHEADERSIZE; /* magic number + frame length */
            }
            core::ptr::write_bytes(zfhPtr as *mut u8, 0, core::mem::size_of::<ZSTD_FrameHeader>());
            (*zfhPtr).frameType = ZSTD_skippableFrame;
            (*zfhPtr).dictID = mem_read_le32(src) - ZSTD_MAGIC_SKIPPABLE_START;
            (*zfhPtr).headerSize = ZSTD_SKIPPABLEHEADERSIZE as U32;
            (*zfhPtr).frameContentSize =
                mem_read_le32((src as *const u8).add(ZSTD_FRAMEIDSIZE) as *const c_void) as u64;
            return 0;
        }
        return error(code::PREFIX_UNKNOWN);
    }

    /* ensure there is enough `srcSize` to fully read/decode frame header */
    {
        let fhsize = ZSTD_frameHeaderSize_internal(src, srcSize, format);
        if srcSize < fhsize {
            return fhsize;
        }
        (*zfhPtr).headerSize = fhsize as U32;
    }

    {
        let fhdByte: BYTE = *ip.add(minInputSize - 1);
        let mut pos = minInputSize;
        let dictIDSizeCode: U32 = (fhdByte & 3) as U32;
        let checksumFlag: U32 = ((fhdByte >> 2) & 1) as U32;
        let singleSegment: U32 = ((fhdByte >> 5) & 1) as U32;
        let fcsID: U32 = (fhdByte >> 6) as U32;
        let mut windowSize: U64 = 0;
        let mut dictID: U32 = 0;
        let mut frameContentSize: U64 = ZSTD_CONTENTSIZE_UNKNOWN;
        if (fhdByte & 0x08) != 0 {
            return error(code::FRAMEPARAMETER_UNSUPPORTED);
        }

        if singleSegment == 0 {
            let wlByte: BYTE = *ip.add(pos);
            pos += 1;
            let windowLog: U32 = ((wlByte >> 3) as U32) + ZSTD_WINDOWLOG_ABSOLUTEMIN;
            if windowLog > ZSTD_WINDOWLOG_MAX {
                return error(code::FRAMEPARAMETER_WINDOWTOOLARGE);
            }
            windowSize = 1u64 << windowLog;
            windowSize += (windowSize >> 3) * ((wlByte & 7) as u64);
        }
        match dictIDSizeCode {
            0 => {}
            1 => {
                dictID = *ip.add(pos) as U32;
                pos += 1;
            }
            2 => {
                dictID = mem_read_le16(ip.add(pos) as *const c_void) as U32;
                pos += 2;
            }
            3 => {
                dictID = mem_read_le32(ip.add(pos) as *const c_void);
                pos += 4;
            }
            _ => {
                debug_assert!(false); /* impossible */
            }
        }
        match fcsID {
            0 => {
                if singleSegment != 0 {
                    frameContentSize = *ip.add(pos) as U64;
                }
            }
            1 => {
                frameContentSize = mem_read_le16(ip.add(pos) as *const c_void) as U64 + 256;
            }
            2 => {
                frameContentSize = mem_read_le32(ip.add(pos) as *const c_void) as U64;
            }
            3 => {
                frameContentSize = mem_read_le64(ip.add(pos) as *const c_void);
            }
            _ => {
                debug_assert!(false); /* impossible */
            }
        }
        if singleSegment != 0 {
            windowSize = frameContentSize;
        }

        (*zfhPtr).frameType = ZSTD_frame;
        (*zfhPtr).frameContentSize = frameContentSize;
        (*zfhPtr).windowSize = windowSize;
        (*zfhPtr).blockSizeMax = MIN_u64(windowSize, ZSTD_BLOCKSIZE_MAX as u64) as u32;
        (*zfhPtr).dictID = dictID;
        (*zfhPtr).checksumFlag = checksumFlag;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader(
    zfhPtr: *mut ZSTD_FrameHeader,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_getFrameHeader_advanced(zfhPtr, src, srcSize, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameContentSize(
    src: *const c_void,
    srcSize: usize,
) -> c_ulonglong {
    if ZSTD_isLegacy(src, srcSize) != 0 {
        let ret = ZSTD_getDecompressedSize_legacy(src, srcSize);
        return if ret == 0 { ZSTD_CONTENTSIZE_UNKNOWN } else { ret };
    }
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

unsafe fn readSkippableFrameSize(src: *const c_void, srcSize: usize) -> usize {
    let skippableHeaderSize = ZSTD_SKIPPABLEHEADERSIZE;

    if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
        return error(code::SRCSIZE_WRONG);
    }

    let sizeU32: U32 = mem_read_le32((src as *const BYTE).add(ZSTD_FRAMEIDSIZE) as *const c_void);
    if (sizeU32.wrapping_add(ZSTD_SKIPPABLEHEADERSIZE as U32)) < sizeU32 {
        return error(code::FRAMEPARAMETER_UNSUPPORTED);
    }
    let skippableSize = skippableHeaderSize + sizeU32 as usize;
    if skippableSize > srcSize {
        return error(code::SRCSIZE_WRONG);
    }
    skippableSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_readSkippableFrame(
    dst: *mut c_void,
    dstCapacity: usize,
    magicVariant: *mut c_uint,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < ZSTD_SKIPPABLEHEADERSIZE {
        return error(code::SRCSIZE_WRONG);
    }

    let magicNumber: U32 = mem_read_le32(src);
    let skippableFrameSize = readSkippableFrameSize(src, srcSize);
    let skippableContentSize = skippableFrameSize - ZSTD_SKIPPABLEHEADERSIZE;

    /* check input validity */
    if ZSTD_isSkippableFrame(src, srcSize) == 0 {
        return error(code::FRAMEPARAMETER_UNSUPPORTED);
    }
    if skippableFrameSize < ZSTD_SKIPPABLEHEADERSIZE || skippableFrameSize > srcSize {
        return error(code::SRCSIZE_WRONG);
    }
    if skippableContentSize > dstCapacity {
        return error(code::DSTSIZE_TOOSMALL);
    }

    /* deliver payload */
    if skippableContentSize > 0 && !dst.is_null() {
        core::ptr::copy_nonoverlapping(
            (src as *const BYTE).add(ZSTD_SKIPPABLEHEADERSIZE),
            dst as *mut u8,
            skippableContentSize,
        );
    }
    if !magicVariant.is_null() {
        *magicVariant = magicNumber - ZSTD_MAGIC_SKIPPABLE_START;
    }
    skippableContentSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findDecompressedSize(
    src: *const c_void,
    srcSize: usize,
) -> c_ulonglong {
    let mut totalDstSize: c_ulonglong = 0;
    let mut src = src;
    let mut srcSize = srcSize;

    while srcSize >= ZSTD_startingInputLength(ZSTD_f_zstd1) {
        let magicNumber: U32 = mem_read_le32(src);

        if (magicNumber & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
            let skippableSize = readSkippableFrameSize(src, srcSize);
            if ZSTD_isError(skippableSize) != 0 {
                return ZSTD_CONTENTSIZE_ERROR;
            }
            src = (src as *const BYTE).add(skippableSize) as *const c_void;
            srcSize -= skippableSize;
            continue;
        }

        let fcs = ZSTD_getFrameContentSize(src, srcSize);
        if fcs >= ZSTD_CONTENTSIZE_ERROR {
            return fcs;
        }

        if totalDstSize.wrapping_add(fcs) < totalDstSize {
            return ZSTD_CONTENTSIZE_ERROR; /* check for overflow */
        }
        totalDstSize = totalDstSize.wrapping_add(fcs);

        /* skip to next frame */
        let frameSrcSize = ZSTD_findFrameCompressedSize(src, srcSize);
        if ZSTD_isError(frameSrcSize) != 0 {
            return ZSTD_CONTENTSIZE_ERROR;
        }

        src = (src as *const BYTE).add(frameSrcSize) as *const c_void;
        srcSize -= frameSrcSize;
    }

    if srcSize != 0 {
        return ZSTD_CONTENTSIZE_ERROR;
    }

    totalDstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDecompressedSize(
    src: *const c_void,
    srcSize: usize,
) -> c_ulonglong {
    let ret = ZSTD_getFrameContentSize(src, srcSize);
    if ret >= ZSTD_CONTENTSIZE_ERROR {
        0
    } else {
        ret
    }
}

unsafe fn ZSTD_decodeFrameHeader(dctx: *mut ZSTD_DCtx, src: *const c_void, headerSize: usize) -> usize {
    let result = ZSTD_getFrameHeader_advanced(&mut (*dctx).fParams, src, headerSize, (*dctx).format);
    if ZSTD_isError(result) != 0 {
        return result; /* invalid header */
    }
    if result > 0 {
        return error(code::SRCSIZE_WRONG);
    }

    /* Reference DDict requested by frame if dctx references multiple ddicts */
    if (*dctx).refMultipleDDicts == ZSTD_rmd_refMultipleDDicts && !(*dctx).ddictSet.is_null() {
        ZSTD_DCtx_selectFrameDDict(dctx);
    }

    if (*dctx).fParams.dictID != 0 && ((*dctx).dictID != (*dctx).fParams.dictID) {
        return error(code::DICTIONARY_WRONG);
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
    (*dctx).processedCSize = (*dctx).processedCSize.wrapping_add(headerSize as u64);
    0
}

fn ZSTD_errorFrameSizeInfo(ret: usize) -> ZSTD_frameSizeInfo {
    ZSTD_frameSizeInfo {
        nbBlocks: 0,
        compressedSize: ret,
        decompressedBound: ZSTD_CONTENTSIZE_ERROR,
    }
}

unsafe fn ZSTD_findFrameSizeInfo(
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = core::mem::zeroed();

    if format == ZSTD_f_zstd1 && ZSTD_isLegacy(src, srcSize) != 0 {
        return ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    }

    if format == ZSTD_f_zstd1
        && (srcSize >= ZSTD_SKIPPABLEHEADERSIZE)
        && (mem_read_le32(src) & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START
    {
        frameSizeInfo.compressedSize = readSkippableFrameSize(src, srcSize);
        return frameSizeInfo;
    } else {
        let ipstart = src as *const BYTE;
        let mut ip = ipstart;
        let mut remainingSize = srcSize;
        let mut nbBlocks: usize = 0;
        let mut zfh: ZSTD_FrameHeader = core::mem::zeroed();

        /* Extract Frame Header */
        let ret = ZSTD_getFrameHeader_advanced(&mut zfh, src, srcSize, format);
        if ZSTD_isError(ret) != 0 {
            return ZSTD_errorFrameSizeInfo(ret);
        }
        if ret > 0 {
            return ZSTD_errorFrameSizeInfo(error(code::SRCSIZE_WRONG));
        }

        ip = ip.add(zfh.headerSize as usize);
        remainingSize -= zfh.headerSize as usize;

        /* Iterate over each block */
        loop {
            let mut blockProperties: blockProperties_t = core::mem::zeroed();
            let cBlockSize =
                ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
            if ZSTD_isError(cBlockSize) != 0 {
                return ZSTD_errorFrameSizeInfo(cBlockSize);
            }

            if ZSTD_blockHeaderSize + cBlockSize > remainingSize {
                return ZSTD_errorFrameSizeInfo(error(code::SRCSIZE_WRONG));
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
                return ZSTD_errorFrameSizeInfo(error(code::SRCSIZE_WRONG));
            }
            ip = ip.add(4);
        }

        frameSizeInfo.nbBlocks = nbBlocks;
        frameSizeInfo.compressedSize = ip.offset_from(ipstart) as usize;
        frameSizeInfo.decompressedBound = if zfh.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
            zfh.frameContentSize
        } else {
            (nbBlocks as c_ulonglong) * (zfh.blockSizeMax as c_ulonglong)
        };
        return frameSizeInfo;
    }
}

unsafe fn ZSTD_findFrameCompressedSize_advanced(
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> usize {
    let frameSizeInfo = ZSTD_findFrameSizeInfo(src, srcSize, format);
    frameSizeInfo.compressedSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findFrameCompressedSize(src: *const c_void, srcSize: usize) -> usize {
    ZSTD_findFrameCompressedSize_advanced(src, srcSize, ZSTD_f_zstd1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBound(src: *const c_void, srcSize: usize) -> c_ulonglong {
    let mut bound: c_ulonglong = 0;
    let mut src = src;
    let mut srcSize = srcSize;
    /* Iterate over each frame */
    while srcSize > 0 {
        let frameSizeInfo = ZSTD_findFrameSizeInfo(src, srcSize, ZSTD_f_zstd1);
        let compressedSize = frameSizeInfo.compressedSize;
        let decompressedBound = frameSizeInfo.decompressedBound;
        if ZSTD_isError(compressedSize) != 0 || decompressedBound == ZSTD_CONTENTSIZE_ERROR {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        src = (src as *const BYTE).add(compressedSize) as *const c_void;
        srcSize -= compressedSize;
        bound = bound.wrapping_add(decompressedBound);
    }
    bound
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressionMargin(src: *const c_void, srcSize: usize) -> usize {
    let mut margin: usize = 0;
    let mut maxBlockSize: c_uint = 0;
    let mut src = src;
    let mut srcSize = srcSize;

    /* Iterate over each frame */
    while srcSize > 0 {
        let frameSizeInfo = ZSTD_findFrameSizeInfo(src, srcSize, ZSTD_f_zstd1);
        let compressedSize = frameSizeInfo.compressedSize;
        let decompressedBound = frameSizeInfo.decompressedBound;
        let mut zfh: ZSTD_FrameHeader = core::mem::zeroed();

        let e = ZSTD_getFrameHeader(&mut zfh, src, srcSize);
        if ZSTD_isError(e) != 0 {
            return e;
        }
        if ZSTD_isError(compressedSize) != 0 || decompressedBound == ZSTD_CONTENTSIZE_ERROR {
            return error(code::CORRUPTION_DETECTED);
        }

        if zfh.frameType == ZSTD_frame {
            /* Add the frame header to our margin */
            margin += zfh.headerSize as usize;
            /* Add the checksum to our margin */
            margin += if zfh.checksumFlag != 0 { 4 } else { 0 };
            /* Add 3 bytes per block */
            margin += 3 * frameSizeInfo.nbBlocks;

            /* Compute the max block size */
            if zfh.blockSizeMax > maxBlockSize {
                maxBlockSize = zfh.blockSizeMax;
            }
        } else {
            debug_assert!(zfh.frameType == ZSTD_skippableFrame);
            /* Add the entire skippable frame size to our margin. */
            margin += compressedSize;
        }

        src = (src as *const BYTE).add(compressedSize) as *const c_void;
        srcSize -= compressedSize;
    }

    /* Add the max block size back to the margin. */
    margin += maxBlockSize as usize;

    margin
}

/*-*************************************************************
 *   Legacy support dispatch (ZSTD_LEGACY_SUPPORT == 5)
 *   translated from legacy/zstd_legacy.h
 ***************************************************************/

const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525;
const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526;
const ZSTDv07_MAGICNUMBER: U32 = 0xFD2FB527;

/* Local copies of legacy frameParams structs matching the legacy modules'
 * private layouts (needed to call getFrameParams). */
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTDv05_parameters_local {
    srcSize: U64,
    windowLog: U32,
    contentLog: U32,
    hashLog: U32,
    searchLog: U32,
    searchLength: U32,
    targetLength: U32,
    strategy: U32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTDv06_frameParams_local {
    frameContentSize: U64,
    windowLog: c_uint,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTDv07_frameParams_local {
    frameContentSize: c_ulonglong,
    windowSize: c_uint,
    dictID: c_uint,
    checksumFlag: c_uint,
}

unsafe fn ZSTD_isLegacy(src: *const c_void, srcSize: usize) -> c_uint {
    if srcSize < 4 {
        return 0;
    }
    let magicNumberLE = mem_read_le32(src);
    match magicNumberLE {
        ZSTDv05_MAGICNUMBER => 5,
        ZSTDv06_MAGICNUMBER => 6,
        ZSTDv07_MAGICNUMBER => 7,
        _ => 0,
    }
}

unsafe fn ZSTD_getDecompressedSize_legacy(src: *const c_void, srcSize: usize) -> c_ulonglong {
    let version = ZSTD_isLegacy(src, srcSize);
    if version < 5 {
        return 0;
    }
    if version == 5 {
        let mut fParams: ZSTDv05_parameters_local = core::mem::zeroed();
        let frResult = ZSTDv05_getFrameParams(
            &mut fParams as *mut ZSTDv05_parameters_local as *mut _,
            src,
            srcSize,
        );
        if frResult != 0 {
            return 0;
        }
        return fParams.srcSize;
    }
    if version == 6 {
        let mut fParams: ZSTDv06_frameParams_local = core::mem::zeroed();
        let frResult = ZSTDv06_getFrameParams(
            &mut fParams as *mut ZSTDv06_frameParams_local as *mut _,
            src,
            srcSize,
        );
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    if version == 7 {
        let mut fParams: ZSTDv07_frameParams_local = core::mem::zeroed();
        let frResult = ZSTDv07_getFrameParams(
            &mut fParams as *mut ZSTDv07_frameParams_local as *mut _,
            src,
            srcSize,
        );
        if frResult != 0 {
            return 0;
        }
        return fParams.frameContentSize;
    }
    0 /* should not be possible */
}

unsafe fn ZSTD_decompressLegacy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    compressedSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let version = ZSTD_isLegacy(src, compressedSize);
    let mut x: core::ffi::c_char = 0;
    let mut dst = dst;
    let mut src = src;
    let mut dict = dict;
    /* Avoid passing NULL to legacy decoding. */
    if dst.is_null() {
        debug_assert!(dstCapacity == 0);
        dst = &mut x as *mut core::ffi::c_char as *mut c_void;
    }
    if src.is_null() {
        debug_assert!(compressedSize == 0);
        src = &x as *const core::ffi::c_char as *const c_void;
    }
    if dict.is_null() {
        debug_assert!(dictSize == 0);
        dict = &x as *const core::ffi::c_char as *const c_void;
    }
    match version {
        5 => {
            let zd = ZSTDv05_createDCtx();
            if zd.is_null() {
                return error(code::MEMORY_ALLOCATION);
            }
            let result =
                ZSTDv05_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv05_freeDCtx(zd);
            result
        }
        6 => {
            let zd = ZSTDv06_createDCtx();
            if zd.is_null() {
                return error(code::MEMORY_ALLOCATION);
            }
            let result =
                ZSTDv06_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv06_freeDCtx(zd);
            result
        }
        7 => {
            let zd = ZSTDv07_createDCtx();
            if zd.is_null() {
                return error(code::MEMORY_ALLOCATION);
            }
            let result =
                ZSTDv07_decompress_usingDict(zd, dst, dstCapacity, src, compressedSize, dict, dictSize);
            ZSTDv07_freeDCtx(zd);
            result
        }
        _ => error(code::PREFIX_UNKNOWN),
    }
}

unsafe fn ZSTD_findFrameSizeInfoLegacy(src: *const c_void, srcSize: usize) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = core::mem::zeroed();
    let version = ZSTD_isLegacy(src, srcSize);
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
            frameSizeInfo.compressedSize = error(code::PREFIX_UNKNOWN);
            frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
        }
    }
    if ZSTD_isError(frameSizeInfo.compressedSize) == 0 && frameSizeInfo.compressedSize > srcSize {
        frameSizeInfo.compressedSize = error(code::SRCSIZE_WRONG);
        frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
    }
    /* In all cases, decompressedBound == nbBlocks * ZSTD_BLOCKSIZE_MAX. */
    if frameSizeInfo.decompressedBound != ZSTD_CONTENTSIZE_ERROR {
        frameSizeInfo.nbBlocks =
            (frameSizeInfo.decompressedBound / (ZSTD_BLOCKSIZE_MAX as u64)) as usize;
    }
    frameSizeInfo
}

unsafe fn ZSTD_findFrameCompressedSizeLegacy(src: *const c_void, srcSize: usize) -> usize {
    let frameSizeInfo = ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    frameSizeInfo.compressedSize
}

unsafe fn ZSTD_freeLegacyStreamContext(legacyContext: *mut c_void, version: U32) -> usize {
    match version {
        5 => ZBUFFv05_freeDCtx(legacyContext as *mut _),
        6 => ZBUFFv06_freeDCtx(legacyContext as *mut _),
        7 => ZBUFFv07_freeDCtx(legacyContext as *mut _),
        _ => error(code::VERSION_UNSUPPORTED),
    }
}

unsafe fn ZSTD_initLegacyStream(
    legacyContext: *mut *mut c_void,
    prevVersion: U32,
    newVersion: U32,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut x: core::ffi::c_char = 0;
    let mut dict = dict;
    /* Avoid passing NULL to legacy decoding. */
    if dict.is_null() {
        debug_assert!(dictSize == 0);
        dict = &x as *const core::ffi::c_char as *const c_void;
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
                return error(code::MEMORY_ALLOCATION);
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
                return error(code::MEMORY_ALLOCATION);
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
                return error(code::MEMORY_ALLOCATION);
            }
            ZBUFFv07_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx as *mut c_void;
            0
        }
        _ => 0,
    }
}

unsafe fn ZSTD_decompressLegacyStream(
    legacyContext: *mut c_void,
    version: U32,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    static mut X: core::ffi::c_char = 0;
    /* Avoid passing NULL to legacy decoding. */
    if (*output).dst.is_null() {
        debug_assert!((*output).size == 0);
        (*output).dst = core::ptr::addr_of_mut!(X) as *mut c_void;
    }
    if (*input).src.is_null() {
        debug_assert!((*input).size == 0);
        (*input).src = core::ptr::addr_of!(X) as *const c_void;
    }
    match version {
        5 => {
            let dctx = legacyContext as *mut _;
            let src = ((*input).src as *const core::ffi::c_char).add((*input).pos) as *const c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut core::ffi::c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize =
                ZBUFFv05_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        6 => {
            let dctx = legacyContext as *mut _;
            let src = ((*input).src as *const core::ffi::c_char).add((*input).pos) as *const c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut core::ffi::c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize =
                ZBUFFv06_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        7 => {
            let dctx = legacyContext as *mut _;
            let src = ((*input).src as *const core::ffi::c_char).add((*input).pos) as *const c_void;
            let mut readSize = (*input).size - (*input).pos;
            let dst = ((*output).dst as *mut core::ffi::c_char).add((*output).pos) as *mut c_void;
            let mut decodedSize = (*output).size - (*output).pos;
            let hintSize =
                ZBUFFv07_decompressContinue(dctx, dst, &mut decodedSize, src, &mut readSize);
            (*output).pos += decodedSize;
            (*input).pos += readSize;
            hintSize
        }
        _ => error(code::VERSION_UNSUPPORTED),
    }
}

/*-*************************************************************
 *   Frame decoding
 ***************************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_insertBlock(
    dctx: *mut ZSTD_DCtx,
    blockStart: *const c_void,
    blockSize: usize,
) -> usize {
    ZSTD_checkContinuity(dctx, blockStart, blockSize);
    (*dctx).previousDstEnd = (blockStart as *const core::ffi::c_char).add(blockSize) as *const c_void;
    blockSize
}

unsafe fn ZSTD_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize > dstCapacity {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if dst.is_null() {
        if srcSize == 0 {
            return 0;
        }
        return error(code::DSTBUFFER_NULL);
    }
    core::ptr::copy(src as *const u8, dst as *mut u8, srcSize);
    srcSize
}

unsafe fn ZSTD_setRleBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    b: BYTE,
    regenSize: usize,
) -> usize {
    if regenSize > dstCapacity {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if dst.is_null() {
        if regenSize == 0 {
            return 0;
        }
        return error(code::DSTBUFFER_NULL);
    }
    core::ptr::write_bytes(dst as *mut u8, b, regenSize);
    regenSize
}

unsafe fn ZSTD_DCtx_trace_end(
    _dctx: *const ZSTD_DCtx,
    _uncompressedSize: U64,
    _compressedSize: U64,
    _streaming: c_int,
) {
    /* ZSTD_TRACE == 0 */
}

unsafe fn ZSTD_decompressFrame(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    srcPtr: *mut *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart = *srcPtr as *const BYTE;
    let mut ip = istart;
    let ostart = dst as *mut BYTE;
    let oend = if dstCapacity != 0 {
        ostart.add(dstCapacity)
    } else {
        ostart
    };
    let mut op = ostart;
    let mut remainingSrcSize = *srcSizePtr;

    /* check */
    if remainingSrcSize < ZSTD_FRAMEHEADERSIZE_MIN((*dctx).format) + ZSTD_blockHeaderSize {
        return error(code::SRCSIZE_WRONG);
    }

    /* Frame Header */
    {
        let frameHeaderSize = ZSTD_frameHeaderSize_internal(
            ip as *const c_void,
            ZSTD_FRAMEHEADERSIZE_PREFIX((*dctx).format),
            (*dctx).format,
        );
        if ZSTD_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if remainingSrcSize < frameHeaderSize + ZSTD_blockHeaderSize {
            return error(code::SRCSIZE_WRONG);
        }
        let e = ZSTD_decodeFrameHeader(dctx, ip as *const c_void, frameHeaderSize);
        if ZSTD_isError(e) != 0 {
            return e;
        }
        ip = ip.add(frameHeaderSize);
        remainingSrcSize -= frameHeaderSize;
    }

    /* Shrink the blockSizeMax if enabled */
    if (*dctx).maxBlockSizeParam != 0 {
        (*dctx).fParams.blockSizeMax = MIN_usize(
            (*dctx).fParams.blockSizeMax as usize,
            (*dctx).maxBlockSizeParam as usize,
        ) as u32;
    }

    /* Loop on each block */
    loop {
        let mut oBlockEnd = oend;
        let decodedSize: usize;
        let mut blockProperties: blockProperties_t = core::mem::zeroed();
        let cBlockSize =
            ZSTD_getcBlockSize(ip as *const c_void, remainingSrcSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSrcSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSrcSize {
            return error(code::SRCSIZE_WRONG);
        }

        if ip >= op && ip < oBlockEnd {
            /* We are decompressing in-place. Limit the output pointer. */
            oBlockEnd = op.add(ip.offset_from(op) as usize);
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                debug_assert!((*dctx).isFrameDecompression == 1);
                decodedSize = ZSTD_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    oBlockEnd.offset_from(op) as usize,
                    ip as *const c_void,
                    cBlockSize,
                    not_streaming,
                );
            }
            x if x == bt_raw => {
                /* Use oend instead of oBlockEnd because this is safe to overlap. */
                decodedSize = ZSTD_copyRawBlock(
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_rle => {
                decodedSize = ZSTD_setRleBlock(
                    op as *mut c_void,
                    oBlockEnd.offset_from(op) as usize,
                    *ip,
                    blockProperties.origSize as usize,
                );
            }
            _ => {
                /* bt_reserved and default */
                return error(code::CORRUPTION_DETECTED);
            }
        }
        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        if (*dctx).validateChecksum != 0 {
            ZSTD_XXH64_update(&mut (*dctx).xxhState, op as *const c_void, decodedSize);
        }
        if decodedSize != 0 {
            /* support dst = NULL,0 */
            op = op.add(decodedSize);
        }
        debug_assert!(!ip.is_null());
        ip = ip.add(cBlockSize);
        remainingSrcSize -= cBlockSize;
        if blockProperties.lastBlock != 0 {
            break;
        }
    }

    if (*dctx).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
        if (op.offset_from(ostart) as u64) != (*dctx).fParams.frameContentSize {
            return error(code::CORRUPTION_DETECTED);
        }
    }
    if (*dctx).fParams.checksumFlag != 0 {
        /* Frame content checksum verification */
        if remainingSrcSize < 4 {
            return error(code::CHECKSUM_WRONG);
        }
        if (*dctx).forceIgnoreChecksum == 0 {
            let checkCalc = ZSTD_XXH64_digest(&(*dctx).xxhState) as U32;
            let checkRead = mem_read_le32(ip as *const c_void);
            if checkRead != checkCalc {
                return error(code::CHECKSUM_WRONG);
            }
        }
        ip = ip.add(4);
        remainingSrcSize -= 4;
    }
    ZSTD_DCtx_trace_end(
        dctx,
        op.offset_from(ostart) as U64,
        ip.offset_from(istart) as U64,
        0,
    );
    /* Allow caller to get size read */
    *srcPtr = ip as *const c_void;
    *srcSizePtr = remainingSrcSize;
    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_decompressMultiFrame(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
    ddict: *const ZSTD_DDict,
) -> usize {
    let dststart = dst;
    let mut moreThan1Frame: c_int = 0;
    let mut dst = dst;
    let mut dstCapacity = dstCapacity;
    let mut src = src;
    let mut srcSize = srcSize;
    let mut dict = dict;
    let mut dictSize = dictSize;

    debug_assert!(dict.is_null() || ddict.is_null());

    if !ddict.is_null() {
        dict = ZSTD_DDict_dictContent(ddict);
        dictSize = ZSTD_DDict_dictSize(ddict);
    }

    while srcSize >= ZSTD_startingInputLength((*dctx).format) {
        if (*dctx).format == ZSTD_f_zstd1 && ZSTD_isLegacy(src, srcSize) != 0 {
            let frameSize = ZSTD_findFrameCompressedSizeLegacy(src, srcSize);
            if ZSTD_isError(frameSize) != 0 {
                return frameSize;
            }
            if (*dctx).staticSize != 0 {
                return error(code::MEMORY_ALLOCATION);
            }

            let decodedSize =
                ZSTD_decompressLegacy(dst, dstCapacity, src, frameSize, dict, dictSize);
            if ZSTD_isError(decodedSize) != 0 {
                return decodedSize;
            }

            {
                let expectedSize = ZSTD_getFrameContentSize(src, srcSize);
                if expectedSize == ZSTD_CONTENTSIZE_ERROR {
                    return error(code::CORRUPTION_DETECTED);
                }
                if expectedSize != ZSTD_CONTENTSIZE_UNKNOWN {
                    if expectedSize != decodedSize as u64 {
                        return error(code::CORRUPTION_DETECTED);
                    }
                }
            }

            dst = (dst as *mut BYTE).add(decodedSize) as *mut c_void;
            dstCapacity -= decodedSize;

            src = (src as *const BYTE).add(frameSize) as *const c_void;
            srcSize -= frameSize;

            continue;
        }

        if (*dctx).format == ZSTD_f_zstd1 && srcSize >= 4 {
            let magicNumber = mem_read_le32(src);
            if (magicNumber & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
                /* skippable frame detected : skip it */
                let skippableSize = readSkippableFrameSize(src, srcSize);
                if ZSTD_isError(skippableSize) != 0 {
                    return skippableSize;
                }

                src = (src as *const BYTE).add(skippableSize) as *const c_void;
                srcSize -= skippableSize;
                continue; /* check next frame */
            }
        }

        if !ddict.is_null() {
            /* we were called from ZSTD_decompress_usingDDict */
            let e = ZSTD_decompressBegin_usingDDict(dctx, ddict);
            if ZSTD_isError(e) != 0 {
                return e;
            }
        } else {
            let e = ZSTD_decompressBegin_usingDict(dctx, dict, dictSize);
            if ZSTD_isError(e) != 0 {
                return e;
            }
        }
        ZSTD_checkContinuity(dctx, dst, dstCapacity);

        {
            let res = ZSTD_decompressFrame(dctx, dst, dstCapacity, &mut src, &mut srcSize);
            if ZSTD_getErrorCode(res) == ZSTD_error_prefix_unknown && moreThan1Frame == 1 {
                return error(code::SRCSIZE_WRONG);
            }
            if ZSTD_isError(res) != 0 {
                return res;
            }
            if res != 0 {
                dst = (dst as *mut BYTE).add(res) as *mut c_void;
            }
            dstCapacity -= res;
        }
        moreThan1Frame = 1;
    }

    if srcSize != 0 {
        return error(code::SRCSIZE_WRONG);
    }

    (dst as *mut BYTE).offset_from(dststart as *mut BYTE) as usize
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

unsafe fn ZSTD_getDDict(dctx: *mut ZSTD_DCtx) -> *const ZSTD_DDict {
    match (*dctx).dictUses {
        ZSTD_dont_use => {
            ZSTD_clearDict(dctx);
            core::ptr::null()
        }
        ZSTD_use_indefinitely => (*dctx).ddict,
        ZSTD_use_once => {
            (*dctx).dictUses = ZSTD_dont_use;
            (*dctx).ddict
        }
        _ => {
            debug_assert!(false); /* Impossible */
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
    /* ZSTD_HEAPMODE == 1 */
    let dctx = ZSTD_createDCtx_internal(ZSTD_defaultCMem);
    if dctx.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    let regenSize = ZSTD_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
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

unsafe fn ZSTD_nextSrcSizeToDecompressWithInputSize(
    dctx: *mut ZSTD_DCtx,
    inputSize: usize,
) -> usize {
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
        x if x == ZSTDds_getFrameHeaderSize => ZSTDnit_frameHeader,
        x if x == ZSTDds_decodeFrameHeader => ZSTDnit_frameHeader,
        x if x == ZSTDds_decodeBlockHeader => ZSTDnit_blockHeader,
        x if x == ZSTDds_decompressBlock => ZSTDnit_block,
        x if x == ZSTDds_decompressLastBlock => ZSTDnit_lastBlock,
        x if x == ZSTDds_checkChecksum => ZSTDnit_checksum,
        x if x == ZSTDds_decodeSkippableHeader => ZSTDnit_skippableFrame,
        x if x == ZSTDds_skipFrame => ZSTDnit_skippableFrame,
        _ => {
            debug_assert!(false);
            ZSTDnit_frameHeader
        }
    }
}

unsafe fn ZSTD_isSkipFrame(dctx: *mut ZSTD_DCtx) -> c_int {
    ((*dctx).stage == ZSTDds_skipFrame) as c_int
}

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
        return error(code::SRCSIZE_WRONG);
    }
    ZSTD_checkContinuity(dctx, dst, dstCapacity);

    (*dctx).processedCSize = (*dctx).processedCSize.wrapping_add(srcSize as u64);

    match (*dctx).stage {
        x if x == ZSTDds_getFrameHeaderSize => {
            debug_assert!(!src.is_null());
            if (*dctx).format == ZSTD_f_zstd1 {
                /* allows header */
                debug_assert!(srcSize >= ZSTD_FRAMEIDSIZE);
                if (mem_read_le32(src) & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
                    /* skippable frame */
                    core::ptr::copy_nonoverlapping(
                        src as *const u8,
                        (*dctx).headerBuffer.as_mut_ptr(),
                        srcSize,
                    );
                    (*dctx).expected = ZSTD_SKIPPABLEHEADERSIZE - srcSize;
                    (*dctx).stage = ZSTDds_decodeSkippableHeader;
                    return 0;
                }
            }
            (*dctx).headerSize = ZSTD_frameHeaderSize_internal(src, srcSize, (*dctx).format);
            if ZSTD_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            core::ptr::copy_nonoverlapping(
                src as *const u8,
                (*dctx).headerBuffer.as_mut_ptr(),
                srcSize,
            );
            (*dctx).expected = (*dctx).headerSize - srcSize;
            (*dctx).stage = ZSTDds_decodeFrameHeader;
            0
        }

        x if x == ZSTDds_decodeFrameHeader => {
            debug_assert!(!src.is_null());
            core::ptr::copy_nonoverlapping(
                src as *const u8,
                (*dctx).headerBuffer.as_mut_ptr().add((*dctx).headerSize - srcSize),
                srcSize,
            );
            let e = ZSTD_decodeFrameHeader(
                dctx,
                (*dctx).headerBuffer.as_ptr() as *const c_void,
                (*dctx).headerSize,
            );
            if ZSTD_isError(e) != 0 {
                return e;
            }
            (*dctx).expected = ZSTD_blockHeaderSize;
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            0
        }

        x if x == ZSTDds_decodeBlockHeader => {
            let mut bp: blockProperties_t = core::mem::zeroed();
            let cBlockSize = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
            if ZSTD_isError(cBlockSize) != 0 {
                return cBlockSize;
            }
            if cBlockSize > (*dctx).fParams.blockSizeMax as usize {
                return error(code::CORRUPTION_DETECTED);
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
            0
        }

        x if x == ZSTDds_decompressLastBlock || x == ZSTDds_decompressBlock => {
            let rSize: usize;
            match (*dctx).bType {
                b if b == bt_compressed => {
                    debug_assert!((*dctx).isFrameDecompression == 1);
                    rSize = ZSTD_decompressBlock_internal(
                        dctx, dst, dstCapacity, src, srcSize, is_streaming,
                    );
                    (*dctx).expected = 0; /* Streaming not supported */
                }
                b if b == bt_raw => {
                    debug_assert!(srcSize <= (*dctx).expected);
                    rSize = ZSTD_copyRawBlock(dst, dstCapacity, src, srcSize);
                    if ZSTD_isError(rSize) != 0 {
                        return rSize;
                    }
                    debug_assert!(rSize == srcSize);
                    (*dctx).expected -= rSize;
                }
                b if b == bt_rle => {
                    rSize = ZSTD_setRleBlock(dst, dstCapacity, *(src as *const BYTE), (*dctx).rleSize);
                    (*dctx).expected = 0; /* Streaming not supported */
                }
                _ => {
                    /* bt_reserved and default */
                    return error(code::CORRUPTION_DETECTED);
                }
            }
            if ZSTD_isError(rSize) != 0 {
                return rSize;
            }
            if rSize > (*dctx).fParams.blockSizeMax as usize {
                return error(code::CORRUPTION_DETECTED);
            }
            (*dctx).decodedSize = (*dctx).decodedSize.wrapping_add(rSize as u64);
            if (*dctx).validateChecksum != 0 {
                ZSTD_XXH64_update(&mut (*dctx).xxhState, dst as *const c_void, rSize);
            }
            (*dctx).previousDstEnd = (dst as *const core::ffi::c_char).add(rSize) as *const c_void;

            /* Stay on the same stage until we are finished streaming the block. */
            if (*dctx).expected > 0 {
                return rSize;
            }

            if (*dctx).stage == ZSTDds_decompressLastBlock {
                /* end of frame */
                if (*dctx).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
                    && (*dctx).decodedSize != (*dctx).fParams.frameContentSize
                {
                    return error(code::CORRUPTION_DETECTED);
                }
                if (*dctx).fParams.checksumFlag != 0 {
                    /* another round for frame checksum */
                    (*dctx).expected = 4;
                    (*dctx).stage = ZSTDds_checkChecksum;
                } else {
                    ZSTD_DCtx_trace_end(dctx, (*dctx).decodedSize, (*dctx).processedCSize, 1);
                    (*dctx).expected = 0; /* ends here */
                    (*dctx).stage = ZSTDds_getFrameHeaderSize;
                }
            } else {
                (*dctx).stage = ZSTDds_decodeBlockHeader;
                (*dctx).expected = ZSTD_blockHeaderSize;
            }
            rSize
        }

        x if x == ZSTDds_checkChecksum => {
            debug_assert!(srcSize == 4); /* guaranteed by dctx->expected */
            if (*dctx).validateChecksum != 0 {
                let h32 = ZSTD_XXH64_digest(&(*dctx).xxhState) as U32;
                let check32 = mem_read_le32(src);
                if check32 != h32 {
                    return error(code::CHECKSUM_WRONG);
                }
            }
            ZSTD_DCtx_trace_end(dctx, (*dctx).decodedSize, (*dctx).processedCSize, 1);
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            0
        }

        x if x == ZSTDds_decodeSkippableHeader => {
            debug_assert!(!src.is_null());
            debug_assert!(srcSize <= ZSTD_SKIPPABLEHEADERSIZE);
            debug_assert!((*dctx).format != ZSTD_f_zstd1_magicless);
            core::ptr::copy_nonoverlapping(
                src as *const u8,
                (*dctx)
                    .headerBuffer
                    .as_mut_ptr()
                    .add(ZSTD_SKIPPABLEHEADERSIZE - srcSize),
                srcSize,
            ); /* complete skippable header */
            (*dctx).expected = mem_read_le32(
                (*dctx).headerBuffer.as_ptr().add(ZSTD_FRAMEIDSIZE) as *const c_void,
            ) as usize;
            (*dctx).stage = ZSTDds_skipFrame;
            0
        }

        x if x == ZSTDds_skipFrame => {
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            0
        }

        _ => {
            debug_assert!(false); /* impossible */
            error(code::GENERIC)
        }
    }
}

unsafe fn ZSTD_refDictContent(dctx: *mut ZSTD_DCtx, dict: *const c_void, dictSize: usize) -> usize {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).virtualStart = (dict as *const core::ffi::c_char).offset(
        -(((*dctx).previousDstEnd as *const core::ffi::c_char)
            .offset_from((*dctx).prefixStart as *const core::ffi::c_char)),
    ) as *const c_void;
    (*dctx).prefixStart = dict;
    (*dctx).previousDstEnd = (dict as *const core::ffi::c_char).add(dictSize) as *const c_void;
    0
}

unsafe fn ZSTD_decompress_insertDictionary(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut dict = dict;
    let mut dictSize = dictSize;
    if dictSize < 8 {
        return ZSTD_refDictContent(dctx, dict, dictSize);
    }
    {
        let magic = mem_read_le32(dict);
        if magic != ZSTD_MAGIC_DICTIONARY {
            return ZSTD_refDictContent(dctx, dict, dictSize); /* pure content mode */
        }
    }
    (*dctx).dictID =
        mem_read_le32((dict as *const core::ffi::c_char).add(ZSTD_FRAMEIDSIZE) as *const c_void);

    /* load entropy tables */
    {
        let eSize = ZSTD_loadDEntropy(&mut (*dctx).entropy, dict, dictSize);
        if ZSTD_isError(eSize) != 0 {
            return error(code::DICTIONARY_CORRUPTED);
        }
        dict = (dict as *const core::ffi::c_char).add(eSize) as *const c_void;
        dictSize -= eSize;
    }
    (*dctx).litEntropy = 1;
    (*dctx).fseEntropy = 1;

    /* reference dictionary content */
    ZSTD_refDictContent(dctx, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin(dctx: *mut ZSTD_DCtx) -> usize {
    debug_assert!(!dctx.is_null());
    (*dctx).expected = ZSTD_startingInputLength((*dctx).format); /* dctx->format must be properly set */
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).processedCSize = 0;
    (*dctx).decodedSize = 0;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).prefixStart = core::ptr::null();
    (*dctx).virtualStart = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).entropy.hufTable[0] =
        (ZSTD_HUFFDTABLE_CAPACITY_LOG).wrapping_mul(0x1000001) as HUF_DTable; /* cover both little and big endian */
    (*dctx).litEntropy = 0;
    (*dctx).fseEntropy = 0;
    (*dctx).dictID = 0;
    (*dctx).bType = bt_reserved;
    (*dctx).isFrameDecompression = 1;
    core::ptr::copy_nonoverlapping(
        repStartValue.as_ptr(),
        (*dctx).entropy.rep.as_mut_ptr(),
        repStartValue.len(),
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
    dictSize: usize,
) -> usize {
    let e = ZSTD_decompressBegin(dctx);
    if ZSTD_isError(e) != 0 {
        return e;
    }
    if !dict.is_null() && dictSize != 0 {
        if ZSTD_isError(ZSTD_decompress_insertDictionary(dctx, dict, dictSize)) != 0 {
            return error(code::DICTIONARY_CORRUPTED);
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
    debug_assert!(!dctx.is_null());
    if !ddict.is_null() {
        let dictStart = ZSTD_DDict_dictContent(ddict) as *const core::ffi::c_char;
        let dictSize = ZSTD_DDict_dictSize(ddict);
        let dictEnd = dictStart.add(dictSize) as *const c_void;
        (*dctx).ddictIsCold = ((*dctx).dictEnd != dictEnd) as i32;
    }
    let e = ZSTD_decompressBegin(dctx);
    if ZSTD_isError(e) != 0 {
        return e;
    }
    if !ddict.is_null() {
        /* NULL ddict is equivalent to no dictionary */
        ZSTD_copyDDictParameters(dctx, ddict);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDict(dict: *const c_void, dictSize: usize) -> c_uint {
    if dictSize < 8 {
        return 0;
    }
    if mem_read_le32(dict) != ZSTD_MAGIC_DICTIONARY {
        return 0;
    }
    mem_read_le32((dict as *const core::ffi::c_char).add(ZSTD_FRAMEIDSIZE) as *const c_void)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromFrame(src: *const c_void, srcSize: usize) -> c_uint {
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
    let hError = ZSTD_getFrameHeader(&mut zfp, src, srcSize);
    if ZSTD_isError(hError) != 0 {
        return 0;
    }
    zfp.dictID
}

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
pub unsafe extern "C" fn ZSTD_createDStream() -> *mut ZSTD_DStream {
    ZSTD_createDCtx_internal(ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DStream {
    ZSTD_initStaticDCtx(workspace, workspaceSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_DStream {
    ZSTD_createDCtx_internal(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDStream(zds: *mut ZSTD_DStream) -> usize {
    ZSTD_freeDCtx(zds)
}

/* ***  Initialization  *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DStreamInSize() -> usize {
    ZSTD_BLOCKSIZE_MAX + ZSTD_blockHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DStreamOutSize() -> usize {
    ZSTD_BLOCKSIZE_MAX
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
        return error(code::STAGE_WRONG);
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
            return error(code::MEMORY_ALLOCATION);
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
    let e = ZSTD_DCtx_loadDictionary_advanced(
        dctx,
        prefix,
        prefixSize,
        ZSTD_dlm_byRef,
        dictContentType,
    );
    if ZSTD_isError(e) != 0 {
        return e;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDict(
    zds: *mut ZSTD_DStream,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let e = ZSTD_DCtx_reset(zds, ZSTD_reset_session_only);
    if ZSTD_isError(e) != 0 {
        return e;
    }
    let e = ZSTD_DCtx_loadDictionary(zds, dict, dictSize);
    if ZSTD_isError(e) != 0 {
        return e;
    }
    ZSTD_startingInputLength((*zds).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream(zds: *mut ZSTD_DStream) -> usize {
    let e = ZSTD_DCtx_reset(zds, ZSTD_reset_session_only);
    if ZSTD_isError(e) != 0 {
        return e;
    }
    let e = ZSTD_DCtx_refDDict(zds, core::ptr::null());
    if ZSTD_isError(e) != 0 {
        return e;
    }
    ZSTD_startingInputLength((*zds).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDDict(
    dctx: *mut ZSTD_DStream,
    ddict: *const ZSTD_DDict,
) -> usize {
    let e = ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only);
    if ZSTD_isError(e) != 0 {
        return e;
    }
    let e = ZSTD_DCtx_refDDict(dctx, ddict);
    if ZSTD_isError(e) != 0 {
        return e;
    }
    ZSTD_startingInputLength((*dctx).format)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetDStream(dctx: *mut ZSTD_DStream) -> usize {
    let e = ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only);
    if ZSTD_isError(e) != 0 {
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
        return error(code::STAGE_WRONG);
    }
    ZSTD_clearDict(dctx);
    if !ddict.is_null() {
        (*dctx).ddict = ddict;
        (*dctx).dictUses = ZSTD_use_indefinitely;
        if (*dctx).refMultipleDDicts == ZSTD_rmd_refMultipleDDicts {
            if (*dctx).ddictSet.is_null() {
                (*dctx).ddictSet = ZSTD_createDDictHashSet((*dctx).customMem);
                if (*dctx).ddictSet.is_null() {
                    return error(code::MEMORY_ALLOCATION);
                }
            }
            debug_assert!((*dctx).staticSize == 0);
            let e = ZSTD_DDictHashSet_addDDict((*dctx).ddictSet, ddict, (*dctx).customMem);
            if ZSTD_isError(e) != 0 {
                return e;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setMaxWindowSize(
    dctx: *mut ZSTD_DCtx,
    maxWindowSize: usize,
) -> usize {
    let bounds = ZSTD_dParam_getBounds(ZSTD_d_windowLogMax);
    let min = 1usize << bounds.lowerBound;
    let max = 1usize << bounds.upperBound;
    if (*dctx).streamStage != zdss_init {
        return error(code::STAGE_WRONG);
    }
    if maxWindowSize < min {
        return error(code::PARAMETER_OUTOFBOUND);
    }
    if maxWindowSize > max {
        return error(code::PARAMETER_OUTOFBOUND);
    }
    (*dctx).maxWindowSize = maxWindowSize;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setFormat(dctx: *mut ZSTD_DCtx, format: ZSTD_format_e) -> usize {
    ZSTD_DCtx_setParameter(dctx, ZSTD_d_format, format as c_int)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_dParam_getBounds(dParam: c_int) -> ZSTD_bounds {
    let mut bounds = ZSTD_bounds {
        error: 0,
        lowerBound: 0,
        upperBound: 0,
    };
    match dParam {
        d if d == ZSTD_d_windowLogMax => {
            bounds.lowerBound = ZSTD_WINDOWLOG_ABSOLUTEMIN as c_int;
            bounds.upperBound = ZSTD_WINDOWLOG_MAX as c_int;
            return bounds;
        }
        d if d == ZSTD_d_format => {
            bounds.lowerBound = ZSTD_f_zstd1 as c_int;
            bounds.upperBound = ZSTD_f_zstd1_magicless as c_int;
            return bounds;
        }
        d if d == ZSTD_d_stableOutBuffer => {
            bounds.lowerBound = ZSTD_bm_buffered as c_int;
            bounds.upperBound = ZSTD_bm_stable as c_int;
            return bounds;
        }
        d if d == ZSTD_d_forceIgnoreChecksum => {
            bounds.lowerBound = ZSTD_d_validateChecksum as c_int;
            bounds.upperBound = ZSTD_d_ignoreChecksum as c_int;
            return bounds;
        }
        d if d == ZSTD_d_refMultipleDDicts => {
            bounds.lowerBound = ZSTD_rmd_refSingleDDict as c_int;
            bounds.upperBound = ZSTD_rmd_refMultipleDDicts as c_int;
            return bounds;
        }
        d if d == ZSTD_d_disableHuffmanAssembly => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }
        d if d == ZSTD_d_maxBlockSize => {
            bounds.lowerBound = ZSTD_BLOCKSIZE_MAX_MIN;
            bounds.upperBound = ZSTD_BLOCKSIZE_MAX as c_int;
            return bounds;
        }
        _ => {}
    }
    bounds.error = error(code::PARAMETER_UNSUPPORTED);
    bounds
}

fn ZSTD_dParam_withinBounds(dParam: c_int, value: c_int) -> c_int {
    let bounds = unsafe { ZSTD_dParam_getBounds(dParam) };
    if err_is_error(bounds.error) != 0 {
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
    param: c_int,
    value: *mut c_int,
) -> usize {
    match param {
        p if p == ZSTD_d_windowLogMax => {
            *value = highbit32((*dctx).maxWindowSize as U32) as c_int;
            0
        }
        p if p == ZSTD_d_format => {
            *value = (*dctx).format as c_int;
            0
        }
        p if p == ZSTD_d_stableOutBuffer => {
            *value = (*dctx).outBufferMode as c_int;
            0
        }
        p if p == ZSTD_d_forceIgnoreChecksum => {
            *value = (*dctx).forceIgnoreChecksum as c_int;
            0
        }
        p if p == ZSTD_d_refMultipleDDicts => {
            *value = (*dctx).refMultipleDDicts as c_int;
            0
        }
        p if p == ZSTD_d_disableHuffmanAssembly => {
            *value = (*dctx).disableHufAsm as c_int;
            0
        }
        p if p == ZSTD_d_maxBlockSize => {
            *value = (*dctx).maxBlockSizeParam;
            0
        }
        _ => error(code::PARAMETER_UNSUPPORTED),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setParameter(
    dctx: *mut ZSTD_DCtx,
    dParam: c_int,
    value: c_int,
) -> usize {
    if (*dctx).streamStage != zdss_init {
        return error(code::STAGE_WRONG);
    }
    let mut value = value;
    match dParam {
        d if d == ZSTD_d_windowLogMax => {
            if value == 0 {
                value = ZSTD_WINDOWLOG_LIMIT_DEFAULT as c_int;
            }
            if ZSTD_dParam_withinBounds(ZSTD_d_windowLogMax, value) == 0 {
                return error(code::PARAMETER_OUTOFBOUND);
            }
            (*dctx).maxWindowSize = 1usize << value;
            0
        }
        d if d == ZSTD_d_format => {
            if ZSTD_dParam_withinBounds(ZSTD_d_format, value) == 0 {
                return error(code::PARAMETER_OUTOFBOUND);
            }
            (*dctx).format = value as ZSTD_format_e;
            0
        }
        d if d == ZSTD_d_stableOutBuffer => {
            if ZSTD_dParam_withinBounds(ZSTD_d_stableOutBuffer, value) == 0 {
                return error(code::PARAMETER_OUTOFBOUND);
            }
            (*dctx).outBufferMode = value as u32;
            0
        }
        d if d == ZSTD_d_forceIgnoreChecksum => {
            if ZSTD_dParam_withinBounds(ZSTD_d_forceIgnoreChecksum, value) == 0 {
                return error(code::PARAMETER_OUTOFBOUND);
            }
            (*dctx).forceIgnoreChecksum = value as u32;
            0
        }
        d if d == ZSTD_d_refMultipleDDicts => {
            if ZSTD_dParam_withinBounds(ZSTD_d_refMultipleDDicts, value) == 0 {
                return error(code::PARAMETER_OUTOFBOUND);
            }
            if (*dctx).staticSize != 0 {
                return error(code::PARAMETER_UNSUPPORTED);
            }
            (*dctx).refMultipleDDicts = value as u32;
            0
        }
        d if d == ZSTD_d_disableHuffmanAssembly => {
            if ZSTD_dParam_withinBounds(ZSTD_d_disableHuffmanAssembly, value) == 0 {
                return error(code::PARAMETER_OUTOFBOUND);
            }
            (*dctx).disableHufAsm = (value != 0) as i32;
            0
        }
        d if d == ZSTD_d_maxBlockSize => {
            if value != 0 && ZSTD_dParam_withinBounds(ZSTD_d_maxBlockSize, value) == 0 {
                return error(code::PARAMETER_OUTOFBOUND);
            }
            (*dctx).maxBlockSizeParam = value;
            0
        }
        _ => error(code::PARAMETER_UNSUPPORTED),
    }
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
            return error(code::STAGE_WRONG);
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

unsafe fn ZSTD_decodingBufferSize_internal(
    windowSize: c_ulonglong,
    frameContentSize: c_ulonglong,
    blockSizeMax: usize,
) -> usize {
    let blockSize = MIN_usize(
        MIN_usize(windowSize as usize, ZSTD_BLOCKSIZE_MAX),
        blockSizeMax,
    );
    let neededRBSize = windowSize
        + (blockSize as c_ulonglong * 2)
        + (WILDCOPY_OVERLENGTH as c_ulonglong * 2);
    let neededSize = MIN_u64(frameContentSize, neededRBSize);
    let minRBSize = neededSize as usize;
    if minRBSize as c_ulonglong != neededSize {
        return error(code::FRAMEPARAMETER_WINDOWTOOLARGE);
    }
    minRBSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodingBufferSize_min(
    windowSize: c_ulonglong,
    frameContentSize: c_ulonglong,
) -> usize {
    ZSTD_decodingBufferSize_internal(windowSize, frameContentSize, ZSTD_BLOCKSIZE_MAX)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize(windowSize: usize) -> usize {
    let blockSize = MIN_usize(windowSize, ZSTD_BLOCKSIZE_MAX);
    let inBuffSize = blockSize; /* no block can be larger */
    let outBuffSize =
        ZSTD_decodingBufferSize_min(windowSize as c_ulonglong, ZSTD_CONTENTSIZE_UNKNOWN);
    ZSTD_estimateDCtxSize() + inBuffSize + outBuffSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize_fromFrame(
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let windowSizeMax: U32 = 1u32 << ZSTD_WINDOWLOG_MAX;
    let mut zfh: ZSTD_FrameHeader = core::mem::zeroed();
    let err = ZSTD_getFrameHeader(&mut zfh, src, srcSize);
    if ZSTD_isError(err) != 0 {
        return err;
    }
    if err > 0 {
        return error(code::SRCSIZE_WRONG);
    }
    if zfh.windowSize > windowSizeMax as u64 {
        return error(code::FRAMEPARAMETER_WINDOWTOOLARGE);
    }
    ZSTD_estimateDStreamSize(zfh.windowSize as usize)
}

/* *****   Decompression   ***** */

unsafe fn ZSTD_DCtx_isOverflow(
    zds: *mut ZSTD_DStream,
    neededInBuffSize: usize,
    neededOutBuffSize: usize,
) -> c_int {
    (((*zds).inBuffSize + (*zds).outBuffSize)
        >= (neededInBuffSize + neededOutBuffSize) * ZSTD_WORKSPACETOOLARGE_FACTOR) as c_int
}

unsafe fn ZSTD_DCtx_updateOversizedDuration(
    zds: *mut ZSTD_DStream,
    neededInBuffSize: usize,
    neededOutBuffSize: usize,
) {
    if ZSTD_DCtx_isOverflow(zds, neededInBuffSize, neededOutBuffSize) != 0 {
        (*zds).oversizedDuration += 1;
    } else {
        (*zds).oversizedDuration = 0;
    }
}

unsafe fn ZSTD_DCtx_isOversizedTooLong(zds: *mut ZSTD_DStream) -> c_int {
    ((*zds).oversizedDuration >= ZSTD_WORKSPACETOOLARGE_MAXDURATION) as c_int
}

/* Checks that the output buffer hasn't changed if ZSTD_obm_stable is used. */
unsafe fn ZSTD_checkOutBuffer(zds: *const ZSTD_DStream, output: *const ZSTD_outBuffer) -> usize {
    let expect = (*zds).expectedOutBuffer;
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
    error(code::DSTBUFFER_WRONG)
}

unsafe fn ZSTD_decompressContinueStream(
    zds: *mut ZSTD_DStream,
    op: *mut *mut core::ffi::c_char,
    oend: *mut core::ffi::c_char,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let isSkipFrame = ZSTD_isSkipFrame(zds);
    if (*zds).outBufferMode == ZSTD_bm_buffered {
        let dstSize = if isSkipFrame != 0 {
            0
        } else {
            (*zds).outBuffSize - (*zds).outStart
        };
        let decodedSize = ZSTD_decompressContinue(
            zds,
            (*zds).outBuff.add((*zds).outStart) as *mut c_void,
            dstSize,
            src,
            srcSize,
        );
        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        if decodedSize == 0 && isSkipFrame == 0 {
            (*zds).streamStage = zdss_read;
        } else {
            (*zds).outEnd = (*zds).outStart + decodedSize;
            (*zds).streamStage = zdss_flush;
        }
    } else {
        /* Write directly into the output buffer */
        let dstSize = if isSkipFrame != 0 {
            0
        } else {
            oend.offset_from(*op) as usize
        };
        let decodedSize = ZSTD_decompressContinue(zds, *op as *mut c_void, dstSize, src, srcSize);
        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        *op = (*op).add(decodedSize);
        /* Flushing is not needed. */
        (*zds).streamStage = zdss_read;
        debug_assert!(*op <= oend);
        debug_assert!((*zds).outBufferMode == ZSTD_bm_stable);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream(
    zds: *mut ZSTD_DStream,
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
    let mut someMoreWork: U32 = 1;

    debug_assert!(!zds.is_null());
    if (*input).pos > (*input).size {
        return error(code::SRCSIZE_WRONG);
    }
    if (*output).pos > (*output).size {
        return error(code::DSTSIZE_TOOSMALL);
    }
    {
        let e = ZSTD_checkOutBuffer(zds, output);
        if ZSTD_isError(e) != 0 {
            return e;
        }
    }

    while someMoreWork != 0 {
        match (*zds).streamStage {
            s if s == zdss_init => {
                (*zds).streamStage = zdss_loadHeader;
                (*zds).lhSize = 0;
                (*zds).inPos = 0;
                (*zds).outStart = 0;
                (*zds).outEnd = 0;
                (*zds).legacyVersion = 0;
                (*zds).hostageByte = 0;
                (*zds).expectedOutBuffer = *output;
                /* fallthrough into zdss_loadHeader */
                if !ZSTD_decompressStream_loadHeader(
                    zds,
                    &mut ip,
                    iend,
                    istart,
                    &mut op,
                    oend,
                    &mut someMoreWork,
                    output,
                    input,
                ) {
                    /* returned an early value */
                    return LOADHEADER_RETVAL;
                }
            }
            s if s == zdss_loadHeader => {
                if !ZSTD_decompressStream_loadHeader(
                    zds,
                    &mut ip,
                    iend,
                    istart,
                    &mut op,
                    oend,
                    &mut someMoreWork,
                    output,
                    input,
                ) {
                    return LOADHEADER_RETVAL;
                }
            }
            s if s == zdss_read => {
                let neededInSize =
                    ZSTD_nextSrcSizeToDecompressWithInputSize(zds, iend.offset_from(ip) as usize);
                if neededInSize == 0 {
                    /* end of frame */
                    (*zds).streamStage = zdss_init;
                    someMoreWork = 0;
                    continue;
                }
                if (iend.offset_from(ip) as usize) >= neededInSize {
                    /* decode directly from src */
                    let e = ZSTD_decompressContinueStream(
                        zds,
                        &mut op,
                        oend,
                        ip as *const c_void,
                        neededInSize,
                    );
                    if ZSTD_isError(e) != 0 {
                        return e;
                    }
                    ip = ip.add(neededInSize);
                    /* Function modifies the stage so we must break */
                    continue;
                }
                if ip == iend {
                    someMoreWork = 0;
                    continue;
                } /* no more input */
                (*zds).streamStage = zdss_load;
                /* fallthrough into zdss_load handled below by falling through */
                if !ZSTD_decompressStream_load(zds, &mut ip, iend, &mut op, oend, &mut someMoreWork) {
                    return LOADHEADER_RETVAL;
                }
            }
            s if s == zdss_load => {
                if !ZSTD_decompressStream_load(zds, &mut ip, iend, &mut op, oend, &mut someMoreWork) {
                    return LOADHEADER_RETVAL;
                }
            }
            s if s == zdss_flush => {
                let toFlushSize = (*zds).outEnd - (*zds).outStart;
                let flushedSize = ZSTD_limitCopy(
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    (*zds).outBuff.add((*zds).outStart) as *const c_void,
                    toFlushSize,
                );

                op = op.add(flushedSize);

                (*zds).outStart += flushedSize;
                if flushedSize == toFlushSize {
                    /* flush completed */
                    (*zds).streamStage = zdss_read;
                    if ((*zds).outBuffSize as u64) < (*zds).fParams.frameContentSize
                        && ((*zds).outStart + (*zds).fParams.blockSizeMax as usize)
                            > (*zds).outBuffSize
                    {
                        (*zds).outStart = 0;
                        (*zds).outEnd = 0;
                    }
                    continue;
                }
                /* cannot complete flush */
                someMoreWork = 0;
            }
            _ => {
                debug_assert!(false); /* impossible */
                return error(code::GENERIC);
            }
        }
    }

    /* result */
    (*input).pos = ip.offset_from((*input).src as *const core::ffi::c_char) as usize;
    (*output).pos = op.offset_from((*output).dst as *mut core::ffi::c_char) as usize;

    /* Update the expected output buffer for ZSTD_obm_stable. */
    (*zds).expectedOutBuffer = *output;

    if ip == istart && op == ostart {
        /* no forward progress */
        (*zds).noForwardProgress += 1;
        if (*zds).noForwardProgress >= ZSTD_NO_FORWARD_PROGRESS_MAX {
            if op == oend {
                return error(code::NOFORWARDPROGRESS_DESTFULL);
            }
            if ip == iend {
                return error(code::NOFORWARDPROGRESS_INPUTEMPTY);
            }
            debug_assert!(false);
        }
    } else {
        (*zds).noForwardProgress = 0;
    }
    {
        let mut nextSrcSizeHint = ZSTD_nextSrcSizeToDecompress(zds);
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
                (*input).pos -= 1;
                (*zds).hostageByte = 1;
            }
            return 1;
        }
        nextSrcSizeHint +=
            ZSTD_blockHeaderSize * ((ZSTD_nextInputType(zds) == ZSTDnit_block) as usize);
        debug_assert!((*zds).inPos <= nextSrcSizeHint);
        nextSrcSizeHint -= (*zds).inPos; /* part already loaded */
        nextSrcSizeHint
    }
}

/* The C control flow of ZSTD_decompressStream uses fallthrough and `return`
 * inside a switch within a while loop. To faithfully reproduce it in Rust
 * while keeping the outer loop, we use thread-local scratch for the "early
 * return" value produced by the loadHeader / load helpers. Since the crate is
 * single-threaded per the build config, a static is used. */
static mut LOADHEADER_RETVAL: usize = 0;

/* Returns true to continue the outer loop, false to return LOADHEADER_RETVAL. */
unsafe fn ZSTD_decompressStream_loadHeader(
    zds: *mut ZSTD_DStream,
    ip_ref: *mut *const core::ffi::c_char,
    iend: *const core::ffi::c_char,
    istart: *const core::ffi::c_char,
    op_ref: *mut *mut core::ffi::c_char,
    oend: *mut core::ffi::c_char,
    someMoreWork: *mut U32,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> bool {
    let mut ip = *ip_ref;
    let mut op = *op_ref;

    if (*zds).legacyVersion != 0 {
        if (*zds).staticSize != 0 {
            LOADHEADER_RETVAL = error(code::MEMORY_ALLOCATION);
            return false;
        }
        let hint =
            ZSTD_decompressLegacyStream((*zds).legacyContext, (*zds).legacyVersion, output, input);
        if hint == 0 {
            (*zds).streamStage = zdss_init;
        }
        LOADHEADER_RETVAL = hint;
        return false;
    }
    {
        let hSize = ZSTD_getFrameHeader_advanced(
            &mut (*zds).fParams,
            (*zds).headerBuffer.as_ptr() as *const c_void,
            (*zds).lhSize,
            (*zds).format,
        );
        if (*zds).refMultipleDDicts != 0 && !(*zds).ddictSet.is_null() {
            ZSTD_DCtx_selectFrameDDict(zds);
        }
        if ZSTD_isError(hSize) != 0 {
            let legacyVersion = ZSTD_isLegacy(istart as *const c_void, iend.offset_from(istart) as usize);
            if legacyVersion != 0 {
                let ddict = ZSTD_getDDict(zds);
                let dict = if !ddict.is_null() {
                    ZSTD_DDict_dictContent(ddict)
                } else {
                    core::ptr::null()
                };
                let dictSize = if !ddict.is_null() {
                    ZSTD_DDict_dictSize(ddict)
                } else {
                    0
                };
                if (*zds).staticSize != 0 {
                    LOADHEADER_RETVAL = error(code::MEMORY_ALLOCATION);
                    return false;
                }
                let e = ZSTD_initLegacyStream(
                    &mut (*zds).legacyContext,
                    (*zds).previousLegacyVersion,
                    legacyVersion,
                    dict,
                    dictSize,
                );
                if ZSTD_isError(e) != 0 {
                    LOADHEADER_RETVAL = e;
                    return false;
                }
                (*zds).legacyVersion = legacyVersion;
                (*zds).previousLegacyVersion = legacyVersion;
                let hint =
                    ZSTD_decompressLegacyStream((*zds).legacyContext, legacyVersion, output, input);
                if hint == 0 {
                    (*zds).streamStage = zdss_init;
                }
                LOADHEADER_RETVAL = hint;
                return false;
            }
            LOADHEADER_RETVAL = hSize; /* error */
            return false;
        }
        if hSize != 0 {
            /* need more input */
            let toLoad = hSize - (*zds).lhSize;
            let remainingInput = iend.offset_from(ip) as usize;
            debug_assert!(iend >= ip);
            if toLoad > remainingInput {
                /* not enough input to load full header */
                if remainingInput > 0 {
                    core::ptr::copy_nonoverlapping(
                        ip as *const u8,
                        (*zds).headerBuffer.as_mut_ptr().add((*zds).lhSize),
                        remainingInput,
                    );
                    (*zds).lhSize += remainingInput;
                }
                (*input).pos = (*input).size;
                /* check first few bytes */
                let e = ZSTD_getFrameHeader_advanced(
                    &mut (*zds).fParams,
                    (*zds).headerBuffer.as_ptr() as *const c_void,
                    (*zds).lhSize,
                    (*zds).format,
                );
                if ZSTD_isError(e) != 0 {
                    LOADHEADER_RETVAL = e;
                    return false;
                }
                /* return hint input size */
                LOADHEADER_RETVAL =
                    (MAX_usize(ZSTD_FRAMEHEADERSIZE_MIN((*zds).format), hSize) - (*zds).lhSize)
                        + ZSTD_blockHeaderSize;
                return false;
            }
            debug_assert!(!ip.is_null());
            core::ptr::copy_nonoverlapping(
                ip as *const u8,
                (*zds).headerBuffer.as_mut_ptr().add((*zds).lhSize),
                toLoad,
            );
            (*zds).lhSize = hSize;
            ip = ip.add(toLoad);
            *ip_ref = ip;
            *op_ref = op;
            return true; /* break out of switch (continue outer loop) */
        }
    }

    /* check for single-pass mode opportunity */
    if (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
        && (*zds).fParams.frameType != ZSTD_skippableFrame
        && (oend.offset_from(op) as u64) >= (*zds).fParams.frameContentSize
    {
        let cSize = ZSTD_findFrameCompressedSize_advanced(
            istart as *const c_void,
            iend.offset_from(istart) as usize,
            (*zds).format,
        );
        if cSize <= (iend.offset_from(istart) as usize) {
            /* shortcut : using single-pass mode */
            let decompressedSize = ZSTD_decompress_usingDDict(
                zds,
                op as *mut c_void,
                oend.offset_from(op) as usize,
                istart as *const c_void,
                cSize,
                ZSTD_getDDict(zds),
            );
            if ZSTD_isError(decompressedSize) != 0 {
                LOADHEADER_RETVAL = decompressedSize;
                return false;
            }
            debug_assert!(!istart.is_null());
            ip = istart.add(cSize);
            op = if !op.is_null() { op.add(decompressedSize) } else { op };
            (*zds).expected = 0;
            (*zds).streamStage = zdss_init;
            *someMoreWork = 0;
            *ip_ref = ip;
            *op_ref = op;
            return true;
        }
    }

    /* Check output buffer is large enough for ZSTD_odm_stable. */
    if (*zds).outBufferMode == ZSTD_bm_stable
        && (*zds).fParams.frameType != ZSTD_skippableFrame
        && (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
        && (oend.offset_from(op) as u64) < (*zds).fParams.frameContentSize
    {
        LOADHEADER_RETVAL = error(code::DSTSIZE_TOOSMALL);
        return false;
    }

    /* Consume header (see ZSTDds_decodeFrameHeader) */
    {
        let e = ZSTD_decompressBegin_usingDDict(zds, ZSTD_getDDict(zds));
        if ZSTD_isError(e) != 0 {
            LOADHEADER_RETVAL = e;
            return false;
        }
    }

    if (*zds).format == ZSTD_f_zstd1
        && (mem_read_le32((*zds).headerBuffer.as_ptr() as *const c_void) & ZSTD_MAGIC_SKIPPABLE_MASK)
            == ZSTD_MAGIC_SKIPPABLE_START
    {
        /* skippable frame */
        (*zds).expected = mem_read_le32(
            (*zds).headerBuffer.as_ptr().add(ZSTD_FRAMEIDSIZE) as *const c_void,
        ) as usize;
        (*zds).stage = ZSTDds_skipFrame;
    } else {
        let e = ZSTD_decodeFrameHeader(
            zds,
            (*zds).headerBuffer.as_ptr() as *const c_void,
            (*zds).lhSize,
        );
        if ZSTD_isError(e) != 0 {
            LOADHEADER_RETVAL = e;
            return false;
        }
        (*zds).expected = ZSTD_blockHeaderSize;
        (*zds).stage = ZSTDds_decodeBlockHeader;
    }

    /* control buffer memory usage */
    (*zds).fParams.windowSize =
        MAX_u64((*zds).fParams.windowSize, 1u64 << ZSTD_WINDOWLOG_ABSOLUTEMIN);
    if (*zds).fParams.windowSize > (*zds).maxWindowSize as u64 {
        LOADHEADER_RETVAL = error(code::FRAMEPARAMETER_WINDOWTOOLARGE);
        return false;
    }
    if (*zds).maxBlockSizeParam != 0 {
        (*zds).fParams.blockSizeMax = MIN_usize(
            (*zds).fParams.blockSizeMax as usize,
            (*zds).maxBlockSizeParam as usize,
        ) as u32;
    }

    /* Adapt buffer sizes to frame header instructions */
    {
        let neededInBuffSize = MAX_usize((*zds).fParams.blockSizeMax as usize, 4);
        let neededOutBuffSize = if (*zds).outBufferMode == ZSTD_bm_buffered {
            ZSTD_decodingBufferSize_internal(
                (*zds).fParams.windowSize,
                (*zds).fParams.frameContentSize,
                (*zds).fParams.blockSizeMax as usize,
            )
        } else {
            0
        };

        ZSTD_DCtx_updateOversizedDuration(zds, neededInBuffSize, neededOutBuffSize);

        let tooSmall =
            ((*zds).inBuffSize < neededInBuffSize) || ((*zds).outBuffSize < neededOutBuffSize);
        let tooLarge = ZSTD_DCtx_isOversizedTooLong(zds) != 0;

        if tooSmall || tooLarge {
            let bufferSize = neededInBuffSize + neededOutBuffSize;
            if (*zds).staticSize != 0 {
                /* static DCtx */
                debug_assert!((*zds).staticSize >= core::mem::size_of::<ZSTD_DCtx>());
                if bufferSize > (*zds).staticSize - core::mem::size_of::<ZSTD_DCtx>() {
                    LOADHEADER_RETVAL = error(code::MEMORY_ALLOCATION);
                    return false;
                }
            } else {
                zstd_custom_free((*zds).inBuff as *mut c_void, (*zds).customMem);
                (*zds).inBuffSize = 0;
                (*zds).outBuffSize = 0;
                (*zds).inBuff = zstd_custom_malloc(bufferSize, (*zds).customMem) as *mut u8;
                if (*zds).inBuff.is_null() {
                    LOADHEADER_RETVAL = error(code::MEMORY_ALLOCATION);
                    return false;
                }
            }
            (*zds).inBuffSize = neededInBuffSize;
            (*zds).outBuff = (*zds).inBuff.add((*zds).inBuffSize);
            (*zds).outBuffSize = neededOutBuffSize;
        }
    }
    (*zds).streamStage = zdss_read;
    /* fallthrough into zdss_read: return true, outer loop re-dispatches */
    *ip_ref = ip;
    *op_ref = op;
    true
}

/* Returns true to continue the outer loop, false to return LOADHEADER_RETVAL. */
unsafe fn ZSTD_decompressStream_load(
    zds: *mut ZSTD_DStream,
    ip_ref: *mut *const core::ffi::c_char,
    iend: *const core::ffi::c_char,
    op_ref: *mut *mut core::ffi::c_char,
    oend: *mut core::ffi::c_char,
    someMoreWork: *mut U32,
) -> bool {
    let mut ip = *ip_ref;
    let mut op = *op_ref;

    let neededInSize = ZSTD_nextSrcSizeToDecompress(zds);
    let toLoad = neededInSize - (*zds).inPos;
    let isSkipFrame = ZSTD_isSkipFrame(zds);
    let loadedSize: usize;
    debug_assert!(
        neededInSize == ZSTD_nextSrcSizeToDecompressWithInputSize(zds, iend.offset_from(ip) as usize)
    );
    if isSkipFrame != 0 {
        loadedSize = MIN_usize(toLoad, iend.offset_from(ip) as usize);
    } else {
        if toLoad > (*zds).inBuffSize - (*zds).inPos {
            LOADHEADER_RETVAL = error(code::CORRUPTION_DETECTED);
            return false;
        }
        loadedSize = ZSTD_limitCopy(
            (*zds).inBuff.add((*zds).inPos) as *mut c_void,
            toLoad,
            ip as *const c_void,
            iend.offset_from(ip) as usize,
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
        *op_ref = op;
        return true;
    } /* not enough input, wait for more */

    /* decode loaded input */
    (*zds).inPos = 0; /* input is consumed */
    let e = ZSTD_decompressContinueStream(
        zds,
        &mut op,
        oend,
        (*zds).inBuff as *const c_void,
        neededInSize,
    );
    if ZSTD_isError(e) != 0 {
        LOADHEADER_RETVAL = e;
        return false;
    }
    *ip_ref = ip;
    *op_ref = op;
    true
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
    let cErr = ZSTD_decompressStream(dctx, &mut output, &mut input);
    *dstPos = output.pos;
    *srcPos = input.pos;
    cErr
}
