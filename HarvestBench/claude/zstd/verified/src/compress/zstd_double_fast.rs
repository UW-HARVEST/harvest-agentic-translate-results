//! Translation of compress/zstd_double_fast.c — double-fast block compressor.

use core::ffi::c_void;

use crate::common::mem::{mem_read32, mem_read64, U32};

use crate::compress::zstd_compress_internal::{
    ZSTD_MatchState_t, SeqStore_t,
    ZSTD_dictTableLoadMethod_e, ZSTD_tableFillPurpose_e, ZSTD_tfp_forCDict, ZSTD_dtlm_fast,
    ZSTD_hashPtr, ZSTD_writeTaggedIndex, ZSTD_selectAddr, ZSTD_comparePackedTags,
    ZSTD_index_overlap_check, ZSTD_getLowestPrefixIndex, ZSTD_getLowestMatchIndex,
    ZSTD_storeSeq, ZSTD_count, ZSTD_count_2segments,
    REPCODE1_TO_OFFBASE, OFFSET_TO_OFFBASE,
    ZSTD_SHORT_CACHE_TAG_BITS, kSearchStrength, HASH_READ_SIZE,
};

#[inline]
unsafe fn MEM_read32(p: *const u8) -> u32 {
    mem_read32(p as *const c_void)
}

#[inline]
unsafe fn MEM_read64(p: *const u8) -> u64 {
    mem_read64(p as *const c_void)
}

#[inline]
unsafe fn hashPtr(p: *const u8, hBits: U32, mls: U32) -> usize {
    ZSTD_hashPtr(p as *const c_void, hBits, mls)
}

unsafe fn ZSTD_fillDoubleHashTableForCDict(
    ms: *mut ZSTD_MatchState_t,
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams = &(*ms).cParams;
    let hashLarge = (*ms).hashTable;
    let hBitsL: U32 = cParams.hashLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let mls: U32 = cParams.minMatch;
    let hashSmall = (*ms).chainTable;
    let hBitsS: U32 = cParams.chainLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let base = (*ms).window.base;
    let mut ip = base.wrapping_add((*ms).nextToUpdate as usize);
    let iend = (end as *const u8).wrapping_sub(HASH_READ_SIZE);
    let fastHashFillStep: U32 = 3;

    /* Always insert every fastHashFillStep position into the hash tables.
     * Insert the other positions into the large hash table if their entry
     * is empty.
     */
    while ip.wrapping_add(fastHashFillStep as usize - 1) <= iend {
        let curr: U32 = ip.offset_from(base) as U32;
        let mut i: U32 = 0;
        while i < fastHashFillStep {
            let smHashAndTag: usize = hashPtr(ip.wrapping_add(i as usize), hBitsS, mls);
            let lgHashAndTag: usize = hashPtr(ip.wrapping_add(i as usize), hBitsL, 8);
            if i == 0 {
                ZSTD_writeTaggedIndex(hashSmall, smHashAndTag, curr + i);
            }
            if i == 0 || *hashLarge.add(lgHashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS) == 0 {
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

unsafe fn ZSTD_fillDoubleHashTableForCCtx(
    ms: *mut ZSTD_MatchState_t,
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams = &(*ms).cParams;
    let hashLarge = (*ms).hashTable;
    let hBitsL: U32 = cParams.hashLog;
    let mls: U32 = cParams.minMatch;
    let hashSmall = (*ms).chainTable;
    let hBitsS: U32 = cParams.chainLog;
    let base = (*ms).window.base;
    let mut ip = base.wrapping_add((*ms).nextToUpdate as usize);
    let iend = (end as *const u8).wrapping_sub(HASH_READ_SIZE);
    let fastHashFillStep: U32 = 3;

    while ip.wrapping_add(fastHashFillStep as usize - 1) <= iend {
        let curr: U32 = ip.offset_from(base) as U32;
        let mut i: U32 = 0;
        while i < fastHashFillStep {
            let smHash: usize = hashPtr(ip.wrapping_add(i as usize), hBitsS, mls);
            let lgHash: usize = hashPtr(ip.wrapping_add(i as usize), hBitsL, 8);
            if i == 0 {
                *hashSmall.add(smHash) = curr + i;
            }
            if i == 0 || *hashLarge.add(lgHash) == 0 {
                *hashLarge.add(lgHash) = curr + i;
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

unsafe fn ZSTD_compressBlock_doubleFast_noDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32, /* template */
) -> usize {
    let cParams = &(*ms).cParams;
    let hashLong = (*ms).hashTable;
    let hBitsL: U32 = cParams.hashLog;
    let hashSmall = (*ms).chainTable;
    let hBitsS: U32 = cParams.chainLog;
    let base = (*ms).window.base;
    let istart = src as *const u8;
    let mut anchor = istart;
    let endIndex: U32 = ((istart.offset_from(base) as usize) + srcSize) as U32;
    /* presumes that, if there is a dictionary, it must be using Attach mode */
    let prefixLowestIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, cParams.windowLog);
    let prefixLowest = base.wrapping_add(prefixLowestIndex as usize);
    let iend = istart.wrapping_add(srcSize);
    let ilimit = iend.wrapping_sub(HASH_READ_SIZE);
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
    let mut nextStep: *const u8;
    let mut step: usize; /* the current step size */

    let mut hl0: usize; /* the long hash at ip */
    let mut hl1: usize; /* the long hash at ip1 */

    let mut idxl0: U32; /* the long match index for ip */
    let mut idxl1: U32; /* the long match index for ip1 */

    let mut matchl0: *const u8; /* the long match for ip */
    let mut matchs0: *const u8; /* the short match for ip */
    let mut matchl1: *const u8; /* the long match for ip1 */
    let mut matchs0_safe: *const u8; /* matchs0 or safe address */

    let mut ip = istart; /* the current position */
    let mut ip1: *const u8; /* the next position */
    /* Array of ~random data, should have low probability of matching data
     * we load from here instead of from tables, if matchl0/matchl1 are
     * invalid indices. Used to avoid unpredictable branches. */
    let dummy: [u8; 10] = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xe2, 0xb4];

    /* init */
    ip = ip.wrapping_add((ip.offset_from(prefixLowest) == 0) as usize);
    {
        let current: U32 = ip.offset_from(base) as U32;
        let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, current, cParams.windowLog);
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
            /* _cleanup */
            offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
                offsetSaved1
            } else {
                offsetSaved2
            };
            *rep.add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
            *rep.add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };
            return iend.offset_from(anchor) as usize;
        }

        hl0 = hashPtr(ip, hBitsL, 8);
        idxl0 = *hashLong.add(hl0);
        matchl0 = base.wrapping_add(idxl0 as usize);

        'ms: {
            'mf: {
                'snl: {
                    /* Inner Loop: one iteration per search / position */
                    loop {
                        let hs0: usize = hashPtr(ip, hBitsS, mls);
                        let idxs0: U32 = *hashSmall.add(hs0);
                        curr = ip.offset_from(base) as U32;
                        matchs0 = base.wrapping_add(idxs0 as usize);

                        *hashLong.add(hl0) = curr; /* update hash tables */
                        *hashSmall.add(hs0) = curr;

                        /* check noDict repcode */
                        if (offset_1 > 0)
                            & (MEM_read32(ip.wrapping_add(1).wrapping_sub(offset_1 as usize))
                                == MEM_read32(ip.wrapping_add(1)))
                        {
                            mLength = ZSTD_count(
                                ip.wrapping_add(5),
                                ip.wrapping_add(5).wrapping_sub(offset_1 as usize),
                                iend,
                            ) + 4;
                            ip = ip.wrapping_add(1);
                            ZSTD_storeSeq(
                                seqStore,
                                ip.offset_from(anchor) as usize,
                                anchor,
                                iend,
                                REPCODE1_TO_OFFBASE,
                                mLength,
                            );
                            break 'ms;
                        }

                        hl1 = hashPtr(ip1, hBitsL, 8);

                        {
                            let matchl0_safe: *const u8 =
                                ZSTD_selectAddr(idxl0, prefixLowestIndex, matchl0, dummy.as_ptr());

                            /* check prefix long match */
                            if MEM_read64(matchl0_safe) == MEM_read64(ip) && matchl0_safe == matchl0
                            {
                                mLength = ZSTD_count(ip.wrapping_add(8), matchl0.wrapping_add(8), iend) + 8;
                                offset = ip.offset_from(matchl0) as U32;
                                while ((ip > anchor) & (matchl0 > prefixLowest))
                                    && (*ip.wrapping_sub(1) == *matchl0.wrapping_sub(1))
                                {
                                    ip = ip.wrapping_sub(1);
                                    matchl0 = matchl0.wrapping_sub(1);
                                    mLength += 1;
                                } /* catch up */
                                break 'mf;
                            }
                        }

                        idxl1 = *hashLong.add(hl1);
                        matchl1 = base.wrapping_add(idxl1 as usize);

                        /* Same optimization as matchl0 above */
                        matchs0_safe =
                            ZSTD_selectAddr(idxs0, prefixLowestIndex, matchs0, dummy.as_ptr());

                        /* check prefix short match */
                        if MEM_read32(matchs0_safe) == MEM_read32(ip) && matchs0_safe == matchs0 {
                            break 'snl;
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
                            break;
                        }
                    }

                    /* _cleanup */
                    offsetSaved2 = if (offsetSaved1 != 0) && (offset_1 != 0) {
                        offsetSaved1
                    } else {
                        offsetSaved2
                    };
                    *rep.add(0) = if offset_1 != 0 { offset_1 } else { offsetSaved1 };
                    *rep.add(1) = if offset_2 != 0 { offset_2 } else { offsetSaved2 };
                    return iend.offset_from(anchor) as usize;
                }

                /* _search_next_long */
                /* short match found: let's check for a longer one */
                mLength = ZSTD_count(ip.wrapping_add(4), matchs0.wrapping_add(4), iend) + 4;
                offset = ip.offset_from(matchs0) as U32;

                /* check long match at +1 position */
                if (idxl1 > prefixLowestIndex) && (MEM_read64(matchl1) == MEM_read64(ip1)) {
                    let l1len: usize = ZSTD_count(ip1.wrapping_add(8), matchl1.wrapping_add(8), iend) + 8;
                    if l1len > mLength {
                        /* use the long match instead */
                        ip = ip1;
                        mLength = l1len;
                        offset = ip.offset_from(matchl1) as U32;
                        matchs0 = matchl1;
                    }
                }

                while ((ip > anchor) & (matchs0 > prefixLowest))
                    && (*ip.wrapping_sub(1) == *matchs0.wrapping_sub(1))
                {
                    ip = ip.wrapping_sub(1);
                    matchs0 = matchs0.wrapping_sub(1);
                    mLength += 1;
                } /* complete backward */

                /* fall-through */
            }

            /* _match_found: requires ip, offset, mLength */
            offset_2 = offset_1;
            offset_1 = offset;

            if step < 4 {
                /* It is unsafe to write this value back to the hashtable when ip1 is
                 * greater than or equal to the new ip we will have after we're done
                 * processing this match. */
                *hashLong.add(hl1) = ip1.offset_from(base) as U32;
            }

            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as usize,
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
                let indexToInsert: U32 = curr + 2;
                *hashLong.add(hashPtr(base.wrapping_add(indexToInsert as usize), hBitsL, 8)) =
                    indexToInsert;
                *hashLong.add(hashPtr(ip.wrapping_sub(2), hBitsL, 8)) =
                    ip.wrapping_sub(2).offset_from(base) as U32;
                *hashSmall.add(hashPtr(base.wrapping_add(indexToInsert as usize), hBitsS, mls)) =
                    indexToInsert;
                *hashSmall.add(hashPtr(ip.wrapping_sub(1), hBitsS, mls)) =
                    ip.wrapping_sub(1).offset_from(base) as U32;
            }

            /* check immediate repcode */
            while (ip <= ilimit)
                && ((offset_2 > 0)
                    & (MEM_read32(ip) == MEM_read32(ip.wrapping_sub(offset_2 as usize))))
            {
                /* store sequence */
                let rLength: usize =
                    ZSTD_count(ip.wrapping_add(4), ip.wrapping_add(4).wrapping_sub(offset_2 as usize), iend) + 4;
                let tmpOff: U32 = offset_2;
                offset_2 = offset_1;
                offset_1 = tmpOff; /* swap offset_2 <=> offset_1 */
                *hashSmall.add(hashPtr(ip, hBitsS, mls)) = ip.offset_from(base) as U32;
                *hashLong.add(hashPtr(ip, hBitsL, 8)) = ip.offset_from(base) as U32;
                ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, rLength);
                ip = ip.wrapping_add(rLength);
                anchor = ip;
                continue; /* faster when present ... (?) */
            }
        }
    }
}

unsafe fn ZSTD_compressBlock_doubleFast_dictMatchState_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32, /* template */
) -> usize {
    let cParams = &(*ms).cParams;
    let hashLong = (*ms).hashTable;
    let hBitsL: U32 = cParams.hashLog;
    let hashSmall = (*ms).chainTable;
    let hBitsS: U32 = cParams.chainLog;
    let base = (*ms).window.base;
    let istart = src as *const u8;
    let mut ip = istart;
    let mut anchor = istart;
    let endIndex: U32 = ((istart.offset_from(base) as usize) + srcSize) as U32;
    /* presumes that, if there is a dictionary, it must be using Attach mode */
    let prefixLowestIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, cParams.windowLog);
    let prefixLowest = base.wrapping_add(prefixLowestIndex as usize);
    let iend = istart.wrapping_add(srcSize);
    let ilimit = iend.wrapping_sub(HASH_READ_SIZE);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);

    let dms = (*ms).dictMatchState;
    let dictCParams = &(*dms).cParams;
    let dictHashLong = (*dms).hashTable;
    let dictHashSmall = (*dms).chainTable;
    let dictStartIndex: U32 = (*dms).window.dictLimit;
    let dictBase = (*dms).window.base;
    let dictStart = dictBase.wrapping_add(dictStartIndex as usize);
    let dictEnd = (*dms).window.nextSrc;
    let dictIndexDelta: U32 = prefixLowestIndex.wrapping_sub(dictEnd.offset_from(dictBase) as U32);
    let dictHBitsL: U32 = dictCParams.hashLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let dictHBitsS: U32 = dictCParams.chainLog + ZSTD_SHORT_CACHE_TAG_BITS;
    let dictAndPrefixLength: U32 =
        ((ip.offset_from(prefixLowest)) + (dictEnd.offset_from(dictStart))) as U32;

    /* if a dictionary is attached, it must be within window range */
    debug_assert!(
        (*ms).window.dictLimit.wrapping_add(1u32 << cParams.windowLog) >= endIndex
    );

    if (*ms).prefetchCDictTables != 0 {
        /* PREFETCH_AREA no-ops on this target */
    }

    /* init */
    ip = ip.wrapping_add((dictAndPrefixLength == 0) as usize);

    /* dictMatchState repCode checks don't currently handle repCode == 0 disabling. */
    debug_assert!(offset_1 <= dictAndPrefixLength);
    debug_assert!(offset_2 <= dictAndPrefixLength);

    /* Main Search Loop */
    'search: while ip < ilimit {
        /* < instead of <=, because repcode check at (ip+1) */
        let mut mLength: usize;
        let mut offset: U32 = 0;
        let h2: usize = hashPtr(ip, hBitsL, 8);
        let h: usize = hashPtr(ip, hBitsS, mls);
        let dictHashAndTagL: usize = hashPtr(ip, dictHBitsL, 8);
        let dictHashAndTagS: usize = hashPtr(ip, dictHBitsS, mls);
        let dictMatchIndexAndTagL: U32 =
            *dictHashLong.add(dictHashAndTagL >> ZSTD_SHORT_CACHE_TAG_BITS);
        let dictMatchIndexAndTagS: U32 =
            *dictHashSmall.add(dictHashAndTagS >> ZSTD_SHORT_CACHE_TAG_BITS);
        let dictTagsMatchL: i32 =
            ZSTD_comparePackedTags(dictMatchIndexAndTagL as usize, dictHashAndTagL);
        let dictTagsMatchS: i32 =
            ZSTD_comparePackedTags(dictMatchIndexAndTagS as usize, dictHashAndTagS);
        let curr: U32 = ip.offset_from(base) as U32;
        let matchIndexL: U32 = *hashLong.add(h2);
        let mut matchIndexS: U32 = *hashSmall.add(h);
        let mut matchLong = base.wrapping_add(matchIndexL as usize);
        let mut r#match = base.wrapping_add(matchIndexS as usize);
        let repIndex: U32 = curr.wrapping_add(1).wrapping_sub(offset_1);
        let repMatch: *const u8 = if repIndex < prefixLowestIndex {
            dictBase.wrapping_add(repIndex.wrapping_sub(dictIndexDelta) as usize)
        } else {
            base.wrapping_add(repIndex as usize)
        };
        *hashLong.add(h2) = curr; /* update hash tables */
        *hashSmall.add(h) = curr;

        'ms: {
            'mf: {
                'snl: {
                    /* check repcode */
                    if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0)
                        && (MEM_read32(repMatch) == MEM_read32(ip.wrapping_add(1)))
                    {
                        let repMatchEnd = if repIndex < prefixLowestIndex { dictEnd } else { iend };
                        mLength = ZSTD_count_2segments(
                            ip.wrapping_add(5),
                            repMatch.wrapping_add(4),
                            iend,
                            repMatchEnd,
                            prefixLowest,
                        ) + 4;
                        ip = ip.wrapping_add(1);
                        ZSTD_storeSeq(
                            seqStore,
                            ip.offset_from(anchor) as usize,
                            anchor,
                            iend,
                            REPCODE1_TO_OFFBASE,
                            mLength,
                        );
                        /* goto _match_stored */
                        ip = ip.wrapping_add(mLength);
                        anchor = ip;
                        break 'ms;
                    }

                    if (matchIndexL >= prefixLowestIndex) && (MEM_read64(matchLong) == MEM_read64(ip))
                    {
                        /* check prefix long match */
                        mLength = ZSTD_count(ip.wrapping_add(8), matchLong.wrapping_add(8), iend) + 8;
                        offset = ip.offset_from(matchLong) as U32;
                        while ((ip > anchor) & (matchLong > prefixLowest))
                            && (*ip.wrapping_sub(1) == *matchLong.wrapping_sub(1))
                        {
                            ip = ip.wrapping_sub(1);
                            matchLong = matchLong.wrapping_sub(1);
                            mLength += 1;
                        } /* catch up */
                        break 'mf;
                    } else if dictTagsMatchL != 0 {
                        /* check dictMatchState long match */
                        let dictMatchIndexL: U32 = dictMatchIndexAndTagL >> ZSTD_SHORT_CACHE_TAG_BITS;
                        let mut dictMatchL = dictBase.wrapping_add(dictMatchIndexL as usize);
                        debug_assert!(dictMatchL < dictEnd);

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
                            while ((ip > anchor) & (dictMatchL > dictStart))
                                && (*ip.wrapping_sub(1) == *dictMatchL.wrapping_sub(1))
                            {
                                ip = ip.wrapping_sub(1);
                                dictMatchL = dictMatchL.wrapping_sub(1);
                                mLength += 1;
                            } /* catch up */
                            break 'mf;
                        }
                    }

                    if matchIndexS > prefixLowestIndex {
                        /* short match candidate */
                        if MEM_read32(r#match) == MEM_read32(ip) {
                            break 'snl;
                        }
                    } else if dictTagsMatchS != 0 {
                        /* check dictMatchState short match */
                        let dictMatchIndexS: U32 = dictMatchIndexAndTagS >> ZSTD_SHORT_CACHE_TAG_BITS;
                        r#match = dictBase.wrapping_add(dictMatchIndexS as usize);
                        matchIndexS = dictMatchIndexS.wrapping_add(dictIndexDelta);

                        if r#match > dictStart && MEM_read32(r#match) == MEM_read32(ip) {
                            break 'snl;
                        }
                    }

                    ip = ip.wrapping_add((((ip.offset_from(anchor)) >> kSearchStrength) + 1) as usize);
                    continue 'search;
                }

                /* _search_next_long */
                {
                    let hl3: usize = hashPtr(ip.wrapping_add(1), hBitsL, 8);
                    let dictHashAndTagL3: usize = hashPtr(ip.wrapping_add(1), dictHBitsL, 8);
                    let matchIndexL3: U32 = *hashLong.add(hl3);
                    let dictMatchIndexAndTagL3: U32 =
                        *dictHashLong.add(dictHashAndTagL3 >> ZSTD_SHORT_CACHE_TAG_BITS);
                    let dictTagsMatchL3: i32 =
                        ZSTD_comparePackedTags(dictMatchIndexAndTagL3 as usize, dictHashAndTagL3);
                    let mut matchL3 = base.wrapping_add(matchIndexL3 as usize);
                    *hashLong.add(hl3) = curr + 1;

                    /* check prefix long +1 match */
                    if (matchIndexL3 >= prefixLowestIndex)
                        && (MEM_read64(matchL3) == MEM_read64(ip.wrapping_add(1)))
                    {
                        mLength = ZSTD_count(ip.wrapping_add(9), matchL3.wrapping_add(8), iend) + 8;
                        ip = ip.wrapping_add(1);
                        offset = ip.offset_from(matchL3) as U32;
                        while ((ip > anchor) & (matchL3 > prefixLowest))
                            && (*ip.wrapping_sub(1) == *matchL3.wrapping_sub(1))
                        {
                            ip = ip.wrapping_sub(1);
                            matchL3 = matchL3.wrapping_sub(1);
                            mLength += 1;
                        } /* catch up */
                        break 'mf;
                    } else if dictTagsMatchL3 != 0 {
                        /* check dict long +1 match */
                        let dictMatchIndexL3: U32 =
                            dictMatchIndexAndTagL3 >> ZSTD_SHORT_CACHE_TAG_BITS;
                        let mut dictMatchL3 = dictBase.wrapping_add(dictMatchIndexL3 as usize);
                        debug_assert!(dictMatchL3 < dictEnd);
                        if dictMatchL3 > dictStart
                            && MEM_read64(dictMatchL3) == MEM_read64(ip.wrapping_add(1))
                        {
                            mLength = ZSTD_count_2segments(
                                ip.wrapping_add(9),
                                dictMatchL3.wrapping_add(8),
                                iend,
                                dictEnd,
                                prefixLowest,
                            ) + 8;
                            ip = ip.wrapping_add(1);
                            offset = (curr + 1)
                                .wrapping_sub(dictMatchIndexL3)
                                .wrapping_sub(dictIndexDelta);
                            while ((ip > anchor) & (dictMatchL3 > dictStart))
                                && (*ip.wrapping_sub(1) == *dictMatchL3.wrapping_sub(1))
                            {
                                ip = ip.wrapping_sub(1);
                                dictMatchL3 = dictMatchL3.wrapping_sub(1);
                                mLength += 1;
                            } /* catch up */
                            break 'mf;
                        }
                    }
                }

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
                    while ((ip > anchor) & (r#match > dictStart))
                        && (*ip.wrapping_sub(1) == *r#match.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        r#match = r#match.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                } else {
                    mLength = ZSTD_count(ip.wrapping_add(4), r#match.wrapping_add(4), iend) + 4;
                    offset = ip.offset_from(r#match) as U32;
                    while ((ip > anchor) & (r#match > prefixLowest))
                        && (*ip.wrapping_sub(1) == *r#match.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        r#match = r#match.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                }
            }

            /* _match_found */
            offset_2 = offset_1;
            offset_1 = offset;

            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as usize,
                anchor,
                iend,
                OFFSET_TO_OFFBASE(offset),
                mLength,
            );

            /* _match_stored */
            ip = ip.wrapping_add(mLength);
            anchor = ip;
        }

        if ip <= ilimit {
            /* Complementary insertion */
            /* done after iLimit test, as candidates could be > iend-8 */
            {
                let indexToInsert: U32 = curr + 2;
                *hashLong.add(hashPtr(base.wrapping_add(indexToInsert as usize), hBitsL, 8)) =
                    indexToInsert;
                *hashLong.add(hashPtr(ip.wrapping_sub(2), hBitsL, 8)) =
                    ip.wrapping_sub(2).offset_from(base) as U32;
                *hashSmall.add(hashPtr(base.wrapping_add(indexToInsert as usize), hBitsS, mls)) =
                    indexToInsert;
                *hashSmall.add(hashPtr(ip.wrapping_sub(1), hBitsS, mls)) =
                    ip.wrapping_sub(1).offset_from(base) as U32;
            }

            /* check immediate repcode */
            while ip <= ilimit {
                let current2: U32 = ip.offset_from(base) as U32;
                let repIndex2: U32 = current2.wrapping_sub(offset_2);
                let repMatch2: *const u8 = if repIndex2 < prefixLowestIndex {
                    dictBase.wrapping_add(repIndex2 as usize).wrapping_sub(dictIndexDelta as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if (ZSTD_index_overlap_check(prefixLowestIndex, repIndex2) != 0)
                    && (MEM_read32(repMatch2) == MEM_read32(ip))
                {
                    let repEnd2 = if repIndex2 < prefixLowestIndex { dictEnd } else { iend };
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
                    *hashSmall.add(hashPtr(ip, hBitsS, mls)) = current2;
                    *hashLong.add(hashPtr(ip, hBitsL, 8)) = current2;
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
    iend.offset_from(anchor) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 5),
        6 => ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 6),
        7 => ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 7),
        /* default: includes case 3 */
        _ => ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 4),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 5),
        6 => ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 6),
        7 => ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 7),
        /* default: includes case 3 */
        _ => ZSTD_compressBlock_doubleFast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 4),
    }
}

unsafe fn ZSTD_compressBlock_doubleFast_extDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32, /* template */
) -> usize {
    let cParams = &(*ms).cParams;
    let hashLong = (*ms).hashTable;
    let hBitsL: U32 = cParams.hashLog;
    let hashSmall = (*ms).chainTable;
    let hBitsS: U32 = cParams.chainLog;
    let istart = src as *const u8;
    let mut ip = istart;
    let mut anchor = istart;
    let iend = istart.wrapping_add(srcSize);
    let ilimit = iend.wrapping_sub(8);
    let base = (*ms).window.base;
    let endIndex: U32 = ((istart.offset_from(base) as usize) + srcSize) as U32;
    let lowLimit: U32 = ZSTD_getLowestMatchIndex(ms, endIndex, cParams.windowLog);
    let dictStartIndex: U32 = lowLimit;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStartIndex: U32 = if dictLimit > lowLimit { dictLimit } else { lowLimit };
    let prefixStart = base.wrapping_add(prefixStartIndex as usize);
    let dictBase = (*ms).window.dictBase;
    let dictStart = dictBase.wrapping_add(dictStartIndex as usize);
    let dictEnd = dictBase.wrapping_add(prefixStartIndex as usize);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);

    /* if extDict is invalidated due to maxDistance, switch to "regular" variant */
    if prefixStartIndex == dictStartIndex {
        return ZSTD_compressBlock_doubleFast(ms, seqStore, rep, src, srcSize);
    }

    /* Search Loop */
    while ip < ilimit {
        /* < instead of <=, because (ip+1) */
        let hSmall: usize = hashPtr(ip, hBitsS, mls);
        let matchIndex: U32 = *hashSmall.add(hSmall);
        let matchBase = if matchIndex < prefixStartIndex { dictBase } else { base };
        let mut r#match = matchBase.wrapping_add(matchIndex as usize);

        let hLong: usize = hashPtr(ip, hBitsL, 8);
        let matchLongIndex: U32 = *hashLong.add(hLong);
        let matchLongBase = if matchLongIndex < prefixStartIndex { dictBase } else { base };
        let mut matchLong = matchLongBase.wrapping_add(matchLongIndex as usize);

        let curr: U32 = ip.offset_from(base) as U32;
        let repIndex: U32 = curr.wrapping_add(1).wrapping_sub(offset_1); /* offset_1 expected <= curr +1 */
        let repBase = if repIndex < prefixStartIndex { dictBase } else { base };
        let repMatch = repBase.wrapping_add(repIndex as usize);
        let mut mLength: usize;
        *hashSmall.add(hSmall) = curr; /* update hash table */
        *hashLong.add(hLong) = curr;

        if ((ZSTD_index_overlap_check(prefixStartIndex, repIndex) != 0)
            & (offset_1 <= curr.wrapping_add(1).wrapping_sub(dictStartIndex)))
            && (MEM_read32(repMatch) == MEM_read32(ip.wrapping_add(1)))
        {
            let repMatchEnd = if repIndex < prefixStartIndex { dictEnd } else { iend };
            mLength = ZSTD_count_2segments(
                ip.wrapping_add(5),
                repMatch.wrapping_add(4),
                iend,
                repMatchEnd,
                prefixStart,
            ) + 4;
            ip = ip.wrapping_add(1);
            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as usize,
                anchor,
                iend,
                REPCODE1_TO_OFFBASE,
                mLength,
            );
        } else {
            if (matchLongIndex > dictStartIndex) && (MEM_read64(matchLong) == MEM_read64(ip)) {
                let matchEnd = if matchLongIndex < prefixStartIndex { dictEnd } else { iend };
                let lowMatchPtr = if matchLongIndex < prefixStartIndex { dictStart } else { prefixStart };
                let offset: U32;
                mLength = ZSTD_count_2segments(
                    ip.wrapping_add(8),
                    matchLong.wrapping_add(8),
                    iend,
                    matchEnd,
                    prefixStart,
                ) + 8;
                offset = curr.wrapping_sub(matchLongIndex);
                while ((ip > anchor) & (matchLong > lowMatchPtr))
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
                    ip.offset_from(anchor) as usize,
                    anchor,
                    iend,
                    OFFSET_TO_OFFBASE(offset),
                    mLength,
                );
            } else if (matchIndex > dictStartIndex) && (MEM_read32(r#match) == MEM_read32(ip)) {
                let h3: usize = hashPtr(ip.wrapping_add(1), hBitsL, 8);
                let matchIndex3: U32 = *hashLong.add(h3);
                let match3Base = if matchIndex3 < prefixStartIndex { dictBase } else { base };
                let mut match3 = match3Base.wrapping_add(matchIndex3 as usize);
                let offset: U32;
                *hashLong.add(h3) = curr + 1;
                if (matchIndex3 > dictStartIndex)
                    && (MEM_read64(match3) == MEM_read64(ip.wrapping_add(1)))
                {
                    let matchEnd = if matchIndex3 < prefixStartIndex { dictEnd } else { iend };
                    let lowMatchPtr = if matchIndex3 < prefixStartIndex { dictStart } else { prefixStart };
                    mLength = ZSTD_count_2segments(
                        ip.wrapping_add(9),
                        match3.wrapping_add(8),
                        iend,
                        matchEnd,
                        prefixStart,
                    ) + 8;
                    ip = ip.wrapping_add(1);
                    offset = (curr + 1).wrapping_sub(matchIndex3);
                    while ((ip > anchor) & (match3 > lowMatchPtr))
                        && (*ip.wrapping_sub(1) == *match3.wrapping_sub(1))
                    {
                        ip = ip.wrapping_sub(1);
                        match3 = match3.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                } else {
                    let matchEnd = if matchIndex < prefixStartIndex { dictEnd } else { iend };
                    let lowMatchPtr = if matchIndex < prefixStartIndex { dictStart } else { prefixStart };
                    mLength = ZSTD_count_2segments(
                        ip.wrapping_add(4),
                        r#match.wrapping_add(4),
                        iend,
                        matchEnd,
                        prefixStart,
                    ) + 4;
                    offset = curr.wrapping_sub(matchIndex);
                    while ((ip > anchor) & (r#match > lowMatchPtr))
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
                    ip.offset_from(anchor) as usize,
                    anchor,
                    iend,
                    OFFSET_TO_OFFBASE(offset),
                    mLength,
                );
            } else {
                ip = ip.wrapping_add((((ip.offset_from(anchor)) >> kSearchStrength) + 1) as usize);
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
                let indexToInsert: U32 = curr + 2;
                *hashLong.add(hashPtr(base.wrapping_add(indexToInsert as usize), hBitsL, 8)) =
                    indexToInsert;
                *hashLong.add(hashPtr(ip.wrapping_sub(2), hBitsL, 8)) =
                    ip.wrapping_sub(2).offset_from(base) as U32;
                *hashSmall.add(hashPtr(base.wrapping_add(indexToInsert as usize), hBitsS, mls)) =
                    indexToInsert;
                *hashSmall.add(hashPtr(ip.wrapping_sub(1), hBitsS, mls)) =
                    ip.wrapping_sub(1).offset_from(base) as U32;
            }

            /* check immediate repcode */
            while ip <= ilimit {
                let current2: U32 = ip.offset_from(base) as U32;
                let repIndex2: U32 = current2.wrapping_sub(offset_2);
                let repMatch2: *const u8 = if repIndex2 < prefixStartIndex {
                    dictBase.wrapping_add(repIndex2 as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if ((ZSTD_index_overlap_check(prefixStartIndex, repIndex2) != 0)
                    & (offset_2 <= current2.wrapping_sub(dictStartIndex)))
                    && (MEM_read32(repMatch2) == MEM_read32(ip))
                {
                    let repEnd2 = if repIndex2 < prefixStartIndex { dictEnd } else { iend };
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
                    *hashSmall.add(hashPtr(ip, hBitsS, mls)) = current2;
                    *hashLong.add(hashPtr(ip, hBitsL, 8)) = current2;
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
    iend.offset_from(anchor) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 5),
        6 => ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 6),
        7 => ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 7),
        /* default: includes case 3 */
        _ => ZSTD_compressBlock_doubleFast_extDict_generic(ms, seqStore, rep, src, srcSize, 4),
    }
}
