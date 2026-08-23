//! Translation of compress/zstd_double_fast.c (+ compress/zstd_double_fast.h)
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens
)]

use crate::compiler::{PREFETCH_AREA, PREFETCH_L1};
use crate::mem::*;
use crate::zstd_compress_internal::{
    kSearchStrength, SeqStore_t, ZSTD_MatchState_t, ZSTD_count, ZSTD_count_2segments,
    ZSTD_dictTableLoadMethod_e, ZSTD_dtlm_fast, ZSTD_getLowestMatchIndex,
    ZSTD_getLowestPrefixIndex, ZSTD_hashPtr, ZSTD_index_overlap_check, ZSTD_selectAddr,
    ZSTD_storeSeq, ZSTD_tableFillPurpose_e, ZSTD_tfp_forCDict, ZSTD_writeTaggedIndex,
    ZSTD_comparePackedTags, HASH_READ_SIZE, OFFSET_TO_OFFBASE, REPCODE1_TO_OFFBASE,
    ZSTD_SHORT_CACHE_TAG_BITS,
};
use crate::zstd_h::ZSTD_compressionParameters;

/* --- internal "goto label" identifiers, used to transliterate the C gotos --- */
const GOTO_NONE: core::ffi::c_int = 0;
const GOTO_CLEANUP: core::ffi::c_int = 1;
const GOTO_SEARCH_NEXT_LONG: core::ffi::c_int = 2;
const GOTO_MATCH_FOUND: core::ffi::c_int = 3;
const GOTO_MATCH_STORED: core::ffi::c_int = 4;

pub unsafe fn ZSTD_fillDoubleHashTableForCDict(
    ms: *mut ZSTD_MatchState_t,
    end: *const core::ffi::c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashLarge: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let mls: U32 = (*cParams).minMatch;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.wrapping_add((*ms).nextToUpdate as usize);
    let iend: *const BYTE = (end as *const BYTE).wrapping_sub(HASH_READ_SIZE);
    let fastHashFillStep: U32 = 3;

    /* Always insert every fastHashFillStep position into the hash tables.
     * Insert the other positions into the large hash table if their entry
     * is empty.
     */
    while ip.wrapping_add(fastHashFillStep as usize).wrapping_sub(1) <= iend {
        let curr: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
        let mut i: U32 = 0;
        while i < fastHashFillStep {
            let smHashAndTag: usize = ZSTD_hashPtr(
                ip.wrapping_add(i as usize) as *const core::ffi::c_void,
                hBitsS,
                mls,
            );
            let lgHashAndTag: usize = ZSTD_hashPtr(
                ip.wrapping_add(i as usize) as *const core::ffi::c_void,
                hBitsL,
                8,
            );
            if i == 0 {
                ZSTD_writeTaggedIndex(hashSmall, smHashAndTag, curr.wrapping_add(i));
            }
            if i == 0 || *hashLarge.add(lgHashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS) == 0 {
                ZSTD_writeTaggedIndex(hashLarge, lgHashAndTag, curr.wrapping_add(i));
            }
            /* Only load extra positions for ZSTD_dtlm_full */
            if dtlm == ZSTD_dtlm_fast {
                break;
            }
            i += 1;
        }
        ip = ip.wrapping_add(fastHashFillStep as usize);
    }
}

pub unsafe fn ZSTD_fillDoubleHashTableForCCtx(
    ms: *mut ZSTD_MatchState_t,
    end: *const core::ffi::c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashLarge: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog;
    let mls: U32 = (*cParams).minMatch;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog;
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.wrapping_add((*ms).nextToUpdate as usize);
    let iend: *const BYTE = (end as *const BYTE).wrapping_sub(HASH_READ_SIZE);
    let fastHashFillStep: U32 = 3;

    /* Always insert every fastHashFillStep position into the hash tables.
     * Insert the other positions into the large hash table if their entry
     * is empty.
     */
    while ip.wrapping_add(fastHashFillStep as usize).wrapping_sub(1) <= iend {
        let curr: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
        let mut i: U32 = 0;
        while i < fastHashFillStep {
            let smHash: usize = ZSTD_hashPtr(
                ip.wrapping_add(i as usize) as *const core::ffi::c_void,
                hBitsS,
                mls,
            );
            let lgHash: usize = ZSTD_hashPtr(
                ip.wrapping_add(i as usize) as *const core::ffi::c_void,
                hBitsL,
                8,
            );
            if i == 0 {
                *hashSmall.add(smHash) = curr.wrapping_add(i);
            }
            if i == 0 || *hashLarge.add(lgHash) == 0 {
                *hashLarge.add(lgHash) = curr.wrapping_add(i);
            }
            /* Only load extra positions for ZSTD_dtlm_full */
            if dtlm == ZSTD_dtlm_fast {
                break;
            }
            i += 1;
        }
        ip = ip.wrapping_add(fastHashFillStep as usize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_fillDoubleHashTable(
    ms: *mut ZSTD_MatchState_t,
    end: *const core::ffi::c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
) {
    if tfp == ZSTD_tfp_forCDict {
        ZSTD_fillDoubleHashTableForCDict(ms, end, dtlm);
    } else {
        ZSTD_fillDoubleHashTableForCCtx(ms, end, dtlm);
    }
}

pub unsafe fn ZSTD_compressBlock_doubleFast_noDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
    mls: U32, /* template */
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashLong: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog;
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 =
        (((istart as usize).wrapping_sub(base as usize)).wrapping_add(srcSize)) as U32;
    /* presumes that, if there is a dictionary, it must be using Attach mode */
    let prefixLowestIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog);
    let prefixLowest: *const BYTE = base.wrapping_add(prefixLowestIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let mut mLength: usize = 0;
    let mut offset: U32 = 0;
    let mut curr: U32 = 0;

    /* how many positions to search before increasing step size */
    let kStepIncr: usize = 1usize << kSearchStrength;
    /* the position at which to increment the step size if no match is found */
    let mut nextStep: *const BYTE = core::ptr::null();
    let mut step: usize = 0; /* the current step size */

    let mut hl0: usize = 0; /* the long hash at ip */
    let mut hl1: usize = 0; /* the long hash at ip1 */

    let mut idxl0: U32 = 0; /* the long match index for ip */
    let mut idxl1: U32 = 0; /* the long match index for ip1 */

    let mut matchl0: *const BYTE = core::ptr::null(); /* the long match for ip */
    let mut matchs0: *const BYTE = core::ptr::null(); /* the short match for ip */
    let mut matchl1: *const BYTE = core::ptr::null(); /* the long match for ip1 */
    let mut matchs0_safe: *const BYTE = core::ptr::null(); /* matchs0 or safe address */

    let mut ip: *const BYTE = istart; /* the current position */
    let mut ip1: *const BYTE = core::ptr::null(); /* the next position */
    /* Array of ~random data, should have low probability of matching data
     * we load from here instead of from tables, if matchl0/matchl1 are
     * invalid indices. Used to avoid unpredictable branches. */
    let dummy: [BYTE; 10] = [
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xe2, 0xb4,
    ];

    /* init */
    ip = ip.wrapping_add((((ip as isize).wrapping_sub(prefixLowest as isize)) == 0) as usize);
    {
        let current: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
        let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, current, (*cParams).windowLog);
        let maxRep: U32 = current.wrapping_sub(windowLow);
        if offset_2 > maxRep {
            offsetSaved2 = offset_2;
            offset_2 = 0;
        }
        if offset_1 > maxRep {
            offsetSaved1 = offset_1;
            offset_1 = 0;
        }
    }

    /* Outer Loop: one iteration per match found and stored */
    loop {
        let mut target: core::ffi::c_int = GOTO_NONE;

        step = 1;
        nextStep = ip.wrapping_add(kStepIncr);
        ip1 = ip.wrapping_add(step);

        if ip1 > ilimit {
            target = GOTO_CLEANUP;
        }

        if target == GOTO_NONE {
            hl0 = ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsL, 8);
            idxl0 = *hashLong.add(hl0);
            matchl0 = base.wrapping_add(idxl0 as usize);

            /* Inner Loop: one iteration per search / position */
            loop {
                let hs0: usize = ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsS, mls);
                let idxs0: U32 = *hashSmall.add(hs0);
                curr = ((ip as usize).wrapping_sub(base as usize)) as U32;
                matchs0 = base.wrapping_add(idxs0 as usize);

                *hashSmall.add(hs0) = curr; /* update hash tables */
                *hashLong.add(hl0) = curr;

                /* check noDict repcode */
                if ((offset_1 > 0) as core::ffi::c_int
                    & (MEM_read32(ip.wrapping_add(1).wrapping_sub(offset_1 as usize))
                        == MEM_read32(ip.wrapping_add(1))) as core::ffi::c_int)
                    != 0
                {
                    mLength = ZSTD_count(
                        ip.wrapping_add(1 + 4),
                        ip.wrapping_add(1 + 4).wrapping_sub(offset_1 as usize),
                        iend,
                    ) + 4;
                    ip = ip.wrapping_add(1);
                    ZSTD_storeSeq(
                        seqStore,
                        (ip as usize).wrapping_sub(anchor as usize),
                        anchor,
                        iend,
                        REPCODE1_TO_OFFBASE,
                        mLength,
                    );
                    target = GOTO_MATCH_STORED;
                    break;
                }

                hl1 = ZSTD_hashPtr(ip1 as *const core::ffi::c_void, hBitsL, 8);

                /* idxl0 > prefixLowestIndex is a (somewhat) unpredictable branch.
                 * However expression below complies into conditional move. Since
                 * match is unlikely and we only *branch* on idxl0 > prefixLowestIndex
                 * if there is a match, all branches become predictable. */
                {
                    let matchl0_safe: *const BYTE =
                        ZSTD_selectAddr(idxl0, prefixLowestIndex, matchl0, dummy.as_ptr());

                    /* check prefix long match */
                    if MEM_read64(matchl0_safe) == MEM_read64(ip) && matchl0_safe == matchl0 {
                        mLength =
                            ZSTD_count(ip.wrapping_add(8), matchl0.wrapping_add(8), iend) + 8;
                        offset = ((ip as usize).wrapping_sub(matchl0 as usize)) as U32;
                        while ((ip > anchor) as core::ffi::c_int
                            & (matchl0 > prefixLowest) as core::ffi::c_int)
                            != 0
                            && (*ip.wrapping_sub(1) == *matchl0.wrapping_sub(1))
                        {
                            ip = ip.wrapping_sub(1);
                            matchl0 = matchl0.wrapping_sub(1);
                            mLength += 1;
                        } /* catch up */
                        target = GOTO_MATCH_FOUND;
                        break;
                    }
                }

                idxl1 = *hashLong.add(hl1);
                matchl1 = base.wrapping_add(idxl1 as usize);

                /* Same optimization as matchl0 above */
                matchs0_safe = ZSTD_selectAddr(idxs0, prefixLowestIndex, matchs0, dummy.as_ptr());

                /* check prefix short match */
                if MEM_read32(matchs0_safe) == MEM_read32(ip) && matchs0_safe == matchs0 {
                    target = GOTO_SEARCH_NEXT_LONG;
                    break;
                }

                if ip1 >= nextStep {
                    PREFETCH_L1(ip1.wrapping_add(64));
                    PREFETCH_L1(ip1.wrapping_add(128));
                    step += 1;
                    nextStep = nextStep.wrapping_add(kStepIncr);
                }
                ip = ip1;
                ip1 = ip1.wrapping_add(step);

                hl0 = hl1;
                idxl0 = idxl1;
                matchl0 = matchl1;

                if !(ip1 <= ilimit) {
                    target = GOTO_CLEANUP;
                    break;
                }
            }
        }

        if target == GOTO_CLEANUP {
            /* _cleanup:
             * If offset_1 started invalid (offsetSaved1 != 0) and became valid (offset_1 != 0),
             * rotate saved offsets. See comment in ZSTD_compressBlock_fast_noDict for more context. */
            offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
                offsetSaved1
            } else {
                offsetSaved2
            };

            /* save reps for next block */
            *rep.add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
            *rep.add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };

            /* Return the last literals size */
            return (iend as usize).wrapping_sub(anchor as usize);
        }

        if target == GOTO_SEARCH_NEXT_LONG {
            /* _search_next_long:
             * short match found: let's check for a longer one */
            mLength = ZSTD_count(ip.wrapping_add(4), matchs0.wrapping_add(4), iend) + 4;
            offset = ((ip as usize).wrapping_sub(matchs0 as usize)) as U32;

            /* check long match at +1 position */
            if (idxl1 > prefixLowestIndex) && (MEM_read64(matchl1) == MEM_read64(ip1)) {
                let l1len: usize =
                    ZSTD_count(ip1.wrapping_add(8), matchl1.wrapping_add(8), iend) + 8;
                if l1len > mLength {
                    /* use the long match instead */
                    ip = ip1;
                    mLength = l1len;
                    offset = ((ip as usize).wrapping_sub(matchl1 as usize)) as U32;
                    matchs0 = matchl1;
                }
            }

            while ((ip > anchor) as core::ffi::c_int
                & (matchs0 > prefixLowest) as core::ffi::c_int)
                != 0
                && (*ip.wrapping_sub(1) == *matchs0.wrapping_sub(1))
            {
                ip = ip.wrapping_sub(1);
                matchs0 = matchs0.wrapping_sub(1);
                mLength += 1;
            } /* complete backward */

            /* fall-through */
            target = GOTO_MATCH_FOUND;
        }

        if target == GOTO_MATCH_FOUND {
            /* _match_found: requires ip, offset, mLength */
            offset_2 = offset_1;
            offset_1 = offset;

            if step < 4 {
                /* It is unsafe to write this value back to the hashtable when ip1 is
                 * greater than or equal to the new ip we will have after we're done
                 * processing this match. Rather than perform that test directly
                 * (ip1 >= ip + mLength), which costs speed in practice, we do a simpler
                 * more predictable test. The minmatch even if we take a short match is
                 * 4 bytes, so as long as step, the distance between ip and ip1
                 * (initially) is less than 4, we know ip1 < new ip. */
                *hashLong.add(hl1) = ((ip1 as usize).wrapping_sub(base as usize)) as U32;
            }

            ZSTD_storeSeq(
                seqStore,
                (ip as usize).wrapping_sub(anchor as usize),
                anchor,
                iend,
                OFFSET_TO_OFFBASE(offset),
                mLength,
            );
        }

        /* _match_stored: match found */
        ip = ip.wrapping_add(mLength);
        anchor = ip;

        if ip <= ilimit {
            /* Complementary insertion */
            /* done after iLimit test, as candidates could be > iend-8 */
            {
                let indexToInsert: U32 = curr.wrapping_add(2);
                *hashLong.add(ZSTD_hashPtr(
                    base.wrapping_add(indexToInsert as usize) as *const core::ffi::c_void,
                    hBitsL,
                    8,
                )) = indexToInsert;
                *hashLong.add(ZSTD_hashPtr(
                    ip.wrapping_sub(2) as *const core::ffi::c_void,
                    hBitsL,
                    8,
                )) = ((ip as usize).wrapping_sub(2).wrapping_sub(base as usize)) as U32;
                *hashSmall.add(ZSTD_hashPtr(
                    base.wrapping_add(indexToInsert as usize) as *const core::ffi::c_void,
                    hBitsS,
                    mls,
                )) = indexToInsert;
                *hashSmall.add(ZSTD_hashPtr(
                    ip.wrapping_sub(1) as *const core::ffi::c_void,
                    hBitsS,
                    mls,
                )) = ((ip as usize).wrapping_sub(1).wrapping_sub(base as usize)) as U32;
            }

            /* check immediate repcode */
            while (ip <= ilimit)
                && (((offset_2 > 0) as core::ffi::c_int
                    & (MEM_read32(ip) == MEM_read32(ip.wrapping_sub(offset_2 as usize)))
                        as core::ffi::c_int)
                    != 0)
            {
                /* store sequence */
                let rLength: usize = ZSTD_count(
                    ip.wrapping_add(4),
                    ip.wrapping_add(4).wrapping_sub(offset_2 as usize),
                    iend,
                ) + 4;
                let tmpOff: U32 = offset_2;
                offset_2 = offset_1;
                offset_1 = tmpOff; /* swap offset_2 <=> offset_1 */
                *hashSmall.add(ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsS, mls)) =
                    ((ip as usize).wrapping_sub(base as usize)) as U32;
                *hashLong.add(ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsL, 8)) =
                    ((ip as usize).wrapping_sub(base as usize)) as U32;
                ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, rLength);
                ip = ip.wrapping_add(rLength);
                anchor = ip;
                continue; /* faster when present ... (?) */
            }
        }
    }
}

pub unsafe fn ZSTD_compressBlock_doubleFast_dictMatchState_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
    mls: U32, /* template */
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashLong: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog;
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 =
        (((istart as usize).wrapping_sub(base as usize)).wrapping_add(srcSize)) as U32;
    /* presumes that, if there is a dictionary, it must be using Attach mode */
    let prefixLowestIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog);
    let prefixLowest: *const BYTE = base.wrapping_add(prefixLowestIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);

    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictCParams: *const ZSTD_compressionParameters = &(*dms).cParams;
    let dictHashLong: *const U32 = (*dms).hashTable;
    let dictHashSmall: *const U32 = (*dms).chainTable;
    let dictStartIndex: U32 = (*dms).window.dictLimit;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictStart: *const BYTE = dictBase.wrapping_add(dictStartIndex as usize);
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictIndexDelta: U32 = prefixLowestIndex
        .wrapping_sub(((dictEnd as usize).wrapping_sub(dictBase as usize)) as U32);
    let dictHBitsL: U32 = (*dictCParams).hashLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let dictHBitsS: U32 = (*dictCParams).chainLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let dictAndPrefixLength: U32 = (((ip as isize).wrapping_sub(prefixLowest as isize))
        .wrapping_add((dictEnd as isize).wrapping_sub(dictStart as isize)))
        as U32;

    if (*ms).prefetchCDictTables != 0 {
        let hashTableBytes: usize =
            (1usize << (*dictCParams).hashLog) * core::mem::size_of::<U32>();
        let chainTableBytes: usize =
            (1usize << (*dictCParams).chainLog) * core::mem::size_of::<U32>();
        PREFETCH_AREA(dictHashLong as *const BYTE, hashTableBytes);
        PREFETCH_AREA(dictHashSmall as *const BYTE, chainTableBytes);
    }

    /* init */
    ip = ip.wrapping_add((dictAndPrefixLength == 0) as usize);

    /* Main Search Loop */
    while ip < ilimit {
        /* < instead of <=, because repcode check at (ip+1) */
        let mut mLength: usize = 0;
        let mut offset: U32 = 0;
        let h2: usize = ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsL, 8);
        let h: usize = ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsS, mls);
        let dictHashAndTagL: usize =
            ZSTD_hashPtr(ip as *const core::ffi::c_void, dictHBitsL, 8);
        let dictHashAndTagS: usize =
            ZSTD_hashPtr(ip as *const core::ffi::c_void, dictHBitsS, mls);
        let dictMatchIndexAndTagL: U32 =
            *dictHashLong.add(dictHashAndTagL >> ZSTD_SHORT_CACHE_TAG_BITS);
        let dictMatchIndexAndTagS: U32 =
            *dictHashSmall.add(dictHashAndTagS >> ZSTD_SHORT_CACHE_TAG_BITS);
        let dictTagsMatchL: core::ffi::c_int =
            ZSTD_comparePackedTags(dictMatchIndexAndTagL as usize, dictHashAndTagL);
        let dictTagsMatchS: core::ffi::c_int =
            ZSTD_comparePackedTags(dictMatchIndexAndTagS as usize, dictHashAndTagS);
        let curr: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
        let matchIndexL: U32 = *hashLong.add(h2);
        let mut matchIndexS: U32 = *hashSmall.add(h);
        let mut matchLong: *const BYTE = base.wrapping_add(matchIndexL as usize);
        let mut r#match: *const BYTE = base.wrapping_add(matchIndexS as usize);
        let repIndex: U32 = curr.wrapping_add(1).wrapping_sub(offset_1);
        let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
            dictBase.wrapping_add(repIndex.wrapping_sub(dictIndexDelta) as usize)
        } else {
            base.wrapping_add(repIndex as usize)
        };
        *hashSmall.add(h) = curr; /* update hash tables */
        *hashLong.add(h2) = curr;

        let mut target: core::ffi::c_int = GOTO_NONE;

        /* check repcode */
        if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
            && (MEM_read32(repMatch) == MEM_read32(ip.wrapping_add(1)))
        {
            let repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                dictEnd
            } else {
                iend
            };
            mLength = ZSTD_count_2segments(
                ip.wrapping_add(1 + 4),
                repMatch.wrapping_add(4),
                iend,
                repMatchEnd,
                prefixLowest,
            ) + 4;
            ip = ip.wrapping_add(1);
            ZSTD_storeSeq(
                seqStore,
                (ip as usize).wrapping_sub(anchor as usize),
                anchor,
                iend,
                REPCODE1_TO_OFFBASE,
                mLength,
            );
            target = GOTO_MATCH_STORED;
        }

        if target == GOTO_NONE {
            if (matchIndexL >= prefixLowestIndex) && (MEM_read64(matchLong) == MEM_read64(ip)) {
                /* check prefix long match */
                mLength = ZSTD_count(ip.wrapping_add(8), matchLong.wrapping_add(8), iend) + 8;
                offset = ((ip as usize).wrapping_sub(matchLong as usize)) as U32;
                while ((ip > anchor) as core::ffi::c_int
                    & (matchLong > prefixLowest) as core::ffi::c_int)
                    != 0
                    && (*ip.wrapping_sub(1) == *matchLong.wrapping_sub(1))
                {
                    ip = ip.wrapping_sub(1);
                    matchLong = matchLong.wrapping_sub(1);
                    mLength += 1;
                } /* catch up */
                target = GOTO_MATCH_FOUND;
            } else if dictTagsMatchL != 0 {
                /* check dictMatchState long match */
                let dictMatchIndexL: U32 = dictMatchIndexAndTagL >> ZSTD_SHORT_CACHE_TAG_BITS;
                let mut dictMatchL: *const BYTE = dictBase.wrapping_add(dictMatchIndexL as usize);

                if dictMatchL > dictStart && MEM_read64(dictMatchL) == MEM_read64(ip) {
                    mLength = ZSTD_count_2segments(
                        ip.wrapping_add(8),
                        dictMatchL.wrapping_add(8),
                        iend,
                        dictEnd,
                        prefixLowest,
                    ) + 8;
                    offset = curr
                        .wrapping_sub(dictMatchIndexL)
                        .wrapping_sub(dictIndexDelta);
                    while ((ip > anchor) as core::ffi::c_int
                        & (dictMatchL > dictStart) as core::ffi::c_int)
                        != 0
                        && (*ip.wrapping_sub(1) == *dictMatchL.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        dictMatchL = dictMatchL.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                    target = GOTO_MATCH_FOUND;
                }
            }
        }

        if target == GOTO_NONE {
            if matchIndexS > prefixLowestIndex {
                /* short match candidate */
                if MEM_read32(r#match) == MEM_read32(ip) {
                    target = GOTO_SEARCH_NEXT_LONG;
                }
            } else if dictTagsMatchS != 0 {
                /* check dictMatchState short match */
                let dictMatchIndexS: U32 = dictMatchIndexAndTagS >> ZSTD_SHORT_CACHE_TAG_BITS;
                r#match = dictBase.wrapping_add(dictMatchIndexS as usize);
                matchIndexS = dictMatchIndexS.wrapping_add(dictIndexDelta);

                if r#match > dictStart && MEM_read32(r#match) == MEM_read32(ip) {
                    target = GOTO_SEARCH_NEXT_LONG;
                }
            }

            if target == GOTO_NONE {
                ip = ip.wrapping_offset(
                    (((ip as isize).wrapping_sub(anchor as isize)) >> kSearchStrength) + 1,
                );
                continue;
            }
        }

        if target == GOTO_SEARCH_NEXT_LONG {
            /* _search_next_long: */
            {
                let hl3: usize =
                    ZSTD_hashPtr(ip.wrapping_add(1) as *const core::ffi::c_void, hBitsL, 8);
                let dictHashAndTagL3: usize =
                    ZSTD_hashPtr(ip.wrapping_add(1) as *const core::ffi::c_void, dictHBitsL, 8);
                let matchIndexL3: U32 = *hashLong.add(hl3);
                let dictMatchIndexAndTagL3: U32 =
                    *dictHashLong.add(dictHashAndTagL3 >> ZSTD_SHORT_CACHE_TAG_BITS);
                let dictTagsMatchL3: core::ffi::c_int =
                    ZSTD_comparePackedTags(dictMatchIndexAndTagL3 as usize, dictHashAndTagL3);
                let mut matchL3: *const BYTE = base.wrapping_add(matchIndexL3 as usize);
                *hashLong.add(hl3) = curr.wrapping_add(1);

                /* check prefix long +1 match */
                if (matchIndexL3 >= prefixLowestIndex)
                    && (MEM_read64(matchL3) == MEM_read64(ip.wrapping_add(1)))
                {
                    mLength = ZSTD_count(ip.wrapping_add(9), matchL3.wrapping_add(8), iend) + 8;
                    ip = ip.wrapping_add(1);
                    offset = ((ip as usize).wrapping_sub(matchL3 as usize)) as U32;
                    while ((ip > anchor) as core::ffi::c_int
                        & (matchL3 > prefixLowest) as core::ffi::c_int)
                        != 0
                        && (*ip.wrapping_sub(1) == *matchL3.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        matchL3 = matchL3.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                    target = GOTO_MATCH_FOUND;
                } else if dictTagsMatchL3 != 0 {
                    /* check dict long +1 match */
                    let dictMatchIndexL3: U32 =
                        dictMatchIndexAndTagL3 >> ZSTD_SHORT_CACHE_TAG_BITS;
                    let mut dictMatchL3: *const BYTE =
                        dictBase.wrapping_add(dictMatchIndexL3 as usize);
                    if dictMatchL3 > dictStart
                        && MEM_read64(dictMatchL3) == MEM_read64(ip.wrapping_add(1))
                    {
                        mLength = ZSTD_count_2segments(
                            ip.wrapping_add(1 + 8),
                            dictMatchL3.wrapping_add(8),
                            iend,
                            dictEnd,
                            prefixLowest,
                        ) + 8;
                        ip = ip.wrapping_add(1);
                        offset = curr
                            .wrapping_add(1)
                            .wrapping_sub(dictMatchIndexL3)
                            .wrapping_sub(dictIndexDelta);
                        while ((ip > anchor) as core::ffi::c_int
                            & (dictMatchL3 > dictStart) as core::ffi::c_int)
                            != 0
                            && (*ip.wrapping_sub(1) == *dictMatchL3.wrapping_sub(1))
                        {
                            ip = ip.wrapping_sub(1);
                            dictMatchL3 = dictMatchL3.wrapping_sub(1);
                            mLength += 1;
                        } /* catch up */
                        target = GOTO_MATCH_FOUND;
                    }
                }
            }

            if target == GOTO_SEARCH_NEXT_LONG {
                /* if no long +1 match, explore the short match we found */
                if matchIndexS < prefixLowestIndex {
                    mLength = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        r#match.wrapping_add(4),
                        iend,
                        dictEnd,
                        prefixLowest,
                    ) + 4;
                    offset = curr.wrapping_sub(matchIndexS);
                    while ((ip > anchor) as core::ffi::c_int
                        & (r#match > dictStart) as core::ffi::c_int)
                        != 0
                        && (*ip.wrapping_sub(1) == *r#match.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        r#match = r#match.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                } else {
                    mLength = ZSTD_count(ip.wrapping_add(4), r#match.wrapping_add(4), iend) + 4;
                    offset = ((ip as usize).wrapping_sub(r#match as usize)) as U32;
                    while ((ip > anchor) as core::ffi::c_int
                        & (r#match > prefixLowest) as core::ffi::c_int)
                        != 0
                        && (*ip.wrapping_sub(1) == *r#match.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        r#match = r#match.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                }
                target = GOTO_MATCH_FOUND;
            }
        }

        if target == GOTO_MATCH_FOUND {
            /* _match_found: */
            offset_2 = offset_1;
            offset_1 = offset;

            ZSTD_storeSeq(
                seqStore,
                (ip as usize).wrapping_sub(anchor as usize),
                anchor,
                iend,
                OFFSET_TO_OFFBASE(offset),
                mLength,
            );
        }

        /* _match_stored: match found */
        ip = ip.wrapping_add(mLength);
        anchor = ip;

        if ip <= ilimit {
            /* Complementary insertion */
            /* done after iLimit test, as candidates could be > iend-8 */
            {
                let indexToInsert: U32 = curr.wrapping_add(2);
                *hashLong.add(ZSTD_hashPtr(
                    base.wrapping_add(indexToInsert as usize) as *const core::ffi::c_void,
                    hBitsL,
                    8,
                )) = indexToInsert;
                *hashLong.add(ZSTD_hashPtr(
                    ip.wrapping_sub(2) as *const core::ffi::c_void,
                    hBitsL,
                    8,
                )) = ((ip as usize).wrapping_sub(2).wrapping_sub(base as usize)) as U32;
                *hashSmall.add(ZSTD_hashPtr(
                    base.wrapping_add(indexToInsert as usize) as *const core::ffi::c_void,
                    hBitsS,
                    mls,
                )) = indexToInsert;
                *hashSmall.add(ZSTD_hashPtr(
                    ip.wrapping_sub(1) as *const core::ffi::c_void,
                    hBitsS,
                    mls,
                )) = ((ip as usize).wrapping_sub(1).wrapping_sub(base as usize)) as U32;
            }

            /* check immediate repcode */
            while ip <= ilimit {
                let current2: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
                let repIndex2: U32 = current2.wrapping_sub(offset_2);
                let repMatch2: *const BYTE = if repIndex2 < prefixLowestIndex {
                    dictBase
                        .wrapping_add(repIndex2 as usize)
                        .wrapping_sub(dictIndexDelta as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex2) != 0)
                    && (MEM_read32(repMatch2) == MEM_read32(ip))
                {
                    let repEnd2: *const BYTE = if repIndex2 < prefixLowestIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    let repLength2: usize = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        repMatch2.wrapping_add(4),
                        iend,
                        repEnd2,
                        prefixLowest,
                    ) + 4;
                    let tmpOffset: U32 = offset_2;
                    offset_2 = offset_1;
                    offset_1 = tmpOffset; /* swap offset_2 <=> offset_1 */
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, repLength2);
                    *hashSmall.add(ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsS, mls)) =
                        current2;
                    *hashLong.add(ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsL, 8)) =
                        current2;
                    ip = ip.wrapping_add(repLength2);
                    anchor = ip;
                    continue;
                }
                break;
            }
        }
    } /* while (ip < ilimit) */

    /* save reps for next block */
    *rep.add(0) = offset_1;
    *rep.add(1) = offset_2;

    /* Return the last literals size */
    (iend as usize).wrapping_sub(anchor as usize)
}

/* ZSTD_GEN_DFAST_FN(noDict, 4) */
pub unsafe fn ZSTD_compressBlock_doubleFast_noDict_4(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 4)
}

/* ZSTD_GEN_DFAST_FN(noDict, 5) */
pub unsafe fn ZSTD_compressBlock_doubleFast_noDict_5(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 5)
}

/* ZSTD_GEN_DFAST_FN(noDict, 6) */
pub unsafe fn ZSTD_compressBlock_doubleFast_noDict_6(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 6)
}

/* ZSTD_GEN_DFAST_FN(noDict, 7) */
pub unsafe fn ZSTD_compressBlock_doubleFast_noDict_7(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 7)
}

/* ZSTD_GEN_DFAST_FN(dictMatchState, 4) */
pub unsafe fn ZSTD_compressBlock_doubleFast_dictMatchState_4(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 4)
}

/* ZSTD_GEN_DFAST_FN(dictMatchState, 5) */
pub unsafe fn ZSTD_compressBlock_doubleFast_dictMatchState_5(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 5)
}

/* ZSTD_GEN_DFAST_FN(dictMatchState, 6) */
pub unsafe fn ZSTD_compressBlock_doubleFast_dictMatchState_6(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 6)
}

/* ZSTD_GEN_DFAST_FN(dictMatchState, 7) */
pub unsafe fn ZSTD_compressBlock_doubleFast_dictMatchState_7(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 7)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_noDict_5(ms, seqStore, rep, src, srcSize),
        6 => ZSTD_compressBlock_doubleFast_noDict_6(ms, seqStore, rep, src, srcSize),
        7 => ZSTD_compressBlock_doubleFast_noDict_7(ms, seqStore, rep, src, srcSize),
        /* default: includes case 3 */
        _ => ZSTD_compressBlock_doubleFast_noDict_4(ms, seqStore, rep, src, srcSize),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_dictMatchState_5(ms, seqStore, rep, src, srcSize),
        6 => ZSTD_compressBlock_doubleFast_dictMatchState_6(ms, seqStore, rep, src, srcSize),
        7 => ZSTD_compressBlock_doubleFast_dictMatchState_7(ms, seqStore, rep, src, srcSize),
        /* default: includes case 3 */
        _ => ZSTD_compressBlock_doubleFast_dictMatchState_4(ms, seqStore, rep, src, srcSize),
    }
}

pub unsafe fn ZSTD_compressBlock_doubleFast_extDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
    mls: U32, /* template */
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashLong: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(8);
    let base: *const BYTE = (*ms).window.base;
    let endIndex: U32 =
        (((istart as usize).wrapping_sub(base as usize)).wrapping_add(srcSize)) as U32;
    let lowLimit: U32 = ZSTD_getLowestMatchIndex(ms, endIndex, (*cParams).windowLog);
    let dictStartIndex: U32 = lowLimit;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStartIndex: U32 = if dictLimit > lowLimit {
        dictLimit
    } else {
        lowLimit
    };
    let prefixStart: *const BYTE = base.wrapping_add(prefixStartIndex as usize);
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictStart: *const BYTE = dictBase.wrapping_add(dictStartIndex as usize);
    let dictEnd: *const BYTE = dictBase.wrapping_add(prefixStartIndex as usize);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);

    /* if extDict is invalidated due to maxDistance, switch to "regular" variant */
    if prefixStartIndex == dictStartIndex {
        return ZSTD_compressBlock_doubleFast(ms, seqStore, rep, src, srcSize);
    }

    /* Search Loop */
    while ip < ilimit {
        /* < instead of <=, because (ip+1) */
        let hSmall: usize = ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsS, mls);
        let matchIndex: U32 = *hashSmall.add(hSmall);
        let matchBase: *const BYTE = if matchIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let mut r#match: *const BYTE = matchBase.wrapping_add(matchIndex as usize);

        let hLong: usize = ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsL, 8);
        let matchLongIndex: U32 = *hashLong.add(hLong);
        let matchLongBase: *const BYTE = if matchLongIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let mut matchLong: *const BYTE = matchLongBase.wrapping_add(matchLongIndex as usize);

        let curr: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
        let repIndex: U32 = curr.wrapping_add(1).wrapping_sub(offset_1); /* offset_1 expected <= curr +1 */
        let repBase: *const BYTE = if repIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let repMatch: *const BYTE = repBase.wrapping_add(repIndex as usize);
        let mut mLength: usize = 0;
        *hashLong.add(hLong) = curr; /* update hash table */
        *hashSmall.add(hSmall) = curr;

        if ((ZSTD_index_overlap_check(prefixStartIndex, repIndex)
            & ((offset_1 <= curr.wrapping_add(1).wrapping_sub(dictStartIndex))
                as core::ffi::c_int))
            != 0)
            && (MEM_read32(repMatch) == MEM_read32(ip.wrapping_add(1)))
        {
            let repMatchEnd: *const BYTE = if repIndex < prefixStartIndex {
                dictEnd
            } else {
                iend
            };
            mLength = ZSTD_count_2segments(
                ip.wrapping_add(1 + 4),
                repMatch.wrapping_add(4),
                iend,
                repMatchEnd,
                prefixStart,
            ) + 4;
            ip = ip.wrapping_add(1);
            ZSTD_storeSeq(
                seqStore,
                (ip as usize).wrapping_sub(anchor as usize),
                anchor,
                iend,
                REPCODE1_TO_OFFBASE,
                mLength,
            );
        } else {
            if (matchLongIndex > dictStartIndex) && (MEM_read64(matchLong) == MEM_read64(ip)) {
                let matchEnd: *const BYTE = if matchLongIndex < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                let lowMatchPtr: *const BYTE = if matchLongIndex < prefixStartIndex {
                    dictStart
                } else {
                    prefixStart
                };
                let mut offset: U32;
                mLength = ZSTD_count_2segments(
                    ip.wrapping_add(8),
                    matchLong.wrapping_add(8),
                    iend,
                    matchEnd,
                    prefixStart,
                ) + 8;
                offset = curr.wrapping_sub(matchLongIndex);
                while ((ip > anchor) as core::ffi::c_int
                    & (matchLong > lowMatchPtr) as core::ffi::c_int)
                    != 0
                    && (*ip.wrapping_sub(1) == *matchLong.wrapping_sub(1))
                {
                    ip = ip.wrapping_sub(1);
                    matchLong = matchLong.wrapping_sub(1);
                    mLength += 1;
                } /* catch up */
                offset_2 = offset_1;
                offset_1 = offset;
                ZSTD_storeSeq(
                    seqStore,
                    (ip as usize).wrapping_sub(anchor as usize),
                    anchor,
                    iend,
                    OFFSET_TO_OFFBASE(offset),
                    mLength,
                );
            } else if (matchIndex > dictStartIndex) && (MEM_read32(r#match) == MEM_read32(ip)) {
                let h3: usize =
                    ZSTD_hashPtr(ip.wrapping_add(1) as *const core::ffi::c_void, hBitsL, 8);
                let matchIndex3: U32 = *hashLong.add(h3);
                let match3Base: *const BYTE = if matchIndex3 < prefixStartIndex {
                    dictBase
                } else {
                    base
                };
                let mut match3: *const BYTE = match3Base.wrapping_add(matchIndex3 as usize);
                let mut offset: U32;
                *hashLong.add(h3) = curr.wrapping_add(1);
                if (matchIndex3 > dictStartIndex)
                    && (MEM_read64(match3) == MEM_read64(ip.wrapping_add(1)))
                {
                    let matchEnd: *const BYTE = if matchIndex3 < prefixStartIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    let lowMatchPtr: *const BYTE = if matchIndex3 < prefixStartIndex {
                        dictStart
                    } else {
                        prefixStart
                    };
                    mLength = ZSTD_count_2segments(
                        ip.wrapping_add(9),
                        match3.wrapping_add(8),
                        iend,
                        matchEnd,
                        prefixStart,
                    ) + 8;
                    ip = ip.wrapping_add(1);
                    offset = curr.wrapping_add(1).wrapping_sub(matchIndex3);
                    while ((ip > anchor) as core::ffi::c_int
                        & (match3 > lowMatchPtr) as core::ffi::c_int)
                        != 0
                        && (*ip.wrapping_sub(1) == *match3.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        match3 = match3.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                } else {
                    let matchEnd: *const BYTE = if matchIndex < prefixStartIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    let lowMatchPtr: *const BYTE = if matchIndex < prefixStartIndex {
                        dictStart
                    } else {
                        prefixStart
                    };
                    mLength = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        r#match.wrapping_add(4),
                        iend,
                        matchEnd,
                        prefixStart,
                    ) + 4;
                    offset = curr.wrapping_sub(matchIndex);
                    while ((ip > anchor) as core::ffi::c_int
                        & (r#match > lowMatchPtr) as core::ffi::c_int)
                        != 0
                        && (*ip.wrapping_sub(1) == *r#match.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        r#match = r#match.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                }
                offset_2 = offset_1;
                offset_1 = offset;
                ZSTD_storeSeq(
                    seqStore,
                    (ip as usize).wrapping_sub(anchor as usize),
                    anchor,
                    iend,
                    OFFSET_TO_OFFBASE(offset),
                    mLength,
                );
            } else {
                ip = ip.wrapping_offset(
                    (((ip as isize).wrapping_sub(anchor as isize)) >> kSearchStrength) + 1,
                );
                continue;
            }
        }

        /* move to next sequence start */
        ip = ip.wrapping_add(mLength);
        anchor = ip;

        if ip <= ilimit {
            /* Complementary insertion */
            /* done after iLimit test, as candidates could be > iend-8 */
            {
                let indexToInsert: U32 = curr.wrapping_add(2);
                *hashLong.add(ZSTD_hashPtr(
                    base.wrapping_add(indexToInsert as usize) as *const core::ffi::c_void,
                    hBitsL,
                    8,
                )) = indexToInsert;
                *hashLong.add(ZSTD_hashPtr(
                    ip.wrapping_sub(2) as *const core::ffi::c_void,
                    hBitsL,
                    8,
                )) = ((ip as usize).wrapping_sub(2).wrapping_sub(base as usize)) as U32;
                *hashSmall.add(ZSTD_hashPtr(
                    base.wrapping_add(indexToInsert as usize) as *const core::ffi::c_void,
                    hBitsS,
                    mls,
                )) = indexToInsert;
                *hashSmall.add(ZSTD_hashPtr(
                    ip.wrapping_sub(1) as *const core::ffi::c_void,
                    hBitsS,
                    mls,
                )) = ((ip as usize).wrapping_sub(1).wrapping_sub(base as usize)) as U32;
            }

            /* check immediate repcode */
            while ip <= ilimit {
                let current2: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
                let repIndex2: U32 = current2.wrapping_sub(offset_2);
                let repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictBase.wrapping_add(repIndex2 as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if ((ZSTD_index_overlap_check(prefixStartIndex, repIndex2)
                    & ((offset_2 <= current2.wrapping_sub(dictStartIndex))
                        as core::ffi::c_int))
                    != 0)
                    && (MEM_read32(repMatch2) == MEM_read32(ip))
                {
                    let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    let repLength2: usize = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        repMatch2.wrapping_add(4),
                        iend,
                        repEnd2,
                        prefixStart,
                    ) + 4;
                    let tmpOffset: U32 = offset_2;
                    offset_2 = offset_1;
                    offset_1 = tmpOffset; /* swap offset_2 <=> offset_1 */
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, repLength2);
                    *hashSmall.add(ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsS, mls)) =
                        current2;
                    *hashLong.add(ZSTD_hashPtr(ip as *const core::ffi::c_void, hBitsL, 8)) =
                        current2;
                    ip = ip.wrapping_add(repLength2);
                    anchor = ip;
                    continue;
                }
                break;
            }
        }
    }

    /* save reps for next block */
    *rep.add(0) = offset_1;
    *rep.add(1) = offset_2;

    /* Return the last literals size */
    (iend as usize).wrapping_sub(anchor as usize)
}

/* ZSTD_GEN_DFAST_FN(extDict, 4) */
pub unsafe fn ZSTD_compressBlock_doubleFast_extDict_4(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 4)
}

/* ZSTD_GEN_DFAST_FN(extDict, 5) */
pub unsafe fn ZSTD_compressBlock_doubleFast_extDict_5(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 5)
}

/* ZSTD_GEN_DFAST_FN(extDict, 6) */
pub unsafe fn ZSTD_compressBlock_doubleFast_extDict_6(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 6)
}

/* ZSTD_GEN_DFAST_FN(extDict, 7) */
pub unsafe fn ZSTD_compressBlock_doubleFast_extDict_7(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 7)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_extDict_5(ms, seqStore, rep, src, srcSize),
        6 => ZSTD_compressBlock_doubleFast_extDict_6(ms, seqStore, rep, src, srcSize),
        7 => ZSTD_compressBlock_doubleFast_extDict_7(ms, seqStore, rep, src, srcSize),
        /* default: includes case 3 */
        _ => ZSTD_compressBlock_doubleFast_extDict_4(ms, seqStore, rep, src, srcSize),
    }
}
