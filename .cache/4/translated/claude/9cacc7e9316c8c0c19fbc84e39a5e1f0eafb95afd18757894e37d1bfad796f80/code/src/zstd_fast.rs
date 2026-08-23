//! Translation of compress/zstd_fast.c (+ compress/zstd_fast.h)
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

use core::ffi::{c_int, c_uint, c_void};

use crate::compiler::{PREFETCH_AREA, PREFETCH_L1};
use crate::mem::*;
use crate::zstd_h::ZSTD_compressionParameters;

use crate::zstd_compress_internal::{
    kSearchStrength, SeqStore_t, ZSTD_MatchState_t, ZSTD_comparePackedTags, ZSTD_count,
    ZSTD_count_2segments, ZSTD_dictTableLoadMethod_e, ZSTD_dtlm_fast,
    ZSTD_getLowestMatchIndex, ZSTD_getLowestPrefixIndex, ZSTD_hashPtr, ZSTD_index_overlap_check,
    ZSTD_selectAddr, ZSTD_storeSeq, ZSTD_tableFillPurpose_e, ZSTD_tfp_forCDict,
    ZSTD_writeTaggedIndex, HASH_READ_SIZE, OFFSET_TO_OFFBASE, REPCODE1_TO_OFFBASE,
    ZSTD_SHORT_CACHE_TAG_BITS,
};

/* ===  ZSTD_fillHashTableForCDict  === */

pub unsafe fn ZSTD_fillHashTableForCDict(
    ms: *mut ZSTD_MatchState_t,
    end: *const c_void,
    dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hBits: U32 = (*cParams).hashLog.wrapping_add(ZSTD_SHORT_CACHE_TAG_BITS);
    let mls: U32 = (*cParams).minMatch;
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.wrapping_add((*ms).nextToUpdate as usize);
    let iend: *const BYTE = (end as *const BYTE).wrapping_sub(HASH_READ_SIZE);
    let fastHashFillStep: U32 = 3;

    /* Always insert every fastHashFillStep position into the hash table.
     * Insert the other positions if their hash entry is empty.
     */
    while ip.wrapping_add(fastHashFillStep as usize) < iend.wrapping_add(2) {
        let curr: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
        {
            let hashAndTag: usize = ZSTD_hashPtr(ip as *const c_void, hBits, mls);
            ZSTD_writeTaggedIndex(hashTable, hashAndTag, curr);
        }

        if dtlm == ZSTD_dtlm_fast {
            ip = ip.wrapping_add(fastHashFillStep as usize);
            continue;
        }
        /* Only load extra positions for ZSTD_dtlm_full */
        {
            let mut p: U32 = 1;
            while p < fastHashFillStep {
                let hashAndTag: usize =
                    ZSTD_hashPtr(ip.wrapping_add(p as usize) as *const c_void, hBits, mls);
                if *hashTable.add(hashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS) == 0 {
                    /* not yet filled */
                    ZSTD_writeTaggedIndex(hashTable, hashAndTag, curr.wrapping_add(p));
                }
                p += 1;
            }
        }
        ip = ip.wrapping_add(fastHashFillStep as usize);
    }
}

/* ===  ZSTD_fillHashTableForCCtx  === */

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
    let iend: *const BYTE = (end as *const BYTE).wrapping_sub(HASH_READ_SIZE);
    let fastHashFillStep: U32 = 3;

    /* Always insert every fastHashFillStep position into the hash table.
     * Insert the other positions if their hash entry is empty.
     */
    while ip.wrapping_add(fastHashFillStep as usize) < iend.wrapping_add(2) {
        let curr: U32 = ((ip as usize).wrapping_sub(base as usize)) as U32;
        let hash0: usize = ZSTD_hashPtr(ip as *const c_void, hBits, mls);
        *hashTable.add(hash0) = curr;
        if dtlm == ZSTD_dtlm_fast {
            ip = ip.wrapping_add(fastHashFillStep as usize);
            continue;
        }
        /* Only load extra positions for ZSTD_dtlm_full */
        {
            let mut p: U32 = 1;
            while p < fastHashFillStep {
                let hash: usize =
                    ZSTD_hashPtr(ip.wrapping_add(p as usize) as *const c_void, hBits, mls);
                if *hashTable.add(hash) == 0 {
                    /* not yet filled */
                    *hashTable.add(hash) = curr.wrapping_add(p);
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

/* ===  match-found predicates  === */

pub type ZSTD_match4Found =
    unsafe fn(currentPtr: *const BYTE, matchAddress: *const BYTE, matchIdx: U32, idxLowLimit: U32) -> c_int;

/* Array of ~random data, should have low probability of matching data.
 * Load from here if the index is invalid.
 * Used to avoid unpredictable branches. */
static dummy: [BYTE; 4] = [0x12, 0x34, 0x56, 0x78];

pub unsafe fn ZSTD_match4Found_cmov(
    currentPtr: *const BYTE,
    matchAddress: *const BYTE,
    matchIdx: U32,
    idxLowLimit: U32,
) -> c_int {
    /* currentIdx >= lowLimit is a (somewhat) unpredictable branch.
     * However expression below compiles into conditional move.
     */
    let mvalAddr: *const BYTE =
        ZSTD_selectAddr(matchIdx, idxLowLimit, matchAddress, dummy.as_ptr());
    /* Note: this used to be written as : return test1 && test2;
     * Unfortunately, once inlined, these tests become branches,
     * in which case it becomes critical that they are executed in the right order (test1 then test2).
     * So we have to write these tests in a specific manner to ensure their ordering.
     */
    if MEM_read32(currentPtr) != MEM_read32(mvalAddr) {
        return 0;
    }
    /* force ordering of these tests, which matters once the function is inlined, as they become branches */
    core::arch::asm!("", options(nomem, nostack, preserves_flags));
    (matchIdx >= idxLowLimit) as c_int
}

pub unsafe fn ZSTD_match4Found_branch(
    currentPtr: *const BYTE,
    matchAddress: *const BYTE,
    matchIdx: U32,
    idxLowLimit: U32,
) -> c_int {
    /* using a branch instead of a cmov,
     * because it's faster in scenarios where matchIdx >= idxLowLimit is generally true,
     * aka almost all candidates are within range */
    let mval: U32;
    if matchIdx >= idxLowLimit {
        mval = MEM_read32(matchAddress);
    } else {
        mval = MEM_read32(currentPtr) ^ 1; /* guaranteed to not match. */
    }

    (MEM_read32(currentPtr) == mval) as c_int
}

/**
 * If you squint hard enough (and ignore repcodes), the search operation at any
 * given position is broken into 4 stages:
 *
 * 1. Hash   (map position to hash value via input read)
 * 2. Lookup (map hash val to index via hashtable read)
 * 3. Load   (map index to value at that position via input read)
 * 4. Compare
 *
 * Each of these steps involves a memory read at an address which is computed
 * from the previous step. This means these steps must be sequenced and their
 * latencies are cumulative.
 *
 * Rather than do 1->2->3->4 sequentially for a single position before moving
 * onto the next, this implementation interleaves these operations across the
 * next few positions.
 */
pub unsafe fn ZSTD_compressBlock_fast_noDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32,
    useCmov: c_int,
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog;
    /* min 2 */
    let stepSize: usize = ((*cParams).targetLength as usize)
        .wrapping_add(((*cParams).targetLength == 0) as usize)
        .wrapping_add(1);
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let endIndex: U32 = (((istart as usize).wrapping_sub(base as usize)).wrapping_add(srcSize)) as U32;
    let prefixStartIndex: U32 = ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog);
    let prefixStart: *const BYTE = base.wrapping_add(prefixStartIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE);

    let mut anchor: *const BYTE = istart;
    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE;
    let mut ip2: *const BYTE;
    let mut ip3: *const BYTE;
    let mut current0: U32 = 0;

    let mut rep_offset1: U32 = *rep.add(0);
    let mut rep_offset2: U32 = *rep.add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let mut hash0: usize = 0; /* hash for ip0 */
    let mut hash1: usize = 0; /* hash for ip1 */
    let mut matchIdx: U32 = 0; /* match idx for ip0 */

    let mut offcode: U32 = 0;
    let mut match0: *const BYTE = core::ptr::null();
    let mut mLength: usize = 0;

    /* ip0 and ip1 are always adjacent. The targetLength skipping and
     * uncompressibility acceleration is applied to every other position,
     * matching the behavior of #1562. step therefore represents the gap
     * between pairs of positions, from ip0 to ip2 or ip1 to ip3. */
    let mut step: usize;
    let mut nextStep: *const BYTE;
    let kStepIncr: usize = 1usize << (kSearchStrength - 1);
    let matchFound: ZSTD_match4Found = if useCmov != 0 {
        ZSTD_match4Found_cmov
    } else {
        ZSTD_match4Found_branch
    };

    ip0 = ip0.wrapping_add((ip0 == prefixStart) as usize);
    {
        let curr: U32 = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
        let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, curr, (*cParams).windowLog);
        let maxRep: U32 = curr.wrapping_sub(windowLow);
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
    /* _start: Requires: ip0 */
    'start: loop {
        step = stepSize;
        nextStep = ip0.wrapping_add(kStepIncr);

        /* calculate positions, ip0 - anchor == 0, so we skip step calc */
        ip1 = ip0.wrapping_add(1);
        ip2 = ip0.wrapping_add(step);
        ip3 = ip2.wrapping_add(1);

        /* 0 -> _cleanup, 1 -> _offset, 2 -> _match */
        let mut jump: u32 = 0;

        if ip3 < ilimit {
            hash0 = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);
            hash1 = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);

            matchIdx = *hashTable.add(hash0);

            loop {
                /* load repcode match for ip[2]*/
                let rval: U32 = MEM_read32(ip2.wrapping_sub(rep_offset1 as usize));

                /* write back hash table entry */
                current0 = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
                *hashTable.add(hash0) = current0;

                /* check repcode at ip[2] */
                if (((MEM_read32(ip2) == rval) as u32) & ((rep_offset1 > 0) as u32)) != 0 {
                    ip0 = ip2;
                    match0 = ip0.wrapping_sub(rep_offset1 as usize);
                    mLength = (*ip0.offset(-1) == *match0.offset(-1)) as usize;
                    ip0 = ip0.wrapping_sub(mLength);
                    match0 = match0.wrapping_sub(mLength);
                    offcode = REPCODE1_TO_OFFBASE;
                    mLength = mLength.wrapping_add(4);

                    /* Write next hash table entry: it's already calculated.
                     * This write is known to be safe because ip1 is before the
                     * repcode (ip2). */
                    *hashTable.add(hash1) = ((ip1 as usize).wrapping_sub(base as usize)) as U32;

                    jump = 2; /* goto _match */
                    break;
                }

                if matchFound(ip0, base.wrapping_add(matchIdx as usize), matchIdx, prefixStartIndex)
                    != 0
                {
                    /* Write next hash table entry (it's already calculated).
                     * This write is known to be safe because the ip1 == ip0 + 1,
                     * so searching will resume after ip1 */
                    *hashTable.add(hash1) = ((ip1 as usize).wrapping_sub(base as usize)) as U32;

                    jump = 1; /* goto _offset */
                    break;
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
                current0 = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
                *hashTable.add(hash0) = current0;

                if matchFound(ip0, base.wrapping_add(matchIdx as usize), matchIdx, prefixStartIndex)
                    != 0
                {
                    /* Write next hash table entry, since it's already calculated */
                    if step <= 4 {
                        /* Avoid writing an index if it's >= position where search will resume.
                         * The minimum possible match has length 4, so search can resume at ip0 + 4.
                         */
                        *hashTable.add(hash1) = ((ip1 as usize).wrapping_sub(base as usize)) as U32;
                    }
                    jump = 1; /* goto _offset */
                    break;
                }

                /* lookup ip[1] */
                matchIdx = *hashTable.add(hash1);

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
                    PREFETCH_L1(ip1.wrapping_add(64));
                    PREFETCH_L1(ip1.wrapping_add(128));
                    nextStep = nextStep.wrapping_add(kStepIncr);
                }

                if !(ip3 < ilimit) {
                    break;
                }
            }
        }

        if jump == 0 {
            /* _cleanup:
             * Note that there are probably still a couple positions one could search.
             * However, it seems to be a meaningful performance hit to try to search
             * them. So let's not.
             *
             * When the repcodes are outside of the prefix, we set them to zero before the loop.
             * When the offsets are still zero, we need to restore them after the block to have a
             * correct repcode history. If only one offset was invalid, it is easy. The tricky case
             * is when both offsets were invalid. We need to figure out which offset to refill with.
             *     - If both offsets are zero they are in the same order.
             *     - If both offsets are non-zero, we won't restore the offsets from `offsetSaved[12]`.
             *     - If only one is zero, we need to decide which offset to restore.
             *         - If rep_offset1 is non-zero, then rep_offset2 must be offsetSaved1.
             *         - It is impossible for rep_offset2 to be non-zero.
             *
             * So if rep_offset1 started invalid (offsetSaved1 != 0) and became valid
             * (rep_offset1 != 0), then set rep[0] = rep_offset1 and rep[1] = offsetSaved1.
             */
            offsetSaved2 = if (offsetSaved1 != 0) && (rep_offset1 != 0) {
                offsetSaved1
            } else {
                offsetSaved2
            };

            /* save reps for next block */
            *rep.add(0) = if rep_offset1 != 0 {
                rep_offset1
            } else {
                offsetSaved1
            };
            *rep.add(1) = if rep_offset2 != 0 {
                rep_offset2
            } else {
                offsetSaved2
            };

            /* Return the last literals size */
            return (iend as usize).wrapping_sub(anchor as usize);
        }

        if jump == 1 {
            /* _offset: Requires: ip0, idx */

            /* Compute the offset code. */
            match0 = base.wrapping_add(matchIdx as usize);
            rep_offset2 = rep_offset1;
            rep_offset1 = ((ip0 as usize).wrapping_sub(match0 as usize)) as U32;
            offcode = OFFSET_TO_OFFBASE(rep_offset1);
            mLength = 4;

            /* Count the backwards match length. */
            while ((((ip0 > anchor) as u32) & ((match0 > prefixStart) as u32)) != 0)
                && (*ip0.offset(-1) == *match0.offset(-1))
            {
                ip0 = ip0.wrapping_sub(1);
                match0 = match0.wrapping_sub(1);
                mLength += 1;
            }
        }

        /* _match: Requires: ip0, match0, offcode */

        /* Count the forward length. */
        mLength = mLength.wrapping_add(ZSTD_count(
            ip0.wrapping_add(mLength),
            match0.wrapping_add(mLength),
            iend,
        ));

        ZSTD_storeSeq(
            seqStore,
            (ip0 as usize).wrapping_sub(anchor as usize),
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
            /* here because current+2 could be > iend-8 */
            *hashTable.add(ZSTD_hashPtr(
                base.wrapping_add(current0 as usize).wrapping_add(2) as *const c_void,
                hlog,
                mls,
            )) = current0.wrapping_add(2);
            *hashTable.add(ZSTD_hashPtr(
                ip0.wrapping_sub(2) as *const c_void,
                hlog,
                mls,
            )) = ((ip0 as usize).wrapping_sub(2).wrapping_sub(base as usize)) as U32;

            if rep_offset2 > 0 {
                /* rep_offset2==0 means rep_offset2 is invalidated */
                while (ip0 <= ilimit)
                    && (MEM_read32(ip0) == MEM_read32(ip0.wrapping_sub(rep_offset2 as usize)))
                {
                    /* store sequence */
                    let rLength: usize = ZSTD_count(
                        ip0.wrapping_add(4),
                        ip0.wrapping_add(4).wrapping_sub(rep_offset2 as usize),
                        iend,
                    )
                    .wrapping_add(4);
                    {
                        /* swap rep_offset2 <=> rep_offset1 */
                        let tmpOff: U32 = rep_offset2;
                        rep_offset2 = rep_offset1;
                        rep_offset1 = tmpOff;
                    }
                    *hashTable.add(ZSTD_hashPtr(ip0 as *const c_void, hlog, mls)) =
                        ((ip0 as usize).wrapping_sub(base as usize)) as U32;
                    ip0 = ip0.wrapping_add(rLength);
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, rLength);
                    anchor = ip0;
                    continue; /* faster when present (confirmed on gcc-8) ... (?) */
                }
            }
        }

        continue 'start; /* goto _start */
    }
}

/* ===  ZSTD_GEN_FAST_FN(noDict, mml, cmov)  === */

pub unsafe fn ZSTD_compressBlock_fast_noDict_4_1(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 4, 1)
}

pub unsafe fn ZSTD_compressBlock_fast_noDict_5_1(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 5, 1)
}

pub unsafe fn ZSTD_compressBlock_fast_noDict_6_1(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 6, 1)
}

pub unsafe fn ZSTD_compressBlock_fast_noDict_7_1(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 7, 1)
}

pub unsafe fn ZSTD_compressBlock_fast_noDict_4_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 4, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_noDict_5_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 5, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_noDict_6_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 6, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_noDict_7_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_noDict_generic(ms, seqStore, rep, src, srcSize, 7, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mml: U32 = (*ms).cParams.minMatch;
    /* use cmov when "candidate in range" branch is likely unpredictable */
    let useCmov: c_int = ((*ms).cParams.windowLog < 19) as c_int;
    if useCmov != 0 {
        match mml {
            5 => ZSTD_compressBlock_fast_noDict_5_1(ms, seqStore, rep, src, srcSize),
            6 => ZSTD_compressBlock_fast_noDict_6_1(ms, seqStore, rep, src, srcSize),
            7 => ZSTD_compressBlock_fast_noDict_7_1(ms, seqStore, rep, src, srcSize),
            /* default: includes case 3, and case 4 */
            _ => ZSTD_compressBlock_fast_noDict_4_1(ms, seqStore, rep, src, srcSize),
        }
    } else {
        /* use a branch instead */
        match mml {
            5 => ZSTD_compressBlock_fast_noDict_5_0(ms, seqStore, rep, src, srcSize),
            6 => ZSTD_compressBlock_fast_noDict_6_0(ms, seqStore, rep, src, srcSize),
            7 => ZSTD_compressBlock_fast_noDict_7_0(ms, seqStore, rep, src, srcSize),
            /* default: includes case 3, and case 4 */
            _ => ZSTD_compressBlock_fast_noDict_4_0(ms, seqStore, rep, src, srcSize),
        }
    }
}

/* ===  dictMatchState  === */

pub unsafe fn ZSTD_compressBlock_fast_dictMatchState_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32,
    hasStep: U32,
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog;
    /* support stepSize of 0 */
    let stepSize: U32 = (*cParams)
        .targetLength
        .wrapping_add(((*cParams).targetLength == 0) as c_uint);
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE = ip0.wrapping_add(stepSize as usize); /* we assert below that stepSize >= 1 */
    let mut anchor: *const BYTE = istart;
    let prefixStartIndex: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.wrapping_add(prefixStartIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);

    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictCParams: *const ZSTD_compressionParameters = &(*dms).cParams;
    let dictHashTable: *const U32 = (*dms).hashTable;
    let dictStartIndex: U32 = (*dms).window.dictLimit;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictStart: *const BYTE = dictBase.wrapping_add(dictStartIndex as usize);
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictIndexDelta: U32 =
        prefixStartIndex.wrapping_sub(((dictEnd as usize).wrapping_sub(dictBase as usize)) as U32);
    let dictAndPrefixLength: U32 = ((istart as isize)
        .wrapping_sub(prefixStart as isize)
        .wrapping_add((dictEnd as isize).wrapping_sub(dictStart as isize)))
        as U32;
    let dictHBits: U32 = (*dictCParams).hashLog.wrapping_add(ZSTD_SHORT_CACHE_TAG_BITS);

    /* if a dictionary is still attached, it necessarily means that
     * it is within window size. So we just check it. */
    let maxDistance: U32 = 1u32 << (*cParams).windowLog;
    let endIndex: U32 = (((istart as usize).wrapping_sub(base as usize)).wrapping_add(srcSize)) as U32;
    let _ = maxDistance;
    let _ = endIndex;

    let _ = hasStep; /* not currently specialized on whether it's accelerated */

    if (*ms).prefetchCDictTables != 0 {
        let hashTableBytes: usize =
            ((1usize) << (*dictCParams).hashLog) * core::mem::size_of::<U32>();
        PREFETCH_AREA(dictHashTable as *const u8, hashTableBytes);
    }

    /* init */
    ip0 = ip0.wrapping_add((dictAndPrefixLength == 0) as usize);
    /* dictMatchState repCode checks don't currently handle repCode == 0
     * disabling. */

    /* Outer search loop */
    'cleanup: {
        while ip1 <= ilimit {
            /* repcode check at (ip0 + 1) is safe because ip0 < ip1 */
            let mut mLength: usize = 0;
            let mut hash0: usize = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);

            let dictHashAndTag0: usize = ZSTD_hashPtr(ip0 as *const c_void, dictHBits, mls);
            let mut dictMatchIndexAndTag: U32 =
                *dictHashTable.add(dictHashAndTag0 >> ZSTD_SHORT_CACHE_TAG_BITS);
            let mut dictTagsMatch: c_int =
                ZSTD_comparePackedTags(dictMatchIndexAndTag as usize, dictHashAndTag0);

            let mut matchIndex: U32 = *hashTable.add(hash0);
            let mut curr: U32 = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
            let mut step: usize = stepSize as usize;
            let kStepIncr: usize = 1usize << kSearchStrength;
            let mut nextStep: *const BYTE = ip0.wrapping_add(kStepIncr);

            /* Inner search loop */
            loop {
                let mut r#match: *const BYTE = base.wrapping_add(matchIndex as usize);
                let repIndex: U32 = curr.wrapping_add(1).wrapping_sub(offset_1);
                let repMatch: *const BYTE = if repIndex < prefixStartIndex {
                    dictBase.wrapping_add(repIndex.wrapping_sub(dictIndexDelta) as usize)
                } else {
                    base.wrapping_add(repIndex as usize)
                };
                let hash1: usize = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);
                let dictHashAndTag1: usize = ZSTD_hashPtr(ip1 as *const c_void, dictHBits, mls);
                *hashTable.add(hash0) = curr; /* update hash table */

                if (ZSTD_index_overlap_check(prefixStartIndex, repIndex) != 0)
                    && (MEM_read32(repMatch) == MEM_read32(ip0.wrapping_add(1)))
                {
                    let repMatchEnd: *const BYTE = if repIndex < prefixStartIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    mLength = ZSTD_count_2segments(
                        ip0.wrapping_add(1).wrapping_add(4),
                        repMatch.wrapping_add(4),
                        iend,
                        repMatchEnd,
                        prefixStart,
                    )
                    .wrapping_add(4);
                    ip0 = ip0.wrapping_add(1);
                    ZSTD_storeSeq(
                        seqStore,
                        (ip0 as usize).wrapping_sub(anchor as usize),
                        anchor,
                        iend,
                        REPCODE1_TO_OFFBASE,
                        mLength,
                    );
                    break;
                }

                if dictTagsMatch != 0 {
                    /* Found a possible dict match */
                    let dictMatchIndex: U32 = dictMatchIndexAndTag >> ZSTD_SHORT_CACHE_TAG_BITS;
                    let mut dictMatch: *const BYTE = dictBase.wrapping_add(dictMatchIndex as usize);
                    if dictMatchIndex > dictStartIndex
                        && MEM_read32(dictMatch) == MEM_read32(ip0)
                    {
                        /* To replicate extDict parse behavior, we only use dict matches when the
                         * normal matchIndex is invalid */
                        if matchIndex <= prefixStartIndex {
                            let offset: U32 = curr
                                .wrapping_sub(dictMatchIndex)
                                .wrapping_sub(dictIndexDelta);
                            mLength = ZSTD_count_2segments(
                                ip0.wrapping_add(4),
                                dictMatch.wrapping_add(4),
                                iend,
                                dictEnd,
                                prefixStart,
                            )
                            .wrapping_add(4);
                            while ((((ip0 > anchor) as u32) & ((dictMatch > dictStart) as u32)) != 0)
                                && (*ip0.offset(-1) == *dictMatch.offset(-1))
                            {
                                ip0 = ip0.wrapping_sub(1);
                                dictMatch = dictMatch.wrapping_sub(1);
                                mLength += 1;
                            } /* catch up */
                            offset_2 = offset_1;
                            offset_1 = offset;
                            ZSTD_storeSeq(
                                seqStore,
                                (ip0 as usize).wrapping_sub(anchor as usize),
                                anchor,
                                iend,
                                OFFSET_TO_OFFBASE(offset),
                                mLength,
                            );
                            break;
                        }
                    }
                }

                if ZSTD_match4Found_cmov(ip0, r#match, matchIndex, prefixStartIndex) != 0 {
                    /* found a regular match of size >= 4 */
                    let offset: U32 = ((ip0 as usize).wrapping_sub(r#match as usize)) as U32;
                    mLength = ZSTD_count(ip0.wrapping_add(4), r#match.wrapping_add(4), iend)
                        .wrapping_add(4);
                    while ((((ip0 > anchor) as u32) & ((r#match > prefixStart) as u32)) != 0)
                        && (*ip0.offset(-1) == *r#match.offset(-1))
                    {
                        ip0 = ip0.wrapping_sub(1);
                        r#match = r#match.wrapping_sub(1);
                        mLength += 1;
                    } /* catch up */
                    offset_2 = offset_1;
                    offset_1 = offset;
                    ZSTD_storeSeq(
                        seqStore,
                        (ip0 as usize).wrapping_sub(anchor as usize),
                        anchor,
                        iend,
                        OFFSET_TO_OFFBASE(offset),
                        mLength,
                    );
                    break;
                }

                /* Prepare for next iteration */
                dictMatchIndexAndTag =
                    *dictHashTable.add(dictHashAndTag1 >> ZSTD_SHORT_CACHE_TAG_BITS);
                dictTagsMatch =
                    ZSTD_comparePackedTags(dictMatchIndexAndTag as usize, dictHashAndTag1);
                matchIndex = *hashTable.add(hash1);

                if ip1 >= nextStep {
                    step += 1;
                    nextStep = nextStep.wrapping_add(kStepIncr);
                }
                ip0 = ip1;
                ip1 = ip1.wrapping_add(step);
                if ip1 > ilimit {
                    break 'cleanup; /* goto _cleanup */
                }

                curr = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
                hash0 = hash1;
            } /* end inner search loop */

            /* match found */
            ip0 = ip0.wrapping_add(mLength);
            anchor = ip0;

            if ip0 <= ilimit {
                /* Fill Table */
                /* here because curr+2 could be > iend-8 */
                *hashTable.add(ZSTD_hashPtr(
                    base.wrapping_add(curr as usize).wrapping_add(2) as *const c_void,
                    hlog,
                    mls,
                )) = curr.wrapping_add(2);
                *hashTable.add(ZSTD_hashPtr(
                    ip0.wrapping_sub(2) as *const c_void,
                    hlog,
                    mls,
                )) = ((ip0 as usize).wrapping_sub(2).wrapping_sub(base as usize)) as U32;

                /* check immediate repcode */
                while ip0 <= ilimit {
                    let current2: U32 = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
                    let repIndex2: U32 = current2.wrapping_sub(offset_2);
                    let repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                        dictBase
                            .wrapping_sub(dictIndexDelta as usize)
                            .wrapping_add(repIndex2 as usize)
                    } else {
                        base.wrapping_add(repIndex2 as usize)
                    };
                    if (ZSTD_index_overlap_check(prefixStartIndex, repIndex2) != 0)
                        && (MEM_read32(repMatch2) == MEM_read32(ip0))
                    {
                        let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                            dictEnd
                        } else {
                            iend
                        };
                        let repLength2: usize = ZSTD_count_2segments(
                            ip0.wrapping_add(4),
                            repMatch2.wrapping_add(4),
                            iend,
                            repEnd2,
                            prefixStart,
                        )
                        .wrapping_add(4);
                        let tmpOffset: U32 = offset_2;
                        offset_2 = offset_1;
                        offset_1 = tmpOffset; /* swap offset_2 <=> offset_1 */
                        ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, repLength2);
                        *hashTable.add(ZSTD_hashPtr(ip0 as *const c_void, hlog, mls)) = current2;
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
    }

    /* _cleanup: */
    /* save reps for next block */
    *rep.add(0) = offset_1;
    *rep.add(1) = offset_2;

    /* Return the last literals size */
    (iend as usize).wrapping_sub(anchor as usize)
}

/* ===  ZSTD_GEN_FAST_FN(dictMatchState, mml, 0)  === */

pub unsafe fn ZSTD_compressBlock_fast_dictMatchState_4_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 4, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_dictMatchState_5_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 5, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_dictMatchState_6_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 6, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_dictMatchState_7_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_dictMatchState_generic(ms, seqStore, rep, src, srcSize, 7, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_fast_dictMatchState_5_0(ms, seqStore, rep, src, srcSize),
        6 => ZSTD_compressBlock_fast_dictMatchState_6_0(ms, seqStore, rep, src, srcSize),
        7 => ZSTD_compressBlock_fast_dictMatchState_7_0(ms, seqStore, rep, src, srcSize),
        /* default: includes case 3, and case 4 */
        _ => ZSTD_compressBlock_fast_dictMatchState_4_0(ms, seqStore, rep, src, srcSize),
    }
}

/* ===  extDict  === */

pub unsafe fn ZSTD_compressBlock_fast_extDict_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    mls: U32,
    hasStep: U32,
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog;
    /* support stepSize of 0 */
    let stepSize: usize = ((*cParams).targetLength as usize)
        .wrapping_add(((*cParams).targetLength == 0) as usize)
        .wrapping_add(1);
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let istart: *const BYTE = src as *const BYTE;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 = (((istart as usize).wrapping_sub(base as usize)).wrapping_add(srcSize)) as U32;
    let lowLimit: U32 = ZSTD_getLowestMatchIndex(ms, endIndex, (*cParams).windowLog);
    let dictStartIndex: U32 = lowLimit;
    let dictStart: *const BYTE = dictBase.wrapping_add(dictStartIndex as usize);
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStartIndex: U32 = if dictLimit < lowLimit { lowLimit } else { dictLimit };
    let prefixStart: *const BYTE = base.wrapping_add(prefixStartIndex as usize);
    let dictEnd: *const BYTE = dictBase.wrapping_add(prefixStartIndex as usize);
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(8);
    let mut offset_1: U32 = *rep.add(0);
    let mut offset_2: U32 = *rep.add(1);
    let mut offsetSaved1: U32 = 0;
    let mut offsetSaved2: U32 = 0;

    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE;
    let mut ip2: *const BYTE;
    let mut ip3: *const BYTE;
    let mut current0: U32 = 0;

    let mut hash0: usize = 0; /* hash for ip0 */
    let mut hash1: usize = 0; /* hash for ip1 */
    let mut idx: U32 = 0; /* match idx for ip0 */
    let mut idxBase: *const BYTE = core::ptr::null(); /* base pointer for idx */

    let mut offcode: U32 = 0;
    let mut match0: *const BYTE = core::ptr::null();
    let mut mLength: usize = 0;
    let mut matchEnd: *const BYTE = core::ptr::null(); /* initialize to avoid warning */

    let mut step: usize;
    let mut nextStep: *const BYTE;
    let kStepIncr: usize = 1usize << (kSearchStrength - 1);

    let _ = hasStep; /* not currently specialized on whether it's accelerated */

    /* switch to "regular" variant if extDict is invalidated due to maxDistance */
    if prefixStartIndex == dictStartIndex {
        return ZSTD_compressBlock_fast(ms, seqStore, rep, src, srcSize);
    }

    {
        let curr: U32 = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
        let maxRep: U32 = curr.wrapping_sub(dictStartIndex);
        if offset_2 >= maxRep {
            offsetSaved2 = offset_2;
            offset_2 = 0;
        }
        if offset_1 >= maxRep {
            offsetSaved1 = offset_1;
            offset_1 = 0;
        }
    }

    /* start each op */
    /* _start: Requires: ip0 */
    'start: loop {
        step = stepSize;
        nextStep = ip0.wrapping_add(kStepIncr);

        /* calculate positions, ip0 - anchor == 0, so we skip step calc */
        ip1 = ip0.wrapping_add(1);
        ip2 = ip0.wrapping_add(step);
        ip3 = ip2.wrapping_add(1);

        /* 0 -> _cleanup, 1 -> _offset, 2 -> _match */
        let mut jump: u32 = 0;

        if ip3 < ilimit {
            hash0 = ZSTD_hashPtr(ip0 as *const c_void, hlog, mls);
            hash1 = ZSTD_hashPtr(ip1 as *const c_void, hlog, mls);

            idx = *hashTable.add(hash0);
            idxBase = if idx < prefixStartIndex { dictBase } else { base };

            loop {
                {
                    /* load repcode match for ip[2] */
                    let current2: U32 = ((ip2 as usize).wrapping_sub(base as usize)) as U32;
                    let repIndex: U32 = current2.wrapping_sub(offset_1);
                    let repBase: *const BYTE = if repIndex < prefixStartIndex {
                        dictBase
                    } else {
                        base
                    };
                    let rval: U32;
                    if (((prefixStartIndex.wrapping_sub(repIndex) >= 4) as u32) /* intentional underflow */
                        & ((offset_1 > 0) as u32))
                        != 0
                    {
                        rval = MEM_read32(repBase.wrapping_add(repIndex as usize));
                    } else {
                        rval = MEM_read32(ip2) ^ 1; /* guaranteed to not match. */
                    }

                    /* write back hash table entry */
                    current0 = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
                    *hashTable.add(hash0) = current0;

                    /* check repcode at ip[2] */
                    if MEM_read32(ip2) == rval {
                        ip0 = ip2;
                        match0 = repBase.wrapping_add(repIndex as usize);
                        matchEnd = if repIndex < prefixStartIndex {
                            dictEnd
                        } else {
                            iend
                        };
                        mLength = (*ip0.offset(-1) == *match0.offset(-1)) as usize;
                        ip0 = ip0.wrapping_sub(mLength);
                        match0 = match0.wrapping_sub(mLength);
                        offcode = REPCODE1_TO_OFFBASE;
                        mLength = mLength.wrapping_add(4);
                        jump = 2; /* goto _match */
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
                        /* found a match! */
                        jump = 1; /* goto _offset */
                        break;
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
                current0 = ((ip0 as usize).wrapping_sub(base as usize)) as U32;
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
                        jump = 1; /* goto _offset */
                        break;
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
                ip2 = ip0.wrapping_add(step);
                ip3 = ip1.wrapping_add(step);

                /* calculate step */
                if ip2 >= nextStep {
                    step += 1;
                    PREFETCH_L1(ip1.wrapping_add(64));
                    PREFETCH_L1(ip1.wrapping_add(128));
                    nextStep = nextStep.wrapping_add(kStepIncr);
                }

                if !(ip3 < ilimit) {
                    break;
                }
            }
        }

        if jump == 0 {
            /* _cleanup:
             * Note that there are probably still a couple positions we could search.
             * However, it seems to be a meaningful performance hit to try to search
             * them. So let's not.
             *
             * If offset_1 started invalid (offsetSaved1 != 0) and became valid (offset_1 != 0),
             * rotate saved offsets. See comment in ZSTD_compressBlock_fast_noDict for more context.
             */
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

        if jump == 1 {
            /* _offset: Requires: ip0, idx, idxBase */

            /* Compute the offset code. */
            {
                let offset: U32 = current0.wrapping_sub(idx);
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
                while ((((ip0 > anchor) as u32) & ((match0 > lowMatchPtr) as u32)) != 0)
                    && (*ip0.offset(-1) == *match0.offset(-1))
                {
                    ip0 = ip0.wrapping_sub(1);
                    match0 = match0.wrapping_sub(1);
                    mLength += 1;
                }
            }
        }

        /* _match: Requires: ip0, match0, offcode, matchEnd */

        /* Count the forward length. */
        mLength = mLength.wrapping_add(ZSTD_count_2segments(
            ip0.wrapping_add(mLength),
            match0.wrapping_add(mLength),
            iend,
            matchEnd,
            prefixStart,
        ));

        ZSTD_storeSeq(
            seqStore,
            (ip0 as usize).wrapping_sub(anchor as usize),
            anchor,
            iend,
            offcode,
            mLength,
        );

        ip0 = ip0.wrapping_add(mLength);
        anchor = ip0;

        /* write next hash table entry */
        if ip1 < ip0 {
            *hashTable.add(hash1) = ((ip1 as usize).wrapping_sub(base as usize)) as U32;
        }

        /* Fill table and check for immediate repcode. */
        if ip0 <= ilimit {
            /* Fill Table */
            /* here because current+2 could be > iend-8 */
            *hashTable.add(ZSTD_hashPtr(
                base.wrapping_add(current0 as usize).wrapping_add(2) as *const c_void,
                hlog,
                mls,
            )) = current0.wrapping_add(2);
            *hashTable.add(ZSTD_hashPtr(
                ip0.wrapping_sub(2) as *const c_void,
                hlog,
                mls,
            )) = ((ip0 as usize).wrapping_sub(2).wrapping_sub(base as usize)) as U32;

            while ip0 <= ilimit {
                let repIndex2: U32 =
                    (((ip0 as usize).wrapping_sub(base as usize)) as U32).wrapping_sub(offset_2);
                let repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictBase.wrapping_add(repIndex2 as usize)
                } else {
                    base.wrapping_add(repIndex2 as usize)
                };
                if ((ZSTD_index_overlap_check(prefixStartIndex, repIndex2)
                    & ((offset_2 > 0) as c_int))
                    != 0)
                    && (MEM_read32(repMatch2) == MEM_read32(ip0))
                {
                    let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                        dictEnd
                    } else {
                        iend
                    };
                    let repLength2: usize = ZSTD_count_2segments(
                        ip0.wrapping_add(4),
                        repMatch2.wrapping_add(4),
                        iend,
                        repEnd2,
                        prefixStart,
                    )
                    .wrapping_add(4);
                    {
                        /* swap offset_2 <=> offset_1 */
                        let tmpOffset: U32 = offset_2;
                        offset_2 = offset_1;
                        offset_1 = tmpOffset;
                    }
                    ZSTD_storeSeq(seqStore, 0, anchor, iend, REPCODE1_TO_OFFBASE, repLength2);
                    *hashTable.add(ZSTD_hashPtr(ip0 as *const c_void, hlog, mls)) =
                        ((ip0 as usize).wrapping_sub(base as usize)) as U32;
                    ip0 = ip0.wrapping_add(repLength2);
                    anchor = ip0;
                    continue;
                }
                break;
            }
        }

        continue 'start; /* goto _start */
    }
}

/* ===  ZSTD_GEN_FAST_FN(extDict, mml, 0)  === */

pub unsafe fn ZSTD_compressBlock_fast_extDict_4_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, 4, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_extDict_5_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, 5, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_extDict_6_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, 6, 0)
}

pub unsafe fn ZSTD_compressBlock_fast_extDict_7_0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_fast_extDict_generic(ms, seqStore, rep, src, srcSize, 7, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mls: U32 = (*ms).cParams.minMatch;
    match mls {
        5 => ZSTD_compressBlock_fast_extDict_5_0(ms, seqStore, rep, src, srcSize),
        6 => ZSTD_compressBlock_fast_extDict_6_0(ms, seqStore, rep, src, srcSize),
        7 => ZSTD_compressBlock_fast_extDict_7_0(ms, seqStore, rep, src, srcSize),
        /* default: includes case 3, and case 4 */
        _ => ZSTD_compressBlock_fast_extDict_4_0(ms, seqStore, rep, src, srcSize),
    }
}
