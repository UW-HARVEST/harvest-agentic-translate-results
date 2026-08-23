//! Translation of `compress/zstd_lazy.c`
#![allow(dead_code)]

use crate::common::bits::*;
use crate::common::mem::*;
use crate::common::zstd_internal::*;
use crate::compress::zstd_compress_internal::*;
use core::ffi::{c_int, c_uint, c_void};

/* PREFETCH_L1 / PREFETCH_L2: semantically no-ops (cache hints only). */
#[inline(always)]
unsafe fn PREFETCH_L1(ptr: *const BYTE) {
    #[cfg(target_arch = "x86_64")]
    {
        core::arch::x86_64::_mm_prefetch(ptr as *const i8, core::arch::x86_64::_MM_HINT_T0);
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = ptr;
    }
}

#[inline(always)]
unsafe fn PREFETCH_L2(ptr: *const BYTE) {
    #[cfg(target_arch = "x86_64")]
    {
        core::arch::x86_64::_mm_prefetch(ptr as *const i8, core::arch::x86_64::_MM_HINT_T1);
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = ptr;
    }
}

/* from zstd_lazy.h */
pub const ZSTD_LAZY_DDSS_BUCKET_LOG: U32 = 2;
pub const ZSTD_ROW_HASH_TAG_BITS: U32 = 8;

const kLazySkippingStep: usize = 8;

/* `size_t`-flavoured OFFBASE helpers, mirroring the C macros which operate on
 * whatever type they are handed (here: size_t). */
#[inline(always)]
const fn OFFSET_TO_OFFBASE_ST(o: usize) -> usize {
    o + ZSTD_REP_NUM
}

#[inline(always)]
const fn OFFBASE_IS_OFFSET_ST(o: usize) -> bool {
    o > ZSTD_REP_NUM
}

#[inline(always)]
const fn OFFBASE_TO_OFFSET_ST(o: usize) -> usize {
    o.wrapping_sub(ZSTD_REP_NUM)
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
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;

    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog.wrapping_sub(1);
    let btMask: U32 = (1u32 << btLog).wrapping_sub(1);

    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as U32;
    let mut idx: U32 = (*ms).nextToUpdate;

    let _ = iend;

    while idx < target {
        let h: usize = ZSTD_hashPtr(base.offset(idx as isize) as *const c_void, hashLog, mls);
        let matchIndex: U32 = *hashTable.add(h);

        let nextCandidatePtr: *mut U32 = bt.add((2u32.wrapping_mul(idx & btMask)) as usize);
        let sortMarkPtr: *mut U32 = nextCandidatePtr.add(1);

        *hashTable.add(h) = idx; /* Update Hash Table */
        *nextCandidatePtr = matchIndex; /* update BT like a chain */
        *sortMarkPtr = ZSTD_DUBT_UNSORTED_MARK;
        idx = idx.wrapping_add(1);
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
        base.wrapping_offset(curr as isize)
    } else {
        dictBase.wrapping_offset(curr as isize)
    };
    let iend: *const BYTE = if curr >= dictLimit {
        inputEnd
    } else {
        dictBase.wrapping_offset(dictLimit as isize)
    };
    let dictEnd: *const BYTE = dictBase.wrapping_offset(dictLimit as isize);
    let prefixStart: *const BYTE = base.wrapping_offset(dictLimit as isize);
    let mut match_: *const BYTE;
    let mut smallerPtr: *mut U32 = bt.add((2u32.wrapping_mul(curr & btMask)) as usize);
    let mut largerPtr: *mut U32 = smallerPtr.add(1);
    /* this candidate is unsorted : next sorted candidate is reached through
     * *smallerPtr, while *largerPtr contains previous unsorted candidate
     * (which is already saved and can be overwritten) */
    let mut matchIndex: U32 = *smallerPtr;
    let mut dummy32: U32 = 0; /* to be nullified at the end */
    let windowValid: U32 = (*ms).window.lowLimit;
    let maxDistance: U32 = 1u32 << (*cParams).windowLog;
    let windowLow: U32 = if curr.wrapping_sub(windowValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        windowValid
    };

    while nbCompares != 0 && (matchIndex > windowLow) {
        let nextPtr: *mut U32 = bt.add((2u32.wrapping_mul(matchIndex & btMask)) as usize);
        /* guaranteed minimum nb of common bytes */
        let mut matchLength: usize = MIN(commonLengthSmaller, commonLengthLarger);
        /* note : all candidates are now supposed sorted,
         * but it's still possible to have nextPtr[1] == ZSTD_DUBT_UNSORTED_MARK
         * when a real index has the same value as ZSTD_DUBT_UNSORTED_MARK */

        if (dictMode != ZSTD_extDict)
            || (matchIndex.wrapping_add(matchLength as U32) >= dictLimit) /* both in current segment */
            || (curr < dictLimit)
        /* both in extDict */
        {
            let mBase: *const BYTE = if (dictMode != ZSTD_extDict)
                || (matchIndex.wrapping_add(matchLength as U32) >= dictLimit)
            {
                base
            } else {
                dictBase
            };
            match_ = mBase.wrapping_offset(matchIndex as isize);
            matchLength += ZSTD_count(
                ip.wrapping_add(matchLength),
                match_.wrapping_add(matchLength),
                iend,
            );
        } else {
            match_ = dictBase.wrapping_offset(matchIndex as isize);
            matchLength += ZSTD_count_2segments(
                ip.wrapping_add(matchLength),
                match_.wrapping_add(matchLength),
                iend,
                dictEnd,
                prefixStart,
            );
            if matchIndex.wrapping_add(matchLength as U32) >= dictLimit {
                /* preparation for next read of match[matchLength] */
                match_ = base.wrapping_offset(matchIndex as isize);
            }
        }

        if ip.wrapping_add(matchLength) == iend {
            /* equal : no way to know if inf or sup */
            /* drop, to guarantee consistency ; miss a bit of compression, but
             * other solutions can corrupt tree */
            break;
        }

        if *match_.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
            /* match is smaller than current */
            *smallerPtr = matchIndex; /* update smaller idx */
            commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
            if matchIndex <= btLow {
                smallerPtr = &mut dummy32;
                break;
            } /* beyond tree size, stop searching */
            smallerPtr = nextPtr.add(1); /* new "candidate" => larger than match, which was smaller than target */
            matchIndex = *nextPtr.add(1); /* new matchIndex, larger than previous and closer to current */
        } else {
            /* match is larger than current */
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &mut dummy32;
                break;
            } /* beyond tree size, stop searching */
            largerPtr = nextPtr;
            matchIndex = *nextPtr.add(0);
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
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dmsCParams: *const ZSTD_compressionParameters = &(*dms).cParams;
    let dictHashTable: *const U32 = (*dms).hashTable;
    let hashLog: U32 = (*dmsCParams).hashLog;
    let h: usize = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut dictMatchIndex: U32 = *dictHashTable.add(h);

    let base: *const BYTE = (*ms).window.base;
    let prefixStart: *const BYTE = base.wrapping_offset((*ms).window.dictLimit as isize);
    let curr: U32 = ip.offset_from(base) as U32;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictHighLimit: U32 = (*dms).window.nextSrc.offset_from((*dms).window.base) as U32;
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

    let _ = dictMode;

    while nbCompares != 0 && (dictMatchIndex > dictLowLimit) {
        let nextPtr: *mut U32 = dictBt.add((2u32.wrapping_mul(dictMatchIndex & btMask)) as usize);
        /* guaranteed minimum nb of common bytes */
        let mut matchLength: usize = MIN(commonLengthSmaller, commonLengthLarger);
        let mut match_: *const BYTE = dictBase.wrapping_offset(dictMatchIndex as isize);
        matchLength += ZSTD_count_2segments(
            ip.wrapping_add(matchLength),
            match_.wrapping_add(matchLength),
            iend,
            dictEnd,
            prefixStart,
        );
        if dictMatchIndex.wrapping_add(matchLength as U32) >= dictHighLimit {
            /* to prepare for next usage of match[matchLength] */
            match_ = base
                .wrapping_offset(dictMatchIndex as isize)
                .wrapping_offset(dictIndexDelta as isize);
        }

        if matchLength > bestLength {
            let matchIndex: U32 = dictMatchIndex.wrapping_add(dictIndexDelta);
            if (4i32.wrapping_mul(matchLength.wrapping_sub(bestLength) as c_int))
                > (ZSTD_highbit32(curr.wrapping_sub(matchIndex).wrapping_add(1)).wrapping_sub(
                    ZSTD_highbit32((*offsetPtr as U32).wrapping_add(1)),
                )) as c_int
            {
                bestLength = matchLength;
                *offsetPtr = OFFSET_TO_OFFBASE_ST(curr.wrapping_sub(matchIndex) as usize);
            }
            if ip.wrapping_add(matchLength) == iend {
                /* reached end of input : ip[matchLength] is not valid, no way to
                 * know if it's larger or smaller than match */
                break; /* drop, to guarantee consistency (miss a little bit of compression) */
            }
        }

        if *match_.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
            if dictMatchIndex <= btLow {
                break;
            } /* beyond tree size, stop the search */
            commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
            dictMatchIndex = *nextPtr.add(1); /* new matchIndex larger than previous (closer to current) */
        } else {
            /* match is larger than current */
            if dictMatchIndex <= btLow {
                break;
            } /* beyond tree size, stop the search */
            commonLengthLarger = matchLength;
            dictMatchIndex = *nextPtr.add(0);
        }
        nbCompares -= 1;
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
    let curr: U32 = ip.offset_from(base) as U32;
    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, (*cParams).windowLog as c_uint);

    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog.wrapping_sub(1);
    let btMask: U32 = (1u32 << btLog).wrapping_sub(1);
    let btLow: U32 = if btMask >= curr {
        0
    } else {
        curr.wrapping_sub(btMask)
    };
    let unsortLimit: U32 = MAX(btLow, windowLow);

    let mut nextCandidate: *mut U32 = bt.add((2u32.wrapping_mul(matchIndex & btMask)) as usize);
    let mut unsortedMark: *mut U32 =
        bt.add((2u32.wrapping_mul(matchIndex & btMask)).wrapping_add(1) as usize);
    let mut nbCompares: U32 = 1u32 << (*cParams).searchLog;
    let mut nbCandidates: U32 = nbCompares;
    let mut previousCandidate: U32 = 0;

    /* reach end of unsorted candidates list */
    while (matchIndex > unsortLimit)
        && (*unsortedMark == ZSTD_DUBT_UNSORTED_MARK)
        && (nbCandidates > 1)
    {
        /* the unsortedMark becomes a reversed chain, to move up back to original position */
        *unsortedMark = previousCandidate;
        previousCandidate = matchIndex;
        matchIndex = *nextCandidate;
        nextCandidate = bt.add((2u32.wrapping_mul(matchIndex & btMask)) as usize);
        unsortedMark = bt.add((2u32.wrapping_mul(matchIndex & btMask)).wrapping_add(1) as usize);
        nbCandidates -= 1;
    }

    /* nullify last candidate if it's still unsorted
     * simplification, detrimental to compression ratio, beneficial for speed */
    if (matchIndex > unsortLimit) && (*unsortedMark == ZSTD_DUBT_UNSORTED_MARK) {
        *unsortedMark = 0;
        *nextCandidate = 0;
    }

    /* batch sort stacked candidates */
    matchIndex = previousCandidate;
    while matchIndex != 0 {
        /* will end on matchIndex == 0 */
        let nextCandidateIdxPtr: *mut U32 =
            bt.add((2u32.wrapping_mul(matchIndex & btMask)).wrapping_add(1) as usize);
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
        let dictEnd: *const BYTE = dictBase.wrapping_offset(dictLimit as isize);
        let prefixStart: *const BYTE = base.wrapping_offset(dictLimit as isize);
        let mut smallerPtr: *mut U32 = bt.add((2u32.wrapping_mul(curr & btMask)) as usize);
        let mut largerPtr: *mut U32 =
            bt.add((2u32.wrapping_mul(curr & btMask)).wrapping_add(1) as usize);
        let mut matchEndIdx: U32 = curr.wrapping_add(8).wrapping_add(1);
        let mut dummy32: U32 = 0; /* to be nullified at the end */
        let mut bestLength: usize = 0;

        matchIndex = *hashTable.add(h);
        *hashTable.add(h) = curr; /* Update Hash Table */

        while nbCompares != 0 && (matchIndex > windowLow) {
            let nextPtr: *mut U32 = bt.add((2u32.wrapping_mul(matchIndex & btMask)) as usize);
            /* guaranteed minimum nb of common bytes */
            let mut matchLength: usize = MIN(commonLengthSmaller, commonLengthLarger);
            let mut match_: *const BYTE;

            if (dictMode != ZSTD_extDict)
                || (matchIndex.wrapping_add(matchLength as U32) >= dictLimit)
            {
                match_ = base.wrapping_offset(matchIndex as isize);
                matchLength += ZSTD_count(
                    ip.wrapping_add(matchLength),
                    match_.wrapping_add(matchLength),
                    iend,
                );
            } else {
                match_ = dictBase.wrapping_offset(matchIndex as isize);
                matchLength += ZSTD_count_2segments(
                    ip.wrapping_add(matchLength),
                    match_.wrapping_add(matchLength),
                    iend,
                    dictEnd,
                    prefixStart,
                );
                if matchIndex.wrapping_add(matchLength as U32) >= dictLimit {
                    /* to prepare for next usage of match[matchLength] */
                    match_ = base.wrapping_offset(matchIndex as isize);
                }
            }

            if matchLength > bestLength {
                if matchLength > matchEndIdx.wrapping_sub(matchIndex) as usize {
                    matchEndIdx = matchIndex.wrapping_add(matchLength as U32);
                }
                if (4i32.wrapping_mul(matchLength.wrapping_sub(bestLength) as c_int))
                    > (ZSTD_highbit32(curr.wrapping_sub(matchIndex).wrapping_add(1))
                        .wrapping_sub(ZSTD_highbit32(*offBasePtr as U32))) as c_int
                {
                    bestLength = matchLength;
                    *offBasePtr = OFFSET_TO_OFFBASE_ST(curr.wrapping_sub(matchIndex) as usize);
                }
                if ip.wrapping_add(matchLength) == iend {
                    /* equal : no way to know if inf or sup */
                    if dictMode == ZSTD_dictMatchState {
                        /* in addition to avoiding checking any further in this
                         * loop, make sure we skip checking in the dictionary. */
                        nbCompares = 0;
                    }
                    break; /* drop, to guarantee consistency (miss a little bit of compression) */
                }
            }

            if *match_.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
                /* match is smaller than current */
                *smallerPtr = matchIndex; /* update smaller idx */
                commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
                if matchIndex <= btLow {
                    smallerPtr = &mut dummy32;
                    break;
                } /* beyond tree size, stop the search */
                smallerPtr = nextPtr.add(1); /* new "smaller" => larger of match */
                matchIndex = *nextPtr.add(1); /* new matchIndex larger than previous (closer to current) */
            } else {
                /* match is larger than current */
                *largerPtr = matchIndex;
                commonLengthLarger = matchLength;
                if matchIndex <= btLow {
                    largerPtr = &mut dummy32;
                    break;
                } /* beyond tree size, stop the search */
                largerPtr = nextPtr;
                matchIndex = *nextPtr.add(0);
            }
            nbCompares -= 1;
        }

        *largerPtr = 0;
        *smallerPtr = 0;

        if dictMode == ZSTD_dictMatchState && nbCompares != 0 {
            bestLength = ZSTD_DUBT_findBetterDictMatch(
                ms, ip, iend, offBasePtr, bestLength, nbCompares, mls, dictMode,
            );
        }

        (*ms).nextToUpdate = matchEndIdx.wrapping_sub(8); /* skip repetitive patterns */
        return bestLength;
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
    if ip < (*ms).window.base.wrapping_offset((*ms).nextToUpdate as isize) {
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
    let target: U32 = ip.offset_from(base) as U32;
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

    /* We know the hashtable is oversized by a factor of `bucketSize`.
     * We are going to temporarily pretend `bucketSize == 1`, keeping only a
     * single entry. We will use the rest of the space to construct a temporary
     * chaintable.
     */
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
            base.wrapping_offset(idx as isize) as *const c_void,
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
                        /* note: `!i ||` short-circuits, so countBeyondMinChain
                         * is only incremented when i != 0 */
                        if i == 0
                            || {
                                countBeyondMinChain = countBeyondMinChain.wrapping_add(1);
                                countBeyondMinChain > cacheSize
                            }
                        {
                            /* only allow pulling `cacheSize` number of entries
                             * into the cache or chainTable beyond `minChain`,
                             * to replace the entries pulled out of the
                             * chainTable into the cache. This lets us reach
                             * back further without increasing the total number
                             * of entries in the chainTable, guaranteeing the
                             * DDSS chain table will fit into the space
                             * allocated for the regular one. */
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
            base.wrapping_offset(idx as isize) as *const c_void,
            hashLog,
            (*ms).cParams.minMatch,
        ) as U32)
            << ZSTD_LAZY_DDSS_BUCKET_LOG;
        let mut i: U32;
        /* Shift hash cache down 1. */
        i = cacheSize.wrapping_sub(1);
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
    let ddsLowestIndex: U32 = (*dms).window.dictLimit;
    let ddsBase: *const BYTE = (*dms).window.base;
    let ddsEnd: *const BYTE = (*dms).window.nextSrc;
    let ddsSize: U32 = ddsEnd.offset_from(ddsBase) as U32;
    let ddsIndexDelta: U32 = dictLimit.wrapping_sub(ddsSize);
    let bucketSize: U32 = 1u32 << ZSTD_LAZY_DDSS_BUCKET_LOG;
    let bucketLimit: U32 = if nbAttempts < bucketSize.wrapping_sub(1) {
        nbAttempts
    } else {
        bucketSize.wrapping_sub(1)
    };
    let mut ddsAttempt: U32;
    let mut matchIndex: U32;

    let _ = ddsLowestIndex;

    ddsAttempt = 0;
    while ddsAttempt < bucketSize.wrapping_sub(1) {
        PREFETCH_L1(
            ddsBase.wrapping_offset(*(*dms).hashTable.add(ddsIdx + ddsAttempt as usize) as isize),
        );
        ddsAttempt = ddsAttempt.wrapping_add(1);
    }

    {
        let chainPackedPointer: U32 =
            *(*dms).hashTable.add(ddsIdx + bucketSize as usize - 1);
        let chainIndex: U32 = chainPackedPointer >> 8;

        PREFETCH_L1((*dms).chainTable.add(chainIndex as usize) as *const BYTE);
    }

    ddsAttempt = 0;
    while ddsAttempt < bucketLimit {
        let mut currentMl: usize = 0;
        let match_: *const BYTE;
        matchIndex = *(*dms).hashTable.add(ddsIdx + ddsAttempt as usize);
        match_ = ddsBase.wrapping_offset(matchIndex as isize);

        if matchIndex == 0 {
            return ml;
        }

        /* guaranteed by table construction */
        if MEM_read32(match_ as *const c_void) == MEM_read32(ip as *const c_void) {
            /* assumption : matchIndex <= dictLimit-4 (by table construction) */
            currentMl = ZSTD_count_2segments(
                ip.wrapping_add(4),
                match_.wrapping_add(4),
                iLimit,
                ddsEnd,
                prefixStart,
            ) + 4;
        }

        /* save best solution */
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = OFFSET_TO_OFFBASE_ST(
                curr.wrapping_sub(matchIndex.wrapping_add(ddsIndexDelta)) as usize,
            );
            if ip.wrapping_add(currentMl) == iLimit {
                /* best possible, avoids read overflow on next attempt */
                return ml;
            }
        }
        ddsAttempt = ddsAttempt.wrapping_add(1);
    }

    {
        let chainPackedPointer: U32 =
            *(*dms).hashTable.add(ddsIdx + bucketSize as usize - 1);
        let mut chainIndex: U32 = chainPackedPointer >> 8;
        let chainLength: U32 = chainPackedPointer & 0xFF;
        let chainAttempts: U32 = nbAttempts.wrapping_sub(ddsAttempt);
        let chainLimit: U32 = if chainAttempts > chainLength {
            chainLength
        } else {
            chainAttempts
        };
        let mut chainAttempt: U32;

        chainAttempt = 0;
        while chainAttempt < chainLimit {
            PREFETCH_L1(ddsBase.wrapping_offset(
                *(*dms).chainTable.add(chainIndex.wrapping_add(chainAttempt) as usize) as isize,
            ));
            chainAttempt = chainAttempt.wrapping_add(1);
        }

        chainAttempt = 0;
        while chainAttempt < chainLimit {
            let mut currentMl: usize = 0;
            let match_: *const BYTE;
            matchIndex = *(*dms).chainTable.add(chainIndex as usize);
            match_ = ddsBase.wrapping_offset(matchIndex as isize);

            /* guaranteed by table construction */
            if MEM_read32(match_ as *const c_void) == MEM_read32(ip as *const c_void) {
                /* assumption : matchIndex <= dictLimit-4 (by table construction) */
                currentMl = ZSTD_count_2segments(
                    ip.wrapping_add(4),
                    match_.wrapping_add(4),
                    iLimit,
                    ddsEnd,
                    prefixStart,
                ) + 4;
            }

            /* save best solution */
            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE_ST(
                    curr.wrapping_sub(matchIndex.wrapping_add(ddsIndexDelta)) as usize,
                );
                if ip.wrapping_add(currentMl) == iLimit {
                    break; /* best possible, avoids read overflow on next attempt */
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
/* NEXT_IN_CHAIN(d, mask) == chainTable[(d) & (mask)] */

/* Update chains up to ip (excluded)
 * Assumption : always within prefix (i.e. not within extDict) */
#[inline(always)]
unsafe fn ZSTD_insertAndFindFirstIndex_internal(
    ms: *mut ZSTD_MatchState_t,
    cParams: *const ZSTD_compressionParameters,
    ip: *const BYTE,
    mls: U32,
    lazySkipping: c_int,
) -> U32 {
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainMask: U32 = (1u32 << (*cParams).chainLog).wrapping_sub(1);
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as U32;
    let mut idx: U32 = (*ms).nextToUpdate;

    while idx < target {
        /* catch up */
        let h: usize = ZSTD_hashPtr(
            base.wrapping_offset(idx as isize) as *const c_void,
            hashLog,
            mls,
        );
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
    ZSTD_insertAndFindFirstIndex_internal(
        ms,
        cParams,
        ip,
        (*ms).cParams.minMatch,
        0, /* lazySkipping */
    )
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
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainSize: U32 = 1u32 << (*cParams).chainLog;
    let chainMask: U32 = chainSize.wrapping_sub(1);
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.wrapping_offset(dictLimit as isize);
    let dictEnd: *const BYTE = dictBase.wrapping_offset(dictLimit as isize);
    let curr: U32 = ip.offset_from(base) as U32;
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

    if dictMode == ZSTD_dedicatedDictSearch {
        let entry: *const U32 = (*dms).hashTable.add(ddsIdx);
        PREFETCH_L1(entry as *const BYTE);
    }

    /* HC4 match finder */
    matchIndex =
        ZSTD_insertAndFindFirstIndex_internal(ms, cParams, ip, mls, (*ms).lazySkipping);

    while ((matchIndex >= lowLimit) as c_int & (nbAttempts > 0) as c_int) != 0 {
        let mut currentMl: usize = 0;
        if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
            let match_: *const BYTE = base.wrapping_offset(matchIndex as isize);
            /* read 4B starting from (match + ml + 1 - sizeof(U32)) */
            if MEM_read32(match_.wrapping_add(ml).wrapping_sub(3) as *const c_void)
                == MEM_read32(ip.wrapping_add(ml).wrapping_sub(3) as *const c_void)
            {
                /* potentially better */
                currentMl = ZSTD_count(ip, match_, iLimit);
            }
        } else {
            let match_: *const BYTE = dictBase.wrapping_offset(matchIndex as isize);
            if MEM_read32(match_ as *const c_void) == MEM_read32(ip as *const c_void) {
                /* assumption : matchIndex <= dictLimit-4 (by table construction) */
                currentMl = ZSTD_count_2segments(
                    ip.wrapping_add(4),
                    match_.wrapping_add(4),
                    iLimit,
                    dictEnd,
                    prefixStart,
                ) + 4;
            }
        }

        /* save best solution */
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = OFFSET_TO_OFFBASE_ST(curr.wrapping_sub(matchIndex) as usize);
            if ip.wrapping_add(currentMl) == iLimit {
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
        let dmsSize: U32 = dmsEnd.offset_from(dmsBase) as U32;
        let dmsIndexDelta: U32 = dictLimit.wrapping_sub(dmsSize);
        let dmsMinChain: U32 = if dmsSize > dmsChainSize {
            dmsSize.wrapping_sub(dmsChainSize)
        } else {
            0
        };

        matchIndex = *(*dms)
            .hashTable
            .add(ZSTD_hashPtr(ip as *const c_void, (*dms).cParams.hashLog, mls));

        while ((matchIndex >= dmsLowestIndex) as c_int & (nbAttempts > 0) as c_int) != 0 {
            let mut currentMl: usize = 0;
            let match_: *const BYTE = dmsBase.wrapping_offset(matchIndex as isize);
            if MEM_read32(match_ as *const c_void) == MEM_read32(ip as *const c_void) {
                /* assumption : matchIndex <= dictLimit-4 (by table construction) */
                currentMl = ZSTD_count_2segments(
                    ip.wrapping_add(4),
                    match_.wrapping_add(4),
                    iLimit,
                    dmsEnd,
                    prefixStart,
                ) + 4;
            }

            /* save best solution */
            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE_ST(
                    curr.wrapping_sub(matchIndex.wrapping_add(dmsIndexDelta)) as usize,
                );
                if ip.wrapping_add(currentMl) == iLimit {
                    break; /* best possible, avoids read overflow on next attempt */
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
const ZSTD_ROW_HASH_TAG_MASK: U32 = (1u32 << ZSTD_ROW_HASH_TAG_BITS) - 1;
/* absolute maximum number of entries per row, for all configurations */
const ZSTD_ROW_HASH_MAX_ENTRIES: usize = 64;

const ZSTD_ROW_HASH_CACHE_MASK: U32 = (ZSTD_ROW_HASH_CACHE_SIZE - 1) as U32;

/* Clarifies when we are interacting with a U64 representing a mask of matches */
type ZSTD_VecMask = U64;

/* ZSTD_VecMask_next():
 * Starting from the LSB, returns the idx of the next non-zero bit.
 * Basically counting the nb of trailing zeroes.
 */
#[inline]
fn ZSTD_VecMask_next(val: ZSTD_VecMask) -> U32 {
    ZSTD_countTrailingZeros64(val)
}

/* ZSTD_row_nextIndex():
 * Returns the next index to insert at within a tagTable row, and updates the "head"
 * value to reflect the update. Essentially cycles backwards from [1, {entries per row})
 */
#[inline(always)]
unsafe fn ZSTD_row_nextIndex(tagRow: *mut BYTE, rowMask: U32) -> U32 {
    let mut next: U32 = (*tagRow as U32).wrapping_sub(1) & rowMask;
    next = next.wrapping_add(if next == 0 { rowMask } else { 0 }); /* skip first position */
    *tagRow = next as BYTE;
    next
}

/* ZSTD_isAligned():
 * Checks that a pointer is aligned to "align" bytes which must be a power of 2.
 */
#[inline]
unsafe fn ZSTD_isAligned(ptr: *const c_void, align: usize) -> c_int {
    (((ptr as usize) & (align.wrapping_sub(1))) == 0) as c_int
}

/* ZSTD_row_prefetch():
 * Performs prefetching for the hashTable and tagTable at a given row.
 */
#[inline(always)]
unsafe fn ZSTD_row_prefetch(
    hashTable: *const U32,
    tagTable: *const BYTE,
    relRow: U32,
    rowLog: U32,
) {
    PREFETCH_L1(hashTable.wrapping_offset(relRow as isize) as *const BYTE);
    if rowLog >= 5 {
        PREFETCH_L1(hashTable.wrapping_offset(relRow as isize).wrapping_add(16) as *const BYTE);
        /* Note: prefetching more of the hash table does not appear to be
         * beneficial for 128-entry rows */
    }
    PREFETCH_L1(tagTable.wrapping_offset(relRow as isize));
    if rowLog == 6 {
        PREFETCH_L1(tagTable.wrapping_offset(relRow as isize).wrapping_add(32));
    }
}

/* ZSTD_row_fillHashCache():
 * Fill up the hash cache starting at idx, prefetching up to ZSTD_ROW_HASH_CACHE_SIZE entries,
 * but not beyond iLimit.
 */
#[inline(always)]
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
    let maxElemsToPrefetch: U32 = if base.wrapping_offset(idx as isize) > iLimit {
        0
    } else {
        (iLimit.offset_from(base.wrapping_offset(idx as isize)) + 1) as U32
    };
    let lim: U32 = idx.wrapping_add(MIN(
        ZSTD_ROW_HASH_CACHE_SIZE as U32,
        maxElemsToPrefetch,
    ));

    while idx < lim {
        let hash: U32 = ZSTD_hashPtrSalted(
            base.wrapping_offset(idx as isize) as *const c_void,
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
 * Returns the hash of base + idx, and replaces the hash in the hash cache with the byte at
 * base + idx + ZSTD_ROW_HASH_CACHE_SIZE. Also prefetches the appropriate rows from hashTable and tagTable.
 */
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
    let newHash: U32 = ZSTD_hashPtrSalted(
        base.wrapping_offset(idx as isize)
            .wrapping_add(ZSTD_ROW_HASH_CACHE_SIZE) as *const c_void,
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
 * Updates the hash table with positions starting from updateStartIdx until updateEndIdx.
 */
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
                base.wrapping_offset(updateStartIdx as isize) as *const c_void,
                hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS),
                mls,
                (*ms).hashSalt,
            ) as U32
        };
        let relRow: U32 = (hash >> ZSTD_ROW_HASH_TAG_BITS) << rowLog;
        let row: *mut U32 = hashTable.wrapping_offset(relRow as isize);
        let tagRow: *mut BYTE = tagTable.wrapping_offset(relRow as isize);
        let pos: U32 = ZSTD_row_nextIndex(tagRow, rowMask);

        *tagRow.add(pos as usize) = (hash & ZSTD_ROW_HASH_TAG_MASK) as BYTE;
        *row.add(pos as usize) = updateStartIdx;
        updateStartIdx = updateStartIdx.wrapping_add(1);
    }
}

/* ZSTD_row_update_internal():
 * Inserts the byte at ip into the appropriate position in the hash table, and updates ms->nextToUpdate.
 * Skips sections of long matches as is necessary.
 */
#[inline(always)]
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
    let target: U32 = ip.offset_from(base) as U32;
    let kSkipThreshold: U32 = 384;
    let kMaxMatchStartPositionsToUpdate: U32 = 96;
    let kMaxMatchEndPositionsToUpdate: U32 = 32;

    if useCache != 0 {
        /* Only skip positions when using hash cache, i.e.
         * if we are loading a dict, don't skip anything.
         * If we decide to skip, then we only update a set number
         * of positions at the beginning and end of the match.
         */
        if target.wrapping_sub(idx) > kSkipThreshold {
            let bound: U32 = idx.wrapping_add(kMaxMatchStartPositionsToUpdate);
            ZSTD_row_update_internalImpl(ms, idx, bound, mls, rowLog, rowMask, useCache);
            idx = target.wrapping_sub(kMaxMatchEndPositionsToUpdate);
            ZSTD_row_fillHashCache(ms, base, rowLog, mls, idx, ip.wrapping_add(1));
        }
    }
    ZSTD_row_update_internalImpl(ms, idx, target, mls, rowLog, rowMask, useCache);
    (*ms).nextToUpdate = target;
}

/* ZSTD_row_update():
 * External wrapper for ZSTD_row_update_internal(). Used for filling the hashtable during dictionary
 * processing.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_row_update(ms: *mut ZSTD_MatchState_t, ip: *const BYTE) {
    let rowLog: U32 = BOUNDED(4, (*ms).cParams.searchLog, 6);
    let rowMask: U32 = (1u32 << rowLog) - 1;
    let mls: U32 = MIN((*ms).cParams.minMatch, 6 /* mls caps out at 6 */);

    ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 0 /* don't use cache */);
}

/* Returns the mask width of bits group of which will be set to 1. Given not all
 * architectures have easy movemask instruction, this helps to iterate over
 * groups of bits easier and faster.
 */
#[inline(always)]
fn ZSTD_row_matchMaskGroupWidth(rowEntries: U32) -> U32 {
    let _ = rowEntries;
    /* The NEON block is guarded by ZSTD_ARCH_ARM_NEON, which is not defined for
     * this build (x86-64). */
    #[cfg(target_arch = "aarch64")]
    {
        /* NEON path only works for little endian */
        if MEM_isLittleEndian() == 0 {
            return 1;
        }
        if rowEntries == 16 {
            return 4;
        }
        if rowEntries == 32 {
            return 2;
        }
        if rowEntries == 64 {
            return 1;
        }
    }
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
    use core::arch::x86_64::{
        __m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8,
    };
    let comparisonMask: __m128i = _mm_set1_epi8(tag as i8);
    let mut matches: [c_int; 4] = [0; 4];
    let mut i: c_int = 0;
    while i < nbChunks {
        let chunk: __m128i =
            _mm_loadu_si128(src.wrapping_offset((16 * i) as isize) as *const __m128i);
        let equalMask: __m128i = _mm_cmpeq_epi8(chunk, comparisonMask);
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

/* Returns a ZSTD_VecMask (U64) that has the nth group (determined by
 * ZSTD_row_matchMaskGroupWidth) of bits set to 1 if the newly-computed "tag"
 * matches the hash at the nth position in a row of the tagTable.
 * Each row is a circular buffer beginning at the value of "headGrouped". So we
 * must rotate the "matches" bitfield to match up with the actual layout of the
 * entries within the hashTable */
#[inline(always)]
unsafe fn ZSTD_row_getMatchMask(
    tagRow: *const BYTE,
    tag: BYTE,
    headGrouped: U32,
    rowEntries: U32,
) -> ZSTD_VecMask {
    let src: *const BYTE = tagRow;

    #[cfg(target_arch = "x86_64")]
    return ZSTD_row_getSSEMask((rowEntries / 16) as c_int, src, tag, headGrouped);

    #[cfg(not(target_arch = "x86_64"))]
    return ZSTD_row_getSWARMask(src, tag, headGrouped, rowEntries);
}

/* SWAR fallback of ZSTD_row_getMatchMask(), used when neither SSE2 nor
 * little-endian NEON is available. Kept compiled for completeness. */
#[inline(always)]
unsafe fn ZSTD_row_getSWARMask(
    src: *const BYTE,
    tag: BYTE,
    headGrouped: U32,
    rowEntries: U32,
) -> ZSTD_VecMask {
    {
        let chunkSize: c_int = core::mem::size_of::<usize>() as c_int;
        let shiftAmount: usize = ((chunkSize * 8) - chunkSize) as usize;
        let xFF: usize = !0usize;
        let x01: usize = xFF / 0xFF;
        let x80: usize = x01 << 7;
        let splatChar: usize = (tag as usize).wrapping_mul(x01);
        let mut matches: ZSTD_VecMask = 0;
        let mut i: c_int = rowEntries.wrapping_sub(chunkSize as U32) as c_int;
        if MEM_isLittleEndian() != 0 {
            /* runtime check so have two loops */
            let extractMagic: usize = (xFF / 0x7F) >> chunkSize;
            loop {
                let mut chunk: usize =
                    MEM_readST(src.wrapping_offset(i as isize) as *const c_void);
                chunk ^= splatChar;
                chunk = (((chunk | x80).wrapping_sub(x01)) | chunk) & x80;
                matches <<= chunkSize as u32;
                matches |= ((chunk.wrapping_mul(extractMagic)) >> shiftAmount) as ZSTD_VecMask;
                i -= chunkSize;
                if i < 0 {
                    break;
                }
            }
        } else {
            /* big endian: reverse bits during extraction */
            let msb: usize = xFF ^ (xFF >> 1);
            let extractMagic: usize = (msb / 0x1FF) | msb;
            loop {
                let mut chunk: usize =
                    MEM_readST(src.wrapping_offset(i as isize) as *const c_void);
                chunk ^= splatChar;
                chunk = (((chunk | x80).wrapping_sub(x01)) | chunk) & x80;
                matches <<= chunkSize as u32;
                matches |= (((chunk >> 7).wrapping_mul(extractMagic)) >> shiftAmount)
                    as ZSTD_VecMask;
                i -= chunkSize;
                if i < 0 {
                    break;
                }
            }
        }
        matches = !matches;
        if rowEntries == 16 {
            return ZSTD_rotateRight_U16(matches as U16, headGrouped) as ZSTD_VecMask;
        } else if rowEntries == 32 {
            return ZSTD_rotateRight_U32(matches as U32, headGrouped) as ZSTD_VecMask;
        } else {
            return ZSTD_rotateRight_U64(matches as U64, headGrouped);
        }
    }
}

/* The high-level approach of the SIMD row based match finder is as follows:
 * - Figure out where to insert the new entry:
 *      - Generate a hash for current input position and split it into a one byte of tag and `rowHashLog` bits of index.
 *      - The hashTable is effectively split into groups or "rows" of 15 or 31 entries of U32, and the index determines
 *        which row to insert into.
 *      - Determine the correct position within the row to insert the entry into.
 * - Use SIMD to efficiently compare the tags in the tagTable to the 1-byte tag calculated for the position and
 *   generate a bitfield that we can cycle through to check the collisions in the hash table.
 * - Pick the longest match.
 * - Insert the tag into the equivalent row and position in the tagTable.
 */
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
    let hashTable: *mut U32 = (*ms).hashTable;
    let tagTable: *mut BYTE = (*ms).tagTable;
    let hashCache: *mut U32 = (*ms).hashCache.as_mut_ptr();
    let hashLog: U32 = (*ms).rowHashLog;
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.wrapping_offset(dictLimit as isize);
    let dictEnd: *const BYTE = dictBase.wrapping_offset(dictLimit as isize);
    let curr: U32 = ip.offset_from(base) as U32;
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
    let cappedSearchLog: U32 = MIN((*cParams).searchLog, rowLog);
    let groupWidth: U32 = ZSTD_row_matchMaskGroupWidth(rowEntries);
    let hashSalt: U64 = (*ms).hashSalt;
    let mut nbAttempts: U32 = 1u32 << cappedSearchLog;
    let mut ml: usize = 4 - 1;
    let hash: U32;

    /* DMS/DDS variables that may be referenced later */
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;

    /* Initialize the following variables to satisfy static analyzer */
    let mut ddsIdx: usize = 0;
    let mut ddsExtraAttempts: U32 = 0; /* cctx hash tables are limited in searches, but allow extra searches into DDS */
    let mut dmsTag: U32 = 0;
    let mut dmsRow: *mut U32 = core::ptr::null_mut();
    let mut dmsTagRow: *mut BYTE = core::ptr::null_mut();

    if dictMode == ZSTD_dedicatedDictSearch {
        let ddsHashLog: U32 = (*dms).cParams.hashLog.wrapping_sub(ZSTD_LAZY_DDSS_BUCKET_LOG);
        {
            /* Prefetch DDS hashtable entry */
            ddsIdx = ZSTD_hashPtr(ip as *const c_void, ddsHashLog, mls) << ZSTD_LAZY_DDSS_BUCKET_LOG;
            PREFETCH_L1((*dms).hashTable.add(ddsIdx) as *const BYTE);
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
        dmsTagRow = dmsTagTable.wrapping_offset(dmsRelRow as isize);
        dmsRow = dmsHashTable.wrapping_offset(dmsRelRow as isize);
        ZSTD_row_prefetch(dmsHashTable, dmsTagTable, dmsRelRow, rowLog);
    }

    /* Update the hashTable and tagTable up to (but not including) ip */
    if (*ms).lazySkipping == 0 {
        ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 1 /* useCache */);
        hash = ZSTD_row_nextCachedHash(
            hashCache, hashTable, tagTable, base, curr, hashLog, rowLog, mls, hashSalt,
        );
    } else {
        /* Stop inserting every position when in the lazy skipping mode.
         * The hash cache is also not kept up to date in this mode.
         */
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
        let row: *mut U32 = hashTable.wrapping_offset(relRow as isize);
        let tagRow: *mut BYTE = tagTable.wrapping_offset(relRow as isize);
        let headGrouped: U32 = ((*tagRow as U32) & rowMask).wrapping_mul(groupWidth);
        let mut matchBuffer: [U32; ZSTD_ROW_HASH_MAX_ENTRIES] = [0; ZSTD_ROW_HASH_MAX_ENTRIES];
        let mut numMatches: usize = 0;
        let mut currMatch: usize = 0;
        let mut matches: ZSTD_VecMask =
            ZSTD_row_getMatchMask(tagRow, tag as BYTE, headGrouped, rowEntries);

        /* Cycle through the matches and prefetch */
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
            if (dictMode != ZSTD_extDict) || matchIndex >= dictLimit {
                PREFETCH_L1(base.wrapping_offset(matchIndex as isize));
            } else {
                PREFETCH_L1(dictBase.wrapping_offset(matchIndex as isize));
            }
            matchBuffer[numMatches] = matchIndex;
            numMatches += 1;
            nbAttempts = nbAttempts.wrapping_sub(1);
            matches &= matches.wrapping_sub(1);
        }

        /* Speed opt: insert current byte into hashtable too. This allows us to
         * avoid one iteration of the loop in ZSTD_row_update_internal() at the
         * next search. */
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
                let match_: *const BYTE = base.wrapping_offset(matchIndex as isize);
                /* read 4B starting from (match + ml + 1 - sizeof(U32)) */
                if MEM_read32(match_.wrapping_add(ml).wrapping_sub(3) as *const c_void)
                    == MEM_read32(ip.wrapping_add(ml).wrapping_sub(3) as *const c_void)
                {
                    /* potentially better */
                    currentMl = ZSTD_count(ip, match_, iLimit);
                }
            } else {
                let match_: *const BYTE = dictBase.wrapping_offset(matchIndex as isize);
                if MEM_read32(match_ as *const c_void) == MEM_read32(ip as *const c_void) {
                    /* assumption : matchIndex <= dictLimit-4 (by table construction) */
                    currentMl = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        match_.wrapping_add(4),
                        iLimit,
                        dictEnd,
                        prefixStart,
                    ) + 4;
                }
            }

            /* Save best solution */
            if currentMl > ml {
                ml = currentMl;
                *offsetPtr = OFFSET_TO_OFFBASE_ST(curr.wrapping_sub(matchIndex) as usize);
                if ip.wrapping_add(currentMl) == iLimit {
                    break; /* best possible, avoids read overflow on next attempt */
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
        /* TODO: Measure and potentially add prefetching to DMS */
        let dmsLowestIndex: U32 = (*dms).window.dictLimit;
        let dmsBase: *const BYTE = (*dms).window.base;
        let dmsEnd: *const BYTE = (*dms).window.nextSrc;
        let dmsSize: U32 = dmsEnd.offset_from(dmsBase) as U32;
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
                PREFETCH_L1(dmsBase.wrapping_offset(matchIndex as isize));
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
                    let match_: *const BYTE = dmsBase.wrapping_offset(matchIndex as isize);
                    if MEM_read32(match_ as *const c_void) == MEM_read32(ip as *const c_void) {
                        currentMl = ZSTD_count_2segments(
                            ip.wrapping_add(4),
                            match_.wrapping_add(4),
                            iLimit,
                            dmsEnd,
                            prefixStart,
                        ) + 4;
                    }
                }

                if currentMl > ml {
                    ml = currentMl;
                    *offsetPtr = OFFSET_TO_OFFBASE_ST(
                        curr.wrapping_sub(matchIndex.wrapping_add(dmsIndexDelta)) as usize,
                    );
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

/**
 * Generate search functions templated on (dictMode, mls, rowLog).
 * These functions are outlined for code size & compilation time.
 * ZSTD_searchMax() dispatches to the correct implementation function.
 */

macro_rules! GEN_ZSTD_BT_SEARCH_FN {
    ($name:ident, $dictMode:expr, $mls:expr) => {
        #[inline(never)]
        unsafe fn $name(
            ms: *mut ZSTD_MatchState_t,
            ip: *const BYTE,
            iLimit: *const BYTE,
            offBasePtr: *mut usize,
        ) -> usize {
            ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, $mls, $dictMode)
        }
    };
}

macro_rules! GEN_ZSTD_HC_SEARCH_FN {
    ($name:ident, $dictMode:expr, $mls:expr) => {
        #[inline(never)]
        unsafe fn $name(
            ms: *mut ZSTD_MatchState_t,
            ip: *const BYTE,
            iLimit: *const BYTE,
            offsetPtr: *mut usize,
        ) -> usize {
            ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, $mls, $dictMode)
        }
    };
}

macro_rules! GEN_ZSTD_ROW_SEARCH_FN {
    ($name:ident, $dictMode:expr, $mls:expr, $rowLog:expr) => {
        #[inline(never)]
        unsafe fn $name(
            ms: *mut ZSTD_MatchState_t,
            ip: *const BYTE,
            iLimit: *const BYTE,
            offsetPtr: *mut usize,
        ) -> usize {
            ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, $mls, $dictMode, $rowLog)
        }
    };
}

/* Generate row search fns for each combination of (dictMode, mls, rowLog) */
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_4_4, ZSTD_noDict, 4, 4);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_4_5, ZSTD_noDict, 4, 5);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_4_6, ZSTD_noDict, 4, 6);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_5_4, ZSTD_noDict, 5, 4);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_5_5, ZSTD_noDict, 5, 5);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_5_6, ZSTD_noDict, 5, 6);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_6_4, ZSTD_noDict, 6, 4);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_6_5, ZSTD_noDict, 6, 5);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_noDict_6_6, ZSTD_noDict, 6, 6);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_4_4, ZSTD_extDict, 4, 4);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_4_5, ZSTD_extDict, 4, 5);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_4_6, ZSTD_extDict, 4, 6);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_5_4, ZSTD_extDict, 5, 4);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_5_5, ZSTD_extDict, 5, 5);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_5_6, ZSTD_extDict, 5, 6);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_6_4, ZSTD_extDict, 6, 4);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_6_5, ZSTD_extDict, 6, 5);
GEN_ZSTD_ROW_SEARCH_FN!(ZSTD_RowFindBestMatch_extDict_6_6, ZSTD_extDict, 6, 6);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_4_4,
    ZSTD_dictMatchState,
    4,
    4
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_4_5,
    ZSTD_dictMatchState,
    4,
    5
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_4_6,
    ZSTD_dictMatchState,
    4,
    6
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_5_4,
    ZSTD_dictMatchState,
    5,
    4
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_5_5,
    ZSTD_dictMatchState,
    5,
    5
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_5_6,
    ZSTD_dictMatchState,
    5,
    6
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_6_4,
    ZSTD_dictMatchState,
    6,
    4
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_6_5,
    ZSTD_dictMatchState,
    6,
    5
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dictMatchState_6_6,
    ZSTD_dictMatchState,
    6,
    6
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_4_4,
    ZSTD_dedicatedDictSearch,
    4,
    4
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_4_5,
    ZSTD_dedicatedDictSearch,
    4,
    5
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_4_6,
    ZSTD_dedicatedDictSearch,
    4,
    6
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_5_4,
    ZSTD_dedicatedDictSearch,
    5,
    4
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_5_5,
    ZSTD_dedicatedDictSearch,
    5,
    5
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_5_6,
    ZSTD_dedicatedDictSearch,
    5,
    6
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_6_4,
    ZSTD_dedicatedDictSearch,
    6,
    4
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_6_5,
    ZSTD_dedicatedDictSearch,
    6,
    5
);
GEN_ZSTD_ROW_SEARCH_FN!(
    ZSTD_RowFindBestMatch_dedicatedDictSearch_6_6,
    ZSTD_dedicatedDictSearch,
    6,
    6
);

/* Generate binary Tree search fns for each combination of (dictMode, mls) */
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_noDict_4, ZSTD_noDict, 4);
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_noDict_5, ZSTD_noDict, 5);
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_noDict_6, ZSTD_noDict, 6);
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_extDict_4, ZSTD_extDict, 4);
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_extDict_5, ZSTD_extDict, 5);
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_extDict_6, ZSTD_extDict, 6);
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_dictMatchState_4, ZSTD_dictMatchState, 4);
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_dictMatchState_5, ZSTD_dictMatchState, 5);
GEN_ZSTD_BT_SEARCH_FN!(ZSTD_BtFindBestMatch_dictMatchState_6, ZSTD_dictMatchState, 6);
GEN_ZSTD_BT_SEARCH_FN!(
    ZSTD_BtFindBestMatch_dedicatedDictSearch_4,
    ZSTD_dedicatedDictSearch,
    4
);
GEN_ZSTD_BT_SEARCH_FN!(
    ZSTD_BtFindBestMatch_dedicatedDictSearch_5,
    ZSTD_dedicatedDictSearch,
    5
);
GEN_ZSTD_BT_SEARCH_FN!(
    ZSTD_BtFindBestMatch_dedicatedDictSearch_6,
    ZSTD_dedicatedDictSearch,
    6
);

/* Generate hash chain search fns for each combination of (dictMode, mls) */
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_noDict_4, ZSTD_noDict, 4);
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_noDict_5, ZSTD_noDict, 5);
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_noDict_6, ZSTD_noDict, 6);
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_extDict_4, ZSTD_extDict, 4);
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_extDict_5, ZSTD_extDict, 5);
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_extDict_6, ZSTD_extDict, 6);
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_dictMatchState_4, ZSTD_dictMatchState, 4);
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_dictMatchState_5, ZSTD_dictMatchState, 5);
GEN_ZSTD_HC_SEARCH_FN!(ZSTD_HcFindBestMatch_dictMatchState_6, ZSTD_dictMatchState, 6);
GEN_ZSTD_HC_SEARCH_FN!(
    ZSTD_HcFindBestMatch_dedicatedDictSearch_4,
    ZSTD_dedicatedDictSearch,
    4
);
GEN_ZSTD_HC_SEARCH_FN!(
    ZSTD_HcFindBestMatch_dedicatedDictSearch_5,
    ZSTD_dedicatedDictSearch,
    5
);
GEN_ZSTD_HC_SEARCH_FN!(
    ZSTD_HcFindBestMatch_dedicatedDictSearch_6,
    ZSTD_dedicatedDictSearch,
    6
);

pub type searchMethod_e = c_uint;
pub const search_hashChain: searchMethod_e = 0;
pub const search_binaryTree: searchMethod_e = 1;
pub const search_rowHash: searchMethod_e = 2;

macro_rules! ZSTD_SWITCH_SEARCH_METHOD {
    ($ms:ident, $ip:ident, $iend:ident, $offsetPtr:ident, $mls:ident, $rowLog:ident,
     $searchMethod:ident,
     $hc4:ident, $hc5:ident, $hc6:ident,
     $bt4:ident, $bt5:ident, $bt6:ident,
     $r44:ident, $r45:ident, $r46:ident,
     $r54:ident, $r55:ident, $r56:ident,
     $r64:ident, $r65:ident, $r66:ident) => {
        if $searchMethod == search_hashChain {
            match $mls {
                4 => return $hc4($ms, $ip, $iend, $offsetPtr),
                5 => return $hc5($ms, $ip, $iend, $offsetPtr),
                6 => return $hc6($ms, $ip, $iend, $offsetPtr),
                _ => {}
            }
        } else if $searchMethod == search_binaryTree {
            match $mls {
                4 => return $bt4($ms, $ip, $iend, $offsetPtr),
                5 => return $bt5($ms, $ip, $iend, $offsetPtr),
                6 => return $bt6($ms, $ip, $iend, $offsetPtr),
                _ => {}
            }
        } else if $searchMethod == search_rowHash {
            match $mls {
                4 => match $rowLog {
                    4 => return $r44($ms, $ip, $iend, $offsetPtr),
                    5 => return $r45($ms, $ip, $iend, $offsetPtr),
                    6 => return $r46($ms, $ip, $iend, $offsetPtr),
                    _ => {}
                },
                5 => match $rowLog {
                    4 => return $r54($ms, $ip, $iend, $offsetPtr),
                    5 => return $r55($ms, $ip, $iend, $offsetPtr),
                    6 => return $r56($ms, $ip, $iend, $offsetPtr),
                    _ => {}
                },
                6 => match $rowLog {
                    4 => return $r64($ms, $ip, $iend, $offsetPtr),
                    5 => return $r65($ms, $ip, $iend, $offsetPtr),
                    6 => return $r66($ms, $ip, $iend, $offsetPtr),
                    _ => {}
                },
                _ => {}
            }
        }
    };
}

/**
 * Searches for the longest match at @p ip.
 * Dispatches to the correct implementation function based on the
 * (searchMethod, dictMode, mls, rowLog).
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
    if dictMode == ZSTD_noDict {
        ZSTD_SWITCH_SEARCH_METHOD!(
            ms,
            ip,
            iend,
            offsetPtr,
            mls,
            rowLog,
            searchMethod,
            ZSTD_HcFindBestMatch_noDict_4,
            ZSTD_HcFindBestMatch_noDict_5,
            ZSTD_HcFindBestMatch_noDict_6,
            ZSTD_BtFindBestMatch_noDict_4,
            ZSTD_BtFindBestMatch_noDict_5,
            ZSTD_BtFindBestMatch_noDict_6,
            ZSTD_RowFindBestMatch_noDict_4_4,
            ZSTD_RowFindBestMatch_noDict_4_5,
            ZSTD_RowFindBestMatch_noDict_4_6,
            ZSTD_RowFindBestMatch_noDict_5_4,
            ZSTD_RowFindBestMatch_noDict_5_5,
            ZSTD_RowFindBestMatch_noDict_5_6,
            ZSTD_RowFindBestMatch_noDict_6_4,
            ZSTD_RowFindBestMatch_noDict_6_5,
            ZSTD_RowFindBestMatch_noDict_6_6
        );
    } else if dictMode == ZSTD_extDict {
        ZSTD_SWITCH_SEARCH_METHOD!(
            ms,
            ip,
            iend,
            offsetPtr,
            mls,
            rowLog,
            searchMethod,
            ZSTD_HcFindBestMatch_extDict_4,
            ZSTD_HcFindBestMatch_extDict_5,
            ZSTD_HcFindBestMatch_extDict_6,
            ZSTD_BtFindBestMatch_extDict_4,
            ZSTD_BtFindBestMatch_extDict_5,
            ZSTD_BtFindBestMatch_extDict_6,
            ZSTD_RowFindBestMatch_extDict_4_4,
            ZSTD_RowFindBestMatch_extDict_4_5,
            ZSTD_RowFindBestMatch_extDict_4_6,
            ZSTD_RowFindBestMatch_extDict_5_4,
            ZSTD_RowFindBestMatch_extDict_5_5,
            ZSTD_RowFindBestMatch_extDict_5_6,
            ZSTD_RowFindBestMatch_extDict_6_4,
            ZSTD_RowFindBestMatch_extDict_6_5,
            ZSTD_RowFindBestMatch_extDict_6_6
        );
    } else if dictMode == ZSTD_dictMatchState {
        ZSTD_SWITCH_SEARCH_METHOD!(
            ms,
            ip,
            iend,
            offsetPtr,
            mls,
            rowLog,
            searchMethod,
            ZSTD_HcFindBestMatch_dictMatchState_4,
            ZSTD_HcFindBestMatch_dictMatchState_5,
            ZSTD_HcFindBestMatch_dictMatchState_6,
            ZSTD_BtFindBestMatch_dictMatchState_4,
            ZSTD_BtFindBestMatch_dictMatchState_5,
            ZSTD_BtFindBestMatch_dictMatchState_6,
            ZSTD_RowFindBestMatch_dictMatchState_4_4,
            ZSTD_RowFindBestMatch_dictMatchState_4_5,
            ZSTD_RowFindBestMatch_dictMatchState_4_6,
            ZSTD_RowFindBestMatch_dictMatchState_5_4,
            ZSTD_RowFindBestMatch_dictMatchState_5_5,
            ZSTD_RowFindBestMatch_dictMatchState_5_6,
            ZSTD_RowFindBestMatch_dictMatchState_6_4,
            ZSTD_RowFindBestMatch_dictMatchState_6_5,
            ZSTD_RowFindBestMatch_dictMatchState_6_6
        );
    } else if dictMode == ZSTD_dedicatedDictSearch {
        ZSTD_SWITCH_SEARCH_METHOD!(
            ms,
            ip,
            iend,
            offsetPtr,
            mls,
            rowLog,
            searchMethod,
            ZSTD_HcFindBestMatch_dedicatedDictSearch_4,
            ZSTD_HcFindBestMatch_dedicatedDictSearch_5,
            ZSTD_HcFindBestMatch_dedicatedDictSearch_6,
            ZSTD_BtFindBestMatch_dedicatedDictSearch_4,
            ZSTD_BtFindBestMatch_dedicatedDictSearch_5,
            ZSTD_BtFindBestMatch_dedicatedDictSearch_6,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_4_4,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_4_5,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_4_6,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_5_4,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_5_5,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_5_6,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_6_4,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_6_5,
            ZSTD_RowFindBestMatch_dedicatedDictSearch_6_6
        );
    }
    0
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
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = if searchMethod == search_rowHash {
        iend.wrapping_sub(8).wrapping_sub(ZSTD_ROW_HASH_CACHE_SIZE)
    } else {
        iend.wrapping_sub(8)
    };
    let base: *const BYTE = (*ms).window.base;
    let prefixLowestIndex: U32 = (*ms).window.dictLimit;
    let prefixLowest: *const BYTE = base.wrapping_offset(prefixLowestIndex as isize);
    let mls: U32 = BOUNDED(4, (*ms).cParams.minMatch, 6);
    let rowLog: U32 = BOUNDED(4, (*ms).cParams.searchLog, 6);

    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let isDMS: c_int = (dictMode == ZSTD_dictMatchState) as c_int;
    let isDDS: c_int = (dictMode == ZSTD_dedicatedDictSearch) as c_int;
    let isDxS: c_int = (isDMS != 0 || isDDS != 0) as c_int;
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictLowestIndex: U32 = if isDxS != 0 {
        (*dms).window.dictLimit
    } else {
        0
    };
    let dictBase: *const BYTE = if isDxS != 0 {
        (*dms).window.base
    } else {
        core::ptr::null()
    };
    let dictLowest: *const BYTE = if isDxS != 0 {
        dictBase.wrapping_offset(dictLowestIndex as isize)
    } else {
        core::ptr::null()
    };
    let dictEnd: *const BYTE = if isDxS != 0 {
        (*dms).window.nextSrc
    } else {
        core::ptr::null()
    };
    let dictIndexDelta: U32 = if isDxS != 0 {
        prefixLowestIndex.wrapping_sub((dictEnd as isize - dictBase as isize) as U32)
    } else {
        0
    };
    let dictAndPrefixLength: U32 = ((ip as isize - prefixLowest as isize)
        + (dictEnd as isize - dictLowest as isize)) as U32;

    ip = ip.wrapping_add((dictAndPrefixLength == 0) as usize);
    if dictMode == ZSTD_noDict {
        let curr: U32 = (ip as isize - base as isize) as U32;
        let windowLow: U32 =
            ZSTD_getLowestPrefixIndex(ms, curr, (*ms).cParams.windowLog as c_uint);
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
    while ip < ilimit {
        let mut matchLength: usize = 0;
        let mut offBase: usize = REPCODE1_TO_OFFBASE as usize;
        let mut start: *const BYTE = ip.wrapping_add(1);
        let mut goto_storeSequence: bool = false;

        /* check repCode */
        if isDxS != 0 {
            let repIndex: U32 = ((ip as isize - base as isize) as U32)
                .wrapping_add(1)
                .wrapping_sub(offset_1);
            let repMatch: *const BYTE = if (dictMode == ZSTD_dictMatchState
                || dictMode == ZSTD_dedicatedDictSearch)
                && repIndex < prefixLowestIndex
            {
                dictBase.wrapping_offset(repIndex.wrapping_sub(dictIndexDelta) as isize)
            } else {
                base.wrapping_offset(repIndex as isize)
            };
            if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                && (MEM_read32(repMatch as *const c_void)
                    == MEM_read32(ip.wrapping_add(1) as *const c_void))
            {
                let repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                    dictEnd
                } else {
                    iend
                };
                matchLength = ZSTD_count_2segments(
                    ip.wrapping_add(1).wrapping_add(4),
                    repMatch.wrapping_add(4),
                    iend,
                    repMatchEnd,
                    prefixLowest,
                ) + 4;
                if depth == 0 {
                    goto_storeSequence = true;
                }
            }
        }
        if !goto_storeSequence
            && dictMode == ZSTD_noDict
            && (((offset_1 > 0) as c_int
                & (MEM_read32(ip.wrapping_add(1).wrapping_sub(offset_1 as usize) as *const c_void)
                    == MEM_read32(ip.wrapping_add(1) as *const c_void)) as c_int)
                != 0)
        {
            matchLength = ZSTD_count(
                ip.wrapping_add(1).wrapping_add(4),
                ip.wrapping_add(1)
                    .wrapping_add(4)
                    .wrapping_sub(offset_1 as usize),
                iend,
            ) + 4;
            if depth == 0 {
                goto_storeSequence = true;
            }
        }

        if !goto_storeSequence {
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
                let step: usize =
                    (((ip as isize - anchor as isize) as usize) >> kSearchStrength) + 1;
                ip = ip.wrapping_add(step);
                /* Enter the lazy skipping mode once we are skipping more than 8
                 * bytes at a time. */
                (*ms).lazySkipping = (step > kLazySkippingStep) as c_int;
                continue;
            }

            /* let's try to find a better solution */
            if depth >= 1 {
                while ip < ilimit {
                    ip = ip.wrapping_add(1);
                    if (dictMode == ZSTD_noDict)
                        && (offBase != 0)
                        && (((offset_1 > 0) as c_int
                            & (MEM_read32(ip as *const c_void)
                                == MEM_read32(
                                    ip.wrapping_sub(offset_1 as usize) as *const c_void
                                )) as c_int)
                            != 0)
                    {
                        let mlRep: usize = ZSTD_count(
                            ip.wrapping_add(4),
                            ip.wrapping_add(4).wrapping_sub(offset_1 as usize),
                            iend,
                        ) + 4;
                        let gain2: c_int = mlRep.wrapping_mul(3) as c_int;
                        let gain1: c_int = matchLength
                            .wrapping_mul(3)
                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                            .wrapping_add(1) as c_int;
                        if (mlRep >= 4) && (gain2 > gain1) {
                            matchLength = mlRep;
                            offBase = REPCODE1_TO_OFFBASE as usize;
                            start = ip;
                        }
                    }
                    if isDxS != 0 {
                        let repIndex: U32 =
                            ((ip as isize - base as isize) as U32).wrapping_sub(offset_1);
                        let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                            dictBase
                                .wrapping_offset(repIndex.wrapping_sub(dictIndexDelta) as isize)
                        } else {
                            base.wrapping_offset(repIndex as isize)
                        };
                        if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                            && (MEM_read32(repMatch as *const c_void)
                                == MEM_read32(ip as *const c_void))
                        {
                            let repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                                dictEnd
                            } else {
                                iend
                            };
                            let mlRep: usize = ZSTD_count_2segments(
                                ip.wrapping_add(4),
                                repMatch.wrapping_add(4),
                                iend,
                                repMatchEnd,
                                prefixLowest,
                            ) + 4;
                            let gain2: c_int = mlRep.wrapping_mul(3) as c_int;
                            let gain1: c_int = matchLength
                                .wrapping_mul(3)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                .wrapping_add(1) as c_int;
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
                        let gain2: c_int = ml2
                            .wrapping_mul(4)
                            .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as usize)
                            as c_int;
                        let gain1: c_int = matchLength
                            .wrapping_mul(4)
                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                            .wrapping_add(4) as c_int;
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
                            && (((offset_1 > 0) as c_int
                                & (MEM_read32(ip as *const c_void)
                                    == MEM_read32(
                                        ip.wrapping_sub(offset_1 as usize) as *const c_void
                                    )) as c_int)
                                != 0)
                        {
                            let mlRep: usize = ZSTD_count(
                                ip.wrapping_add(4),
                                ip.wrapping_add(4).wrapping_sub(offset_1 as usize),
                                iend,
                            ) + 4;
                            let gain2: c_int = mlRep.wrapping_mul(4) as c_int;
                            let gain1: c_int = matchLength
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                .wrapping_add(1) as c_int;
                            if (mlRep >= 4) && (gain2 > gain1) {
                                matchLength = mlRep;
                                offBase = REPCODE1_TO_OFFBASE as usize;
                                start = ip;
                            }
                        }
                        if isDxS != 0 {
                            let repIndex: U32 =
                                ((ip as isize - base as isize) as U32).wrapping_sub(offset_1);
                            let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                                dictBase.wrapping_offset(
                                    repIndex.wrapping_sub(dictIndexDelta) as isize
                                )
                            } else {
                                base.wrapping_offset(repIndex as isize)
                            };
                            if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                                && (MEM_read32(repMatch as *const c_void)
                                    == MEM_read32(ip as *const c_void))
                            {
                                let repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                                    dictEnd
                                } else {
                                    iend
                                };
                                let mlRep: usize = ZSTD_count_2segments(
                                    ip.wrapping_add(4),
                                    repMatch.wrapping_add(4),
                                    iend,
                                    repMatchEnd,
                                    prefixLowest,
                                ) + 4;
                                let gain2: c_int = mlRep.wrapping_mul(4) as c_int;
                                let gain1: c_int = matchLength
                                    .wrapping_mul(4)
                                    .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                    .wrapping_add(1) as c_int;
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
                            let gain2: c_int = ml2
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as usize)
                                as c_int;
                            let gain1: c_int = matchLength
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                .wrapping_add(7) as c_int;
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

            /* NOTE:
             * Pay attention that `start[-value]` can lead to strange undefined
             * behavior notably if `value` is unsigned, resulting in a large
             * positive `-value`.
             */
            /* catch up */
            if OFFBASE_IS_OFFSET_ST(offBase) {
                if dictMode == ZSTD_noDict {
                    /* only search for offset within prefix */
                    while (((start > anchor) as c_int
                        & (start.wrapping_sub(OFFBASE_TO_OFFSET_ST(offBase)) > prefixLowest)
                            as c_int)
                        != 0)
                        && (*start.wrapping_sub(1)
                            == *start
                                .wrapping_sub(OFFBASE_TO_OFFSET_ST(offBase))
                                .wrapping_sub(1))
                    {
                        start = start.wrapping_sub(1);
                        matchLength += 1;
                    }
                }
                if isDxS != 0 {
                    let matchIndex: U32 = (((start as isize - base as isize) as usize)
                        .wrapping_sub(OFFBASE_TO_OFFSET_ST(offBase)))
                        as U32;
                    let mut match_: *const BYTE = if matchIndex < prefixLowestIndex {
                        dictBase
                            .wrapping_offset(matchIndex as isize)
                            .wrapping_offset(-(dictIndexDelta as isize))
                    } else {
                        base.wrapping_offset(matchIndex as isize)
                    };
                    let mStart: *const BYTE = if matchIndex < prefixLowestIndex {
                        dictLowest
                    } else {
                        prefixLowest
                    };
                    /* catch up */
                    while (start > anchor)
                        && (match_ > mStart)
                        && (*start.wrapping_sub(1) == *match_.wrapping_sub(1))
                    {
                        start = start.wrapping_sub(1);
                        match_ = match_.wrapping_sub(1);
                        matchLength += 1;
                    }
                }
                offset_2 = offset_1;
                offset_1 = OFFBASE_TO_OFFSET_ST(offBase) as U32;
            }
        }

        /* store sequence */
        /* _storeSequence: */
        {
            let litLength: usize = (start as isize - anchor as isize) as usize;
            ZSTD_storeSeq(
                seqStore,
                litLength,
                anchor,
                iend,
                offBase as U32,
                matchLength,
            );
            ip = start.wrapping_add(matchLength);
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
        if isDxS != 0 {
            while ip <= ilimit {
                let current2: U32 = (ip as isize - base as isize) as U32;
                let repIndex: U32 = current2.wrapping_sub(offset_2);
                let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
                    dictBase
                        .wrapping_offset(-(dictIndexDelta as isize))
                        .wrapping_offset(repIndex as isize)
                } else {
                    base.wrapping_offset(repIndex as isize)
                };
                if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                    && (MEM_read32(repMatch as *const c_void)
                        == MEM_read32(ip as *const c_void))
                {
                    let repEnd2: *const BYTE = if repIndex < prefixLowestIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    matchLength = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        repMatch.wrapping_add(4),
                        iend,
                        repEnd2,
                        prefixLowest,
                    ) + 4;
                    /* swap offset_2 <=> offset_1 */
                    offBase = offset_2 as usize;
                    offset_2 = offset_1;
                    offset_1 = offBase as U32;
                    ZSTD_storeSeq(
                        seqStore,
                        0,
                        anchor,
                        iend,
                        REPCODE1_TO_OFFBASE,
                        matchLength,
                    );
                    ip = ip.wrapping_add(matchLength);
                    anchor = ip;
                    continue;
                }
                break;
            }
        }

        if dictMode == ZSTD_noDict {
            while (((ip <= ilimit) as c_int & (offset_2 > 0) as c_int) != 0)
                && (MEM_read32(ip as *const c_void)
                    == MEM_read32(ip.wrapping_sub(offset_2 as usize) as *const c_void))
            {
                /* store sequence */
                matchLength = ZSTD_count(
                    ip.wrapping_add(4),
                    ip.wrapping_add(4).wrapping_sub(offset_2 as usize),
                    iend,
                ) + 4;
                /* swap repcodes */
                offBase = offset_2 as usize;
                offset_2 = offset_1;
                offset_1 = offBase as U32;
                ZSTD_storeSeq(
                    seqStore,
                    0,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE,
                    matchLength,
                );
                ip = ip.wrapping_add(matchLength);
                anchor = ip;
                /* faster when present ... (?) */
            }
        }
    }

    /* If offset_1 started invalid (offsetSaved1 != 0) and became valid
     * (offset_1 != 0), rotate saved offsets. */
    offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
        offsetSaved1
    } else {
        offsetSaved2
    };

    /* save reps for next block */
    *rep.add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
    *rep.add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };

    /* Return the last literals size */
    (iend as isize - anchor as isize) as usize
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
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = if searchMethod == search_rowHash {
        iend.wrapping_sub(8).wrapping_sub(ZSTD_ROW_HASH_CACHE_SIZE)
    } else {
        iend.wrapping_sub(8)
    };
    let base: *const BYTE = (*ms).window.base;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.wrapping_offset(dictLimit as isize);
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictEnd: *const BYTE = dictBase.wrapping_offset(dictLimit as isize);
    let dictStart: *const BYTE = dictBase.wrapping_offset((*ms).window.lowLimit as isize);
    let windowLog: U32 = (*ms).cParams.windowLog;
    let mls: U32 = BOUNDED(4, (*ms).cParams.minMatch, 6);
    let rowLog: U32 = BOUNDED(4, (*ms).cParams.searchLog, 6);

    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);

    /* Reset the lazy skipping state */
    (*ms).lazySkipping = 0;

    /* init */
    ip = ip.wrapping_add((ip == prefixStart) as usize);
    if searchMethod == search_rowHash {
        ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
    }

    /* Match Loop */
    while ip < ilimit {
        let mut matchLength: usize = 0;
        let mut offBase: usize = REPCODE1_TO_OFFBASE as usize;
        let mut start: *const BYTE = ip.wrapping_add(1);
        let mut curr: U32 = (ip as isize - base as isize) as U32;
        let mut goto_storeSequence: bool = false;

        /* check repCode */
        {
            let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr.wrapping_add(1), windowLog);
            let repIndex: U32 = curr.wrapping_add(1).wrapping_sub(offset_1);
            let repBase: *const BYTE = if repIndex < dictLimit { dictBase } else { base };
            let repMatch: *const BYTE = repBase.wrapping_offset(repIndex as isize);
            /* note: we are searching at curr+1 */
            if (ZSTD_index_overlap_check(dictLimit, repIndex)
                & (offset_1 <= curr.wrapping_add(1).wrapping_sub(windowLow)) as c_int)
                != 0
            {
                if MEM_read32(ip.wrapping_add(1) as *const c_void)
                    == MEM_read32(repMatch as *const c_void)
                {
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
                        goto_storeSequence = true;
                    }
                }
            }
        }

        if !goto_storeSequence {
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
                let step: usize = ((ip as isize - anchor as isize) as usize) >> kSearchStrength;
                ip = ip.wrapping_add(step + 1); /* jump faster over incompressible sections */
                /* Enter the lazy skipping mode once we are skipping more than 8
                 * bytes at a time. */
                (*ms).lazySkipping = (step > kLazySkippingStep) as c_int;
                continue;
            }

            /* let's try to find a better solution */
            if depth >= 1 {
                while ip < ilimit {
                    ip = ip.wrapping_add(1);
                    curr = curr.wrapping_add(1);
                    /* check repCode */
                    if offBase != 0 {
                        let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, windowLog);
                        let repIndex: U32 = curr.wrapping_sub(offset_1);
                        let repBase: *const BYTE =
                            if repIndex < dictLimit { dictBase } else { base };
                        let repMatch: *const BYTE = repBase.wrapping_offset(repIndex as isize);
                        /* equivalent to `curr > repIndex >= windowLow` */
                        if (ZSTD_index_overlap_check(dictLimit, repIndex)
                            & (offset_1 <= curr.wrapping_sub(windowLow)) as c_int)
                            != 0
                        {
                            if MEM_read32(ip as *const c_void)
                                == MEM_read32(repMatch as *const c_void)
                            {
                                /* repcode detected */
                                let repEnd: *const BYTE =
                                    if repIndex < dictLimit { dictEnd } else { iend };
                                let repLength: usize = ZSTD_count_2segments(
                                    ip.wrapping_add(4),
                                    repMatch.wrapping_add(4),
                                    iend,
                                    repEnd,
                                    prefixStart,
                                ) + 4;
                                let gain2: c_int = repLength.wrapping_mul(3) as c_int;
                                let gain1: c_int = matchLength
                                    .wrapping_mul(3)
                                    .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                    .wrapping_add(1) as c_int;
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
                        let gain2: c_int = ml2
                            .wrapping_mul(4)
                            .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as usize)
                            as c_int;
                        let gain1: c_int = matchLength
                            .wrapping_mul(4)
                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                            .wrapping_add(4) as c_int;
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
                        curr = curr.wrapping_add(1);
                        /* check repCode */
                        if offBase != 0 {
                            let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, windowLog);
                            let repIndex: U32 = curr.wrapping_sub(offset_1);
                            let repBase: *const BYTE =
                                if repIndex < dictLimit { dictBase } else { base };
                            let repMatch: *const BYTE =
                                repBase.wrapping_offset(repIndex as isize);
                            /* equivalent to `curr > repIndex >= windowLow` */
                            if (ZSTD_index_overlap_check(dictLimit, repIndex)
                                & (offset_1 <= curr.wrapping_sub(windowLow)) as c_int)
                                != 0
                            {
                                if MEM_read32(ip as *const c_void)
                                    == MEM_read32(repMatch as *const c_void)
                                {
                                    /* repcode detected */
                                    let repEnd: *const BYTE =
                                        if repIndex < dictLimit { dictEnd } else { iend };
                                    let repLength: usize = ZSTD_count_2segments(
                                        ip.wrapping_add(4),
                                        repMatch.wrapping_add(4),
                                        iend,
                                        repEnd,
                                        prefixStart,
                                    ) + 4;
                                    let gain2: c_int = repLength.wrapping_mul(4) as c_int;
                                    let gain1: c_int = matchLength
                                        .wrapping_mul(4)
                                        .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                        .wrapping_add(1) as c_int;
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
                            let gain2: c_int = ml2
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as usize)
                                as c_int;
                            let gain1: c_int = matchLength
                                .wrapping_mul(4)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as usize)
                                .wrapping_add(7) as c_int;
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
            if OFFBASE_IS_OFFSET_ST(offBase) {
                let matchIndex: U32 = (((start as isize - base as isize) as usize)
                    .wrapping_sub(OFFBASE_TO_OFFSET_ST(offBase))) as U32;
                let mut match_: *const BYTE = if matchIndex < dictLimit {
                    dictBase.wrapping_offset(matchIndex as isize)
                } else {
                    base.wrapping_offset(matchIndex as isize)
                };
                let mStart: *const BYTE = if matchIndex < dictLimit {
                    dictStart
                } else {
                    prefixStart
                };
                /* catch up */
                while (start > anchor)
                    && (match_ > mStart)
                    && (*start.wrapping_sub(1) == *match_.wrapping_sub(1))
                {
                    start = start.wrapping_sub(1);
                    match_ = match_.wrapping_sub(1);
                    matchLength += 1;
                }
                offset_2 = offset_1;
                offset_1 = OFFBASE_TO_OFFSET_ST(offBase) as U32;
            }
        }

        /* store sequence */
        /* _storeSequence: */
        {
            let litLength: usize = (start as isize - anchor as isize) as usize;
            ZSTD_storeSeq(
                seqStore,
                litLength,
                anchor,
                iend,
                offBase as U32,
                matchLength,
            );
            ip = start.wrapping_add(matchLength);
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
            let repCurrent: U32 = (ip as isize - base as isize) as U32;
            let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, repCurrent, windowLog);
            let repIndex: U32 = repCurrent.wrapping_sub(offset_2);
            let repBase: *const BYTE = if repIndex < dictLimit { dictBase } else { base };
            let repMatch: *const BYTE = repBase.wrapping_offset(repIndex as isize);
            /* equivalent to `curr > repIndex >= windowLow` */
            if (ZSTD_index_overlap_check(dictLimit, repIndex)
                & (offset_2 <= repCurrent.wrapping_sub(windowLow)) as c_int)
                != 0
            {
                if MEM_read32(ip as *const c_void) == MEM_read32(repMatch as *const c_void) {
                    /* repcode detected we should take it */
                    let repEnd: *const BYTE = if repIndex < dictLimit { dictEnd } else { iend };
                    matchLength = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        repMatch.wrapping_add(4),
                        iend,
                        repEnd,
                        prefixStart,
                    ) + 4;
                    /* swap offset history */
                    offBase = offset_2 as usize;
                    offset_2 = offset_1;
                    offset_1 = offBase as U32;
                    ZSTD_storeSeq(
                        seqStore,
                        0,
                        anchor,
                        iend,
                        REPCODE1_TO_OFFBASE,
                        matchLength,
                    );
                    ip = ip.wrapping_add(matchLength);
                    anchor = ip;
                    continue; /* faster when present ... (?) */
                }
            }
            break;
        }
    }

    /* Save reps for next block */
    *rep.add(0) = offset_1;
    *rep.add(1) = offset_2;

    /* Return the last literals size */
    (iend as isize - anchor as isize) as usize
}

/* greedy */
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

/* lazy */
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

/* lazy2 */
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

/* btlazy2 */
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
