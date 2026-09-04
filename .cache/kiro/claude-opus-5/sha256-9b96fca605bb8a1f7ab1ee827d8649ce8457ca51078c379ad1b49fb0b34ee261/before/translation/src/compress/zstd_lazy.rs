//! Translation of `compress/zstd_lazy.c` — greedy / lazy / lazy2 / btlazy2
//! match finders, plus the row-based (SIMD) hash match finder.
//!
//! Literal, semantics-preserving transliteration. Build configuration:
//! `DYNAMIC_BMI2=0`, no `ZSTD_MULTITHREAD`, `DEBUGLEVEL 0`
//! (asserts / DEBUGLOG dropped). Target is x86_64, so `ZSTD_ARCH_X86_SSE2`
//! is defined and the SSE2 path of `ZSTD_row_getMatchMask` is taken.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_assignments)]

use core::ffi::c_void;
use core::ptr::null_mut;

use crate::common::bits::*;
use crate::common::mem::*;
use crate::common::zstd_internal::*;
use crate::common::zstd_h::ZSTD_compressionParameters;

use crate::compress::zstd_compress_internal::*;

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{_mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8};

/* PREFETCH_L1 : in this build (compiled with SSE2) maps to _mm_prefetch T0.
 * Prefetching has no observable effect on output, so it is a no-op here. */
#[inline(always)]
unsafe fn PREFETCH_L1<T>(_ptr: *const T) {}

pub const kLazySkippingStep: U32 = 8;

/* from zstd_lazy.h */
pub const ZSTD_LAZY_DDSS_BUCKET_LOG: U32 = 2;
pub const ZSTD_ROW_HASH_TAG_BITS: U32 = 8;

/* *************************************
*  Binary Tree search
***************************************/

pub unsafe fn ZSTD_updateDUBT(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    mls: U32,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;

    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog - 1;
    let btMask: U32 = (1u32 << btLog) - 1;

    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as U32;
    let mut idx: U32 = (*ms).nextToUpdate;

    let _ = iend;

    while idx < target {
        let h: size_t = ZSTD_hashPtr(base.wrapping_add(idx as usize) as *const c_void, hashLog, mls);
        let matchIndex: U32 = *hashTable.wrapping_add(h as usize);

        let nextCandidatePtr: *mut U32 = bt.wrapping_add((2 * (idx & btMask)) as usize);
        let sortMarkPtr: *mut U32 = nextCandidatePtr.wrapping_add(1);

        *hashTable.wrapping_add(h as usize) = idx; /* Update Hash Table */
        *nextCandidatePtr = matchIndex; /* update BT like a chain */
        *sortMarkPtr = ZSTD_DUBT_UNSORTED_MARK;
        idx = idx.wrapping_add(1);
    }
    (*ms).nextToUpdate = target;
}

/** ZSTD_insertDUBT1() :
 *  sort one already inserted but unsorted position */
pub unsafe fn ZSTD_insertDUBT1(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    inputEnd: *const BYTE,
    mut nbCompares: U32,
    btLow: U32,
    dictMode: ZSTD_dictMode_e,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog - 1;
    let btMask: U32 = (1u32 << btLog) - 1;
    let mut commonLengthSmaller: size_t = 0;
    let mut commonLengthLarger: size_t = 0;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let ip: *const BYTE = if curr >= dictLimit {
        base.wrapping_add(curr as usize)
    } else {
        dictBase.wrapping_add(curr as usize)
    };
    let iend: *const BYTE = if curr >= dictLimit {
        inputEnd
    } else {
        dictBase.wrapping_add(dictLimit as usize)
    };
    let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
    let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
    let mut r#match: *const BYTE;
    let mut smallerPtr: *mut U32 = bt.wrapping_add((2 * (curr & btMask)) as usize);
    let mut largerPtr: *mut U32 = smallerPtr.wrapping_add(1);
    let mut matchIndex: U32 = *smallerPtr;
    let mut dummy32: U32 = 0;
    let windowValid: U32 = (*ms).window.lowLimit;
    let maxDistance: U32 = 1u32 << (*cParams).windowLog;
    let windowLow: U32 = if curr - windowValid > maxDistance {
        curr - maxDistance
    } else {
        windowValid
    };

    while nbCompares != 0 && (matchIndex > windowLow) {
        let nextPtr: *mut U32 = bt.wrapping_add((2 * (matchIndex & btMask)) as usize);
        let mut matchLength: size_t = MIN(commonLengthSmaller, commonLengthLarger);

        if (dictMode != ZSTD_extDict)
            || ((matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t)
            || (curr < dictLimit)
        {
            let mBase: *const BYTE = if (dictMode != ZSTD_extDict)
                || ((matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t)
            {
                base
            } else {
                dictBase
            };
            r#match = mBase.wrapping_add(matchIndex as usize);
            matchLength += ZSTD_count(
                ip.wrapping_add(matchLength),
                r#match.wrapping_add(matchLength),
                iend,
            );
        } else {
            r#match = dictBase.wrapping_add(matchIndex as usize);
            matchLength += ZSTD_count_2segments(
                ip.wrapping_add(matchLength),
                r#match.wrapping_add(matchLength),
                iend,
                dictEnd,
                prefixStart,
            );
            if (matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t {
                r#match = base.wrapping_add(matchIndex as usize);
            }
        }

        if ip.wrapping_add(matchLength) == iend {
            /* equal : no way to know if inf or sup */
            break;
        }

        if *r#match.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
            /* match is smaller than current */
            *smallerPtr = matchIndex;
            commonLengthSmaller = matchLength;
            if matchIndex <= btLow {
                smallerPtr = &mut dummy32;
                break;
            }
            smallerPtr = nextPtr.wrapping_add(1);
            matchIndex = *nextPtr.wrapping_add(1);
        } else {
            /* match is larger than current */
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &mut dummy32;
                break;
            }
            largerPtr = nextPtr;
            matchIndex = *nextPtr;
        }
        nbCompares -= 1;
    }

    *smallerPtr = 0;
    *largerPtr = 0;
}

pub unsafe fn ZSTD_DUBT_findBetterDictMatch(
    ms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    offsetPtr: *mut size_t,
    mut bestLength: size_t,
    mut nbCompares: U32,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dmsCParams: *const ZSTD_compressionParameters = &(*dms).cParams;
    let dictHashTable: *const U32 = (*dms).hashTable;
    let hashLog: U32 = (*dmsCParams).hashLog;
    let h: size_t = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut dictMatchIndex: U32 = *dictHashTable.wrapping_add(h as usize);

    let base: *const BYTE = (*ms).window.base;
    let prefixStart: *const BYTE = base.wrapping_add((*ms).window.dictLimit as usize);
    let curr: U32 = ip.offset_from(base) as U32;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictHighLimit: U32 = (*dms).window.nextSrc.offset_from((*dms).window.base) as U32;
    let dictLowLimit: U32 = (*dms).window.lowLimit;
    let dictIndexDelta: U32 = (*ms).window.lowLimit.wrapping_sub(dictHighLimit);

    let dictBt: *mut U32 = (*dms).chainTable;
    let btLog: U32 = (*dmsCParams).chainLog - 1;
    let btMask: U32 = (1u32 << btLog) - 1;
    let btLow: U32 = if btMask >= dictHighLimit - dictLowLimit {
        dictLowLimit
    } else {
        dictHighLimit - btMask
    };

    let mut commonLengthSmaller: size_t = 0;
    let mut commonLengthLarger: size_t = 0;

    let _ = dictMode;

    while nbCompares != 0 && (dictMatchIndex > dictLowLimit) {
        let nextPtr: *const U32 = dictBt.wrapping_add((2 * (dictMatchIndex & btMask)) as usize);
        let mut matchLength: size_t = MIN(commonLengthSmaller, commonLengthLarger);
        let mut r#match: *const BYTE = dictBase.wrapping_add(dictMatchIndex as usize);
        matchLength += ZSTD_count_2segments(
            ip.wrapping_add(matchLength),
            r#match.wrapping_add(matchLength),
            iend,
            dictEnd,
            prefixStart,
        );
        if (dictMatchIndex as size_t).wrapping_add(matchLength) >= dictHighLimit as size_t {
            r#match = base
                .wrapping_add(dictMatchIndex as usize)
                .wrapping_add(dictIndexDelta as usize);
        }

        if matchLength > bestLength {
            let matchIndex: U32 = dictMatchIndex.wrapping_add(dictIndexDelta);
            if (4 * (matchLength.wrapping_sub(bestLength) as i32))
                > (ZSTD_highbit32(curr.wrapping_sub(matchIndex).wrapping_add(1)) as i32
                    - ZSTD_highbit32((*offsetPtr as U32).wrapping_add(1)) as i32)
            {
                bestLength = matchLength;
                *offsetPtr = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex)) as size_t;
            }
            if ip.wrapping_add(matchLength) == iend {
                /* reached end of input */
                break;
            }
        }

        if *r#match.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
            if dictMatchIndex <= btLow {
                break;
            }
            commonLengthSmaller = matchLength;
            dictMatchIndex = *nextPtr.wrapping_add(1);
        } else {
            /* match is larger than current */
            if dictMatchIndex <= btLow {
                break;
            }
            commonLengthLarger = matchLength;
            dictMatchIndex = *nextPtr;
        }
        nbCompares -= 1;
    }

    bestLength
}

pub unsafe fn ZSTD_DUBT_findBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    offBasePtr: *mut size_t,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;
    let h: size_t = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut matchIndex: U32 = *hashTable.wrapping_add(h as usize);

    let base: *const BYTE = (*ms).window.base;
    let curr: U32 = ip.offset_from(base) as U32;
    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, (*cParams).windowLog);

    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog - 1;
    let btMask: U32 = (1u32 << btLog) - 1;
    let btLow: U32 = if btMask >= curr { 0 } else { curr - btMask };
    let unsortLimit: U32 = MAX(btLow, windowLow);

    let mut nextCandidate: *mut U32 = bt.wrapping_add((2 * (matchIndex & btMask)) as usize);
    let mut unsortedMark: *mut U32 = bt.wrapping_add((2 * (matchIndex & btMask) + 1) as usize);
    let mut nbCompares: U32 = 1u32 << (*cParams).searchLog;
    let mut nbCandidates: U32 = nbCompares;
    let mut previousCandidate: U32 = 0;

    /* reach end of unsorted candidates list */
    while (matchIndex > unsortLimit)
        && (*unsortedMark == ZSTD_DUBT_UNSORTED_MARK)
        && (nbCandidates > 1)
    {
        *unsortedMark = previousCandidate;
        previousCandidate = matchIndex;
        matchIndex = *nextCandidate;
        nextCandidate = bt.wrapping_add((2 * (matchIndex & btMask)) as usize);
        unsortedMark = bt.wrapping_add((2 * (matchIndex & btMask) + 1) as usize);
        nbCandidates -= 1;
    }

    /* nullify last candidate if it's still unsorted */
    if (matchIndex > unsortLimit) && (*unsortedMark == ZSTD_DUBT_UNSORTED_MARK) {
        *nextCandidate = 0;
        *unsortedMark = 0;
    }

    /* batch sort stacked candidates */
    matchIndex = previousCandidate;
    while matchIndex != 0 {
        let nextCandidateIdxPtr: *mut U32 =
            bt.wrapping_add((2 * (matchIndex & btMask) + 1) as usize);
        let nextCandidateIdx: U32 = *nextCandidateIdxPtr;
        ZSTD_insertDUBT1(ms, matchIndex, iend, nbCandidates, unsortLimit, dictMode);
        matchIndex = nextCandidateIdx;
        nbCandidates += 1;
    }

    /* find longest match */
    {
        let mut commonLengthSmaller: size_t = 0;
        let mut commonLengthLarger: size_t = 0;
        let dictBase: *const BYTE = (*ms).window.dictBase;
        let dictLimit: U32 = (*ms).window.dictLimit;
        let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
        let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
        let mut smallerPtr: *mut U32 = bt.wrapping_add((2 * (curr & btMask)) as usize);
        let mut largerPtr: *mut U32 = bt.wrapping_add((2 * (curr & btMask) + 1) as usize);
        let mut matchEndIdx: U32 = curr + 8 + 1;
        let mut dummy32: U32 = 0;
        let mut bestLength: size_t = 0;

        matchIndex = *hashTable.wrapping_add(h as usize);
        *hashTable.wrapping_add(h as usize) = curr; /* Update Hash Table */

        while nbCompares != 0 && (matchIndex > windowLow) {
            let nextPtr: *const U32 = bt.wrapping_add((2 * (matchIndex & btMask)) as usize);
            let mut matchLength: size_t = MIN(commonLengthSmaller, commonLengthLarger);
            let r#match: *const BYTE;

            if (dictMode != ZSTD_extDict)
                || ((matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t)
            {
                r#match = base.wrapping_add(matchIndex as usize);
                matchLength += ZSTD_count(
                    ip.wrapping_add(matchLength),
                    r#match.wrapping_add(matchLength),
                    iend,
                );
            } else {
                let mut m: *const BYTE = dictBase.wrapping_add(matchIndex as usize);
                matchLength += ZSTD_count_2segments(
                    ip.wrapping_add(matchLength),
                    m.wrapping_add(matchLength),
                    iend,
                    dictEnd,
                    prefixStart,
                );
                if (matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t {
                    m = base.wrapping_add(matchIndex as usize);
                }
                r#match = m;
            }

            if matchLength > bestLength {
                if matchLength > (matchEndIdx.wrapping_sub(matchIndex)) as size_t {
                    matchEndIdx = matchIndex.wrapping_add(matchLength as U32);
                }
                if (4 * (matchLength.wrapping_sub(bestLength) as i32))
                    > (ZSTD_highbit32(curr.wrapping_sub(matchIndex).wrapping_add(1)) as i32
                        - ZSTD_highbit32(*offBasePtr as U32) as i32)
                {
                    bestLength = matchLength;
                    *offBasePtr = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex)) as size_t;
                }
                if ip.wrapping_add(matchLength) == iend {
                    /* equal : no way to know if inf or sup */
                    if dictMode == ZSTD_dictMatchState {
                        nbCompares = 0;
                    }
                    break;
                }
            }

            if *r#match.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
                /* match is smaller than current */
                *smallerPtr = matchIndex;
                commonLengthSmaller = matchLength;
                if matchIndex <= btLow {
                    smallerPtr = &mut dummy32;
                    break;
                }
                smallerPtr = (nextPtr as *mut U32).wrapping_add(1);
                matchIndex = *nextPtr.wrapping_add(1);
            } else {
                /* match is larger than current */
                *largerPtr = matchIndex;
                commonLengthLarger = matchLength;
                if matchIndex <= btLow {
                    largerPtr = &mut dummy32;
                    break;
                }
                largerPtr = nextPtr as *mut U32;
                matchIndex = *nextPtr;
            }
            nbCompares -= 1;
        }

        *smallerPtr = 0;
        *largerPtr = 0;

        if dictMode == ZSTD_dictMatchState && nbCompares != 0 {
            bestLength = ZSTD_DUBT_findBetterDictMatch(
                ms, ip, iend, offBasePtr, bestLength, nbCompares, mls, dictMode,
            );
        }

        (*ms).nextToUpdate = matchEndIdx - 8; /* skip repetitive patterns */
        return bestLength;
    }
}

/** ZSTD_BtFindBestMatch() : Tree updater, providing best match */
pub unsafe fn ZSTD_BtFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut size_t,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    if ip < (*ms).window.base.wrapping_add((*ms).nextToUpdate as usize) {
        return 0; /* skipped area */
    }
    ZSTD_updateDUBT(ms, ip, iLimit, mls);
    ZSTD_DUBT_findBestMatch(ms, ip, iLimit, offBasePtr, mls, dictMode)
}

/* *********************************
* Dedicated dict search
***********************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_dedicatedDictSearch_lazy_loadDictionary(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
) {
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as U32;
    let hashTable: *mut U32 = (*ms).hashTable;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainSize: U32 = 1u32 << (*ms).cParams.chainLog;
    let mut idx: U32 = (*ms).nextToUpdate;
    let minChain: U32 = if chainSize < target - idx {
        target - chainSize
    } else {
        idx
    };
    let bucketSize: U32 = 1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG;
    let cacheSize: U32 = bucketSize - 1;
    let chainAttempts: U32 = (1u32 << (*ms).cParams.searchLog) - cacheSize;
    let chainLimit: U32 = if chainAttempts > 255 { 255 } else { chainAttempts };

    let hashLog: U32 = (*ms).cParams.hashLog - ZSTD_LAZY_DDSS_BUCKET_LOG;
    let tmpHashTable: *mut U32 = hashTable;
    let tmpChainTable: *mut U32 = hashTable.wrapping_add((1usize) << hashLog);
    let tmpChainSize: U32 =
        (((1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG) - 1) as U32).wrapping_shl(hashLog);
    let tmpMinChain: U32 = if tmpChainSize < target {
        target - tmpChainSize
    } else {
        idx
    };
    let mut hashIdx: U32;

    /* fill conventional hash table and conventional chain table */
    while idx < target {
        let h: U32 = ZSTD_hashPtr(
            base.wrapping_add(idx as usize) as *const c_void,
            hashLog,
            (*ms).cParams.minMatch,
        ) as U32;
        if idx >= tmpMinChain {
            *tmpChainTable.wrapping_add((idx - tmpMinChain) as usize) =
                *hashTable.wrapping_add(h as usize);
        }
        *tmpHashTable.wrapping_add(h as usize) = idx;
        idx += 1;
    }

    /* sort chains into ddss chain table */
    {
        let mut chainPos: U32 = 0;
        hashIdx = 0;
        while hashIdx < (1u32 << hashLog) {
            let mut count: U32;
            let mut countBeyondMinChain: U32 = 0;
            let mut i: U32 = *tmpHashTable.wrapping_add(hashIdx as usize);
            count = 0;
            while i >= tmpMinChain && count < cacheSize {
                if i < minChain {
                    countBeyondMinChain += 1;
                }
                i = *tmpChainTable.wrapping_add((i - tmpMinChain) as usize);
                count += 1;
            }
            if count == cacheSize {
                count = 0;
                while count < chainLimit {
                    if i < minChain {
                        if i == 0 || {
                            countBeyondMinChain += 1;
                            countBeyondMinChain > cacheSize
                        } {
                            break;
                        }
                    }
                    *chainTable.wrapping_add(chainPos as usize) = i;
                    chainPos += 1;
                    count += 1;
                    if i < tmpMinChain {
                        break;
                    }
                    i = *tmpChainTable.wrapping_add((i - tmpMinChain) as usize);
                }
            } else {
                count = 0;
            }
            if count != 0 {
                *tmpHashTable.wrapping_add(hashIdx as usize) =
                    ((chainPos - count) << 8) + count;
            } else {
                *tmpHashTable.wrapping_add(hashIdx as usize) = 0;
            }
            hashIdx += 1;
        }
    }

    /* move chain pointers into the last entry of each hash bucket */
    hashIdx = 1u32 << hashLog;
    while hashIdx != 0 {
        hashIdx -= 1;
        let bucketIdx: U32 = hashIdx << ZSTD_LAZY_DDSS_BUCKET_LOG;
        let chainPackedPointer: U32 = *tmpHashTable.wrapping_add(hashIdx as usize);
        let mut i: U32 = 0;
        while i < cacheSize {
            *hashTable.wrapping_add((bucketIdx + i) as usize) = 0;
            i += 1;
        }
        *hashTable.wrapping_add((bucketIdx + bucketSize - 1) as usize) = chainPackedPointer;
    }

    /* fill the buckets of the hash table */
    idx = (*ms).nextToUpdate;
    while idx < target {
        let h: U32 = (ZSTD_hashPtr(
            base.wrapping_add(idx as usize) as *const c_void,
            hashLog,
            (*ms).cParams.minMatch,
        ) as U32)
            << ZSTD_LAZY_DDSS_BUCKET_LOG;
        let mut i: U32 = cacheSize - 1;
        /* Shift hash cache down 1. */
        while i != 0 {
            *hashTable.wrapping_add((h + i) as usize) =
                *hashTable.wrapping_add((h + i - 1) as usize);
            i -= 1;
        }
        *hashTable.wrapping_add(h as usize) = idx;
        idx += 1;
    }

    (*ms).nextToUpdate = target;
}

/* Returns the longest match length found in the dedicated dict search structure. */
pub unsafe fn ZSTD_dedicatedDictSearch_lazy_search(
    offsetPtr: *mut size_t,
    mut ml: size_t,
    nbAttempts: U32,
    dms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    prefixStart: *const BYTE,
    curr: U32,
    dictLimit: U32,
    ddsIdx: size_t,
) -> size_t {
    let ddsLowestIndex: U32 = (*dms).window.dictLimit;
    let ddsBase: *const BYTE = (*dms).window.base;
    let ddsEnd: *const BYTE = (*dms).window.nextSrc;
    let ddsSize: U32 = ddsEnd.offset_from(ddsBase) as U32;
    let ddsIndexDelta: U32 = dictLimit.wrapping_sub(ddsSize);
    let bucketSize: U32 = 1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG;
    let bucketLimit: U32 = if nbAttempts < bucketSize - 1 {
        nbAttempts
    } else {
        bucketSize - 1
    };
    let mut ddsAttempt: U32;
    let mut matchIndex: U32;

    let _ = ddsLowestIndex;

    ddsAttempt = 0;
    while ddsAttempt < bucketSize - 1 {
        PREFETCH_L1(
            ddsBase.wrapping_add(
                *(*dms).hashTable.wrapping_add((ddsIdx + ddsAttempt as size_t) as usize) as usize,
            ),
        );
        ddsAttempt += 1;
    }

    {
        let chainPackedPointer: U32 =
            *(*dms).hashTable.wrapping_add((ddsIdx + (bucketSize as size_t) - 1) as usize);
        let chainIndex: U32 = chainPackedPointer >> 8;
        PREFETCH_L1((*dms).chainTable.wrapping_add(chainIndex as usize));
    }

    ddsAttempt = 0;
    while ddsAttempt < bucketLimit {
        let mut currentMl: size_t = 0;
        matchIndex = *(*dms).hashTable.wrapping_add((ddsIdx + ddsAttempt as size_t) as usize);
        let r#match: *const BYTE = ddsBase.wrapping_add(matchIndex as usize);

        if matchIndex == 0 {
            return ml;
        }

        if MEM_read32(r#match) == MEM_read32(ip) {
            currentMl = ZSTD_count_2segments(
                ip.wrapping_add(4),
                r#match.wrapping_add(4),
                iLimit,
                ddsEnd,
                prefixStart,
            ) + 4;
        }

        /* save best solution */
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = OFFSET_TO_OFFBASE(
                curr.wrapping_sub(matchIndex.wrapping_add(ddsIndexDelta)),
            ) as size_t;
            if ip.wrapping_add(currentMl) == iLimit {
                return ml;
            }
        }
        ddsAttempt += 1;
    }

    {
        let chainPackedPointer: U32 =
            *(*dms).hashTable.wrapping_add((ddsIdx + (bucketSize as size_t) - 1) as usize);
        let mut chainIndex: U32 = chainPackedPointer >> 8;
        let chainLength: U32 = chainPackedPointer & 0xFF;
        let chainAttempts: U32 = nbAttempts - ddsAttempt;
        let chainLimit: U32 = if chainAttempts > chainLength {
            chainLength
        } else {
            chainAttempts
        };
        let mut chainAttempt: U32;

        chainAttempt = 0;
        while chainAttempt < chainLimit {
            PREFETCH_L1(ddsBase.wrapping_add(
                *(*dms).chainTable.wrapping_add((chainIndex + chainAttempt) as usize) as usize,
            ));
            chainAttempt += 1;
        }

        chainAttempt = 0;
        while chainAttempt < chainLimit {
            let mut currentMl: size_t = 0;
            matchIndex = *(*dms).chainTable.wrapping_add(chainIndex as usize);
            let r#match: *const BYTE = ddsBase.wrapping_add(matchIndex as usize);

            if MEM_read32(r#match) == MEM_read32(ip) {
                currentMl = ZSTD_count_2segments(
                    ip.wrapping_add(4),
                    r#match.wrapping_add(4),
                    iLimit,
                    ddsEnd,
                    prefixStart,
                ) + 4;
            }

            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(
                    curr.wrapping_sub(matchIndex.wrapping_add(ddsIndexDelta)),
                ) as size_t;
                if ip.wrapping_add(currentMl) == iLimit {
                    break;
                }
            }
            chainAttempt += 1;
            chainIndex += 1;
        }
    }
    ml
}

/* *********************************
*  Hash Chain
***********************************/

/* Update chains up to ip (excluded).
   Assumption : always within prefix (i.e. not within extDict) */
pub unsafe fn ZSTD_insertAndFindFirstIndex_internal(
    ms: *mut ZSTD_MatchState_t,
    cParams: *const ZSTD_compressionParameters,
    ip: *const BYTE,
    mls: U32,
    lazySkipping: U32,
) -> U32 {
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainMask: U32 = (1u32 << (*cParams).chainLog) - 1;
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as U32;
    let mut idx: U32 = (*ms).nextToUpdate;

    while idx < target {
        /* catch up */
        let h: size_t = ZSTD_hashPtr(base.wrapping_add(idx as usize) as *const c_void, hashLog, mls);
        *chainTable.wrapping_add((idx & chainMask) as usize) = *hashTable.wrapping_add(h as usize);
        *hashTable.wrapping_add(h as usize) = idx;
        idx += 1;
        /* Stop inserting every position when in the lazy skipping mode. */
        if lazySkipping != 0 {
            break;
        }
    }

    (*ms).nextToUpdate = target;
    *hashTable.wrapping_add(ZSTD_hashPtr(ip as *const c_void, hashLog, mls) as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_insertAndFindFirstIndex(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
) -> U32 {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    ZSTD_insertAndFindFirstIndex_internal(ms, cParams, ip, (*ms).cParams.minMatch, 0)
}

pub unsafe fn ZSTD_HcFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut size_t,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainSize: U32 = 1u32 << (*cParams).chainLog;
    let chainMask: U32 = chainSize - 1;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
    let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
    let curr: U32 = ip.offset_from(base) as U32;
    let maxDistance: U32 = 1u32 << (*cParams).windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinMaxDistance: U32 = if curr - lowestValid > maxDistance {
        curr - maxDistance
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0) as U32;
    let lowLimit: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinMaxDistance
    };
    let minChain: U32 = if curr > chainSize { curr - chainSize } else { 0 };
    let mut nbAttempts: U32 = 1u32 << (*cParams).searchLog;
    let mut ml: size_t = 4 - 1;

    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let ddsHashLog: U32 = if dictMode == ZSTD_dedicatedDictSearch {
        (*dms).cParams.hashLog - ZSTD_LAZY_DDSS_BUCKET_LOG
    } else {
        0
    };
    let ddsIdx: size_t = if dictMode == ZSTD_dedicatedDictSearch {
        (ZSTD_hashPtr(ip as *const c_void, ddsHashLog, mls)) << ZSTD_LAZY_DDSS_BUCKET_LOG
    } else {
        0
    };

    let mut matchIndex: U32;

    if dictMode == ZSTD_dedicatedDictSearch {
        let entry: *const U32 = (*dms).hashTable.wrapping_add(ddsIdx as usize);
        PREFETCH_L1(entry);
    }

    /* HC4 match finder */
    matchIndex = ZSTD_insertAndFindFirstIndex_internal(
        ms,
        cParams,
        ip,
        mls,
        (*ms).lazySkipping as U32,
    );

    while ((matchIndex >= lowLimit) as u32 & (nbAttempts > 0) as u32) != 0 {
        let mut currentMl: size_t = 0;
        if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
            let r#match: *const BYTE = base.wrapping_add(matchIndex as usize);
            /* read 4B starting from (match + ml + 1 - sizeof(U32)) */
            if MEM_read32(r#match.wrapping_offset(ml as isize - 3))
                == MEM_read32(ip.wrapping_offset(ml as isize - 3))
            {
                currentMl = ZSTD_count(ip, r#match, iLimit);
            }
        } else {
            let r#match: *const BYTE = dictBase.wrapping_add(matchIndex as usize);
            if MEM_read32(r#match) == MEM_read32(ip) {
                currentMl = ZSTD_count_2segments(
                    ip.wrapping_add(4),
                    r#match.wrapping_add(4),
                    iLimit,
                    dictEnd,
                    prefixStart,
                ) + 4;
            }
        }

        /* save best solution */
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex)) as size_t;
            if ip.wrapping_add(currentMl) == iLimit {
                break; /* best possible, avoids read overflow on next attempt */
            }
        }

        if matchIndex <= minChain {
            break;
        }
        matchIndex = *chainTable.wrapping_add((matchIndex & chainMask) as usize);
        nbAttempts -= 1;
    }

    if dictMode == ZSTD_dedicatedDictSearch {
        ml = ZSTD_dedicatedDictSearch_lazy_search(
            offsetPtr, ml, nbAttempts, dms, ip, iLimit, prefixStart, curr, dictLimit, ddsIdx,
        );
    } else if dictMode == ZSTD_dictMatchState {
        let dmsChainTable: *const U32 = (*dms).chainTable;
        let dmsChainSize: U32 = 1u32 << (*dms).cParams.chainLog;
        let dmsChainMask: U32 = dmsChainSize - 1;
        let dmsLowestIndex: U32 = (*dms).window.dictLimit;
        let dmsBase: *const BYTE = (*dms).window.base;
        let dmsEnd: *const BYTE = (*dms).window.nextSrc;
        let dmsSize: U32 = dmsEnd.offset_from(dmsBase) as U32;
        let dmsIndexDelta: U32 = dictLimit.wrapping_sub(dmsSize);
        let dmsMinChain: U32 = if dmsSize > dmsChainSize {
            dmsSize - dmsChainSize
        } else {
            0
        };

        matchIndex = *(*dms)
            .hashTable
            .wrapping_add(ZSTD_hashPtr(ip as *const c_void, (*dms).cParams.hashLog, mls) as usize);

        while ((matchIndex >= dmsLowestIndex) as u32 & (nbAttempts > 0) as u32) != 0 {
            let mut currentMl: size_t = 0;
            let r#match: *const BYTE = dmsBase.wrapping_add(matchIndex as usize);
            if MEM_read32(r#match) == MEM_read32(ip) {
                currentMl = ZSTD_count_2segments(
                    ip.wrapping_add(4),
                    r#match.wrapping_add(4),
                    iLimit,
                    dmsEnd,
                    prefixStart,
                ) + 4;
            }

            /* save best solution */
            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(
                    curr.wrapping_sub(matchIndex.wrapping_add(dmsIndexDelta)),
                ) as size_t;
                if ip.wrapping_add(currentMl) == iLimit {
                    break;
                }
            }

            if matchIndex <= dmsMinChain {
                break;
            }

            matchIndex = *dmsChainTable.wrapping_add((matchIndex & dmsChainMask) as usize);
            nbAttempts -= 1;
        }
    }

    ml
}

/* *********************************
* (SIMD) Row-based matchfinder
***********************************/
/* Constants for row-based hash */
pub const ZSTD_ROW_HASH_TAG_MASK: U32 = (1u32 << ZSTD_ROW_HASH_TAG_BITS) - 1;
pub const ZSTD_ROW_HASH_MAX_ENTRIES: U32 = 64;

pub const ZSTD_ROW_HASH_CACHE_MASK: U32 = (ZSTD_ROW_HASH_CACHE_SIZE as U32) - 1;

pub type ZSTD_VecMask = U64;

/* ZSTD_VecMask_next(): counts trailing zeroes. */
pub unsafe fn ZSTD_VecMask_next(val: ZSTD_VecMask) -> U32 {
    ZSTD_countTrailingZeros64(val)
}

/* ZSTD_row_nextIndex(): next index to insert at within a tagTable row. */
pub unsafe fn ZSTD_row_nextIndex(tagRow: *mut BYTE, rowMask: U32) -> U32 {
    let mut next: U32 = ((*tagRow as U32).wrapping_sub(1)) & rowMask;
    next += if next == 0 { rowMask } else { 0 }; /* skip first position */
    *tagRow = next as BYTE;
    next
}

/* ZSTD_isAligned(): checks pointer alignment. */
pub unsafe fn ZSTD_isAligned(ptr: *const c_void, align: size_t) -> i32 {
    (((ptr as size_t) & (align - 1)) == 0) as i32
}

/* ZSTD_row_prefetch(): prefetching for hashTable and tagTable at a given row. */
pub unsafe fn ZSTD_row_prefetch(
    hashTable: *const U32,
    tagTable: *const BYTE,
    relRow: U32,
    rowLog: U32,
) {
    PREFETCH_L1(hashTable.wrapping_add(relRow as usize));
    if rowLog >= 5 {
        PREFETCH_L1(hashTable.wrapping_add((relRow + 16) as usize));
    }
    PREFETCH_L1(tagTable.wrapping_add(relRow as usize));
    if rowLog == 6 {
        PREFETCH_L1(tagTable.wrapping_add((relRow + 32) as usize));
    }
}

/* ZSTD_row_fillHashCache(): fill up the hash cache starting at idx. */
pub unsafe fn ZSTD_row_fillHashCache(
    ms: *mut ZSTD_MatchState_t,
    base: *const BYTE,
    rowLog: U32,
    mls: U32,
    mut idx: U32,
    iLimit: *const BYTE,
) {
    let hashTable: *const U32 = (*ms).hashTable;
    let tagTable: *const BYTE = (*ms).tagTable;
    let hashLog: U32 = (*ms).rowHashLog;
    let maxElemsToPrefetch: U32 = if base.wrapping_add(idx as usize) > iLimit {
        0
    } else {
        (iLimit.offset_from(base.wrapping_add(idx as usize)) + 1) as U32
    };
    let lim: U32 = idx + MIN(ZSTD_ROW_HASH_CACHE_SIZE as U32, maxElemsToPrefetch);

    while idx < lim {
        let hash: U32 = ZSTD_hashPtrSalted(
            base.wrapping_add(idx as usize) as *const c_void,
            hashLog + ZSTD_ROW_HASH_TAG_BITS,
            mls,
            (*ms).hashSalt,
        ) as U32;
        let row: U32 = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        ZSTD_row_prefetch(hashTable, tagTable, row, rowLog);
        (*ms).hashCache[(idx & ZSTD_ROW_HASH_CACHE_MASK) as usize] = hash;
        idx += 1;
    }
}

/* ZSTD_row_nextCachedHash(): returns hash of base+idx and updates the cache. */
pub unsafe fn ZSTD_row_nextCachedHash(
    cache: *mut U32,
    hashTable: *const U32,
    tagTable: *const BYTE,
    base: *const BYTE,
    idx: U32,
    hashLog: U32,
    rowLog: U32,
    mls: U32,
    hashSalt: U64,
) -> U32 {
    let newHash: U32 = ZSTD_hashPtrSalted(
        base.wrapping_add(idx as usize)
            .wrapping_add(ZSTD_ROW_HASH_CACHE_SIZE as usize) as *const c_void,
        hashLog + ZSTD_ROW_HASH_TAG_BITS,
        mls,
        hashSalt,
    ) as U32;
    let row: U32 = (newHash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
    ZSTD_row_prefetch(hashTable, tagTable, row, rowLog);
    {
        let hash: U32 = *cache.wrapping_add((idx & ZSTD_ROW_HASH_CACHE_MASK) as usize);
        *cache.wrapping_add((idx & ZSTD_ROW_HASH_CACHE_MASK) as usize) = newHash;
        hash
    }
}

/* ZSTD_row_update_internalImpl(): updates hash table from updateStartIdx to updateEndIdx. */
pub unsafe fn ZSTD_row_update_internalImpl(
    ms: *mut ZSTD_MatchState_t,
    mut updateStartIdx: U32,
    updateEndIdx: U32,
    mls: U32,
    rowLog: U32,
    rowMask: U32,
    useCache: U32,
) {
    let hashTable: *mut U32 = (*ms).hashTable;
    let tagTable: *mut BYTE = (*ms).tagTable;
    let hashLog: U32 = (*ms).rowHashLog;
    let base: *const BYTE = (*ms).window.base;

    while updateStartIdx < updateEndIdx {
        let hash: U32 = if useCache != 0 {
            ZSTD_row_nextCachedHash(
                (*ms).hashCache.as_mut_ptr(),
                hashTable,
                tagTable,
                base,
                updateStartIdx,
                hashLog,
                rowLog,
                mls,
                (*ms).hashSalt,
            )
        } else {
            ZSTD_hashPtrSalted(
                base.wrapping_add(updateStartIdx as usize) as *const c_void,
                hashLog + ZSTD_ROW_HASH_TAG_BITS,
                mls,
                (*ms).hashSalt,
            ) as U32
        };
        let relRow: U32 = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        let row: *mut U32 = hashTable.wrapping_add(relRow as usize);
        let tagRow: *mut BYTE = tagTable.wrapping_add(relRow as usize);
        let pos: U32 = ZSTD_row_nextIndex(tagRow, rowMask);

        *tagRow.wrapping_add(pos as usize) = (hash & ZSTD_ROW_HASH_TAG_MASK) as BYTE;
        *row.wrapping_add(pos as usize) = updateStartIdx;
        updateStartIdx += 1;
    }
}

/* ZSTD_row_update_internal(): inserts the byte at ip into the hash table, updates nextToUpdate. */
pub unsafe fn ZSTD_row_update_internal(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    mls: U32,
    rowLog: U32,
    rowMask: U32,
    useCache: U32,
) {
    let mut idx: U32 = (*ms).nextToUpdate;
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as U32;
    let kSkipThreshold: U32 = 384;
    let kMaxMatchStartPositionsToUpdate: U32 = 96;
    let kMaxMatchEndPositionsToUpdate: U32 = 32;

    if useCache != 0 {
        if target - idx > kSkipThreshold {
            let bound: U32 = idx + kMaxMatchStartPositionsToUpdate;
            ZSTD_row_update_internalImpl(ms, idx, bound, mls, rowLog, rowMask, useCache);
            idx = target - kMaxMatchEndPositionsToUpdate;
            ZSTD_row_fillHashCache(ms, base, rowLog, mls, idx, ip.wrapping_add(1));
        }
    }
    ZSTD_row_update_internalImpl(ms, idx, target, mls, rowLog, rowMask, useCache);
    (*ms).nextToUpdate = target;
}

/* ZSTD_row_update(): external wrapper. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_row_update(ms: *mut ZSTD_MatchState_t, ip: *const BYTE) {
    let rowLog: U32 = BOUNDED(4, (*ms).cParams.searchLog, 6);
    let rowMask: U32 = (1u32 << rowLog) - 1;
    let mls: U32 = MIN((*ms).cParams.minMatch, 6);

    ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 0);
}

/* Returns the mask width of bits group. On x86_64 (non-NEON) this is 1. */
pub unsafe fn ZSTD_row_matchMaskGroupWidth(rowEntries: U32) -> U32 {
    let _ = rowEntries;
    1
}

/* SSE2 mask computation (x86_64 build takes this path). */
#[cfg(target_arch = "x86_64")]
pub unsafe fn ZSTD_row_getSSEMask(
    nbChunks: i32,
    src: *const BYTE,
    tag: BYTE,
    head: U32,
) -> ZSTD_VecMask {
    let comparisonMask = _mm_set1_epi8(tag as i8);
    let mut matches: [i32; 4] = [0; 4];
    let mut i: i32 = 0;
    while i < nbChunks {
        let chunk = _mm_loadu_si128(src.wrapping_add((16 * i) as usize) as *const _);
        let equalMask = _mm_cmpeq_epi8(chunk, comparisonMask);
        matches[i as usize] = _mm_movemask_epi8(equalMask);
        i += 1;
    }
    if nbChunks == 1 {
        return ZSTD_rotateRight_U16(matches[0] as U16, head) as ZSTD_VecMask;
    }
    if nbChunks == 2 {
        return ZSTD_rotateRight_U32(
            ((matches[1] as U32) << 16) | (matches[0] as U32),
            head,
        ) as ZSTD_VecMask;
    }
    ZSTD_rotateRight_U64(
        ((matches[3] as U64) << 48)
            | ((matches[2] as U64) << 32)
            | ((matches[1] as U64) << 16)
            | (matches[0] as U64),
        head,
    )
}

/* ZSTD_row_getMatchMask(): x86_64 build dispatches to the SSE2 implementation. */
pub unsafe fn ZSTD_row_getMatchMask(
    tagRow: *const BYTE,
    tag: BYTE,
    headGrouped: U32,
    rowEntries: U32,
) -> ZSTD_VecMask {
    let src: *const BYTE = tagRow;
    ZSTD_row_getSSEMask((rowEntries / 16) as i32, src, tag, headGrouped)
}

pub unsafe fn ZSTD_RowFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut size_t,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
    rowLog: U32,
) -> size_t {
    let hashTable: *mut U32 = (*ms).hashTable;
    let tagTable: *mut BYTE = (*ms).tagTable;
    let hashCache: *mut U32 = (*ms).hashCache.as_mut_ptr();
    let hashLog: U32 = (*ms).rowHashLog;
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
    let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
    let curr: U32 = ip.offset_from(base) as U32;
    let maxDistance: U32 = 1u32 << (*cParams).windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinMaxDistance: U32 = if curr - lowestValid > maxDistance {
        curr - maxDistance
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0) as U32;
    let lowLimit: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinMaxDistance
    };
    let rowEntries: U32 = 1u32 << rowLog;
    let rowMask: U32 = rowEntries - 1;
    let cappedSearchLog: U32 = MIN((*cParams).searchLog, rowLog);
    let groupWidth: U32 = ZSTD_row_matchMaskGroupWidth(rowEntries);
    let hashSalt: U64 = (*ms).hashSalt;
    let mut nbAttempts: U32 = 1u32 << cappedSearchLog;
    let mut ml: size_t = 4 - 1;
    let mut hash: U32;

    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;

    let mut ddsIdx: size_t = 0;
    let mut ddsExtraAttempts: U32 = 0;
    let mut dmsTag: U32 = 0;
    let mut dmsRow: *mut U32 = null_mut();
    let mut dmsTagRow: *mut BYTE = null_mut();

    if dictMode == ZSTD_dedicatedDictSearch {
        let ddsHashLog: U32 = (*dms).cParams.hashLog - ZSTD_LAZY_DDSS_BUCKET_LOG;
        {
            ddsIdx = ZSTD_hashPtr(ip as *const c_void, ddsHashLog, mls) << ZSTD_LAZY_DDSS_BUCKET_LOG;
            PREFETCH_L1((*dms).hashTable.wrapping_add(ddsIdx as usize));
        }
        ddsExtraAttempts = if (*cParams).searchLog > rowLog {
            1u32 << ((*cParams).searchLog - rowLog)
        } else {
            0
        };
    }

    if dictMode == ZSTD_dictMatchState {
        let dmsHashTable: *const U32 = (*dms).hashTable;
        let dmsTagTable: *const BYTE = (*dms).tagTable;
        let dmsHash: U32 = ZSTD_hashPtr(
            ip as *const c_void,
            (*dms).rowHashLog + ZSTD_ROW_HASH_TAG_BITS,
            mls,
        ) as U32;
        let dmsRelRow: U32 = (dmsHash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        dmsTag = dmsHash & ZSTD_ROW_HASH_TAG_MASK;
        dmsTagRow = dmsTagTable.wrapping_add(dmsRelRow as usize) as *mut BYTE;
        dmsRow = dmsHashTable.wrapping_add(dmsRelRow as usize) as *mut U32;
        ZSTD_row_prefetch(dmsHashTable, dmsTagTable, dmsRelRow, rowLog);
    }

    /* Update the hashTable and tagTable up to (but not including) ip */
    if (*ms).lazySkipping == 0 {
        ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 1);
        hash = ZSTD_row_nextCachedHash(
            hashCache, hashTable, tagTable, base, curr, hashLog, rowLog, mls, hashSalt,
        );
    } else {
        hash = ZSTD_hashPtrSalted(
            ip as *const c_void,
            hashLog + ZSTD_ROW_HASH_TAG_BITS,
            mls,
            hashSalt,
        ) as U32;
        (*ms).nextToUpdate = curr;
    }
    (*ms).hashSaltEntropy = (*ms).hashSaltEntropy.wrapping_add(hash); /* collect salt entropy */

    {
        /* Get the hash for ip, compute the appropriate row */
        let relRow: U32 = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        let tag: U32 = hash & ZSTD_ROW_HASH_TAG_MASK;
        let row: *mut U32 = hashTable.wrapping_add(relRow as usize);
        let tagRow: *mut BYTE = tagTable.wrapping_add(relRow as usize);
        let headGrouped: U32 = ((*tagRow as U32) & rowMask) * groupWidth;
        let mut matchBuffer: [U32; ZSTD_ROW_HASH_MAX_ENTRIES as usize] =
            [0; ZSTD_ROW_HASH_MAX_ENTRIES as usize];
        let mut numMatches: size_t = 0;
        let mut currMatch: size_t = 0;
        let mut matches: ZSTD_VecMask =
            ZSTD_row_getMatchMask(tagRow, tag as BYTE, headGrouped, rowEntries);

        /* Cycle through the matches and prefetch */
        while (matches > 0) && (nbAttempts > 0) {
            let matchPos: U32 =
                ((headGrouped + ZSTD_VecMask_next(matches)) / groupWidth) & rowMask;
            let matchIndex: U32 = *row.wrapping_add(matchPos as usize);
            if matchPos == 0 {
                matches &= matches - 1;
                continue;
            }
            if matchIndex < lowLimit {
                break;
            }
            if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
                PREFETCH_L1(base.wrapping_add(matchIndex as usize));
            } else {
                PREFETCH_L1(dictBase.wrapping_add(matchIndex as usize));
            }
            matchBuffer[numMatches as usize] = matchIndex;
            numMatches += 1;
            nbAttempts -= 1;
            matches &= matches - 1;
        }

        /* Speed opt: insert current byte into hashtable too. */
        {
            let pos: U32 = ZSTD_row_nextIndex(tagRow, rowMask);
            *tagRow.wrapping_add(pos as usize) = tag as BYTE;
            *row.wrapping_add(pos as usize) = (*ms).nextToUpdate;
            (*ms).nextToUpdate += 1;
        }

        /* Return the longest match */
        while currMatch < numMatches {
            let matchIndex: U32 = matchBuffer[currMatch as usize];
            let mut currentMl: size_t = 0;

            if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
                let r#match: *const BYTE = base.wrapping_add(matchIndex as usize);
                if MEM_read32(r#match.wrapping_offset(ml as isize - 3))
                    == MEM_read32(ip.wrapping_offset(ml as isize - 3))
                {
                    currentMl = ZSTD_count(ip, r#match, iLimit);
                }
            } else {
                let r#match: *const BYTE = dictBase.wrapping_add(matchIndex as usize);
                if MEM_read32(r#match) == MEM_read32(ip) {
                    currentMl = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        r#match.wrapping_add(4),
                        iLimit,
                        dictEnd,
                        prefixStart,
                    ) + 4;
                }
            }

            /* Save best solution */
            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex)) as size_t;
                if ip.wrapping_add(currentMl) == iLimit {
                    break;
                }
            }
            currMatch += 1;
        }
    }

    if dictMode == ZSTD_dedicatedDictSearch {
        ml = ZSTD_dedicatedDictSearch_lazy_search(
            offsetPtr,
            ml,
            nbAttempts + ddsExtraAttempts,
            dms,
            ip,
            iLimit,
            prefixStart,
            curr,
            dictLimit,
            ddsIdx,
        );
    } else if dictMode == ZSTD_dictMatchState {
        let dmsLowestIndex: U32 = (*dms).window.dictLimit;
        let dmsBase: *const BYTE = (*dms).window.base;
        let dmsEnd: *const BYTE = (*dms).window.nextSrc;
        let dmsSize: U32 = dmsEnd.offset_from(dmsBase) as U32;
        let dmsIndexDelta: U32 = dictLimit.wrapping_sub(dmsSize);

        {
            let headGrouped: U32 = ((*dmsTagRow as U32) & rowMask) * groupWidth;
            let mut matchBuffer: [U32; ZSTD_ROW_HASH_MAX_ENTRIES as usize] =
                [0; ZSTD_ROW_HASH_MAX_ENTRIES as usize];
            let mut numMatches: size_t = 0;
            let mut currMatch: size_t = 0;
            let mut matches: ZSTD_VecMask =
                ZSTD_row_getMatchMask(dmsTagRow, dmsTag as BYTE, headGrouped, rowEntries);

            while (matches > 0) && (nbAttempts > 0) {
                let matchPos: U32 =
                    ((headGrouped + ZSTD_VecMask_next(matches)) / groupWidth) & rowMask;
                let matchIndex: U32 = *dmsRow.wrapping_add(matchPos as usize);
                if matchPos == 0 {
                    matches &= matches - 1;
                    continue;
                }
                if matchIndex < dmsLowestIndex {
                    break;
                }
                PREFETCH_L1(dmsBase.wrapping_add(matchIndex as usize));
                matchBuffer[numMatches as usize] = matchIndex;
                numMatches += 1;
                nbAttempts -= 1;
                matches &= matches - 1;
            }

            /* Return the longest match */
            while currMatch < numMatches {
                let matchIndex: U32 = matchBuffer[currMatch as usize];
                let mut currentMl: size_t = 0;

                {
                    let r#match: *const BYTE = dmsBase.wrapping_add(matchIndex as usize);
                    if MEM_read32(r#match) == MEM_read32(ip) {
                        currentMl = ZSTD_count_2segments(
                            ip.wrapping_add(4),
                            r#match.wrapping_add(4),
                            iLimit,
                            dmsEnd,
                            prefixStart,
                        ) + 4;
                    }
                }

                if currentMl > ml {
                    ml = currentMl;
                    *offsetPtr = OFFSET_TO_OFFBASE(
                        curr.wrapping_sub(matchIndex.wrapping_add(dmsIndexDelta)),
                    ) as size_t;
                    if ip.wrapping_add(currentMl) == iLimit {
                        break;
                    }
                }
                currMatch += 1;
            }
        }
    }
    ml
}

/* *********************************
* Search dispatch
***********************************/

pub type searchMethod_e = u32;
pub const search_hashChain: searchMethod_e = 0;
pub const search_binaryTree: searchMethod_e = 1;
pub const search_rowHash: searchMethod_e = 2;

/**
 * Searches for the longest match at ip. In C this dispatches through
 * switch statements over (searchMethod, dictMode, mls, rowLog) to
 * force-inlined template instantiations that all share identical bodies.
 * The runtime-parameterized calls below are semantically identical.
 */
pub unsafe fn ZSTD_searchMax(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    offsetPtr: *mut size_t,
    mls: U32,
    rowLog: U32,
    searchMethod: searchMethod_e,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    match searchMethod {
        search_hashChain => ZSTD_HcFindBestMatch(ms, ip, iend, offsetPtr, mls, dictMode),
        search_binaryTree => ZSTD_BtFindBestMatch(ms, ip, iend, offsetPtr, mls, dictMode),
        _ /* search_rowHash */ => {
            ZSTD_RowFindBestMatch(ms, ip, iend, offsetPtr, mls, dictMode, rowLog)
        }
    }
}

/* *******************************
*  Common parser - lazy strategy
*********************************/

pub unsafe fn ZSTD_compressBlock_lazy_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    searchMethod: searchMethod_e,
    depth: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = if searchMethod == search_rowHash {
        iend.wrapping_sub(8).wrapping_sub(ZSTD_ROW_HASH_CACHE_SIZE as usize)
    } else {
        iend.wrapping_sub(8)
    };
    let base: *const BYTE = (*ms).window.base;
    let prefixLowestIndex: U32 = (*ms).window.dictLimit;
    let prefixLowest: *const BYTE = base.wrapping_add(prefixLowestIndex as usize);
    let mls: U32 = BOUNDED(4, (*ms).cParams.minMatch, 6);
    let rowLog: U32 = BOUNDED(4, (*ms).cParams.searchLog, 6);

    let mut offset_1: U32 = *rep.wrapping_add(0);
    let mut offset_2: U32 = *rep.wrapping_add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let isDMS: bool = dictMode == ZSTD_dictMatchState;
    let isDDS: bool = dictMode == ZSTD_dedicatedDictSearch;
    let isDxS: bool = isDMS || isDDS;
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictLowestIndex: U32 = if isDxS { (*dms).window.dictLimit } else { 0 };
    let dictBase: *const BYTE = if isDxS { (*dms).window.base } else { null_mut() };
    let dictLowest: *const BYTE = if isDxS {
        dictBase.wrapping_add(dictLowestIndex as usize)
    } else {
        null_mut()
    };
    let dictEnd: *const BYTE = if isDxS { (*dms).window.nextSrc } else { null_mut() };
    let dictIndexDelta: U32 = if isDxS {
        prefixLowestIndex.wrapping_sub(dictEnd.offset_from(dictBase) as U32)
    } else {
        0
    };
    let dictAndPrefixLength: U32 =
        ((ip.offset_from(prefixLowest)) + (dictEnd.offset_from(dictLowest))) as U32;

    ip = ip.wrapping_add((dictAndPrefixLength == 0) as usize);
    if dictMode == ZSTD_noDict {
        let curr: U32 = ip.offset_from(base) as U32;
        let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, curr, (*ms).cParams.windowLog);
        let maxRep: U32 = curr - windowLow;
        if offset_2 > maxRep {
            offsetSaved2 = offset_2;
            offset_2 = 0;
        }
        if offset_1 > maxRep {
            offsetSaved1 = offset_1;
            offset_1 = 0;
        }
    }

    /* Reset the lazy skipping state */
    (*ms).lazySkipping = 0;

    if searchMethod == search_rowHash {
        ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
    }

    /* Match Loop */
    while ip < ilimit {
        let mut matchLength: size_t = 0;
        let mut offBase: size_t = REPCODE1_TO_OFFBASE as size_t;
        let mut start: *const BYTE = ip.wrapping_add(1);

        /* check repCode */
        if isDxS {
            let repIndex: U32 = (ip.offset_from(base) as U32) + 1 - offset_1;
            let repMatch: *const BYTE = if (dictMode == ZSTD_dictMatchState
                || dictMode == ZSTD_dedicatedDictSearch)
                && repIndex < prefixLowestIndex
            {
                dictBase.wrapping_add((repIndex.wrapping_sub(dictIndexDelta)) as usize)
            } else {
                base.wrapping_add(repIndex as usize)
            };
            if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                && (MEM_read32(repMatch) == MEM_read32(ip.wrapping_add(1)))
            {
                let repMatchEnd: *const BYTE =
                    if repIndex < prefixLowestIndex { dictEnd } else { iend };
                matchLength = ZSTD_count_2segments(
                    ip.wrapping_add(1).wrapping_add(4),
                    repMatch.wrapping_add(4),
                    iend,
                    repMatchEnd,
                    prefixLowest,
                ) + 4;
                if depth == 0 {
                    // goto _storeSequence
                    ZSTD_lazy_store_seq(
                        seqStore, iend, &mut anchor, &mut ip, start, matchLength, offBase, ms,
                        searchMethod, base, rowLog, mls, ilimit,
                    );
                    ZSTD_lazy_immediate_repcodes(
                        ms, seqStore, iend, &mut anchor, &mut ip, ilimit, &mut offset_1,
                        &mut offset_2, dictMode, isDxS, dictBase, dictIndexDelta,
                        prefixLowestIndex, dictEnd, prefixLowest,
                    );
                    continue;
                }
            }
        }
        if dictMode == ZSTD_noDict
            && ((offset_1 > 0) as u32
                & (MEM_read32(ip.wrapping_add(1).wrapping_sub(offset_1 as usize))
                    == MEM_read32(ip.wrapping_add(1))) as u32)
                != 0
        {
            matchLength = ZSTD_count(
                ip.wrapping_add(1).wrapping_add(4),
                ip.wrapping_add(1).wrapping_add(4).wrapping_sub(offset_1 as usize),
                iend,
            ) + 4;
            if depth == 0 {
                ZSTD_lazy_store_seq(
                    seqStore, iend, &mut anchor, &mut ip, start, matchLength, offBase, ms,
                    searchMethod, base, rowLog, mls, ilimit,
                );
                ZSTD_lazy_immediate_repcodes(
                    ms, seqStore, iend, &mut anchor, &mut ip, ilimit, &mut offset_1,
                    &mut offset_2, dictMode, isDxS, dictBase, dictIndexDelta,
                    prefixLowestIndex, dictEnd, prefixLowest,
                );
                continue;
            }
        }

        /* first search (depth 0) */
        {
            let mut offbaseFound: size_t = 999999999;
            let ml2: size_t =
                ZSTD_searchMax(ms, ip, iend, &mut offbaseFound, mls, rowLog, searchMethod, dictMode);
            if ml2 > matchLength {
                matchLength = ml2;
                start = ip;
                offBase = offbaseFound;
            }
        }

        if matchLength < 4 {
            let step: size_t = ((ip.offset_from(anchor) as size_t) >> kSearchStrength) + 1;
            ip = ip.wrapping_add(step);
            (*ms).lazySkipping = (step > kLazySkippingStep as size_t) as i32;
            continue;
        }

        /* let's try to find a better solution */
        if depth >= 1 {
            while ip < ilimit {
                ip = ip.wrapping_add(1);
                if (dictMode == ZSTD_noDict)
                    && (offBase != 0)
                    && (((offset_1 > 0) as u32
                        & (MEM_read32(ip) == MEM_read32(ip.wrapping_sub(offset_1 as usize))) as u32)
                        != 0)
                {
                    let mlRep: size_t = ZSTD_count(
                        ip.wrapping_add(4),
                        ip.wrapping_add(4).wrapping_sub(offset_1 as usize),
                        iend,
                    ) + 4;
                    let gain2: i32 = (mlRep * 3) as i32;
                    let gain1: i32 =
                        (matchLength * 3) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
                    if (mlRep >= 4) && (gain2 > gain1) {
                        matchLength = mlRep;
                        offBase = REPCODE1_TO_OFFBASE as size_t;
                        start = ip;
                    }
                }
                if isDxS {
                    let repIndex: U32 = (ip.offset_from(base) as U32) - offset_1;
                    let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                        dictBase.wrapping_add((repIndex.wrapping_sub(dictIndexDelta)) as usize)
                    } else {
                        base.wrapping_add(repIndex as usize)
                    };
                    if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                        && (MEM_read32(repMatch) == MEM_read32(ip))
                    {
                        let repMatchEnd: *const BYTE =
                            if repIndex < prefixLowestIndex { dictEnd } else { iend };
                        let mlRep: size_t = ZSTD_count_2segments(
                            ip.wrapping_add(4),
                            repMatch.wrapping_add(4),
                            iend,
                            repMatchEnd,
                            prefixLowest,
                        ) + 4;
                        let gain2: i32 = (mlRep * 3) as i32;
                        let gain1: i32 =
                            (matchLength * 3) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
                        if (mlRep >= 4) && (gain2 > gain1) {
                            matchLength = mlRep;
                            offBase = REPCODE1_TO_OFFBASE as size_t;
                            start = ip;
                        }
                    }
                }
                {
                    let mut ofbCandidate: size_t = 999999999;
                    let ml2: size_t = ZSTD_searchMax(
                        ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, dictMode,
                    );
                    let gain2: i32 = (ml2 * 4) as i32 - ZSTD_highbit32(ofbCandidate as U32) as i32;
                    let gain1: i32 =
                        (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 4;
                    if (ml2 >= 4) && (gain2 > gain1) {
                        matchLength = ml2;
                        offBase = ofbCandidate;
                        start = ip;
                        continue; /* search a better one */
                    }
                }

                /* let's find an even better one */
                if (depth == 2) && (ip < ilimit) {
                    ip = ip.wrapping_add(1);
                    if (dictMode == ZSTD_noDict)
                        && (offBase != 0)
                        && (((offset_1 > 0) as u32
                            & (MEM_read32(ip)
                                == MEM_read32(ip.wrapping_sub(offset_1 as usize)))
                                as u32)
                            != 0)
                    {
                        let mlRep: size_t = ZSTD_count(
                            ip.wrapping_add(4),
                            ip.wrapping_add(4).wrapping_sub(offset_1 as usize),
                            iend,
                        ) + 4;
                        let gain2: i32 = (mlRep * 4) as i32;
                        let gain1: i32 =
                            (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
                        if (mlRep >= 4) && (gain2 > gain1) {
                            matchLength = mlRep;
                            offBase = REPCODE1_TO_OFFBASE as size_t;
                            start = ip;
                        }
                    }
                    if isDxS {
                        let repIndex: U32 = (ip.offset_from(base) as U32) - offset_1;
                        let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                            dictBase.wrapping_add((repIndex.wrapping_sub(dictIndexDelta)) as usize)
                        } else {
                            base.wrapping_add(repIndex as usize)
                        };
                        if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                            && (MEM_read32(repMatch) == MEM_read32(ip))
                        {
                            let repMatchEnd: *const BYTE =
                                if repIndex < prefixLowestIndex { dictEnd } else { iend };
                            let mlRep: size_t = ZSTD_count_2segments(
                                ip.wrapping_add(4),
                                repMatch.wrapping_add(4),
                                iend,
                                repMatchEnd,
                                prefixLowest,
                            ) + 4;
                            let gain2: i32 = (mlRep * 4) as i32;
                            let gain1: i32 = (matchLength * 4) as i32
                                - ZSTD_highbit32(offBase as U32) as i32
                                + 1;
                            if (mlRep >= 4) && (gain2 > gain1) {
                                matchLength = mlRep;
                                offBase = REPCODE1_TO_OFFBASE as size_t;
                                start = ip;
                            }
                        }
                    }
                    {
                        let mut ofbCandidate: size_t = 999999999;
                        let ml2: size_t = ZSTD_searchMax(
                            ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, dictMode,
                        );
                        let gain2: i32 =
                            (ml2 * 4) as i32 - ZSTD_highbit32(ofbCandidate as U32) as i32;
                        let gain1: i32 =
                            (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 7;
                        if (ml2 >= 4) && (gain2 > gain1) {
                            matchLength = ml2;
                            offBase = ofbCandidate;
                            start = ip;
                            continue;
                        }
                    }
                }
                break; /* nothing found : store previous solution */
            }
        }

        /* catch up */
        if OFFBASE_IS_OFFSET(offBase as U32) {
            if dictMode == ZSTD_noDict {
                while (((start > anchor) as u32
                    & (start.wrapping_sub(OFFBASE_TO_OFFSET(offBase as U32) as usize)
                        > prefixLowest) as u32)
                    != 0)
                    && (*start.wrapping_sub(1)
                        == *start
                            .wrapping_sub(OFFBASE_TO_OFFSET(offBase as U32) as usize)
                            .wrapping_sub(1))
                {
                    start = start.wrapping_sub(1);
                    matchLength += 1;
                }
            }
            if isDxS {
                let matchIndex: U32 = ((start.offset_from(base) as size_t)
                    - OFFBASE_TO_OFFSET(offBase as U32) as size_t)
                    as U32;
                let mut r#match: *const BYTE = if matchIndex < prefixLowestIndex {
                    dictBase
                        .wrapping_add(matchIndex as usize)
                        .wrapping_sub(dictIndexDelta as usize)
                } else {
                    base.wrapping_add(matchIndex as usize)
                };
                let mStart: *const BYTE = if matchIndex < prefixLowestIndex {
                    dictLowest
                } else {
                    prefixLowest
                };
                while (start > anchor)
                    && (r#match > mStart)
                    && (*start.wrapping_sub(1) == *r#match.wrapping_sub(1))
                {
                    start = start.wrapping_sub(1);
                    r#match = r#match.wrapping_sub(1);
                    matchLength += 1;
                }
            }
            offset_2 = offset_1;
            offset_1 = OFFBASE_TO_OFFSET(offBase as U32);
        }
        /* store sequence */
        ZSTD_lazy_store_seq(
            seqStore, iend, &mut anchor, &mut ip, start, matchLength, offBase, ms, searchMethod,
            base, rowLog, mls, ilimit,
        );
        ZSTD_lazy_immediate_repcodes(
            ms, seqStore, iend, &mut anchor, &mut ip, ilimit, &mut offset_1, &mut offset_2,
            dictMode, isDxS, dictBase, dictIndexDelta, prefixLowestIndex, dictEnd, prefixLowest,
        );
    }

    /* If offset_1 started invalid and became valid, rotate saved offsets. */
    offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
        offsetSaved1
    } else {
        offsetSaved2
    };

    /* save reps for next block */
    *rep.wrapping_add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
    *rep.wrapping_add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };

    /* Return the last literals size */
    (iend.offset_from(anchor)) as size_t
}

/* Helper for the `_storeSequence` label of ZSTD_compressBlock_lazy_generic.
 * Stores the sequence, advances anchor/ip, and handles lazySkipping reset. */
#[inline(always)]
unsafe fn ZSTD_lazy_store_seq(
    seqStore: *mut SeqStore_t,
    iend: *const BYTE,
    anchor: &mut *const BYTE,
    ip: &mut *const BYTE,
    start: *const BYTE,
    matchLength: size_t,
    offBase: size_t,
    ms: *mut ZSTD_MatchState_t,
    searchMethod: searchMethod_e,
    base: *const BYTE,
    rowLog: U32,
    mls: U32,
    ilimit: *const BYTE,
) {
    {
        let litLength: size_t = start.offset_from(*anchor) as size_t;
        ZSTD_storeSeq(seqStore, litLength, *anchor, iend, offBase as U32, matchLength);
        *ip = start.wrapping_add(matchLength);
        *anchor = *ip;
    }
    if (*ms).lazySkipping != 0 {
        /* We've found a match, disable lazy skipping mode, and refill the hash cache. */
        if searchMethod == search_rowHash {
            ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
        }
        (*ms).lazySkipping = 0;
    }
}

/* Helper for the immediate-repcode checks of ZSTD_compressBlock_lazy_generic. */
#[inline(always)]
unsafe fn ZSTD_lazy_immediate_repcodes(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    iend: *const BYTE,
    anchor: &mut *const BYTE,
    ip: &mut *const BYTE,
    ilimit: *const BYTE,
    offset_1: &mut U32,
    offset_2: &mut U32,
    dictMode: ZSTD_dictMode_e,
    isDxS: bool,
    dictBase: *const BYTE,
    dictIndexDelta: U32,
    prefixLowestIndex: U32,
    dictEnd: *const BYTE,
    prefixLowest: *const BYTE,
) {
    let _ = ms;
    if isDxS {
        while *ip <= ilimit {
            let current2: U32 = (*ip).offset_from((*ms).window.base) as U32;
            let repIndex: U32 = current2 - *offset_2;
            let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                dictBase
                    .wrapping_sub(dictIndexDelta as usize)
                    .wrapping_add(repIndex as usize)
            } else {
                (*ms).window.base.wrapping_add(repIndex as usize)
            };
            if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                && (MEM_read32(repMatch) == MEM_read32(*ip))
            {
                let repEnd2: *const BYTE =
                    if repIndex < prefixLowestIndex { dictEnd } else { iend };
                let matchLength: size_t = ZSTD_count_2segments(
                    (*ip).wrapping_add(4),
                    repMatch.wrapping_add(4),
                    iend,
                    repEnd2,
                    prefixLowest,
                ) + 4;
                let offBase: U32 = *offset_2;
                *offset_2 = *offset_1;
                *offset_1 = offBase; /* swap offset_2 <=> offset_1 */
                ZSTD_storeSeq(seqStore, 0, *anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
                *ip = (*ip).wrapping_add(matchLength);
                *anchor = *ip;
                continue;
            }
            break;
        }
    }

    if dictMode == ZSTD_noDict {
        while (((*ip <= ilimit) as u32) & ((*offset_2 > 0) as u32)) != 0
            && (MEM_read32(*ip) == MEM_read32((*ip).wrapping_sub(*offset_2 as usize)))
        {
            /* store sequence */
            let matchLength: size_t = ZSTD_count(
                (*ip).wrapping_add(4),
                (*ip).wrapping_add(4).wrapping_sub(*offset_2 as usize),
                iend,
            ) + 4;
            let offBase: U32 = *offset_2;
            *offset_2 = *offset_1;
            *offset_1 = offBase; /* swap repcodes */
            ZSTD_storeSeq(seqStore, 0, *anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
            *ip = (*ip).wrapping_add(matchLength);
            *anchor = *ip;
            continue;
        }
    }
}

pub unsafe fn ZSTD_compressBlock_lazy_extDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    searchMethod: searchMethod_e,
    depth: U32,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = if searchMethod == search_rowHash {
        iend.wrapping_sub(8).wrapping_sub(ZSTD_ROW_HASH_CACHE_SIZE as usize)
    } else {
        iend.wrapping_sub(8)
    };
    let base: *const BYTE = (*ms).window.base;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
    let dictStart: *const BYTE = dictBase.wrapping_add((*ms).window.lowLimit as usize);
    let windowLog: U32 = (*ms).cParams.windowLog;
    let mls: U32 = BOUNDED(4, (*ms).cParams.minMatch, 6);
    let rowLog: U32 = BOUNDED(4, (*ms).cParams.searchLog, 6);

    let mut offset_1: U32 = *rep.wrapping_add(0);
    let mut offset_2: U32 = *rep.wrapping_add(1);

    /* Reset the lazy skipping state */
    (*ms).lazySkipping = 0;

    /* init */
    ip = ip.wrapping_add((ip == prefixStart) as usize);
    if searchMethod == search_rowHash {
        ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
    }

    /* Match Loop */
    while ip < ilimit {
        let mut matchLength: size_t = 0;
        let mut offBase: size_t = REPCODE1_TO_OFFBASE as size_t;
        let mut start: *const BYTE = ip.wrapping_add(1);
        let mut curr: U32 = ip.offset_from(base) as U32;

        /* check repCode */
        {
            let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr + 1, windowLog);
            let repIndex: U32 = curr + 1 - offset_1;
            let repBase: *const BYTE = if repIndex < dictLimit { dictBase } else { base };
            let repMatch: *const BYTE = repBase.wrapping_add(repIndex as usize);
            if ((ZSTD_index_overlap_check(dictLimit, repIndex) as u32)
                & ((offset_1 <= curr + 1 - windowLow) as u32))
                != 0
            {
                if MEM_read32(ip.wrapping_add(1)) == MEM_read32(repMatch) {
                    /* repcode detected we should take it */
                    let repEnd: *const BYTE = if repIndex < dictLimit { dictEnd } else { iend };
                    matchLength = ZSTD_count_2segments(
                        ip.wrapping_add(1).wrapping_add(4),
                        repMatch.wrapping_add(4),
                        iend,
                        repEnd,
                        prefixStart,
                    ) + 4;
                    if depth == 0 {
                        ZSTD_lazy_ext_store_seq(
                            seqStore, iend, &mut anchor, &mut ip, start, matchLength, offBase, ms,
                            searchMethod, base, rowLog, mls, ilimit,
                        );
                        ZSTD_lazy_ext_immediate_repcodes(
                            ms, seqStore, iend, &mut anchor, &mut ip, ilimit, &mut offset_1,
                            &mut offset_2, base, dictBase, dictLimit, dictEnd, prefixStart,
                            windowLog,
                        );
                        continue;
                    }
                }
            }
        }

        /* first search (depth 0) */
        {
            let mut ofbCandidate: size_t = 999999999;
            let ml2: size_t = ZSTD_searchMax(
                ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, ZSTD_extDict,
            );
            if ml2 > matchLength {
                matchLength = ml2;
                start = ip;
                offBase = ofbCandidate;
            }
        }

        if matchLength < 4 {
            let step: size_t = (ip.offset_from(anchor) as size_t) >> kSearchStrength;
            ip = ip.wrapping_add(step + 1); /* jump faster over incompressible sections */
            (*ms).lazySkipping = (step > kLazySkippingStep as size_t) as i32;
            continue;
        }

        /* let's try to find a better solution */
        if depth >= 1 {
            while ip < ilimit {
                ip = ip.wrapping_add(1);
                curr += 1;
                /* check repCode */
                if offBase != 0 {
                    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, windowLog);
                    let repIndex: U32 = curr - offset_1;
                    let repBase: *const BYTE = if repIndex < dictLimit { dictBase } else { base };
                    let repMatch: *const BYTE = repBase.wrapping_add(repIndex as usize);
                    if ((ZSTD_index_overlap_check(dictLimit, repIndex) as u32)
                        & ((offset_1 <= curr - windowLow) as u32))
                        != 0
                    {
                        if MEM_read32(ip) == MEM_read32(repMatch) {
                            /* repcode detected */
                            let repEnd: *const BYTE =
                                if repIndex < dictLimit { dictEnd } else { iend };
                            let repLength: size_t = ZSTD_count_2segments(
                                ip.wrapping_add(4),
                                repMatch.wrapping_add(4),
                                iend,
                                repEnd,
                                prefixStart,
                            ) + 4;
                            let gain2: i32 = (repLength * 3) as i32;
                            let gain1: i32 = (matchLength * 3) as i32
                                - ZSTD_highbit32(offBase as U32) as i32
                                + 1;
                            if (repLength >= 4) && (gain2 > gain1) {
                                matchLength = repLength;
                                offBase = REPCODE1_TO_OFFBASE as size_t;
                                start = ip;
                            }
                        }
                    }
                }

                /* search match, depth 1 */
                {
                    let mut ofbCandidate: size_t = 999999999;
                    let ml2: size_t = ZSTD_searchMax(
                        ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, ZSTD_extDict,
                    );
                    let gain2: i32 = (ml2 * 4) as i32 - ZSTD_highbit32(ofbCandidate as U32) as i32;
                    let gain1: i32 =
                        (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 4;
                    if (ml2 >= 4) && (gain2 > gain1) {
                        matchLength = ml2;
                        offBase = ofbCandidate;
                        start = ip;
                        continue; /* search a better one */
                    }
                }

                /* let's find an even better one */
                if (depth == 2) && (ip < ilimit) {
                    ip = ip.wrapping_add(1);
                    curr += 1;
                    /* check repCode */
                    if offBase != 0 {
                        let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, windowLog);
                        let repIndex: U32 = curr - offset_1;
                        let repBase: *const BYTE =
                            if repIndex < dictLimit { dictBase } else { base };
                        let repMatch: *const BYTE = repBase.wrapping_add(repIndex as usize);
                        if ((ZSTD_index_overlap_check(dictLimit, repIndex) as u32)
                            & ((offset_1 <= curr - windowLow) as u32))
                            != 0
                        {
                            if MEM_read32(ip) == MEM_read32(repMatch) {
                                /* repcode detected */
                                let repEnd: *const BYTE =
                                    if repIndex < dictLimit { dictEnd } else { iend };
                                let repLength: size_t = ZSTD_count_2segments(
                                    ip.wrapping_add(4),
                                    repMatch.wrapping_add(4),
                                    iend,
                                    repEnd,
                                    prefixStart,
                                ) + 4;
                                let gain2: i32 = (repLength * 4) as i32;
                                let gain1: i32 = (matchLength * 4) as i32
                                    - ZSTD_highbit32(offBase as U32) as i32
                                    + 1;
                                if (repLength >= 4) && (gain2 > gain1) {
                                    matchLength = repLength;
                                    offBase = REPCODE1_TO_OFFBASE as size_t;
                                    start = ip;
                                }
                            }
                        }
                    }

                    /* search match, depth 2 */
                    {
                        let mut ofbCandidate: size_t = 999999999;
                        let ml2: size_t = ZSTD_searchMax(
                            ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod,
                            ZSTD_extDict,
                        );
                        let gain2: i32 =
                            (ml2 * 4) as i32 - ZSTD_highbit32(ofbCandidate as U32) as i32;
                        let gain1: i32 =
                            (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 7;
                        if (ml2 >= 4) && (gain2 > gain1) {
                            matchLength = ml2;
                            offBase = ofbCandidate;
                            start = ip;
                            continue;
                        }
                    }
                }
                break; /* nothing found : store previous solution */
            }
        }

        /* catch up */
        if OFFBASE_IS_OFFSET(offBase as U32) {
            let matchIndex: U32 = ((start.offset_from(base) as size_t)
                - OFFBASE_TO_OFFSET(offBase as U32) as size_t) as U32;
            let mut r#match: *const BYTE = if matchIndex < dictLimit {
                dictBase.wrapping_add(matchIndex as usize)
            } else {
                base.wrapping_add(matchIndex as usize)
            };
            let mStart: *const BYTE = if matchIndex < dictLimit { dictStart } else { prefixStart };
            while (start > anchor)
                && (r#match > mStart)
                && (*start.wrapping_sub(1) == *r#match.wrapping_sub(1))
            {
                start = start.wrapping_sub(1);
                r#match = r#match.wrapping_sub(1);
                matchLength += 1;
            }
            offset_2 = offset_1;
            offset_1 = OFFBASE_TO_OFFSET(offBase as U32);
        }

        /* store sequence */
        ZSTD_lazy_ext_store_seq(
            seqStore, iend, &mut anchor, &mut ip, start, matchLength, offBase, ms, searchMethod,
            base, rowLog, mls, ilimit,
        );
        ZSTD_lazy_ext_immediate_repcodes(
            ms, seqStore, iend, &mut anchor, &mut ip, ilimit, &mut offset_1, &mut offset_2, base,
            dictBase, dictLimit, dictEnd, prefixStart, windowLog,
        );
    }

    /* Save reps for next block */
    *rep.wrapping_add(0) = offset_1;
    *rep.wrapping_add(1) = offset_2;

    /* Return the last literals size */
    (iend.offset_from(anchor)) as size_t
}

/* Helper for the `_storeSequence` label of ZSTD_compressBlock_lazy_extDict_generic. */
#[inline(always)]
unsafe fn ZSTD_lazy_ext_store_seq(
    seqStore: *mut SeqStore_t,
    iend: *const BYTE,
    anchor: &mut *const BYTE,
    ip: &mut *const BYTE,
    start: *const BYTE,
    matchLength: size_t,
    offBase: size_t,
    ms: *mut ZSTD_MatchState_t,
    searchMethod: searchMethod_e,
    base: *const BYTE,
    rowLog: U32,
    mls: U32,
    ilimit: *const BYTE,
) {
    {
        let litLength: size_t = start.offset_from(*anchor) as size_t;
        ZSTD_storeSeq(seqStore, litLength, *anchor, iend, offBase as U32, matchLength);
        *ip = start.wrapping_add(matchLength);
        *anchor = *ip;
    }
    if (*ms).lazySkipping != 0 {
        if searchMethod == search_rowHash {
            ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
        }
        (*ms).lazySkipping = 0;
    }
}

/* Helper for the immediate-repcode checks of ZSTD_compressBlock_lazy_extDict_generic. */
#[inline(always)]
unsafe fn ZSTD_lazy_ext_immediate_repcodes(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    iend: *const BYTE,
    anchor: &mut *const BYTE,
    ip: &mut *const BYTE,
    ilimit: *const BYTE,
    offset_1: &mut U32,
    offset_2: &mut U32,
    base: *const BYTE,
    dictBase: *const BYTE,
    dictLimit: U32,
    dictEnd: *const BYTE,
    prefixStart: *const BYTE,
    windowLog: U32,
) {
    while *ip <= ilimit {
        let repCurrent: U32 = (*ip).offset_from(base) as U32;
        let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, repCurrent, windowLog);
        let repIndex: U32 = repCurrent - *offset_2;
        let repBase: *const BYTE = if repIndex < dictLimit { dictBase } else { base };
        let repMatch: *const BYTE = repBase.wrapping_add(repIndex as usize);
        if ((ZSTD_index_overlap_check(dictLimit, repIndex) as u32)
            & ((*offset_2 <= repCurrent - windowLow) as u32))
            != 0
        {
            if MEM_read32(*ip) == MEM_read32(repMatch) {
                /* repcode detected we should take it */
                let repEnd: *const BYTE = if repIndex < dictLimit { dictEnd } else { iend };
                let matchLength: size_t = ZSTD_count_2segments(
                    (*ip).wrapping_add(4),
                    repMatch.wrapping_add(4),
                    iend,
                    repEnd,
                    prefixStart,
                ) + 4;
                let offBase: U32 = *offset_2;
                *offset_2 = *offset_1;
                *offset_1 = offBase; /* swap offset history */
                ZSTD_storeSeq(seqStore, 0, *anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
                *ip = (*ip).wrapping_add(matchLength);
                *anchor = *ip;
                continue;
            }
        }
        break;
    }
}

/* *******************************
*  Public block compressors
*********************************/

/* --- greedy (depth 0) --- */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 0, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_hashChain, 0, ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_hashChain, 0, ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 0, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_rowHash, 0, ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_rowHash, 0, ZSTD_dedicatedDictSearch,
    )
}

/* --- lazy (depth 1) --- */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 1, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_hashChain, 1, ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_hashChain, 1, ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 1, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_rowHash, 1, ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_rowHash, 1, ZSTD_dedicatedDictSearch,
    )
}

/* --- lazy2 (depth 2) --- */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 2, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_hashChain, 2, ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_hashChain, 2, ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 2, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_rowHash, 2, ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_rowHash, 2, ZSTD_dedicatedDictSearch,
    )
}

/* --- btlazy2 (depth 2, binary tree) --- */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_binaryTree, 2, ZSTD_noDict,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_generic(
        ms, seqStore, rep, src, srcSize, search_binaryTree, 2, ZSTD_dictMatchState,
    )
}

/* --- extDict variants --- */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_extDict_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_extDict_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_binaryTree, 2)
}
