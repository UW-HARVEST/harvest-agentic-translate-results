//! Translation of `compress/zstd_lazy.c` (and `compress/zstd_lazy.h`)
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8,
};

use crate::bits::*;
use crate::cmem::*;
use crate::compress::zstd_compress_internal::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* ===== zstd_lazy.h ===== */

/// Dedicated Dictionary Search Structure bucket log.
pub const ZSTD_LAZY_DDSS_BUCKET_LOG: U32 = 2;
/// nb bits to use for the tag
pub const ZSTD_ROW_HASH_TAG_BITS: U32 = 8;

const kLazySkippingStep: usize = 8;

/* ===== small mechanical helpers =====
 * The C is compiled with ZSTD_ALLOW_POINTER_OVERFLOW_ATTR, and routinely forms
 * pointers that are outside of any real object (e.g. `base + idx` where `base`
 * is a virtual base). These helpers reproduce the raw address arithmetic. */

#[inline(always)]
fn pdiff(a: *const BYTE, b: *const BYTE) -> usize {
    (a as usize).wrapping_sub(b as usize)
}

#[inline(always)]
fn padd(p: *const BYTE, n: usize) -> *const BYTE {
    (p as usize).wrapping_add(n) as *const BYTE
}

#[inline(always)]
fn psub(p: *const BYTE, n: usize) -> *const BYTE {
    (p as usize).wrapping_sub(n) as *const BYTE
}

#[inline(always)]
unsafe fn rd32(p: *const BYTE) -> U32 {
    MEM_read32(p as *const c_void)
}

#[inline(always)]
fn bounded_u32(min: U32, val: U32, max: U32) -> U32 {
    let m = if val < max { val } else { max };
    if min > m {
        min
    } else {
        m
    }
}

/*-*************************************
*  Binary Tree search
***************************************/

unsafe fn ZSTD_updateDUBT(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    _iend: *const BYTE,
    mls: U32,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;

    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog.wrapping_sub(1);
    let btMask: U32 = (1u32 << btLog).wrapping_sub(1);

    let base: *const BYTE = (*ms).window.base;
    let target: U32 = pdiff(ip, base) as U32;
    let mut idx: U32 = (*ms).nextToUpdate;

    while idx < target {
        let h: usize = ZSTD_hashPtr(padd(base, idx as usize) as *const c_void, hashLog, mls);
        let matchIndex: U32 = *hashTable.add(h);

        let nextCandidatePtr: *mut U32 = bt.add(2 * ((idx & btMask) as usize));
        let sortMarkPtr: *mut U32 = nextCandidatePtr.add(1);

        *hashTable.add(h) = idx; /* Update Hash Table */
        *nextCandidatePtr = matchIndex; /* update BT like a chain */
        *sortMarkPtr = ZSTD_DUBT_UNSORTED_MARK;
        idx = idx.wrapping_add(1);
    }
    (*ms).nextToUpdate = target;
}

/** ZSTD_insertDUBT1() :
 *  sort one already inserted but unsorted position */
unsafe fn ZSTD_insertDUBT1(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    inputEnd: *const BYTE,
    mut nbCompares: U32,
    btLow: U32,
    dictMode: ZSTD_dictMode_e,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog.wrapping_sub(1);
    let btMask: U32 = (1u32 << btLog).wrapping_sub(1);
    let mut commonLengthSmaller: usize = 0;
    let mut commonLengthLarger: usize = 0;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let ip: *const BYTE = if curr >= dictLimit {
        padd(base, curr as usize)
    } else {
        padd(dictBase, curr as usize)
    };
    let iend: *const BYTE = if curr >= dictLimit {
        inputEnd
    } else {
        padd(dictBase, dictLimit as usize)
    };
    let dictEnd: *const BYTE = padd(dictBase, dictLimit as usize);
    let prefixStart: *const BYTE = padd(base, dictLimit as usize);
    let mut match_: *const BYTE;
    let mut smallerPtr: *mut U32 = bt.add(2 * ((curr & btMask) as usize));
    let mut largerPtr: *mut U32 = smallerPtr.add(1);
    let mut matchIndex: U32 = *smallerPtr;
    let mut dummy32: U32 = 0;
    let windowValid: U32 = (*ms).window.lowLimit;
    let maxDistance: U32 = 1u32 << (*cParams).windowLog;
    let windowLow: U32 = if curr.wrapping_sub(windowValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        windowValid
    };

    while nbCompares != 0 && matchIndex > windowLow {
        let nextPtr: *mut U32 = bt.add(2 * ((matchIndex & btMask) as usize));
        let mut matchLength: usize = if commonLengthSmaller < commonLengthLarger {
            commonLengthSmaller
        } else {
            commonLengthLarger
        };

        if (dictMode != ZSTD_extDict)
            || ((matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize)
            || (curr < dictLimit)
        {
            let mBase: *const BYTE = if (dictMode != ZSTD_extDict)
                || ((matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize)
            {
                base
            } else {
                dictBase
            };
            match_ = padd(mBase, matchIndex as usize);
            matchLength += ZSTD_count(padd(ip, matchLength), padd(match_, matchLength), iend);
        } else {
            match_ = padd(dictBase, matchIndex as usize);
            matchLength += ZSTD_count_2segments(
                padd(ip, matchLength),
                padd(match_, matchLength),
                iend,
                dictEnd,
                prefixStart,
            );
            if (matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize {
                match_ = padd(base, matchIndex as usize);
            }
        }

        if padd(ip, matchLength) == iend {
            /* equal : no way to know if inf or sup */
            break;
        }

        if *padd(match_, matchLength) < *padd(ip, matchLength) {
            /* match is smaller than current */
            *smallerPtr = matchIndex;
            commonLengthSmaller = matchLength;
            if matchIndex <= btLow {
                smallerPtr = &mut dummy32 as *mut U32;
                break;
            }
            smallerPtr = nextPtr.add(1);
            matchIndex = *nextPtr.add(1);
        } else {
            /* match is larger than current */
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &mut dummy32 as *mut U32;
                break;
            }
            largerPtr = nextPtr;
            matchIndex = *nextPtr;
        }
        nbCompares = nbCompares.wrapping_sub(1);
    }

    *largerPtr = 0;
    *smallerPtr = 0;
}

unsafe fn ZSTD_DUBT_findBetterDictMatch(
    ms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    offsetPtr: *mut usize,
    mut bestLength: usize,
    mut nbCompares: U32,
    mls: U32,
    _dictMode: ZSTD_dictMode_e,
) -> usize {
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dmsCParams: *const ZSTD_compressionParameters = &(*dms).cParams;
    let dictHashTable: *const U32 = (*dms).hashTable;
    let hashLog: U32 = (*dmsCParams).hashLog;
    let h: usize = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut dictMatchIndex: U32 = *dictHashTable.add(h);

    let base: *const BYTE = (*ms).window.base;
    let prefixStart: *const BYTE = padd(base, (*ms).window.dictLimit as usize);
    let curr: U32 = pdiff(ip, base) as U32;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictHighLimit: U32 = pdiff((*dms).window.nextSrc, (*dms).window.base) as U32;
    let dictLowLimit: U32 = (*dms).window.lowLimit;
    let dictIndexDelta: U32 = (*ms).window.lowLimit.wrapping_sub(dictHighLimit);

    let dictBt: *mut U32 = (*dms).chainTable;
    let btLog: U32 = (*dmsCParams).chainLog.wrapping_sub(1);
    let btMask: U32 = (1u32 << btLog).wrapping_sub(1);
    let btLow: U32 = if btMask >= dictHighLimit.wrapping_sub(dictLowLimit) {
        dictLowLimit
    } else {
        dictHighLimit.wrapping_sub(btMask)
    };

    let mut commonLengthSmaller: usize = 0;
    let mut commonLengthLarger: usize = 0;

    while nbCompares != 0 && dictMatchIndex > dictLowLimit {
        let nextPtr: *const U32 = dictBt.add(2 * ((dictMatchIndex & btMask) as usize));
        let mut matchLength: usize = if commonLengthSmaller < commonLengthLarger {
            commonLengthSmaller
        } else {
            commonLengthLarger
        };
        let mut match_: *const BYTE = padd(dictBase, dictMatchIndex as usize);
        matchLength += ZSTD_count_2segments(
            padd(ip, matchLength),
            padd(match_, matchLength),
            iend,
            dictEnd,
            prefixStart,
        );
        if (dictMatchIndex as usize).wrapping_add(matchLength) >= dictHighLimit as usize {
            /* to prepare for next usage of match[matchLength] */
            match_ = padd(padd(base, dictMatchIndex as usize), dictIndexDelta as usize);
        }

        if matchLength > bestLength {
            let matchIndex: U32 = dictMatchIndex.wrapping_add(dictIndexDelta);
            if (4i32).wrapping_mul(matchLength.wrapping_sub(bestLength) as c_int)
                > (ZSTD_highbit32(curr.wrapping_sub(matchIndex).wrapping_add(1))
                    .wrapping_sub(ZSTD_highbit32((*offsetPtr as U32).wrapping_add(1))))
                    as c_int
            {
                bestLength = matchLength;
                *offsetPtr = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex)) as usize;
            }
            if padd(ip, matchLength) == iend {
                /* reached end of input */
                break;
            }
        }

        if *padd(match_, matchLength) < *padd(ip, matchLength) {
            if dictMatchIndex <= btLow {
                break;
            }
            commonLengthSmaller = matchLength;
            dictMatchIndex = *nextPtr.add(1);
        } else {
            /* match is larger than current */
            if dictMatchIndex <= btLow {
                break;
            }
            commonLengthLarger = matchLength;
            dictMatchIndex = *nextPtr;
        }
        nbCompares = nbCompares.wrapping_sub(1);
    }

    bestLength
}

unsafe fn ZSTD_DUBT_findBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    offBasePtr: *mut usize,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;
    let h: usize = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut matchIndex: U32 = *hashTable.add(h);

    let base: *const BYTE = (*ms).window.base;
    let curr: U32 = pdiff(ip, base) as U32;
    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, (*cParams).windowLog);

    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog.wrapping_sub(1);
    let btMask: U32 = (1u32 << btLog).wrapping_sub(1);
    let btLow: U32 = if btMask >= curr {
        0
    } else {
        curr.wrapping_sub(btMask)
    };
    let unsortLimit: U32 = if btLow > windowLow { btLow } else { windowLow };

    let mut nextCandidate: *mut U32 = bt.add(2 * ((matchIndex & btMask) as usize));
    let mut unsortedMark: *mut U32 = bt.add(2 * ((matchIndex & btMask) as usize) + 1);
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
        nextCandidate = bt.add(2 * ((matchIndex & btMask) as usize));
        unsortedMark = bt.add(2 * ((matchIndex & btMask) as usize) + 1);
        nbCandidates = nbCandidates.wrapping_sub(1);
    }

    /* nullify last candidate if it's still unsorted */
    if (matchIndex > unsortLimit) && (*unsortedMark == ZSTD_DUBT_UNSORTED_MARK) {
        *unsortedMark = 0;
        *nextCandidate = 0;
    }

    /* batch sort stacked candidates */
    matchIndex = previousCandidate;
    while matchIndex != 0 {
        let nextCandidateIdxPtr: *mut U32 = bt.add(2 * ((matchIndex & btMask) as usize) + 1);
        let nextCandidateIdx: U32 = *nextCandidateIdxPtr;
        ZSTD_insertDUBT1(ms, matchIndex, iend, nbCandidates, unsortLimit, dictMode);
        matchIndex = nextCandidateIdx;
        nbCandidates = nbCandidates.wrapping_add(1);
    }

    /* find longest match */
    {
        let mut commonLengthSmaller: usize = 0;
        let mut commonLengthLarger: usize = 0;
        let dictBase: *const BYTE = (*ms).window.dictBase;
        let dictLimit: U32 = (*ms).window.dictLimit;
        let dictEnd: *const BYTE = padd(dictBase, dictLimit as usize);
        let prefixStart: *const BYTE = padd(base, dictLimit as usize);
        let mut smallerPtr: *mut U32 = bt.add(2 * ((curr & btMask) as usize));
        let mut largerPtr: *mut U32 = bt.add(2 * ((curr & btMask) as usize) + 1);
        let mut matchEndIdx: U32 = curr.wrapping_add(8).wrapping_add(1);
        let mut dummy32: U32 = 0;
        let mut bestLength: usize = 0;

        matchIndex = *hashTable.add(h);
        *hashTable.add(h) = curr; /* Update Hash Table */

        while nbCompares != 0 && matchIndex > windowLow {
            let nextPtr: *mut U32 = bt.add(2 * ((matchIndex & btMask) as usize));
            let mut matchLength: usize = if commonLengthSmaller < commonLengthLarger {
                commonLengthSmaller
            } else {
                commonLengthLarger
            };
            let mut match_: *const BYTE;

            if (dictMode != ZSTD_extDict)
                || ((matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize)
            {
                match_ = padd(base, matchIndex as usize);
                matchLength += ZSTD_count(padd(ip, matchLength), padd(match_, matchLength), iend);
            } else {
                match_ = padd(dictBase, matchIndex as usize);
                matchLength += ZSTD_count_2segments(
                    padd(ip, matchLength),
                    padd(match_, matchLength),
                    iend,
                    dictEnd,
                    prefixStart,
                );
                if (matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize {
                    match_ = padd(base, matchIndex as usize);
                }
            }

            if matchLength > bestLength {
                if matchLength > matchEndIdx.wrapping_sub(matchIndex) as usize {
                    matchEndIdx = matchIndex.wrapping_add(matchLength as U32);
                }
                if (4i32).wrapping_mul(matchLength.wrapping_sub(bestLength) as c_int)
                    > (ZSTD_highbit32(curr.wrapping_sub(matchIndex).wrapping_add(1))
                        .wrapping_sub(ZSTD_highbit32(*offBasePtr as U32))) as c_int
                {
                    bestLength = matchLength;
                    *offBasePtr = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex)) as usize;
                }
                if padd(ip, matchLength) == iend {
                    /* equal : no way to know if inf or sup */
                    if dictMode == ZSTD_dictMatchState {
                        nbCompares = 0;
                    }
                    break;
                }
            }

            if *padd(match_, matchLength) < *padd(ip, matchLength) {
                /* match is smaller than current */
                *smallerPtr = matchIndex;
                commonLengthSmaller = matchLength;
                if matchIndex <= btLow {
                    smallerPtr = &mut dummy32 as *mut U32;
                    break;
                }
                smallerPtr = nextPtr.add(1);
                matchIndex = *nextPtr.add(1);
            } else {
                /* match is larger than current */
                *largerPtr = matchIndex;
                commonLengthLarger = matchLength;
                if matchIndex <= btLow {
                    largerPtr = &mut dummy32 as *mut U32;
                    break;
                }
                largerPtr = nextPtr;
                matchIndex = *nextPtr;
            }
            nbCompares = nbCompares.wrapping_sub(1);
        }

        *largerPtr = 0;
        *smallerPtr = 0;

        if dictMode == ZSTD_dictMatchState && nbCompares != 0 {
            bestLength = ZSTD_DUBT_findBetterDictMatch(
                ms,
                ip,
                iend,
                offBasePtr,
                bestLength,
                nbCompares,
                mls,
                dictMode,
            );
        }

        (*ms).nextToUpdate = matchEndIdx.wrapping_sub(8); /* skip repetitive patterns */
        bestLength
    }
}

/** ZSTD_BtFindBestMatch() : Tree updater, providing best match */
unsafe fn ZSTD_BtFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    if ip < padd((*ms).window.base, (*ms).nextToUpdate as usize) {
        return 0; /* skipped area */
    }
    ZSTD_updateDUBT(ms, ip, iLimit, mls);
    ZSTD_DUBT_findBestMatch(ms, ip, iLimit, offBasePtr, mls, dictMode)
}

/***********************************
* Dedicated dict search
***********************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_dedicatedDictSearch_lazy_loadDictionary(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
) {
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = pdiff(ip, base) as U32;
    let hashTable: *mut U32 = (*ms).hashTable;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainSize: U32 = 1u32 << (*ms).cParams.chainLog;
    let mut idx: U32 = (*ms).nextToUpdate;
    let minChain: U32 = if chainSize < target.wrapping_sub(idx) {
        target.wrapping_sub(chainSize)
    } else {
        idx
    };
    let bucketSize: U32 = 1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG;
    let cacheSize: U32 = bucketSize.wrapping_sub(1);
    let chainAttempts: U32 = (1u32 << (*ms).cParams.searchLog).wrapping_sub(cacheSize);
    let chainLimit: U32 = if chainAttempts > 255 { 255 } else { chainAttempts };

    /* We know the hashtable is oversized by a factor of `bucketSize`. */
    let hashLog: U32 = (*ms).cParams.hashLog.wrapping_sub(ZSTD_LAZY_DDSS_BUCKET_LOG);
    let tmpHashTable: *mut U32 = hashTable;
    let tmpChainTable: *mut U32 = hashTable.add(1usize << hashLog);
    let tmpChainSize: U32 = ((1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG) - 1) << hashLog;
    let tmpMinChain: U32 = if tmpChainSize < target {
        target.wrapping_sub(tmpChainSize)
    } else {
        idx
    };
    let mut hashIdx: U32;

    /* fill conventional hash table and conventional chain table */
    while idx < target {
        let h: U32 = ZSTD_hashPtr(
            padd(base, idx as usize) as *const c_void,
            hashLog,
            (*ms).cParams.minMatch,
        ) as U32;
        if idx >= tmpMinChain {
            *tmpChainTable.add(idx.wrapping_sub(tmpMinChain) as usize) = *hashTable.add(h as usize);
        }
        *tmpHashTable.add(h as usize) = idx;
        idx = idx.wrapping_add(1);
    }

    /* sort chains into ddss chain table */
    {
        let mut chainPos: U32 = 0;
        hashIdx = 0;
        while hashIdx < (1u32 << hashLog) {
            let mut count: U32;
            let mut countBeyondMinChain: U32 = 0;
            let mut i: U32 = *tmpHashTable.add(hashIdx as usize);
            count = 0;
            while i >= tmpMinChain && count < cacheSize {
                /* skip through the chain to the first position that won't be
                 * in the hash cache bucket */
                if i < minChain {
                    countBeyondMinChain = countBeyondMinChain.wrapping_add(1);
                }
                i = *tmpChainTable.add(i.wrapping_sub(tmpMinChain) as usize);
                count = count.wrapping_add(1);
            }
            if count == cacheSize {
                count = 0;
                while count < chainLimit {
                    if i < minChain {
                        if i == 0 {
                            break;
                        }
                        countBeyondMinChain = countBeyondMinChain.wrapping_add(1);
                        if countBeyondMinChain > cacheSize {
                            break;
                        }
                    }
                    *chainTable.add(chainPos as usize) = i;
                    chainPos = chainPos.wrapping_add(1);
                    count = count.wrapping_add(1);
                    if i < tmpMinChain {
                        break;
                    }
                    i = *tmpChainTable.add(i.wrapping_sub(tmpMinChain) as usize);
                }
            } else {
                count = 0;
            }
            if count != 0 {
                *tmpHashTable.add(hashIdx as usize) =
                    (chainPos.wrapping_sub(count) << 8).wrapping_add(count);
            } else {
                *tmpHashTable.add(hashIdx as usize) = 0;
            }
            hashIdx = hashIdx.wrapping_add(1);
        }
    }

    /* move chain pointers into the last entry of each hash bucket */
    hashIdx = 1u32 << hashLog;
    while hashIdx != 0 {
        hashIdx = hashIdx.wrapping_sub(1);
        let bucketIdx: U32 = hashIdx << ZSTD_LAZY_DDSS_BUCKET_LOG;
        let chainPackedPointer: U32 = *tmpHashTable.add(hashIdx as usize);
        let mut i: U32 = 0;
        while i < cacheSize {
            *hashTable.add(bucketIdx.wrapping_add(i) as usize) = 0;
            i = i.wrapping_add(1);
        }
        *hashTable.add(bucketIdx.wrapping_add(bucketSize).wrapping_sub(1) as usize) =
            chainPackedPointer;
    }

    /* fill the buckets of the hash table */
    idx = (*ms).nextToUpdate;
    while idx < target {
        let h: U32 = (ZSTD_hashPtr(
            padd(base, idx as usize) as *const c_void,
            hashLog,
            (*ms).cParams.minMatch,
        ) as U32)
            << ZSTD_LAZY_DDSS_BUCKET_LOG;
        /* Shift hash cache down 1. */
        let mut i: U32 = cacheSize.wrapping_sub(1);
        while i != 0 {
            *hashTable.add(h.wrapping_add(i) as usize) =
                *hashTable.add(h.wrapping_add(i).wrapping_sub(1) as usize);
            i = i.wrapping_sub(1);
        }
        *hashTable.add(h as usize) = idx;
        idx = idx.wrapping_add(1);
    }

    (*ms).nextToUpdate = target;
}

/* Returns the longest match length found in the dedicated dict search structure. */
unsafe fn ZSTD_dedicatedDictSearch_lazy_search(
    offsetPtr: *mut usize,
    mut ml: usize,
    nbAttempts: U32,
    dms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    prefixStart: *const BYTE,
    curr: U32,
    dictLimit: U32,
    ddsIdx: usize,
) -> usize {
    let _ddsLowestIndex: U32 = (*dms).window.dictLimit;
    let ddsBase: *const BYTE = (*dms).window.base;
    let ddsEnd: *const BYTE = (*dms).window.nextSrc;
    let ddsSize: U32 = pdiff(ddsEnd, ddsBase) as U32;
    let ddsIndexDelta: U32 = dictLimit.wrapping_sub(ddsSize);
    let bucketSize: U32 = 1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG;
    let bucketLimit: U32 = if nbAttempts < bucketSize.wrapping_sub(1) {
        nbAttempts
    } else {
        bucketSize.wrapping_sub(1)
    };
    let mut ddsAttempt: U32 = 0;
    let mut matchIndex: U32;

    while ddsAttempt < bucketLimit {
        let mut currentMl: usize = 0;
        matchIndex = *(*dms).hashTable.add(ddsIdx.wrapping_add(ddsAttempt as usize));
        let match_: *const BYTE = padd(ddsBase, matchIndex as usize);

        if matchIndex == 0 {
            return ml;
        }

        if rd32(match_) == rd32(ip) {
            currentMl =
                ZSTD_count_2segments(padd(ip, 4), padd(match_, 4), iLimit, ddsEnd, prefixStart) + 4;
        }

        /* save best solution */
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = OFFSET_TO_OFFBASE(
                curr.wrapping_sub(matchIndex.wrapping_add(ddsIndexDelta)),
            ) as usize;
            if padd(ip, currentMl) == iLimit {
                /* best possible, avoids read overflow on next attempt */
                return ml;
            }
        }
        ddsAttempt = ddsAttempt.wrapping_add(1);
    }

    {
        let chainPackedPointer: U32 = *(*dms)
            .hashTable
            .add(ddsIdx.wrapping_add(bucketSize.wrapping_sub(1) as usize));
        let mut chainIndex: U32 = chainPackedPointer >> 8;
        let chainLength: U32 = chainPackedPointer & 0xFF;
        let chainAttempts: U32 = nbAttempts.wrapping_sub(ddsAttempt);
        let chainLimit: U32 = if chainAttempts > chainLength {
            chainLength
        } else {
            chainAttempts
        };
        let mut chainAttempt: U32 = 0;

        while chainAttempt < chainLimit {
            let mut currentMl: usize = 0;
            matchIndex = *(*dms).chainTable.add(chainIndex as usize);
            let match_: *const BYTE = padd(ddsBase, matchIndex as usize);

            if rd32(match_) == rd32(ip) {
                currentMl =
                    ZSTD_count_2segments(padd(ip, 4), padd(match_, 4), iLimit, ddsEnd, prefixStart)
                        + 4;
            }

            /* save best solution */
            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(
                    curr.wrapping_sub(matchIndex.wrapping_add(ddsIndexDelta)),
                ) as usize;
                if padd(ip, currentMl) == iLimit {
                    break;
                }
            }
            chainAttempt = chainAttempt.wrapping_add(1);
            chainIndex = chainIndex.wrapping_add(1);
        }
    }
    ml
}

/* *********************************
*  Hash Chain
***********************************/

/* Update chains up to ip (excluded) */
unsafe fn ZSTD_insertAndFindFirstIndex_internal(
    ms: *mut ZSTD_MatchState_t,
    cParams: *const ZSTD_compressionParameters,
    ip: *const BYTE,
    mls: U32,
    lazySkipping: U32,
) -> U32 {
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainMask: U32 = (1u32 << (*cParams).chainLog).wrapping_sub(1);
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = pdiff(ip, base) as U32;
    let mut idx: U32 = (*ms).nextToUpdate;

    while idx < target {
        /* catch up */
        let h: usize = ZSTD_hashPtr(padd(base, idx as usize) as *const c_void, hashLog, mls);
        *chainTable.add((idx & chainMask) as usize) = *hashTable.add(h);
        *hashTable.add(h) = idx;
        idx = idx.wrapping_add(1);
        /* Stop inserting every position when in the lazy skipping mode. */
        if lazySkipping != 0 {
            break;
        }
    }

    (*ms).nextToUpdate = target;
    *hashTable.add(ZSTD_hashPtr(ip as *const c_void, hashLog, mls))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_insertAndFindFirstIndex(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
) -> U32 {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    ZSTD_insertAndFindFirstIndex_internal(ms, cParams, ip, (*ms).cParams.minMatch, 0)
}

unsafe fn ZSTD_HcFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainSize: U32 = 1u32 << (*cParams).chainLog;
    let chainMask: U32 = chainSize.wrapping_sub(1);
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = padd(base, dictLimit as usize);
    let dictEnd: *const BYTE = padd(dictBase, dictLimit as usize);
    let curr: U32 = pdiff(ip, base) as U32;
    let maxDistance: U32 = 1u32 << (*cParams).windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinMaxDistance: U32 = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0) as U32;
    let lowLimit: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinMaxDistance
    };
    let minChain: U32 = if curr > chainSize {
        curr.wrapping_sub(chainSize)
    } else {
        0
    };
    let mut nbAttempts: U32 = 1u32 << (*cParams).searchLog;
    let mut ml: usize = 4 - 1;

    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let ddsHashLog: U32 = if dictMode == ZSTD_dedicatedDictSearch {
        (*dms).cParams.hashLog.wrapping_sub(ZSTD_LAZY_DDSS_BUCKET_LOG)
    } else {
        0
    };
    let ddsIdx: usize = if dictMode == ZSTD_dedicatedDictSearch {
        ZSTD_hashPtr(ip as *const c_void, ddsHashLog, mls) << ZSTD_LAZY_DDSS_BUCKET_LOG
    } else {
        0
    };

    let mut matchIndex: U32;

    /* HC4 match finder */
    matchIndex = ZSTD_insertAndFindFirstIndex_internal(
        ms,
        cParams,
        ip,
        mls,
        (*ms).lazySkipping as U32,
    );

    while (matchIndex >= lowLimit) && (nbAttempts > 0) {
        let mut currentMl: usize = 0;
        if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
            let match_: *const BYTE = padd(base, matchIndex as usize);
            /* read 4B starting from (match + ml + 1 - sizeof(U32)) */
            if rd32(psub(padd(match_, ml), 3)) == rd32(psub(padd(ip, ml), 3)) {
                currentMl = ZSTD_count(ip, match_, iLimit);
            }
        } else {
            let match_: *const BYTE = padd(dictBase, matchIndex as usize);
            if rd32(match_) == rd32(ip) {
                currentMl = ZSTD_count_2segments(
                    padd(ip, 4),
                    padd(match_, 4),
                    iLimit,
                    dictEnd,
                    prefixStart,
                ) + 4;
            }
        }

        /* save best solution */
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex)) as usize;
            if padd(ip, currentMl) == iLimit {
                break; /* best possible, avoids read overflow on next attempt */
            }
        }

        if matchIndex <= minChain {
            break;
        }
        matchIndex = *chainTable.add((matchIndex & chainMask) as usize);
        nbAttempts = nbAttempts.wrapping_sub(1);
    }

    if dictMode == ZSTD_dedicatedDictSearch {
        ml = ZSTD_dedicatedDictSearch_lazy_search(
            offsetPtr,
            ml,
            nbAttempts,
            dms,
            ip,
            iLimit,
            prefixStart,
            curr,
            dictLimit,
            ddsIdx,
        );
    } else if dictMode == ZSTD_dictMatchState {
        let dmsChainTable: *const U32 = (*dms).chainTable;
        let dmsChainSize: U32 = 1u32 << (*dms).cParams.chainLog;
        let dmsChainMask: U32 = dmsChainSize.wrapping_sub(1);
        let dmsLowestIndex: U32 = (*dms).window.dictLimit;
        let dmsBase: *const BYTE = (*dms).window.base;
        let dmsEnd: *const BYTE = (*dms).window.nextSrc;
        let dmsSize: U32 = pdiff(dmsEnd, dmsBase) as U32;
        let dmsIndexDelta: U32 = dictLimit.wrapping_sub(dmsSize);
        let dmsMinChain: U32 = if dmsSize > dmsChainSize {
            dmsSize.wrapping_sub(dmsChainSize)
        } else {
            0
        };

        matchIndex = *(*dms)
            .hashTable
            .add(ZSTD_hashPtr(ip as *const c_void, (*dms).cParams.hashLog, mls));

        while (matchIndex >= dmsLowestIndex) && (nbAttempts > 0) {
            let mut currentMl: usize = 0;
            let match_: *const BYTE = padd(dmsBase, matchIndex as usize);
            if rd32(match_) == rd32(ip) {
                currentMl =
                    ZSTD_count_2segments(padd(ip, 4), padd(match_, 4), iLimit, dmsEnd, prefixStart)
                        + 4;
            }

            /* save best solution */
            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(
                    curr.wrapping_sub(matchIndex.wrapping_add(dmsIndexDelta)),
                ) as usize;
                if padd(ip, currentMl) == iLimit {
                    break;
                }
            }

            if matchIndex <= dmsMinChain {
                break;
            }

            matchIndex = *dmsChainTable.add((matchIndex & dmsChainMask) as usize);
            nbAttempts = nbAttempts.wrapping_sub(1);
        }
    }

    ml
}

/* *********************************
* (SIMD) Row-based matchfinder
***********************************/
/* Constants for row-based hash */
pub const ZSTD_ROW_HASH_TAG_MASK: U32 = (1u32 << ZSTD_ROW_HASH_TAG_BITS) - 1;
/* absolute maximum number of entries per row, for all configurations */
pub const ZSTD_ROW_HASH_MAX_ENTRIES: usize = 64;

pub const ZSTD_ROW_HASH_CACHE_MASK: U32 = (ZSTD_ROW_HASH_CACHE_SIZE - 1) as U32;

/// Clarifies when we are interacting with a U64 representing a mask of matches
pub type ZSTD_VecMask = U64;

/* ZSTD_VecMask_next():
 * Starting from the LSB, returns the idx of the next non-zero bit. */
#[inline(always)]
fn ZSTD_VecMask_next(val: ZSTD_VecMask) -> U32 {
    ZSTD_countTrailingZeros64(val)
}

/* ZSTD_row_nextIndex():
 * Returns the next index to insert at within a tagTable row, and updates the
 * "head" value to reflect the update. */
#[inline(always)]
unsafe fn ZSTD_row_nextIndex(tagRow: *mut BYTE, rowMask: U32) -> U32 {
    let mut next: U32 = ((*tagRow as U32).wrapping_sub(1)) & rowMask;
    next = next.wrapping_add(if next == 0 { rowMask } else { 0 }); /* skip first position */
    *tagRow = next as BYTE;
    next
}

/* ZSTD_isAligned():
 * Checks that a pointer is aligned to "align" bytes which must be a power of 2. */
#[inline(always)]
fn ZSTD_isAligned(ptr: *const c_void, align: usize) -> c_int {
    (((ptr as usize) & align.wrapping_sub(1)) == 0) as c_int
}

/* ZSTD_row_prefetch():
 * PREFETCH_L1 is a no-op in this build. */
#[inline(always)]
unsafe fn ZSTD_row_prefetch(
    _hashTable: *const U32,
    _tagTable: *const BYTE,
    _relRow: U32,
    _rowLog: U32,
) {
}

/* ZSTD_row_fillHashCache():
 * Fill up the hash cache starting at idx, but not beyond iLimit. */
unsafe fn ZSTD_row_fillHashCache(
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
    let maxElemsToPrefetch: U32 = if padd(base, idx as usize) > iLimit {
        0
    } else {
        (pdiff(iLimit, padd(base, idx as usize)) as U32).wrapping_add(1)
    };
    let lim: U32 = idx.wrapping_add(if (ZSTD_ROW_HASH_CACHE_SIZE as U32) < maxElemsToPrefetch {
        ZSTD_ROW_HASH_CACHE_SIZE as U32
    } else {
        maxElemsToPrefetch
    });

    while idx < lim {
        let hash: U32 = ZSTD_hashPtrSalted(
            padd(base, idx as usize) as *const c_void,
            hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS),
            mls,
            (*ms).hashSalt,
        ) as U32;
        let row: U32 = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        ZSTD_row_prefetch(hashTable, tagTable, row, rowLog);
        (*ms).hashCache[(idx & ZSTD_ROW_HASH_CACHE_MASK) as usize] = hash;
        idx = idx.wrapping_add(1);
    }
}

/* ZSTD_row_nextCachedHash():
 * Returns the hash of base + idx, and replaces the hash in the hash cache with
 * the byte at base + idx + ZSTD_ROW_HASH_CACHE_SIZE. */
unsafe fn ZSTD_row_nextCachedHash(
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
        padd(base, (idx as usize).wrapping_add(ZSTD_ROW_HASH_CACHE_SIZE)) as *const c_void,
        hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS),
        mls,
        hashSalt,
    ) as U32;
    let row: U32 = (newHash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
    ZSTD_row_prefetch(hashTable, tagTable, row, rowLog);
    {
        let hash: U32 = *cache.add((idx & ZSTD_ROW_HASH_CACHE_MASK) as usize);
        *cache.add((idx & ZSTD_ROW_HASH_CACHE_MASK) as usize) = newHash;
        hash
    }
}

/* ZSTD_row_update_internalImpl():
 * Updates the hash table with positions starting from updateStartIdx until
 * updateEndIdx. */
unsafe fn ZSTD_row_update_internalImpl(
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
                padd(base, updateStartIdx as usize) as *const c_void,
                hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS),
                mls,
                (*ms).hashSalt,
            ) as U32
        };
        let relRow: U32 = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        let row: *mut U32 = hashTable.add(relRow as usize);
        let tagRow: *mut BYTE = tagTable.add(relRow as usize);
        let pos: U32 = ZSTD_row_nextIndex(tagRow, rowMask);

        *tagRow.add(pos as usize) = (hash & ZSTD_ROW_HASH_TAG_MASK) as BYTE;
        *row.add(pos as usize) = updateStartIdx;
        updateStartIdx = updateStartIdx.wrapping_add(1);
    }
}

/* ZSTD_row_update_internal():
 * Inserts the byte at ip into the appropriate position in the hash table, and
 * updates ms->nextToUpdate. Skips sections of long matches as is necessary. */
unsafe fn ZSTD_row_update_internal(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    mls: U32,
    rowLog: U32,
    rowMask: U32,
    useCache: U32,
) {
    let mut idx: U32 = (*ms).nextToUpdate;
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = pdiff(ip, base) as U32;
    let kSkipThreshold: U32 = 384;
    let kMaxMatchStartPositionsToUpdate: U32 = 96;
    let kMaxMatchEndPositionsToUpdate: U32 = 32;

    if useCache != 0 {
        /* Only skip positions when using hash cache. */
        if target.wrapping_sub(idx) > kSkipThreshold {
            let bound: U32 = idx.wrapping_add(kMaxMatchStartPositionsToUpdate);
            ZSTD_row_update_internalImpl(ms, idx, bound, mls, rowLog, rowMask, useCache);
            idx = target.wrapping_sub(kMaxMatchEndPositionsToUpdate);
            ZSTD_row_fillHashCache(ms, base, rowLog, mls, idx, padd(ip, 1));
        }
    }
    ZSTD_row_update_internalImpl(ms, idx, target, mls, rowLog, rowMask, useCache);
    (*ms).nextToUpdate = target;
}

/* ZSTD_row_update():
 * External wrapper for ZSTD_row_update_internal(). */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_row_update(ms: *mut ZSTD_MatchState_t, ip: *const BYTE) {
    let rowLog: U32 = bounded_u32(4, (*ms).cParams.searchLog, 6);
    let rowMask: U32 = (1u32 << rowLog).wrapping_sub(1);
    let mls: U32 = if (*ms).cParams.minMatch < 6 {
        (*ms).cParams.minMatch
    } else {
        6
    };

    ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 0 /* don't use cache */);
}

/* Returns the mask width of bits group of which will be set to 1.
 * ZSTD_ARCH_ARM_NEON is not defined on this target, so this is always 1. */
#[inline(always)]
fn ZSTD_row_matchMaskGroupWidth(_rowEntries: U32) -> U32 {
    1
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn ZSTD_row_getSSEMask(
    nbChunks: c_int,
    src: *const BYTE,
    tag: BYTE,
    head: U32,
) -> ZSTD_VecMask {
    let comparisonMask = _mm_set1_epi8(tag as i8);
    let mut matches: [c_int; 4] = [0; 4];
    let mut i: c_int = 0;
    while i < nbChunks {
        let chunk = _mm_loadu_si128(padd(src, 16 * i as usize) as *const __m128i);
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

/* Returns a ZSTD_VecMask (U64) that has the nth group of bits set to 1 if the
 * newly-computed "tag" matches the hash at the nth position in a row of the
 * tagTable. `ZSTD_ARCH_X86_SSE2` is defined for x86-64 (GCC defines __SSE2__),
 * so the SSE2 path is the one compiled. */
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn ZSTD_row_getMatchMask(
    tagRow: *const BYTE,
    tag: BYTE,
    headGrouped: U32,
    rowEntries: U32,
) -> ZSTD_VecMask {
    let src: *const BYTE = tagRow;
    ZSTD_row_getSSEMask((rowEntries / 16) as c_int, src, tag, headGrouped)
}

/* SWAR fallback for non-x86-64 targets (not used by the reference build). */
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
unsafe fn ZSTD_row_getMatchMask(
    tagRow: *const BYTE,
    tag: BYTE,
    headGrouped: U32,
    rowEntries: U32,
) -> ZSTD_VecMask {
    let src: *const BYTE = tagRow;
    let chunkSize: c_int = core::mem::size_of::<usize>() as c_int;
    let shiftAmount: usize = ((chunkSize * 8) - chunkSize) as usize;
    let xFF: usize = !(0usize);
    let x01: usize = xFF / 0xFF;
    let x80: usize = x01 << 7;
    let splatChar: usize = (tag as usize).wrapping_mul(x01);
    let mut matches: ZSTD_VecMask = 0;
    let mut i: c_int = rowEntries as c_int - chunkSize;
    let extractMagic: usize = (xFF / 0x7F) >> chunkSize;
    loop {
        let mut chunk: usize = MEM_readST(padd(src, i as usize) as *const c_void);
        chunk ^= splatChar;
        chunk = (((chunk | x80).wrapping_sub(x01)) | chunk) & x80;
        matches <<= chunkSize as u32;
        matches |= ((chunk.wrapping_mul(extractMagic)) >> shiftAmount) as ZSTD_VecMask;
        i -= chunkSize;
        if i < 0 {
            break;
        }
    }
    matches = !matches;
    if rowEntries == 16 {
        ZSTD_rotateRight_U16(matches as U16, headGrouped) as ZSTD_VecMask
    } else if rowEntries == 32 {
        ZSTD_rotateRight_U32(matches as U32, headGrouped) as ZSTD_VecMask
    } else {
        ZSTD_rotateRight_U64(matches, headGrouped)
    }
}

unsafe fn ZSTD_RowFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
    rowLog: U32,
) -> usize {
    let hashTable: *mut U32 = (*ms).hashTable;
    let tagTable: *mut BYTE = (*ms).tagTable;
    let hashCache: *mut U32 = (*ms).hashCache.as_mut_ptr();
    let hashLog: U32 = (*ms).rowHashLog;
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = padd(base, dictLimit as usize);
    let dictEnd: *const BYTE = padd(dictBase, dictLimit as usize);
    let curr: U32 = pdiff(ip, base) as U32;
    let maxDistance: U32 = 1u32 << (*cParams).windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinMaxDistance: U32 = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
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
    let rowMask: U32 = rowEntries.wrapping_sub(1);
    /* nb of searches is capped at nb entries per row */
    let cappedSearchLog: U32 = if (*cParams).searchLog < rowLog {
        (*cParams).searchLog
    } else {
        rowLog
    };
    let groupWidth: U32 = ZSTD_row_matchMaskGroupWidth(rowEntries);
    let hashSalt: U64 = (*ms).hashSalt;
    let mut nbAttempts: U32 = 1u32 << cappedSearchLog;
    let mut ml: usize = 4 - 1;
    let hash: U32;

    /* DMS/DDS variables that may be referenced later */
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;

    /* Initialize the following variables to satisfy static analyzer */
    let mut ddsIdx: usize = 0;
    let mut ddsExtraAttempts: U32 = 0;
    let mut dmsTag: U32 = 0;
    let mut dmsRow: *mut U32 = core::ptr::null_mut();
    let mut dmsTagRow: *mut BYTE = core::ptr::null_mut();

    if dictMode == ZSTD_dedicatedDictSearch {
        let ddsHashLog: U32 = (*dms).cParams.hashLog.wrapping_sub(ZSTD_LAZY_DDSS_BUCKET_LOG);
        {
            /* Prefetch DDS hashtable entry */
            ddsIdx = ZSTD_hashPtr(ip as *const c_void, ddsHashLog, mls)
                << ZSTD_LAZY_DDSS_BUCKET_LOG;
        }
        ddsExtraAttempts = if (*cParams).searchLog > rowLog {
            1u32 << ((*cParams).searchLog.wrapping_sub(rowLog))
        } else {
            0
        };
    }

    if dictMode == ZSTD_dictMatchState {
        /* Prefetch DMS rows */
        let dmsHashTable: *mut U32 = (*dms).hashTable;
        let dmsTagTable: *mut BYTE = (*dms).tagTable;
        let dmsHash: U32 = ZSTD_hashPtr(
            ip as *const c_void,
            (*dms).rowHashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS),
            mls,
        ) as U32;
        let dmsRelRow: U32 = (dmsHash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        dmsTag = dmsHash & ZSTD_ROW_HASH_TAG_MASK;
        dmsTagRow = dmsTagTable.add(dmsRelRow as usize);
        dmsRow = dmsHashTable.add(dmsRelRow as usize);
        ZSTD_row_prefetch(dmsHashTable, dmsTagTable, dmsRelRow, rowLog);
    }

    /* Update the hashTable and tagTable up to (but not including) ip */
    if (*ms).lazySkipping == 0 {
        ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 1 /* useCache */);
        hash = ZSTD_row_nextCachedHash(
            hashCache, hashTable, tagTable, base, curr, hashLog, rowLog, mls, hashSalt,
        );
    } else {
        /* Stop inserting every position when in the lazy skipping mode. */
        hash = ZSTD_hashPtrSalted(
            ip as *const c_void,
            hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS),
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
        let row: *mut U32 = hashTable.add(relRow as usize);
        let tagRow: *mut BYTE = tagTable.add(relRow as usize);
        let headGrouped: U32 = ((*tagRow as U32) & rowMask).wrapping_mul(groupWidth);
        let mut matchBuffer: [U32; ZSTD_ROW_HASH_MAX_ENTRIES] = [0; ZSTD_ROW_HASH_MAX_ENTRIES];
        let mut numMatches: usize = 0;
        let mut currMatch: usize = 0;
        let mut matches: ZSTD_VecMask =
            ZSTD_row_getMatchMask(tagRow, tag as BYTE, headGrouped, rowEntries);

        /* Cycle through the matches */
        while (matches > 0) && (nbAttempts > 0) {
            let matchPos: U32 = ((headGrouped.wrapping_add(ZSTD_VecMask_next(matches)))
                / groupWidth)
                & rowMask;
            let matchIndex: U32 = *row.add(matchPos as usize);
            if matchPos == 0 {
                matches &= matches.wrapping_sub(1);
                continue;
            }
            if matchIndex < lowLimit {
                break;
            }
            matchBuffer[numMatches] = matchIndex;
            numMatches += 1;
            nbAttempts = nbAttempts.wrapping_sub(1);
            matches &= matches.wrapping_sub(1);
        }

        /* Speed opt: insert current byte into hashtable too. */
        {
            let pos: U32 = ZSTD_row_nextIndex(tagRow, rowMask);
            *tagRow.add(pos as usize) = tag as BYTE;
            *row.add(pos as usize) = (*ms).nextToUpdate;
            (*ms).nextToUpdate = (*ms).nextToUpdate.wrapping_add(1);
        }

        /* Return the longest match */
        while currMatch < numMatches {
            let matchIndex: U32 = matchBuffer[currMatch];
            let mut currentMl: usize = 0;

            if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
                let match_: *const BYTE = padd(base, matchIndex as usize);
                /* read 4B starting from (match + ml + 1 - sizeof(U32)) */
                if rd32(psub(padd(match_, ml), 3)) == rd32(psub(padd(ip, ml), 3)) {
                    currentMl = ZSTD_count(ip, match_, iLimit);
                }
            } else {
                let match_: *const BYTE = padd(dictBase, matchIndex as usize);
                if rd32(match_) == rd32(ip) {
                    currentMl = ZSTD_count_2segments(
                        padd(ip, 4),
                        padd(match_, 4),
                        iLimit,
                        dictEnd,
                        prefixStart,
                    ) + 4;
                }
            }

            /* Save best solution */
            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex)) as usize;
                if padd(ip, currentMl) == iLimit {
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
            nbAttempts.wrapping_add(ddsExtraAttempts),
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
        let dmsSize: U32 = pdiff(dmsEnd, dmsBase) as U32;
        let dmsIndexDelta: U32 = dictLimit.wrapping_sub(dmsSize);

        {
            let headGrouped: U32 = ((*dmsTagRow as U32) & rowMask).wrapping_mul(groupWidth);
            let mut matchBuffer: [U32; ZSTD_ROW_HASH_MAX_ENTRIES] =
                [0; ZSTD_ROW_HASH_MAX_ENTRIES];
            let mut numMatches: usize = 0;
            let mut currMatch: usize = 0;
            let mut matches: ZSTD_VecMask =
                ZSTD_row_getMatchMask(dmsTagRow, dmsTag as BYTE, headGrouped, rowEntries);

            while (matches > 0) && (nbAttempts > 0) {
                let matchPos: U32 = ((headGrouped.wrapping_add(ZSTD_VecMask_next(matches)))
                    / groupWidth)
                    & rowMask;
                let matchIndex: U32 = *dmsRow.add(matchPos as usize);
                if matchPos == 0 {
                    matches &= matches.wrapping_sub(1);
                    continue;
                }
                if matchIndex < dmsLowestIndex {
                    break;
                }
                matchBuffer[numMatches] = matchIndex;
                numMatches += 1;
                nbAttempts = nbAttempts.wrapping_sub(1);
                matches &= matches.wrapping_sub(1);
            }

            /* Return the longest match */
            while currMatch < numMatches {
                let matchIndex: U32 = matchBuffer[currMatch];
                let mut currentMl: usize = 0;

                {
                    let match_: *const BYTE = padd(dmsBase, matchIndex as usize);
                    if rd32(match_) == rd32(ip) {
                        currentMl = ZSTD_count_2segments(
                            padd(ip, 4),
                            padd(match_, 4),
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
                    ) as usize;
                    if padd(ip, currentMl) == iLimit {
                        break;
                    }
                }
                currMatch += 1;
            }
        }
    }
    ml
}

/**
 * Generated search functions templated on (dictMode, mls, rowLog).
 * (expansion of GEN_ZSTD_ROW_SEARCH_FN / GEN_ZSTD_BT_SEARCH_FN /
 *  GEN_ZSTD_HC_SEARCH_FN via ZSTD_FOR_EACH_DICT_MODE)
 */

/* ---- row hash search fns: ZSTD_RowFindBestMatch_<dictMode>_<mls>_<rowLog> ---- */

pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_4_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_noDict, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_4_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_noDict, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_4_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_noDict, 6)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_5_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_noDict, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_5_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_noDict, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_5_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_noDict, 6)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_6_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_noDict, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_6_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_noDict, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_noDict_6_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_noDict, 6)
}

pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_4_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_extDict, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_4_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_extDict, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_4_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_extDict, 6)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_5_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_extDict, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_5_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_extDict, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_5_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_extDict, 6)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_6_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_extDict, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_6_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_extDict, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_extDict_6_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_extDict, 6)
}

pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_4_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_dictMatchState, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_4_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_dictMatchState, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_4_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_dictMatchState, 6)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_5_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_dictMatchState, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_5_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_dictMatchState, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_5_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_dictMatchState, 6)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_6_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_dictMatchState, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_6_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_dictMatchState, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dictMatchState_6_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_dictMatchState, 6)
}

pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_4_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_dedicatedDictSearch, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_4_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_dedicatedDictSearch, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_4_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_dedicatedDictSearch, 6)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_5_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_dedicatedDictSearch, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_5_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_dedicatedDictSearch, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_5_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_dedicatedDictSearch, 6)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_6_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_dedicatedDictSearch, 4)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_6_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_dedicatedDictSearch, 5)
}
pub(crate) unsafe fn ZSTD_RowFindBestMatch_dedicatedDictSearch_6_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_dedicatedDictSearch, 6)
}

/* ---- binary tree search fns: ZSTD_BtFindBestMatch_<dictMode>_<mls> ---- */

pub(crate) unsafe fn ZSTD_BtFindBestMatch_noDict_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 4, ZSTD_noDict)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_noDict_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 5, ZSTD_noDict)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_noDict_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 6, ZSTD_noDict)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_extDict_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 4, ZSTD_extDict)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_extDict_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 5, ZSTD_extDict)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_extDict_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 6, ZSTD_extDict)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_dictMatchState_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 4, ZSTD_dictMatchState)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_dictMatchState_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 5, ZSTD_dictMatchState)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_dictMatchState_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 6, ZSTD_dictMatchState)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_dedicatedDictSearch_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 4, ZSTD_dedicatedDictSearch)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_dedicatedDictSearch_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 5, ZSTD_dedicatedDictSearch)
}
pub(crate) unsafe fn ZSTD_BtFindBestMatch_dedicatedDictSearch_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
) -> usize {
    ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 6, ZSTD_dedicatedDictSearch)
}

/* ---- hash chain search fns: ZSTD_HcFindBestMatch_<dictMode>_<mls> ---- */

pub(crate) unsafe fn ZSTD_HcFindBestMatch_noDict_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_noDict)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_noDict_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_noDict)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_noDict_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_noDict)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_extDict_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_extDict)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_extDict_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_extDict)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_extDict_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_extDict)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_dictMatchState_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_dictMatchState)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_dictMatchState_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_dictMatchState)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_dictMatchState_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_dictMatchState)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_dedicatedDictSearch_4(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 4, ZSTD_dedicatedDictSearch)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_dedicatedDictSearch_5(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 5, ZSTD_dedicatedDictSearch)
}
pub(crate) unsafe fn ZSTD_HcFindBestMatch_dedicatedDictSearch_6(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
) -> usize {
    ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 6, ZSTD_dedicatedDictSearch)
}

pub type searchMethod_e = c_uint;
pub const search_hashChain: searchMethod_e = 0;
pub const search_binaryTree: searchMethod_e = 1;
pub const search_rowHash: searchMethod_e = 2;

/**
 * Searches for the longest match at @p ip.
 * (expansion of ZSTD_SWITCH_SEARCH_METHOD / ZSTD_SWITCH_MLS / ZSTD_SWITCH_ROWLOG)
 */
unsafe fn ZSTD_searchMax(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    offsetPtr: *mut usize,
    mls: U32,
    rowLog: U32,
    searchMethod: searchMethod_e,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    if dictMode == ZSTD_noDict {
        match searchMethod {
            search_hashChain => match mls {
                4 => return ZSTD_HcFindBestMatch_noDict_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_HcFindBestMatch_noDict_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_HcFindBestMatch_noDict_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            search_binaryTree => match mls {
                4 => return ZSTD_BtFindBestMatch_noDict_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_BtFindBestMatch_noDict_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_BtFindBestMatch_noDict_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            search_rowHash => match mls {
                4 => match rowLog {
                    4 => return ZSTD_RowFindBestMatch_noDict_4_4(ms, ip, iend, offsetPtr),
                    5 => return ZSTD_RowFindBestMatch_noDict_4_5(ms, ip, iend, offsetPtr),
                    6 => return ZSTD_RowFindBestMatch_noDict_4_6(ms, ip, iend, offsetPtr),
                    _ => {}
                },
                5 => match rowLog {
                    4 => return ZSTD_RowFindBestMatch_noDict_5_4(ms, ip, iend, offsetPtr),
                    5 => return ZSTD_RowFindBestMatch_noDict_5_5(ms, ip, iend, offsetPtr),
                    6 => return ZSTD_RowFindBestMatch_noDict_5_6(ms, ip, iend, offsetPtr),
                    _ => {}
                },
                6 => match rowLog {
                    4 => return ZSTD_RowFindBestMatch_noDict_6_4(ms, ip, iend, offsetPtr),
                    5 => return ZSTD_RowFindBestMatch_noDict_6_5(ms, ip, iend, offsetPtr),
                    6 => return ZSTD_RowFindBestMatch_noDict_6_6(ms, ip, iend, offsetPtr),
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    } else if dictMode == ZSTD_extDict {
        match searchMethod {
            search_hashChain => match mls {
                4 => return ZSTD_HcFindBestMatch_extDict_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_HcFindBestMatch_extDict_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_HcFindBestMatch_extDict_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            search_binaryTree => match mls {
                4 => return ZSTD_BtFindBestMatch_extDict_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_BtFindBestMatch_extDict_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_BtFindBestMatch_extDict_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            search_rowHash => match mls {
                4 => match rowLog {
                    4 => return ZSTD_RowFindBestMatch_extDict_4_4(ms, ip, iend, offsetPtr),
                    5 => return ZSTD_RowFindBestMatch_extDict_4_5(ms, ip, iend, offsetPtr),
                    6 => return ZSTD_RowFindBestMatch_extDict_4_6(ms, ip, iend, offsetPtr),
                    _ => {}
                },
                5 => match rowLog {
                    4 => return ZSTD_RowFindBestMatch_extDict_5_4(ms, ip, iend, offsetPtr),
                    5 => return ZSTD_RowFindBestMatch_extDict_5_5(ms, ip, iend, offsetPtr),
                    6 => return ZSTD_RowFindBestMatch_extDict_5_6(ms, ip, iend, offsetPtr),
                    _ => {}
                },
                6 => match rowLog {
                    4 => return ZSTD_RowFindBestMatch_extDict_6_4(ms, ip, iend, offsetPtr),
                    5 => return ZSTD_RowFindBestMatch_extDict_6_5(ms, ip, iend, offsetPtr),
                    6 => return ZSTD_RowFindBestMatch_extDict_6_6(ms, ip, iend, offsetPtr),
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    } else if dictMode == ZSTD_dictMatchState {
        match searchMethod {
            search_hashChain => match mls {
                4 => return ZSTD_HcFindBestMatch_dictMatchState_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_HcFindBestMatch_dictMatchState_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_HcFindBestMatch_dictMatchState_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            search_binaryTree => match mls {
                4 => return ZSTD_BtFindBestMatch_dictMatchState_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_BtFindBestMatch_dictMatchState_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_BtFindBestMatch_dictMatchState_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            search_rowHash => match mls {
                4 => match rowLog {
                    4 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_4_4(ms, ip, iend, offsetPtr)
                    }
                    5 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_4_5(ms, ip, iend, offsetPtr)
                    }
                    6 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_4_6(ms, ip, iend, offsetPtr)
                    }
                    _ => {}
                },
                5 => match rowLog {
                    4 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_5_4(ms, ip, iend, offsetPtr)
                    }
                    5 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_5_5(ms, ip, iend, offsetPtr)
                    }
                    6 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_5_6(ms, ip, iend, offsetPtr)
                    }
                    _ => {}
                },
                6 => match rowLog {
                    4 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_6_4(ms, ip, iend, offsetPtr)
                    }
                    5 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_6_5(ms, ip, iend, offsetPtr)
                    }
                    6 => {
                        return ZSTD_RowFindBestMatch_dictMatchState_6_6(ms, ip, iend, offsetPtr)
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    } else if dictMode == ZSTD_dedicatedDictSearch {
        match searchMethod {
            search_hashChain => match mls {
                4 => return ZSTD_HcFindBestMatch_dedicatedDictSearch_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_HcFindBestMatch_dedicatedDictSearch_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_HcFindBestMatch_dedicatedDictSearch_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            search_binaryTree => match mls {
                4 => return ZSTD_BtFindBestMatch_dedicatedDictSearch_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_BtFindBestMatch_dedicatedDictSearch_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_BtFindBestMatch_dedicatedDictSearch_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            search_rowHash => match mls {
                4 => match rowLog {
                    4 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_4_4(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    5 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_4_5(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    6 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_4_6(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    _ => {}
                },
                5 => match rowLog {
                    4 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_5_4(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    5 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_5_5(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    6 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_5_6(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    _ => {}
                },
                6 => match rowLog {
                    4 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_6_4(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    5 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_6_5(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    6 => {
                        return ZSTD_RowFindBestMatch_dedicatedDictSearch_6_6(
                            ms, ip, iend, offsetPtr,
                        )
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
    0
}

/* *******************************
*  Common parser - lazy strategy
*********************************/

unsafe fn ZSTD_compressBlock_lazy_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    searchMethod: searchMethod_e,
    depth: U32,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = padd(istart, srcSize);
    let ilimit: *const BYTE = if searchMethod == search_rowHash {
        psub(iend, 8 + ZSTD_ROW_HASH_CACHE_SIZE)
    } else {
        psub(iend, 8)
    };
    let base: *const BYTE = (*ms).window.base;
    let prefixLowestIndex: U32 = (*ms).window.dictLimit;
    let prefixLowest: *const BYTE = padd(base, prefixLowestIndex as usize);
    let mls: U32 = bounded_u32(4, (*ms).cParams.minMatch, 6);
    let rowLog: U32 = bounded_u32(4, (*ms).cParams.searchLog, 6);

    let mut offset_1: U32 = *rep;
    let mut offset_2: U32 = *rep.add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let isDMS: bool = dictMode == ZSTD_dictMatchState;
    let isDDS: bool = dictMode == ZSTD_dedicatedDictSearch;
    let isDxS: bool = isDMS || isDDS;
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictLowestIndex: U32 = if isDxS { (*dms).window.dictLimit } else { 0 };
    let dictBase: *const BYTE = if isDxS {
        (*dms).window.base
    } else {
        core::ptr::null()
    };
    let dictLowest: *const BYTE = if isDxS {
        padd(dictBase, dictLowestIndex as usize)
    } else {
        core::ptr::null()
    };
    let dictEnd: *const BYTE = if isDxS {
        (*dms).window.nextSrc
    } else {
        core::ptr::null()
    };
    let dictIndexDelta: U32 = if isDxS {
        prefixLowestIndex.wrapping_sub(pdiff(dictEnd, dictBase) as U32)
    } else {
        0
    };
    let dictAndPrefixLength: U32 =
        (pdiff(ip, prefixLowest).wrapping_add(pdiff(dictEnd, dictLowest))) as U32;

    ip = padd(ip, (dictAndPrefixLength == 0) as usize);
    if dictMode == ZSTD_noDict {
        let curr: U32 = pdiff(ip, base) as U32;
        let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, curr, (*ms).cParams.windowLog);
        let maxRep: U32 = curr.wrapping_sub(windowLow);
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
    'main: while ip < ilimit {
        let mut matchLength: usize = 0;
        let mut offBase: usize = REPCODE1_TO_OFFBASE as usize;
        let mut start: *const BYTE = padd(ip, 1);

        'store: {
            /* check repCode */
            if isDxS {
                let repIndex: U32 = (pdiff(ip, base) as U32)
                    .wrapping_add(1)
                    .wrapping_sub(offset_1);
                let repMatch: *const BYTE = if (dictMode == ZSTD_dictMatchState
                    || dictMode == ZSTD_dedicatedDictSearch)
                    && repIndex < prefixLowestIndex
                {
                    padd(dictBase, repIndex.wrapping_sub(dictIndexDelta) as usize)
                } else {
                    padd(base, repIndex as usize)
                };
                if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                    && (rd32(repMatch) == rd32(padd(ip, 1)))
                {
                    let repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    matchLength = ZSTD_count_2segments(
                        padd(ip, 1 + 4),
                        padd(repMatch, 4),
                        iend,
                        repMatchEnd,
                        prefixLowest,
                    ) + 4;
                    if depth == 0 {
                        break 'store;
                    }
                }
            }
            if dictMode == ZSTD_noDict
                && (offset_1 > 0)
                && (rd32(psub(padd(ip, 1), offset_1 as usize)) == rd32(padd(ip, 1)))
            {
                matchLength = ZSTD_count(
                    padd(ip, 1 + 4),
                    psub(padd(ip, 1 + 4), offset_1 as usize),
                    iend,
                ) + 4;
                if depth == 0 {
                    break 'store;
                }
            }

            /* first search (depth 0) */
            {
                let mut offbaseFound: usize = 999999999;
                let ml2: usize = ZSTD_searchMax(
                    ms,
                    ip,
                    iend,
                    &mut offbaseFound,
                    mls,
                    rowLog,
                    searchMethod,
                    dictMode,
                );
                if ml2 > matchLength {
                    matchLength = ml2;
                    start = ip;
                    offBase = offbaseFound;
                }
            }

            if matchLength < 4 {
                /* jump faster over incompressible sections */
                let step: usize = (pdiff(ip, anchor) >> kSearchStrength) + 1;
                ip = padd(ip, step);
                (*ms).lazySkipping = (step > kLazySkippingStep) as c_int;
                continue 'main;
            }

            /* let's try to find a better solution */
            if depth >= 1 {
                'depth: while ip < ilimit {
                    ip = padd(ip, 1);
                    if (dictMode == ZSTD_noDict)
                        && (offBase != 0)
                        && (offset_1 > 0)
                        && (rd32(ip) == rd32(psub(ip, offset_1 as usize)))
                    {
                        let mlRep: usize =
                            ZSTD_count(padd(ip, 4), psub(padd(ip, 4), offset_1 as usize), iend) + 4;
                        let gain2: c_int = (mlRep.wrapping_mul(3)) as c_int;
                        let gain1: c_int = (matchLength
                            .wrapping_mul(3)
                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                            .wrapping_add(1)) as c_int;
                        if (mlRep >= 4) && (gain2 > gain1) {
                            matchLength = mlRep;
                            offBase = REPCODE1_TO_OFFBASE as usize;
                            start = ip;
                        }
                    }
                    if isDxS {
                        let repIndex: U32 = (pdiff(ip, base) as U32).wrapping_sub(offset_1);
                        let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                            padd(dictBase, repIndex.wrapping_sub(dictIndexDelta) as usize)
                        } else {
                            padd(base, repIndex as usize)
                        };
                        if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                            && (rd32(repMatch) == rd32(ip))
                        {
                            let repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                                dictEnd
                            } else {
                                iend
                            };
                            let mlRep: usize = ZSTD_count_2segments(
                                padd(ip, 4),
                                padd(repMatch, 4),
                                iend,
                                repMatchEnd,
                                prefixLowest,
                            ) + 4;
                            let gain2: c_int = (mlRep.wrapping_mul(3)) as c_int;
                            let gain1: c_int = (matchLength
                                .wrapping_mul(3)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                .wrapping_add(1)) as c_int;
                            if (mlRep >= 4) && (gain2 > gain1) {
                                matchLength = mlRep;
                                offBase = REPCODE1_TO_OFFBASE as usize;
                                start = ip;
                            }
                        }
                    }
                    {
                        let mut ofbCandidate: usize = 999999999;
                        let ml2: usize = ZSTD_searchMax(
                            ms,
                            ip,
                            iend,
                            &mut ofbCandidate,
                            mls,
                            rowLog,
                            searchMethod,
                            dictMode,
                        );
                        /* raw approx */
                        let gain2: c_int = (ml2
                            .wrapping_mul(4)
                            .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as usize))
                            as c_int;
                        let gain1: c_int = (matchLength
                            .wrapping_mul(4)
                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                            .wrapping_add(4)) as c_int;
                        if (ml2 >= 4) && (gain2 > gain1) {
                            matchLength = ml2;
                            offBase = ofbCandidate;
                            start = ip;
                            continue 'depth; /* search a better one */
                        }
                    }

                    /* let's find an even better one */
                    if (depth == 2) && (ip < ilimit) {
                        ip = padd(ip, 1);
                        if (dictMode == ZSTD_noDict)
                            && (offBase != 0)
                            && (offset_1 > 0)
                            && (rd32(ip) == rd32(psub(ip, offset_1 as usize)))
                        {
                            let mlRep: usize =
                                ZSTD_count(padd(ip, 4), psub(padd(ip, 4), offset_1 as usize), iend)
                                    + 4;
                            let gain2: c_int = (mlRep.wrapping_mul(4)) as c_int;
                            let gain1: c_int = (matchLength
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                .wrapping_add(1)) as c_int;
                            if (mlRep >= 4) && (gain2 > gain1) {
                                matchLength = mlRep;
                                offBase = REPCODE1_TO_OFFBASE as usize;
                                start = ip;
                            }
                        }
                        if isDxS {
                            let repIndex: U32 = (pdiff(ip, base) as U32).wrapping_sub(offset_1);
                            let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                                padd(dictBase, repIndex.wrapping_sub(dictIndexDelta) as usize)
                            } else {
                                padd(base, repIndex as usize)
                            };
                            if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                                && (rd32(repMatch) == rd32(ip))
                            {
                                let repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                                    dictEnd
                                } else {
                                    iend
                                };
                                let mlRep: usize = ZSTD_count_2segments(
                                    padd(ip, 4),
                                    padd(repMatch, 4),
                                    iend,
                                    repMatchEnd,
                                    prefixLowest,
                                ) + 4;
                                let gain2: c_int = (mlRep.wrapping_mul(4)) as c_int;
                                let gain1: c_int = (matchLength
                                    .wrapping_mul(4)
                                    .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                    .wrapping_add(1)) as c_int;
                                if (mlRep >= 4) && (gain2 > gain1) {
                                    matchLength = mlRep;
                                    offBase = REPCODE1_TO_OFFBASE as usize;
                                    start = ip;
                                }
                            }
                        }
                        {
                            let mut ofbCandidate: usize = 999999999;
                            let ml2: usize = ZSTD_searchMax(
                                ms,
                                ip,
                                iend,
                                &mut ofbCandidate,
                                mls,
                                rowLog,
                                searchMethod,
                                dictMode,
                            );
                            /* raw approx */
                            let gain2: c_int = (ml2
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as usize))
                                as c_int;
                            let gain1: c_int = (matchLength
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                .wrapping_add(7)) as c_int;
                            if (ml2 >= 4) && (gain2 > gain1) {
                                matchLength = ml2;
                                offBase = ofbCandidate;
                                start = ip;
                                continue 'depth;
                            }
                        }
                    }
                    break; /* nothing found : store previous solution */
                }
            }

            /* catch up */
            if OFFBASE_IS_OFFSET(offBase as U32) {
                if dictMode == ZSTD_noDict {
                    while ((start > anchor)
                        && (psub(start, offBase.wrapping_sub(ZSTD_REP_NUM)) > prefixLowest))
                        && (*psub(start, 1)
                            == *psub(psub(start, offBase.wrapping_sub(ZSTD_REP_NUM)), 1))
                    {
                        start = psub(start, 1);
                        matchLength += 1;
                    }
                }
                if isDxS {
                    let matchIndex: U32 = (pdiff(start, base)
                        .wrapping_sub(offBase.wrapping_sub(ZSTD_REP_NUM)))
                        as U32;
                    let mut match_: *const BYTE = if matchIndex < prefixLowestIndex {
                        psub(padd(dictBase, matchIndex as usize), dictIndexDelta as usize)
                    } else {
                        padd(base, matchIndex as usize)
                    };
                    let mStart: *const BYTE = if matchIndex < prefixLowestIndex {
                        dictLowest
                    } else {
                        prefixLowest
                    };
                    while (start > anchor) && (match_ > mStart) && (*psub(start, 1) == *psub(match_, 1))
                    {
                        start = psub(start, 1);
                        match_ = psub(match_, 1);
                        matchLength += 1;
                    }
                }
                offset_2 = offset_1;
                offset_1 = OFFBASE_TO_OFFSET(offBase as U32);
            }
        }
        /* store sequence (_storeSequence) */
        {
            let litLength: usize = pdiff(start, anchor);
            ZSTD_storeSeq(
                seqStore,
                litLength,
                anchor,
                iend,
                offBase as U32,
                matchLength,
            );
            ip = padd(start, matchLength);
            anchor = ip;
        }
        if (*ms).lazySkipping != 0 {
            /* We've found a match, disable lazy skipping mode, and refill the hash cache. */
            if searchMethod == search_rowHash {
                ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
            }
            (*ms).lazySkipping = 0;
        }

        /* check immediate repcode */
        if isDxS {
            while ip <= ilimit {
                let current2: U32 = pdiff(ip, base) as U32;
                let repIndex: U32 = current2.wrapping_sub(offset_2);
                let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                    padd(psub(dictBase, dictIndexDelta as usize), repIndex as usize)
                } else {
                    padd(base, repIndex as usize)
                };
                if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                    && (rd32(repMatch) == rd32(ip))
                {
                    let repEnd2: *const BYTE = if repIndex < prefixLowestIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    matchLength = ZSTD_count_2segments(
                        padd(ip, 4),
                        padd(repMatch, 4),
                        iend,
                        repEnd2,
                        prefixLowest,
                    ) + 4;
                    /* swap offset_2 <=> offset_1 */
                    offBase = offset_2 as usize;
                    offset_2 = offset_1;
                    offset_1 = offBase as U32;
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
                    ip = padd(ip, matchLength);
                    anchor = ip;
                    continue;
                }
                break;
            }
        }

        if dictMode == ZSTD_noDict {
            while ((ip <= ilimit) && (offset_2 > 0))
                && (rd32(ip) == rd32(psub(ip, offset_2 as usize)))
            {
                /* store sequence */
                matchLength =
                    ZSTD_count(padd(ip, 4), psub(padd(ip, 4), offset_2 as usize), iend) + 4;
                /* swap repcodes */
                offBase = offset_2 as usize;
                offset_2 = offset_1;
                offset_1 = offBase as U32;
                ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
                ip = padd(ip, matchLength);
                anchor = ip;
                /* faster when present ... (?) */
            }
        }
    }

    /* If offset_1 started invalid (offsetSaved1 != 0) and became valid (offset_1 != 0),
     * rotate saved offsets. */
    offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
        offsetSaved1
    } else {
        offsetSaved2
    };

    /* save reps for next block */
    *rep = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
    *rep.add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };

    /* Return the last literals size */
    pdiff(iend, anchor)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        0,
        ZSTD_noDict,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        0,
        ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        0,
        ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        0,
        ZSTD_noDict,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        0,
        ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        0,
        ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        1,
        ZSTD_noDict,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        1,
        ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        1,
        ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        1,
        ZSTD_noDict,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        1,
        ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        1,
        ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        2,
        ZSTD_noDict,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        2,
        ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        2,
        ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        2,
        ZSTD_noDict,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        2,
        ZSTD_dictMatchState,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        2,
        ZSTD_dedicatedDictSearch,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_binaryTree,
        2,
        ZSTD_noDict,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_binaryTree,
        2,
        ZSTD_dictMatchState,
    )
}

unsafe fn ZSTD_compressBlock_lazy_extDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    searchMethod: searchMethod_e,
    depth: U32,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = padd(istart, srcSize);
    let ilimit: *const BYTE = if searchMethod == search_rowHash {
        psub(iend, 8 + ZSTD_ROW_HASH_CACHE_SIZE)
    } else {
        psub(iend, 8)
    };
    let base: *const BYTE = (*ms).window.base;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = padd(base, dictLimit as usize);
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictEnd: *const BYTE = padd(dictBase, dictLimit as usize);
    let dictStart: *const BYTE = padd(dictBase, (*ms).window.lowLimit as usize);
    let windowLog: U32 = (*ms).cParams.windowLog;
    let mls: U32 = bounded_u32(4, (*ms).cParams.minMatch, 6);
    let rowLog: U32 = bounded_u32(4, (*ms).cParams.searchLog, 6);

    let mut offset_1: U32 = *rep;
    let mut offset_2: U32 = *rep.add(1);

    /* Reset the lazy skipping state */
    (*ms).lazySkipping = 0;

    /* init */
    ip = padd(ip, (ip == prefixStart) as usize);
    if searchMethod == search_rowHash {
        ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
    }

    /* Match Loop */
    'main: while ip < ilimit {
        let mut matchLength: usize = 0;
        let mut offBase: usize = REPCODE1_TO_OFFBASE as usize;
        let mut start: *const BYTE = padd(ip, 1);
        let mut curr: U32 = pdiff(ip, base) as U32;

        'store: {
            /* check repCode */
            {
                let windowLow: U32 =
                    ZSTD_getLowestMatchIndex(ms, curr.wrapping_add(1), windowLog);
                let repIndex: U32 = curr.wrapping_add(1).wrapping_sub(offset_1);
                let repBase: *const BYTE = if repIndex < dictLimit { dictBase } else { base };
                let repMatch: *const BYTE = padd(repBase, repIndex as usize);
                if (ZSTD_index_overlap_check(dictLimit, repIndex)
                    & ((offset_1 <= curr.wrapping_add(1).wrapping_sub(windowLow)) as c_int))
                    != 0
                {
                    /* note: we are searching at curr+1 */
                    if rd32(padd(ip, 1)) == rd32(repMatch) {
                        /* repcode detected we should take it */
                        let repEnd: *const BYTE = if repIndex < dictLimit { dictEnd } else { iend };
                        matchLength = ZSTD_count_2segments(
                            padd(ip, 1 + 4),
                            padd(repMatch, 4),
                            iend,
                            repEnd,
                            prefixStart,
                        ) + 4;
                        if depth == 0 {
                            break 'store;
                        }
                    }
                }
            }

            /* first search (depth 0) */
            {
                let mut ofbCandidate: usize = 999999999;
                let ml2: usize = ZSTD_searchMax(
                    ms,
                    ip,
                    iend,
                    &mut ofbCandidate,
                    mls,
                    rowLog,
                    searchMethod,
                    ZSTD_extDict,
                );
                if ml2 > matchLength {
                    matchLength = ml2;
                    start = ip;
                    offBase = ofbCandidate;
                }
            }

            if matchLength < 4 {
                let step: usize = pdiff(ip, anchor) >> kSearchStrength;
                /* jump faster over incompressible sections */
                ip = padd(ip, step + 1);
                (*ms).lazySkipping = (step > kLazySkippingStep) as c_int;
                continue 'main;
            }

            /* let's try to find a better solution */
            if depth >= 1 {
                'depth: while ip < ilimit {
                    ip = padd(ip, 1);
                    curr = curr.wrapping_add(1);
                    /* check repCode */
                    if offBase != 0 {
                        let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, windowLog);
                        let repIndex: U32 = curr.wrapping_sub(offset_1);
                        let repBase: *const BYTE =
                            if repIndex < dictLimit { dictBase } else { base };
                        let repMatch: *const BYTE = padd(repBase, repIndex as usize);
                        if (ZSTD_index_overlap_check(dictLimit, repIndex)
                            & ((offset_1 <= curr.wrapping_sub(windowLow)) as c_int))
                            != 0
                        {
                            if rd32(ip) == rd32(repMatch) {
                                /* repcode detected */
                                let repEnd: *const BYTE =
                                    if repIndex < dictLimit { dictEnd } else { iend };
                                let repLength: usize = ZSTD_count_2segments(
                                    padd(ip, 4),
                                    padd(repMatch, 4),
                                    iend,
                                    repEnd,
                                    prefixStart,
                                ) + 4;
                                let gain2: c_int = (repLength.wrapping_mul(3)) as c_int;
                                let gain1: c_int = (matchLength
                                    .wrapping_mul(3)
                                    .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                    .wrapping_add(1)) as c_int;
                                if (repLength >= 4) && (gain2 > gain1) {
                                    matchLength = repLength;
                                    offBase = REPCODE1_TO_OFFBASE as usize;
                                    start = ip;
                                }
                            }
                        }
                    }

                    /* search match, depth 1 */
                    {
                        let mut ofbCandidate: usize = 999999999;
                        let ml2: usize = ZSTD_searchMax(
                            ms,
                            ip,
                            iend,
                            &mut ofbCandidate,
                            mls,
                            rowLog,
                            searchMethod,
                            ZSTD_extDict,
                        );
                        /* raw approx */
                        let gain2: c_int = (ml2
                            .wrapping_mul(4)
                            .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as usize))
                            as c_int;
                        let gain1: c_int = (matchLength
                            .wrapping_mul(4)
                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                            .wrapping_add(4)) as c_int;
                        if (ml2 >= 4) && (gain2 > gain1) {
                            matchLength = ml2;
                            offBase = ofbCandidate;
                            start = ip;
                            continue 'depth; /* search a better one */
                        }
                    }

                    /* let's find an even better one */
                    if (depth == 2) && (ip < ilimit) {
                        ip = padd(ip, 1);
                        curr = curr.wrapping_add(1);
                        /* check repCode */
                        if offBase != 0 {
                            let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, windowLog);
                            let repIndex: U32 = curr.wrapping_sub(offset_1);
                            let repBase: *const BYTE =
                                if repIndex < dictLimit { dictBase } else { base };
                            let repMatch: *const BYTE = padd(repBase, repIndex as usize);
                            if (ZSTD_index_overlap_check(dictLimit, repIndex)
                                & ((offset_1 <= curr.wrapping_sub(windowLow)) as c_int))
                                != 0
                            {
                                if rd32(ip) == rd32(repMatch) {
                                    /* repcode detected */
                                    let repEnd: *const BYTE =
                                        if repIndex < dictLimit { dictEnd } else { iend };
                                    let repLength: usize = ZSTD_count_2segments(
                                        padd(ip, 4),
                                        padd(repMatch, 4),
                                        iend,
                                        repEnd,
                                        prefixStart,
                                    ) + 4;
                                    let gain2: c_int = (repLength.wrapping_mul(4)) as c_int;
                                    let gain1: c_int = (matchLength
                                        .wrapping_mul(4)
                                        .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                        .wrapping_add(1)) as c_int;
                                    if (repLength >= 4) && (gain2 > gain1) {
                                        matchLength = repLength;
                                        offBase = REPCODE1_TO_OFFBASE as usize;
                                        start = ip;
                                    }
                                }
                            }
                        }

                        /* search match, depth 2 */
                        {
                            let mut ofbCandidate: usize = 999999999;
                            let ml2: usize = ZSTD_searchMax(
                                ms,
                                ip,
                                iend,
                                &mut ofbCandidate,
                                mls,
                                rowLog,
                                searchMethod,
                                ZSTD_extDict,
                            );
                            /* raw approx */
                            let gain2: c_int = (ml2
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as usize))
                                as c_int;
                            let gain1: c_int = (matchLength
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                .wrapping_add(7)) as c_int;
                            if (ml2 >= 4) && (gain2 > gain1) {
                                matchLength = ml2;
                                offBase = ofbCandidate;
                                start = ip;
                                continue 'depth;
                            }
                        }
                    }
                    break; /* nothing found : store previous solution */
                }
            }

            /* catch up */
            if OFFBASE_IS_OFFSET(offBase as U32) {
                let matchIndex: U32 =
                    (pdiff(start, base).wrapping_sub(offBase.wrapping_sub(ZSTD_REP_NUM))) as U32;
                let mut match_: *const BYTE = if matchIndex < dictLimit {
                    padd(dictBase, matchIndex as usize)
                } else {
                    padd(base, matchIndex as usize)
                };
                let mStart: *const BYTE = if matchIndex < dictLimit {
                    dictStart
                } else {
                    prefixStart
                };
                while (start > anchor) && (match_ > mStart) && (*psub(start, 1) == *psub(match_, 1))
                {
                    start = psub(start, 1);
                    match_ = psub(match_, 1);
                    matchLength += 1;
                }
                offset_2 = offset_1;
                offset_1 = OFFBASE_TO_OFFSET(offBase as U32);
            }
        }

        /* store sequence (_storeSequence) */
        {
            let litLength: usize = pdiff(start, anchor);
            ZSTD_storeSeq(
                seqStore,
                litLength,
                anchor,
                iend,
                offBase as U32,
                matchLength,
            );
            ip = padd(start, matchLength);
            anchor = ip;
        }
        if (*ms).lazySkipping != 0 {
            /* We've found a match, disable lazy skipping mode, and refill the hash cache. */
            if searchMethod == search_rowHash {
                ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
            }
            (*ms).lazySkipping = 0;
        }

        /* check immediate repcode */
        while ip <= ilimit {
            let repCurrent: U32 = pdiff(ip, base) as U32;
            let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, repCurrent, windowLog);
            let repIndex: U32 = repCurrent.wrapping_sub(offset_2);
            let repBase: *const BYTE = if repIndex < dictLimit { dictBase } else { base };
            let repMatch: *const BYTE = padd(repBase, repIndex as usize);
            if (ZSTD_index_overlap_check(dictLimit, repIndex)
                & ((offset_2 <= repCurrent.wrapping_sub(windowLow)) as c_int))
                != 0
            {
                if rd32(ip) == rd32(repMatch) {
                    /* repcode detected we should take it */
                    let repEnd: *const BYTE = if repIndex < dictLimit { dictEnd } else { iend };
                    matchLength = ZSTD_count_2segments(
                        padd(ip, 4),
                        padd(repMatch, 4),
                        iend,
                        repEnd,
                        prefixStart,
                    ) + 4;
                    /* swap offset history */
                    offBase = offset_2 as usize;
                    offset_2 = offset_1;
                    offset_1 = offBase as U32;
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
                    ip = padd(ip, matchLength);
                    anchor = ip;
                    continue; /* faster when present ... (?) */
                }
            }
            break;
        }
    }

    /* Save reps for next block */
    *rep = offset_1;
    *rep.add(1) = offset_2;

    /* Return the last literals size */
    pdiff(iend, anchor)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_extDict_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        1,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        2,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_extDict_row(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_binaryTree,
        2,
    )
}
