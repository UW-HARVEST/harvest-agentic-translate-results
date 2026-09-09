//! Translation of `compress/zstd_double_fast.c` (DOUBLE-FAST match finder).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use crate::common::mem::*;
use crate::common::zstd_h::ZSTD_compressionParameters;
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_fast::ZSTD_compressBlock_fast;

use core::ffi::{c_int, c_void};

/* ===  ZSTD_fillDoubleHashTable  ======================================== */

pub unsafe fn ZSTD_fillDoubleHashTableForCDict(
    ms: *mut ZSTD_MatchState_t,
    end: *const c_void,
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
    let iend: *const BYTE = (end as *const BYTE).wrapping_sub(HASH_READ_SIZE as usize);
    let fastHashFillStep: U32 = 3;

    while ip.wrapping_add((fastHashFillStep - 1) as usize) <= iend {
        let curr: U32 = ip.offset_from(base) as U32;
        let mut i: U32 = 0;
        while i < fastHashFillStep {
            let smHashAndTag: size_t =
                ZSTD_hashPtr(ip.wrapping_add(i as usize) as *const c_void, hBitsS, mls);
            let lgHashAndTag: size_t =
                ZSTD_hashPtr(ip.wrapping_add(i as usize) as *const c_void, hBitsL, 8);
            if i == 0 {
                ZSTD_writeTaggedIndex(hashSmall, smHashAndTag, curr + i);
            }
            if i == 0
                || *hashLarge.wrapping_add((lgHashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS) as usize)
                    == 0
            {
                ZSTD_writeTaggedIndex(hashLarge, lgHashAndTag, curr + i);
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
    end: *const c_void,
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
    let iend: *const BYTE = (end as *const BYTE).wrapping_sub(HASH_READ_SIZE as usize);
    let fastHashFillStep: U32 = 3;

    while ip.wrapping_add((fastHashFillStep - 1) as usize) <= iend {
        let curr: U32 = ip.offset_from(base) as U32;
        let mut i: U32 = 0;
        while i < fastHashFillStep {
            let smHash: size_t =
                ZSTD_hashPtr(ip.wrapping_add(i as usize) as *const c_void, hBitsS, mls);
            let lgHash: size_t =
                ZSTD_hashPtr(ip.wrapping_add(i as usize) as *const c_void, hBitsL, 8);
            if i == 0 {
                *hashSmall.wrapping_add(smHash as usize) = curr + i;
            }
            if i == 0 || *hashLarge.wrapping_add(lgHash as usize) == 0 {
                *hashLarge.wrapping_add(lgHash as usize) = curr + i;
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
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
) {
    if tfp == ZSTD_tfp_forCDict {
        ZSTD_fillDoubleHashTableForCDict(ms, end, dtlm);
    } else {
        ZSTD_fillDoubleHashTableForCCtx(ms, end, dtlm);
    }
}

/* ===  ZSTD_compressBlock_doubleFast (noDict)  ========================== */

pub unsafe fn ZSTD_compressBlock_doubleFast_noDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    mls: U32,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashLong: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog;
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 = ((istart.offset_from(base) as size_t) + srcSize) as U32;
    let prefixLowestIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog);
    let prefixLowest: *const BYTE = base.wrapping_add(prefixLowestIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE as usize);
    let mut offset_1: U32 = *rep.wrapping_add(0);
    let mut offset_2: U32 = *rep.wrapping_add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let mut mLength: size_t = 0;
    let mut offset: U32 = 0;
    let mut curr: U32 = 0;

    let kStepIncr: size_t = 1 << kSearchStrength;
    let mut nextStep: *const BYTE;
    let mut step: size_t;

    let mut hl0: size_t; /* the long hash at ip */
    let mut hl1: size_t = 0; /* the long hash at ip1 */

    let mut idxl0: U32; /* the long match index for ip */
    let mut idxl1: U32 = 0; /* the long match index for ip1 */

    let mut matchl0: *const BYTE; /* the long match for ip */
    let mut matchs0: *const BYTE; /* the short match for ip */
    let mut matchl1: *const BYTE = core::ptr::null(); /* the long match for ip1 */
    let mut matchs0_safe: *const BYTE = core::ptr::null();

    let mut ip: *const BYTE = istart;
    let mut ip1: *const BYTE;
    let dummy: [BYTE; 10] = [
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xe2, 0xb4,
    ];

    /* init */
    ip = ip.wrapping_add(((ip.offset_from(prefixLowest)) == 0) as usize);
    {
        let current: U32 = ip.offset_from(base) as U32;
        let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, current, (*cParams).windowLog);
        let maxRep: U32 = current - windowLow;
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
        step = 1;
        nextStep = ip.wrapping_add(kStepIncr);
        ip1 = ip.wrapping_add(step);

        if ip1 > ilimit {
            // goto _cleanup
            offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
                offsetSaved1
            } else {
                offsetSaved2
            };
            *rep.wrapping_add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
            *rep.wrapping_add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };
            return iend.offset_from(anchor) as size_t;
        }

        hl0 = ZSTD_hashPtr(ip as *const c_void, hBitsL, 8);
        idxl0 = *hashLong.wrapping_add(hl0 as usize);
        matchl0 = base.wrapping_add(idxl0 as usize);

        // gotos out of the inner loop
        let mut goto_cleanup = false;
        let mut goto_search_next_long = false;
        let mut goto_match_found = false;
        let mut goto_match_stored = false;

        /* Inner Loop: one iteration per search / position */
        loop {
            let hs0: size_t = ZSTD_hashPtr(ip as *const c_void, hBitsS, mls);
            let idxs0: U32 = *hashSmall.wrapping_add(hs0 as usize);
            curr = ip.offset_from(base) as U32;
            matchs0 = base.wrapping_add(idxs0 as usize);

            *hashLong.wrapping_add(hl0 as usize) = curr;
            *hashSmall.wrapping_add(hs0 as usize) = curr; /* update hash tables */

            /* check noDict repcode */
            if ((offset_1 > 0) as u32
                & (MEM_read32(ip.wrapping_add(1).wrapping_sub(offset_1 as usize))
                    == MEM_read32(ip.wrapping_add(1))) as u32)
                != 0
            {
                mLength = ZSTD_count(
                    ip.wrapping_add(1).wrapping_add(4),
                    ip.wrapping_add(1).wrapping_add(4).wrapping_sub(offset_1 as usize),
                    iend,
                ) + 4;
                ip = ip.wrapping_add(1);
                ZSTD_storeSeq(
                    seqStore,
                    ip.offset_from(anchor) as size_t,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE,
                    mLength,
                );
                goto_match_stored = true;
                break;
            }

            hl1 = ZSTD_hashPtr(ip1 as *const c_void, hBitsL, 8);

            {
                let matchl0_safe: *const BYTE =
                    ZSTD_selectAddr(idxl0, prefixLowestIndex, matchl0, dummy.as_ptr());

                /* check prefix long match */
                if MEM_read64(matchl0_safe) == MEM_read64(ip)
                    && matchl0_safe == matchl0
                {
                    mLength =
                        ZSTD_count(ip.wrapping_add(8), matchl0.wrapping_add(8), iend) + 8;
                    offset = ip.offset_from(matchl0) as U32;
                    while ((ip > anchor) as u32 & (matchl0 > prefixLowest) as u32) != 0
                        && (*ip.wrapping_offset(-1) == *matchl0.wrapping_offset(-1))
                    {
                        ip = ip.wrapping_offset(-1);
                        matchl0 = matchl0.wrapping_offset(-1);
                        mLength += 1;
                    } /* catch up */
                    goto_match_found = true;
                    break;
                }
            }

            idxl1 = *hashLong.wrapping_add(hl1 as usize);
            matchl1 = base.wrapping_add(idxl1 as usize);

            /* Same optimization as matchl0 above */
            matchs0_safe = ZSTD_selectAddr(idxs0, prefixLowestIndex, matchs0, dummy.as_ptr());

            /* check prefix short match */
            if MEM_read32(matchs0_safe) == MEM_read32(ip)
                && matchs0_safe == matchs0
            {
                goto_search_next_long = true;
                break;
            }

            if ip1 >= nextStep {
                step += 1;
                nextStep = nextStep.wrapping_add(kStepIncr);
            }
            ip = ip1;
            ip1 = ip1.wrapping_add(step);

            hl0 = hl1;
            idxl0 = idxl1;
            matchl0 = matchl1;

            if !(ip1 <= ilimit) {
                goto_cleanup = true;
                break;
            }
        } // end inner loop

        if goto_cleanup {
            // _cleanup:
            offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
                offsetSaved1
            } else {
                offsetSaved2
            };
            *rep.wrapping_add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
            *rep.wrapping_add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };
            return iend.offset_from(anchor) as size_t;
        }

        if goto_search_next_long {
            // _search_next_long:
            mLength = ZSTD_count(ip.wrapping_add(4), matchs0.wrapping_add(4), iend) + 4;
            offset = ip.offset_from(matchs0) as U32;

            /* check long match at +1 position */
            if (idxl1 > prefixLowestIndex)
                && (MEM_read64(matchl1)
                    == MEM_read64(ip1))
            {
                let l1len: size_t =
                    ZSTD_count(ip1.wrapping_add(8), matchl1.wrapping_add(8), iend) + 8;
                if l1len > mLength {
                    /* use the long match instead */
                    ip = ip1;
                    mLength = l1len;
                    offset = ip.offset_from(matchl1) as U32;
                    matchs0 = matchl1;
                }
            }

            while ((ip > anchor) as u32 & (matchs0 > prefixLowest) as u32) != 0
                && (*ip.wrapping_offset(-1) == *matchs0.wrapping_offset(-1))
            {
                ip = ip.wrapping_offset(-1);
                matchs0 = matchs0.wrapping_offset(-1);
                mLength += 1;
            } /* complete backward */

            /* fall-through to _match_found */
            goto_match_found = true;
        }

        if goto_match_found {
            // _match_found: requires ip, offset, mLength
            offset_2 = offset_1;
            offset_1 = offset;

            if step < 4 {
                *hashLong.wrapping_add(hl1 as usize) = ip1.offset_from(base) as U32;
            }

            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as size_t,
                anchor,
                iend,
                OFFSET_TO_OFFBASE(offset),
                mLength,
            );
            goto_match_stored = true;
        }

        if goto_match_stored {
            // _match_stored:
            ip = ip.wrapping_add(mLength);
            anchor = ip;

            if ip <= ilimit {
                /* Complementary insertion */
                {
                    let indexToInsert: U32 = curr + 2;
                    *hashLong.wrapping_add(ZSTD_hashPtr(
                        base.wrapping_add(indexToInsert as usize) as *const c_void,
                        hBitsL,
                        8,
                    ) as usize) = indexToInsert;
                    *hashLong.wrapping_add(ZSTD_hashPtr(
                        ip.wrapping_sub(2) as *const c_void,
                        hBitsL,
                        8,
                    ) as usize) = ip.wrapping_sub(2).offset_from(base) as U32;
                    *hashSmall.wrapping_add(ZSTD_hashPtr(
                        base.wrapping_add(indexToInsert as usize) as *const c_void,
                        hBitsS,
                        mls,
                    ) as usize) = indexToInsert;
                    *hashSmall.wrapping_add(ZSTD_hashPtr(
                        ip.wrapping_sub(1) as *const c_void,
                        hBitsS,
                        mls,
                    ) as usize) = ip.wrapping_sub(1).offset_from(base) as U32;
                }

                /* check immediate repcode */
                while (ip <= ilimit)
                    && (((offset_2 > 0) as u32
                        & (MEM_read32(ip)
                            == MEM_read32(ip.wrapping_sub(offset_2 as usize)))
                            as u32)
                        != 0)
                {
                    /* store sequence */
                    let rLength: size_t = ZSTD_count(
                        ip.wrapping_add(4),
                        ip.wrapping_add(4).wrapping_sub(offset_2 as usize),
                        iend,
                    ) + 4;
                    let tmpOff: U32 = offset_2;
                    offset_2 = offset_1;
                    offset_1 = tmpOff; /* swap offset_2 <=> offset_1 */
                    *hashSmall.wrapping_add(ZSTD_hashPtr(ip as *const c_void, hBitsS, mls) as usize) =
                        ip.offset_from(base) as U32;
                    *hashLong.wrapping_add(ZSTD_hashPtr(ip as *const c_void, hBitsL, 8) as usize) =
                        ip.offset_from(base) as U32;
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, rLength);
                    ip = ip.wrapping_add(rLength);
                    anchor = ip;
                    continue; /* faster when present */
                }
            }
        }

        // loop back to outer 'while(1)'
    }
}

/* ===  ZSTD_compressBlock_doubleFast_dictMatchState  ==================== */

pub unsafe fn ZSTD_compressBlock_doubleFast_dictMatchState_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    mls: U32,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashLong: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog;
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 = ((istart.offset_from(base) as size_t) + srcSize) as U32;
    let prefixLowestIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog);
    let prefixLowest: *const BYTE = base.wrapping_add(prefixLowestIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE as usize);
    let mut offset_1: U32 = *rep.wrapping_add(0);
    let mut offset_2: U32 = *rep.wrapping_add(1);

    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictCParams: *const ZSTD_compressionParameters = &(*dms).cParams;
    let dictHashLong: *const U32 = (*dms).hashTable;
    let dictHashSmall: *const U32 = (*dms).chainTable;
    let dictStartIndex: U32 = (*dms).window.dictLimit;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictStart: *const BYTE = dictBase.wrapping_add(dictStartIndex as usize);
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictIndexDelta: U32 =
        prefixLowestIndex.wrapping_sub(dictEnd.offset_from(dictBase) as U32);
    let dictHBitsL: U32 = (*dictCParams).hashLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let dictHBitsS: U32 = (*dictCParams).chainLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let dictAndPrefixLength: U32 =
        (ip.offset_from(prefixLowest) + dictEnd.offset_from(dictStart)) as U32;

    if (*ms).prefetchCDictTables != 0 {
        // PREFETCH_AREA no-op
    }

    /* init */
    ip = ip.wrapping_add((dictAndPrefixLength == 0) as usize);

    /* Main Search Loop */
    while ip < ilimit {
        let mut mLength: size_t = 0;
        let mut offset: U32 = 0;
        let h2: size_t = ZSTD_hashPtr(ip as *const c_void, hBitsL, 8);
        let h: size_t = ZSTD_hashPtr(ip as *const c_void, hBitsS, mls);
        let dictHashAndTagL: size_t = ZSTD_hashPtr(ip as *const c_void, dictHBitsL, 8);
        let dictHashAndTagS: size_t = ZSTD_hashPtr(ip as *const c_void, dictHBitsS, mls);
        let dictMatchIndexAndTagL: U32 = *dictHashLong
            .wrapping_add((dictHashAndTagL >> ZSTD_SHORT_CACHE_TAG_BITS) as usize);
        let dictMatchIndexAndTagS: U32 = *dictHashSmall
            .wrapping_add((dictHashAndTagS >> ZSTD_SHORT_CACHE_TAG_BITS) as usize);
        let dictTagsMatchL: c_int =
            ZSTD_comparePackedTags(dictMatchIndexAndTagL as size_t, dictHashAndTagL);
        let dictTagsMatchS: c_int =
            ZSTD_comparePackedTags(dictMatchIndexAndTagS as size_t, dictHashAndTagS);
        let curr: U32 = ip.offset_from(base) as U32;
        let matchIndexL: U32 = *hashLong.wrapping_add(h2 as usize);
        let mut matchIndexS: U32 = *hashSmall.wrapping_add(h as usize);
        let mut matchLong: *const BYTE = base.wrapping_add(matchIndexL as usize);
        let mut r#match: *const BYTE = base.wrapping_add(matchIndexS as usize);
        let repIndex: U32 = curr + 1 - offset_1;
        let repMatch: *const BYTE = if repIndex < prefixLowestIndex {
            dictBase.wrapping_add((repIndex.wrapping_sub(dictIndexDelta)) as usize)
        } else {
            base.wrapping_add(repIndex as usize)
        };
        *hashLong.wrapping_add(h2 as usize) = curr;
        *hashSmall.wrapping_add(h as usize) = curr; /* update hash tables */

        let mut goto_match_found = false;
        let mut goto_match_stored = false;
        let mut goto_search_next_long = false;

        /* check repcode */
        if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
            && (MEM_read32(repMatch)
                == MEM_read32(ip.wrapping_add(1)))
        {
            let repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                dictEnd
            } else {
                iend
            };
            mLength = ZSTD_count_2segments(
                ip.wrapping_add(1).wrapping_add(4),
                repMatch.wrapping_add(4),
                iend,
                repMatchEnd,
                prefixLowest,
            ) + 4;
            ip = ip.wrapping_add(1);
            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as size_t,
                anchor,
                iend,
                REPCODE1_TO_OFFBASE,
                mLength,
            );
            goto_match_stored = true;
        }

        if !goto_match_stored {
            if (matchIndexL >= prefixLowestIndex)
                && (MEM_read64(matchLong) == MEM_read64(ip))
            {
                /* check prefix long match */
                mLength = ZSTD_count(ip.wrapping_add(8), matchLong.wrapping_add(8), iend) + 8;
                offset = ip.offset_from(matchLong) as U32;
                while ((ip > anchor) as u32 & (matchLong > prefixLowest) as u32) != 0
                    && (*ip.wrapping_offset(-1) == *matchLong.wrapping_offset(-1))
                {
                    ip = ip.wrapping_offset(-1);
                    matchLong = matchLong.wrapping_offset(-1);
                    mLength += 1;
                } /* catch up */
                goto_match_found = true;
            } else if dictTagsMatchL != 0 {
                /* check dictMatchState long match */
                let dictMatchIndexL: U32 = dictMatchIndexAndTagL >> ZSTD_SHORT_CACHE_TAG_BITS;
                let mut dictMatchL: *const BYTE = dictBase.wrapping_add(dictMatchIndexL as usize);

                if dictMatchL > dictStart
                    && MEM_read64(dictMatchL) == MEM_read64(ip)
                {
                    mLength = ZSTD_count_2segments(
                        ip.wrapping_add(8),
                        dictMatchL.wrapping_add(8),
                        iend,
                        dictEnd,
                        prefixLowest,
                    ) + 8;
                    offset =
                        curr.wrapping_sub(dictMatchIndexL).wrapping_sub(dictIndexDelta);
                    while ((ip > anchor) as u32 & (dictMatchL > dictStart) as u32) != 0
                        && (*ip.wrapping_offset(-1) == *dictMatchL.wrapping_offset(-1))
                    {
                        ip = ip.wrapping_offset(-1);
                        dictMatchL = dictMatchL.wrapping_offset(-1);
                        mLength += 1;
                    } /* catch up */
                    goto_match_found = true;
                }
            }

            if !goto_match_found {
                let mut do_search_next_long = false;
                if matchIndexS > prefixLowestIndex {
                    /* short match candidate */
                    if MEM_read32(r#match) == MEM_read32(ip) {
                        do_search_next_long = true;
                    }
                } else if dictTagsMatchS != 0 {
                    /* check dictMatchState short match */
                    let dictMatchIndexS: U32 = dictMatchIndexAndTagS >> ZSTD_SHORT_CACHE_TAG_BITS;
                    r#match = dictBase.wrapping_add(dictMatchIndexS as usize);
                    matchIndexS = dictMatchIndexS.wrapping_add(dictIndexDelta);

                    if r#match > dictStart
                        && MEM_read32(r#match) == MEM_read32(ip)
                    {
                        do_search_next_long = true;
                    }
                }

                if do_search_next_long {
                    goto_search_next_long = true;
                } else {
                    ip = ip.wrapping_add((((ip.offset_from(anchor)) as size_t >> kSearchStrength)
                        + 1) as usize);
                    continue;
                }
            }
        }

        if goto_search_next_long {
            // _search_next_long:
            {
                let hl3: size_t = ZSTD_hashPtr(ip.wrapping_add(1) as *const c_void, hBitsL, 8);
                let dictHashAndTagL3: size_t =
                    ZSTD_hashPtr(ip.wrapping_add(1) as *const c_void, dictHBitsL, 8);
                let matchIndexL3: U32 = *hashLong.wrapping_add(hl3 as usize);
                let dictMatchIndexAndTagL3: U32 = *dictHashLong
                    .wrapping_add((dictHashAndTagL3 >> ZSTD_SHORT_CACHE_TAG_BITS) as usize);
                let dictTagsMatchL3: c_int =
                    ZSTD_comparePackedTags(dictMatchIndexAndTagL3 as size_t, dictHashAndTagL3);
                let mut matchL3: *const BYTE = base.wrapping_add(matchIndexL3 as usize);
                *hashLong.wrapping_add(hl3 as usize) = curr + 1;

                /* check prefix long +1 match */
                if (matchIndexL3 >= prefixLowestIndex)
                    && (MEM_read64(matchL3)
                        == MEM_read64(ip.wrapping_add(1)))
                {
                    mLength =
                        ZSTD_count(ip.wrapping_add(9), matchL3.wrapping_add(8), iend) + 8;
                    ip = ip.wrapping_add(1);
                    offset = ip.offset_from(matchL3) as U32;
                    while ((ip > anchor) as u32 & (matchL3 > prefixLowest) as u32) != 0
                        && (*ip.wrapping_offset(-1) == *matchL3.wrapping_offset(-1))
                    {
                        ip = ip.wrapping_offset(-1);
                        matchL3 = matchL3.wrapping_offset(-1);
                        mLength += 1;
                    } /* catch up */
                    goto_match_found = true;
                } else if dictTagsMatchL3 != 0 {
                    /* check dict long +1 match */
                    let dictMatchIndexL3: U32 =
                        dictMatchIndexAndTagL3 >> ZSTD_SHORT_CACHE_TAG_BITS;
                    let mut dictMatchL3: *const BYTE =
                        dictBase.wrapping_add(dictMatchIndexL3 as usize);
                    if dictMatchL3 > dictStart
                        && MEM_read64(dictMatchL3)
                            == MEM_read64(ip.wrapping_add(1))
                    {
                        mLength = ZSTD_count_2segments(
                            ip.wrapping_add(1).wrapping_add(8),
                            dictMatchL3.wrapping_add(8),
                            iend,
                            dictEnd,
                            prefixLowest,
                        ) + 8;
                        ip = ip.wrapping_add(1);
                        offset = (curr + 1)
                            .wrapping_sub(dictMatchIndexL3)
                            .wrapping_sub(dictIndexDelta);
                        while ((ip > anchor) as u32 & (dictMatchL3 > dictStart) as u32) != 0
                            && (*ip.wrapping_offset(-1) == *dictMatchL3.wrapping_offset(-1))
                        {
                            ip = ip.wrapping_offset(-1);
                            dictMatchL3 = dictMatchL3.wrapping_offset(-1);
                            mLength += 1;
                        } /* catch up */
                        goto_match_found = true;
                    }
                }
            }

            if !goto_match_found {
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
                    while ((ip > anchor) as u32 & (r#match > dictStart) as u32) != 0
                        && (*ip.wrapping_offset(-1) == *r#match.wrapping_offset(-1))
                    {
                        ip = ip.wrapping_offset(-1);
                        r#match = r#match.wrapping_offset(-1);
                        mLength += 1;
                    } /* catch up */
                } else {
                    mLength = ZSTD_count(ip.wrapping_add(4), r#match.wrapping_add(4), iend) + 4;
                    offset = ip.offset_from(r#match) as U32;
                    while ((ip > anchor) as u32 & (r#match > prefixLowest) as u32) != 0
                        && (*ip.wrapping_offset(-1) == *r#match.wrapping_offset(-1))
                    {
                        ip = ip.wrapping_offset(-1);
                        r#match = r#match.wrapping_offset(-1);
                        mLength += 1;
                    } /* catch up */
                }
                goto_match_found = true;
            }
        }

        if goto_match_found {
            // _match_found:
            offset_2 = offset_1;
            offset_1 = offset;

            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as size_t,
                anchor,
                iend,
                OFFSET_TO_OFFBASE(offset),
                mLength,
            );
            goto_match_stored = true;
        }

        if goto_match_stored {
            // _match_stored:
            ip = ip.wrapping_add(mLength);
            anchor = ip;

            if ip <= ilimit {
                /* Complementary insertion */
                {
                    let indexToInsert: U32 = curr + 2;
                    *hashLong.wrapping_add(ZSTD_hashPtr(
                        base.wrapping_add(indexToInsert as usize) as *const c_void,
                        hBitsL,
                        8,
                    ) as usize) = indexToInsert;
                    *hashLong.wrapping_add(ZSTD_hashPtr(
                        ip.wrapping_sub(2) as *const c_void,
                        hBitsL,
                        8,
                    ) as usize) = ip.wrapping_sub(2).offset_from(base) as U32;
                    *hashSmall.wrapping_add(ZSTD_hashPtr(
                        base.wrapping_add(indexToInsert as usize) as *const c_void,
                        hBitsS,
                        mls,
                    ) as usize) = indexToInsert;
                    *hashSmall.wrapping_add(ZSTD_hashPtr(
                        ip.wrapping_sub(1) as *const c_void,
                        hBitsS,
                        mls,
                    ) as usize) = ip.wrapping_sub(1).offset_from(base) as U32;
                }

                /* check immediate repcode */
                while ip <= ilimit {
                    let current2: U32 = ip.offset_from(base) as U32;
                    let repIndex2: U32 = current2 - offset_2;
                    let repMatch2: *const BYTE = if repIndex2 < prefixLowestIndex {
                        dictBase
                            .wrapping_add(repIndex2 as usize)
                            .wrapping_sub(dictIndexDelta as usize)
                    } else {
                        base.wrapping_add(repIndex2 as usize)
                    };
                    if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex2) != 0)
                        && (MEM_read32(repMatch2)
                            == MEM_read32(ip))
                    {
                        let repEnd2: *const BYTE = if repIndex2 < prefixLowestIndex {
                            dictEnd
                        } else {
                            iend
                        };
                        let repLength2: size_t = ZSTD_count_2segments(
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
                        *hashSmall
                            .wrapping_add(ZSTD_hashPtr(ip as *const c_void, hBitsS, mls) as usize) =
                            current2;
                        *hashLong
                            .wrapping_add(ZSTD_hashPtr(ip as *const c_void, hBitsL, 8) as usize) =
                            current2;
                        ip = ip.wrapping_add(repLength2);
                        anchor = ip;
                        continue;
                    }
                    break;
                }
            }
        }
    } /* while (ip < ilimit) */

    /* save reps for next block */
    *rep.wrapping_add(0) = offset_1;
    *rep.wrapping_add(1) = offset_2;

    iend.offset_from(anchor) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 5),
        6 => ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 6),
        7 => ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 7),
        _ => ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 4),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 5),
        6 => ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 6),
        7 => ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 7),
        _ => ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 4),
    }
}

/* ===  ZSTD_compressBlock_doubleFast_extDict  =========================== */

pub unsafe fn ZSTD_compressBlock_doubleFast_extDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    mls: U32,
) -> size_t {
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
    let endIndex: U32 = ((istart.offset_from(base) as size_t) + srcSize) as U32;
    let lowLimit: U32 = ZSTD_getLowestMatchIndex(ms, endIndex, (*cParams).windowLog);
    let dictStartIndex: U32 = lowLimit;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStartIndex: U32 = if dictLimit > lowLimit { dictLimit } else { lowLimit };
    let prefixStart: *const BYTE = base.wrapping_add(prefixStartIndex as usize);
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictStart: *const BYTE = dictBase.wrapping_add(dictStartIndex as usize);
    let dictEnd: *const BYTE = dictBase.wrapping_add(prefixStartIndex as usize);
    let mut offset_1: U32 = *rep.wrapping_add(0);
    let mut offset_2: U32 = *rep.wrapping_add(1);

    /* if extDict is invalidated due to maxDistance, switch to "regular" variant */
    if prefixStartIndex == dictStartIndex {
        return ZSTD_compressBlock_doubleFast(ms, seqStore, rep, src, srcSize);
    }

    /* Search Loop */
    while ip < ilimit {
        let hSmall: size_t = ZSTD_hashPtr(ip as *const c_void, hBitsS, mls);
        let matchIndex: U32 = *hashSmall.wrapping_add(hSmall as usize);
        let matchBase: *const BYTE = if matchIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let mut r#match: *const BYTE = matchBase.wrapping_add(matchIndex as usize);

        let hLong: size_t = ZSTD_hashPtr(ip as *const c_void, hBitsL, 8);
        let matchLongIndex: U32 = *hashLong.wrapping_add(hLong as usize);
        let matchLongBase: *const BYTE = if matchLongIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let mut matchLong: *const BYTE = matchLongBase.wrapping_add(matchLongIndex as usize);

        let curr: U32 = ip.offset_from(base) as U32;
        let repIndex: U32 = curr + 1 - offset_1;
        let repBase: *const BYTE = if repIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let repMatch: *const BYTE = repBase.wrapping_add(repIndex as usize);
        let mut mLength: size_t;
        *hashSmall.wrapping_add(hSmall as usize) = curr;
        *hashLong.wrapping_add(hLong as usize) = curr; /* update hash table */

        if (((ZSTD_index_overlap_check(prefixStartIndex, repIndex) as u32)
            & (offset_1 <= curr + 1 - dictStartIndex) as u32)
            != 0)
            && (MEM_read32(repMatch)
                == MEM_read32(ip.wrapping_add(1)))
        {
            let repMatchEnd: *const BYTE = if repIndex < prefixStartIndex {
                dictEnd
            } else {
                iend
            };
            mLength = ZSTD_count_2segments(
                ip.wrapping_add(1).wrapping_add(4),
                repMatch.wrapping_add(4),
                iend,
                repMatchEnd,
                prefixStart,
            ) + 4;
            ip = ip.wrapping_add(1);
            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as size_t,
                anchor,
                iend,
                REPCODE1_TO_OFFBASE,
                mLength,
            );
        } else {
            if (matchLongIndex > dictStartIndex)
                && (MEM_read64(matchLong) == MEM_read64(ip))
            {
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
                let offset: U32;
                mLength = ZSTD_count_2segments(
                    ip.wrapping_add(8),
                    matchLong.wrapping_add(8),
                    iend,
                    matchEnd,
                    prefixStart,
                ) + 8;
                offset = curr - matchLongIndex;
                while ((ip > anchor) as u32 & (matchLong > lowMatchPtr) as u32) != 0
                    && (*ip.wrapping_offset(-1) == *matchLong.wrapping_offset(-1))
                {
                    ip = ip.wrapping_offset(-1);
                    matchLong = matchLong.wrapping_offset(-1);
                    mLength += 1;
                } /* catch up */
                offset_2 = offset_1;
                offset_1 = offset;
                ZSTD_storeSeq(
                    seqStore,
                    ip.offset_from(anchor) as size_t,
                    anchor,
                    iend,
                    OFFSET_TO_OFFBASE(offset),
                    mLength,
                );
            } else if (matchIndex > dictStartIndex)
                && (MEM_read32(r#match) == MEM_read32(ip))
            {
                let h3: size_t = ZSTD_hashPtr(ip.wrapping_add(1) as *const c_void, hBitsL, 8);
                let matchIndex3: U32 = *hashLong.wrapping_add(h3 as usize);
                let match3Base: *const BYTE = if matchIndex3 < prefixStartIndex {
                    dictBase
                } else {
                    base
                };
                let mut match3: *const BYTE = match3Base.wrapping_add(matchIndex3 as usize);
                let offset: U32;
                *hashLong.wrapping_add(h3 as usize) = curr + 1;
                if (matchIndex3 > dictStartIndex)
                    && (MEM_read64(match3)
                        == MEM_read64(ip.wrapping_add(1)))
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
                    offset = curr + 1 - matchIndex3;
                    while ((ip > anchor) as u32 & (match3 > lowMatchPtr) as u32) != 0
                        && (*ip.wrapping_offset(-1) == *match3.wrapping_offset(-1))
                    {
                        ip = ip.wrapping_offset(-1);
                        match3 = match3.wrapping_offset(-1);
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
                    offset = curr - matchIndex;
                    while ((ip > anchor) as u32 & (r#match > lowMatchPtr) as u32) != 0
                        && (*ip.wrapping_offset(-1) == *r#match.wrapping_offset(-1))
                    {
                        ip = ip.wrapping_offset(-1);
                        r#match = r#match.wrapping_offset(-1);
                        mLength += 1;
                    } /* catch up */
                }
                offset_2 = offset_1;
                offset_1 = offset;
                ZSTD_storeSeq(
                    seqStore,
                    ip.offset_from(anchor) as size_t,
                    anchor,
                    iend,
                    OFFSET_TO_OFFBASE(offset),
                    mLength,
                );
            } else {
                ip = ip.wrapping_add((((ip.offset_from(anchor)) as size_t >> kSearchStrength)
                    + 1) as usize);
                continue;
            }
        }

        /* move to next sequence start */
        ip = ip.wrapping_add(mLength);
        anchor = ip;

        if ip <= ilimit {
            /* Complementary insertion */
            {
                let indexToInsert: U32 = curr + 2;
                *hashLong.wrapping_add(ZSTD_hashPtr(
                    base.wrapping_add(indexToInsert as usize) as *const c_void,
                    hBitsL,
                    8,
                ) as usize) = indexToInsert;
                *hashLong.wrapping_add(ZSTD_hashPtr(
                    ip.wrapping_sub(2) as *const c_void,
                    hBitsL,
                    8,
                ) as usize) = ip.wrapping_sub(2).offset_from(base) as U32;
                *hashSmall.wrapping_add(ZSTD_hashPtr(
                    base.wrapping_add(indexToInsert as usize) as *const c_void,
                    hBitsS,
                    mls,
                ) as usize) = indexToInsert;
                *hashSmall.wrapping_add(ZSTD_hashPtr(
                    ip.wrapping_sub(1) as *const c_void,
                    hBitsS,
                    mls,
                ) as usize) = ip.wrapping_sub(1).offset_from(base) as U32;
            }

            /* check immediate repcode */
            while ip <= ilimit {
                let current2: U32 = ip.offset_from(base) as U32;
                let repIndex2: U32 = current2 - offset_2;
                let repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictBase.wrapping_add(repIndex2 as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if (((ZSTD_index_overlap_check(prefixStartIndex, repIndex2) as u32)
                    & (offset_2 <= current2 - dictStartIndex) as u32)
                    != 0)
                    && (MEM_read32(repMatch2)
                        == MEM_read32(ip))
                {
                    let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    let repLength2: size_t = ZSTD_count_2segments(
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
                    *hashSmall.wrapping_add(ZSTD_hashPtr(ip as *const c_void, hBitsS, mls) as usize) =
                        current2;
                    *hashLong.wrapping_add(ZSTD_hashPtr(ip as *const c_void, hBitsL, 8) as usize) =
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
    *rep.wrapping_add(0) = offset_1;
    *rep.wrapping_add(1) = offset_2;

    iend.offset_from(anchor) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 5),
        6 => ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 6),
        7 => ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 7),
        _ => ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 4),
    }
}
