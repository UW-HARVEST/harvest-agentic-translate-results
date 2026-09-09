//! Translation of `dictBuilder/zdict.c`
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_short, c_uint, c_void};

use crate::bits::{ZSTD_NbCommonBytes, ZSTD_highbit32};
use crate::cmem::*;
use crate::compress::fse_compress::{FSE_normalizeCount, FSE_writeNCount};
use crate::compress::huf_compress::{HUF_buildCTable_wksp, HUF_writeCTable_wksp};
use crate::compress::zstd_compress_internal::{
    SeqDef, SeqStore_t, ZSTD_CCtx, ZSTD_CDict, ZSTD_compressedBlockState_t,
};
use crate::dictbuilder::divsufsort::{divbwt, divsufsort};
use crate::entropy_common::{FSE_isError, HUF_isError};
use crate::error_private::*;
use crate::fse::*;
use crate::huf::*;
use crate::xxhash::XXH64;
use crate::zstd_common::ZSTD_isError;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/*-**************************************
*  Tuning parameters
****************************************/
const MINRATIO: c_uint = 4; /* minimum nb of apparition to be selected in dictionary */
const ZDICT_MAX_SAMPLES_SIZE: usize = (2000u32 << 20) as usize;
const ZDICT_MIN_SAMPLES_SIZE: usize = (ZDICT_CONTENTSIZE_MIN * MINRATIO) as usize;

/*-*************************************
*  zdict.h  (public dictBuilder API types & constants)
***************************************/
pub const ZDICT_DICTSIZE_MIN: usize = 256;
/* Deprecated: Remove in v1.6.0 */
pub const ZDICT_CONTENTSIZE_MIN: c_uint = 128;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZDICT_params_t {
    /// optimize for a specific zstd compression level; 0 means default
    pub compressionLevel: c_int,
    /// Write log to stderr; 0 = none (default); 1 = errors; 2 = progression; 3 = details; 4 = debug;
    pub notificationLevel: c_uint,
    /// force dictID value; 0 means auto mode (32-bits random value)
    pub dictID: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
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
#[derive(Copy, Clone, Default)]
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
#[derive(Copy, Clone, Default)]
pub struct ZDICT_legacy_params_t {
    pub selectivityLevel: c_uint,
    pub zParams: ZDICT_params_t,
}

/*-*************************************
*  Constants
***************************************/
const DICTLISTSIZE_DEFAULT: c_uint = 10000;

const NOISELENGTH: usize = 32;

static g_selectivity_default: U32 = 9;

/* `ZSTD_CLEVEL_DEFAULT` from zstd.h */
const ZSTD_CLEVEL_DEFAULT: c_int = 3;

/* `ZSTD_defaultCMem` from zstd.h */
const ZSTD_defaultCMem: ZSTD_customMem = ZSTD_customMem {
    customAlloc: None,
    customFree: None,
    opaque: core::ptr::null_mut(),
};

/* <time.h> */
pub type clock_t = i64;
const CLOCKS_PER_SEC: clock_t = 1000000;

unsafe extern "C" {
    fn clock() -> clock_t;
}

/*-*************************************
*  Console display
***************************************/
/* #define DISPLAY(...)         do { fprintf(stderr, __VA_ARGS__); fflush( stderr ); } while (0) */
macro_rules! DISPLAY {
    ($($arg:expr),* $(,)?) => {{
        crate::cmem::fprintf(crate::cmem::stderr, $($arg),*);
        crate::cmem::fflush(crate::cmem::stderr);
    }};
}

/* #define DISPLAYLEVEL(l, ...) do { if (notificationLevel>=l) { DISPLAY(__VA_ARGS__); } } while (0)
 * The level variable is passed explicitly as the first argument because
 * `macro_rules!` cannot capture the caller's `notificationLevel` binding. */
macro_rules! DISPLAYLEVEL {
    ($notificationLevel:expr, $l:expr, $($arg:expr),* $(,)?) => {{
        if $notificationLevel >= $l {
            DISPLAY!($($arg),*);
        }
    }};
}

/* #define DISPLAYUPDATE(l, ...)  (ZDICT_trainBuffer_legacy local redefinition)
 *   do { if (notificationLevel>=l) {
 *            if (ZDICT_clockSpan(displayClock) > refreshRate) {
 *                displayClock = clock(); DISPLAY(__VA_ARGS__); }
 *            if (notificationLevel>=4) fflush(stderr); } } while (0) */
macro_rules! DISPLAYUPDATE {
    ($notificationLevel:expr, $displayClock:expr, $refreshRate:expr, $l:expr, $($arg:expr),* $(,)?) => {{
        if $notificationLevel >= $l {
            if ZDICT_clockSpan($displayClock) > $refreshRate {
                $displayClock = clock();
                DISPLAY!($($arg),*);
            }
            if $notificationLevel >= 4 {
                crate::cmem::fflush(crate::cmem::stderr);
            }
        }
    }};
}

unsafe fn ZDICT_clockSpan(nPrevious: clock_t) -> clock_t {
    clock().wrapping_sub(nPrevious)
}

unsafe fn ZDICT_printHex(ptr: *const c_void, length: usize) {
    let b = ptr as *const BYTE;
    let mut u: usize = 0;
    while u < length {
        let mut c: BYTE = *b.add(u);
        if c < 32 || c > 126 {
            c = b'.'; /* non-printable char */
        }
        DISPLAY!(c"%c".as_ptr(), c as c_int);
        u += 1;
    }
}

/*-********************************************************
*  Helper functions
**********************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getDictID(dictBuffer: *const c_void, dictSize: usize) -> c_uint {
    if dictSize < 8 {
        return 0;
    }
    if MEM_readLE32(dictBuffer) != ZSTD_MAGIC_DICTIONARY {
        return 0;
    }
    MEM_readLE32((dictBuffer as *const c_char).add(4) as *const c_void)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_getDictHeaderSize(
    dictBuffer: *const c_void,
    dictSize: usize,
) -> usize {
    let headerSize: usize;
    if dictSize <= 8 || MEM_readLE32(dictBuffer) != ZSTD_MAGIC_DICTIONARY {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    {
        let bs: *mut ZSTD_compressedBlockState_t =
            malloc(core::mem::size_of::<ZSTD_compressedBlockState_t>())
                as *mut ZSTD_compressedBlockState_t;
        let wksp: *mut U32 = malloc(HUF_WORKSPACE_SIZE) as *mut U32;
        if bs.is_null() || wksp.is_null() {
            headerSize = ERROR(ZSTD_error_memory_allocation);
        } else {
            crate::compress::zstd_compress::ZSTD_reset_compressedBlockState(bs);
            headerSize = crate::compress::zstd_compress::ZSTD_loadCEntropy(
                bs,
                wksp as *mut c_void,
                dictBuffer,
                dictSize,
            );
        }

        free(bs as *mut c_void);
        free(wksp as *mut c_void);
    }

    headerSize
}

/*-********************************************************
*  Dictionary training functions
**********************************************************/
/// `ZDICT_count()` :
/// Count the nb of common bytes between 2 pointers.
/// Note : this function presumes end of buffer followed by noisy guard band.
unsafe fn ZDICT_count(pIn: *const c_void, pMatch: *const c_void) -> usize {
    let pStart = pIn as *const c_char;
    let mut pIn = pIn;
    let mut pMatch = pMatch;
    loop {
        let diff: usize = MEM_readST(pMatch) ^ MEM_readST(pIn);
        if diff == 0 {
            pIn = (pIn as *const c_char).add(core::mem::size_of::<usize>()) as *const c_void;
            pMatch = (pMatch as *const c_char).add(core::mem::size_of::<usize>()) as *const c_void;
            continue;
        }
        pIn = (pIn as *const c_char).add(ZSTD_NbCommonBytes(diff) as usize) as *const c_void;
        return (pIn as *const c_char).offset_from(pStart) as usize;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct dictItem {
    pos: U32,
    length: U32,
    savings: U32,
}

unsafe fn ZDICT_initDictItem(d: *mut dictItem) {
    (*d).pos = 1;
    (*d).length = 0;
    (*d).savings = 0u32.wrapping_sub(1);
}

const LLIMIT: usize = 64; /* heuristic determined experimentally */
const MINMATCHLENGTH: usize = 7; /* heuristic determined experimentally */

unsafe fn ZDICT_analyzePos(
    doneMarks: *mut BYTE,
    suffix: *const c_int,
    mut start: U32,
    buffer: *const c_void,
    minRatio: U32,
    notificationLevel: U32,
) -> dictItem {
    let mut lengthList: [U32; LLIMIT] = [0; LLIMIT];
    let mut cumulLength: [U32; LLIMIT] = [0; LLIMIT];
    let mut savings: [U32; LLIMIT] = [0; LLIMIT];
    let b = buffer as *const BYTE;
    let mut maxLength: usize = LLIMIT;
    let mut pos: usize = *suffix.add(start as usize) as usize;
    let mut end: U32 = start;
    let mut solution: dictItem;

    /* init */
    solution = dictItem {
        pos: 0,
        length: 0,
        savings: 0,
    };
    *doneMarks.add(pos) = 1;

    /* trivial repetition cases */
    if (MEM_read16(b.add(pos + 0) as *const c_void) == MEM_read16(b.add(pos + 2) as *const c_void))
        || (MEM_read16(b.add(pos + 1) as *const c_void)
            == MEM_read16(b.add(pos + 3) as *const c_void))
        || (MEM_read16(b.add(pos + 2) as *const c_void)
            == MEM_read16(b.add(pos + 4) as *const c_void))
    {
        /* skip and mark segment */
        let pattern16: U16 = MEM_read16(b.add(pos + 4) as *const c_void);
        let mut u: U32;
        let mut patternEnd: U32 = 6;
        while MEM_read16(b.add(pos + patternEnd as usize) as *const c_void) == pattern16 {
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
                b.offset(*suffix.add(end as usize) as isize) as *const c_void,
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
                b.offset(*suffix.add(start as usize).offset(-1) as isize) as *const c_void,
            );
            if length >= MINMATCHLENGTH {
                start = start.wrapping_sub(1);
            }
            if !(length >= MINMATCHLENGTH) {
                break;
            }
        }
    }

    /* exit if not found a minimum nb of repetitions */
    if end.wrapping_sub(start) < minRatio {
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

        DISPLAYLEVEL!(notificationLevel, 4, c"\n".as_ptr());
        DISPLAYLEVEL!(
            notificationLevel,
            4,
            c"found %3u matches of length >= %i at pos %7u  ".as_ptr(),
            end.wrapping_sub(start) as c_uint,
            MINMATCHLENGTH as c_int,
            pos as c_uint,
        );
        DISPLAYLEVEL!(notificationLevel, 4, c"\n".as_ptr());

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
                if *b.offset(*suffix.add(id as usize) as isize + mml as isize) != currentChar {
                    if currentCount > selectedCount {
                        selectedCount = currentCount;
                        selectedID = currentID;
                    }
                    currentID = id;
                    currentChar = *b.offset(*suffix.add(id as usize) as isize + mml as isize);
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
            refinedEnd = refinedStart.wrapping_add(selectedCount);

            mml += 1;
        }

        /* evaluate gain based on new dict */
        start = refinedStart;
        pos = *suffix.add(refinedStart as usize) as usize;
        end = start;
        lengthList = [0; LLIMIT];

        /* look forward */
        {
            let mut length: usize;
            loop {
                end += 1;
                length = ZDICT_count(
                    b.add(pos) as *const c_void,
                    b.offset(*suffix.add(end as usize) as isize) as *const c_void,
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
            while (((length >= MINMATCHLENGTH) as u32) & ((start > 0) as u32)) != 0 {
                length = ZDICT_count(
                    b.add(pos) as *const c_void,
                    b.offset(*suffix.add(start as usize - 1) as isize) as *const c_void,
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
        cumulLength = [0; LLIMIT];
        cumulLength[maxLength - 1] = lengthList[maxLength - 1];
        i = (maxLength - 2) as c_int;
        while i >= 0 {
            cumulLength[i as usize] =
                cumulLength[i as usize + 1].wrapping_add(lengthList[i as usize]);
            i -= 1;
        }

        i = LLIMIT as c_int - 1;
        while i >= MINMATCHLENGTH as c_int {
            if cumulLength[i as usize] >= minRatio {
                break;
            }
            i -= 1;
        }
        maxLength = i as usize;

        /* reduce maxLength in case of final into repetitive data */
        {
            let mut l: U32 = maxLength as U32;
            let c: BYTE = *b.add(pos + maxLength - 1);
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
            savings[i as usize] = savings[i as usize - 1]
                .wrapping_add(lengthList[i as usize].wrapping_mul((i - 3) as U32));
            i += 1;
        }

        DISPLAYLEVEL!(
            notificationLevel,
            4,
            c"Selected dict at position %u, of length %u : saves %u (ratio: %.2f)  \n".as_ptr(),
            pos as c_uint,
            maxLength as c_uint,
            savings[maxLength] as c_uint,
            savings[maxLength] as f64 / maxLength as f64,
        );

        solution.pos = pos as U32;
        solution.length = maxLength as U32;
        solution.savings = savings[maxLength];

        /* mark positions done */
        {
            let mut id: U32 = start;
            while id < end {
                let mut p: U32;
                let pEnd: U32;
                let mut length: U32;
                let testedPos: U32 = *suffix.add(id as usize) as U32;
                if testedPos as usize == pos {
                    length = solution.length;
                } else {
                    length = ZDICT_count(
                        b.add(pos) as *const c_void,
                        b.add(testedPos as usize) as *const c_void,
                    ) as U32;
                    if length > solution.length {
                        length = solution.length;
                    }
                }
                pEnd = testedPos.wrapping_add(length);
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

unsafe fn isIncluded(in_: *const c_void, container: *const c_void, length: usize) -> c_int {
    let ip = in_ as *const c_char;
    let into = container as *const c_char;
    let mut u: usize = 0;

    /* works because end of buffer is a noisy guard band */
    while u < length {
        if *ip.add(u) != *into.add(u) {
            break;
        }
        u += 1;
    }

    (u == length) as c_int
}

/// `ZDICT_tryMerge()` :
/// check if dictItem can be merged, do it if possible
/// @return : id of destination elt, 0 if not merged
unsafe fn ZDICT_tryMerge(
    table: *mut dictItem,
    mut elt: dictItem,
    eltNbToSkip: U32,
    buffer: *const c_void,
) -> U32 {
    let tableSize: U32 = (*table).pos;
    let eltEnd: U32 = elt.pos.wrapping_add(elt.length);
    let buf = buffer as *const c_char;

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
                let addedLength: U32 = (*table.add(u as usize)).pos.wrapping_sub(elt.pos);
                (*table.add(u as usize)).length =
                    (*table.add(u as usize)).length.wrapping_add(addedLength);
                (*table.add(u as usize)).pos = elt.pos;
                (*table.add(u as usize)).savings = (*table.add(u as usize))
                    .savings
                    .wrapping_add(elt.savings.wrapping_mul(addedLength) / elt.length); /* rough approx */
                (*table.add(u as usize)).savings =
                    (*table.add(u as usize)).savings.wrapping_add(elt.length / 8); /* rough approx bonus */
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

            if ((*table.add(u as usize))
                .pos
                .wrapping_add((*table.add(u as usize)).length)
                >= elt.pos)
                && ((*table.add(u as usize)).pos < elt.pos)
            {
                /* overlap, existing < new */
                /* append */
                let addedLength: c_int = (eltEnd as c_int).wrapping_sub(
                    ((*table.add(u as usize))
                        .pos
                        .wrapping_add((*table.add(u as usize)).length)) as c_int,
                );
                (*table.add(u as usize)).savings =
                    (*table.add(u as usize)).savings.wrapping_add(elt.length / 8); /* rough approx bonus */
                if addedLength > 0 {
                    /* otherwise, elt fully included into existing */
                    (*table.add(u as usize)).length = (*table.add(u as usize))
                        .length
                        .wrapping_add(addedLength as U32);
                    (*table.add(u as usize)).savings = (*table.add(u as usize)).savings.wrapping_add(
                        elt.savings.wrapping_mul(addedLength as U32) / elt.length,
                    ); /* rough approx */
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

            if MEM_read64(buf.add((*table.add(u as usize)).pos as usize) as *const c_void)
                == MEM_read64(buf.add(elt.pos as usize + 1) as *const c_void)
            {
                if isIncluded(
                    buf.add((*table.add(u as usize)).pos as usize) as *const c_void,
                    buf.add(elt.pos as usize + 1) as *const c_void,
                    (*table.add(u as usize)).length as usize,
                ) != 0
                {
                    let a = (elt.length as c_int).wrapping_sub((*table.add(u as usize)).length as c_int);
                    let addedLength: usize = (if a > 1 { a } else { 1 }) as usize;
                    (*table.add(u as usize)).pos = elt.pos;
                    (*table.add(u as usize)).savings =
                        (*table.add(u as usize)).savings.wrapping_add(
                            ((elt.savings as usize).wrapping_mul(addedLength)
                                / elt.length as usize) as U32,
                        );
                    (*table.add(u as usize)).length = {
                        let x = elt.length;
                        let y = (*table.add(u as usize)).length.wrapping_add(1);
                        if x < y { x } else { y }
                    };
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
        return; /* protection, should never happen */
    }
    u = id;
    while u < max.wrapping_sub(1) {
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
            nextElt = maxSize.wrapping_sub(1);
        }
        current = nextElt.wrapping_sub(1);
        while (*table.add(current as usize)).savings < elt.savings {
            *table.add(current as usize + 1) = *table.add(current as usize);
            current = current.wrapping_sub(1);
        }
        *table.add(current as usize + 1) = elt;
        (*table).pos = nextElt.wrapping_add(1);
    }
}

unsafe fn ZDICT_dictSize(dictList: *const dictItem) -> U32 {
    let mut u: U32;
    let mut dictSize: U32 = 0;
    u = 1;
    while u < (*dictList.add(0)).pos {
        dictSize = dictSize.wrapping_add((*dictList.add(u as usize)).length);
        u += 1;
    }
    dictSize
}

unsafe fn ZDICT_trainBuffer_legacy(
    dictList: *mut dictItem,
    dictListSize: U32,
    buffer: *const c_void,
    mut bufferSize: usize, /* buffer must end with noisy guard band */
    fileSizes: *const usize,
    mut nbFiles: c_uint,
    mut minRatio: c_uint,
    notificationLevel: U32,
) -> usize {
    let suffix0: *mut c_int =
        malloc((bufferSize + 2) * core::mem::size_of::<c_int>()) as *mut c_int;
    let suffix: *mut c_int =
        (suffix0 as usize).wrapping_add(core::mem::size_of::<c_int>()) as *mut c_int;
    let reverseSuffix: *mut U32 =
        malloc(bufferSize * core::mem::size_of::<U32>()) as *mut U32;
    /* +16 for overflow security */
    let doneMarks: *mut BYTE =
        malloc((bufferSize + 16) * core::mem::size_of::<BYTE>()) as *mut BYTE;
    let filePos: *mut U32 =
        malloc(nbFiles as usize * core::mem::size_of::<U32>()) as *mut U32;
    let mut result: usize = 0;
    let mut displayClock: clock_t = 0;
    let refreshRate: clock_t = CLOCKS_PER_SEC * 3 / 10;

    'cleanup: {
        /* init */
        DISPLAYLEVEL!(
            notificationLevel,
            2,
            c"\r%70s\r".as_ptr(),
            c"".as_ptr()
        ); /* clean display line */
        if suffix0.is_null()
            || reverseSuffix.is_null()
            || doneMarks.is_null()
            || filePos.is_null()
        {
            result = ERROR(ZSTD_error_memory_allocation);
            break 'cleanup;
        }
        if minRatio < MINRATIO {
            minRatio = MINRATIO;
        }
        ZSTD_memset(doneMarks as *mut c_void, 0, bufferSize + 16);

        /* limit sample set size (divsufsort limitation)*/
        if bufferSize > ZDICT_MAX_SAMPLES_SIZE {
            DISPLAYLEVEL!(
                notificationLevel,
                3,
                c"sample set too large : reduced to %u MB ...\n".as_ptr(),
                (ZDICT_MAX_SAMPLES_SIZE >> 20) as c_uint,
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
            c"sorting %u files of total size %u MB ...\n".as_ptr(),
            nbFiles as c_uint,
            (bufferSize >> 20) as c_uint,
        );
        {
            let divSuftSortResult: c_int =
                divsufsort(buffer as *const u8, suffix, bufferSize as c_int, 0);
            if divSuftSortResult != 0 {
                result = ERROR(ZSTD_error_GENERIC);
                break 'cleanup;
            }
        }
        *suffix.add(bufferSize) = bufferSize as c_int; /* leads into noise */
        *suffix0.add(0) = bufferSize as c_int; /* leads into noise */
        /* build reverse suffix sort */
        {
            let mut pos: usize;
            pos = 0;
            while pos < bufferSize {
                *reverseSuffix.add(*suffix.add(pos) as usize) = pos as U32;
                pos += 1;
            }
            /* note filePos tracks borders between samples.
               It's not used at this stage, but planned to become useful in a later update */
            *filePos.add(0) = 0;
            pos = 1;
            while pos < nbFiles as usize {
                *filePos.add(pos) =
                    (*filePos.add(pos - 1)).wrapping_add(*fileSizes.add(pos - 1) as U32);
                pos += 1;
            }
        }

        DISPLAYLEVEL!(notificationLevel, 2, c"finding patterns ... \n".as_ptr());
        DISPLAYLEVEL!(
            notificationLevel,
            3,
            c"minimum ratio : %u \n".as_ptr(),
            minRatio as c_uint
        );

        {
            let mut cursor: U32 = 0;
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
                cursor = cursor.wrapping_add(solution.length);
                DISPLAYUPDATE!(
                    notificationLevel,
                    displayClock,
                    refreshRate,
                    2,
                    c"\r%4.2f %% \r".as_ptr(),
                    cursor as f64 / bufferSize as f64 * 100.0,
                );
            }
        }
    }

    /* _cleanup: */
    free(suffix0 as *mut c_void);
    free(reverseSuffix as *mut c_void);
    free(doneMarks as *mut c_void);
    free(filePos as *mut c_void);
    result
}

unsafe fn ZDICT_fillNoise(buffer: *mut c_void, length: usize) {
    let prime1: c_uint = 2654435761;
    let prime2: c_uint = 2246822519;
    let mut acc: c_uint = prime1;
    let mut p: usize = 0;
    while p < length {
        acc = acc.wrapping_mul(prime2);
        *(buffer as *mut u8).add(p) = (acc >> 21) as u8;
        p += 1;
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct EStats_ress_t {
    /// dictionary
    dict: *mut ZSTD_CDict,
    /// working context
    zc: *mut ZSTD_CCtx,
    /// must be ZSTD_BLOCKSIZE_MAX allocated
    workPlace: *mut c_void,
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
    mut srcSize: usize,
    notificationLevel: U32,
) {
    let blockSizeMax: usize = {
        let a = ZSTD_BLOCKSIZE_MAX;
        let b = (1i32 << (*params).cParams.windowLog) as usize;
        if a < b { a } else { b }
    };
    let cSize: usize;

    if srcSize > blockSizeMax {
        srcSize = blockSizeMax; /* protection vs large samples */
    }
    {
        let errorCode: usize =
            crate::compress::zstd_compress::ZSTD_compressBegin_usingCDict_deprecated(
                esr.zc, esr.dict,
            );
        if ZSTD_isError(errorCode) != 0 {
            DISPLAYLEVEL!(
                notificationLevel,
                1,
                c"warning : ZSTD_compressBegin_usingCDict failed \n".as_ptr()
            );
            return;
        }
    }
    cSize = crate::compress::zstd_compress::ZSTD_compressBlock_deprecated(
        esr.zc,
        esr.workPlace,
        ZSTD_BLOCKSIZE_MAX,
        src,
        srcSize,
    );
    if ZSTD_isError(cSize) != 0 {
        DISPLAYLEVEL!(
            notificationLevel,
            3,
            c"warning : could not compress sample size %u \n".as_ptr(),
            srcSize as c_uint,
        );
        return;
    }

    if cSize != 0 {
        /* if == 0; block is not compressible */
        let seqStorePtr: *const SeqStore_t =
            crate::compress::zstd_compress::ZSTD_getSeqStore(esr.zc);

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
            let nbSeq: U32 = (*seqStorePtr)
                .sequences
                .offset_from((*seqStorePtr).sequencesStart) as U32;
            crate::compress::zstd_compress::ZSTD_seqToCodes(seqStorePtr);

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
                let mut offset1: U32 = (*seq.add(0)).offBase.wrapping_sub(ZSTD_REP_NUM as U32);
                let mut offset2: U32 = (*seq.add(1)).offBase.wrapping_sub(ZSTD_REP_NUM as U32);
                if offset1 as usize >= MAXREPOFFSET {
                    offset1 = 0;
                }
                if offset2 as usize >= MAXREPOFFSET {
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
#[derive(Copy, Clone, Default)]
struct offsetCount_t {
    offset: U32,
    count: U32,
}

unsafe fn ZDICT_insertSortCount(table: *mut offsetCount_t, val: U32, count: U32) {
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

/// `ZDICT_flatLit()` :
/// rewrite `countLit` to contain a mostly flat but still compressible distribution of literals.
/// necessary to avoid generating a non-compressible distribution that HUF_writeCTable() cannot encode.
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

const OFFCODE_MAX: usize = 30; /* only applicable to first block */

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
    let mut hufTable: [HUF_CElt; HUF_CTABLE_SIZE_ST(255)] = [0; HUF_CTABLE_SIZE_ST(255)];
    let mut offcodeCount: [c_uint; OFFCODE_MAX + 1] = [0; OFFCODE_MAX + 1];
    let mut offcodeNCount: [c_short; OFFCODE_MAX + 1] = [0; OFFCODE_MAX + 1];
    let offcodeMax: U32 = ZSTD_highbit32((dictBufferSize + 128 * (1 << 10)) as U32);
    let mut matchLengthCount: [c_uint; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
    let mut matchLengthNCount: [c_short; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
    let mut litLengthCount: [c_uint; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
    let mut litLengthNCount: [c_short; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
    let mut repOffset: [U32; MAXREPOFFSET] = [0; MAXREPOFFSET];
    let mut bestRepOffset: [offsetCount_t; ZSTD_REP_NUM + 1] =
        [offsetCount_t { offset: 0, count: 0 }; ZSTD_REP_NUM + 1];
    let mut esr = EStats_ress_t {
        dict: core::ptr::null_mut(),
        zc: core::ptr::null_mut(),
        workPlace: core::ptr::null_mut(),
    };
    let mut params: ZSTD_parameters = ZSTD_parameters::default();
    let mut u: U32;
    let mut huffLog: U32 = 11;
    let mut Offlog: U32 = OffFSELog;
    let mut mlLog: U32 = MLFSELog;
    let mut llLog: U32 = LLFSELog;
    let mut total: U32;
    let mut pos: usize = 0;
    let mut errorCode: usize;
    let mut eSize: usize = 0;
    let totalSrcSize: usize = ZDICT_totalSampleSize(fileSizes, nbFiles);
    let averageSampleSize: usize = totalSrcSize / (nbFiles as usize + (nbFiles == 0) as usize);
    let mut dstPtr: *mut BYTE = dstBuffer as *mut BYTE;
    let mut wksp: [U32; HUF_CTABLE_WORKSPACE_SIZE_U32] = [0; HUF_CTABLE_WORKSPACE_SIZE_U32];

    'cleanup: {
        /* init */
        if offcodeMax as usize > OFFCODE_MAX {
            eSize = ERROR(ZSTD_error_dictionaryCreation_failed);
            break 'cleanup;
        } /* too large dictionary */
        u = 0;
        while u < 256 {
            countLit[u as usize] = 1; /* any character must be described */
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
        repOffset = [0; MAXREPOFFSET];
        repOffset[1] = 1;
        repOffset[4] = 1;
        repOffset[8] = 1;
        bestRepOffset = [offsetCount_t { offset: 0, count: 0 }; ZSTD_REP_NUM + 1];
        if compressionLevel == 0 {
            compressionLevel = ZSTD_CLEVEL_DEFAULT;
        }
        params = crate::compress::zstd_compress::ZSTD_getParams(
            compressionLevel,
            averageSampleSize as u64,
            dictBufferSize,
        );

        esr.dict = crate::compress::zstd_compress::ZSTD_createCDict_advanced(
            dictBuffer,
            dictBufferSize,
            ZSTD_dlm_byRef,
            ZSTD_dct_rawContent,
            params.cParams,
            ZSTD_defaultCMem,
        );
        esr.zc = crate::compress::zstd_compress::ZSTD_createCCtx();
        esr.workPlace = malloc(ZSTD_BLOCKSIZE_MAX);
        if esr.dict.is_null() || esr.zc.is_null() || esr.workPlace.is_null() {
            eSize = ERROR(ZSTD_error_memory_allocation);
            DISPLAYLEVEL!(notificationLevel, 1, c"Not enough memory \n".as_ptr());
            break 'cleanup;
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

        if notificationLevel >= 4 {
            /* writeStats */
            DISPLAYLEVEL!(
                notificationLevel,
                4,
                c"Offset Code Frequencies : \n".as_ptr()
            );
            u = 0;
            while u <= offcodeMax {
                DISPLAYLEVEL!(
                    notificationLevel,
                    4,
                    c"%2u :%7u \n".as_ptr(),
                    u as c_uint,
                    offcodeCount[u as usize] as c_uint,
                );
                u += 1;
            }
        }

        /* analyze, build stats, starting with literals */
        {
            let mut maxNbBits: usize = HUF_buildCTable_wksp(
                hufTable.as_mut_ptr(),
                countLit.as_ptr(),
                255,
                huffLog,
                wksp.as_mut_ptr() as *mut c_void,
                core::mem::size_of::<[U32; HUF_CTABLE_WORKSPACE_SIZE_U32]>(),
            );
            if HUF_isError(maxNbBits) != 0 {
                eSize = maxNbBits;
                DISPLAYLEVEL!(notificationLevel, 1, c" HUF_buildCTable error \n".as_ptr());
                break 'cleanup;
            }
            if maxNbBits == 8 {
                /* not compressible : will fail on HUF_writeCTable() */
                DISPLAYLEVEL!(
                    notificationLevel,
                    2,
                    c"warning : pathological dataset : literals are not compressible : samples are noisy or too regular \n".as_ptr()
                );
                /* replace distribution by a fake "mostly flat but still compressible"
                 * distribution, that HUF_writeCTable() can encode */
                ZDICT_flatLit(countLit.as_mut_ptr());
                maxNbBits = HUF_buildCTable_wksp(
                    hufTable.as_mut_ptr(),
                    countLit.as_ptr(),
                    255,
                    huffLog,
                    wksp.as_mut_ptr() as *mut c_void,
                    core::mem::size_of::<[U32; HUF_CTABLE_WORKSPACE_SIZE_U32]>(),
                );
            }
            huffLog = maxNbBits as U32;
        }

        /* looking for most common first offsets */
        {
            let mut offset: U32 = 1;
            while (offset as usize) < MAXREPOFFSET {
                ZDICT_insertSortCount(
                    bestRepOffset.as_mut_ptr(),
                    offset,
                    repOffset[offset as usize],
                );
                offset += 1;
            }
        }
        /* note : the result of this phase should be used to better appreciate the impact on statistics */

        total = 0;
        u = 0;
        while u <= offcodeMax {
            total = total.wrapping_add(offcodeCount[u as usize]);
            u += 1;
        }
        errorCode = FSE_normalizeCount(
            offcodeNCount.as_mut_ptr(),
            Offlog,
            offcodeCount.as_ptr(),
            total as usize,
            offcodeMax,
            /* useLowProbCount */ 1,
        );
        if FSE_isError(errorCode) != 0 {
            eSize = errorCode;
            DISPLAYLEVEL!(notificationLevel, 1, c"FSE_normalizeCount error with offcodeCount \n".as_ptr());
            break 'cleanup;
        }
        Offlog = errorCode as U32;

        total = 0;
        u = 0;
        while u <= MaxML {
            total = total.wrapping_add(matchLengthCount[u as usize]);
            u += 1;
        }
        errorCode = FSE_normalizeCount(
            matchLengthNCount.as_mut_ptr(),
            mlLog,
            matchLengthCount.as_ptr(),
            total as usize,
            MaxML,
            /* useLowProbCount */ 1,
        );
        if FSE_isError(errorCode) != 0 {
            eSize = errorCode;
            DISPLAYLEVEL!(notificationLevel, 1, c"FSE_normalizeCount error with matchLengthCount \n".as_ptr());
            break 'cleanup;
        }
        mlLog = errorCode as U32;

        total = 0;
        u = 0;
        while u <= MaxLL {
            total = total.wrapping_add(litLengthCount[u as usize]);
            u += 1;
        }
        errorCode = FSE_normalizeCount(
            litLengthNCount.as_mut_ptr(),
            llLog,
            litLengthCount.as_ptr(),
            total as usize,
            MaxLL,
            /* useLowProbCount */ 1,
        );
        if FSE_isError(errorCode) != 0 {
            eSize = errorCode;
            DISPLAYLEVEL!(notificationLevel, 1, c"FSE_normalizeCount error with litLengthCount \n".as_ptr());
            break 'cleanup;
        }
        llLog = errorCode as U32;

        /* write result to buffer */
        {
            let hhSize: usize = HUF_writeCTable_wksp(
                dstPtr as *mut c_void,
                maxDstSize,
                hufTable.as_ptr(),
                255,
                huffLog,
                wksp.as_mut_ptr() as *mut c_void,
                core::mem::size_of::<[U32; HUF_CTABLE_WORKSPACE_SIZE_U32]>(),
            );
            if HUF_isError(hhSize) != 0 {
                eSize = hhSize;
                DISPLAYLEVEL!(notificationLevel, 1, c"HUF_writeCTable error \n".as_ptr());
                break 'cleanup;
            }
            dstPtr = dstPtr.add(hhSize);
            maxDstSize -= hhSize;
            eSize += hhSize;
        }

        {
            let ohSize: usize = FSE_writeNCount(
                dstPtr as *mut c_void,
                maxDstSize,
                offcodeNCount.as_ptr(),
                OFFCODE_MAX as c_uint,
                Offlog,
            );
            if FSE_isError(ohSize) != 0 {
                eSize = ohSize;
                DISPLAYLEVEL!(notificationLevel, 1, c"FSE_writeNCount error with offcodeNCount \n".as_ptr());
                break 'cleanup;
            }
            dstPtr = dstPtr.add(ohSize);
            maxDstSize -= ohSize;
            eSize += ohSize;
        }

        {
            let mhSize: usize = FSE_writeNCount(
                dstPtr as *mut c_void,
                maxDstSize,
                matchLengthNCount.as_ptr(),
                MaxML,
                mlLog,
            );
            if FSE_isError(mhSize) != 0 {
                eSize = mhSize;
                DISPLAYLEVEL!(notificationLevel, 1, c"FSE_writeNCount error with matchLengthNCount \n".as_ptr());
                break 'cleanup;
            }
            dstPtr = dstPtr.add(mhSize);
            maxDstSize -= mhSize;
            eSize += mhSize;
        }

        {
            let lhSize: usize = FSE_writeNCount(
                dstPtr as *mut c_void,
                maxDstSize,
                litLengthNCount.as_ptr(),
                MaxLL,
                llLog,
            );
            if FSE_isError(lhSize) != 0 {
                eSize = lhSize;
                DISPLAYLEVEL!(notificationLevel, 1, c"FSE_writeNCount error with litlengthNCount \n".as_ptr());
                break 'cleanup;
            }
            dstPtr = dstPtr.add(lhSize);
            maxDstSize -= lhSize;
            eSize += lhSize;
        }

        if maxDstSize < 12 {
            eSize = ERROR(ZSTD_error_dstSize_tooSmall);
            DISPLAYLEVEL!(notificationLevel, 1, c"not enough space to write RepOffsets \n".as_ptr());
            break 'cleanup;
        }
        /* at this stage, we don't use the result of "most common first offset",
         * as the impact of statistics is not properly evaluated */
        MEM_writeLE32(dstPtr.add(0) as *mut c_void, repStartValue[0]);
        MEM_writeLE32(dstPtr.add(4) as *mut c_void, repStartValue[1]);
        MEM_writeLE32(dstPtr.add(8) as *mut c_void, repStartValue[2]);
        eSize += 12;
    }

    /* _cleanup: */
    crate::compress::zstd_compress::ZSTD_freeCDict(esr.dict);
    crate::compress::zstd_compress::ZSTD_freeCCtx(esr.zc);
    free(esr.workPlace);

    eSize
}

/// @returns the maximum repcode value
unsafe fn ZDICT_maxRep(reps: *const U32) -> U32 {
    let mut maxRep: U32 = *reps.add(0);
    let mut r: c_int = 1;
    while r < ZSTD_REP_NUM as c_int {
        let v = *reps.add(r as usize);
        maxRep = if maxRep > v { maxRep } else { v };
        r += 1;
    }
    maxRep
}

const HBUFFSIZE: usize = 256; /* should prove large enough for all entropy headers */

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
    let mut header: [BYTE; HBUFFSIZE] = [0; HBUFFSIZE];
    let compressionLevel: c_int = if params.compressionLevel == 0 {
        ZSTD_CLEVEL_DEFAULT
    } else {
        params.compressionLevel
    };
    let notificationLevel: U32 = params.notificationLevel;
    /* The final dictionary content must be at least as large as the largest repcode */
    let minContentSize: usize = ZDICT_maxRep(repStartValue.as_ptr()) as usize;
    let paddingSize: usize;

    /* check conditions */
    if dictBufferCapacity < dictContentSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    /* dictionary header */
    MEM_writeLE32(header.as_mut_ptr() as *mut c_void, ZSTD_MAGIC_DICTIONARY);
    {
        let randomID: U64 = XXH64(customDictContent, dictContentSize, 0);
        let compliantID: U32 =
            ((randomID % ((1u32 << 31) - 32768) as U64).wrapping_add(32768)) as U32;
        let dictID: U32 = if params.dictID != 0 {
            params.dictID
        } else {
            compliantID
        };
        MEM_writeLE32(header.as_mut_ptr().add(4) as *mut c_void, dictID);
    }
    hSize = 8;

    /* entropy tables */
    DISPLAYLEVEL!(
        notificationLevel,
        2,
        c"\r%70s\r".as_ptr(),
        c"".as_ptr()
    ); /* clean display line */
    DISPLAYLEVEL!(notificationLevel, 2, c"statistics ... \n".as_ptr());
    {
        let eSize: usize = ZDICT_analyzeEntropy(
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
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        paddingSize = minContentSize - dictContentSize;
    } else {
        paddingSize = 0;
    }

    {
        let dictSize: usize = hSize + paddingSize + dictContentSize;

        /* The dictionary consists of the header, optional padding, and the content.
         * The padding comes before the content because the "best" position in the
         * dictionary is the last byte.
         */
        let outDictHeader: *mut BYTE = dictBuffer as *mut BYTE;
        let outDictPadding: *mut BYTE = outDictHeader.add(hSize);
        let outDictContent: *mut BYTE = outDictPadding.add(paddingSize);

        /* First copy the customDictContent into its final location.
         * `customDictContent` and `dictBuffer` may overlap, so we must
         * do this before any other writes into the output buffer.
         * Then copy the header & padding into the output buffer.
         */
        ZSTD_memmove(
            outDictContent as *mut c_void,
            customDictContent,
            dictContentSize,
        );
        ZSTD_memcpy(
            outDictHeader as *mut c_void,
            header.as_ptr() as *const c_void,
            hSize,
        );
        ZSTD_memset(outDictPadding as *mut c_void, 0, paddingSize);

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
    let notificationLevel: U32 = params.notificationLevel;
    let mut hSize: usize = 8;

    /* calculate entropy tables */
    DISPLAYLEVEL!(
        notificationLevel,
        2,
        c"\r%70s\r".as_ptr(),
        c"".as_ptr()
    ); /* clean display line */
    DISPLAYLEVEL!(notificationLevel, 2, c"statistics ... \n".as_ptr());
    {
        let eSize: usize = ZDICT_analyzeEntropy(
            (dictBuffer as *mut c_char).add(hSize) as *mut c_void,
            dictBufferCapacity - hSize,
            compressionLevel,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            (dictBuffer as *mut c_char).add(dictBufferCapacity - dictContentSize) as *const c_void,
            dictContentSize,
            notificationLevel,
        );
        if ZDICT_isError(eSize) != 0 {
            return eSize;
        }
        hSize += eSize;
    }

    /* add dictionary header (after entropy tables) */
    MEM_writeLE32(dictBuffer, ZSTD_MAGIC_DICTIONARY);
    {
        let randomID: U64 = XXH64(
            (dictBuffer as *mut c_char).add(dictBufferCapacity - dictContentSize) as *const c_void,
            dictContentSize,
            0,
        );
        let compliantID: U32 =
            ((randomID % ((1u32 << 31) - 32768) as U64).wrapping_add(32768)) as U32;
        let dictID: U32 = if params.dictID != 0 {
            params.dictID
        } else {
            compliantID
        };
        MEM_writeLE32((dictBuffer as *mut c_char).add(4) as *mut c_void, dictID);
    }

    if hSize + dictContentSize < dictBufferCapacity {
        ZSTD_memmove(
            (dictBuffer as *mut c_char).add(hSize) as *mut c_void,
            (dictBuffer as *mut c_char).add(dictBufferCapacity - dictContentSize) as *const c_void,
            dictContentSize,
        );
    }
    if dictBufferCapacity < hSize + dictContentSize {
        dictBufferCapacity
    } else {
        hSize + dictContentSize
    }
}

/// `ZDICT_trainFromBuffer_unsafe_legacy()` :
/// Warning : `samplesBuffer` must be followed by noisy guard band !!!
/// @return : size of dictionary, or an error code which can be tested with ZDICT_isError()
unsafe fn ZDICT_trainFromBuffer_unsafe_legacy(
    dictBuffer: *mut c_void,
    maxDictSize: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    params: ZDICT_legacy_params_t,
) -> usize {
    let dictListSize: U32 = {
        let a = if DICTLISTSIZE_DEFAULT > nbSamples {
            DICTLISTSIZE_DEFAULT
        } else {
            nbSamples
        };
        let b = (maxDictSize / 16) as U32;
        if a > b { a } else { b }
    };
    let dictList: *mut dictItem =
        malloc(dictListSize as usize * core::mem::size_of::<dictItem>()) as *mut dictItem;
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
        let nb: c_uint = if 25 < (*dictList.add(0)).pos {
            25
        } else {
            (*dictList.add(0)).pos
        };
        let dictContentSize: c_uint = ZDICT_dictSize(dictList);
        let mut u: c_uint;
        DISPLAYLEVEL!(
            notificationLevel,
            3,
            c"\n %u segments found, of total size %u \n".as_ptr(),
            ((*dictList.add(0)).pos as c_uint).wrapping_sub(1),
            dictContentSize as c_uint,
        );
        DISPLAYLEVEL!(
            notificationLevel,
            3,
            c"list %u best segments \n".as_ptr(),
            nb.wrapping_sub(1)
        );
        u = 1;
        while u < nb {
            let pos: c_uint = (*dictList.add(u as usize)).pos;
            let length: c_uint = (*dictList.add(u as usize)).length;
            let printedLength: U32 = if 40 < length { 40 } else { length };
            if (pos as usize > samplesBuffSize)
                || ((pos as usize + length as usize) > samplesBuffSize)
            {
                free(dictList as *mut c_void);
                return ERROR(ZSTD_error_GENERIC); /* should never happen */
            }
            DISPLAYLEVEL!(
                notificationLevel,
                3,
                c"%3u:%3u bytes at pos %8u, savings %7u bytes |".as_ptr(),
                u as c_uint,
                length as c_uint,
                pos as c_uint,
                (*dictList.add(u as usize)).savings as c_uint,
            );
            ZDICT_printHex(
                (samplesBuffer as *const c_char).add(pos as usize) as *const c_void,
                printedLength as usize,
            );
            DISPLAYLEVEL!(notificationLevel, 3, c"| \n".as_ptr());
            u += 1;
        }
    }

    /* create dictionary */
    {
        let mut dictContentSize: c_uint = ZDICT_dictSize(dictList);
        if dictContentSize < ZDICT_CONTENTSIZE_MIN {
            free(dictList as *mut c_void);
            return ERROR(ZSTD_error_dictionaryCreation_failed);
        } /* dictionary content too small */
        if (dictContentSize as usize) < targetDictSize / 4 {
            DISPLAYLEVEL!(
                notificationLevel,
                2,
                c"!  warning : selected content significantly smaller than requested (%u < %u) \n"
                    .as_ptr(),
                dictContentSize as c_uint,
                maxDictSize as c_uint,
            );
            if samplesBuffSize < 10 * targetDictSize {
                DISPLAYLEVEL!(
                    notificationLevel,
                    2,
                    c"!  consider increasing the number of samples (total size : %u MB)\n".as_ptr(),
                    (samplesBuffSize >> 20) as c_uint,
                );
            }
            if minRep > MINRATIO {
                DISPLAYLEVEL!(
                    notificationLevel,
                    2,
                    c"!  consider increasing selectivity to produce larger dictionary (-s%u) \n"
                        .as_ptr(),
                    selectivity.wrapping_add(1) as c_uint,
                );
                DISPLAYLEVEL!(
                    notificationLevel,
                    2,
                    c"!  note : larger dictionaries are not necessarily better, test its efficiency on samples \n".as_ptr()
                );
            }
        }

        if ((dictContentSize as usize) > targetDictSize * 3)
            && (nbSamples > 2 * MINRATIO)
            && (selectivity > 1)
        {
            let mut proposedSelectivity: c_uint = selectivity - 1;
            while (nbSamples >> proposedSelectivity) <= MINRATIO {
                proposedSelectivity -= 1;
            }
            DISPLAYLEVEL!(
                notificationLevel,
                2,
                c"!  note : calculated dictionary significantly larger than requested (%u > %u) \n"
                    .as_ptr(),
                dictContentSize as c_uint,
                maxDictSize as c_uint,
            );
            DISPLAYLEVEL!(
                notificationLevel,
                2,
                c"!  consider increasing dictionary size, or produce denser dictionary (-s%u) \n"
                    .as_ptr(),
                proposedSelectivity as c_uint,
            );
            DISPLAYLEVEL!(
                notificationLevel,
                2,
                c"!  always test dictionary efficiency on real samples \n".as_ptr()
            );
        }

        /* limit dictionary size */
        {
            let max: U32 = (*dictList).pos; /* convention : nb of useful elts within dictList */
            let mut currentSize: U32 = 0;
            let mut n: U32 = 1;
            while n < max {
                currentSize = currentSize.wrapping_add((*dictList.add(n as usize)).length);
                if currentSize as usize > targetDictSize {
                    currentSize =
                        currentSize.wrapping_sub((*dictList.add(n as usize)).length);
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
            let mut ptr: *mut BYTE = (dictBuffer as *mut BYTE).add(maxDictSize);
            u = 1;
            while u < (*dictList).pos {
                let l: U32 = (*dictList.add(u as usize)).length;
                ptr = ptr.sub(l as usize);
                if ptr < dictBuffer as *mut BYTE {
                    free(dictList as *mut c_void);
                    return ERROR(ZSTD_error_GENERIC);
                } /* should not happen */
                ZSTD_memcpy(
                    ptr as *mut c_void,
                    (samplesBuffer as *const c_char).add((*dictList.add(u as usize)).pos as usize)
                        as *const c_void,
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

/// `ZDICT_trainFromBuffer_legacy()` :
/// issue : samplesBuffer need to be followed by a noisy guard band.
/// work around : duplicate the buffer, and add the noise
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
        return ERROR(ZSTD_error_memory_allocation);
    }

    ZSTD_memcpy(newBuff, samplesBuffer, sBuffSize);
    /* guard band, for end of buffer condition */
    ZDICT_fillNoise(
        (newBuff as *mut c_char).add(sBuffSize) as *mut c_void,
        NOISELENGTH,
    );

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
    let mut params: ZDICT_fastCover_params_t = ZDICT_fastCover_params_t::default();
    ZSTD_memset(
        &mut params as *mut ZDICT_fastCover_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_fastCover_params_t>(),
    );
    params.d = 8;
    params.steps = 4;
    /* Use default level since no compression level information is available */
    params.zParams.compressionLevel = ZSTD_CLEVEL_DEFAULT;
    crate::dictbuilder::fastcover::ZDICT_optimizeTrainFromBuffer_fastCover(
        dictBuffer,
        dictBufferCapacity,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        &raw mut params,
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
    let mut params: ZDICT_params_t = ZDICT_params_t::default();
    ZSTD_memset(
        &mut params as *mut ZDICT_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_params_t>(),
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
