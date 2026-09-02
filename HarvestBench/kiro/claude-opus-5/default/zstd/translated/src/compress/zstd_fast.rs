//! Translation of `compress/zstd_fast.c` (FAST match finder, compression level 1).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use crate::common::mem::*;
use crate::common::zstd_h::ZSTD_compressionParameters;
use crate::compress::zstd_compress_internal::*;

use core::ffi::{c_int, c_void};

/* ===  ZSTD_fillHashTable  =============================================== */

pub unsafe fn ZSTD_fillHashTableForCDict(
    ms: *mut ZSTD_MatchState_t,
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hBits: U32 = (*cParams).hashLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let mls: U32 = (*cParams).minMatch;
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.wrapping_add((*ms).nextToUpdate as usize);
    let iend: *const BYTE = (end as *const BYTE).wrapping_sub(HASH_READ_SIZE as usize);
    let fastHashFillStep: U32 = 3;

    /* Always insert every fastHashFillStep position into the hash table.
     * Insert the other positions if their hash entry is empty.
     */
    while ip.wrapping_add(fastHashFillStep as usize) < iend.wrapping_add(2) {
        let curr: U32 = ip.offset_from(base) as U32;
        {
            let hashAndTag: size_t = ZSTD_hashPtr(ip as *const c_void, hBits, mls);
            ZSTD_writeTaggedIndex(hashTable, hashAndTag, curr);
        }

        if dtlm != ZSTD_dtlm_fast {
            /* Only load extra positions for ZSTD_dtlm_full */
            let mut p: U32 = 1;
            while p < fastHashFillStep {
                let hashAndTag: size_t =
                    ZSTD_hashPtr(ip.wrapping_add(p as usize) as *const c_void, hBits, mls);
                if *hashTable.wrapping_add((hashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS) as usize) == 0
                {
                    /* not yet filled */
                    ZSTD_writeTaggedIndex(hashTable, hashAndTag, curr + p);
                }
                p += 1;
            }
        }

        ip = ip.wrapping_add(fastHashFillStep as usize);
    }
}

pub unsafe fn ZSTD_fillHashTableForCCtx(
    ms: *mut ZSTD_MatchState_t,
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hBits: U32 = (*cParams).hashLog;
    let mls: U32 = (*cParams).minMatch;
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.wrapping_add((*ms).nextToUpdate as usize);
    let iend: *const BYTE = (end as *const BYTE).wrapping_sub(HASH_READ_SIZE as usize);
    let fastHashFillStep: U32 = 3;

    while ip.wrapping_add(fastHashFillStep as usize) < iend.wrapping_add(2) {
        let curr: U32 = ip.offset_from(base) as U32;
        let hash0: size_t = ZSTD_hashPtr(ip as *const c_void, hBits, mls);
        *hashTable.wrapping_add(hash0 as usize) = curr;
        if dtlm != ZSTD_dtlm_fast {
            /* Only load extra positions for ZSTD_dtlm_full */
            let mut p: U32 = 1;
            while p < fastHashFillStep {
                let hash: size_t =
                    ZSTD_hashPtr(ip.wrapping_add(p as usize) as *const c_void, hBits, mls);
                if *hashTable.wrapping_add(hash as usize) == 0 {
                    /* not yet filled */
                    *hashTable.wrapping_add(hash as usize) = curr + p;
                }
                p += 1;
            }
        }

        ip = ip.wrapping_add(fastHashFillStep as usize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_fillHashTable(
    ms: *mut ZSTD_MatchState_t,
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

/* ===  match4Found helpers  ============================================== */

unsafe fn ZSTD_match4Found_cmov(
    currentPtr: *const BYTE,
    matchAddress: *const BYTE,
    matchIdx: U32,
    idxLowLimit: U32,
) -> c_int {
    /* Array of ~random data, should have low probability of matching data. */
    static dummy: [BYTE; 4] = [0x12, 0x34, 0x56, 0x78];

    let mvalAddr: *const BYTE =
        ZSTD_selectAddr(matchIdx, idxLowLimit, matchAddress, dummy.as_ptr());
    if MEM_read32(currentPtr) != MEM_read32(mvalAddr) {
        return 0;
    }
    (matchIdx >= idxLowLimit) as c_int
}

unsafe fn ZSTD_match4Found_branch(
    currentPtr: *const BYTE,
    matchAddress: *const BYTE,
    matchIdx: U32,
    idxLowLimit: U32,
) -> c_int {
    let mval: U32;
    if matchIdx >= idxLowLimit {
        mval = MEM_read32(matchAddress);
    } else {
        mval = MEM_read32(currentPtr) ^ 1; /* guaranteed to not match. */
    }
    (MEM_read32(currentPtr) == mval) as c_int
}

/* ===  ZSTD_compressBlock_fast (noDict)  ================================= */

pub unsafe fn ZSTD_compressBlock_fast_noDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    mls: U32,
    useCmov: c_int,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog;
    let stepSize: size_t = (*cParams).targetLength as size_t
        + (!((*cParams).targetLength != 0) as size_t)
        + 1; /* min 2 */
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let endIndex: U32 = ((istart.offset_from(base) as size_t) + srcSize) as U32;
    let prefixStartIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog);
    let prefixStart: *const BYTE = base.wrapping_add(prefixStartIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE as usize);

    let mut anchor: *const BYTE = istart;
    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE;
    let mut ip2: *const BYTE;
    let mut ip3: *const BYTE;
    let mut current0: U32;

    let mut rep_offset1: U32 = *rep.wrapping_add(0);
    let mut rep_offset2: U32 = *rep.wrapping_add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let mut hash0: size_t; /* hash for ip0 */
    let mut hash1: size_t; /* hash for ip1 */
    let mut matchIdx: U32; /* match idx for ip0 */

    let mut offcode: U32 = 0;
    let mut match0: *const BYTE = core::ptr::null();
    let mut mLength: size_t = 0;

    let mut step: size_t;
    let mut nextStep: *const BYTE;
    let kStepIncr: size_t = 1 << (kSearchStrength - 1);
    let matchFound: unsafe fn(*const BYTE, *const BYTE, U32, U32) -> c_int = if useCmov != 0 {
        ZSTD_match4Found_cmov
    } else {
        ZSTD_match4Found_branch
    };

    ip0 = ip0.wrapping_add((ip0 == prefixStart) as usize);
    {
        let curr: U32 = ip0.offset_from(base) as U32;
        let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, curr, (*cParams).windowLog);
        let maxRep: U32 = curr - windowLow;
        if rep_offset2 > maxRep {
            offsetSaved2 = rep_offset2;
            rep_offset2 = 0;
        }
        if rep_offset1 > maxRep {
            offsetSaved1 = rep_offset1;
            rep_offset1 = 0;
        }
    }

    /* start each op */
    '_start: loop {
        step = stepSize;
        nextStep = ip0.wrapping_add(kStepIncr);

        /* calculate positions, ip0 - anchor == 0, so we skip step calc */
        ip1 = ip0.wrapping_add(1);
        ip2 = ip0.wrapping_add(step);
        ip3 = ip2.wrapping_add(1);

        if ip3 >= ilimit {
            break '_start; /* goto _cleanup */
        }

        hash0 = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);
        hash1 = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);

        matchIdx = *hashTable.wrapping_add(hash0 as usize);

        // label targets from within the do-while
        let mut goto_offset = false;
        let mut goto_match = false;

        loop {
            /* load repcode match for ip[2]*/
            let rval: U32 = MEM_read32(ip2.wrapping_sub(rep_offset1 as usize));

            /* write back hash table entry */
            current0 = ip0.offset_from(base) as U32;
            *hashTable.wrapping_add(hash0 as usize) = current0;

            /* check repcode at ip[2] */
            if ((MEM_read32(ip2) == rval) as u32 & (rep_offset1 > 0) as u32) != 0 {
                ip0 = ip2;
                match0 = ip0.wrapping_sub(rep_offset1 as usize);
                mLength = (*ip0.wrapping_offset(-1) == *match0.wrapping_offset(-1)) as size_t;
                ip0 = ip0.wrapping_sub(mLength);
                match0 = match0.wrapping_sub(mLength);
                offcode = REPCODE1_TO_OFFBASE;
                mLength += 4;

                *hashTable.wrapping_add(hash1 as usize) = ip1.offset_from(base) as U32;

                goto_match = true;
                break;
            }

            if matchFound(
                ip0,
                base.wrapping_add(matchIdx as usize),
                matchIdx,
                prefixStartIndex,
            ) != 0
            {
                *hashTable.wrapping_add(hash1 as usize) = ip1.offset_from(base) as U32;
                goto_offset = true;
                break;
            }

            /* lookup ip[1] */
            matchIdx = *hashTable.wrapping_add(hash1 as usize);

            /* hash ip[2] */
            hash0 = hash1;
            hash1 = ZSTD_hashPtr(ip2 as *const c_void, hlog, mls);

            /* advance to next positions */
            ip0 = ip1;
            ip1 = ip2;
            ip2 = ip3;

            /* write back hash table entry */
            current0 = ip0.offset_from(base) as U32;
            *hashTable.wrapping_add(hash0 as usize) = current0;

            if matchFound(
                ip0,
                base.wrapping_add(matchIdx as usize),
                matchIdx,
                prefixStartIndex,
            ) != 0
            {
                if step <= 4 {
                    *hashTable.wrapping_add(hash1 as usize) = ip1.offset_from(base) as U32;
                }
                goto_offset = true;
                break;
            }

            /* lookup ip[1] */
            matchIdx = *hashTable.wrapping_add(hash1 as usize);

            /* hash ip[2] */
            hash0 = hash1;
            hash1 = ZSTD_hashPtr(ip2 as *const c_void, hlog, mls);

            /* advance to next positions */
            ip0 = ip1;
            ip1 = ip2;
            ip2 = ip0.wrapping_add(step);
            ip3 = ip1.wrapping_add(step);

            /* calculate step */
            if ip2 >= nextStep {
                step += 1;
                nextStep = nextStep.wrapping_add(kStepIncr);
            }

            if !(ip3 < ilimit) {
                break;
            }
        } // end do-while

        if goto_offset {
            // _offset: Requires: ip0, idx
            match0 = base.wrapping_add(matchIdx as usize);
            rep_offset2 = rep_offset1;
            rep_offset1 = ip0.offset_from(match0) as U32;
            offcode = OFFSET_TO_OFFBASE(rep_offset1);
            mLength = 4;

            /* Count the backwards match length. */
            while ((ip0 > anchor) as u32 & (match0 > prefixStart) as u32) != 0
                && (*ip0.wrapping_offset(-1) == *match0.wrapping_offset(-1))
            {
                ip0 = ip0.wrapping_offset(-1);
                match0 = match0.wrapping_offset(-1);
                mLength += 1;
            }
            goto_match = true;
        }

        if goto_match {
            // _match: Requires: ip0, match0, offcode
            mLength += ZSTD_count(
                ip0.wrapping_add(mLength),
                match0.wrapping_add(mLength),
                iend,
            );

            ZSTD_storeSeq(
                seqStore,
                ip0.offset_from(anchor) as size_t,
                anchor,
                iend,
                offcode,
                mLength,
            );

            ip0 = ip0.wrapping_add(mLength);
            anchor = ip0;

            /* Fill table and check for immediate repcode. */
            if ip0 <= ilimit {
                /* Fill Table */
                *hashTable.wrapping_add(ZSTD_hashPtr(
                    base.wrapping_add((current0 + 2) as usize) as *const c_void,
                    hlog,
                    mls,
                ) as usize) = current0 + 2;
                *hashTable.wrapping_add(ZSTD_hashPtr(
                    ip0.wrapping_sub(2) as *const c_void,
                    hlog,
                    mls,
                ) as usize) = ip0.wrapping_sub(2).offset_from(base) as U32;

                if rep_offset2 > 0 {
                    while (ip0 <= ilimit)
                        && (MEM_read32(ip0)
                            == MEM_read32(ip0.wrapping_sub(rep_offset2 as usize)))
                    {
                        /* store sequence */
                        let rLength: size_t = ZSTD_count(
                            ip0.wrapping_add(4),
                            ip0.wrapping_add(4).wrapping_sub(rep_offset2 as usize),
                            iend,
                        ) + 4;
                        {
                            let tmpOff: U32 = rep_offset2;
                            rep_offset2 = rep_offset1;
                            rep_offset1 = tmpOff;
                        } /* swap rep_offset2 <=> rep_offset1 */
                        *hashTable.wrapping_add(
                            ZSTD_hashPtr(ip0 as *const c_void, hlog, mls) as usize,
                        ) = ip0.offset_from(base) as U32;
                        ip0 = ip0.wrapping_add(rLength);
                        ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, rLength);
                        anchor = ip0;
                        continue; /* faster when present */
                    }
                }
            }

            continue '_start; /* goto _start */
        }

        // if neither goto fired, the do-while exited via its condition -> _cleanup
        break '_start;
    }

    // _cleanup:
    offsetSaved2 = if (offsetSaved1 != 0) && (rep_offset1 != 0) {
        offsetSaved1
    } else {
        offsetSaved2
    };

    /* save reps for next block */
    *rep.wrapping_add(0) = if rep_offset1 != 0 {
        rep_offset1
    } else {
        offsetSaved1
    };
    *rep.wrapping_add(1) = if rep_offset2 != 0 {
        rep_offset2
    } else {
        offsetSaved2
    };

    /* Return the last literals size */
    iend.offset_from(anchor) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mml: U32 = (*ms).cParams.minMatch;
    /* use cmov when "candidate in range" branch is likely unpredictable */
    let useCmov: c_int = ((*ms).cParams.windowLog < 19) as c_int;
    if useCmov != 0 {
        match mml {
            5 => ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 5, 1),
            6 => ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 6, 1),
            7 => ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 7, 1),
            /* default (includes case 3) and case 4 */
            _ => ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 4, 1),
        }
    } else {
        /* use a branch instead */
        match mml {
            5 => ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 5, 0),
            6 => ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 6, 0),
            7 => ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 7, 0),
            _ => ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 4, 0),
        }
    }
}

/* ===  ZSTD_compressBlock_fast_dictMatchState  =========================== */

pub unsafe fn ZSTD_compressBlock_fast_dictMatchState_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    mls: U32,
    _hasStep: U32,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog;
    /* support stepSize of 0 */
    let stepSize: U32 = (*cParams).targetLength + (!((*cParams).targetLength != 0) as U32);
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE = ip0.wrapping_add(stepSize as usize);
    let mut anchor: *const BYTE = istart;
    let prefixStartIndex: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.wrapping_add(prefixStartIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE as usize);
    let mut offset_1: U32 = *rep.wrapping_add(0);
    let mut offset_2: U32 = *rep.wrapping_add(1);

    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictCParams: *const ZSTD_compressionParameters = &(*dms).cParams;
    let dictHashTable: *const U32 = (*dms).hashTable;
    let dictStartIndex: U32 = (*dms).window.dictLimit;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictStart: *const BYTE = dictBase.wrapping_add(dictStartIndex as usize);
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictIndexDelta: U32 =
        prefixStartIndex.wrapping_sub(dictEnd.offset_from(dictBase) as U32);
    let dictAndPrefixLength: U32 =
        (istart.offset_from(prefixStart) + dictEnd.offset_from(dictStart)) as U32;
    let dictHBits: U32 = (*dictCParams).hashLog + ZSTD_SHORT_CACHE_TAG_BITS;

    if (*ms).prefetchCDictTables != 0 {
        // PREFETCH_AREA is a no-op hint
    }

    /* init */
    ip0 = ip0.wrapping_add((dictAndPrefixLength == 0) as usize);

    /* Outer search loop */
    while ip1 <= ilimit {
        let mLength: size_t;
        let mut hash0: size_t = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);

        let dictHashAndTag0: size_t = ZSTD_hashPtr(ip0 as *const c_void, dictHBits, mls);
        let mut dictMatchIndexAndTag: U32 = *dictHashTable
            .wrapping_add((dictHashAndTag0 >> ZSTD_SHORT_CACHE_TAG_BITS) as usize);
        let mut dictTagsMatch: c_int =
            ZSTD_comparePackedTags(dictMatchIndexAndTag as size_t, dictHashAndTag0);

        let mut matchIndex: U32 = *hashTable.wrapping_add(hash0 as usize);
        let mut curr: U32 = ip0.offset_from(base) as U32;
        let mut step: size_t = stepSize as size_t;
        let kStepIncr: size_t = 1 << kSearchStrength;
        let mut nextStep: *const BYTE = ip0.wrapping_add(kStepIncr);

        /* Inner search loop */
        loop {
            let r#match: *const BYTE = base.wrapping_add(matchIndex as usize);
            let repIndex: U32 = curr + 1 - offset_1;
            let repMatch: *const BYTE = if repIndex < prefixStartIndex {
                dictBase.wrapping_add((repIndex.wrapping_sub(dictIndexDelta)) as usize)
            } else {
                base.wrapping_add(repIndex as usize)
            };
            let hash1: size_t = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);
            let dictHashAndTag1: size_t = ZSTD_hashPtr(ip1 as *const c_void, dictHBits, mls);
            *hashTable.wrapping_add(hash0 as usize) = curr; /* update hash table */

            if (ZSTD_index_overlap_check(prefixStartIndex, repIndex) != 0)
                && (MEM_read32(repMatch)
                    == MEM_read32(ip0.wrapping_add(1)))
            {
                let repMatchEnd: *const BYTE = if repIndex < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                let mLen: size_t = ZSTD_count_2segments(
                    ip0.wrapping_add(1).wrapping_add(4),
                    repMatch.wrapping_add(4),
                    iend,
                    repMatchEnd,
                    prefixStart,
                ) + 4;
                ip0 = ip0.wrapping_add(1);
                ZSTD_storeSeq(
                    seqStore,
                    ip0.offset_from(anchor) as size_t,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE,
                    mLen,
                );
                mLength = mLen;
                break;
            }

            if dictTagsMatch != 0 {
                /* Found a possible dict match */
                let dictMatchIndex: U32 = dictMatchIndexAndTag >> ZSTD_SHORT_CACHE_TAG_BITS;
                let dictMatch: *const BYTE = dictBase.wrapping_add(dictMatchIndex as usize);
                if dictMatchIndex > dictStartIndex
                    && MEM_read32(dictMatch) == MEM_read32(ip0)
                {
                    /* To replicate extDict parse behavior, we only use dict matches when the normal matchIndex is invalid */
                    if matchIndex <= prefixStartIndex {
                        let offset: U32 =
                            curr.wrapping_sub(dictMatchIndex).wrapping_sub(dictIndexDelta);
                        let mut mLen: size_t = ZSTD_count_2segments(
                            ip0.wrapping_add(4),
                            dictMatch.wrapping_add(4),
                            iend,
                            dictEnd,
                            prefixStart,
                        ) + 4;
                        let mut dictMatch = dictMatch;
                        while ((ip0 > anchor) as u32 & (dictMatch > dictStart) as u32) != 0
                            && (*ip0.wrapping_offset(-1) == *dictMatch.wrapping_offset(-1))
                        {
                            ip0 = ip0.wrapping_offset(-1);
                            dictMatch = dictMatch.wrapping_offset(-1);
                            mLen += 1;
                        } /* catch up */
                        offset_2 = offset_1;
                        offset_1 = offset;
                        ZSTD_storeSeq(
                            seqStore,
                            ip0.offset_from(anchor) as size_t,
                            anchor,
                            iend,
                            OFFSET_TO_OFFBASE(offset),
                            mLen,
                        );
                        mLength = mLen;
                        break;
                    }
                }
            }

            if ZSTD_match4Found_cmov(ip0, r#match, matchIndex, prefixStartIndex) != 0 {
                /* found a regular match of size >= 4 */
                let offset: U32 = ip0.offset_from(r#match) as U32;
                let mut mLen: size_t =
                    ZSTD_count(ip0.wrapping_add(4), r#match.wrapping_add(4), iend) + 4;
                let mut r#match = r#match;
                while ((ip0 > anchor) as u32 & (r#match > prefixStart) as u32) != 0
                    && (*ip0.wrapping_offset(-1) == *r#match.wrapping_offset(-1))
                {
                    ip0 = ip0.wrapping_offset(-1);
                    r#match = r#match.wrapping_offset(-1);
                    mLen += 1;
                } /* catch up */
                offset_2 = offset_1;
                offset_1 = offset;
                ZSTD_storeSeq(
                    seqStore,
                    ip0.offset_from(anchor) as size_t,
                    anchor,
                    iend,
                    OFFSET_TO_OFFBASE(offset),
                    mLen,
                );
                mLength = mLen;
                break;
            }

            /* Prepare for next iteration */
            dictMatchIndexAndTag = *dictHashTable
                .wrapping_add((dictHashAndTag1 >> ZSTD_SHORT_CACHE_TAG_BITS) as usize);
            dictTagsMatch =
                ZSTD_comparePackedTags(dictMatchIndexAndTag as size_t, dictHashAndTag1);
            matchIndex = *hashTable.wrapping_add(hash1 as usize);

            if ip1 >= nextStep {
                step += 1;
                nextStep = nextStep.wrapping_add(kStepIncr);
            }
            ip0 = ip1;
            ip1 = ip1.wrapping_add(step);
            if ip1 > ilimit {
                // goto _cleanup
                *rep.wrapping_add(0) = offset_1;
                *rep.wrapping_add(1) = offset_2;
                return iend.offset_from(anchor) as size_t;
            }

            curr = ip0.offset_from(base) as U32;
            hash0 = hash1;
        } /* end inner search loop */

        /* match found */
        ip0 = ip0.wrapping_add(mLength);
        anchor = ip0;

        if ip0 <= ilimit {
            /* Fill Table */
            *hashTable.wrapping_add(ZSTD_hashPtr(
                base.wrapping_add((curr + 2) as usize) as *const c_void,
                hlog,
                mls,
            ) as usize) = curr + 2;
            *hashTable.wrapping_add(ZSTD_hashPtr(
                ip0.wrapping_sub(2) as *const c_void,
                hlog,
                mls,
            ) as usize) = ip0.wrapping_sub(2).offset_from(base) as U32;

            /* check immediate repcode */
            while ip0 <= ilimit {
                let current2: U32 = ip0.offset_from(base) as U32;
                let repIndex2: U32 = current2 - offset_2;
                let repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictBase
                        .wrapping_sub(dictIndexDelta as usize)
                        .wrapping_add(repIndex2 as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if (ZSTD_index_overlap_check(prefixStartIndex, repIndex2) != 0)
                    && (MEM_read32(repMatch2)
                        == MEM_read32(ip0))
                {
                    let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    let repLength2: size_t = ZSTD_count_2segments(
                        ip0.wrapping_add(4),
                        repMatch2.wrapping_add(4),
                        iend,
                        repEnd2,
                        prefixStart,
                    ) + 4;
                    let tmpOffset: U32 = offset_2;
                    offset_2 = offset_1;
                    offset_1 = tmpOffset; /* swap offset_2 <=> offset_1 */
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, repLength2);
                    *hashTable.wrapping_add(ZSTD_hashPtr(ip0 as *const c_void, hlog, mls) as usize) =
                        current2;
                    ip0 = ip0.wrapping_add(repLength2);
                    anchor = ip0;
                    continue;
                }
                break;
            }
        }

        /* Prepare for next iteration */
        ip1 = ip0.wrapping_add(stepSize as usize);
    }

    // _cleanup:
    *rep.wrapping_add(0) = offset_1;
    *rep.wrapping_add(1) = offset_2;

    iend.offset_from(anchor) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_fast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 5, 0),
        6 => ZSTD_compressBlock_fast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 6, 0),
        7 => ZSTD_compressBlock_fast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 7, 0),
        _ => ZSTD_compressBlock_fast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 4, 0),
    }
}

/* ===  ZSTD_compressBlock_fast_extDict  ================================== */

pub unsafe fn ZSTD_compressBlock_fast_extDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    mls: U32,
    _hasStep: U32,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog;
    /* support stepSize of 0 */
    let stepSize: size_t = (*cParams).targetLength as size_t
        + (!((*cParams).targetLength != 0) as size_t)
        + 1;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let istart: *const BYTE = src as *const BYTE;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 = ((istart.offset_from(base) as size_t) + srcSize) as U32;
    let lowLimit: U32 = ZSTD_getLowestMatchIndex(ms, endIndex, (*cParams).windowLog);
    let dictStartIndex: U32 = lowLimit;
    let dictStart: *const BYTE = dictBase.wrapping_add(dictStartIndex as usize);
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStartIndex: U32 = if dictLimit < lowLimit { lowLimit } else { dictLimit };
    let prefixStart: *const BYTE = base.wrapping_add(prefixStartIndex as usize);
    let dictEnd: *const BYTE = dictBase.wrapping_add(prefixStartIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(8);
    let mut offset_1: U32 = *rep.wrapping_add(0);
    let mut offset_2: U32 = *rep.wrapping_add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE;
    let mut ip2: *const BYTE;
    let mut ip3: *const BYTE;
    let mut current0: U32;

    let mut hash0: size_t; /* hash for ip0 */
    let mut hash1: size_t; /* hash for ip1 */
    let mut idx: U32; /* match idx for ip0 */
    let mut idxBase: *const BYTE; /* base pointer for idx */

    let mut offcode: U32 = 0;
    let mut match0: *const BYTE = core::ptr::null();
    let mut mLength: size_t = 0;
    let mut matchEnd: *const BYTE = core::ptr::null(); /* initialize to avoid warning */

    let mut step: size_t;
    let mut nextStep: *const BYTE;
    let kStepIncr: size_t = 1 << (kSearchStrength - 1);

    /* switch to "regular" variant if extDict is invalidated due to maxDistance */
    if prefixStartIndex == dictStartIndex {
        return ZSTD_compressBlock_fast(ms, seqStore, rep, src, srcSize);
    }

    {
        let curr: U32 = ip0.offset_from(base) as U32;
        let maxRep: U32 = curr - dictStartIndex;
        if offset_2 >= maxRep {
            offsetSaved2 = offset_2;
            offset_2 = 0;
        }
        if offset_1 >= maxRep {
            offsetSaved1 = offset_1;
            offset_1 = 0;
        }
    }

    '_start: loop {
        step = stepSize;
        nextStep = ip0.wrapping_add(kStepIncr);

        ip1 = ip0.wrapping_add(1);
        ip2 = ip0.wrapping_add(step);
        ip3 = ip2.wrapping_add(1);

        if ip3 >= ilimit {
            break '_start; /* goto _cleanup */
        }

        hash0 = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);
        hash1 = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);

        idx = *hashTable.wrapping_add(hash0 as usize);
        idxBase = if idx < prefixStartIndex { dictBase } else { base };

        let mut goto_offset = false;
        let mut goto_match = false;

        loop {
            {
                /* load repcode match for ip[2] */
                let current2: U32 = ip2.offset_from(base) as U32;
                let repIndex: U32 = current2 - offset_1;
                let repBase: *const BYTE = if repIndex < prefixStartIndex {
                    dictBase
                } else {
                    base
                };
                let rval: U32;
                if ((prefixStartIndex.wrapping_sub(repIndex) >= 4) as u32
                    & (offset_1 > 0) as u32)
                    != 0
                {
                    rval = MEM_read32(repBase.wrapping_add(repIndex as usize));
                } else {
                    rval = MEM_read32(ip2) ^ 1; /* guaranteed to not match. */
                }

                /* write back hash table entry */
                current0 = ip0.offset_from(base) as U32;
                *hashTable.wrapping_add(hash0 as usize) = current0;

                /* check repcode at ip[2] */
                if MEM_read32(ip2) == rval {
                    ip0 = ip2;
                    match0 = repBase.wrapping_add(repIndex as usize);
                    matchEnd = if repIndex < prefixStartIndex { dictEnd } else { iend };
                    mLength = (*ip0.wrapping_offset(-1) == *match0.wrapping_offset(-1)) as size_t;
                    ip0 = ip0.wrapping_sub(mLength);
                    match0 = match0.wrapping_sub(mLength);
                    offcode = REPCODE1_TO_OFFBASE;
                    mLength += 4;
                    goto_match = true;
                    break;
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
                    goto_offset = true;
                    break;
                }
            }

            /* lookup ip[1] */
            idx = *hashTable.wrapping_add(hash1 as usize);
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
            *hashTable.wrapping_add(hash0 as usize) = current0;

            {
                /* load match for ip[0] */
                let mval: U32 = if idx >= dictStartIndex {
                    MEM_read32(idxBase.wrapping_add(idx as usize))
                } else {
                    MEM_read32(ip0) ^ 1 /* guaranteed not to match */
                };

                if MEM_read32(ip0) == mval {
                    goto_offset = true;
                    break;
                }
            }

            /* lookup ip[1] */
            idx = *hashTable.wrapping_add(hash1 as usize);
            idxBase = if idx < prefixStartIndex { dictBase } else { base };

            /* hash ip[2] */
            hash0 = hash1;
            hash1 = ZSTD_hashPtr(ip2 as *const c_void, hlog, mls);

            /* advance to next positions */
            ip0 = ip1;
            ip1 = ip2;
            ip2 = ip0.wrapping_add(step);
            ip3 = ip1.wrapping_add(step);

            /* calculate step */
            if ip2 >= nextStep {
                step += 1;
                nextStep = nextStep.wrapping_add(kStepIncr);
            }

            if !(ip3 < ilimit) {
                break;
            }
        } // end do-while

        if goto_offset {
            // _offset: Requires: ip0, idx, idxBase
            {
                let offset: U32 = current0 - idx;
                let lowMatchPtr: *const BYTE = if idx < prefixStartIndex {
                    dictStart
                } else {
                    prefixStart
                };
                matchEnd = if idx < prefixStartIndex { dictEnd } else { iend };
                match0 = idxBase.wrapping_add(idx as usize);
                offset_2 = offset_1;
                offset_1 = offset;
                offcode = OFFSET_TO_OFFBASE(offset);
                mLength = 4;

                /* Count the backwards match length. */
                while ((ip0 > anchor) as u32 & (match0 > lowMatchPtr) as u32) != 0
                    && (*ip0.wrapping_offset(-1) == *match0.wrapping_offset(-1))
                {
                    ip0 = ip0.wrapping_offset(-1);
                    match0 = match0.wrapping_offset(-1);
                    mLength += 1;
                }
            }
            goto_match = true;
        }

        if goto_match {
            // _match: Requires: ip0, match0, offcode, matchEnd
            mLength += ZSTD_count_2segments(
                ip0.wrapping_add(mLength),
                match0.wrapping_add(mLength),
                iend,
                matchEnd,
                prefixStart,
            );

            ZSTD_storeSeq(
                seqStore,
                ip0.offset_from(anchor) as size_t,
                anchor,
                iend,
                offcode,
                mLength,
            );

            ip0 = ip0.wrapping_add(mLength);
            anchor = ip0;

            /* write next hash table entry */
            if ip1 < ip0 {
                *hashTable.wrapping_add(hash1 as usize) = ip1.offset_from(base) as U32;
            }

            /* Fill table and check for immediate repcode. */
            if ip0 <= ilimit {
                /* Fill Table */
                *hashTable.wrapping_add(ZSTD_hashPtr(
                    base.wrapping_add((current0 + 2) as usize) as *const c_void,
                    hlog,
                    mls,
                ) as usize) = current0 + 2;
                *hashTable.wrapping_add(ZSTD_hashPtr(
                    ip0.wrapping_sub(2) as *const c_void,
                    hlog,
                    mls,
                ) as usize) = ip0.wrapping_sub(2).offset_from(base) as U32;

                while ip0 <= ilimit {
                    let repIndex2: U32 = (ip0.offset_from(base) as U32) - offset_2;
                    let repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                        dictBase.wrapping_add(repIndex2 as usize)
                    } else {
                        base.wrapping_add(repIndex2 as usize)
                    };
                    if ((ZSTD_index_overlap_check(prefixStartIndex, repIndex2) as u32)
                        & (offset_2 > 0) as u32)
                        != 0
                        && (MEM_read32(repMatch2)
                            == MEM_read32(ip0))
                    {
                        let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                            dictEnd
                        } else {
                            iend
                        };
                        let repLength2: size_t = ZSTD_count_2segments(
                            ip0.wrapping_add(4),
                            repMatch2.wrapping_add(4),
                            iend,
                            repEnd2,
                            prefixStart,
                        ) + 4;
                        {
                            let tmpOffset: U32 = offset_2;
                            offset_2 = offset_1;
                            offset_1 = tmpOffset;
                        } /* swap offset_2 <=> offset_1 */
                        ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, repLength2);
                        *hashTable
                            .wrapping_add(ZSTD_hashPtr(ip0 as *const c_void, hlog, mls) as usize) =
                            ip0.offset_from(base) as U32;
                        ip0 = ip0.wrapping_add(repLength2);
                        anchor = ip0;
                        continue;
                    }
                    break;
                }
            }

            continue '_start; /* goto _start */
        }

        // do-while exited via condition -> _cleanup
        break '_start;
    }

    // _cleanup:
    offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
        offsetSaved1
    } else {
        offsetSaved2
    };

    /* save reps for next block */
    *rep.wrapping_add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
    *rep.wrapping_add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };

    iend.offset_from(anchor) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, 5, 0),
        6 => ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, 6, 0),
        7 => ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, 7, 0),
        _ => ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, 4, 0),
    }
}
