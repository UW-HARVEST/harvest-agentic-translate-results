/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

//! Rust translation of `c_src/src/dictBuilder/zdict.c` (+ `include/zdict.h`).
//!
//! Notes on faithfulness:
//! * Built with `ZDICT_STATIC_LINKING_ONLY`, `DEBUGLEVEL==0` (so `DEBUGLOG` /
//!   `assert` compile to nothing and are dropped), `ZSTD_TRACE==1`.
//! * `DISPLAY` / `DISPLAYLEVEL` / `DISPLAYUPDATE` print to stderr gated on
//!   `notificationLevel`. They are faithfully reproduced via libc `fprintf`.
//!   All public entry points that don't take a params struct use
//!   `notificationLevel==0`, which silences them.
//! * The cover / fastCover entry points (ZDICT_trainFromBuffer_cover,
//!   ZDICT_optimizeTrainFromBuffer_cover, ZDICT_trainFromBuffer_fastCover,
//!   ZDICT_optimizeTrainFromBuffer_fastCover) live in cover.c / fastcover.c and
//!   are owned by a concurrent agent; ZDICT_optimizeTrainFromBuffer_fastCover is
//!   forward-declared here.

use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_internal::{MAX, MIN};
use core::ffi::{c_char, c_double, c_int, c_uchar, c_uint, c_ulonglong, c_void};

use crate::common::bits::ZSTD_highbit32;
use crate::common::error_private::{
    ERR_getErrorName, ERR_isError, ERROR, ZSTD_error_GENERIC, ZSTD_error_dictionaryCreation_failed,
    ZSTD_error_dictionary_corrupted, ZSTD_error_dstSize_tooSmall, ZSTD_error_memory_allocation,
};
use crate::common::mem::{
    size_t, MEM_read16, MEM_read64, MEM_readLE32, MEM_readST, MEM_writeLE32, BYTE, S16, U16, U32,
    U64,
};
use crate::common::zstd_h::{
    ZSTD_compressionParameters, ZSTD_dct_rawContent, ZSTD_dlm_byRef, ZSTD_parameters,
    ZSTD_BLOCKSIZE_MAX, ZSTD_CLEVEL_DEFAULT, ZSTD_MAGIC_DICTIONARY,
};
use crate::common::zstd_internal::{
    clock, free, malloc, memcpy, memmove, memset, repStartValue, ZSTD_customMem, ZSTD_defaultCMem,
    ZSTD_REP_NUM,
};
use crate::common::xxhash::XXH64;

use crate::common::entropy_common::{FSE_isError, HUF_isError};
use crate::common::huf::{HUF_CElt, HUF_CTABLE_SIZE_ST, HUF_CTABLE_WORKSPACE_SIZE_U32};
use crate::common::zstd_internal::{LLFSELog, MLFSELog, MaxLL, MaxML, OffFSELog};

use crate::compress::fse_compress::{FSE_normalizeCount, FSE_writeNCount};
use crate::compress::huf_compress::{HUF_buildCTable_wksp, HUF_writeCTable_wksp};

use crate::compress::zstd_compress_internal::{
    SeqDef, SeqStore_t, ZSTD_CCtx, ZSTD_CDict, ZSTD_compressBegin_usingCDict_deprecated,
    ZSTD_compressBlock_deprecated, ZSTD_compressedBlockState_t, ZSTD_getSeqStore,
    ZSTD_loadCEntropy, ZSTD_reset_compressedBlockState, ZSTD_seqToCodes,
};

use crate::dictbuilder::divsufsort::divsufsort;

/* =============================================================================
 * SHARED TYPES from the public header include/zdict.h.
 *
 * `ZDICT_params_t`, `ZDICT_cover_params_t`, `ZDICT_fastCover_params_t` are defined
 * by the concurrent agent in `crate::dictbuilder::cover`. We import them from there.
 * `ZDICT_legacy_params_t` is defined here (this file's responsibility).
 * =========================================================================== */
use crate::dictbuilder::cover::{ZDICT_cover_params_t, ZDICT_fastCover_params_t, ZDICT_params_t};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZDICT_legacy_params_t {
    pub selectivityLevel: c_uint, /* 0 means default; larger => select more => larger dictionary */
    pub zParams: ZDICT_params_t,
}

/*-*************************************
*  Constants from include/zdict.h
***************************************/
pub const ZDICT_DICTSIZE_MIN: size_t = 256;
/* Deprecated: Remove in v1.6.0 */
pub const ZDICT_CONTENTSIZE_MIN: size_t = 128;

/* ======  Forward declaration of the fastCover entry point (cover/fastcover.c) ====== */
unsafe extern "C" {
    fn ZDICT_optimizeTrainFromBuffer_fastCover(
        dictBuffer: *mut c_void,
        dictBufferCapacity: size_t,
        samplesBuffer: *const c_void,
        samplesSizes: *const size_t,
        nbSamples: c_uint,
        parameters: *mut ZDICT_fastCover_params_t,
    ) -> size_t;
}

/* ======  Forward declarations of compress-side symbols (zstd_compress.c) ====== */
unsafe extern "C" {
    fn ZSTD_getParams(
        compressionLevel: c_int,
        estimatedSrcSize: core::ffi::c_ulonglong,
        dictSize: size_t,
    ) -> ZSTD_parameters;
    fn ZSTD_createCDict_advanced(
        dict: *const c_void,
        dictSize: size_t,
        dictLoadMethod: c_uint,
        dictContentType: c_uint,
        cParams: ZSTD_compressionParameters,
        customMem: ZSTD_customMem,
    ) -> *mut ZSTD_CDict;
    fn ZSTD_freeCDict(cdict: *mut ZSTD_CDict) -> size_t;
    fn ZSTD_createCCtx() -> *mut ZSTD_CCtx;
    fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> size_t;
}

/* ======  libc stdio for DISPLAY (faithful stderr output) ====== */
unsafe extern "C" {
    static mut stderr: *mut c_void;
    fn fprintf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/* HUF_WORKSPACE_SIZE from huf.h : ((8 << 10) + 512) */
const HUF_WORKSPACE_SIZE: size_t = (8 << 10) + 512;

/* POSIX mandates CLOCKS_PER_SEC == 1000000 */
const CLOCKS_PER_SEC: i64 = 1000000;

/*-**************************************
*  Tuning parameters
****************************************/
const MINRATIO: c_uint = 4; /* minimum nb of apparition to be selected in dictionary */
const ZDICT_MAX_SAMPLES_SIZE: size_t = (2000u32 as size_t) << 20;
const ZDICT_MIN_SAMPLES_SIZE: size_t = ZDICT_CONTENTSIZE_MIN * MINRATIO as size_t;

/*-*************************************
*  Constants
***************************************/
const DICTLISTSIZE_DEFAULT: U32 = 10000;

const NOISELENGTH: size_t = 32;

static g_selectivity_default: U32 = 9;

/*-*************************************
*  Console display
***************************************/
/* #define DISPLAY(...) do { fprintf(stderr, __VA_ARGS__); fflush(stderr); } while(0) */
macro_rules! DISPLAY {
    ($($arg:tt)*) => {{
        fprintf(stderr, $($arg)*);
        fflush(stderr);
    }};
}
/* DISPLAYLEVEL(l, ...) : if (notificationLevel>=l) DISPLAY(...)
 * `notificationLevel` must be in scope at the call site (matching the C macro). */
macro_rules! DISPLAYLEVEL {
    ($nl:expr, $l:expr, $($arg:tt)*) => {{
        if $nl >= $l {
            DISPLAY!($($arg)*);
        }
    }};
}

unsafe fn ZDICT_clockSpan(nPrevious: i64) -> i64 {
    clock() - nPrevious
}

unsafe fn ZDICT_printHex(ptr: *const c_void, length: size_t) {
    let b: *const BYTE = ptr as *const BYTE;
    let mut u: size_t = 0;
    while u < length {
        let mut c: BYTE = *b.add(u as usize);
        if c < 32 || c > 126 {
            c = b'.';
        } /* non-printable char */
        DISPLAY!(b"%c\0".as_ptr() as *const c_char, c as c_int);
        u += 1;
    }
}

/*-********************************************************
*  Helper functions
**********************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_isError(errorCode: size_t) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getErrorName(errorCode: size_t) -> *const c_char {
    ERR_getErrorName(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getDictID(dictBuffer: *const c_void, dictSize: size_t) -> c_uint {
    if dictSize < 8 {
        return 0;
    }
    if MEM_readLE32(dictBuffer as *const u8) != ZSTD_MAGIC_DICTIONARY {
        return 0;
    }
    MEM_readLE32((dictBuffer as *const c_char).wrapping_add(4) as *const u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getDictHeaderSize(
    dictBuffer: *const c_void,
    dictSize: size_t,
) -> size_t {
    let headerSize: size_t;
    if dictSize <= 8 || MEM_readLE32(dictBuffer as *const u8) != ZSTD_MAGIC_DICTIONARY {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    {
        let bs: *mut ZSTD_compressedBlockState_t =
            malloc(core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t)
                as *mut ZSTD_compressedBlockState_t;
        let wksp: *mut U32 = malloc(HUF_WORKSPACE_SIZE) as *mut U32;
        if bs.is_null() || wksp.is_null() {
            headerSize = ERROR(ZSTD_error_memory_allocation);
        } else {
            ZSTD_reset_compressedBlockState(bs);
            headerSize = ZSTD_loadCEntropy(bs, wksp as *mut c_void, dictBuffer, dictSize);
        }

        free(bs as *mut c_void);
        free(wksp as *mut c_void);

        headerSize
    }
}

/*-********************************************************
*  Dictionary training functions
**********************************************************/
/* ZDICT_count() :
    Count the nb of common bytes between 2 pointers.
    Note : this function presumes end of buffer followed by noisy guard band.
*/
unsafe fn ZDICT_count(pIn: *const c_void, pMatch: *const c_void) -> size_t {
    let pStart: *const c_char = pIn as *const c_char;
    let mut pIn = pIn;
    let mut pMatch = pMatch;
    loop {
        let diff: size_t = MEM_readST(pMatch as *const u8) ^ MEM_readST(pIn as *const u8);
        if diff == 0 {
            pIn = (pIn as *const c_char).wrapping_add(core::mem::size_of::<size_t>()) as *const c_void;
            pMatch =
                (pMatch as *const c_char).wrapping_add(core::mem::size_of::<size_t>()) as *const c_void;
            continue;
        }
        pIn = (pIn as *const c_char)
            .wrapping_add(crate::common::bits::ZSTD_NbCommonBytes(diff) as usize)
            as *const c_void;
        return ((pIn as *const c_char) as isize - (pStart as isize)) as size_t;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct dictItem {
    pos: U32,
    length: U32,
    savings: U32,
}

unsafe fn ZDICT_initDictItem(d: *mut dictItem) {
    (*d).pos = 1;
    (*d).length = 0;
    (*d).savings = (-1i32) as U32;
}

const LLIMIT: usize = 64; /* heuristic determined experimentally */
const MINMATCHLENGTH: usize = 7; /* heuristic determined experimentally */

unsafe fn ZDICT_analyzePos(
    doneMarks: *mut BYTE,
    suffix: *const c_int,
    start: U32,
    buffer: *const c_void,
    minRatio: U32,
    notificationLevel: U32,
) -> dictItem {
    let mut lengthList: [U32; LLIMIT] = [0; LLIMIT];
    let mut cumulLength: [U32; LLIMIT] = [0; LLIMIT];
    let mut savings: [U32; LLIMIT] = [0; LLIMIT];
    let b: *const BYTE = buffer as *const BYTE;
    let mut maxLength: size_t = LLIMIT as size_t;
    let mut pos: size_t = *suffix.add(start as usize) as size_t;
    let mut end: U32 = start;
    let mut start = start;
    let mut solution: dictItem = dictItem {
        pos: 0,
        length: 0,
        savings: 0,
    };

    /* init */
    memset(
        &mut solution as *mut dictItem as *mut c_void,
        0,
        core::mem::size_of::<dictItem>() as size_t,
    );
    *doneMarks.add(pos as usize) = 1;

    /* trivial repetition cases */
    if (MEM_read16(b.add(pos as usize + 0)) == MEM_read16(b.add(pos as usize + 2)))
        || (MEM_read16(b.add(pos as usize + 1)) == MEM_read16(b.add(pos as usize + 3)))
        || (MEM_read16(b.add(pos as usize + 2)) == MEM_read16(b.add(pos as usize + 4)))
    {
        /* skip and mark segment */
        let pattern16: U16 = MEM_read16(b.add(pos as usize + 4));
        let mut u: U32;
        let mut patternEnd: U32 = 6;
        while MEM_read16(b.add(pos as usize + patternEnd as usize)) == pattern16 {
            patternEnd += 2;
        }
        if *b.add(pos as usize + patternEnd as usize) == *b.add(pos as usize + patternEnd as usize - 1)
        {
            patternEnd += 1;
        }
        u = 1;
        while u < patternEnd {
            *doneMarks.add(pos as usize + u as usize) = 1;
            u += 1;
        }
        return solution;
    }

    /* look forward */
    {
        let mut length: size_t;
        loop {
            end += 1;
            length = ZDICT_count(
                b.add(pos as usize) as *const c_void,
                b.add(*suffix.add(end as usize) as usize) as *const c_void,
            );
            if !(length >= MINMATCHLENGTH as size_t) {
                break;
            }
        }
    }

    /* look backward */
    {
        let mut length: size_t;
        loop {
            length = ZDICT_count(
                b.add(pos as usize) as *const c_void,
                b.add(*suffix.wrapping_offset(start as isize - 1) as usize) as *const c_void,
            );
            if length >= MINMATCHLENGTH as size_t {
                start -= 1;
            }
            if !(length >= MINMATCHLENGTH as size_t) {
                break;
            }
        }
    }

    /* exit if not found a minimum nb of repetitions */
    if end - start < minRatio {
        let mut idx: U32 = start;
        while idx < end {
            *doneMarks.add(*suffix.add(idx as usize) as usize) = 1;
            idx += 1;
        }
        return solution;
    }

    {
        let mut i: c_int;
        let mut mml: U32;
        let mut refinedStart: U32 = start;
        let mut refinedEnd: U32 = end;

        DISPLAYLEVEL!(notificationLevel, 4, b"\n\0".as_ptr() as *const c_char);
        DISPLAYLEVEL!(
            notificationLevel,
            4,
            b"found %3u matches of length >= %i at pos %7u  \0".as_ptr() as *const c_char,
            (end - start) as c_uint,
            MINMATCHLENGTH as c_int,
            pos as c_uint
        );
        DISPLAYLEVEL!(notificationLevel, 4, b"\n\0".as_ptr() as *const c_char);

        mml = MINMATCHLENGTH as U32;
        loop {
            let mut currentChar: BYTE = 0;
            let mut currentCount: U32 = 0;
            let mut currentID: U32 = refinedStart;
            let mut id: U32;
            let mut selectedCount: U32 = 0;
            let mut selectedID: U32 = currentID;
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
        pos = *suffix.add(refinedStart as usize) as size_t;
        end = start;
        memset(
            lengthList.as_mut_ptr() as *mut c_void,
            0,
            core::mem::size_of_val(&lengthList) as size_t,
        );

        /* look forward */
        {
            let mut length: size_t;
            loop {
                end += 1;
                length = ZDICT_count(
                    b.add(pos as usize) as *const c_void,
                    b.add(*suffix.add(end as usize) as usize) as *const c_void,
                );
                if length >= LLIMIT as size_t {
                    length = LLIMIT as size_t - 1;
                }
                lengthList[length as usize] += 1;
                if !(length >= MINMATCHLENGTH as size_t) {
                    break;
                }
            }
        }

        /* look backward */
        {
            let mut length: size_t = MINMATCHLENGTH as size_t;
            while (length >= MINMATCHLENGTH as size_t) & (start > 0) {
                length = ZDICT_count(
                    b.add(pos as usize) as *const c_void,
                    b.add(*suffix.add(start as usize - 1) as usize) as *const c_void,
                );
                if length >= LLIMIT as size_t {
                    length = LLIMIT as size_t - 1;
                }
                lengthList[length as usize] += 1;
                if length >= MINMATCHLENGTH as size_t {
                    start -= 1;
                }
            }
        }

        /* largest useful length */
        memset(
            cumulLength.as_mut_ptr() as *mut c_void,
            0,
            core::mem::size_of_val(&cumulLength) as size_t,
        );
        cumulLength[maxLength as usize - 1] = lengthList[maxLength as usize - 1];
        i = (maxLength as c_int) - 2;
        while i >= 0 {
            cumulLength[i as usize] = cumulLength[i as usize + 1] + lengthList[i as usize];
            i -= 1;
        }

        i = LLIMIT as c_int - 1;
        while i >= MINMATCHLENGTH as c_int {
            if cumulLength[i as usize] >= minRatio {
                break;
            }
            i -= 1;
        }
        maxLength = i as size_t;

        /* reduce maxLength in case of final into repetitive data */
        {
            let mut l: U32 = maxLength as U32;
            let c: BYTE = *b.add(pos as usize + maxLength as usize - 1);
            while *b.add(pos as usize + l as usize - 2) == c {
                l -= 1;
            }
            maxLength = l as size_t;
        }
        if maxLength < MINMATCHLENGTH as size_t {
            return solution;
        } /* skip : no long-enough solution */

        /* calculate savings */
        savings[5] = 0;
        i = MINMATCHLENGTH as c_int;
        while i <= maxLength as c_int {
            savings[i as usize] =
                savings[i as usize - 1] + (lengthList[i as usize] * (i as U32 - 3));
            i += 1;
        }

        DISPLAYLEVEL!(
            notificationLevel,
            4,
            b"Selected dict at position %u, of length %u : saves %u (ratio: %.2f)  \n\0".as_ptr()
                as *const c_char,
            pos as c_uint,
            maxLength as c_uint,
            savings[maxLength as usize] as c_uint,
            savings[maxLength as usize] as c_double / maxLength as c_double
        );

        solution.pos = pos as U32;
        solution.length = maxLength as U32;
        solution.savings = savings[maxLength as usize];

        /* mark positions done */
        {
            let mut id: U32;
            id = start;
            while id < end {
                let mut p: U32;
                let pEnd: U32;
                let mut length: U32;
                let testedPos: U32 = *suffix.add(id as usize) as U32;
                if testedPos == pos as U32 {
                    length = solution.length;
                } else {
                    length = ZDICT_count(
                        b.add(pos as usize) as *const c_void,
                        b.add(testedPos as usize) as *const c_void,
                    ) as U32;
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

unsafe fn isIncluded(inptr: *const c_void, container: *const c_void, length: size_t) -> c_int {
    let ip: *const c_char = inptr as *const c_char;
    let into: *const c_char = container as *const c_char;
    let mut u: size_t = 0;

    while u < length {
        /* works because end of buffer is a noisy guard band */
        if *ip.add(u as usize) != *into.add(u as usize) {
            break;
        }
        u += 1;
    }

    (u == length) as c_int
}

/* ZDICT_tryMerge() :
    check if dictItem can be merged, do it if possible
    @return : id of destination elt, 0 if not merged
*/
unsafe fn ZDICT_tryMerge(
    table: *mut dictItem,
    mut elt: dictItem,
    eltNbToSkip: U32,
    buffer: *const c_void,
) -> U32 {
    let tableSize: U32 = (*table).pos;
    let eltEnd: U32 = elt.pos + elt.length;
    let buf: *const c_char = buffer as *const c_char;

    /* tail overlap */
    {
        let mut u: U32 = 1;
        while u < tableSize {
            if u == eltNbToSkip {
                u += 1;
                continue;
            }
            if ((*table.add(u as usize)).pos > elt.pos)
                && ((*table.add(u as usize)).pos <= eltEnd)
            {
                /* overlap, existing > new */
                /* append */
                let addedLength: U32 = (*table.add(u as usize)).pos - elt.pos;
                (*table.add(u as usize)).length += addedLength;
                (*table.add(u as usize)).pos = elt.pos;
                (*table.add(u as usize)).savings += elt.savings * addedLength / elt.length; /* rough approx */
                (*table.add(u as usize)).savings += elt.length / 8; /* rough approx bonus */
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
    }

    /* front overlap */
    {
        let mut u: U32 = 1;
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
                let addedLength: c_int = (eltEnd as c_int)
                    - ((*table.add(u as usize)).pos + (*table.add(u as usize)).length) as c_int;
                (*table.add(u as usize)).savings += elt.length / 8; /* rough approx bonus */
                if addedLength > 0 {
                    /* otherwise, elt fully included into existing */
                    (*table.add(u as usize)).length += addedLength as U32;
                    (*table.add(u as usize)).savings +=
                        elt.savings * addedLength as U32 / elt.length; /* rough approx */
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

            if MEM_read64(buf.add((*table.add(u as usize)).pos as usize) as *const u8)
                == MEM_read64(buf.add(elt.pos as usize + 1) as *const u8)
            {
                if isIncluded(
                    buf.add((*table.add(u as usize)).pos as usize) as *const c_void,
                    buf.add(elt.pos as usize + 1) as *const c_void,
                    (*table.add(u as usize)).length as size_t,
                ) != 0
                {
                    let addedLength: size_t = MAX(
                        elt.length as c_int - (*table.add(u as usize)).length as c_int,
                        1,
                    ) as size_t;
                    (*table.add(u as usize)).pos = elt.pos;
                    (*table.add(u as usize)).savings +=
                        (elt.savings as size_t * addedLength / elt.length as size_t) as U32;
                    (*table.add(u as usize)).length =
                        MIN(elt.length, (*table.add(u as usize)).length + 1);
                    return u;
                }
            }
            u += 1;
        }
    }

    0
}

unsafe fn ZDICT_removeDictItem(table: *mut dictItem, id: U32) {
    /* convention : table[0].pos stores nb of elts */
    let max: U32 = (*table.add(0)).pos;
    let mut u: U32;
    if id == 0 {
        return;
    } /* protection, should never happen */
    u = id;
    while u < max - 1 {
        *table.add(u as usize) = *table.add(u as usize + 1);
        u += 1;
    }
    (*table).pos -= 1;
}

unsafe fn ZDICT_insertDictItem(
    table: *mut dictItem,
    maxSize: U32,
    elt: dictItem,
    buffer: *const c_void,
) {
    /* merge if possible */
    let mut mergeId: U32 = ZDICT_tryMerge(table, elt, 0, buffer);
    if mergeId != 0 {
        let mut newMerge: U32 = 1;
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
        let mut current: U32;
        let mut nextElt: U32 = (*table).pos;
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

unsafe fn ZDICT_dictSize(dictList: *const dictItem) -> U32 {
    let mut u: U32;
    let mut dictSize: U32 = 0;
    u = 1;
    while u < (*dictList.add(0)).pos {
        dictSize += (*dictList.add(u as usize)).length;
        u += 1;
    }
    dictSize
}

unsafe fn ZDICT_trainBuffer_legacy(
    dictList: *mut dictItem,
    dictListSize: U32,
    buffer: *const c_void,
    mut bufferSize: size_t, /* buffer must end with noisy guard band */
    fileSizes: *const size_t,
    mut nbFiles: c_uint,
    mut minRatio: c_uint,
    notificationLevel: U32,
) -> size_t {
    let suffix0: *mut c_int =
        malloc((bufferSize + 2) * core::mem::size_of::<c_int>() as size_t) as *mut c_int;
    let suffix: *mut c_int = suffix0.wrapping_add(1);
    let reverseSuffix: *mut U32 =
        malloc(bufferSize * core::mem::size_of::<U32>() as size_t) as *mut U32;
    let doneMarks: *mut BYTE =
        malloc((bufferSize + 16) * core::mem::size_of::<BYTE>() as size_t) as *mut BYTE; /* +16 for overflow security */
    let filePos: *mut U32 =
        malloc(nbFiles as size_t * core::mem::size_of::<U32>() as size_t) as *mut U32;
    let mut result: size_t = 0;
    let mut displayClock: i64 = 0;
    let refreshRate: i64 = CLOCKS_PER_SEC * 3 / 10;

    /* DISPLAYUPDATE(l, ...) : rate-limited DISPLAY gated on notificationLevel */
    macro_rules! DISPLAYUPDATE {
        ($l:expr, $($arg:tt)*) => {{
            if notificationLevel >= $l {
                if ZDICT_clockSpan(displayClock) > refreshRate {
                    displayClock = clock();
                    DISPLAY!($($arg)*);
                }
                if notificationLevel >= 4 {
                    fflush(stderr);
                }
            }
        }};
    }

    /* init */
    DISPLAYLEVEL!(
        notificationLevel,
        2,
        b"\r%70s\r\0".as_ptr() as *const c_char,
        b"\0".as_ptr() as *const c_char
    ); /* clean display line */
    if suffix0.is_null() || reverseSuffix.is_null() || doneMarks.is_null() || filePos.is_null() {
        result = ERROR(ZSTD_error_memory_allocation);
        // goto _cleanup;
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
    if bufferSize > ZDICT_MAX_SAMPLES_SIZE {
        DISPLAYLEVEL!(
            notificationLevel,
            3,
            b"sample set too large : reduced to %u MB ...\n\0".as_ptr() as *const c_char,
            (ZDICT_MAX_SAMPLES_SIZE >> 20) as c_uint
        );
    }
    while bufferSize > ZDICT_MAX_SAMPLES_SIZE {
        nbFiles -= 1;
        bufferSize -= *fileSizes.add(nbFiles as usize);
    }

    /* sort */
    DISPLAYLEVEL!(
        notificationLevel,
        2,
        b"sorting %u files of total size %u MB ...\n\0".as_ptr() as *const c_char,
        nbFiles,
        (bufferSize >> 20) as c_uint
    );
    {
        let divSuftSortResult: c_int =
            divsufsort(buffer as *const c_uchar, suffix, bufferSize as c_int, 0);
        if divSuftSortResult != 0 {
            result = ERROR(ZSTD_error_GENERIC);
            free(suffix0 as *mut c_void);
            free(reverseSuffix as *mut c_void);
            free(doneMarks as *mut c_void);
            free(filePos as *mut c_void);
            return result;
        }
    }
    *suffix.add(bufferSize as usize) = bufferSize as c_int; /* leads into noise */
    *suffix0.add(0) = bufferSize as c_int; /* leads into noise */
    /* build reverse suffix sort */
    {
        let mut pos: size_t;
        pos = 0;
        while pos < bufferSize {
            *reverseSuffix.add(*suffix.add(pos as usize) as usize) = pos as U32;
            pos += 1;
        }
        /* note filePos tracks borders between samples. */
        *filePos.add(0) = 0;
        pos = 1;
        while pos < nbFiles as size_t {
            *filePos.add(pos as usize) =
                (*filePos.add(pos as usize - 1) + *fileSizes.add(pos as usize - 1) as U32) as U32;
            pos += 1;
        }
    }

    DISPLAYLEVEL!(
        notificationLevel,
        2,
        b"finding patterns ... \n\0".as_ptr() as *const c_char
    );
    DISPLAYLEVEL!(
        notificationLevel,
        3,
        b"minimum ratio : %u \n\0".as_ptr() as *const c_char,
        minRatio
    );

    {
        let mut cursor: U32 = 0;
        while (cursor as size_t) < bufferSize {
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
            DISPLAYUPDATE!(
                2,
                b"\r%4.2f %% \r\0".as_ptr() as *const c_char,
                cursor as c_double / bufferSize as c_double * 100.0
            );
        }
    }

    // _cleanup:
    free(suffix0 as *mut c_void);
    free(reverseSuffix as *mut c_void);
    free(doneMarks as *mut c_void);
    free(filePos as *mut c_void);
    result
}

unsafe fn ZDICT_fillNoise(buffer: *mut c_void, length: size_t) {
    let prime1: c_uint = 2654435761u32;
    let prime2: c_uint = 2246822519u32;
    let mut acc: c_uint = prime1;
    let mut p: size_t = 0;
    while p < length {
        acc = acc.wrapping_mul(prime2);
        *(buffer as *mut u8).add(p as usize) = (acc >> 21) as u8;
        p += 1;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EStats_ress_t {
    dict: *mut ZSTD_CDict,   /* dictionary */
    zc: *mut ZSTD_CCtx,      /* working context */
    workPlace: *mut c_void,  /* must be ZSTD_BLOCKSIZE_MAX allocated */
}

const MAXREPOFFSET: usize = 1024;

unsafe fn ZDICT_countEStats(
    esr: EStats_ress_t,
    params: *const ZSTD_parameters,
    countLit: *mut c_uint,
    offsetcodeCount: *mut c_uint,
    matchlengthCount: *mut c_uint,
    litlengthCount: *mut c_uint,
    repOffsets: *mut U32,
    src: *const c_void,
    mut srcSize: size_t,
    notificationLevel: U32,
) {
    let blockSizeMax: size_t = MIN(
        ZSTD_BLOCKSIZE_MAX as size_t,
        (1usize as size_t) << (*params).cParams.windowLog,
    );
    let cSize: size_t;

    if srcSize > blockSizeMax {
        srcSize = blockSizeMax;
    } /* protection vs large samples */
    {
        let errorCode: size_t = ZSTD_compressBegin_usingCDict_deprecated(esr.zc, esr.dict);
        if ZSTD_isError(errorCode) != 0 {
            DISPLAYLEVEL!(
                notificationLevel,
                1,
                b"warning : ZSTD_compressBegin_usingCDict failed \n\0".as_ptr() as *const c_char
            );
            return;
        }
    }
    cSize = ZSTD_compressBlock_deprecated(
        esr.zc,
        esr.workPlace,
        ZSTD_BLOCKSIZE_MAX as size_t,
        src,
        srcSize,
    );
    if ZSTD_isError(cSize) != 0 {
        DISPLAYLEVEL!(
            notificationLevel,
            3,
            b"warning : could not compress sample size %u \n\0".as_ptr() as *const c_char,
            srcSize as c_uint
        );
        return;
    }

    if cSize != 0 {
        /* if == 0; block is not compressible */
        let seqStorePtr: *const SeqStore_t = ZSTD_getSeqStore(esr.zc);

        /* literals stats */
        {
            let mut bytePtr: *const BYTE = (*seqStorePtr).litStart;
            while bytePtr < (*seqStorePtr).lit {
                *countLit.add(*bytePtr as usize) += 1;
                bytePtr = bytePtr.add(1);
            }
        }

        /* seqStats */
        {
            let nbSeq: U32 = ((*seqStorePtr).sequences as isize
                - (*seqStorePtr).sequencesStart as isize)
                as usize as U32
                / core::mem::size_of::<SeqDef>() as U32;
            ZSTD_seqToCodes(seqStorePtr);

            {
                let codePtr: *const BYTE = (*seqStorePtr).ofCode;
                let mut u: U32 = 0;
                while u < nbSeq {
                    *offsetcodeCount.add(*codePtr.add(u as usize) as usize) += 1;
                    u += 1;
                }
            }

            {
                let codePtr: *const BYTE = (*seqStorePtr).mlCode;
                let mut u: U32 = 0;
                while u < nbSeq {
                    *matchlengthCount.add(*codePtr.add(u as usize) as usize) += 1;
                    u += 1;
                }
            }

            {
                let codePtr: *const BYTE = (*seqStorePtr).llCode;
                let mut u: U32 = 0;
                while u < nbSeq {
                    *litlengthCount.add(*codePtr.add(u as usize) as usize) += 1;
                    u += 1;
                }
            }

            if nbSeq >= 2 {
                /* rep offsets */
                let seq: *const SeqDef = (*seqStorePtr).sequencesStart;
                let mut offset1: U32 = (*seq.add(0)).offBase - ZSTD_REP_NUM as U32;
                let mut offset2: U32 = (*seq.add(1)).offBase - ZSTD_REP_NUM as U32;
                if offset1 >= MAXREPOFFSET as U32 {
                    offset1 = 0;
                }
                if offset2 >= MAXREPOFFSET as U32 {
                    offset2 = 0;
                }
                *repOffsets.add(offset1 as usize) += 3;
                *repOffsets.add(offset2 as usize) += 1;
            }
        }
    }
}

unsafe fn ZDICT_totalSampleSize(fileSizes: *const size_t, nbFiles: c_uint) -> size_t {
    let mut total: size_t = 0;
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
    offset: U32,
    count: U32,
}

unsafe fn ZDICT_insertSortCount(
    table: *mut offsetCount_t, /* table[ZSTD_REP_NUM+1] */
    val: U32,
    count: U32,
) {
    let mut u: U32;
    (*table.add(ZSTD_REP_NUM)).offset = val;
    (*table.add(ZSTD_REP_NUM)).count = count;
    u = ZSTD_REP_NUM as U32;
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

/* ZDICT_flatLit() :
 * rewrite `countLit` to contain a mostly flat but still compressible distribution of literals.
 */
unsafe fn ZDICT_flatLit(countLit: *mut c_uint) {
    let mut u: c_int;
    u = 1;
    while u < 256 {
        *countLit.add(u as usize) = 2;
        u += 1;
    }
    *countLit.add(0) = 4;
    *countLit.add(253) = 1;
    *countLit.add(254) = 1;
}

const OFFCODE_MAX: usize = 30; /* only applicable to first block */

unsafe fn ZDICT_analyzeEntropy(
    dstBuffer: *mut c_void,
    mut maxDstSize: size_t,
    mut compressionLevel: c_int,
    srcBuffer: *const c_void,
    fileSizes: *const size_t,
    nbFiles: c_uint,
    dictBuffer: *const c_void,
    dictBufferSize: size_t,
    notificationLevel: c_uint,
) -> size_t {
    let mut countLit: [c_uint; 256] = [0; 256];
    let mut hufTable: [HUF_CElt; HUF_CTABLE_SIZE_ST(255) as usize] =
        [0 as HUF_CElt; HUF_CTABLE_SIZE_ST(255) as usize];
    let mut offcodeCount: [c_uint; OFFCODE_MAX + 1] = [0; OFFCODE_MAX + 1];
    let mut offcodeNCount: [S16; OFFCODE_MAX + 1] = [0; OFFCODE_MAX + 1];
    let offcodeMax: U32 = ZSTD_highbit32((dictBufferSize + (128 << 10)) as U32);
    let mut matchLengthCount: [c_uint; MaxML as usize + 1] = [0; MaxML as usize + 1];
    let mut matchLengthNCount: [S16; MaxML as usize + 1] = [0; MaxML as usize + 1];
    let mut litLengthCount: [c_uint; MaxLL as usize + 1] = [0; MaxLL as usize + 1];
    let mut litLengthNCount: [S16; MaxLL as usize + 1] = [0; MaxLL as usize + 1];
    let mut repOffset: [U32; MAXREPOFFSET] = [0; MAXREPOFFSET];
    let mut bestRepOffset: [offsetCount_t; ZSTD_REP_NUM + 1] =
        [offsetCount_t { offset: 0, count: 0 }; ZSTD_REP_NUM + 1];
    let mut esr: EStats_ress_t = EStats_ress_t {
        dict: core::ptr::null_mut(),
        zc: core::ptr::null_mut(),
        workPlace: core::ptr::null_mut(),
    };
    let mut params: ZSTD_parameters;
    let mut u: U32;
    let mut huffLog: U32 = 11;
    let mut Offlog: U32 = OffFSELog;
    let mut mlLog: U32 = MLFSELog;
    let mut llLog: U32 = LLFSELog;
    let mut total: U32;
    let mut pos: size_t = 0;
    let mut errorCode: size_t;
    let mut eSize: size_t = 0;
    let totalSrcSize: size_t = ZDICT_totalSampleSize(fileSizes, nbFiles);
    let averageSampleSize: size_t = totalSrcSize / (nbFiles + (nbFiles == 0) as c_uint) as size_t;
    let mut dstPtr: *mut BYTE = dstBuffer as *mut BYTE;
    let mut wksp: [U32; HUF_CTABLE_WORKSPACE_SIZE_U32 as usize] =
        [0; HUF_CTABLE_WORKSPACE_SIZE_U32 as usize];

    /* init */
    /* DEBUGLOG(4, "ZDICT_analyzeEntropy"); dropped (DEBUGLEVEL==0) */
    if offcodeMax > OFFCODE_MAX as U32 {
        eSize = ERROR(ZSTD_error_dictionaryCreation_failed);
        // goto _cleanup;
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    } /* too large dictionary */
    u = 0;
    while u < 256 {
        countLit[u as usize] = 1;
        u += 1;
    } /* any character must be described */
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
    memset(
        repOffset.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of_val(&repOffset) as size_t,
    );
    repOffset[1] = 1;
    repOffset[4] = 1;
    repOffset[8] = 1;
    memset(
        bestRepOffset.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of_val(&bestRepOffset) as size_t,
    );
    if compressionLevel == 0 {
        compressionLevel = ZSTD_CLEVEL_DEFAULT;
    }
    params = ZSTD_getParams(compressionLevel, averageSampleSize as c_ulonglong, dictBufferSize);

    esr.dict = ZSTD_createCDict_advanced(
        dictBuffer,
        dictBufferSize,
        ZSTD_dlm_byRef,
        ZSTD_dct_rawContent,
        params.cParams,
        ZSTD_defaultCMem,
    );
    esr.zc = ZSTD_createCCtx();
    esr.workPlace = malloc(ZSTD_BLOCKSIZE_MAX as size_t);
    if esr.dict.is_null() || esr.zc.is_null() || esr.workPlace.is_null() {
        eSize = ERROR(ZSTD_error_memory_allocation);
        DISPLAYLEVEL!(
            notificationLevel,
            1,
            b"Not enough memory \n\0".as_ptr() as *const c_char
        );
        // goto _cleanup;
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
            (srcBuffer as *const c_char).add(pos as usize) as *const c_void,
            *fileSizes.add(u as usize),
            notificationLevel,
        );
        pos += *fileSizes.add(u as usize);
        u += 1;
    }

    if notificationLevel >= 4 {
        /* writeStats */
        DISPLAYLEVEL!(
            notificationLevel,
            4,
            b"Offset Code Frequencies : \n\0".as_ptr() as *const c_char
        );
        u = 0;
        while u <= offcodeMax {
            DISPLAYLEVEL!(
                notificationLevel,
                4,
                b"%2u :%7u \n\0".as_ptr() as *const c_char,
                u,
                offcodeCount[u as usize]
            );
            u += 1;
        }
    }

    /* analyze, build stats, starting with literals */
    {
        let mut maxNbBits: size_t = HUF_buildCTable_wksp(
            hufTable.as_mut_ptr(),
            countLit.as_ptr(),
            255,
            huffLog,
            wksp.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&wksp) as size_t,
        );
        if HUF_isError(maxNbBits) != 0 {
            eSize = maxNbBits;
            DISPLAYLEVEL!(
                notificationLevel,
                1,
                b" HUF_buildCTable error \n\0".as_ptr() as *const c_char
            );
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        if maxNbBits == 8 {
            /* not compressible : will fail on HUF_writeCTable() */
            DISPLAYLEVEL!(notificationLevel, 2, b"warning : pathological dataset : literals are not compressible : samples are noisy or too regular \n\0".as_ptr() as *const c_char);
            ZDICT_flatLit(countLit.as_mut_ptr()); /* replace distribution by a fake "mostly flat but still compressible" distribution */
            maxNbBits = HUF_buildCTable_wksp(
                hufTable.as_mut_ptr(),
                countLit.as_ptr(),
                255,
                huffLog,
                wksp.as_mut_ptr() as *mut c_void,
                core::mem::size_of_val(&wksp) as size_t,
            );
            /* assert(maxNbBits==9); dropped */
        }
        huffLog = maxNbBits as U32;
    }

    /* looking for most common first offsets */
    {
        let mut offset: U32 = 1;
        while offset < MAXREPOFFSET as U32 {
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
        total as size_t,
        offcodeMax,
        1, /* useLowProbCount */
    );
    if FSE_isError(errorCode) != 0 {
        eSize = errorCode;
        DISPLAYLEVEL!(
            notificationLevel,
            1,
            b"FSE_normalizeCount error with offcodeCount \n\0".as_ptr() as *const c_char
        );
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    Offlog = errorCode as U32;

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
        total as size_t,
        MaxML,
        1, /* useLowProbCount */
    );
    if FSE_isError(errorCode) != 0 {
        eSize = errorCode;
        DISPLAYLEVEL!(
            notificationLevel,
            1,
            b"FSE_normalizeCount error with matchLengthCount \n\0".as_ptr() as *const c_char
        );
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    mlLog = errorCode as U32;

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
        total as size_t,
        MaxLL,
        1, /* useLowProbCount */
    );
    if FSE_isError(errorCode) != 0 {
        eSize = errorCode;
        DISPLAYLEVEL!(
            notificationLevel,
            1,
            b"FSE_normalizeCount error with litLengthCount \n\0".as_ptr() as *const c_char
        );
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    llLog = errorCode as U32;

    /* write result to buffer */
    {
        let hhSize: size_t = HUF_writeCTable_wksp(
            dstPtr as *mut c_void,
            maxDstSize,
            hufTable.as_ptr(),
            255,
            huffLog,
            wksp.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&wksp) as size_t,
        );
        if HUF_isError(hhSize) != 0 {
            eSize = hhSize;
            DISPLAYLEVEL!(
                notificationLevel,
                1,
                b"HUF_writeCTable error \n\0".as_ptr() as *const c_char
            );
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        dstPtr = dstPtr.add(hhSize as usize);
        maxDstSize -= hhSize;
        eSize += hhSize;
    }

    {
        let ohSize: size_t = FSE_writeNCount(
            dstPtr as *mut c_void,
            maxDstSize,
            offcodeNCount.as_ptr(),
            OFFCODE_MAX as c_uint,
            Offlog,
        );
        if FSE_isError(ohSize) != 0 {
            eSize = ohSize;
            DISPLAYLEVEL!(
                notificationLevel,
                1,
                b"FSE_writeNCount error with offcodeNCount \n\0".as_ptr() as *const c_char
            );
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        dstPtr = dstPtr.add(ohSize as usize);
        maxDstSize -= ohSize;
        eSize += ohSize;
    }

    {
        let mhSize: size_t = FSE_writeNCount(
            dstPtr as *mut c_void,
            maxDstSize,
            matchLengthNCount.as_ptr(),
            MaxML,
            mlLog,
        );
        if FSE_isError(mhSize) != 0 {
            eSize = mhSize;
            DISPLAYLEVEL!(
                notificationLevel,
                1,
                b"FSE_writeNCount error with matchLengthNCount \n\0".as_ptr() as *const c_char
            );
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        dstPtr = dstPtr.add(mhSize as usize);
        maxDstSize -= mhSize;
        eSize += mhSize;
    }

    {
        let lhSize: size_t = FSE_writeNCount(
            dstPtr as *mut c_void,
            maxDstSize,
            litLengthNCount.as_ptr(),
            MaxLL,
            llLog,
        );
        if FSE_isError(lhSize) != 0 {
            eSize = lhSize;
            DISPLAYLEVEL!(
                notificationLevel,
                1,
                b"FSE_writeNCount error with litlengthNCount \n\0".as_ptr() as *const c_char
            );
            ZSTD_freeCDict(esr.dict);
            ZSTD_freeCCtx(esr.zc);
            free(esr.workPlace);
            return eSize;
        }
        dstPtr = dstPtr.add(lhSize as usize);
        maxDstSize -= lhSize;
        eSize += lhSize;
    }

    if maxDstSize < 12 {
        eSize = ERROR(ZSTD_error_dstSize_tooSmall);
        DISPLAYLEVEL!(
            notificationLevel,
            1,
            b"not enough space to write RepOffsets \n\0".as_ptr() as *const c_char
        );
        ZSTD_freeCDict(esr.dict);
        ZSTD_freeCCtx(esr.zc);
        free(esr.workPlace);
        return eSize;
    }
    /* #if 0 branch (bestRepOffset) is disabled; #else branch is active */
    /* at this stage, we don't use the result of "most common first offset" */
    MEM_writeLE32(dstPtr.add(0), repStartValue[0]);
    MEM_writeLE32(dstPtr.add(4), repStartValue[1]);
    MEM_writeLE32(dstPtr.add(8), repStartValue[2]);
    eSize += 12;

    // _cleanup:
    ZSTD_freeCDict(esr.dict);
    ZSTD_freeCCtx(esr.zc);
    free(esr.workPlace);

    eSize
}

/**
 * @returns the maximum repcode value
 */
unsafe fn ZDICT_maxRep(reps: *const U32 /* [ZSTD_REP_NUM] */) -> U32 {
    let mut maxRep: U32 = *reps.add(0);
    let mut r: c_int = 1;
    while r < ZSTD_REP_NUM as c_int {
        maxRep = MAX(maxRep, *reps.add(r as usize));
        r += 1;
    }
    maxRep
}

const HBUFFSIZE: usize = 256; /* should prove large enough for all entropy headers */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_finalizeDictionary(
    dictBuffer: *mut c_void,
    dictBufferCapacity: size_t,
    customDictContent: *const c_void,
    mut dictContentSize: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    params: ZDICT_params_t,
) -> size_t {
    let mut hSize: size_t;
    let mut header: [BYTE; HBUFFSIZE] = [0; HBUFFSIZE];
    let compressionLevel: c_int = if params.compressionLevel == 0 {
        ZSTD_CLEVEL_DEFAULT
    } else {
        params.compressionLevel
    };
    let notificationLevel: U32 = params.notificationLevel;
    /* The final dictionary content must be at least as large as the largest repcode */
    let minContentSize: size_t = ZDICT_maxRep(repStartValue.as_ptr()) as size_t;
    let paddingSize: size_t;

    /* check conditions */
    /* DEBUGLOG(4, "ZDICT_finalizeDictionary"); dropped */
    if dictBufferCapacity < dictContentSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    /* dictionary header */
    MEM_writeLE32(header.as_mut_ptr(), ZSTD_MAGIC_DICTIONARY);
    {
        let randomID: U64 = XXH64(customDictContent, dictContentSize, 0);
        let compliantID: U32 = (randomID % ((1u32 << 31) - 32768) as U64) as U32 + 32768;
        let dictID: U32 = if params.dictID != 0 {
            params.dictID
        } else {
            compliantID
        };
        MEM_writeLE32(header.as_mut_ptr().add(4), dictID);
    }
    hSize = 8;

    /* entropy tables */
    DISPLAYLEVEL!(
        notificationLevel,
        2,
        b"\r%70s\r\0".as_ptr() as *const c_char,
        b"\0".as_ptr() as *const c_char
    ); /* clean display line */
    DISPLAYLEVEL!(
        notificationLevel,
        2,
        b"statistics ... \n\0".as_ptr() as *const c_char
    );
    {
        let eSize: size_t = ZDICT_analyzeEntropy(
            header.as_mut_ptr().add(hSize as usize) as *mut c_void,
            HBUFFSIZE as size_t - hSize,
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
        /* RETURN_ERROR_IF(hSize + minContentSize > dictBufferCapacity, dstSize_tooSmall, ...) */
        if hSize + minContentSize > dictBufferCapacity {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        paddingSize = minContentSize - dictContentSize;
    } else {
        paddingSize = 0;
    }

    {
        let dictSize: size_t = hSize + paddingSize + dictContentSize;

        /* The dictionary consists of the header, optional padding, and the content. */
        let outDictHeader: *mut BYTE = dictBuffer as *mut BYTE;
        let outDictPadding: *mut BYTE = outDictHeader.add(hSize as usize);
        let outDictContent: *mut BYTE = outDictPadding.add(paddingSize as usize);

        /* assert(dictSize <= dictBufferCapacity); dropped */
        /* assert(outDictContent + dictContentSize == (BYTE*)dictBuffer + dictSize); dropped */

        /* First copy the customDictContent into its final location. */
        memmove(
            outDictContent as *mut c_void,
            customDictContent,
            dictContentSize,
        );
        memcpy(
            outDictHeader as *mut c_void,
            header.as_ptr() as *const c_void,
            hSize,
        );
        memset(outDictPadding as *mut c_void, 0, paddingSize);

        dictSize
    }
}

unsafe fn ZDICT_addEntropyTablesFromBuffer_advanced(
    dictBuffer: *mut c_void,
    dictContentSize: size_t,
    dictBufferCapacity: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    params: ZDICT_params_t,
) -> size_t {
    let compressionLevel: c_int = if params.compressionLevel == 0 {
        ZSTD_CLEVEL_DEFAULT
    } else {
        params.compressionLevel
    };
    let notificationLevel: U32 = params.notificationLevel;
    let mut hSize: size_t = 8;

    /* calculate entropy tables */
    DISPLAYLEVEL!(
        notificationLevel,
        2,
        b"\r%70s\r\0".as_ptr() as *const c_char,
        b"\0".as_ptr() as *const c_char
    ); /* clean display line */
    DISPLAYLEVEL!(
        notificationLevel,
        2,
        b"statistics ... \n\0".as_ptr() as *const c_char
    );
    {
        let eSize: size_t = ZDICT_analyzeEntropy(
            (dictBuffer as *mut c_char).add(hSize as usize) as *mut c_void,
            dictBufferCapacity - hSize,
            compressionLevel,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            (dictBuffer as *const c_char)
                .add(dictBufferCapacity as usize)
                .wrapping_sub(dictContentSize as usize) as *const c_void,
            dictContentSize,
            notificationLevel,
        );
        if ZDICT_isError(eSize) != 0 {
            return eSize;
        }
        hSize += eSize;
    }

    /* add dictionary header (after entropy tables) */
    MEM_writeLE32(dictBuffer as *mut u8, ZSTD_MAGIC_DICTIONARY);
    {
        let randomID: U64 = XXH64(
            (dictBuffer as *const c_char)
                .add(dictBufferCapacity as usize)
                .wrapping_sub(dictContentSize as usize) as *const c_void,
            dictContentSize,
            0,
        );
        let compliantID: U32 = (randomID % ((1u32 << 31) - 32768) as U64) as U32 + 32768;
        let dictID: U32 = if params.dictID != 0 {
            params.dictID
        } else {
            compliantID
        };
        MEM_writeLE32((dictBuffer as *mut c_char).add(4) as *mut u8, dictID);
    }

    if hSize + dictContentSize < dictBufferCapacity {
        memmove(
            (dictBuffer as *mut c_char).add(hSize as usize) as *mut c_void,
            (dictBuffer as *const c_char)
                .add(dictBufferCapacity as usize)
                .wrapping_sub(dictContentSize as usize) as *const c_void,
            dictContentSize,
        );
    }
    MIN(dictBufferCapacity, hSize + dictContentSize)
}

/* ZDICT_trainFromBuffer_unsafe_legacy() :
*   Warning : `samplesBuffer` must be followed by noisy guard band !!!
*/
unsafe fn ZDICT_trainFromBuffer_unsafe_legacy(
    dictBuffer: *mut c_void,
    maxDictSize: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    params: ZDICT_legacy_params_t,
) -> size_t {
    let dictListSize: U32 = MAX(
        MAX(DICTLISTSIZE_DEFAULT, nbSamples),
        (maxDictSize / 16) as U32,
    );
    let dictList: *mut dictItem =
        malloc(dictListSize as size_t * core::mem::size_of::<dictItem>() as size_t)
            as *mut dictItem;
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
    let targetDictSize: size_t = maxDictSize;
    let samplesBuffSize: size_t = ZDICT_totalSampleSize(samplesSizes, nbSamples);
    let mut dictSize: size_t = 0;
    let notificationLevel: U32 = params.zParams.notificationLevel;

    /* checks */
    if dictList.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    if maxDictSize < ZDICT_DICTSIZE_MIN {
        free(dictList as *mut c_void);
        return ERROR(ZSTD_error_dstSize_tooSmall);
    } /* requested dictionary size is too small */
    if samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE {
        free(dictList as *mut c_void);
        return ERROR(ZSTD_error_dictionaryCreation_failed);
    } /* not enough source to create dictionary */

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
        let nb: c_uint = MIN(25, (*dictList.add(0)).pos);
        let dictContentSize: c_uint = ZDICT_dictSize(dictList);
        let mut u: c_uint;
        DISPLAYLEVEL!(
            notificationLevel,
            3,
            b"\n %u segments found, of total size %u \n\0".as_ptr() as *const c_char,
            (*dictList.add(0)).pos - 1,
            dictContentSize
        );
        DISPLAYLEVEL!(
            notificationLevel,
            3,
            b"list %u best segments \n\0".as_ptr() as *const c_char,
            nb - 1
        );
        u = 1;
        while u < nb {
            let pos: c_uint = (*dictList.add(u as usize)).pos;
            let length: c_uint = (*dictList.add(u as usize)).length;
            let printedLength: U32 = MIN(40, length);
            if (pos as size_t > samplesBuffSize)
                || ((pos as size_t + length as size_t) > samplesBuffSize)
            {
                free(dictList as *mut c_void);
                return ERROR(ZSTD_error_GENERIC); /* should never happen */
            }
            DISPLAYLEVEL!(
                notificationLevel,
                3,
                b"%3u:%3u bytes at pos %8u, savings %7u bytes |\0".as_ptr() as *const c_char,
                u,
                length,
                pos,
                (*dictList.add(u as usize)).savings as c_uint
            );
            ZDICT_printHex(
                (samplesBuffer as *const c_char).add(pos as usize) as *const c_void,
                printedLength as size_t,
            );
            DISPLAYLEVEL!(
                notificationLevel,
                3,
                b"| \n\0".as_ptr() as *const c_char
            );
            u += 1;
        }
    }

    /* create dictionary */
    {
        let mut dictContentSize: c_uint = ZDICT_dictSize(dictList);
        if (dictContentSize as size_t) < ZDICT_CONTENTSIZE_MIN {
            free(dictList as *mut c_void);
            return ERROR(ZSTD_error_dictionaryCreation_failed);
        } /* dictionary content too small */
        if (dictContentSize as size_t) < targetDictSize / 4 {
            DISPLAYLEVEL!(
                notificationLevel,
                2,
                b"!  warning : selected content significantly smaller than requested (%u < %u) \n\0"
                    .as_ptr() as *const c_char,
                dictContentSize,
                maxDictSize as c_uint
            );
            if samplesBuffSize < 10 * targetDictSize {
                DISPLAYLEVEL!(
                    notificationLevel,
                    2,
                    b"!  consider increasing the number of samples (total size : %u MB)\n\0".as_ptr()
                        as *const c_char,
                    (samplesBuffSize >> 20) as c_uint
                );
            }
            if minRep > MINRATIO {
                DISPLAYLEVEL!(notificationLevel, 2, b"!  consider increasing selectivity to produce larger dictionary (-s%u) \n\0".as_ptr() as *const c_char, selectivity + 1);
                DISPLAYLEVEL!(notificationLevel, 2, b"!  note : larger dictionaries are not necessarily better, test its efficiency on samples \n\0".as_ptr() as *const c_char);
            }
        }

        if (dictContentSize as size_t > targetDictSize * 3)
            && (nbSamples > 2 * MINRATIO)
            && (selectivity > 1)
        {
            let mut proposedSelectivity: c_uint = selectivity - 1;
            while (nbSamples >> proposedSelectivity) <= MINRATIO {
                proposedSelectivity -= 1;
            }
            DISPLAYLEVEL!(notificationLevel, 2, b"!  note : calculated dictionary significantly larger than requested (%u > %u) \n\0".as_ptr() as *const c_char, dictContentSize, maxDictSize as c_uint);
            DISPLAYLEVEL!(notificationLevel, 2, b"!  consider increasing dictionary size, or produce denser dictionary (-s%u) \n\0".as_ptr() as *const c_char, proposedSelectivity);
            DISPLAYLEVEL!(
                notificationLevel,
                2,
                b"!  always test dictionary efficiency on real samples \n\0".as_ptr()
                    as *const c_char
            );
        }

        /* limit dictionary size */
        {
            let max: U32 = (*dictList).pos; /* convention : nb of useful elts within dictList */
            let mut currentSize: U32 = 0;
            let mut n: U32 = 1;
            while n < max {
                currentSize += (*dictList.add(n as usize)).length;
                if currentSize > targetDictSize as U32 {
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
            let mut u: U32;
            let mut ptr: *mut BYTE = (dictBuffer as *mut BYTE).add(maxDictSize as usize);
            u = 1;
            while u < (*dictList).pos {
                let l: U32 = (*dictList.add(u as usize)).length;
                ptr = ptr.wrapping_sub(l as usize);
                if (ptr as *const BYTE) < (dictBuffer as *const BYTE) {
                    free(dictList as *mut c_void);
                    return ERROR(ZSTD_error_GENERIC);
                } /* should not happen */
                memcpy(
                    ptr as *mut c_void,
                    (samplesBuffer as *const c_char)
                        .add((*dictList.add(u as usize)).pos as usize)
                        as *const c_void,
                    l as size_t,
                );
                u += 1;
            }
        }

        dictSize = ZDICT_addEntropyTablesFromBuffer_advanced(
            dictBuffer,
            dictContentSize as size_t,
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

/* ZDICT_trainFromBuffer_legacy() :
 * issue : samplesBuffer need to be followed by a noisy guard band.
 * work around : duplicate the buffer, and add the noise */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_legacy(
    dictBuffer: *mut c_void,
    dictBufferCapacity: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    params: ZDICT_legacy_params_t,
) -> size_t {
    let result: size_t;
    let newBuff: *mut c_void;
    let sBuffSize: size_t = ZDICT_totalSampleSize(samplesSizes, nbSamples);
    if sBuffSize < ZDICT_MIN_SAMPLES_SIZE {
        return 0;
    } /* not enough content => no dictionary */

    newBuff = malloc(sBuffSize + NOISELENGTH);
    if newBuff.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }

    memcpy(newBuff, samplesBuffer, sBuffSize);
    ZDICT_fillNoise(
        (newBuff as *mut c_char).add(sBuffSize as usize) as *mut c_void,
        NOISELENGTH,
    ); /* guard band, for end of buffer condition */

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
    dictBufferCapacity: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
) -> size_t {
    let mut params: ZDICT_fastCover_params_t;
    /* DEBUGLOG(3, "ZDICT_trainFromBuffer"); dropped */
    params = core::mem::zeroed();
    memset(
        &mut params as *mut ZDICT_fastCover_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_fastCover_params_t>() as size_t,
    );
    params.d = 8;
    params.steps = 4;
    /* Use default level since no compression level information is available */
    params.zParams.compressionLevel = ZSTD_CLEVEL_DEFAULT;
    /* #if defined(DEBUGLEVEL) && (DEBUGLEVEL>=1) ... : DEBUGLEVEL==0, so not set */
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
    dictContentSize: size_t,
    dictBufferCapacity: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
) -> size_t {
    let mut params: ZDICT_params_t = core::mem::zeroed();
    memset(
        &mut params as *mut ZDICT_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_params_t>() as size_t,
    );
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
