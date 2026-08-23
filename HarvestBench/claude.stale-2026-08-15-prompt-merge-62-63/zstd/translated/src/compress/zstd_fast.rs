//! Translation of compress/zstd_fast.c — "fast" match finder.

use core::ffi::c_void;

use crate::common::mem::{mem_read32, U32};
use crate::compress::zstd_compress_internal::{
    kSearchStrength, ZSTD_comparePackedTags, ZSTD_count, ZSTD_count_2segments,
    ZSTD_dictTableLoadMethod_e, ZSTD_dtlm_fast, ZSTD_dtlm_full, ZSTD_getLowestMatchIndex,
    ZSTD_getLowestPrefixIndex, ZSTD_hashPtr, ZSTD_index_overlap_check, ZSTD_selectAddr,
    ZSTD_storeSeq, ZSTD_tableFillPurpose_e, ZSTD_tfp_forCDict, ZSTD_writeTaggedIndex,
    SeqStore_t, HASH_READ_SIZE, OFFSET_TO_OFFBASE, REPCODE1_TO_OFFBASE, ZSTD_SHORT_CACHE_TAG_BITS,
};

// Alias to the real name used in the shared module.
use crate::compress::zstd_compress_internal::ZSTD_MatchState_t as MatchStateAlias;

#[inline]
unsafe fn MEM_read32(p: *const u8) -> U32 {
    mem_read32(p as *const c_void)
}

/* PREFETCH hints have no observable effect on output; treat as no-ops. */
#[inline]
fn PREFETCH_L1(_p: *const u8) {}
#[inline]
fn PREFETCH_AREA(_p: *const U32, _bytes: usize) {}

#[inline]
unsafe fn ZSTD_fillHashTableForCDict(
    ms: *mut MatchStateAlias,
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams = &(*ms).cParams;
    let hashTable = (*ms).hashTable;
    let hBits: U32 = cParams.hashLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let mls: U32 = cParams.minMatch;
    let base = (*ms).window.base;
    let mut ip = base.add((*ms).nextToUpdate as usize);
    let iend = (end as *const u8).sub(HASH_READ_SIZE);
    let fastHashFillStep: U32 = 3;

    /* Currently, we always use ZSTD_dtlm_full for filling CDict tables. */
    debug_assert!(dtlm == ZSTD_dtlm_full);

    while ip.add(fastHashFillStep as usize) < iend.add(2) {
        let curr = ip.offset_from(base) as U32;
        {
            let hashAndTag = ZSTD_hashPtr(ip as *const c_void, hBits, mls);
            ZSTD_writeTaggedIndex(hashTable, hashAndTag, curr);
        }

        if dtlm == ZSTD_dtlm_fast {
            ip = ip.add(fastHashFillStep as usize);
            continue;
        }
        /* Only load extra positions for ZSTD_dtlm_full */
        {
            let mut p: U32 = 1;
            while p < fastHashFillStep {
                let hashAndTag = ZSTD_hashPtr(ip.add(p as usize) as *const c_void, hBits, mls);
                if *hashTable.add(hashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS) == 0 {
                    ZSTD_writeTaggedIndex(hashTable, hashAndTag, curr + p);
                }
                p += 1;
            }
        }
        ip = ip.add(fastHashFillStep as usize);
    }
}

#[inline]
unsafe fn ZSTD_fillHashTableForCCtx(
    ms: *mut MatchStateAlias,
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams = &(*ms).cParams;
    let hashTable = (*ms).hashTable;
    let hBits: U32 = cParams.hashLog;
    let mls: U32 = cParams.minMatch;
    let base = (*ms).window.base;
    let mut ip = base.add((*ms).nextToUpdate as usize);
    let iend = (end as *const u8).sub(HASH_READ_SIZE);
    let fastHashFillStep: U32 = 3;

    /* Currently, we always use ZSTD_dtlm_fast for filling CCtx tables. */
    debug_assert!(dtlm == ZSTD_dtlm_fast);

    while ip.add(fastHashFillStep as usize) < iend.add(2) {
        let curr = ip.offset_from(base) as U32;
        let hash0 = ZSTD_hashPtr(ip as *const c_void, hBits, mls);
        *hashTable.add(hash0) = curr;
        if dtlm == ZSTD_dtlm_fast {
            ip = ip.add(fastHashFillStep as usize);
            continue;
        }
        /* Only load extra positions for ZSTD_dtlm_full */
        {
            let mut p: U32 = 1;
            while p < fastHashFillStep {
                let hash = ZSTD_hashPtr(ip.add(p as usize) as *const c_void, hBits, mls);
                if *hashTable.add(hash) == 0 {
                    *hashTable.add(hash) = curr + p;
                }
                p += 1;
            }
        }
        ip = ip.add(fastHashFillStep as usize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_fillHashTable(
    ms: *mut MatchStateAlias,
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
) {
    if tfp == ZSTD_tfp_forCDict {
        ZSTD_fillHashTableForCDict(ms, end, dtlm);
    } else {
        ZSTD_fillHashTableForCCtx(ms, end, dtlm);
    }
}

type ZSTD_match4Found =
    unsafe fn(currentPtr: *const u8, matchAddress: *const u8, matchIdx: U32, idxLowLimit: U32) -> i32;

unsafe fn ZSTD_match4Found_cmov(
    currentPtr: *const u8,
    matchAddress: *const u8,
    matchIdx: U32,
    idxLowLimit: U32,
) -> i32 {
    /* Array of ~random data, low probability of matching data. */
    static DUMMY: [u8; 4] = [0x12, 0x34, 0x56, 0x78];

    let mvalAddr = ZSTD_selectAddr(matchIdx, idxLowLimit, matchAddress, DUMMY.as_ptr());
    if MEM_read32(currentPtr) != MEM_read32(mvalAddr) {
        return 0;
    }
    (matchIdx >= idxLowLimit) as i32
}

unsafe fn ZSTD_match4Found_branch(
    currentPtr: *const u8,
    matchAddress: *const u8,
    matchIdx: U32,
    idxLowLimit: U32,
) -> i32 {
    let mval: U32;
    if matchIdx >= idxLowLimit {
        mval = MEM_read32(matchAddress);
    } else {
        mval = MEM_read32(currentPtr) ^ 1; /* guaranteed to not match. */
    }
    (MEM_read32(currentPtr) == mval) as i32
}

enum Outcome {
    Cleanup,
    Offset,
    Match,
}

unsafe fn ZSTD_compressBlock_fast_noDict_generic(
    ms: *mut MatchStateAlias,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32,
    useCmov: i32,
) -> usize {
    let cParams = &(*ms).cParams;
    let hashTable = (*ms).hashTable;
    let hlog = cParams.hashLog;
    let stepSize: usize =
        (cParams.targetLength + (cParams.targetLength == 0) as U32 + 1) as usize; /* min 2 */
    let base = (*ms).window.base;
    let istart = src as *const u8;
    let endIndex: U32 = ((istart.offset_from(base) as usize) + srcSize) as U32;
    let prefixStartIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, cParams.windowLog);
    let prefixStart = base.add(prefixStartIndex as usize);
    let iend = istart.add(srcSize);
    let ilimit = iend.sub(HASH_READ_SIZE);

    let mut anchor = istart;
    let mut ip0 = istart;
    let mut ip1;
    let mut ip2;
    let mut ip3;
    let mut current0: U32 = 0;

    let mut rep_offset1: U32 = *rep.add(0);
    let mut rep_offset2: U32 = *rep.add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let mut hash0: usize;
    let mut hash1: usize;
    let mut matchIdx: U32 = 0;

    let mut offcode: U32 = 0;
    let mut match0: *const u8 = core::ptr::null();
    let mut mLength: usize = 0;

    let mut step: usize;
    let mut nextStep: *const u8;
    let kStepIncr: usize = 1 << (kSearchStrength - 1);
    let matchFound: ZSTD_match4Found = if useCmov != 0 {
        ZSTD_match4Found_cmov
    } else {
        ZSTD_match4Found_branch
    };

    ip0 = ip0.add((ip0 == prefixStart) as usize);
    {
        let curr = ip0.offset_from(base) as U32;
        let windowLow = ZSTD_getLowestPrefixIndex(ms, curr, cParams.windowLog);
        let maxRep = curr - windowLow;
        if rep_offset2 > maxRep {
            offsetSaved2 = rep_offset2;
            rep_offset2 = 0;
        }
        if rep_offset1 > maxRep {
            offsetSaved1 = rep_offset1;
            rep_offset1 = 0;
        }
    }

    'start: loop {
        // _start: Requires: ip0
        step = stepSize;
        nextStep = ip0.add(kStepIncr);

        /* calculate positions, ip0 - anchor == 0, so we skip step calc */
        ip1 = ip0.add(1);
        ip2 = ip0.add(step);
        ip3 = ip2.add(1);

        let outcome: Outcome;

        if ip3 >= ilimit {
            outcome = Outcome::Cleanup;
        } else {
            hash0 = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);
            hash1 = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);

            matchIdx = *hashTable.add(hash0);

            outcome = 'search: loop {
                /* load repcode match for ip[2] */
                let rval = MEM_read32(ip2.wrapping_sub(rep_offset1 as usize));

                /* write back hash table entry */
                current0 = ip0.offset_from(base) as U32;
                *hashTable.add(hash0) = current0;

                /* check repcode at ip[2] */
                if ((MEM_read32(ip2) == rval) as i32 & (rep_offset1 > 0) as i32) != 0 {
                    ip0 = ip2;
                    match0 = ip0.wrapping_sub(rep_offset1 as usize);
                    mLength = (*ip0.offset(-1) == *match0.offset(-1)) as usize;
                    ip0 = ip0.sub(mLength);
                    match0 = match0.wrapping_sub(mLength);
                    offcode = REPCODE1_TO_OFFBASE;
                    mLength += 4;

                    *hashTable.add(hash1) = ip1.offset_from(base) as U32;

                    break 'search Outcome::Match;
                }

                if matchFound(
                    ip0,
                    base.wrapping_add(matchIdx as usize),
                    matchIdx,
                    prefixStartIndex,
                ) != 0
                {
                    *hashTable.add(hash1) = ip1.offset_from(base) as U32;
                    break 'search Outcome::Offset;
                }

                /* lookup ip[1] */
                matchIdx = *hashTable.add(hash1);

                /* hash ip[2] */
                hash0 = hash1;
                hash1 = ZSTD_hashPtr(ip2 as *const c_void, hlog, mls);

                /* advance to next positions */
                ip0 = ip1;
                ip1 = ip2;
                ip2 = ip3;

                /* write back hash table entry */
                current0 = ip0.offset_from(base) as U32;
                *hashTable.add(hash0) = current0;

                if matchFound(
                    ip0,
                    base.wrapping_add(matchIdx as usize),
                    matchIdx,
                    prefixStartIndex,
                ) != 0
                {
                    if step <= 4 {
                        *hashTable.add(hash1) = ip1.offset_from(base) as U32;
                    }
                    break 'search Outcome::Offset;
                }

                /* lookup ip[1] */
                matchIdx = *hashTable.add(hash1);

                /* hash ip[2] */
                hash0 = hash1;
                hash1 = ZSTD_hashPtr(ip2 as *const c_void, hlog, mls);

                /* advance to next positions */
                ip0 = ip1;
                ip1 = ip2;
                ip2 = ip0.add(step);
                ip3 = ip1.add(step);

                /* calculate step */
                if ip2 >= nextStep {
                    step += 1;
                    PREFETCH_L1(ip1.add(64));
                    PREFETCH_L1(ip1.add(128));
                    nextStep = nextStep.add(kStepIncr);
                }

                if !(ip3 < ilimit) {
                    break 'search Outcome::Cleanup;
                }
            };
        }

        match outcome {
            Outcome::Cleanup => {
                // _cleanup
                offsetSaved2 = if (offsetSaved1 != 0) && (rep_offset1 != 0) {
                    offsetSaved1
                } else {
                    offsetSaved2
                };

                *rep.add(0) = if rep_offset1 != 0 { rep_offset1 } else { offsetSaved1 };
                *rep.add(1) = if rep_offset2 != 0 { rep_offset2 } else { offsetSaved2 };

                return iend.offset_from(anchor) as usize;
            }
            Outcome::Offset => {
                // _offset: Requires: ip0, idx
                match0 = base.wrapping_add(matchIdx as usize);
                rep_offset2 = rep_offset1;
                rep_offset1 = ip0.offset_from(match0) as U32;
                offcode = OFFSET_TO_OFFBASE(rep_offset1);
                mLength = 4;

                /* Count the backwards match length. */
                while ((ip0 > anchor) as i32 & (match0 > prefixStart) as i32) != 0
                    && (*ip0.offset(-1) == *match0.offset(-1))
                {
                    ip0 = ip0.sub(1);
                    match0 = match0.sub(1);
                    mLength += 1;
                }
                // falls through to _match
            }
            Outcome::Match => {
                // match0, offcode, mLength already set
            }
        }

        // _match: Requires: ip0, match0, offcode
        mLength += ZSTD_count(ip0.add(mLength), match0.add(mLength), iend);

        ZSTD_storeSeq(
            seqStore,
            ip0.offset_from(anchor) as usize,
            anchor,
            iend,
            offcode,
            mLength,
        );

        ip0 = ip0.add(mLength);
        anchor = ip0;

        /* Fill table and check for immediate repcode. */
        if ip0 <= ilimit {
            /* Fill Table */
            debug_assert!(base.add(current0 as usize + 2) > istart);
            *hashTable.add(ZSTD_hashPtr(
                base.add(current0 as usize + 2) as *const c_void,
                hlog,
                mls,
            )) = current0 + 2;
            *hashTable.add(ZSTD_hashPtr(ip0.sub(2) as *const c_void, hlog, mls)) =
                ip0.sub(2).offset_from(base) as U32;

            if rep_offset2 > 0 {
                while (ip0 <= ilimit)
                    && (MEM_read32(ip0) == MEM_read32(ip0.wrapping_sub(rep_offset2 as usize)))
                {
                    /* store sequence */
                    let rLength = ZSTD_count(
                        ip0.add(4),
                        ip0.add(4).wrapping_sub(rep_offset2 as usize),
                        iend,
                    ) + 4;
                    let tmpOff = rep_offset2;
                    rep_offset2 = rep_offset1;
                    rep_offset1 = tmpOff;
                    *hashTable.add(ZSTD_hashPtr(ip0 as *const c_void, hlog, mls)) =
                        ip0.offset_from(base) as U32;
                    ip0 = ip0.add(rLength);
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, rLength);
                    anchor = ip0;
                    continue;
                }
            }
        }

        // goto _start
        continue 'start;
    }
}

macro_rules! gen_fast_noDict {
    ($name:ident, $mml:expr, $cmov:expr) => {
        unsafe fn $name(
            ms: *mut MatchStateAlias,
            seqStore: *mut SeqStore_t,
            rep: *mut U32,
            src: *const c_void,
            srcSize: usize,
        ) -> usize {
            ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, $mml, $cmov)
        }
    };
}

gen_fast_noDict!(ZSTD_compressBlock_fast_noDict_4_1, 4, 1);
gen_fast_noDict!(ZSTD_compressBlock_fast_noDict_5_1, 5, 1);
gen_fast_noDict!(ZSTD_compressBlock_fast_noDict_6_1, 6, 1);
gen_fast_noDict!(ZSTD_compressBlock_fast_noDict_7_1, 7, 1);

gen_fast_noDict!(ZSTD_compressBlock_fast_noDict_4_0, 4, 0);
gen_fast_noDict!(ZSTD_compressBlock_fast_noDict_5_0, 5, 0);
gen_fast_noDict!(ZSTD_compressBlock_fast_noDict_6_0, 6, 0);
gen_fast_noDict!(ZSTD_compressBlock_fast_noDict_7_0, 7, 0);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast(
    ms: *mut MatchStateAlias,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mml = (*ms).cParams.minMatch;
    /* use cmov when "candidate in range" branch is likely unpredictable */
    let useCmov = ((*ms).cParams.windowLog < 19) as i32;
    debug_assert!((*ms).dictMatchState.is_null());
    if useCmov != 0 {
        match mml {
            5 => ZSTD_compressBlock_fast_noDict_5_1(ms, seqStore, rep, src, srcSize),
            6 => ZSTD_compressBlock_fast_noDict_6_1(ms, seqStore, rep, src, srcSize),
            7 => ZSTD_compressBlock_fast_noDict_7_1(ms, seqStore, rep, src, srcSize),
            _ => ZSTD_compressBlock_fast_noDict_4_1(ms, seqStore, rep, src, srcSize),
        }
    } else {
        /* use a branch instead */
        match mml {
            5 => ZSTD_compressBlock_fast_noDict_5_0(ms, seqStore, rep, src, srcSize),
            6 => ZSTD_compressBlock_fast_noDict_6_0(ms, seqStore, rep, src, srcSize),
            7 => ZSTD_compressBlock_fast_noDict_7_0(ms, seqStore, rep, src, srcSize),
            _ => ZSTD_compressBlock_fast_noDict_4_0(ms, seqStore, rep, src, srcSize),
        }
    }
}

unsafe fn ZSTD_compressBlock_fast_dictMatchState_generic(
    ms: *mut MatchStateAlias,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32,
    _hasStep: U32,
) -> usize {
    let cParams = &(*ms).cParams;
    let hashTable = (*ms).hashTable;
    let hlog = cParams.hashLog;
    /* support stepSize of 0 */
    let stepSize: U32 = cParams.targetLength + (cParams.targetLength == 0) as U32;
    let base = (*ms).window.base;
    let istart = src as *const u8;
    let mut ip0 = istart;
    let mut ip1 = ip0.add(stepSize as usize); /* we assert below that stepSize >= 1 */
    let mut anchor = istart;
    let prefixStartIndex: U32 = (*ms).window.dictLimit;
    let prefixStart = base.add(prefixStartIndex as usize);
    let iend = istart.add(srcSize);
    let ilimit = iend.sub(HASH_READ_SIZE);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);

    let dms = (*ms).dictMatchState;
    let dictCParams = &(*dms).cParams;
    let dictHashTable = (*dms).hashTable;
    let dictStartIndex: U32 = (*dms).window.dictLimit;
    let dictBase = (*dms).window.base;
    let dictStart = dictBase.add(dictStartIndex as usize);
    let dictEnd = (*dms).window.nextSrc;
    let dictIndexDelta: U32 = prefixStartIndex.wrapping_sub(dictEnd.offset_from(dictBase) as U32);
    let dictAndPrefixLength: U32 =
        (istart.offset_from(prefixStart) + dictEnd.offset_from(dictStart)) as U32;
    let dictHBits: U32 = dictCParams.hashLog + ZSTD_SHORT_CACHE_TAG_BITS;

    /* if a dictionary is still attached, it necessarily means that
     * it is within window size. So we just check it. */
    let maxDistance: U32 = 1u32 << cParams.windowLog;
    let endIndex: U32 = ((istart.offset_from(base) as usize) + srcSize) as U32;
    debug_assert!(endIndex - prefixStartIndex <= maxDistance);
    let _ = (maxDistance, endIndex);

    /* ensure there will be no underflow
     * when translating a dict index into a local index */
    debug_assert!(prefixStartIndex >= dictEnd.offset_from(dictBase) as U32);

    if (*ms).prefetchCDictTables != 0 {
        let hashTableBytes = ((1usize) << dictCParams.hashLog) * core::mem::size_of::<U32>();
        PREFETCH_AREA(dictHashTable, hashTableBytes);
    }

    /* init */
    ip0 = ip0.add((dictAndPrefixLength == 0) as usize);
    /* dictMatchState repCode checks don't currently handle repCode == 0 disabling. */
    debug_assert!(offset_1 <= dictAndPrefixLength);
    debug_assert!(offset_2 <= dictAndPrefixLength);

    /* Outer search loop */
    debug_assert!(stepSize >= 1);
    'outer: while ip1 <= ilimit {
        /* repcode check at (ip0 + 1) is safe because ip0 < ip1 */
        let mut hash0 = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);

        let dictHashAndTag0 = ZSTD_hashPtr(ip0 as *const c_void, dictHBits, mls);
        let mut dictMatchIndexAndTag =
            *dictHashTable.add(dictHashAndTag0 >> ZSTD_SHORT_CACHE_TAG_BITS);
        let mut dictTagsMatch =
            ZSTD_comparePackedTags(dictMatchIndexAndTag as usize, dictHashAndTag0);

        let mut matchIndex = *hashTable.add(hash0);
        let mut curr = ip0.offset_from(base) as U32;
        let mut step = stepSize;
        let kStepIncr: usize = 1 << kSearchStrength;
        let mut nextStep = ip0.add(kStepIncr);

        /* Inner search loop; breaks with the match length when a match is found. */
        let mLength: usize = loop {
            let r#match = base.wrapping_add(matchIndex as usize);
            let repIndex = (curr + 1).wrapping_sub(offset_1);
            let repMatch = if repIndex < prefixStartIndex {
                dictBase.wrapping_add(repIndex.wrapping_sub(dictIndexDelta) as usize)
            } else {
                base.wrapping_add(repIndex as usize)
            };
            let hash1 = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);
            let dictHashAndTag1 = ZSTD_hashPtr(ip1 as *const c_void, dictHBits, mls);
            *hashTable.add(hash0) = curr; /* update hash table */

            if (ZSTD_index_overlap_check(prefixStartIndex, repIndex) != 0)
                && (MEM_read32(repMatch) == MEM_read32(ip0.add(1)))
            {
                let repMatchEnd = if repIndex < prefixStartIndex { dictEnd } else { iend };
                let mLen = ZSTD_count_2segments(
                    ip0.add(1 + 4),
                    repMatch.add(4),
                    iend,
                    repMatchEnd,
                    prefixStart,
                ) + 4;
                ip0 = ip0.add(1);
                ZSTD_storeSeq(
                    seqStore,
                    ip0.offset_from(anchor) as usize,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE,
                    mLen,
                );
                break mLen;
            }

            if dictTagsMatch != 0 {
                /* Found a possible dict match */
                let dictMatchIndex = dictMatchIndexAndTag >> ZSTD_SHORT_CACHE_TAG_BITS;
                let mut dictMatch = dictBase.wrapping_add(dictMatchIndex as usize);
                if dictMatchIndex > dictStartIndex && MEM_read32(dictMatch) == MEM_read32(ip0) {
                    /* To replicate extDict parse behavior, we only use dict matches when the normal matchIndex is invalid */
                    if matchIndex <= prefixStartIndex {
                        let offset =
                            curr.wrapping_sub(dictMatchIndex).wrapping_sub(dictIndexDelta);
                        let mut mLen = ZSTD_count_2segments(
                            ip0.add(4),
                            dictMatch.add(4),
                            iend,
                            dictEnd,
                            prefixStart,
                        ) + 4;
                        while ((ip0 > anchor) as i32 & (dictMatch > dictStart) as i32) != 0
                            && (*ip0.offset(-1) == *dictMatch.offset(-1))
                        {
                            ip0 = ip0.sub(1);
                            dictMatch = dictMatch.sub(1);
                            mLen += 1;
                        }
                        offset_2 = offset_1;
                        offset_1 = offset;
                        ZSTD_storeSeq(
                            seqStore,
                            ip0.offset_from(anchor) as usize,
                            anchor,
                            iend,
                            OFFSET_TO_OFFBASE(offset),
                            mLen,
                        );
                        break mLen;
                    }
                }
            }

            if ZSTD_match4Found_cmov(ip0, r#match, matchIndex, prefixStartIndex) != 0 {
                /* found a regular match of size >= 4 */
                let offset = ip0.offset_from(r#match) as U32;
                let mut mLen = ZSTD_count(ip0.add(4), r#match.add(4), iend) + 4;
                let mut match_p = r#match;
                while ((ip0 > anchor) as i32 & (match_p > prefixStart) as i32) != 0
                    && (*ip0.offset(-1) == *match_p.offset(-1))
                {
                    ip0 = ip0.sub(1);
                    match_p = match_p.sub(1);
                    mLen += 1;
                }
                offset_2 = offset_1;
                offset_1 = offset;
                ZSTD_storeSeq(
                    seqStore,
                    ip0.offset_from(anchor) as usize,
                    anchor,
                    iend,
                    OFFSET_TO_OFFBASE(offset),
                    mLen,
                );
                break mLen;
            }

            /* Prepare for next iteration */
            dictMatchIndexAndTag =
                *dictHashTable.add(dictHashAndTag1 >> ZSTD_SHORT_CACHE_TAG_BITS);
            dictTagsMatch =
                ZSTD_comparePackedTags(dictMatchIndexAndTag as usize, dictHashAndTag1);
            matchIndex = *hashTable.add(hash1);

            if ip1 >= nextStep {
                step += 1;
                nextStep = nextStep.add(kStepIncr);
            }
            ip0 = ip1;
            ip1 = ip1.add(step as usize);
            if ip1 > ilimit {
                break 'outer; // goto _cleanup
            }

            curr = ip0.offset_from(base) as U32;
            hash0 = hash1;
        }; /* end inner search loop */

        /* match found */
        debug_assert!(mLength != 0);
        ip0 = ip0.add(mLength);
        anchor = ip0;

        if ip0 <= ilimit {
            /* Fill Table */
            debug_assert!(base.add(curr as usize + 2) > istart);
            *hashTable.add(ZSTD_hashPtr(
                base.add(curr as usize + 2) as *const c_void,
                hlog,
                mls,
            )) = curr + 2;
            *hashTable.add(ZSTD_hashPtr(ip0.sub(2) as *const c_void, hlog, mls)) =
                ip0.sub(2).offset_from(base) as U32;

            /* check immediate repcode */
            while ip0 <= ilimit {
                let current2 = ip0.offset_from(base) as U32;
                let repIndex2 = current2.wrapping_sub(offset_2);
                let repMatch2 = if repIndex2 < prefixStartIndex {
                    dictBase
                        .wrapping_sub(dictIndexDelta as usize)
                        .wrapping_add(repIndex2 as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if (ZSTD_index_overlap_check(prefixStartIndex, repIndex2) != 0)
                    && (MEM_read32(repMatch2) == MEM_read32(ip0))
                {
                    let repEnd2 = if repIndex2 < prefixStartIndex { dictEnd } else { iend };
                    let repLength2 = ZSTD_count_2segments(
                        ip0.add(4),
                        repMatch2.add(4),
                        iend,
                        repEnd2,
                        prefixStart,
                    ) + 4;
                    let tmpOffset = offset_2;
                    offset_2 = offset_1;
                    offset_1 = tmpOffset;
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, repLength2);
                    *hashTable.add(ZSTD_hashPtr(ip0 as *const c_void, hlog, mls)) = current2;
                    ip0 = ip0.add(repLength2);
                    anchor = ip0;
                    continue;
                }
                break;
            }
        }

        /* Prepare for next iteration */
        debug_assert!(ip0 == anchor);
        ip1 = ip0.add(stepSize as usize);
    }

    // _cleanup
    *rep.add(0) = offset_1;
    *rep.add(1) = offset_2;

    iend.offset_from(anchor) as usize
}

macro_rules! gen_fast_dictMatchState {
    ($name:ident, $mml:expr, $step:expr) => {
        unsafe fn $name(
            ms: *mut MatchStateAlias,
            seqStore: *mut SeqStore_t,
            rep: *mut U32,
            src: *const c_void,
            srcSize: usize,
        ) -> usize {
            ZSTD_compressBlock_fast_dictMatchState_generic(
                ms, seqStore, rep, src, srcSize, $mml, $step,
            )
        }
    };
}

gen_fast_dictMatchState!(ZSTD_compressBlock_fast_dictMatchState_4_0, 4, 0);
gen_fast_dictMatchState!(ZSTD_compressBlock_fast_dictMatchState_5_0, 5, 0);
gen_fast_dictMatchState!(ZSTD_compressBlock_fast_dictMatchState_6_0, 6, 0);
gen_fast_dictMatchState!(ZSTD_compressBlock_fast_dictMatchState_7_0, 7, 0);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState(
    ms: *mut MatchStateAlias,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mls = (*ms).cParams.minMatch;
    debug_assert!(!(*ms).dictMatchState.is_null());
    match mls {
        5 => ZSTD_compressBlock_fast_dictMatchState_5_0(ms, seqStore, rep, src, srcSize),
        6 => ZSTD_compressBlock_fast_dictMatchState_6_0(ms, seqStore, rep, src, srcSize),
        7 => ZSTD_compressBlock_fast_dictMatchState_7_0(ms, seqStore, rep, src, srcSize),
        _ => ZSTD_compressBlock_fast_dictMatchState_4_0(ms, seqStore, rep, src, srcSize),
    }
}

unsafe fn ZSTD_compressBlock_fast_extDict_generic(
    ms: *mut MatchStateAlias,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32,
    _hasStep: U32,
) -> usize {
    let cParams = &(*ms).cParams;
    let hashTable = (*ms).hashTable;
    let hlog = cParams.hashLog;
    /* support stepSize of 0 */
    let stepSize: usize =
        (cParams.targetLength + (cParams.targetLength == 0) as U32 + 1) as usize;
    let base = (*ms).window.base;
    let dictBase = (*ms).window.dictBase;
    let istart = src as *const u8;
    let mut anchor = istart;
    let endIndex: U32 = ((istart.offset_from(base) as usize) + srcSize) as U32;
    let lowLimit: U32 = ZSTD_getLowestMatchIndex(ms, endIndex, cParams.windowLog);
    let dictStartIndex: U32 = lowLimit;
    let dictStart = dictBase.add(dictStartIndex as usize);
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStartIndex: U32 = if dictLimit < lowLimit { lowLimit } else { dictLimit };
    let prefixStart = base.add(prefixStartIndex as usize);
    let dictEnd = dictBase.add(prefixStartIndex as usize);
    let iend = istart.add(srcSize);
    let ilimit = iend.sub(8);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let mut ip0 = istart;
    let mut ip1;
    let mut ip2;
    let mut ip3;
    let mut current0: U32 = 0;

    let mut hash0: usize;
    let mut hash1: usize = 0;
    let mut idx: U32 = 0;
    let mut idxBase: *const u8 = core::ptr::null();

    let mut offcode: U32 = 0;
    let mut match0: *const u8 = core::ptr::null();
    let mut mLength: usize = 0;
    let mut matchEnd: *const u8 = core::ptr::null();

    let mut step: usize;
    let mut nextStep: *const u8;
    let kStepIncr: usize = 1 << (kSearchStrength - 1);

    /* switch to "regular" variant if extDict is invalidated due to maxDistance */
    if prefixStartIndex == dictStartIndex {
        return ZSTD_compressBlock_fast(ms, seqStore, rep, src, srcSize);
    }

    {
        let curr = ip0.offset_from(base) as U32;
        let maxRep = curr - dictStartIndex;
        if offset_2 >= maxRep {
            offsetSaved2 = offset_2;
            offset_2 = 0;
        }
        if offset_1 >= maxRep {
            offsetSaved1 = offset_1;
            offset_1 = 0;
        }
    }

    'start: loop {
        // _start: Requires: ip0
        step = stepSize;
        nextStep = ip0.add(kStepIncr);

        ip1 = ip0.add(1);
        ip2 = ip0.add(step);
        ip3 = ip2.add(1);

        let outcome: Outcome;

        if ip3 >= ilimit {
            outcome = Outcome::Cleanup;
        } else {
            hash0 = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);
            hash1 = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);

            idx = *hashTable.add(hash0);
            idxBase = if idx < prefixStartIndex { dictBase } else { base };

            outcome = 'search: loop {
                {
                    /* load repcode match for ip[2] */
                    let current2 = ip2.offset_from(base) as U32;
                    let repIndex = current2.wrapping_sub(offset_1);
                    let repBase = if repIndex < prefixStartIndex { dictBase } else { base };
                    let rval: U32;
                    if ((prefixStartIndex.wrapping_sub(repIndex) >= 4) as i32
                        & (offset_1 > 0) as i32)
                        != 0
                    {
                        rval = MEM_read32(repBase.wrapping_add(repIndex as usize));
                    } else {
                        rval = MEM_read32(ip2) ^ 1; /* guaranteed to not match. */
                    }

                    /* write back hash table entry */
                    current0 = ip0.offset_from(base) as U32;
                    *hashTable.add(hash0) = current0;

                    /* check repcode at ip[2] */
                    if MEM_read32(ip2) == rval {
                        ip0 = ip2;
                        match0 = repBase.wrapping_add(repIndex as usize);
                        matchEnd = if repIndex < prefixStartIndex { dictEnd } else { iend };
                        debug_assert!(
                            ((match0 != prefixStart) as i32 & (match0 != dictStart) as i32) != 0
                        );
                        mLength = (*ip0.offset(-1) == *match0.offset(-1)) as usize;
                        ip0 = ip0.sub(mLength);
                        match0 = match0.wrapping_sub(mLength);
                        offcode = REPCODE1_TO_OFFBASE;
                        mLength += 4;
                        break 'search Outcome::Match;
                    }
                }

                {
                    /* load match for ip[0] */
                    let mval: U32 = if idx >= dictStartIndex {
                        MEM_read32(idxBase.wrapping_add(idx as usize))
                    } else {
                        MEM_read32(ip0) ^ 1 /* guaranteed not to match */
                    };

                    /* check match at ip[0] */
                    if MEM_read32(ip0) == mval {
                        /* found a match! */
                        break 'search Outcome::Offset;
                    }
                }

                /* lookup ip[1] */
                idx = *hashTable.add(hash1);
                idxBase = if idx < prefixStartIndex { dictBase } else { base };

                /* hash ip[2] */
                hash0 = hash1;
                hash1 = ZSTD_hashPtr(ip2 as *const c_void, hlog, mls);

                /* advance to next positions */
                ip0 = ip1;
                ip1 = ip2;
                ip2 = ip3;

                /* write back hash table entry */
                current0 = ip0.offset_from(base) as U32;
                *hashTable.add(hash0) = current0;

                {
                    /* load match for ip[0] */
                    let mval: U32 = if idx >= dictStartIndex {
                        MEM_read32(idxBase.wrapping_add(idx as usize))
                    } else {
                        MEM_read32(ip0) ^ 1 /* guaranteed not to match */
                    };

                    /* check match at ip[0] */
                    if MEM_read32(ip0) == mval {
                        /* found a match! */
                        break 'search Outcome::Offset;
                    }
                }

                /* lookup ip[1] */
                idx = *hashTable.add(hash1);
                idxBase = if idx < prefixStartIndex { dictBase } else { base };

                /* hash ip[2] */
                hash0 = hash1;
                hash1 = ZSTD_hashPtr(ip2 as *const c_void, hlog, mls);

                /* advance to next positions */
                ip0 = ip1;
                ip1 = ip2;
                ip2 = ip0.add(step);
                ip3 = ip1.add(step);

                /* calculate step */
                if ip2 >= nextStep {
                    step += 1;
                    PREFETCH_L1(ip1.add(64));
                    PREFETCH_L1(ip1.add(128));
                    nextStep = nextStep.add(kStepIncr);
                }

                if !(ip3 < ilimit) {
                    break 'search Outcome::Cleanup;
                }
            };
        }

        match outcome {
            Outcome::Cleanup => {
                // _cleanup
                offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
                    offsetSaved1
                } else {
                    offsetSaved2
                };

                *rep.add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
                *rep.add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };

                return iend.offset_from(anchor) as usize;
            }
            Outcome::Offset => {
                // _offset: Requires: ip0, idx, idxBase
                let offset = current0.wrapping_sub(idx);
                let lowMatchPtr = if idx < prefixStartIndex { dictStart } else { prefixStart };
                matchEnd = if idx < prefixStartIndex { dictEnd } else { iend };
                match0 = idxBase.wrapping_add(idx as usize);
                offset_2 = offset_1;
                offset_1 = offset;
                offcode = OFFSET_TO_OFFBASE(offset);
                mLength = 4;

                /* Count the backwards match length. */
                while ((ip0 > anchor) as i32 & (match0 > lowMatchPtr) as i32) != 0
                    && (*ip0.offset(-1) == *match0.offset(-1))
                {
                    ip0 = ip0.sub(1);
                    match0 = match0.sub(1);
                    mLength += 1;
                }
                // falls through to _match
            }
            Outcome::Match => {
                // match0, offcode, matchEnd, mLength already set
            }
        }

        // _match: Requires: ip0, match0, offcode, matchEnd
        debug_assert!(!matchEnd.is_null());
        mLength += ZSTD_count_2segments(
            ip0.add(mLength),
            match0.add(mLength),
            iend,
            matchEnd,
            prefixStart,
        );

        ZSTD_storeSeq(
            seqStore,
            ip0.offset_from(anchor) as usize,
            anchor,
            iend,
            offcode,
            mLength,
        );

        ip0 = ip0.add(mLength);
        anchor = ip0;

        /* write next hash table entry */
        if ip1 < ip0 {
            *hashTable.add(hash1) = ip1.offset_from(base) as U32;
        }

        /* Fill table and check for immediate repcode. */
        if ip0 <= ilimit {
            /* Fill Table */
            debug_assert!(base.add(current0 as usize + 2) > istart);
            *hashTable.add(ZSTD_hashPtr(
                base.add(current0 as usize + 2) as *const c_void,
                hlog,
                mls,
            )) = current0 + 2;
            *hashTable.add(ZSTD_hashPtr(ip0.sub(2) as *const c_void, hlog, mls)) =
                ip0.sub(2).offset_from(base) as U32;

            while ip0 <= ilimit {
                let repIndex2 = (ip0.offset_from(base) as U32).wrapping_sub(offset_2);
                let repMatch2 = if repIndex2 < prefixStartIndex {
                    dictBase.wrapping_add(repIndex2 as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if ((ZSTD_index_overlap_check(prefixStartIndex, repIndex2) & (offset_2 > 0) as i32)
                    != 0)
                    && (MEM_read32(repMatch2) == MEM_read32(ip0))
                {
                    let repEnd2 = if repIndex2 < prefixStartIndex { dictEnd } else { iend };
                    let repLength2 = ZSTD_count_2segments(
                        ip0.add(4),
                        repMatch2.add(4),
                        iend,
                        repEnd2,
                        prefixStart,
                    ) + 4;
                    let tmpOffset = offset_2;
                    offset_2 = offset_1;
                    offset_1 = tmpOffset;
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, repLength2);
                    *hashTable.add(ZSTD_hashPtr(ip0 as *const c_void, hlog, mls)) =
                        ip0.offset_from(base) as U32;
                    ip0 = ip0.add(repLength2);
                    anchor = ip0;
                    continue;
                }
                break;
            }
        }

        // goto _start
        continue 'start;
    }
}

macro_rules! gen_fast_extDict {
    ($name:ident, $mml:expr, $step:expr) => {
        unsafe fn $name(
            ms: *mut MatchStateAlias,
            seqStore: *mut SeqStore_t,
            rep: *mut U32,
            src: *const c_void,
            srcSize: usize,
        ) -> usize {
            ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, $mml, $step)
        }
    };
}

gen_fast_extDict!(ZSTD_compressBlock_fast_extDict_4_0, 4, 0);
gen_fast_extDict!(ZSTD_compressBlock_fast_extDict_5_0, 5, 0);
gen_fast_extDict!(ZSTD_compressBlock_fast_extDict_6_0, 6, 0);
gen_fast_extDict!(ZSTD_compressBlock_fast_extDict_7_0, 7, 0);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast_extDict(
    ms: *mut MatchStateAlias,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mls = (*ms).cParams.minMatch;
    debug_assert!((*ms).dictMatchState.is_null());
    match mls {
        5 => ZSTD_compressBlock_fast_extDict_5_0(ms, seqStore, rep, src, srcSize),
        6 => ZSTD_compressBlock_fast_extDict_6_0(ms, seqStore, rep, src, srcSize),
        7 => ZSTD_compressBlock_fast_extDict_7_0(ms, seqStore, rep, src, srcSize),
        _ => ZSTD_compressBlock_fast_extDict_4_0(ms, seqStore, rep, src, srcSize),
    }
}
