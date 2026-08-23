//! Rust translation of `lib/compress/zstd_ldm.c` (long distance matching).

use core::ffi::{c_int, c_uint, c_void};

use crate::common::error_private::{ERROR, ERR_isError, ZSTD_error_dstSize_tooSmall};
use crate::common::mem::{BYTE, U32, U64};
use crate::common::xxhash::ZSTD_XXH64;
use crate::common::zstd_internal::{BOUNDED, MIN, ZSTD_REP_NUM};
use crate::compress::zstd_compress_internal::{
    ldmEntry_t, ldmMatchCandidate_t, ldmParams_t, ldmState_t, rawSeq, RawSeqStore_t, SeqStore_t,
    ZSTD_BlockCompressor_f, ZSTD_MatchState_t, ZSTD_count, ZSTD_count_2segments,
    ZSTD_dictMode_e, ZSTD_dictTableLoadMethod_e, ZSTD_dtlm_fast, ZSTD_matchState_dictMode,
    ZSTD_storeSeq, ZSTD_tableFillPurpose_e, ZSTD_tfp_forCCtx, ZSTD_window_correctOverflow,
    ZSTD_window_enforceMaxDist, ZSTD_window_hasExtDict, ZSTD_window_needOverflowCorrection,
    HASH_READ_SIZE, LDM_BATCH_SIZE, OFFSET_TO_OFFBASE,
};
use crate::compress::zstd_cwksp::ZSTD_cwksp_alloc_size;
use crate::compress::zstd_ldm_geartab::ZSTD_ldm_gearTab;
use crate::zstd_h::{
    ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2, ZSTD_compressionParameters, ZSTD_dfast,
    ZSTD_fast, ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2, ZSTD_ParamSwitch_e, ZSTD_ps_enable,
    ZSTD_strategy, ZSTD_HASHLOG_MAX, ZSTD_HASHLOG_MIN, ZSTD_LDM_BUCKETSIZELOG_MAX,
};

extern "C" {
    fn ZSTD_fillHashTable(
        ms: *mut ZSTD_MatchState_t,
        end: *const c_void,
        dtlm: ZSTD_dictTableLoadMethod_e,
        tfp: ZSTD_tableFillPurpose_e,
    );
    fn ZSTD_fillDoubleHashTable(
        ms: *mut ZSTD_MatchState_t,
        end: *const c_void,
        dtlm: ZSTD_dictTableLoadMethod_e,
        tfp: ZSTD_tableFillPurpose_e,
    );
    fn ZSTD_selectBlockCompressor(
        strat: ZSTD_strategy,
        rowMatchfinderMode: ZSTD_ParamSwitch_e,
        dictMode: ZSTD_dictMode_e,
    ) -> ZSTD_BlockCompressor_f;
}

pub const LDM_BUCKET_SIZE_LOG: c_int = 4;
pub const LDM_MIN_MATCH_LENGTH: c_int = 64;
pub const LDM_HASH_RLOG: c_int = 7;

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct ldmRollingHashState_t {
    rolling: U64,
    stopMask: U64,
}

/** ZSTD_ldm_gear_init():
 *
 * Initializes the rolling hash state such that it will honor the
 * settings in params. */
unsafe fn ZSTD_ldm_gear_init(state: *mut ldmRollingHashState_t, params: *const ldmParams_t) {
    let maxBitsInMask: c_uint = MIN((*params).minMatchLength, 64);
    let hashRateLog: c_uint = (*params).hashRateLog;

    (*state).rolling = !(0 as U32) as U64;

    /* The choice of the splitting criterion is subject to two conditions:
     *   1. it has to trigger on average every 2^(hashRateLog) bytes;
     *   2. ideally, it has to depend on a window of minMatchLength bytes.
     *
     * In the gear hash algorithm, bit n depends on the last n bytes;
     * so in order to obtain a good quality splitting criterion it is
     * preferable to use bits with high weight.
     *
     * To match condition 1 we use a mask with hashRateLog bits set
     * and, because of the previous remark, we make sure these bits
     * have the highest possible weight while still respecting
     * condition 2.
     */
    if hashRateLog > 0 && hashRateLog <= maxBitsInMask {
        (*state).stopMask =
            (((1 as U64) << hashRateLog) - 1) << (maxBitsInMask.wrapping_sub(hashRateLog));
    } else {
        /* In this degenerate case we simply honor the hash rate. */
        (*state).stopMask = ((1 as U64) << hashRateLog) - 1;
    }
}

/** ZSTD_ldm_gear_reset()
 * Feeds [data, data + minMatchLength) into the hash without registering any
 * splits. This effectively resets the hash state. This is used when skipping
 * over data, either at the beginning of a block, or skipping sections.
 */
unsafe fn ZSTD_ldm_gear_reset(
    state: *mut ldmRollingHashState_t,
    data: *const BYTE,
    minMatchLength: usize,
) {
    let mut hash: U64 = (*state).rolling;
    let mut n: usize = 0;

    macro_rules! GEAR_ITER_ONCE {
        () => {{
            hash = (hash << 1).wrapping_add(ZSTD_ldm_gearTab[(*data.add(n) & 0xff) as usize]);
            n += 1;
        }};
    }

    while n + 3 < minMatchLength {
        GEAR_ITER_ONCE!();
        GEAR_ITER_ONCE!();
        GEAR_ITER_ONCE!();
        GEAR_ITER_ONCE!();
    }
    while n < minMatchLength {
        GEAR_ITER_ONCE!();
    }

    /* NOTE: matching the C source exactly -- the computed hash is intentionally
     * (in the C original) not written back into `state->rolling`. */
    let _ = hash;
}

/** ZSTD_ldm_gear_feed():
 *
 * Registers in the splits array all the split points found in the first
 * size bytes following the data pointer. This function terminates when
 * either all the data has been processed or LDM_BATCH_SIZE splits are
 * present in the splits array.
 *
 * Precondition: The splits array must not be full.
 * Returns: The number of bytes processed. */
unsafe fn ZSTD_ldm_gear_feed(
    state: *mut ldmRollingHashState_t,
    data: *const BYTE,
    size: usize,
    splits: *mut usize,
    numSplits: *mut c_uint,
) -> usize {
    let mut n: usize;
    let mut hash: U64;
    let mask: U64;

    hash = (*state).rolling;
    mask = (*state).stopMask;
    n = 0;

    'done: {
        macro_rules! GEAR_ITER_ONCE {
            () => {{
                hash = (hash << 1).wrapping_add(ZSTD_ldm_gearTab[(*data.add(n) & 0xff) as usize]);
                n += 1;
                if (hash & mask) == 0 {
                    *splits.add(*numSplits as usize) = n;
                    *numSplits += 1;
                    if *numSplits as usize == LDM_BATCH_SIZE {
                        break 'done;
                    }
                }
            }};
        }

        while n + 3 < size {
            GEAR_ITER_ONCE!();
            GEAR_ITER_ONCE!();
            GEAR_ITER_ONCE!();
            GEAR_ITER_ONCE!();
        }
        while n < size {
            GEAR_ITER_ONCE!();
        }
    }

    (*state).rolling = hash;
    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_adjustParameters(
    params: *mut ldmParams_t,
    cParams: *const ZSTD_compressionParameters,
) {
    (*params).windowLog = (*cParams).windowLog;
    if (*params).hashRateLog == 0 {
        if (*params).hashLog > 0 {
            /* if params->hashLog is set, derive hashRateLog from it */
            if (*params).windowLog > (*params).hashLog {
                (*params).hashRateLog = (*params).windowLog.wrapping_sub((*params).hashLog);
            }
        } else {
            /* mapping from [fast, rate7] to [btultra2, rate4] */
            (*params).hashRateLog = (7 - ((*cParams).strategy / 3)) as U32;
        }
    }
    if (*params).hashLog == 0 {
        (*params).hashLog = BOUNDED(
            ZSTD_HASHLOG_MIN as U32,
            (*params).windowLog.wrapping_sub((*params).hashRateLog),
            ZSTD_HASHLOG_MAX as U32,
        );
    }
    if (*params).minMatchLength == 0 {
        (*params).minMatchLength = LDM_MIN_MATCH_LENGTH as U32;
        if (*cParams).strategy >= ZSTD_btultra {
            (*params).minMatchLength /= 2;
        }
    }
    if (*params).bucketSizeLog == 0 {
        (*params).bucketSizeLog = BOUNDED(
            LDM_BUCKET_SIZE_LOG as U32,
            (*cParams).strategy as U32,
            ZSTD_LDM_BUCKETSIZELOG_MAX as U32,
        );
    }
    (*params).bucketSizeLog = MIN((*params).bucketSizeLog, (*params).hashLog);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_getTableSize(params: ldmParams_t) -> usize {
    let ldmHSize: usize = (1 as usize) << params.hashLog;
    let ldmBucketSizeLog: usize = MIN(params.bucketSizeLog, params.hashLog) as usize;
    let ldmBucketSize: usize =
        (1 as usize) << ((params.hashLog as usize).wrapping_sub(ldmBucketSizeLog));
    let totalSize: usize = ZSTD_cwksp_alloc_size(ldmBucketSize)
        + ZSTD_cwksp_alloc_size(ldmHSize * core::mem::size_of::<ldmEntry_t>());
    if params.enableLdm == ZSTD_ps_enable {
        totalSize
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_getMaxNbSeq(params: ldmParams_t, maxChunkSize: usize) -> usize {
    if params.enableLdm == ZSTD_ps_enable {
        maxChunkSize / params.minMatchLength as usize
    } else {
        0
    }
}

/** ZSTD_ldm_getBucket() :
 *  Returns a pointer to the start of the bucket associated with hash. */
unsafe fn ZSTD_ldm_getBucket(
    ldmState: *const ldmState_t,
    hash: usize,
    bucketSizeLog: U32,
) -> *mut ldmEntry_t {
    (*ldmState).hashTable.add(hash << bucketSizeLog)
}

/** ZSTD_ldm_insertEntry() :
 *  Insert the entry with corresponding hash into the hash table */
unsafe fn ZSTD_ldm_insertEntry(
    ldmState: *mut ldmState_t,
    hash: usize,
    entry: ldmEntry_t,
    bucketSizeLog: U32,
) {
    let pOffset: *mut BYTE = (*ldmState).bucketOffsets.add(hash);
    let offset: c_uint = *pOffset as c_uint;

    *ZSTD_ldm_getBucket(ldmState, hash, bucketSizeLog).add(offset as usize) = entry;
    *pOffset = (offset.wrapping_add(1) & ((1u32 << bucketSizeLog).wrapping_sub(1))) as BYTE;
}

/** ZSTD_ldm_countBackwardsMatch() :
 *  Returns the number of bytes that match backwards before pIn and pMatch.
 *
 *  We count only bytes where pMatch >= pBase and pIn >= pAnchor. */
unsafe fn ZSTD_ldm_countBackwardsMatch(
    mut pIn: *const BYTE,
    pAnchor: *const BYTE,
    mut pMatch: *const BYTE,
    pMatchBase: *const BYTE,
) -> usize {
    let mut matchLength: usize = 0;
    while pIn > pAnchor && pMatch > pMatchBase && *pIn.offset(-1) == *pMatch.offset(-1) {
        pIn = pIn.offset(-1);
        pMatch = pMatch.offset(-1);
        matchLength += 1;
    }
    matchLength
}

/** ZSTD_ldm_countBackwardsMatch_2segments() :
 *  Returns the number of bytes that match backwards from pMatch,
 *  even with the backwards match spanning 2 different segments.
 *
 *  On reaching `pMatchBase`, start counting from mEnd */
unsafe fn ZSTD_ldm_countBackwardsMatch_2segments(
    pIn: *const BYTE,
    pAnchor: *const BYTE,
    pMatch: *const BYTE,
    pMatchBase: *const BYTE,
    pExtDictStart: *const BYTE,
    pExtDictEnd: *const BYTE,
) -> usize {
    let mut matchLength: usize = ZSTD_ldm_countBackwardsMatch(pIn, pAnchor, pMatch, pMatchBase);
    if pMatch.wrapping_sub(matchLength) != pMatchBase || pMatchBase == pExtDictStart {
        /* If backwards match is entirely in the extDict or prefix, immediately return */
        return matchLength;
    }
    matchLength += ZSTD_ldm_countBackwardsMatch(
        pIn.wrapping_sub(matchLength),
        pAnchor,
        pExtDictEnd,
        pExtDictStart,
    );
    matchLength
}

/** ZSTD_ldm_fillFastTables() :
 *
 *  Fills the relevant tables for the ZSTD_fast and ZSTD_dfast strategies.
 *  This is similar to ZSTD_loadDictionaryContent.
 *
 *  The tables for the other strategies are filled within their
 *  block compressors. */
unsafe fn ZSTD_ldm_fillFastTables(ms: *mut ZSTD_MatchState_t, end: *const c_void) -> usize {
    let iend: *const BYTE = end as *const BYTE;

    match (*ms).cParams.strategy {
        ZSTD_fast => {
            ZSTD_fillHashTable(ms, iend as *const c_void, ZSTD_dtlm_fast, ZSTD_tfp_forCCtx);
        }

        ZSTD_dfast => {
            ZSTD_fillDoubleHashTable(ms, iend as *const c_void, ZSTD_dtlm_fast, ZSTD_tfp_forCCtx);
        }

        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 | ZSTD_btlazy2 | ZSTD_btopt | ZSTD_btultra
        | ZSTD_btultra2 => {}
        _ => {}
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_fillHashTable(
    ldmState: *mut ldmState_t,
    mut ip: *const BYTE,
    iend: *const BYTE,
    params: *const ldmParams_t,
) {
    let minMatchLength: U32 = (*params).minMatchLength;
    let bucketSizeLog: U32 = (*params).bucketSizeLog;
    let hBits: U32 = (*params).hashLog.wrapping_sub(bucketSizeLog);
    let base: *const BYTE = (*ldmState).window.base;
    let istart: *const BYTE = ip;
    let mut hashState: ldmRollingHashState_t = ldmRollingHashState_t::default();
    let splits: *mut usize = (*ldmState).splitIndices.as_mut_ptr();
    let mut numSplits: c_uint;

    ZSTD_ldm_gear_init(&mut hashState, params);
    while ip < iend {
        let hashed: usize;
        let mut n: c_uint;

        numSplits = 0;
        hashed = ZSTD_ldm_gear_feed(
            &mut hashState,
            ip,
            iend.offset_from(ip) as usize,
            splits,
            &mut numSplits,
        );

        n = 0;
        while n < numSplits {
            if ip.wrapping_add(*splits.add(n as usize))
                >= istart.wrapping_add(minMatchLength as usize)
            {
                let split: *const BYTE = ip
                    .wrapping_add(*splits.add(n as usize))
                    .wrapping_sub(minMatchLength as usize);
                let xxhash: U64 =
                    ZSTD_XXH64(split as *const c_void, minMatchLength as usize, 0);
                let hash: U32 = (xxhash & (((1 as U32) << hBits).wrapping_sub(1)) as U64) as U32;
                let mut entry: ldmEntry_t = ldmEntry_t::default();

                entry.offset = split.offset_from(base) as U32;
                entry.checksum = (xxhash >> 32) as U32;
                ZSTD_ldm_insertEntry(ldmState, hash as usize, entry, (*params).bucketSizeLog);
            }
            n += 1;
        }

        ip = ip.wrapping_add(hashed);
    }
}

/** ZSTD_ldm_limitTableUpdate() :
 *
 *  Sets cctx->nextToUpdate to a position corresponding closer to anchor
 *  if it is far way
 *  (after a long match, only update tables a limited amount). */
unsafe fn ZSTD_ldm_limitTableUpdate(ms: *mut ZSTD_MatchState_t, anchor: *const BYTE) {
    let curr: U32 = anchor.offset_from((*ms).window.base) as U32;
    if curr > (*ms).nextToUpdate.wrapping_add(1024) {
        (*ms).nextToUpdate = curr.wrapping_sub(MIN(
            512u32,
            curr.wrapping_sub((*ms).nextToUpdate).wrapping_sub(1024),
        ));
    }
}

unsafe fn ZSTD_ldm_generateSequences_internal(
    ldmState: *mut ldmState_t,
    rawSeqStore: *mut RawSeqStore_t,
    params: *const ldmParams_t,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* LDM parameters */
    let extDict: c_int = ZSTD_window_hasExtDict((*ldmState).window) as c_int;
    let minMatchLength: U32 = (*params).minMatchLength;
    let entsPerBucket: U32 = 1u32 << (*params).bucketSizeLog;
    let hBits: U32 = (*params).hashLog.wrapping_sub((*params).bucketSizeLog);
    /* Prefix and extDict parameters */
    let dictLimit: U32 = (*ldmState).window.dictLimit;
    let lowestIndex: U32 = if extDict != 0 {
        (*ldmState).window.lowLimit
    } else {
        dictLimit
    };
    let base: *const BYTE = (*ldmState).window.base;
    let dictBase: *const BYTE = if extDict != 0 {
        (*ldmState).window.dictBase
    } else {
        core::ptr::null()
    };
    let dictStart: *const BYTE = if extDict != 0 {
        dictBase.wrapping_add(lowestIndex as usize)
    } else {
        core::ptr::null()
    };
    let dictEnd: *const BYTE = if extDict != 0 {
        dictBase.wrapping_add(dictLimit as usize)
    } else {
        core::ptr::null()
    };
    let lowPrefixPtr: *const BYTE = base.wrapping_add(dictLimit as usize);
    /* Input bounds */
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE);
    /* Input positions */
    let mut anchor: *const BYTE = istart;
    let mut ip: *const BYTE = istart;
    /* Rolling hash state */
    let mut hashState: ldmRollingHashState_t = ldmRollingHashState_t::default();
    /* Arrays for staged-processing */
    let splits: *mut usize = (*ldmState).splitIndices.as_mut_ptr();
    let candidates: *mut ldmMatchCandidate_t = (*ldmState).matchCandidates.as_mut_ptr();
    let mut numSplits: c_uint;

    if srcSize < minMatchLength as usize {
        return iend.offset_from(anchor) as usize;
    }

    /* Initialize the rolling hash state with the first minMatchLength bytes */
    ZSTD_ldm_gear_init(&mut hashState, params);
    ZSTD_ldm_gear_reset(&mut hashState, ip, minMatchLength as usize);
    ip = ip.wrapping_add(minMatchLength as usize);

    while ip < ilimit {
        let hashed: usize;
        let mut n: c_uint;

        numSplits = 0;
        hashed = ZSTD_ldm_gear_feed(
            &mut hashState,
            ip,
            ilimit.offset_from(ip) as usize,
            splits,
            &mut numSplits,
        );

        n = 0;
        while n < numSplits {
            let split: *const BYTE = ip
                .wrapping_add(*splits.add(n as usize))
                .wrapping_sub(minMatchLength as usize);
            let xxhash: U64 = ZSTD_XXH64(split as *const c_void, minMatchLength as usize, 0);
            let hash: U32 = (xxhash & (((1 as U32) << hBits).wrapping_sub(1)) as U64) as U32;

            (*candidates.add(n as usize)).split = split;
            (*candidates.add(n as usize)).hash = hash;
            (*candidates.add(n as usize)).checksum = (xxhash >> 32) as U32;
            (*candidates.add(n as usize)).bucket =
                ZSTD_ldm_getBucket(ldmState, hash as usize, (*params).bucketSizeLog);
            /* PREFETCH_L1 -- no functional effect */
            n += 1;
        }

        n = 0;
        while n < numSplits {
            let mut forwardMatchLength: usize = 0;
            let mut backwardMatchLength: usize = 0;
            let mut bestMatchLength: usize = 0;
            let mLength: usize;
            let offset: U32;
            let split: *const BYTE = (*candidates.add(n as usize)).split;
            let checksum: U32 = (*candidates.add(n as usize)).checksum;
            let hash: U32 = (*candidates.add(n as usize)).hash;
            let bucket: *mut ldmEntry_t = (*candidates.add(n as usize)).bucket;
            let mut cur: *const ldmEntry_t;
            let mut bestEntry: *const ldmEntry_t = core::ptr::null();
            let mut newEntry: ldmEntry_t = ldmEntry_t::default();

            newEntry.offset = split.offset_from(base) as U32;
            newEntry.checksum = checksum;

            /* If a split point would generate a sequence overlapping with
             * the previous one, we merely register it in the hash table and
             * move on */
            if split < anchor {
                ZSTD_ldm_insertEntry(ldmState, hash as usize, newEntry, (*params).bucketSizeLog);
                n += 1;
                continue;
            }

            cur = bucket;
            while cur < bucket.wrapping_add(entsPerBucket as usize) {
                let curForwardMatchLength: usize;
                let curBackwardMatchLength: usize;
                let curTotalMatchLength: usize;
                if (*cur).checksum != checksum || (*cur).offset <= lowestIndex {
                    cur = cur.wrapping_add(1);
                    continue;
                }
                if extDict != 0 {
                    let curMatchBase: *const BYTE = if (*cur).offset < dictLimit {
                        dictBase
                    } else {
                        base
                    };
                    let pMatch: *const BYTE = curMatchBase.wrapping_add((*cur).offset as usize);
                    let matchEnd: *const BYTE = if (*cur).offset < dictLimit {
                        dictEnd
                    } else {
                        iend
                    };
                    let lowMatchPtr: *const BYTE = if (*cur).offset < dictLimit {
                        dictStart
                    } else {
                        lowPrefixPtr
                    };
                    curForwardMatchLength =
                        ZSTD_count_2segments(split, pMatch, iend, matchEnd, lowPrefixPtr);
                    if curForwardMatchLength < minMatchLength as usize {
                        cur = cur.wrapping_add(1);
                        continue;
                    }
                    curBackwardMatchLength = ZSTD_ldm_countBackwardsMatch_2segments(
                        split, anchor, pMatch, lowMatchPtr, dictStart, dictEnd,
                    );
                } else {
                    /* !extDict */
                    let pMatch: *const BYTE = base.wrapping_add((*cur).offset as usize);
                    curForwardMatchLength = ZSTD_count(split, pMatch, iend);
                    if curForwardMatchLength < minMatchLength as usize {
                        cur = cur.wrapping_add(1);
                        continue;
                    }
                    curBackwardMatchLength =
                        ZSTD_ldm_countBackwardsMatch(split, anchor, pMatch, lowPrefixPtr);
                }
                curTotalMatchLength = curForwardMatchLength + curBackwardMatchLength;

                if curTotalMatchLength > bestMatchLength {
                    bestMatchLength = curTotalMatchLength;
                    forwardMatchLength = curForwardMatchLength;
                    backwardMatchLength = curBackwardMatchLength;
                    bestEntry = cur;
                }
                cur = cur.wrapping_add(1);
            }

            /* No match found -- insert an entry into the hash table
             * and process the next candidate match */
            if bestEntry.is_null() {
                ZSTD_ldm_insertEntry(ldmState, hash as usize, newEntry, (*params).bucketSizeLog);
                n += 1;
                continue;
            }

            /* Match found */
            offset = (split.offset_from(base) as U32).wrapping_sub((*bestEntry).offset);
            mLength = forwardMatchLength + backwardMatchLength;
            {
                let seq: *mut rawSeq = (*rawSeqStore).seq.wrapping_add((*rawSeqStore).size);

                /* Out of sequence storage */
                if (*rawSeqStore).size == (*rawSeqStore).capacity {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                }
                (*seq).litLength = split
                    .wrapping_sub(backwardMatchLength)
                    .offset_from(anchor) as U32;
                (*seq).matchLength = mLength as U32;
                (*seq).offset = offset;
                (*rawSeqStore).size += 1;
            }

            /* Insert the current entry into the hash table --- it must be
             * done after the previous block to avoid clobbering bestEntry */
            ZSTD_ldm_insertEntry(ldmState, hash as usize, newEntry, (*params).bucketSizeLog);

            anchor = split.wrapping_add(forwardMatchLength);

            /* If we find a match that ends after the data that we've hashed
             * then we have a repeating, overlapping, pattern. E.g. all zeros.
             * If one repetition of the pattern matches our `stopMask` then all
             * repetitions will. We don't need to insert them all into out table,
             * only the first one. So skip over overlapping matches.
             * This is a major speed boost (20x) for compressing a single byte
             * repeated, when that byte ends up in the table.
             */
            if anchor > ip.wrapping_add(hashed) {
                ZSTD_ldm_gear_reset(
                    &mut hashState,
                    anchor.wrapping_sub(minMatchLength as usize),
                    minMatchLength as usize,
                );
                /* Continue the outer loop at anchor (ip + hashed == anchor). */
                ip = anchor.wrapping_sub(hashed);
                break;
            }
            n += 1;
        }

        ip = ip.wrapping_add(hashed);
    }

    iend.offset_from(anchor) as usize
}

/* ZSTD_ldm_reduceTable() :
 *  reduce table indexes by `reducerValue` */
unsafe fn ZSTD_ldm_reduceTable(table: *mut ldmEntry_t, size: U32, reducerValue: U32) {
    let mut u: U32 = 0;
    while u < size {
        if (*table.add(u as usize)).offset < reducerValue {
            (*table.add(u as usize)).offset = 0;
        } else {
            (*table.add(u as usize)).offset =
                (*table.add(u as usize)).offset.wrapping_sub(reducerValue);
        }
        u += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_generateSequences(
    ldmState: *mut ldmState_t,
    sequences: *mut RawSeqStore_t,
    params: *const ldmParams_t,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let maxDist: U32 = 1u32 << (*params).windowLog;
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let kMaxChunkSize: usize = 1 << 20;
    let nbChunks: usize =
        (srcSize / kMaxChunkSize) + ((srcSize % kMaxChunkSize != 0) as usize);
    let mut chunk: usize;
    let mut leftoverSize: usize = 0;

    /* The input could be very large (in zstdmt), so it must be broken up into
     * chunks to enforce the maximum distance and handle overflow correction.
     */
    chunk = 0;
    while chunk < nbChunks && (*sequences).size < (*sequences).capacity {
        let chunkStart: *const BYTE = istart.wrapping_add(chunk * kMaxChunkSize);
        let remaining: usize = iend.offset_from(chunkStart) as usize;
        let chunkEnd: *const BYTE = if remaining < kMaxChunkSize {
            iend
        } else {
            chunkStart.wrapping_add(kMaxChunkSize)
        };
        let chunkSize: usize = chunkEnd.offset_from(chunkStart) as usize;
        let newLeftoverSize: usize;
        let prevSize: usize = (*sequences).size;

        /* 1. Perform overflow correction if necessary. */
        if ZSTD_window_needOverflowCorrection(
            (*ldmState).window,
            0,
            maxDist,
            (*ldmState).loadedDictEnd,
            chunkStart as *const c_void,
            chunkEnd as *const c_void,
        ) != 0
        {
            let ldmHSize: U32 = 1u32 << (*params).hashLog;
            let correction: U32 = ZSTD_window_correctOverflow(
                &mut (*ldmState).window,
                /* cycleLog */ 0,
                maxDist,
                chunkStart as *const c_void,
            );
            ZSTD_ldm_reduceTable((*ldmState).hashTable, ldmHSize, correction);
            /* invalidate dictionaries on overflow correction */
            (*ldmState).loadedDictEnd = 0;
        }
        /* 2. We enforce the maximum offset allowed.
         *
         * kMaxChunkSize should be small enough that we don't lose too much of
         * the window through early invalidation.
         *
         * NOTE: Because of dictionaries + sequence splitting we MUST make sure
         * that any offset used is valid at the END of the sequence, since it may
         * be split into two sequences. This condition holds when using
         * ZSTD_window_enforceMaxDist(), but if we move to checking offsets
         * against maxDist directly, we'll have to carefully handle that case.
         */
        ZSTD_window_enforceMaxDist(
            &mut (*ldmState).window,
            chunkEnd as *const c_void,
            maxDist,
            &mut (*ldmState).loadedDictEnd,
            core::ptr::null_mut(),
        );
        /* 3. Generate the sequences for the chunk, and get newLeftoverSize. */
        newLeftoverSize = ZSTD_ldm_generateSequences_internal(
            ldmState,
            sequences,
            params,
            chunkStart as *const c_void,
            chunkSize,
        );
        if ERR_isError(newLeftoverSize) != 0 {
            return newLeftoverSize;
        }
        /* 4. We add the leftover literals from previous iterations to the first
         *    newly generated sequence, or add the `newLeftoverSize` if none are
         *    generated.
         */
        /* Prepend the leftover literals from the last call */
        if prevSize < (*sequences).size {
            (*(*sequences).seq.add(prevSize)).litLength =
                (*(*sequences).seq.add(prevSize))
                    .litLength
                    .wrapping_add(leftoverSize as U32);
            leftoverSize = newLeftoverSize;
        } else {
            leftoverSize += chunkSize;
        }
        chunk += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_skipSequences(
    rawSeqStore: *mut RawSeqStore_t,
    mut srcSize: usize,
    minMatch: U32,
) {
    while srcSize > 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let seq: *mut rawSeq = (*rawSeqStore).seq.add((*rawSeqStore).pos);
        if srcSize <= (*seq).litLength as usize {
            /* Skip past srcSize literals */
            (*seq).litLength = (*seq).litLength.wrapping_sub(srcSize as U32);
            return;
        }
        srcSize -= (*seq).litLength as usize;
        (*seq).litLength = 0;
        if srcSize < (*seq).matchLength as usize {
            /* Skip past the first srcSize of the match */
            (*seq).matchLength = (*seq).matchLength.wrapping_sub(srcSize as U32);
            if (*seq).matchLength < minMatch {
                /* The match is too short, omit it */
                if (*rawSeqStore).pos + 1 < (*rawSeqStore).size {
                    (*seq.add(1)).litLength =
                        (*seq.add(1)).litLength.wrapping_add((*seq.add(0)).matchLength);
                }
                (*rawSeqStore).pos += 1;
            }
            return;
        }
        srcSize -= (*seq).matchLength as usize;
        (*seq).matchLength = 0;
        (*rawSeqStore).pos += 1;
    }
}

/**
 * If the sequence length is longer than remaining then the sequence is split
 * between this block and the next.
 *
 * Returns the current sequence to handle, or if the rest of the block should
 * be literals, it returns a sequence with offset == 0.
 */
unsafe fn maybeSplitSequence(
    rawSeqStore: *mut RawSeqStore_t,
    remaining: U32,
    minMatch: U32,
) -> rawSeq {
    let mut sequence: rawSeq = *(*rawSeqStore).seq.add((*rawSeqStore).pos);
    /* Likely: No partial sequence */
    if remaining >= sequence.litLength.wrapping_add(sequence.matchLength) {
        (*rawSeqStore).pos += 1;
        return sequence;
    }
    /* Cut the sequence short (offset == 0 ==> rest is literals). */
    if remaining <= sequence.litLength {
        sequence.offset = 0;
    } else if remaining < sequence.litLength.wrapping_add(sequence.matchLength) {
        sequence.matchLength = remaining.wrapping_sub(sequence.litLength);
        if sequence.matchLength < minMatch {
            sequence.offset = 0;
        }
    }
    /* Skip past `remaining` bytes for the future sequences. */
    ZSTD_ldm_skipSequences(rawSeqStore, remaining as usize, minMatch);
    sequence
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_skipRawSeqStoreBytes(
    rawSeqStore: *mut RawSeqStore_t,
    nbBytes: usize,
) {
    let mut currPos: U32 = ((*rawSeqStore).posInSequence.wrapping_add(nbBytes)) as U32;
    while currPos != 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let currSeq: rawSeq = *(*rawSeqStore).seq.add((*rawSeqStore).pos);
        if currPos >= currSeq.litLength.wrapping_add(currSeq.matchLength) {
            currPos = currPos.wrapping_sub(currSeq.litLength.wrapping_add(currSeq.matchLength));
            (*rawSeqStore).pos += 1;
        } else {
            (*rawSeqStore).posInSequence = currPos as usize;
            break;
        }
    }
    if currPos == 0 || (*rawSeqStore).pos == (*rawSeqStore).size {
        (*rawSeqStore).posInSequence = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_blockCompress(
    rawSeqStore: *mut RawSeqStore_t,
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let minMatch: c_uint = (*cParams).minMatch;
    let blockCompressor: ZSTD_BlockCompressor_f = ZSTD_selectBlockCompressor(
        (*cParams).strategy,
        useRowMatchFinder,
        ZSTD_matchState_dictMode(ms),
    );
    let blockCompressor = blockCompressor.unwrap_unchecked();
    /* Input bounds */
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    /* Input positions */
    let mut ip: *const BYTE = istart;

    /* If using opt parser, use LDMs only as candidates rather than always accepting them */
    if (*cParams).strategy >= ZSTD_btopt {
        let lastLLSize: usize;
        (*ms).ldmSeqStore = rawSeqStore as *const RawSeqStore_t;
        lastLLSize = blockCompressor(ms, seqStore, rep, src, srcSize);
        ZSTD_ldm_skipRawSeqStoreBytes(rawSeqStore, srcSize);
        return lastLLSize;
    }

    /* Loop through each sequence and apply the block compressor to the literals */
    while (*rawSeqStore).pos < (*rawSeqStore).size && ip < iend {
        /* maybeSplitSequence updates rawSeqStore->pos */
        let sequence: rawSeq =
            maybeSplitSequence(rawSeqStore, iend.offset_from(ip) as U32, minMatch);
        /* End signal */
        if sequence.offset == 0 {
            break;
        }

        /* Fill tables for block compressor */
        ZSTD_ldm_limitTableUpdate(ms, ip);
        ZSTD_ldm_fillFastTables(ms, ip as *const c_void);
        /* Run the block compressor */
        {
            let mut i: c_int;
            let newLitLength: usize = blockCompressor(
                ms,
                seqStore,
                rep,
                ip as *const c_void,
                sequence.litLength as usize,
            );
            ip = ip.wrapping_add(sequence.litLength as usize);
            /* Update the repcodes */
            i = ZSTD_REP_NUM as c_int - 1;
            while i > 0 {
                *rep.add(i as usize) = *rep.add((i - 1) as usize);
                i -= 1;
            }
            *rep.add(0) = sequence.offset;
            /* Store the sequence */
            ZSTD_storeSeq(
                seqStore,
                newLitLength,
                ip.wrapping_sub(newLitLength),
                iend,
                OFFSET_TO_OFFBASE(sequence.offset),
                sequence.matchLength as usize,
            );
            ip = ip.wrapping_add(sequence.matchLength as usize);
        }
    }
    /* Fill the tables for the block compressor */
    ZSTD_ldm_limitTableUpdate(ms, ip);
    ZSTD_ldm_fillFastTables(ms, ip as *const c_void);
    /* Compress the last literals */
    blockCompressor(
        ms,
        seqStore,
        rep,
        ip as *const c_void,
        iend.offset_from(ip) as usize,
    )
}
