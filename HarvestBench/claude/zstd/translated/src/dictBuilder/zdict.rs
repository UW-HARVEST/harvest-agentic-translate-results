//! Translation of dictBuilder/zdict.c — zstd dictionary builder core.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(dead_code, unused_mut, unused_assignments, unused_parens)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::common::allocations::{free, malloc, memcpy, memmove, memset, ZSTD_customMem, ZSTD_defaultCMem};
use crate::common::bits::{highbit32, nb_common_bytes};
use crate::common::error::{code, err_is_error, err_get_error_name, error};
use crate::common::fse::FSE_isError;
use crate::common::huf_common::HUF_isError;
use crate::common::mem::{mem_read16, mem_read64, mem_read_le32, mem_read_st, mem_write_le32};
use crate::common::xxhash::ZSTD_XXH64;
use crate::common::zstd_internal::{repStartValue, LLFSELog, MLFSELog, MaxLL, MaxML, OffFSELog, ZSTD_REP_NUM};
use crate::compress::fse_compress::{FSE_normalizeCount, FSE_writeNCount};
use crate::compress::huf_compress::{HUF_buildCTable_wksp, HUF_writeCTable_wksp, HUF_CElt};
use crate::compress::zstd_compress_internal::{SeqDef, SeqStore_t, ZSTD_compressedBlockState_t};
use crate::zstd_h::{
    ZSTD_compressionParameters, ZSTD_parameters, ZSTD_BLOCKSIZE_MAX, ZSTD_MAGIC_DICTIONARY,
};

// Public ZDICT types (shared with cover / fastcover). Field layout copied EXACTLY from zdict.h.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZDICT_params_t {
    pub compressionLevel: c_int,
    pub notificationLevel: c_uint,
    pub dictID: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZDICT_cover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZDICT_fastCover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub f: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub accel: c_uint,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZDICT_legacy_params_t {
    pub selectivityLevel: c_uint,
    pub zParams: ZDICT_params_t,
}

// ==== extern "C" cross-module declarations ====
extern "C" {
    // divsufsort (crate::dictBuilder::divsufsort)
    fn divsufsort(T: *const u8, SA: *mut c_int, n: c_int, openMP: c_int) -> c_int;

    // fastcover (crate::dictBuilder::fastcover)
    fn ZDICT_optimizeTrainFromBuffer_fastCover(
        dictBuffer: *mut c_void,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
        parameters: *mut ZDICT_fastCover_params_t,
    ) -> usize;

    // zstd_compress.c
    fn ZSTD_loadCEntropy(
        bs: *mut ZSTD_compressedBlockState_t,
        workspace: *mut c_void,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn ZSTD_reset_compressedBlockState(bs: *mut ZSTD_compressedBlockState_t);

    fn ZSTD_isError(code: usize) -> c_uint;
    fn ZSTD_getParams(
        compressionLevel: c_int,
        estimatedSrcSize: u64,
        dictSize: usize,
    ) -> ZSTD_parameters;
    fn ZSTD_createCDict_advanced(
        dict: *const c_void,
        dictSize: usize,
        dictLoadMethod: c_uint,
        dictContentType: c_uint,
        cParams: ZSTD_compressionParameters,
        customMem: ZSTD_customMem,
    ) -> *mut c_void;
    fn ZSTD_createCCtx() -> *mut c_void;
    fn ZSTD_freeCDict(CDict: *mut c_void) -> usize;
    fn ZSTD_freeCCtx(cctx: *mut c_void) -> usize;
    fn ZSTD_getSeqStore(ctx: *const c_void) -> *const SeqStore_t;
    fn ZSTD_seqToCodes(seqStorePtr: *const SeqStore_t) -> c_int;
    fn ZSTD_compressBegin_usingCDict_deprecated(cctx: *mut c_void, cdict: *const c_void) -> usize;
    fn ZSTD_compressBlock_deprecated(
        cctx: *mut c_void,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
}

// ==== Constants ====
const MINRATIO: u32 = 4;
const ZDICT_MAX_SAMPLES_SIZE: usize = 2000usize << 20;
const ZDICT_CONTENTSIZE_MIN: usize = 128;
const ZDICT_DICTSIZE_MIN: usize = 256;
const ZDICT_MIN_SAMPLES_SIZE: usize = ZDICT_CONTENTSIZE_MIN * MINRATIO as usize;

const DICTLISTSIZE_DEFAULT: u32 = 10000;
const NOISELENGTH: usize = 32;
const g_selectivity_default: u32 = 9;

const LLIMIT: usize = 64;
const MINMATCHLENGTH: usize = 7;
const MAXREPOFFSET: usize = 1024;
const OFFCODE_MAX: u32 = 30;
const HBUFFSIZE: usize = 256;
const ZSTD_CLEVEL_DEFAULT: c_int = 3;

const HUF_CTABLE_WORKSPACE_SIZE_U32: usize = 1216;
const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;

#[inline]
fn MIN(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}
#[inline]
fn MAX_u32(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

// Console display: notificationLevel-gated stderr output. Reproduced as no-ops
// (they never affect the produced dictionary bytes).
unsafe fn ZDICT_printHex(ptr: *const c_void, length: usize) {
    let b = ptr as *const u8;
    let mut u: usize = 0;
    while u < length {
        let mut c = *b.add(u);
        if c < 32 || c > 126 {
            c = b'.';
        }
        let _ = c;
        u += 1;
    }
}

/*-********************************************************
*  Helper functions
**********************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_isError(errorCode: usize) -> c_uint {
    err_is_error(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getErrorName(errorCode: usize) -> *const c_char {
    err_get_error_name(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getDictID(dictBuffer: *const c_void, dictSize: usize) -> c_uint {
    if dictSize < 8 {
        return 0;
    }
    if mem_read_le32(dictBuffer) != ZSTD_MAGIC_DICTIONARY {
        return 0;
    }
    mem_read_le32((dictBuffer as *const c_char).add(4) as *const c_void)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getDictHeaderSize(dictBuffer: *const c_void, dictSize: usize) -> usize {
    let headerSize: usize;
    if dictSize <= 8 || mem_read_le32(dictBuffer) != ZSTD_MAGIC_DICTIONARY {
        return error(code::DICTIONARY_CORRUPTED);
    }

    {
        let bs = malloc(core::mem::size_of::<ZSTD_compressedBlockState_t>()) as *mut ZSTD_compressedBlockState_t;
        let wksp = malloc(HUF_WORKSPACE_SIZE) as *mut u32;
        if bs.is_null() || wksp.is_null() {
            headerSize = error(code::MEMORY_ALLOCATION);
        } else {
            ZSTD_reset_compressedBlockState(bs);
            headerSize = ZSTD_loadCEntropy(bs, wksp as *mut c_void, dictBuffer, dictSize);
        }

        free(bs as *mut c_void);
        free(wksp as *mut c_void);
    }

    headerSize
}

/*-********************************************************
*  Dictionary training functions
**********************************************************/
/* Count the nb of common bytes between 2 pointers. */
unsafe fn ZDICT_count(pIn: *const c_void, pMatch: *const c_void) -> usize {
    let pStart = pIn as *const c_char;
    let mut pIn = pIn as *const c_char;
    let mut pMatch = pMatch as *const c_char;
    loop {
        let diff = mem_read_st(pMatch as *const c_void) ^ mem_read_st(pIn as *const c_void);
        if diff == 0 {
            pIn = pIn.add(core::mem::size_of::<usize>());
            pMatch = pMatch.add(core::mem::size_of::<usize>());
            continue;
        }
        pIn = pIn.add(nb_common_bytes(diff) as usize);
        return (pIn as usize) - (pStart as usize);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct dictItem {
    pos: u32,
    length: u32,
    savings: u32,
}

unsafe fn ZDICT_initDictItem(d: *mut dictItem) {
    (*d).pos = 1;
    (*d).length = 0;
    (*d).savings = (-1i32) as u32;
}

unsafe fn ZDICT_analyzePos(
    doneMarks: *mut u8,
    suffix: *const c_int,
    mut start: u32,
    buffer: *const c_void,
    minRatio: u32,
    notificationLevel: u32,
) -> dictItem {
    let mut lengthList: [u32; LLIMIT] = [0; LLIMIT];
    let mut cumulLength: [u32; LLIMIT] = [0; LLIMIT];
    let mut savings: [u32; LLIMIT] = [0; LLIMIT];
    let b = buffer as *const u8;
    let mut maxLength: usize = LLIMIT;
    let mut pos: usize = *suffix.add(start as usize) as usize;
    let mut end: u32 = start;
    let mut solution: dictItem = dictItem { pos: 0, length: 0, savings: 0 };

    /* init */
    *doneMarks.add(pos) = 1;

    /* trivial repetition cases */
    if (mem_read16(b.add(pos + 0) as *const c_void) == mem_read16(b.add(pos + 2) as *const c_void))
        || (mem_read16(b.add(pos + 1) as *const c_void) == mem_read16(b.add(pos + 3) as *const c_void))
        || (mem_read16(b.add(pos + 2) as *const c_void) == mem_read16(b.add(pos + 4) as *const c_void))
    {
        /* skip and mark segment */
        let pattern16 = mem_read16(b.add(pos + 4) as *const c_void);
        let mut u: u32;
        let mut patternEnd: u32 = 6;
        while mem_read16(b.add(pos + patternEnd as usize) as *const c_void) == pattern16 {
            patternEnd += 2;
        }
        if *b.add(pos + patternEnd as usize) == *b.add(pos + patternEnd as usize - 1) {
            patternEnd += 1;
        }
        u = 1;
        while u < patternEnd {
            *doneMarks.add(pos + u as usize) = 1;
            u += 1;
        }
        return solution;
    }

    /* look forward */
    {
        let mut length: usize;
        loop {
            end += 1;
            length = ZDICT_count(
                b.add(pos) as *const c_void,
                b.add(*suffix.add(end as usize) as usize) as *const c_void,
            );
            if !(length >= MINMATCHLENGTH) {
                break;
            }
        }
    }

    /* look backward */
    {
        let mut length: usize;
        loop {
            length = ZDICT_count(
                b.add(pos) as *const c_void,
                b.add(*suffix.offset(start as isize - 1) as usize) as *const c_void,
            );
            if length >= MINMATCHLENGTH {
                start -= 1;
            }
            if !(length >= MINMATCHLENGTH) {
                break;
            }
        }
    }

    /* exit if not found a minimum nb of repetitions */
    if end - start < minRatio {
        let mut idx: u32 = start;
        while idx < end {
            *doneMarks.add(*suffix.add(idx as usize) as usize) = 1;
            idx += 1;
        }
        return solution;
    }

    {
        let mut i: c_int;
        let mut mml: u32;
        let mut refinedStart: u32 = start;
        let mut refinedEnd: u32 = end;

        mml = MINMATCHLENGTH as u32;
        loop {
            let mut currentChar: u8 = 0;
            let mut currentCount: u32 = 0;
            let mut currentID: u32 = refinedStart;
            let mut id: u32;
            let mut selectedCount: u32 = 0;
            let mut selectedID: u32 = currentID;
            id = refinedStart;
            while id < refinedEnd {
                if *b.add(*suffix.add(id as usize) as usize + mml as usize) != currentChar {
                    if currentCount > selectedCount {
                        selectedCount = currentCount;
                        selectedID = currentID;
                    }
                    currentID = id;
                    currentChar = *b.add(*suffix.add(id as usize) as usize + mml as usize);
                    currentCount = 0;
                }
                currentCount += 1;
                id += 1;
            }
            if currentCount > selectedCount {
                /* for last */
                selectedCount = currentCount;
                selectedID = currentID;
            }

            if selectedCount < minRatio {
                break;
            }
            refinedStart = selectedID;
            refinedEnd = refinedStart + selectedCount;
            mml += 1;
        }

        /* evaluate gain based on new dict */
        start = refinedStart;
        pos = *suffix.add(refinedStart as usize) as usize;
        end = start;
        for e in lengthList.iter_mut() {
            *e = 0;
        }

        /* look forward */
        {
            let mut length: usize;
            loop {
                end += 1;
                length = ZDICT_count(
                    b.add(pos) as *const c_void,
                    b.add(*suffix.add(end as usize) as usize) as *const c_void,
                );
                if length >= LLIMIT {
                    length = LLIMIT - 1;
                }
                lengthList[length] += 1;
                if !(length >= MINMATCHLENGTH) {
                    break;
                }
            }
        }

        /* look backward */
        {
            let mut length: usize = MINMATCHLENGTH;
            while (length >= MINMATCHLENGTH) & (start > 0) {
                length = ZDICT_count(
                    b.add(pos) as *const c_void,
                    b.add(*suffix.add(start as usize - 1) as usize) as *const c_void,
                );
                if length >= LLIMIT {
                    length = LLIMIT - 1;
                }
                lengthList[length] += 1;
                if length >= MINMATCHLENGTH {
                    start -= 1;
                }
            }
        }

        /* largest useful length */
        for e in cumulLength.iter_mut() {
            *e = 0;
        }
        cumulLength[maxLength - 1] = lengthList[maxLength - 1];
        i = (maxLength as c_int) - 2;
        while i >= 0 {
            cumulLength[i as usize] = cumulLength[i as usize + 1] + lengthList[i as usize];
            i -= 1;
        }

        i = (LLIMIT - 1) as c_int;
        while i >= MINMATCHLENGTH as c_int {
            if cumulLength[i as usize] >= minRatio {
                break;
            }
            i -= 1;
        }
        maxLength = i as usize;

        /* reduce maxLength in case of final into repetitive data */
        {
            let mut l: u32 = maxLength as u32;
            let c: u8 = *b.add(pos + maxLength - 1);
            while *b.add(pos + l as usize - 2) == c {
                l -= 1;
            }
            maxLength = l as usize;
        }
        if maxLength < MINMATCHLENGTH {
            return solution; /* skip : no long-enough solution */
        }

        /* calculate savings */
        savings[5] = 0;
        i = MINMATCHLENGTH as c_int;
        while i <= maxLength as c_int {
            savings[i as usize] =
                savings[i as usize - 1] + (lengthList[i as usize] * (i as u32 - 3));
            i += 1;
        }

        solution.pos = pos as u32;
        solution.length = maxLength as u32;
        solution.savings = savings[maxLength];

        /* mark positions done */
        {
            let mut id: u32 = start;
            while id < end {
                let mut p: u32;
                let pEnd: u32;
                let mut length: u32;
                let testedPos: u32 = *suffix.add(id as usize) as u32;
                if testedPos == pos as u32 {
                    length = solution.length;
                } else {
                    length =
                        ZDICT_count(b.add(pos) as *const c_void, b.add(testedPos as usize) as *const c_void) as u32;
                    if length > solution.length {
                        length = solution.length;
                    }
                }
                pEnd = testedPos + length;
                p = testedPos;
                while p < pEnd {
                    *doneMarks.add(p as usize) = 1;
                    p += 1;
                }
                id += 1;
            }
        }
    }

    solution
}

unsafe fn isIncluded(inp: *const c_void, container: *const c_void, length: usize) -> c_int {
    let ip = inp as *const c_char;
    let into = container as *const c_char;
    let mut u: usize = 0;

    while u < length {
        if *ip.add(u) != *into.add(u) {
            break;
        }
        u += 1;
    }

    (u == length) as c_int
}

/* check if dictItem can be merged, do it if possible. return id of dest elt, 0 if not merged */
unsafe fn ZDICT_tryMerge(
    table: *mut dictItem,
    mut elt: dictItem,
    eltNbToSkip: u32,
    buffer: *const c_void,
) -> u32 {
    let tableSize = (*table).pos;
    let eltEnd = elt.pos + elt.length;
    let buf = buffer as *const c_char;

    /* tail overlap */
    let mut u: u32 = 1;
    while u < tableSize {
        if u == eltNbToSkip {
            u += 1;
            continue;
        }
        if ((*table.add(u as usize)).pos > elt.pos) && ((*table.add(u as usize)).pos <= eltEnd) {
            /* overlap, existing > new */
            /* append */
            let addedLength = (*table.add(u as usize)).pos - elt.pos;
            (*table.add(u as usize)).length += addedLength;
            (*table.add(u as usize)).pos = elt.pos;
            (*table.add(u as usize)).savings += elt.savings * addedLength / elt.length;
            (*table.add(u as usize)).savings += elt.length / 8;
            elt = *table.add(u as usize);
            /* sort : improve rank */
            while (u > 1) && ((*table.add(u as usize - 1)).savings < elt.savings) {
                *table.add(u as usize) = *table.add(u as usize - 1);
                u -= 1;
            }
            *table.add(u as usize) = elt;
            return u;
        }
        u += 1;
    }

    /* front overlap */
    u = 1;
    while u < tableSize {
        if u == eltNbToSkip {
            u += 1;
            continue;
        }

        if ((*table.add(u as usize)).pos + (*table.add(u as usize)).length >= elt.pos)
            && ((*table.add(u as usize)).pos < elt.pos)
        {
            /* overlap, existing < new */
            /* append */
            let addedLength: c_int =
                eltEnd as c_int - ((*table.add(u as usize)).pos + (*table.add(u as usize)).length) as c_int;
            (*table.add(u as usize)).savings += elt.length / 8;
            if addedLength > 0 {
                /* otherwise, elt fully included into existing */
                (*table.add(u as usize)).length += addedLength as u32;
                (*table.add(u as usize)).savings += elt.savings * addedLength as u32 / elt.length;
            }
            /* sort : improve rank */
            elt = *table.add(u as usize);
            while (u > 1) && ((*table.add(u as usize - 1)).savings < elt.savings) {
                *table.add(u as usize) = *table.add(u as usize - 1);
                u -= 1;
            }
            *table.add(u as usize) = elt;
            return u;
        }

        if mem_read64(buf.add((*table.add(u as usize)).pos as usize) as *const c_void)
            == mem_read64(buf.add(elt.pos as usize + 1) as *const c_void)
        {
            if isIncluded(
                buf.add((*table.add(u as usize)).pos as usize) as *const c_void,
                buf.add(elt.pos as usize + 1) as *const c_void,
                (*table.add(u as usize)).length as usize,
            ) != 0
            {
                let addedLength: usize = {
                    let a = elt.length as c_int - (*table.add(u as usize)).length as c_int;
                    if a > 1 { a as usize } else { 1usize }
                };
                (*table.add(u as usize)).pos = elt.pos;
                (*table.add(u as usize)).savings +=
                    (elt.savings as usize * addedLength / elt.length as usize) as u32;
                (*table.add(u as usize)).length = MIN(
                    elt.length as usize,
                    (*table.add(u as usize)).length as usize + 1,
                ) as u32;
                return u;
            }
        }
        u += 1;
    }

    0
}

unsafe fn ZDICT_removeDictItem(table: *mut dictItem, id: u32) {
    /* convention : table[0].pos stores nb of elts */
    let max = (*table).pos;
    let mut u: u32;
    if id == 0 {
        return; /* protection, should never happen */
    }
    u = id;
    while u < max - 1 {
        *table.add(u as usize) = *table.add(u as usize + 1);
        u += 1;
    }
    (*table).pos -= 1;
}

unsafe fn ZDICT_insertDictItem(table: *mut dictItem, maxSize: u32, elt: dictItem, buffer: *const c_void) {
    /* merge if possible */
    let mut mergeId = ZDICT_tryMerge(table, elt, 0, buffer);
    if mergeId != 0 {
        let mut newMerge: u32 = 1;
        while newMerge != 0 {
            newMerge = ZDICT_tryMerge(table, *table.add(mergeId as usize), mergeId, buffer);
            if newMerge != 0 {
                ZDICT_removeDictItem(table, mergeId);
            }
            mergeId = newMerge;
        }
        return;
    }

    /* insert */
    {
        let mut current: u32;
        let mut nextElt: u32 = (*table).pos;
        if nextElt >= maxSize {
            nextElt = maxSize - 1;
        }
        current = nextElt - 1;
        while (*table.add(current as usize)).savings < elt.savings {
            *table.add(current as usize + 1) = *table.add(current as usize);
            current = current.wrapping_sub(1);
        }
        *table.add(current as usize + 1) = elt;
        (*table).pos = nextElt + 1;
    }
}

unsafe fn ZDICT_dictSize(dictList: *const dictItem) -> u32 {
    let mut u: u32;
    let mut dictSize: u32 = 0;
    u = 1;
    while u < (*dictList.add(0)).pos {
        dictSize += (*dictList.add(u as usize)).length;
        u += 1;
    }
    dictSize
}

unsafe fn ZDICT_trainBuffer_legacy(
    dictList: *mut dictItem,
    dictListSize: u32,
    buffer: *const c_void,
    mut bufferSize: usize,
    fileSizes: *const usize,
    mut nbFiles: c_uint,
    mut minRatio: c_uint,
    notificationLevel: u32,
) -> usize {
    let suffix0 = malloc((bufferSize + 2) * core::mem::size_of::<c_int>()) as *mut c_int;
    let suffix = suffix0.wrapping_add(1);
    let reverseSuffix = malloc(bufferSize * core::mem::size_of::<u32>()) as *mut u32;
    let doneMarks = malloc((bufferSize + 16) * core::mem::size_of::<u8>()) as *mut u8; /* +16 for overflow security */
    let filePos = malloc(nbFiles as usize * core::mem::size_of::<u32>()) as *mut u32;
    let mut result: usize = 0;

    /* init */
    if suffix0.is_null() || reverseSuffix.is_null() || doneMarks.is_null() || filePos.is_null() {
        result = error(code::MEMORY_ALLOCATION);
        // _cleanup
        free(suffix0 as *mut c_void);
        free(reverseSuffix as *mut c_void);
        free(doneMarks as *mut c_void);
        free(filePos as *mut c_void);
        return result;
    }
    if minRatio < MINRATIO {
        minRatio = MINRATIO;
    }
    memset(doneMarks as *mut c_void, 0, bufferSize + 16);

    /* limit sample set size (divsufsort limitation)*/
    while bufferSize > ZDICT_MAX_SAMPLES_SIZE {
        nbFiles -= 1;
        bufferSize -= *fileSizes.add(nbFiles as usize);
    }

    /* sort */
    {
        let divSuftSortResult = divsufsort(buffer as *const u8, suffix, bufferSize as c_int, 0);
        if divSuftSortResult != 0 {
            result = error(code::GENERIC);
            free(suffix0 as *mut c_void);
            free(reverseSuffix as *mut c_void);
            free(doneMarks as *mut c_void);
            free(filePos as *mut c_void);
            return result;
        }
    }
    *suffix.add(bufferSize) = bufferSize as c_int; /* leads into noise */
    *suffix0.add(0) = bufferSize as c_int; /* leads into noise */
    /* build reverse suffix sort */
    {
        let mut pos: usize;
        pos = 0;
        while pos < bufferSize {
            *reverseSuffix.add(*suffix.add(pos) as usize) = pos as u32;
            pos += 1;
        }
        /* note filePos tracks borders between samples. Not used at this stage. */
        *filePos.add(0) = 0;
        pos = 1;
        while pos < nbFiles as usize {
            *filePos.add(pos) = (*filePos.add(pos - 1) as usize + *fileSizes.add(pos - 1)) as u32;
            pos += 1;
        }
    }

    {
        let mut cursor: u32 = 0;
        while (cursor as usize) < bufferSize {
            let solution: dictItem;
            if *doneMarks.add(cursor as usize) != 0 {
                cursor += 1;
                continue;
            }
            solution = ZDICT_analyzePos(
                doneMarks,
                suffix,
                *reverseSuffix.add(cursor as usize),
                buffer,
                minRatio,
                notificationLevel,
            );
            if solution.length == 0 {
                cursor += 1;
                continue;
            }
            ZDICT_insertDictItem(dictList, dictListSize, solution, buffer);
            cursor += solution.length;
        }
    }

    // _cleanup
    free(suffix0 as *mut c_void);
    free(reverseSuffix as *mut c_void);
    free(doneMarks as *mut c_void);
    free(filePos as *mut c_void);
    result
}

unsafe fn ZDICT_fillNoise(buffer: *mut c_void, length: usize) {
    let prime1: u32 = 2654435761;
    let prime2: u32 = 2246822519;
    let mut acc: u32 = prime1;
    let mut p: usize = 0;
    while p < length {
        acc = acc.wrapping_mul(prime2);
        *(buffer as *mut u8).add(p) = (acc >> 21) as u8;
        p += 1;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EStats_ress_t {
    dict: *mut c_void, /* ZSTD_CDict* */
    zc: *mut c_void,   /* ZSTD_CCtx* */
    workPlace: *mut c_void,
}

unsafe fn ZDICT_countEStats(
    esr: EStats_ress_t,
    params: *const ZSTD_parameters,
    countLit: *mut c_uint,
    offsetcodeCount: *mut c_uint,
    matchlengthCount: *mut c_uint,
    litlengthCount: *mut c_uint,
    repOffsets: *mut u32,
    src: *const c_void,
    mut srcSize: usize,
    notificationLevel: u32,
) {
    let blockSizeMax: usize = MIN(ZSTD_BLOCKSIZE_MAX, 1usize << (*params).cParams.windowLog);
    let cSize: usize;

    if srcSize > blockSizeMax {
        srcSize = blockSizeMax; /* protection vs large samples */
    }
    {
        let errorCode = ZSTD_compressBegin_usingCDict_deprecated(esr.zc, esr.dict);
        if ZSTD_isError(errorCode) != 0 {
            return;
        }
    }
    cSize = ZSTD_compressBlock_deprecated(esr.zc, esr.workPlace, ZSTD_BLOCKSIZE_MAX, src, srcSize);
    if ZSTD_isError(cSize) != 0 {
        return;
    }

    if cSize != 0 {
        /* if == 0; block is not compressible */
        let seqStorePtr = ZSTD_getSeqStore(esr.zc);

        /* literals stats */
        {
            let mut bytePtr = (*seqStorePtr).litStart as *const u8;
            while bytePtr < (*seqStorePtr).lit as *const u8 {
                *countLit.add(*bytePtr as usize) += 1;
                bytePtr = bytePtr.add(1);
            }
        }

        /* seqStats */
        {
            let nbSeq: u32 = (((*seqStorePtr).sequences as usize - (*seqStorePtr).sequencesStart as usize)
                / core::mem::size_of::<SeqDef>()) as u32;
            ZSTD_seqToCodes(seqStorePtr);

            {
                let codePtr = (*seqStorePtr).ofCode as *const u8;
                let mut u: u32 = 0;
                while u < nbSeq {
                    *offsetcodeCount.add(*codePtr.add(u as usize) as usize) += 1;
                    u += 1;
                }
            }

            {
                let codePtr = (*seqStorePtr).mlCode as *const u8;
                let mut u: u32 = 0;
                while u < nbSeq {
                    *matchlengthCount.add(*codePtr.add(u as usize) as usize) += 1;
                    u += 1;
                }
            }

            {
                let codePtr = (*seqStorePtr).llCode as *const u8;
                let mut u: u32 = 0;
                while u < nbSeq {
                    *litlengthCount.add(*codePtr.add(u as usize) as usize) += 1;
                    u += 1;
                }
            }

            if nbSeq >= 2 {
                /* rep offsets */
                let seq = (*seqStorePtr).sequencesStart as *const SeqDef;
                let mut offset1: u32 = (*seq.add(0)).offBase - ZSTD_REP_NUM as u32;
                let mut offset2: u32 = (*seq.add(1)).offBase - ZSTD_REP_NUM as u32;
                if offset1 >= MAXREPOFFSET as u32 {
                    offset1 = 0;
                }
                if offset2 >= MAXREPOFFSET as u32 {
                    offset2 = 0;
                }
                *repOffsets.add(offset1 as usize) += 3;
                *repOffsets.add(offset2 as usize) += 1;
            }
        }
    }
}

unsafe fn ZDICT_totalSampleSize(fileSizes: *const usize, nbFiles: c_uint) -> usize {
    let mut total: usize = 0;
    let mut u: c_uint = 0;
    while u < nbFiles {
        total += *fileSizes.add(u as usize);
        u += 1;
    }
    total
}

#[repr(C)]
#[derive(Clone, Copy)]
struct offsetCount_t {
    offset: u32,
    count: u32,
}

unsafe fn ZDICT_insertSortCount(table: *mut offsetCount_t, val: u32, count: u32) {
    let mut u: u32;
    (*table.add(ZSTD_REP_NUM)).offset = val;
    (*table.add(ZSTD_REP_NUM)).count = count;
    u = ZSTD_REP_NUM as u32;
    while u > 0 {
        let tmp: offsetCount_t;
        if (*table.add(u as usize - 1)).count >= (*table.add(u as usize)).count {
            break;
        }
        tmp = *table.add(u as usize - 1);
        *table.add(u as usize - 1) = *table.add(u as usize);
        *table.add(u as usize) = tmp;
        u -= 1;
    }
}

/* rewrite `countLit` to a mostly flat but still compressible distribution of literals. */
unsafe fn ZDICT_flatLit(countLit: *mut c_uint) {
    let mut u: c_int = 1;
    while u < 256 {
        *countLit.add(u as usize) = 2;
        u += 1;
    }
    *countLit.add(0) = 4;
    *countLit.add(253) = 1;
    *countLit.add(254) = 1;
}

unsafe fn ZDICT_analyzeEntropy(
    dstBuffer: *mut c_void,
    mut maxDstSize: usize,
    mut compressionLevel: c_int,
    srcBuffer: *const c_void,
    fileSizes: *const usize,
    nbFiles: c_uint,
    dictBuffer: *const c_void,
    dictBufferSize: usize,
    notificationLevel: c_uint,
) -> usize {
    let mut countLit: [c_uint; 256] = [0; 256];
    /* HUF_CREATE_STATIC_CTABLE(hufTable, 255) => HUF_CElt hufTable[257] */
    let mut hufTable: [HUF_CElt; 257] = [0; 257];
    let mut offcodeCount: [c_uint; OFFCODE_MAX as usize + 1] = [0; OFFCODE_MAX as usize + 1];
    let mut offcodeNCount: [i16; OFFCODE_MAX as usize + 1] = [0; OFFCODE_MAX as usize + 1];
    let offcodeMax: u32 = highbit32((dictBufferSize + (128usize << 10)) as u32);
    let mut matchLengthCount: [c_uint; MaxML as usize + 1] = [0; MaxML as usize + 1];
    let mut matchLengthNCount: [i16; MaxML as usize + 1] = [0; MaxML as usize + 1];
    let mut litLengthCount: [c_uint; MaxLL as usize + 1] = [0; MaxLL as usize + 1];
    let mut litLengthNCount: [i16; MaxLL as usize + 1] = [0; MaxLL as usize + 1];
    let mut repOffset: [u32; MAXREPOFFSET] = [0; MAXREPOFFSET];
    let mut bestRepOffset: [offsetCount_t; ZSTD_REP_NUM + 1] =
        [offsetCount_t { offset: 0, count: 0 }; ZSTD_REP_NUM + 1];
    let mut esr: EStats_ress_t = EStats_ress_t {
        dict: core::ptr::null_mut(),
        zc: core::ptr::null_mut(),
        workPlace: core::ptr::null_mut(),
    };
    let params: ZSTD_parameters;
    let mut u: u32;
    let mut huffLog: u32 = 11;
    let mut Offlog: u32 = OffFSELog;
    let mut mlLog: u32 = MLFSELog;
    let mut llLog: u32 = LLFSELog;
    let mut total: u32;
    let mut pos: usize = 0;
    let mut errorCode: usize;
    let mut eSize: usize = 0;
    let totalSrcSize: usize = ZDICT_totalSampleSize(fileSizes, nbFiles);
    let averageSampleSize: usize = totalSrcSize / (nbFiles as usize + (nbFiles == 0) as usize);
    let mut dstPtr = dstBuffer as *mut u8;
    let mut wksp: [u32; HUF_CTABLE_WORKSPACE_SIZE_U32] = [0; HUF_CTABLE_WORKSPACE_SIZE_U32];

    /* init */
    if offcodeMax > OFFCODE_MAX {
        eSize = error(code::DICTIONARYCREATION_FAILED);
        // _cleanup
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    u = 0;
    while u < 256 {
        countLit[u as usize] = 1;
        u += 1;
    }
    u = 0;
    while u <= offcodeMax {
        offcodeCount[u as usize] = 1;
        u += 1;
    }
    u = 0;
    while u <= MaxML {
        matchLengthCount[u as usize] = 1;
        u += 1;
    }
    u = 0;
    while u <= MaxLL {
        litLengthCount[u as usize] = 1;
        u += 1;
    }
    for e in repOffset.iter_mut() {
        *e = 0;
    }
    repOffset[1] = 1;
    repOffset[4] = 1;
    repOffset[8] = 1;
    for e in bestRepOffset.iter_mut() {
        *e = offsetCount_t { offset: 0, count: 0 };
    }
    if compressionLevel == 0 {
        compressionLevel = ZSTD_CLEVEL_DEFAULT;
    }
    params = ZSTD_getParams(compressionLevel, averageSampleSize as u64, dictBufferSize);

    esr.dict = ZSTD_createCDict_advanced(
        dictBuffer,
        dictBufferSize,
        crate::zstd_h::ZSTD_dlm_byRef,
        crate::zstd_h::ZSTD_dct_rawContent,
        params.cParams,
        ZSTD_defaultCMem,
    );
    esr.zc = ZSTD_createCCtx();
    esr.workPlace = malloc(ZSTD_BLOCKSIZE_MAX);
    if esr.dict.is_null() || esr.zc.is_null() || esr.workPlace.is_null() {
        eSize = error(code::MEMORY_ALLOCATION);
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }

    /* collect stats on all samples */
    u = 0;
    while u < nbFiles {
        ZDICT_countEStats(
            esr,
            &params,
            countLit.as_mut_ptr(),
            offcodeCount.as_mut_ptr(),
            matchLengthCount.as_mut_ptr(),
            litLengthCount.as_mut_ptr(),
            repOffset.as_mut_ptr(),
            (srcBuffer as *const c_char).add(pos) as *const c_void,
            *fileSizes.add(u as usize),
            notificationLevel,
        );
        pos += *fileSizes.add(u as usize);
        u += 1;
    }

    /* analyze, build stats, starting with literals */
    {
        let mut maxNbBits = HUF_buildCTable_wksp(
            hufTable.as_mut_ptr(),
            countLit.as_ptr(),
            255,
            huffLog,
            wksp.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&wksp),
        );
        if HUF_isError(maxNbBits) != 0 {
            eSize = maxNbBits;
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        if maxNbBits == 8 {
            /* not compressible : will fail on HUF_writeCTable() */
            ZDICT_flatLit(countLit.as_mut_ptr());
            maxNbBits = HUF_buildCTable_wksp(
                hufTable.as_mut_ptr(),
                countLit.as_ptr(),
                255,
                huffLog,
                wksp.as_mut_ptr() as *mut c_void,
                core::mem::size_of_val(&wksp),
            );
            debug_assert!(maxNbBits == 9);
        }
        huffLog = maxNbBits as u32;
    }

    /* looking for most common first offsets */
    {
        let mut offset: u32 = 1;
        while offset < MAXREPOFFSET as u32 {
            ZDICT_insertSortCount(bestRepOffset.as_mut_ptr(), offset, repOffset[offset as usize]);
            offset += 1;
        }
    }

    total = 0;
    u = 0;
    while u <= offcodeMax {
        total += offcodeCount[u as usize];
        u += 1;
    }
    errorCode = FSE_normalizeCount(
        offcodeNCount.as_mut_ptr(),
        Offlog,
        offcodeCount.as_ptr(),
        total as usize,
        offcodeMax,
        1,
    );
    if FSE_isError(errorCode) != 0 {
        eSize = errorCode;
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    Offlog = errorCode as u32;

    total = 0;
    u = 0;
    while u <= MaxML {
        total += matchLengthCount[u as usize];
        u += 1;
    }
    errorCode = FSE_normalizeCount(
        matchLengthNCount.as_mut_ptr(),
        mlLog,
        matchLengthCount.as_ptr(),
        total as usize,
        MaxML,
        1,
    );
    if FSE_isError(errorCode) != 0 {
        eSize = errorCode;
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    mlLog = errorCode as u32;

    total = 0;
    u = 0;
    while u <= MaxLL {
        total += litLengthCount[u as usize];
        u += 1;
    }
    errorCode = FSE_normalizeCount(
        litLengthNCount.as_mut_ptr(),
        llLog,
        litLengthCount.as_ptr(),
        total as usize,
        MaxLL,
        1,
    );
    if FSE_isError(errorCode) != 0 {
        eSize = errorCode;
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    llLog = errorCode as u32;

    /* write result to buffer */
    {
        let hhSize = HUF_writeCTable_wksp(
            dstPtr as *mut c_void,
            maxDstSize,
            hufTable.as_ptr(),
            255,
            huffLog,
            wksp.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&wksp),
        );
        if HUF_isError(hhSize) != 0 {
            eSize = hhSize;
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        dstPtr = dstPtr.add(hhSize);
        maxDstSize -= hhSize;
        eSize += hhSize;
    }

    {
        let ohSize = FSE_writeNCount(
            dstPtr as *mut c_void,
            maxDstSize,
            offcodeNCount.as_ptr(),
            OFFCODE_MAX,
            Offlog,
        );
        if FSE_isError(ohSize) != 0 {
            eSize = ohSize;
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        dstPtr = dstPtr.add(ohSize);
        maxDstSize -= ohSize;
        eSize += ohSize;
    }

    {
        let mhSize = FSE_writeNCount(
            dstPtr as *mut c_void,
            maxDstSize,
            matchLengthNCount.as_ptr(),
            MaxML,
            mlLog,
        );
        if FSE_isError(mhSize) != 0 {
            eSize = mhSize;
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        dstPtr = dstPtr.add(mhSize);
        maxDstSize -= mhSize;
        eSize += mhSize;
    }

    {
        let lhSize = FSE_writeNCount(
            dstPtr as *mut c_void,
            maxDstSize,
            litLengthNCount.as_ptr(),
            MaxLL,
            llLog,
        );
        if FSE_isError(lhSize) != 0 {
            eSize = lhSize;
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        dstPtr = dstPtr.add(lhSize);
        maxDstSize -= lhSize;
        eSize += lhSize;
    }

    if maxDstSize < 12 {
        eSize = error(code::DSTSIZE_TOOSMALL);
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    /* at this stage, we don't use the result of "most common first offset" */
    mem_write_le32(dstPtr.add(0) as *mut c_void, repStartValue[0]);
    mem_write_le32(dstPtr.add(4) as *mut c_void, repStartValue[1]);
    mem_write_le32(dstPtr.add(8) as *mut c_void, repStartValue[2]);
    eSize += 12;

    // _cleanup
    ZSTD_freeCDict(esr.dict);
    ZSTD_freeCCtx(esr.zc);
    free(esr.workPlace);

    eSize
}

/* @returns the maximum repcode value */
fn ZDICT_maxRep(reps: &[u32; ZSTD_REP_NUM]) -> u32 {
    let mut maxRep = reps[0];
    let mut r: c_int = 1;
    while r < ZSTD_REP_NUM as c_int {
        maxRep = MAX_u32(maxRep, reps[r as usize]);
        r += 1;
    }
    maxRep
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_finalizeDictionary(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    customDictContent: *const c_void,
    mut dictContentSize: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    params: ZDICT_params_t,
) -> usize {
    let mut hSize: usize;
    let mut header: [u8; HBUFFSIZE] = [0; HBUFFSIZE];
    let compressionLevel: c_int = if params.compressionLevel == 0 {
        ZSTD_CLEVEL_DEFAULT
    } else {
        params.compressionLevel
    };
    let notificationLevel: u32 = params.notificationLevel;
    /* The final dictionary content must be at least as large as the largest repcode */
    let minContentSize: usize = ZDICT_maxRep(&repStartValue) as usize;
    let paddingSize: usize;

    /* check conditions */
    if dictBufferCapacity < dictContentSize {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        return error(code::DSTSIZE_TOOSMALL);
    }

    /* dictionary header */
    mem_write_le32(header.as_mut_ptr() as *mut c_void, ZSTD_MAGIC_DICTIONARY);
    {
        let randomID: u64 = ZSTD_XXH64(customDictContent, dictContentSize, 0);
        let compliantID: u32 = (randomID % ((1u32 << 31) - 32768) as u64) as u32 + 32768;
        let dictID: u32 = if params.dictID != 0 { params.dictID } else { compliantID };
        mem_write_le32(header.as_mut_ptr().add(4) as *mut c_void, dictID);
    }
    hSize = 8;

    /* entropy tables */
    {
        let eSize = ZDICT_analyzeEntropy(
            header.as_mut_ptr().add(hSize) as *mut c_void,
            HBUFFSIZE - hSize,
            compressionLevel,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            customDictContent,
            dictContentSize,
            notificationLevel,
        );
        if ZDICT_isError(eSize) != 0 {
            return eSize;
        }
        hSize += eSize;
    }

    /* Shrink the content size if it doesn't fit in the buffer */
    if hSize + dictContentSize > dictBufferCapacity {
        dictContentSize = dictBufferCapacity - hSize;
    }

    /* Pad the dictionary content with zeros if it is too small */
    if dictContentSize < minContentSize {
        if hSize + minContentSize > dictBufferCapacity {
            return error(code::DSTSIZE_TOOSMALL);
        }
        paddingSize = minContentSize - dictContentSize;
    } else {
        paddingSize = 0;
    }

    {
        let dictSize: usize = hSize + paddingSize + dictContentSize;

        /* The dictionary consists of the header, optional padding, and the content. */
        let outDictHeader = dictBuffer as *mut u8;
        let outDictPadding = outDictHeader.add(hSize);
        let outDictContent = outDictPadding.add(paddingSize);

        debug_assert!(dictSize <= dictBufferCapacity);

        /* Copy customDictContent into its final location first (may overlap). */
        memmove(outDictContent as *mut c_void, customDictContent, dictContentSize);
        memcpy(outDictHeader as *mut c_void, header.as_ptr() as *const c_void, hSize);
        memset(outDictPadding as *mut c_void, 0, paddingSize);

        dictSize
    }
}

unsafe fn ZDICT_addEntropyTablesFromBuffer_advanced(
    dictBuffer: *mut c_void,
    dictContentSize: usize,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    params: ZDICT_params_t,
) -> usize {
    let compressionLevel: c_int = if params.compressionLevel == 0 {
        ZSTD_CLEVEL_DEFAULT
    } else {
        params.compressionLevel
    };
    let notificationLevel: u32 = params.notificationLevel;
    let mut hSize: usize = 8;

    /* calculate entropy tables */
    {
        let eSize = ZDICT_analyzeEntropy(
            (dictBuffer as *mut c_char).add(hSize) as *mut c_void,
            dictBufferCapacity - hSize,
            compressionLevel,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            (dictBuffer as *const c_char).add(dictBufferCapacity - dictContentSize) as *const c_void,
            dictContentSize,
            notificationLevel,
        );
        if ZDICT_isError(eSize) != 0 {
            return eSize;
        }
        hSize += eSize;
    }

    /* add dictionary header (after entropy tables) */
    mem_write_le32(dictBuffer, ZSTD_MAGIC_DICTIONARY);
    {
        let randomID: u64 = ZSTD_XXH64(
            (dictBuffer as *const c_char).add(dictBufferCapacity - dictContentSize) as *const c_void,
            dictContentSize,
            0,
        );
        let compliantID: u32 = (randomID % ((1u32 << 31) - 32768) as u64) as u32 + 32768;
        let dictID: u32 = if params.dictID != 0 { params.dictID } else { compliantID };
        mem_write_le32((dictBuffer as *mut c_char).add(4) as *mut c_void, dictID);
    }

    if hSize + dictContentSize < dictBufferCapacity {
        memmove(
            (dictBuffer as *mut c_char).add(hSize) as *mut c_void,
            (dictBuffer as *const c_char).add(dictBufferCapacity - dictContentSize) as *const c_void,
            dictContentSize,
        );
    }
    MIN(dictBufferCapacity, hSize + dictContentSize)
}

/* Warning : `samplesBuffer` must be followed by noisy guard band !!! */
unsafe fn ZDICT_trainFromBuffer_unsafe_legacy(
    dictBuffer: *mut c_void,
    maxDictSize: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    params: ZDICT_legacy_params_t,
) -> usize {
    let dictListSize: u32 = MAX_u32(
        MAX_u32(DICTLISTSIZE_DEFAULT, nbSamples),
        (maxDictSize / 16) as u32,
    );
    let dictList = malloc(dictListSize as usize * core::mem::size_of::<dictItem>()) as *mut dictItem;
    let selectivity: c_uint = if params.selectivityLevel == 0 {
        g_selectivity_default
    } else {
        params.selectivityLevel
    };
    let minRep: c_uint = if selectivity > 30 {
        MINRATIO
    } else {
        nbSamples >> selectivity
    };
    let targetDictSize: usize = maxDictSize;
    let samplesBuffSize: usize = ZDICT_totalSampleSize(samplesSizes, nbSamples);
    let mut dictSize: usize = 0;
    let notificationLevel: u32 = params.zParams.notificationLevel;

    /* checks */
    if dictList.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    if maxDictSize < ZDICT_DICTSIZE_MIN {
        free(dictList as *mut c_void);
        return error(code::DSTSIZE_TOOSMALL);
    }
    if samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE {
        free(dictList as *mut c_void);
        return error(code::DICTIONARYCREATION_FAILED);
    }

    /* init */
    ZDICT_initDictItem(dictList);

    /* build dictionary */
    ZDICT_trainBuffer_legacy(
        dictList,
        dictListSize,
        samplesBuffer,
        samplesBuffSize,
        samplesSizes,
        nbSamples,
        minRep,
        notificationLevel,
    );

    /* display best matches */
    if params.zParams.notificationLevel >= 3 {
        let nb: c_uint = MIN(25, (*dictList.add(0)).pos as usize) as c_uint;
        let _dictContentSize: c_uint = ZDICT_dictSize(dictList);
        let mut u: c_uint = 1;
        while u < nb {
            let pos: c_uint = (*dictList.add(u as usize)).pos;
            let length: c_uint = (*dictList.add(u as usize)).length;
            let printedLength: u32 = MIN(40, length as usize) as u32;
            if (pos as usize > samplesBuffSize)
                || ((pos as usize + length as usize) > samplesBuffSize)
            {
                free(dictList as *mut c_void);
                return error(code::GENERIC);
            }
            ZDICT_printHex(
                (samplesBuffer as *const c_char).add(pos as usize) as *const c_void,
                printedLength as usize,
            );
            u += 1;
        }
    }

    /* create dictionary */
    {
        let mut dictContentSize: c_uint = ZDICT_dictSize(dictList);
        if (dictContentSize as usize) < ZDICT_CONTENTSIZE_MIN {
            free(dictList as *mut c_void);
            return error(code::DICTIONARYCREATION_FAILED);
        }
        if (dictContentSize as usize) < targetDictSize / 4 {
            if minRep > MINRATIO {
                // notification only
            }
        }

        if (dictContentSize as usize > targetDictSize * 3)
            && (nbSamples > 2 * MINRATIO)
            && (selectivity > 1)
        {
            let mut proposedSelectivity: c_uint = selectivity - 1;
            while (nbSamples >> proposedSelectivity) <= MINRATIO {
                proposedSelectivity -= 1;
            }
        }

        /* limit dictionary size */
        {
            let max: u32 = (*dictList).pos; /* convention : nb of useful elts within dictList */
            let mut currentSize: u32 = 0;
            let mut n: u32 = 1;
            while n < max {
                currentSize += (*dictList.add(n as usize)).length;
                if currentSize > targetDictSize as u32 {
                    currentSize -= (*dictList.add(n as usize)).length;
                    break;
                }
                n += 1;
            }
            (*dictList).pos = n;
            dictContentSize = currentSize;
        }

        /* build dict content */
        {
            let mut u: u32 = 1;
            let mut ptr = (dictBuffer as *mut u8).add(maxDictSize);
            while u < (*dictList).pos {
                let l: u32 = (*dictList.add(u as usize)).length;
                ptr = ptr.sub(l as usize);
                if (ptr as *const u8) < (dictBuffer as *const u8) {
                    free(dictList as *mut c_void);
                    return error(code::GENERIC);
                }
                memcpy(
                    ptr as *mut c_void,
                    (samplesBuffer as *const c_char)
                        .add((*dictList.add(u as usize)).pos as usize) as *const c_void,
                    l as usize,
                );
                u += 1;
            }
        }

        dictSize = ZDICT_addEntropyTablesFromBuffer_advanced(
            dictBuffer,
            dictContentSize as usize,
            maxDictSize,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            params.zParams,
        );
    }

    /* clean up */
    free(dictList as *mut c_void);
    dictSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_legacy(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    params: ZDICT_legacy_params_t,
) -> usize {
    let result: usize;
    let newBuff: *mut c_void;
    let sBuffSize: usize = ZDICT_totalSampleSize(samplesSizes, nbSamples);
    if sBuffSize < ZDICT_MIN_SAMPLES_SIZE {
        return 0; /* not enough content => no dictionary */
    }

    newBuff = malloc(sBuffSize + NOISELENGTH);
    if newBuff.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }

    memcpy(newBuff, samplesBuffer, sBuffSize);
    ZDICT_fillNoise((newBuff as *mut c_char).add(sBuffSize) as *mut c_void, NOISELENGTH);

    result = ZDICT_trainFromBuffer_unsafe_legacy(
        dictBuffer,
        dictBufferCapacity,
        newBuff,
        samplesSizes,
        nbSamples,
        params,
    );
    free(newBuff);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
) -> usize {
    let mut params: ZDICT_fastCover_params_t = core::mem::zeroed();
    params.d = 8;
    params.steps = 4;
    /* Use default level since no compression level information is available */
    params.zParams.compressionLevel = ZSTD_CLEVEL_DEFAULT;
    ZDICT_optimizeTrainFromBuffer_fastCover(
        dictBuffer,
        dictBufferCapacity,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        &mut params,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_addEntropyTablesFromBuffer(
    dictBuffer: *mut c_void,
    dictContentSize: usize,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
) -> usize {
    let params: ZDICT_params_t = core::mem::zeroed();
    ZDICT_addEntropyTablesFromBuffer_advanced(
        dictBuffer,
        dictContentSize,
        dictBufferCapacity,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        params,
    )
}

