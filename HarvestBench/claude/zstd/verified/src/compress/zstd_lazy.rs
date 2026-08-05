//! Translation of compress/zstd_lazy.c
//! greedy / lazy / lazy2 / btlazy2 block compressors + match finders.

use core::ffi::c_void;

use crate::common::bits::{
    count_trailing_zeros64 as ZSTD_countTrailingZeros64, highbit32 as ZSTD_highbit32,
    rotate_right_u16 as ZSTD_rotateRight_U16, rotate_right_u32 as ZSTD_rotateRight_U32,
    rotate_right_u64 as ZSTD_rotateRight_U64,
};
use crate::common::mem::{
    mem_is_little_endian as MEM_isLittleEndian, mem_read32 as MEM_read32,
    mem_read_st as MEM_readST, U16, U32, U64,
};
use crate::common::zstd_internal::MINMATCH;
use crate::compress::zstd_compress_internal::*;

type BYTE = u8;

/* From zstd_lazy.h */
const ZSTD_LAZY_DDSS_BUCKET_LOG: u32 = 2;
const ZSTD_ROW_HASH_TAG_BITS: u32 = 8;

const kLazySkippingStep: U32 = 8;

/* Constants for row-based hash */
const ZSTD_ROW_HASH_TAG_MASK: u32 = (1u32 << ZSTD_ROW_HASH_TAG_BITS) - 1;
const ZSTD_ROW_HASH_MAX_ENTRIES: usize = 64;
const ZSTD_ROW_HASH_CACHE_MASK: u32 = (ZSTD_ROW_HASH_CACHE_SIZE as u32) - 1;

/* searchMethod_e */
type searchMethod_e = u32;
const search_hashChain: searchMethod_e = 0;
const search_binaryTree: searchMethod_e = 1;
const search_rowHash: searchMethod_e = 2;

/* PREFETCH is disabled in this build configuration (hint only; no effect on output). */
#[inline(always)]
unsafe fn PREFETCH_L1<T>(_ptr: *const T) {}

#[inline(always)]
fn MIN_usize(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}
#[inline(always)]
fn MIN_u32(a: U32, b: U32) -> U32 {
    if a < b { a } else { b }
}
#[inline(always)]
fn MAX_u32(a: U32, b: U32) -> U32 {
    if a > b { a } else { b }
}
#[inline(always)]
fn BOUNDED_u32(min: U32, val: U32, max: U32) -> U32 {
    MAX_u32(min, MIN_u32(val, max))
}

/*-*************************************
*  Binary Tree search
***************************************/

unsafe fn ZSTD_updateDUBT(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    mls: U32,
) {
    let cParams = &(*ms).cParams;
    let hashTable = (*ms).hashTable;
    let hashLog = cParams.hashLog;

    let bt = (*ms).chainTable;
    let btLog = cParams.chainLog - 1;
    let btMask = (1u32 << btLog) - 1;

    let base = (*ms).window.base;
    let target = ip.offset_from(base) as U32;
    let mut idx = (*ms).nextToUpdate;

    let _ = iend;

    while idx < target {
        let h = ZSTD_hashPtr(base.add(idx as usize) as *const c_void, hashLog, mls);
        let matchIndex = *hashTable.add(h);

        let nextCandidatePtr = bt.add((2 * (idx & btMask)) as usize);
        let sortMarkPtr = nextCandidatePtr.add(1);

        *hashTable.add(h) = idx;
        *nextCandidatePtr = matchIndex;
        *sortMarkPtr = ZSTD_DUBT_UNSORTED_MARK;
        idx += 1;
    }
    (*ms).nextToUpdate = target;
}

/** ZSTD_insertDUBT1() :
 *  sort one already inserted but unsorted position
 *  assumption : curr >= btlow == (curr - btmask)
 *  doesn't fail */
unsafe fn ZSTD_insertDUBT1(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    inputEnd: *const BYTE,
    mut nbCompares: U32,
    btLow: U32,
    dictMode: ZSTD_dictMode_e,
) {
    let cParams = &(*ms).cParams;
    let bt = (*ms).chainTable;
    let btLog = cParams.chainLog - 1;
    let btMask = (1u32 << btLog) - 1;
    let mut commonLengthSmaller: usize = 0;
    let mut commonLengthLarger: usize = 0;
    let base = (*ms).window.base;
    let dictBase = (*ms).window.dictBase;
    let dictLimit = (*ms).window.dictLimit;
    let ip = if curr >= dictLimit {
        base.add(curr as usize)
    } else {
        dictBase.add(curr as usize)
    };
    let iend = if curr >= dictLimit {
        inputEnd
    } else {
        dictBase.add(dictLimit as usize)
    };
    let dictEnd = dictBase.add(dictLimit as usize);
    let prefixStart = base.add(dictLimit as usize);
    let mut r#match: *const BYTE;
    let mut smallerPtr = bt.add((2 * (curr & btMask)) as usize);
    let mut largerPtr = smallerPtr.add(1);
    let mut matchIndex = *smallerPtr;
    let mut dummy32: U32 = 0;
    let windowValid = (*ms).window.lowLimit;
    let maxDistance = 1u32 << cParams.windowLog;
    let windowLow = if curr.wrapping_sub(windowValid) > maxDistance {
        curr - maxDistance
    } else {
        windowValid
    };

    while nbCompares != 0 && (matchIndex > windowLow) {
        let nextPtr = bt.add((2 * (matchIndex & btMask)) as usize);
        let mut matchLength = MIN_usize(commonLengthSmaller, commonLengthLarger);

        if (dictMode != ZSTD_extDict)
            || (matchIndex as usize + matchLength >= dictLimit as usize)
            || (curr < dictLimit)
        {
            let mBase = if (dictMode != ZSTD_extDict)
                || (matchIndex as usize + matchLength >= dictLimit as usize)
            {
                base
            } else {
                dictBase
            };
            r#match = mBase.add(matchIndex as usize);
            matchLength += ZSTD_count(ip.add(matchLength), r#match.add(matchLength), iend);
        } else {
            r#match = dictBase.add(matchIndex as usize);
            matchLength += ZSTD_count_2segments(
                ip.add(matchLength),
                r#match.add(matchLength),
                iend,
                dictEnd,
                prefixStart,
            );
            if matchIndex as usize + matchLength >= dictLimit as usize {
                r#match = base.add(matchIndex as usize);
            }
        }

        if ip.add(matchLength) == iend {
            break;
        }

        if *r#match.add(matchLength) < *ip.add(matchLength) {
            /* match is smaller than current */
            *smallerPtr = matchIndex;
            commonLengthSmaller = matchLength;
            if matchIndex <= btLow {
                smallerPtr = &mut dummy32;
                break;
            }
            smallerPtr = nextPtr.add(1);
            matchIndex = *nextPtr.add(1);
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

unsafe fn ZSTD_DUBT_findBetterDictMatch(
    ms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    offsetPtr: *mut usize,
    mut bestLength: usize,
    mut nbCompares: U32,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    let dms = (*ms).dictMatchState;
    let dmsCParams = &(*dms).cParams;
    let dictHashTable = (*dms).hashTable;
    let hashLog = dmsCParams.hashLog;
    let h = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut dictMatchIndex = *dictHashTable.add(h);

    let base = (*ms).window.base;
    let prefixStart = base.add((*ms).window.dictLimit as usize);
    let curr = ip.offset_from(base) as U32;
    let dictBase = (*dms).window.base;
    let dictEnd = (*dms).window.nextSrc;
    let dictHighLimit = ((*dms).window.nextSrc.offset_from((*dms).window.base)) as U32;
    let dictLowLimit = (*dms).window.lowLimit;
    let dictIndexDelta = (*ms).window.lowLimit.wrapping_sub(dictHighLimit);

    let dictBt = (*dms).chainTable;
    let btLog = dmsCParams.chainLog - 1;
    let btMask = (1u32 << btLog) - 1;
    let btLow = if btMask >= dictHighLimit - dictLowLimit {
        dictLowLimit
    } else {
        dictHighLimit - btMask
    };

    let mut commonLengthSmaller: usize = 0;
    let mut commonLengthLarger: usize = 0;

    let _ = dictMode;
    debug_assert!(dictMode == ZSTD_dictMatchState);

    while nbCompares != 0 && (dictMatchIndex > dictLowLimit) {
        let nextPtr = dictBt.add((2 * (dictMatchIndex & btMask)) as usize);
        let mut matchLength = MIN_usize(commonLengthSmaller, commonLengthLarger);
        let mut r#match = dictBase.add(dictMatchIndex as usize);
        matchLength += ZSTD_count_2segments(
            ip.add(matchLength),
            r#match.add(matchLength),
            iend,
            dictEnd,
            prefixStart,
        );
        if dictMatchIndex as usize + matchLength >= dictHighLimit as usize {
            r#match = base.add((dictMatchIndex + dictIndexDelta) as usize);
        }

        if matchLength > bestLength {
            let matchIndex = dictMatchIndex.wrapping_add(dictIndexDelta);
            if (4 * (matchLength as i32 - bestLength as i32))
                > (ZSTD_highbit32(curr - matchIndex + 1) as i32
                    - ZSTD_highbit32(*offsetPtr as U32 + 1) as i32)
            {
                bestLength = matchLength;
                *offsetPtr = OFFSET_TO_OFFBASE(curr - matchIndex) as usize;
            }
            if ip.add(matchLength) == iend {
                break;
            }
        }

        if *r#match.add(matchLength) < *ip.add(matchLength) {
            if dictMatchIndex <= btLow {
                break;
            }
            commonLengthSmaller = matchLength;
            dictMatchIndex = *nextPtr.add(1);
        } else {
            if dictMatchIndex <= btLow {
                break;
            }
            commonLengthLarger = matchLength;
            dictMatchIndex = *nextPtr;
        }
        nbCompares -= 1;
    }

    if bestLength >= MINMATCH as usize {
        let mIndex = curr - OFFBASE_TO_OFFSET(*offsetPtr as U32);
        let _ = mIndex;
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
    let cParams = &(*ms).cParams;
    let hashTable = (*ms).hashTable;
    let hashLog = cParams.hashLog;
    let h = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut matchIndex = *hashTable.add(h);

    let base = (*ms).window.base;
    let curr = ip.offset_from(base) as U32;
    let windowLow = ZSTD_getLowestMatchIndex(ms, curr, cParams.windowLog);

    let bt = (*ms).chainTable;
    let btLog = cParams.chainLog - 1;
    let btMask = (1u32 << btLog) - 1;
    let btLow = if btMask >= curr { 0 } else { curr - btMask };
    let unsortLimit = MAX_u32(btLow, windowLow);

    let mut nextCandidate = bt.add((2 * (matchIndex & btMask)) as usize);
    let mut unsortedMark = bt.add((2 * (matchIndex & btMask) + 1) as usize);
    let mut nbCompares = 1u32 << cParams.searchLog;
    let mut nbCandidates = nbCompares;
    let mut previousCandidate: U32 = 0;

    /* reach end of unsorted candidates list */
    while (matchIndex > unsortLimit)
        && (*unsortedMark == ZSTD_DUBT_UNSORTED_MARK)
        && (nbCandidates > 1)
    {
        *unsortedMark = previousCandidate;
        previousCandidate = matchIndex;
        matchIndex = *nextCandidate;
        nextCandidate = bt.add((2 * (matchIndex & btMask)) as usize);
        unsortedMark = bt.add((2 * (matchIndex & btMask) + 1) as usize);
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
        let nextCandidateIdxPtr = bt.add((2 * (matchIndex & btMask) + 1) as usize);
        let nextCandidateIdx = *nextCandidateIdxPtr;
        ZSTD_insertDUBT1(ms, matchIndex, iend, nbCandidates, unsortLimit, dictMode);
        matchIndex = nextCandidateIdx;
        nbCandidates += 1;
    }

    /* find longest match */
    {
        let mut commonLengthSmaller: usize = 0;
        let mut commonLengthLarger: usize = 0;
        let dictBase = (*ms).window.dictBase;
        let dictLimit = (*ms).window.dictLimit;
        let dictEnd = dictBase.add(dictLimit as usize);
        let prefixStart = base.add(dictLimit as usize);
        let mut smallerPtr = bt.add((2 * (curr & btMask)) as usize);
        let mut largerPtr = bt.add((2 * (curr & btMask) + 1) as usize);
        let mut matchEndIdx = curr + 8 + 1;
        let mut dummy32: U32 = 0;
        let mut bestLength: usize = 0;

        matchIndex = *hashTable.add(h);
        *hashTable.add(h) = curr;

        while nbCompares != 0 && (matchIndex > windowLow) {
            let nextPtr = bt.add((2 * (matchIndex & btMask)) as usize);
            let mut matchLength = MIN_usize(commonLengthSmaller, commonLengthLarger);
            let r#match: *const BYTE;

            if (dictMode != ZSTD_extDict)
                || (matchIndex as usize + matchLength >= dictLimit as usize)
            {
                r#match = base.add(matchIndex as usize);
                matchLength += ZSTD_count(ip.add(matchLength), r#match.add(matchLength), iend);
            } else {
                let mut m = dictBase.add(matchIndex as usize);
                matchLength += ZSTD_count_2segments(
                    ip.add(matchLength),
                    m.add(matchLength),
                    iend,
                    dictEnd,
                    prefixStart,
                );
                if matchIndex as usize + matchLength >= dictLimit as usize {
                    m = base.add(matchIndex as usize);
                }
                r#match = m;
            }

            if matchLength > bestLength {
                if matchLength as U32 > matchEndIdx.wrapping_sub(matchIndex) {
                    matchEndIdx = matchIndex + matchLength as U32;
                }
                if (4 * (matchLength as i32 - bestLength as i32))
                    > (ZSTD_highbit32(curr - matchIndex + 1) as i32
                        - ZSTD_highbit32(*offBasePtr as U32) as i32)
                {
                    bestLength = matchLength;
                    *offBasePtr = OFFSET_TO_OFFBASE(curr - matchIndex) as usize;
                }
                if ip.add(matchLength) == iend {
                    if dictMode == ZSTD_dictMatchState {
                        nbCompares = 0;
                    }
                    break;
                }
            }

            if *r#match.add(matchLength) < *ip.add(matchLength) {
                /* match is smaller than current */
                *smallerPtr = matchIndex;
                commonLengthSmaller = matchLength;
                if matchIndex <= btLow {
                    smallerPtr = &mut dummy32;
                    break;
                }
                smallerPtr = nextPtr.add(1);
                matchIndex = *nextPtr.add(1);
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

        if dictMode == ZSTD_dictMatchState && nbCompares != 0 {
            bestLength = ZSTD_DUBT_findBetterDictMatch(
                ms, ip, iend, offBasePtr, bestLength, nbCompares, mls, dictMode,
            );
        }

        debug_assert!(matchEndIdx > curr + 8);
        (*ms).nextToUpdate = matchEndIdx - 8;
        if bestLength >= MINMATCH as usize {
            let mIndex = curr - OFFBASE_TO_OFFSET(*offBasePtr as U32);
            let _ = mIndex;
        }
        bestLength
    }
}

/** ZSTD_BtFindBestMatch() : Tree updater, providing best match */
#[inline(always)]
unsafe fn ZSTD_BtFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offBasePtr: *mut usize,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    if ip < (*ms).window.base.add((*ms).nextToUpdate as usize) {
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
    let base = (*ms).window.base;
    let target = ip.offset_from(base) as U32;
    let hashTable = (*ms).hashTable;
    let chainTable = (*ms).chainTable;
    let chainSize = 1u32 << (*ms).cParams.chainLog;
    let mut idx = (*ms).nextToUpdate;
    let minChain = if chainSize < target - idx {
        target - chainSize
    } else {
        idx
    };
    let bucketSize = 1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG;
    let cacheSize = bucketSize - 1;
    let chainAttempts = (1u32 << (*ms).cParams.searchLog) - cacheSize;
    let chainLimit = if chainAttempts > 255 { 255 } else { chainAttempts };

    let hashLog = (*ms).cParams.hashLog - ZSTD_LAZY_DDSS_BUCKET_LOG;
    let tmpHashTable = hashTable;
    let tmpChainTable = hashTable.add(1usize << hashLog);
    let tmpChainSize = ((1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG) - 1) << hashLog;
    let tmpMinChain = if tmpChainSize < target {
        target - tmpChainSize
    } else {
        idx
    };
    let mut hashIdx: U32;

    /* fill conventional hash table and conventional chain table */
    while idx < target {
        let h = ZSTD_hashPtr(base.add(idx as usize) as *const c_void, hashLog, (*ms).cParams.minMatch) as U32;
        if idx >= tmpMinChain {
            *tmpChainTable.add((idx - tmpMinChain) as usize) = *hashTable.add(h as usize);
        }
        *tmpHashTable.add(h as usize) = idx;
        idx += 1;
    }

    /* sort chains into ddss chain table */
    {
        let mut chainPos: U32 = 0;
        hashIdx = 0;
        while hashIdx < (1u32 << hashLog) {
            let mut count: U32;
            let mut countBeyondMinChain: U32 = 0;
            let mut i = *tmpHashTable.add(hashIdx as usize);
            count = 0;
            while i >= tmpMinChain && count < cacheSize {
                if i < minChain {
                    countBeyondMinChain += 1;
                }
                i = *tmpChainTable.add((i - tmpMinChain) as usize);
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
                    *chainTable.add(chainPos as usize) = i;
                    chainPos += 1;
                    count += 1;
                    if i < tmpMinChain {
                        break;
                    }
                    i = *tmpChainTable.add((i - tmpMinChain) as usize);
                }
            } else {
                count = 0;
            }
            if count != 0 {
                *tmpHashTable.add(hashIdx as usize) = ((chainPos - count) << 8) + count;
            } else {
                *tmpHashTable.add(hashIdx as usize) = 0;
            }
            hashIdx += 1;
        }
    }

    /* move chain pointers into the last entry of each hash bucket */
    hashIdx = 1u32 << hashLog;
    while hashIdx != 0 {
        hashIdx -= 1;
        let bucketIdx = hashIdx << ZSTD_LAZY_DDSS_BUCKET_LOG;
        let chainPackedPointer = *tmpHashTable.add(hashIdx as usize);
        let mut i: U32 = 0;
        while i < cacheSize {
            *hashTable.add((bucketIdx + i) as usize) = 0;
            i += 1;
        }
        *hashTable.add((bucketIdx + bucketSize - 1) as usize) = chainPackedPointer;
    }

    /* fill the buckets of the hash table */
    idx = (*ms).nextToUpdate;
    while idx < target {
        let h = (ZSTD_hashPtr(base.add(idx as usize) as *const c_void, hashLog, (*ms).cParams.minMatch) as U32)
            << ZSTD_LAZY_DDSS_BUCKET_LOG;
        let mut i = cacheSize - 1;
        while i != 0 {
            *hashTable.add((h + i) as usize) = *hashTable.add((h + i - 1) as usize);
            i -= 1;
        }
        *hashTable.add(h as usize) = idx;
        idx += 1;
    }

    (*ms).nextToUpdate = target;
}

/* Returns the longest match length found in the dedicated dict search structure.
 * If none are longer than the argument ml, then ml will be returned.
 */
#[inline(always)]
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
    let ddsLowestIndex = (*dms).window.dictLimit;
    let ddsBase = (*dms).window.base;
    let ddsEnd = (*dms).window.nextSrc;
    let ddsSize = ddsEnd.offset_from(ddsBase) as U32;
    let ddsIndexDelta = dictLimit.wrapping_sub(ddsSize);
    let bucketSize = 1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG;
    let bucketLimit = if nbAttempts < bucketSize - 1 {
        nbAttempts
    } else {
        bucketSize - 1
    };
    let mut ddsAttempt: U32;
    let mut matchIndex: U32;

    ddsAttempt = 0;
    while ddsAttempt < bucketSize - 1 {
        PREFETCH_L1(ddsBase.add(*(*dms).hashTable.add(ddsIdx + ddsAttempt as usize) as usize));
        ddsAttempt += 1;
    }

    {
        let chainPackedPointer = *(*dms).hashTable.add(ddsIdx + (bucketSize - 1) as usize);
        let chainIndex = chainPackedPointer >> 8;
        PREFETCH_L1((*dms).chainTable.add(chainIndex as usize));
    }

    ddsAttempt = 0;
    while ddsAttempt < bucketLimit {
        let mut currentMl: usize = 0;
        matchIndex = *(*dms).hashTable.add(ddsIdx + ddsAttempt as usize);
        let r#match = ddsBase.add(matchIndex as usize);

        if matchIndex == 0 {
            return ml;
        }

        let _ = ddsLowestIndex;
        if MEM_read32(r#match as *const c_void) == MEM_read32(ip as *const c_void) {
            currentMl = ZSTD_count_2segments(ip.add(4), r#match.add(4), iLimit, ddsEnd, prefixStart) + 4;
        }

        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = OFFSET_TO_OFFBASE(curr - (matchIndex + ddsIndexDelta)) as usize;
            if ip.add(currentMl) == iLimit {
                return ml;
            }
        }
        ddsAttempt += 1;
    }

    {
        let chainPackedPointer = *(*dms).hashTable.add(ddsIdx + (bucketSize - 1) as usize);
        let mut chainIndex = chainPackedPointer >> 8;
        let chainLength = chainPackedPointer & 0xFF;
        let chainAttempts = nbAttempts - ddsAttempt;
        let chainLimit = if chainAttempts > chainLength {
            chainLength
        } else {
            chainAttempts
        };
        let mut chainAttempt: U32;

        chainAttempt = 0;
        while chainAttempt < chainLimit {
            PREFETCH_L1(ddsBase.add(*(*dms).chainTable.add((chainIndex + chainAttempt) as usize) as usize));
            chainAttempt += 1;
        }

        chainAttempt = 0;
        while chainAttempt < chainLimit {
            let mut currentMl: usize = 0;
            matchIndex = *(*dms).chainTable.add(chainIndex as usize);
            let r#match = ddsBase.add(matchIndex as usize);

            if MEM_read32(r#match as *const c_void) == MEM_read32(ip as *const c_void) {
                currentMl = ZSTD_count_2segments(ip.add(4), r#match.add(4), iLimit, ddsEnd, prefixStart) + 4;
            }

            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(curr - (matchIndex + ddsIndexDelta)) as usize;
                if ip.add(currentMl) == iLimit {
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

/* Update chains up to ip (excluded)
   Assumption : always within prefix (i.e. not within extDict) */
#[inline(always)]
unsafe fn ZSTD_insertAndFindFirstIndex_internal(
    ms: *mut ZSTD_MatchState_t,
    cParams: *const ZSTD_compressionParameters,
    ip: *const BYTE,
    mls: U32,
    lazySkipping: U32,
) -> U32 {
    let hashTable = (*ms).hashTable;
    let hashLog = (*cParams).hashLog;
    let chainTable = (*ms).chainTable;
    let chainMask = (1u32 << (*cParams).chainLog) - 1;
    let base = (*ms).window.base;
    let target = ip.offset_from(base) as U32;
    let mut idx = (*ms).nextToUpdate;

    while idx < target {
        let h = ZSTD_hashPtr(base.add(idx as usize) as *const c_void, hashLog, mls);
        *chainTable.add((idx & chainMask) as usize) = *hashTable.add(h);
        *hashTable.add(h) = idx;
        idx += 1;
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
    let cParams = &(*ms).cParams as *const ZSTD_compressionParameters;
    ZSTD_insertAndFindFirstIndex_internal(ms, cParams, ip, (*ms).cParams.minMatch, 0)
}

/* inlining is important to hardwire a hot branch (template emulation) */
#[inline(always)]
unsafe fn ZSTD_HcFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    let cParams = &(*ms).cParams as *const ZSTD_compressionParameters;
    let chainTable = (*ms).chainTable;
    let chainSize = 1u32 << (*cParams).chainLog;
    let chainMask = chainSize - 1;
    let base = (*ms).window.base;
    let dictBase = (*ms).window.dictBase;
    let dictLimit = (*ms).window.dictLimit;
    let prefixStart = base.add(dictLimit as usize);
    let dictEnd = dictBase.add(dictLimit as usize);
    let curr = ip.offset_from(base) as U32;
    let maxDistance = 1u32 << (*cParams).windowLog;
    let lowestValid = (*ms).window.lowLimit;
    let withinMaxDistance = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr - maxDistance
    } else {
        lowestValid
    };
    let isDictionary = ((*ms).loadedDictEnd != 0) as U32;
    let lowLimit = if isDictionary != 0 {
        lowestValid
    } else {
        withinMaxDistance
    };
    let minChain = if curr > chainSize { curr - chainSize } else { 0 };
    let mut nbAttempts = 1u32 << (*cParams).searchLog;
    let mut ml: usize = 4 - 1;

    let dms = (*ms).dictMatchState;
    let ddsHashLog = if dictMode == ZSTD_dedicatedDictSearch {
        (*dms).cParams.hashLog - ZSTD_LAZY_DDSS_BUCKET_LOG
    } else {
        0
    };
    let ddsIdx = if dictMode == ZSTD_dedicatedDictSearch {
        (ZSTD_hashPtr(ip as *const c_void, ddsHashLog, mls)) << ZSTD_LAZY_DDSS_BUCKET_LOG
    } else {
        0
    };

    let mut matchIndex: U32;

    if dictMode == ZSTD_dedicatedDictSearch {
        let entry = (*dms).hashTable.add(ddsIdx);
        PREFETCH_L1(entry);
    }

    /* HC4 match finder */
    matchIndex = ZSTD_insertAndFindFirstIndex_internal(ms, cParams, ip, mls, (*ms).lazySkipping as U32);

    while (matchIndex >= lowLimit) && (nbAttempts > 0) {
        let mut currentMl: usize = 0;
        if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
            let r#match = base.add(matchIndex as usize);
            if MEM_read32(r#match.add(ml).offset(-3) as *const c_void)
                == MEM_read32(ip.add(ml).offset(-3) as *const c_void)
            {
                currentMl = ZSTD_count(ip, r#match, iLimit);
            }
        } else {
            let r#match = dictBase.add(matchIndex as usize);
            if MEM_read32(r#match as *const c_void) == MEM_read32(ip as *const c_void) {
                currentMl = ZSTD_count_2segments(ip.add(4), r#match.add(4), iLimit, dictEnd, prefixStart) + 4;
            }
        }

        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = OFFSET_TO_OFFBASE(curr - matchIndex) as usize;
            if ip.add(currentMl) == iLimit {
                break;
            }
        }

        if matchIndex <= minChain {
            break;
        }
        matchIndex = *chainTable.add((matchIndex & chainMask) as usize);
        nbAttempts -= 1;
    }

    if dictMode == ZSTD_dedicatedDictSearch {
        ml = ZSTD_dedicatedDictSearch_lazy_search(
            offsetPtr, ml, nbAttempts, dms, ip, iLimit, prefixStart, curr, dictLimit, ddsIdx,
        );
    } else if dictMode == ZSTD_dictMatchState {
        let dmsChainTable = (*dms).chainTable;
        let dmsChainSize = 1u32 << (*dms).cParams.chainLog;
        let dmsChainMask = dmsChainSize - 1;
        let dmsLowestIndex = (*dms).window.dictLimit;
        let dmsBase = (*dms).window.base;
        let dmsEnd = (*dms).window.nextSrc;
        let dmsSize = dmsEnd.offset_from(dmsBase) as U32;
        let dmsIndexDelta = dictLimit.wrapping_sub(dmsSize);
        let dmsMinChain = if dmsSize > dmsChainSize {
            dmsSize - dmsChainSize
        } else {
            0
        };

        matchIndex = *(*dms).hashTable.add(ZSTD_hashPtr(ip as *const c_void, (*dms).cParams.hashLog, mls));

        while (matchIndex >= dmsLowestIndex) && (nbAttempts > 0) {
            let mut currentMl: usize = 0;
            let r#match = dmsBase.add(matchIndex as usize);
            if MEM_read32(r#match as *const c_void) == MEM_read32(ip as *const c_void) {
                currentMl = ZSTD_count_2segments(ip.add(4), r#match.add(4), iLimit, dmsEnd, prefixStart) + 4;
            }

            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(curr - (matchIndex + dmsIndexDelta)) as usize;
                if ip.add(currentMl) == iLimit {
                    break;
                }
            }

            if matchIndex <= dmsMinChain {
                break;
            }
            matchIndex = *dmsChainTable.add((matchIndex & dmsChainMask) as usize);
            nbAttempts -= 1;
        }
    }

    ml
}

/* *********************************
* (SIMD) Row-based matchfinder
***********************************/

type ZSTD_VecMask = U64;

/* ZSTD_VecMask_next():
 * Starting from the LSB, returns the idx of the next non-zero bit. */
#[inline(always)]
fn ZSTD_VecMask_next(val: ZSTD_VecMask) -> U32 {
    ZSTD_countTrailingZeros64(val)
}

/* ZSTD_row_nextIndex():
 * Returns the next index to insert at within a tagTable row, and updates the "head". */
#[inline(always)]
unsafe fn ZSTD_row_nextIndex(tagRow: *mut BYTE, rowMask: U32) -> U32 {
    let mut next = (*tagRow as U32).wrapping_sub(1) & rowMask;
    next += if next == 0 { rowMask } else { 0 };
    *tagRow = next as BYTE;
    next
}

/* ZSTD_isAligned() */
#[inline(always)]
fn ZSTD_isAligned(ptr: *const c_void, align: usize) -> i32 {
    ((ptr as usize) & (align - 1) == 0) as i32
}

/* ZSTD_row_prefetch() */
#[inline(always)]
unsafe fn ZSTD_row_prefetch(hashTable: *const U32, tagTable: *const BYTE, relRow: U32, rowLog: U32) {
    PREFETCH_L1(hashTable.add(relRow as usize));
    if rowLog >= 5 {
        PREFETCH_L1(hashTable.add(relRow as usize + 16));
    }
    PREFETCH_L1(tagTable.add(relRow as usize));
    if rowLog == 6 {
        PREFETCH_L1(tagTable.add(relRow as usize + 32));
    }
}

/* ZSTD_row_fillHashCache():
 * Fill up the hash cache starting at idx, prefetching up to ZSTD_ROW_HASH_CACHE_SIZE entries. */
#[inline(always)]
unsafe fn ZSTD_row_fillHashCache(
    ms: *mut ZSTD_MatchState_t,
    base: *const BYTE,
    rowLog: U32,
    mls: U32,
    mut idx: U32,
    iLimit: *const BYTE,
) {
    let hashTable = (*ms).hashTable;
    let tagTable = (*ms).tagTable;
    let hashLog = (*ms).rowHashLog;
    let maxElemsToPrefetch = if base.add(idx as usize) > iLimit {
        0
    } else {
        (iLimit.offset_from(base.add(idx as usize)) + 1) as U32
    };
    let lim = idx + MIN_u32(ZSTD_ROW_HASH_CACHE_SIZE as U32, maxElemsToPrefetch);

    while idx < lim {
        let hash = ZSTD_hashPtrSalted(
            base.add(idx as usize) as *const c_void,
            hashLog + ZSTD_ROW_HASH_TAG_BITS,
            mls,
            (*ms).hashSalt,
        ) as U32;
        let row = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        ZSTD_row_prefetch(hashTable, tagTable, row, rowLog);
        (*ms).hashCache[(idx & ZSTD_ROW_HASH_CACHE_MASK) as usize] = hash;
        idx += 1;
    }
}

/* ZSTD_row_nextCachedHash():
 * Returns the hash of base + idx, and replaces the hash in the hash cache. */
#[inline(always)]
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
    let newHash = ZSTD_hashPtrSalted(
        base.add(idx as usize + ZSTD_ROW_HASH_CACHE_SIZE) as *const c_void,
        hashLog + ZSTD_ROW_HASH_TAG_BITS,
        mls,
        hashSalt,
    ) as U32;
    let row = (newHash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
    ZSTD_row_prefetch(hashTable, tagTable, row, rowLog);
    let hash = *cache.add((idx & ZSTD_ROW_HASH_CACHE_MASK) as usize);
    *cache.add((idx & ZSTD_ROW_HASH_CACHE_MASK) as usize) = newHash;
    hash
}

/* ZSTD_row_update_internalImpl():
 * Updates the hash table with positions starting from updateStartIdx until updateEndIdx. */
#[inline(always)]
unsafe fn ZSTD_row_update_internalImpl(
    ms: *mut ZSTD_MatchState_t,
    mut updateStartIdx: U32,
    updateEndIdx: U32,
    mls: U32,
    rowLog: U32,
    rowMask: U32,
    useCache: U32,
) {
    let hashTable = (*ms).hashTable;
    let tagTable = (*ms).tagTable;
    let hashLog = (*ms).rowHashLog;
    let base = (*ms).window.base;

    while updateStartIdx < updateEndIdx {
        let hash = if useCache != 0 {
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
                base.add(updateStartIdx as usize) as *const c_void,
                hashLog + ZSTD_ROW_HASH_TAG_BITS,
                mls,
                (*ms).hashSalt,
            ) as U32
        };
        let relRow = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        let row = hashTable.add(relRow as usize);
        let tagRow = tagTable.add(relRow as usize);
        let pos = ZSTD_row_nextIndex(tagRow, rowMask);

        *tagRow.add(pos as usize) = (hash & ZSTD_ROW_HASH_TAG_MASK) as BYTE;
        *row.add(pos as usize) = updateStartIdx;
        updateStartIdx += 1;
    }
}

/* ZSTD_row_update_internal():
 * Inserts the byte at ip into the appropriate position in the hash table, and updates ms->nextToUpdate. */
#[inline(always)]
unsafe fn ZSTD_row_update_internal(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    mls: U32,
    rowLog: U32,
    rowMask: U32,
    useCache: U32,
) {
    let mut idx = (*ms).nextToUpdate;
    let base = (*ms).window.base;
    let target = ip.offset_from(base) as U32;
    let kSkipThreshold: U32 = 384;
    let kMaxMatchStartPositionsToUpdate: U32 = 96;
    let kMaxMatchEndPositionsToUpdate: U32 = 32;

    if useCache != 0 {
        if target - idx > kSkipThreshold {
            let bound = idx + kMaxMatchStartPositionsToUpdate;
            ZSTD_row_update_internalImpl(ms, idx, bound, mls, rowLog, rowMask, useCache);
            idx = target - kMaxMatchEndPositionsToUpdate;
            ZSTD_row_fillHashCache(ms, base, rowLog, mls, idx, ip.add(1));
        }
    }
    ZSTD_row_update_internalImpl(ms, idx, target, mls, rowLog, rowMask, useCache);
    (*ms).nextToUpdate = target;
}

/* ZSTD_row_update():
 * External wrapper for ZSTD_row_update_internal(). */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_row_update(ms: *mut ZSTD_MatchState_t, ip: *const BYTE) {
    let rowLog = BOUNDED_u32(4, (*ms).cParams.searchLog, 6);
    let rowMask = (1u32 << rowLog) - 1;
    let mls = MIN_u32((*ms).cParams.minMatch, 6);

    ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 0);
}

/* Returns the mask width of bits group. For scalar path this is always 1. */
#[inline(always)]
fn ZSTD_row_matchMaskGroupWidth(rowEntries: U32) -> U32 {
    let _ = rowEntries;
    1
}

/* Returns a ZSTD_VecMask (U64) that has the nth group of bits set to 1 if the tag matches.
 * Scalar (SWAR) portable path. */
#[inline(always)]
unsafe fn ZSTD_row_getMatchMask(
    tagRow: *const BYTE,
    tag: BYTE,
    headGrouped: U32,
    rowEntries: U32,
) -> ZSTD_VecMask {
    let src = tagRow;

    /* SWAR */
    let chunkSize = core::mem::size_of::<usize>() as i32;
    let shiftAmount = ((chunkSize * 8) - chunkSize) as usize;
    let xFF = !(0usize);
    let x01 = xFF / 0xFF;
    let x80 = x01 << 7;
    let splatChar = (tag as usize).wrapping_mul(x01);
    let mut matches: ZSTD_VecMask = 0;
    let mut i = rowEntries as i32 - chunkSize;
    if MEM_isLittleEndian() != 0 {
        let extractMagic = (xFF / 0x7F) >> chunkSize;
        loop {
            let mut chunk = MEM_readST(src.add(i as usize) as *const c_void);
            chunk ^= splatChar;
            chunk = (((chunk | x80).wrapping_sub(x01)) | chunk) & x80;
            matches <<= chunkSize;
            matches |= ((chunk.wrapping_mul(extractMagic)) >> shiftAmount) as ZSTD_VecMask;
            i -= chunkSize;
            if i < 0 {
                break;
            }
        }
    } else {
        let msb = xFF ^ (xFF >> 1);
        let extractMagic = (msb / 0x1FF) | msb;
        loop {
            let mut chunk = MEM_readST(src.add(i as usize) as *const c_void);
            chunk ^= splatChar;
            chunk = (((chunk | x80).wrapping_sub(x01)) | chunk) & x80;
            matches <<= chunkSize;
            matches |= (((chunk >> 7).wrapping_mul(extractMagic)) >> shiftAmount) as ZSTD_VecMask;
            i -= chunkSize;
            if i < 0 {
                break;
            }
        }
    }
    matches = !matches;
    if rowEntries == 16 {
        ZSTD_rotateRight_U16(matches as U16, headGrouped) as ZSTD_VecMask
    } else if rowEntries == 32 {
        ZSTD_rotateRight_U32(matches as U32, headGrouped) as ZSTD_VecMask
    } else {
        ZSTD_rotateRight_U64(matches as U64, headGrouped)
    }
}

#[inline(always)]
unsafe fn ZSTD_RowFindBestMatch(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    offsetPtr: *mut usize,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
    rowLog: U32,
) -> usize {
    let hashTable = (*ms).hashTable;
    let tagTable = (*ms).tagTable;
    let hashCache = (*ms).hashCache.as_mut_ptr();
    let hashLog = (*ms).rowHashLog;
    let cParams = &(*ms).cParams;
    let base = (*ms).window.base;
    let dictBase = (*ms).window.dictBase;
    let dictLimit = (*ms).window.dictLimit;
    let prefixStart = base.add(dictLimit as usize);
    let dictEnd = dictBase.add(dictLimit as usize);
    let curr = ip.offset_from(base) as U32;
    let maxDistance = 1u32 << cParams.windowLog;
    let lowestValid = (*ms).window.lowLimit;
    let withinMaxDistance = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr - maxDistance
    } else {
        lowestValid
    };
    let isDictionary = ((*ms).loadedDictEnd != 0) as U32;
    let lowLimit = if isDictionary != 0 {
        lowestValid
    } else {
        withinMaxDistance
    };
    let rowEntries = 1u32 << rowLog;
    let rowMask = rowEntries - 1;
    let cappedSearchLog = MIN_u32(cParams.searchLog, rowLog);
    let groupWidth = ZSTD_row_matchMaskGroupWidth(rowEntries);
    let hashSalt = (*ms).hashSalt;
    let mut nbAttempts = 1u32 << cappedSearchLog;
    let mut ml: usize = 4 - 1;
    let hash: U32;

    let dms = (*ms).dictMatchState;

    let mut ddsIdx: usize = 0;
    let mut ddsExtraAttempts: U32 = 0;
    let mut dmsTag: U32 = 0;
    let mut dmsRow: *mut U32 = core::ptr::null_mut();
    let mut dmsTagRow: *mut BYTE = core::ptr::null_mut();

    if dictMode == ZSTD_dedicatedDictSearch {
        let ddsHashLog = (*dms).cParams.hashLog - ZSTD_LAZY_DDSS_BUCKET_LOG;
        ddsIdx = (ZSTD_hashPtr(ip as *const c_void, ddsHashLog, mls)) << ZSTD_LAZY_DDSS_BUCKET_LOG;
        PREFETCH_L1((*dms).hashTable.add(ddsIdx));
        ddsExtraAttempts = if cParams.searchLog > rowLog {
            1u32 << (cParams.searchLog - rowLog)
        } else {
            0
        };
    }

    if dictMode == ZSTD_dictMatchState {
        let dmsHashTable = (*dms).hashTable;
        let dmsTagTable = (*dms).tagTable;
        let dmsHash = ZSTD_hashPtr(
            ip as *const c_void,
            (*dms).rowHashLog + ZSTD_ROW_HASH_TAG_BITS,
            mls,
        ) as U32;
        let dmsRelRow = (dmsHash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        dmsTag = dmsHash & ZSTD_ROW_HASH_TAG_MASK;
        dmsTagRow = dmsTagTable.add(dmsRelRow as usize);
        dmsRow = dmsHashTable.add(dmsRelRow as usize);
        ZSTD_row_prefetch(dmsHashTable, dmsTagTable, dmsRelRow, rowLog);
    }

    /* Update the hashTable and tagTable up to (but not including) ip */
    if (*ms).lazySkipping == 0 {
        ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 1);
        hash = ZSTD_row_nextCachedHash(hashCache, hashTable, tagTable, base, curr, hashLog, rowLog, mls, hashSalt);
    } else {
        hash = ZSTD_hashPtrSalted(ip as *const c_void, hashLog + ZSTD_ROW_HASH_TAG_BITS, mls, hashSalt) as U32;
        (*ms).nextToUpdate = curr;
    }
    (*ms).hashSaltEntropy = (*ms).hashSaltEntropy.wrapping_add(hash);

    {
        /* Get the hash for ip, compute the appropriate row */
        let relRow = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        let tag = hash & ZSTD_ROW_HASH_TAG_MASK;
        let row = hashTable.add(relRow as usize);
        let tagRow = tagTable.add(relRow as usize);
        let headGrouped = ((*tagRow as U32) & rowMask) * groupWidth;
        let mut matchBuffer = [0u32; ZSTD_ROW_HASH_MAX_ENTRIES];
        let mut numMatches: usize = 0;
        let mut currMatch: usize = 0;
        let mut matches = ZSTD_row_getMatchMask(tagRow, tag as BYTE, headGrouped, rowEntries);

        /* Cycle through the matches and prefetch */
        while (matches > 0) && (nbAttempts > 0) {
            let matchPos = ((headGrouped + ZSTD_VecMask_next(matches)) / groupWidth) & rowMask;
            let matchIndex = *row.add(matchPos as usize);
            if matchPos == 0 {
                matches &= matches - 1;
                continue;
            }
            if matchIndex < lowLimit {
                break;
            }
            if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
                PREFETCH_L1(base.add(matchIndex as usize));
            } else {
                PREFETCH_L1(dictBase.add(matchIndex as usize));
            }
            matchBuffer[numMatches] = matchIndex;
            numMatches += 1;
            nbAttempts -= 1;
            matches &= matches - 1;
        }

        /* Speed opt: insert current byte into hashtable too. */
        {
            let pos = ZSTD_row_nextIndex(tagRow, rowMask);
            *tagRow.add(pos as usize) = tag as BYTE;
            *row.add(pos as usize) = (*ms).nextToUpdate;
            (*ms).nextToUpdate += 1;
        }

        /* Return the longest match */
        while currMatch < numMatches {
            let matchIndex = matchBuffer[currMatch];
            let mut currentMl: usize = 0;

            if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
                let r#match = base.add(matchIndex as usize);
                if MEM_read32(r#match.add(ml).offset(-3) as *const c_void)
                    == MEM_read32(ip.add(ml).offset(-3) as *const c_void)
                {
                    currentMl = ZSTD_count(ip, r#match, iLimit);
                }
            } else {
                let r#match = dictBase.add(matchIndex as usize);
                if MEM_read32(r#match as *const c_void) == MEM_read32(ip as *const c_void) {
                    currentMl = ZSTD_count_2segments(ip.add(4), r#match.add(4), iLimit, dictEnd, prefixStart) + 4;
                }
            }

            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE(curr - matchIndex) as usize;
                if ip.add(currentMl) == iLimit {
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
        let dmsLowestIndex = (*dms).window.dictLimit;
        let dmsBase = (*dms).window.base;
        let dmsEnd = (*dms).window.nextSrc;
        let dmsSize = dmsEnd.offset_from(dmsBase) as U32;
        let dmsIndexDelta = dictLimit.wrapping_sub(dmsSize);

        {
            let headGrouped = ((*dmsTagRow as U32) & rowMask) * groupWidth;
            let mut matchBuffer = [0u32; ZSTD_ROW_HASH_MAX_ENTRIES];
            let mut numMatches: usize = 0;
            let mut currMatch: usize = 0;
            let mut matches = ZSTD_row_getMatchMask(dmsTagRow, dmsTag as BYTE, headGrouped, rowEntries);

            while (matches > 0) && (nbAttempts > 0) {
                let matchPos = ((headGrouped + ZSTD_VecMask_next(matches)) / groupWidth) & rowMask;
                let matchIndex = *dmsRow.add(matchPos as usize);
                if matchPos == 0 {
                    matches &= matches - 1;
                    continue;
                }
                if matchIndex < dmsLowestIndex {
                    break;
                }
                PREFETCH_L1(dmsBase.add(matchIndex as usize));
                matchBuffer[numMatches] = matchIndex;
                numMatches += 1;
                nbAttempts -= 1;
                matches &= matches - 1;
            }

            while currMatch < numMatches {
                let matchIndex = matchBuffer[currMatch];
                let mut currentMl: usize = 0;

                {
                    let r#match = dmsBase.add(matchIndex as usize);
                    if MEM_read32(r#match as *const c_void) == MEM_read32(ip as *const c_void) {
                        currentMl = ZSTD_count_2segments(ip.add(4), r#match.add(4), iLimit, dmsEnd, prefixStart) + 4;
                    }
                }

                if currentMl > ml {
                    ml = currentMl;
                    *offsetPtr = OFFSET_TO_OFFBASE(curr - (matchIndex + dmsIndexDelta)) as usize;
                    if ip.add(currentMl) == iLimit {
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
 * Searches for the longest match at @p ip.
 * Dispatches to the correct implementation based on (searchMethod, dictMode, mls, rowLog).
 */
#[inline(always)]
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
    match searchMethod {
        search_hashChain => ZSTD_HcFindBestMatch(ms, ip, iend, offsetPtr, mls, dictMode),
        search_binaryTree => ZSTD_BtFindBestMatch(ms, ip, iend, offsetPtr, mls, dictMode),
        _ /* search_rowHash */ => ZSTD_RowFindBestMatch(ms, ip, iend, offsetPtr, mls, dictMode, rowLog),
    }
}

/* *******************************
*  Common parser - lazy strategy
*********************************/

#[inline(always)]
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
    let istart = src as *const BYTE;
    let mut ip = istart;
    let mut anchor = istart;
    let iend = istart.add(srcSize);
    let ilimit = if searchMethod == search_rowHash {
        iend.offset(-8 - ZSTD_ROW_HASH_CACHE_SIZE as isize)
    } else {
        iend.offset(-8)
    };
    let base = (*ms).window.base;
    let prefixLowestIndex = (*ms).window.dictLimit;
    let prefixLowest = base.add(prefixLowestIndex as usize);
    let mls = BOUNDED_u32(4, (*ms).cParams.minMatch, 6);
    let rowLog = BOUNDED_u32(4, (*ms).cParams.searchLog, 6);

    let mut offset_1 = *rep.add(0);
    let mut offset_2 = *rep.add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let isDMS = dictMode == ZSTD_dictMatchState;
    let isDDS = dictMode == ZSTD_dedicatedDictSearch;
    let isDxS = isDMS || isDDS;
    let dms = (*ms).dictMatchState;
    let dictLowestIndex = if isDxS { (*dms).window.dictLimit } else { 0 };
    let dictBase = if isDxS { (*dms).window.base } else { core::ptr::null() };
    let dictLowest = if isDxS { dictBase.add(dictLowestIndex as usize) } else { core::ptr::null() };
    let dictEnd = if isDxS { (*dms).window.nextSrc } else { core::ptr::null() };
    let dictIndexDelta = if isDxS {
        prefixLowestIndex.wrapping_sub(dictEnd.offset_from(dictBase) as U32)
    } else {
        0
    };
    let dictAndPrefixLength = (ip.offset_from(prefixLowest)
        + (dictEnd as isize - dictLowest as isize)) as U32;

    ip = ip.add((dictAndPrefixLength == 0) as usize);
    if dictMode == ZSTD_noDict {
        let curr = ip.offset_from(base) as U32;
        let windowLow = ZSTD_getLowestPrefixIndex(ms, curr, (*ms).cParams.windowLog);
        let maxRep = curr - windowLow;
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
        let mut matchLength: usize = 0;
        let mut offBase: usize = REPCODE1_TO_OFFBASE as usize;
        let mut start = ip.add(1);
        let mut store_seq = false;

        /* check repCode */
        if isDxS {
            let repIndex = (ip.offset_from(base) as U32) + 1 - offset_1;
            let repMatch = if (dictMode == ZSTD_dictMatchState
                || dictMode == ZSTD_dedicatedDictSearch)
                && repIndex < prefixLowestIndex
            {
                dictBase.add((repIndex - dictIndexDelta) as usize)
            } else {
                base.add(repIndex as usize)
            };
            if ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0
                && MEM_read32(repMatch as *const c_void) == MEM_read32(ip.add(1) as *const c_void)
            {
                let repMatchEnd = if repIndex < prefixLowestIndex { dictEnd } else { iend };
                matchLength = ZSTD_count_2segments(ip.add(1 + 4), repMatch.add(4), iend, repMatchEnd, prefixLowest) + 4;
                if depth == 0 {
                    store_seq = true;
                }
            }
        }
        if !store_seq
            && dictMode == ZSTD_noDict
            && ((offset_1 > 0) && (MEM_read32(ip.add(1).offset(-(offset_1 as isize)) as *const c_void) == MEM_read32(ip.add(1) as *const c_void)))
        {
            matchLength = ZSTD_count(ip.add(1 + 4), ip.add(1 + 4).offset(-(offset_1 as isize)), iend) + 4;
            if depth == 0 {
                store_seq = true;
            }
        }

        if !store_seq {
            /* first search (depth 0) */
            let mut offbaseFound: usize = 999999999;
            let ml2 = ZSTD_searchMax(ms, ip, iend, &mut offbaseFound, mls, rowLog, searchMethod, dictMode);
            if ml2 > matchLength {
                matchLength = ml2;
                start = ip;
                offBase = offbaseFound;
            }

            if matchLength < 4 {
                let step = ((ip.offset_from(anchor) as usize) >> kSearchStrength) + 1;
                ip = ip.add(step);
                (*ms).lazySkipping = (step > kLazySkippingStep as usize) as i32;
                continue;
            }

            /* let's try to find a better solution */
            if depth >= 1 {
                while ip < ilimit {
                    ip = ip.add(1);
                    if dictMode == ZSTD_noDict
                        && offBase != 0
                        && ((offset_1 > 0)
                            && (MEM_read32(ip as *const c_void) == MEM_read32(ip.offset(-(offset_1 as isize)) as *const c_void)))
                    {
                        let mlRep = ZSTD_count(ip.add(4), ip.add(4).offset(-(offset_1 as isize)), iend) + 4;
                        let gain2 = (mlRep * 3) as i32;
                        let gain1 = (matchLength * 3) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
                        if (mlRep >= 4) && (gain2 > gain1) {
                            matchLength = mlRep;
                            offBase = REPCODE1_TO_OFFBASE as usize;
                            start = ip;
                        }
                    }
                    if isDxS {
                        let repIndex = (ip.offset_from(base) as U32) - offset_1;
                        let repMatch = if repIndex < prefixLowestIndex {
                            dictBase.add((repIndex - dictIndexDelta) as usize)
                        } else {
                            base.add(repIndex as usize)
                        };
                        if ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0
                            && MEM_read32(repMatch as *const c_void) == MEM_read32(ip as *const c_void)
                        {
                            let repMatchEnd = if repIndex < prefixLowestIndex { dictEnd } else { iend };
                            let mlRep = ZSTD_count_2segments(ip.add(4), repMatch.add(4), iend, repMatchEnd, prefixLowest) + 4;
                            let gain2 = (mlRep * 3) as i32;
                            let gain1 = (matchLength * 3) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
                            if (mlRep >= 4) && (gain2 > gain1) {
                                matchLength = mlRep;
                                offBase = REPCODE1_TO_OFFBASE as usize;
                                start = ip;
                            }
                        }
                    }
                    {
                        let mut ofbCandidate: usize = 999999999;
                        let ml2 = ZSTD_searchMax(ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, dictMode);
                        let gain2 = (ml2 * 4) as i32 - ZSTD_highbit32(ofbCandidate as U32) as i32;
                        let gain1 = (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 4;
                        if (ml2 >= 4) && (gain2 > gain1) {
                            matchLength = ml2;
                            offBase = ofbCandidate;
                            start = ip;
                            continue;
                        }
                    }

                    /* let's find an even better one */
                    if (depth == 2) && (ip < ilimit) {
                        ip = ip.add(1);
                        if dictMode == ZSTD_noDict
                            && offBase != 0
                            && ((offset_1 > 0)
                                && (MEM_read32(ip as *const c_void) == MEM_read32(ip.offset(-(offset_1 as isize)) as *const c_void)))
                        {
                            let mlRep = ZSTD_count(ip.add(4), ip.add(4).offset(-(offset_1 as isize)), iend) + 4;
                            let gain2 = (mlRep * 4) as i32;
                            let gain1 = (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
                            if (mlRep >= 4) && (gain2 > gain1) {
                                matchLength = mlRep;
                                offBase = REPCODE1_TO_OFFBASE as usize;
                                start = ip;
                            }
                        }
                        if isDxS {
                            let repIndex = (ip.offset_from(base) as U32) - offset_1;
                            let repMatch = if repIndex < prefixLowestIndex {
                                dictBase.add((repIndex - dictIndexDelta) as usize)
                            } else {
                                base.add(repIndex as usize)
                            };
                            if ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0
                                && MEM_read32(repMatch as *const c_void) == MEM_read32(ip as *const c_void)
                            {
                                let repMatchEnd = if repIndex < prefixLowestIndex { dictEnd } else { iend };
                                let mlRep = ZSTD_count_2segments(ip.add(4), repMatch.add(4), iend, repMatchEnd, prefixLowest) + 4;
                                let gain2 = (mlRep * 4) as i32;
                                let gain1 = (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
                                if (mlRep >= 4) && (gain2 > gain1) {
                                    matchLength = mlRep;
                                    offBase = REPCODE1_TO_OFFBASE as usize;
                                    start = ip;
                                }
                            }
                        }
                        {
                            let mut ofbCandidate: usize = 999999999;
                            let ml2 = ZSTD_searchMax(ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, dictMode);
                            let gain2 = (ml2 * 4) as i32 - ZSTD_highbit32(ofbCandidate as U32) as i32;
                            let gain1 = (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 7;
                            if (ml2 >= 4) && (gain2 > gain1) {
                                matchLength = ml2;
                                offBase = ofbCandidate;
                                start = ip;
                                continue;
                            }
                        }
                    }
                    break;
                }
            }

            /* catch up */
            if OFFBASE_IS_OFFSET(offBase as U32) {
                if dictMode == ZSTD_noDict {
                    while ((start > anchor)
                        && (start.offset(-(OFFBASE_TO_OFFSET(offBase as U32) as isize)) > prefixLowest))
                        && (*start.offset(-1) == *start.offset(-(OFFBASE_TO_OFFSET(offBase as U32) as isize)).offset(-1))
                    {
                        start = start.offset(-1);
                        matchLength += 1;
                    }
                }
                if isDxS {
                    let matchIndex = ((start.offset_from(base) as usize) - OFFBASE_TO_OFFSET(offBase as U32) as usize) as U32;
                    let mut r#match = if matchIndex < prefixLowestIndex {
                        dictBase.add(matchIndex as usize).offset(-(dictIndexDelta as isize))
                    } else {
                        base.add(matchIndex as usize)
                    };
                    let mStart = if matchIndex < prefixLowestIndex { dictLowest } else { prefixLowest };
                    while (start > anchor) && (r#match > mStart) && (*start.offset(-1) == *r#match.offset(-1)) {
                        start = start.offset(-1);
                        r#match = r#match.offset(-1);
                        matchLength += 1;
                    }
                }
                offset_2 = offset_1;
                offset_1 = OFFBASE_TO_OFFSET(offBase as U32);
            }
        }

        /* store sequence */
        // _storeSequence:
        {
            let litLength = start.offset_from(anchor) as usize;
            ZSTD_storeSeq(seqStore, litLength, anchor, iend, offBase as U32, matchLength);
            ip = start.add(matchLength);
            anchor = ip;
        }
        if (*ms).lazySkipping != 0 {
            if searchMethod == search_rowHash {
                ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
            }
            (*ms).lazySkipping = 0;
        }

        /* check immediate repcode */
        if isDxS {
            while ip <= ilimit {
                let current2 = ip.offset_from(base) as U32;
                let repIndex = current2 - offset_2;
                let repMatch = if repIndex < prefixLowestIndex {
                    dictBase.offset(-(dictIndexDelta as isize)).add(repIndex as usize)
                } else {
                    base.add(repIndex as usize)
                };
                if ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0
                    && MEM_read32(repMatch as *const c_void) == MEM_read32(ip as *const c_void)
                {
                    let repEnd2 = if repIndex < prefixLowestIndex { dictEnd } else { iend };
                    matchLength = ZSTD_count_2segments(ip.add(4), repMatch.add(4), iend, repEnd2, prefixLowest) + 4;
                    offBase = offset_2 as usize;
                    offset_2 = offset_1;
                    offset_1 = offBase as U32;
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
                    ip = ip.add(matchLength);
                    anchor = ip;
                    continue;
                }
                break;
            }
        }

        if dictMode == ZSTD_noDict {
            while ((ip <= ilimit) && (offset_2 > 0))
                && (MEM_read32(ip as *const c_void) == MEM_read32(ip.offset(-(offset_2 as isize)) as *const c_void))
            {
                matchLength = ZSTD_count(ip.add(4), ip.add(4).offset(-(offset_2 as isize)), iend) + 4;
                offBase = offset_2 as usize;
                offset_2 = offset_1;
                offset_1 = offBase as U32;
                ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
                ip = ip.add(matchLength);
                anchor = ip;
                continue;
            }
        }
    }

    /* If offset_1 started invalid and became valid, rotate saved offsets. */
    offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
        offsetSaved1
    } else {
        offsetSaved2
    };

    /* save reps for next block */
    *rep.add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
    *rep.add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };

    /* Return the last literals size */
    iend.offset_from(anchor) as usize
}

/* Greedy / lazy / lazy2 / btlazy2 — noDict / dictMatchState / dedicatedDictSearch wrappers */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 0, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dictMatchState(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 0, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 0, ZSTD_dedicatedDictSearch)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 0, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 0, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 0, ZSTD_dedicatedDictSearch)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 1, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dictMatchState(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 1, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 1, ZSTD_dedicatedDictSearch)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 1, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 1, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 1, ZSTD_dedicatedDictSearch)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 2, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dictMatchState(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 2, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dedicatedDictSearch(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 2, ZSTD_dedicatedDictSearch)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 2, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dictMatchState_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 2, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dedicatedDictSearch_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 2, ZSTD_dedicatedDictSearch)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_binaryTree, 2, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2_dictMatchState(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_generic(ms, seqStore, rep, src, srcSize, search_binaryTree, 2, ZSTD_dictMatchState)
}

#[inline(always)]
unsafe fn ZSTD_compressBlock_lazy_extDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    searchMethod: searchMethod_e,
    depth: U32,
) -> usize {
    let istart = src as *const BYTE;
    let mut ip = istart;
    let mut anchor = istart;
    let iend = istart.add(srcSize);
    let ilimit = if searchMethod == search_rowHash {
        iend.offset(-8 - ZSTD_ROW_HASH_CACHE_SIZE as isize)
    } else {
        iend.offset(-8)
    };
    let base = (*ms).window.base;
    let dictLimit = (*ms).window.dictLimit;
    let prefixStart = base.add(dictLimit as usize);
    let dictBase = (*ms).window.dictBase;
    let dictEnd = dictBase.add(dictLimit as usize);
    let dictStart = dictBase.add((*ms).window.lowLimit as usize);
    let windowLog = (*ms).cParams.windowLog;
    let mls = BOUNDED_u32(4, (*ms).cParams.minMatch, 6);
    let rowLog = BOUNDED_u32(4, (*ms).cParams.searchLog, 6);

    let mut offset_1 = *rep.add(0);
    let mut offset_2 = *rep.add(1);

    /* Reset the lazy skipping state */
    (*ms).lazySkipping = 0;

    /* init */
    ip = ip.add((ip == prefixStart) as usize);
    if searchMethod == search_rowHash {
        ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
    }

    /* Match Loop */
    while ip < ilimit {
        let mut matchLength: usize = 0;
        let mut offBase: usize = REPCODE1_TO_OFFBASE as usize;
        let mut start = ip.add(1);
        let mut curr = ip.offset_from(base) as U32;
        let mut store_seq = false;

        /* check repCode */
        {
            let windowLow = ZSTD_getLowestMatchIndex(ms, curr + 1, windowLog);
            let repIndex = (curr + 1).wrapping_sub(offset_1);
            let repBase = if repIndex < dictLimit { dictBase } else { base };
            let repMatch = repBase.add(repIndex as usize);
            if (ZSTD_index_overlap_check(dictLimit, repIndex) & ((offset_1 <= curr + 1 - windowLow) as i32)) != 0 {
                if MEM_read32(ip.add(1) as *const c_void) == MEM_read32(repMatch as *const c_void) {
                    let repEnd = if repIndex < dictLimit { dictEnd } else { iend };
                    matchLength = ZSTD_count_2segments(ip.add(1 + 4), repMatch.add(4), iend, repEnd, prefixStart) + 4;
                    if depth == 0 {
                        store_seq = true;
                    }
                }
            }
        }

        if !store_seq {
            /* first search (depth 0) */
            let mut ofbCandidate: usize = 999999999;
            let ml2 = ZSTD_searchMax(ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, ZSTD_extDict);
            if ml2 > matchLength {
                matchLength = ml2;
                start = ip;
                offBase = ofbCandidate;
            }

            if matchLength < 4 {
                let step = (ip.offset_from(anchor) as usize) >> kSearchStrength;
                ip = ip.add(step + 1);
                (*ms).lazySkipping = (step > kLazySkippingStep as usize) as i32;
                continue;
            }

            /* let's try to find a better solution */
            if depth >= 1 {
                while ip < ilimit {
                    ip = ip.add(1);
                    curr += 1;
                    /* check repCode */
                    if offBase != 0 {
                        let windowLow = ZSTD_getLowestMatchIndex(ms, curr, windowLog);
                        let repIndex = curr.wrapping_sub(offset_1);
                        let repBase = if repIndex < dictLimit { dictBase } else { base };
                        let repMatch = repBase.add(repIndex as usize);
                        if (ZSTD_index_overlap_check(dictLimit, repIndex) & ((offset_1 <= curr - windowLow) as i32)) != 0 {
                            if MEM_read32(ip as *const c_void) == MEM_read32(repMatch as *const c_void) {
                                let repEnd = if repIndex < dictLimit { dictEnd } else { iend };
                                let repLength = ZSTD_count_2segments(ip.add(4), repMatch.add(4), iend, repEnd, prefixStart) + 4;
                                let gain2 = (repLength * 3) as i32;
                                let gain1 = (matchLength * 3) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
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
                        let ml2 = ZSTD_searchMax(ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, ZSTD_extDict);
                        let gain2 = (ml2 * 4) as i32 - ZSTD_highbit32(ofbCandidate as U32) as i32;
                        let gain1 = (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 4;
                        if (ml2 >= 4) && (gain2 > gain1) {
                            matchLength = ml2;
                            offBase = ofbCandidate;
                            start = ip;
                            continue;
                        }
                    }

                    /* let's find an even better one */
                    if (depth == 2) && (ip < ilimit) {
                        ip = ip.add(1);
                        curr += 1;
                        /* check repCode */
                        if offBase != 0 {
                            let windowLow = ZSTD_getLowestMatchIndex(ms, curr, windowLog);
                            let repIndex = curr.wrapping_sub(offset_1);
                            let repBase = if repIndex < dictLimit { dictBase } else { base };
                            let repMatch = repBase.add(repIndex as usize);
                            if (ZSTD_index_overlap_check(dictLimit, repIndex) & ((offset_1 <= curr - windowLow) as i32)) != 0 {
                                if MEM_read32(ip as *const c_void) == MEM_read32(repMatch as *const c_void) {
                                    let repEnd = if repIndex < dictLimit { dictEnd } else { iend };
                                    let repLength = ZSTD_count_2segments(ip.add(4), repMatch.add(4), iend, repEnd, prefixStart) + 4;
                                    let gain2 = (repLength * 4) as i32;
                                    let gain1 = (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 1;
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
                            let ml2 = ZSTD_searchMax(ms, ip, iend, &mut ofbCandidate, mls, rowLog, searchMethod, ZSTD_extDict);
                            let gain2 = (ml2 * 4) as i32 - ZSTD_highbit32(ofbCandidate as U32) as i32;
                            let gain1 = (matchLength * 4) as i32 - ZSTD_highbit32(offBase as U32) as i32 + 7;
                            if (ml2 >= 4) && (gain2 > gain1) {
                                matchLength = ml2;
                                offBase = ofbCandidate;
                                start = ip;
                                continue;
                            }
                        }
                    }
                    break;
                }
            }

            /* catch up */
            if OFFBASE_IS_OFFSET(offBase as U32) {
                let matchIndex = ((start.offset_from(base) as usize) - OFFBASE_TO_OFFSET(offBase as U32) as usize) as U32;
                let mut r#match = if matchIndex < dictLimit {
                    dictBase.add(matchIndex as usize)
                } else {
                    base.add(matchIndex as usize)
                };
                let mStart = if matchIndex < dictLimit { dictStart } else { prefixStart };
                while (start > anchor) && (r#match > mStart) && (*start.offset(-1) == *r#match.offset(-1)) {
                    start = start.offset(-1);
                    r#match = r#match.offset(-1);
                    matchLength += 1;
                }
                offset_2 = offset_1;
                offset_1 = OFFBASE_TO_OFFSET(offBase as U32);
            }
        }

        /* store sequence */
        // _storeSequence:
        {
            let litLength = start.offset_from(anchor) as usize;
            ZSTD_storeSeq(seqStore, litLength, anchor, iend, offBase as U32, matchLength);
            ip = start.add(matchLength);
            anchor = ip;
        }
        if (*ms).lazySkipping != 0 {
            if searchMethod == search_rowHash {
                ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
            }
            (*ms).lazySkipping = 0;
        }

        /* check immediate repcode */
        while ip <= ilimit {
            let repCurrent = ip.offset_from(base) as U32;
            let windowLow = ZSTD_getLowestMatchIndex(ms, repCurrent, windowLog);
            let repIndex = repCurrent - offset_2;
            let repBase = if repIndex < dictLimit { dictBase } else { base };
            let repMatch = repBase.add(repIndex as usize);
            if (ZSTD_index_overlap_check(dictLimit, repIndex) & ((offset_2 <= repCurrent - windowLow) as i32)) != 0 {
                if MEM_read32(ip as *const c_void) == MEM_read32(repMatch as *const c_void) {
                    let repEnd = if repIndex < dictLimit { dictEnd } else { iend };
                    matchLength = ZSTD_count_2segments(ip.add(4), repMatch.add(4), iend, repEnd, prefixStart) + 4;
                    offBase = offset_2 as usize;
                    offset_2 = offset_1;
                    offset_1 = offBase as U32;
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, matchLength);
                    ip = ip.add(matchLength);
                    anchor = ip;
                    continue;
                }
            }
            break;
        }
    }

    /* Save reps for next block */
    *rep.add(0) = offset_1;
    *rep.add(1) = offset_2;

    /* Return the last literals size */
    iend.offset_from(anchor) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_extDict(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_extDict_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_extDict(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_hashChain, 2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_extDict_row(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_rowHash, 2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2_extDict(
    ms: *mut ZSTD_MatchState_t, seqStore: *mut SeqStore_t, rep: *mut U32,
    src: *const c_void, srcSize: usize,
) -> usize {
    ZSTD_compressBlock_lazy_extDict_generic(ms, seqStore, rep, src, srcSize, search_binaryTree, 2)
}
