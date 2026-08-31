//! Translation of `compress/zstd_opt.c` (and `compress/zstd_opt.h`)
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};
use core::ptr::{addr_of, addr_of_mut};

use crate::bits::*;
use crate::cmem::*;
use crate::compress::hist::HIST_count_simple;
use crate::compress::zstd_compress_internal::*;
use crate::fse::*;
use crate::huf::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* `HUF_getNbBitsFromCTable()` lives in `compress/huf_compress.c`. */
use crate::compress::huf_compress::HUF_getNbBitsFromCTable;

/* `ZSTD_resetSeqStore()` lives in `compress/zstd_compress.c`; it is an exported
 * (`no_mangle`) symbol of this cdylib, so it is declared here rather than
 * imported, because `crate::compress::zstd_compress` does not define it yet.
 * Once it does, this block can be replaced by
 * `use crate::compress::zstd_compress::ZSTD_resetSeqStore;`. */
unsafe extern "C" {
    fn ZSTD_resetSeqStore(ssPtr: *mut SeqStore_t);
}

/* scaling factor for litFreq, so that frequencies adapt faster to new stats */
const ZSTD_LITFREQ_ADD: c_uint = 2;
const ZSTD_MAX_PRICE: c_int = 1 << 30;

/* if srcSize < ZSTD_PREDEF_THRESHOLD, symbols' cost is assumed static,
 * directly determined by pre-defined distributions */
const ZSTD_PREDEF_THRESHOLD: usize = 8;

/*-*************************************
*  Price functions for optimal parser
***************************************/

/* opt==approx, ultra==accurate */
const BITCOST_ACCURACY: u32 = 8;
const BITCOST_MULTIPLIER: U32 = 1 << BITCOST_ACCURACY;

/// `WEIGHT(stat, opt)`
#[inline(always)]
fn WEIGHT(stat: U32, opt: c_int) -> U32 {
    if opt != 0 {
        ZSTD_fracWeight(stat)
    } else {
        ZSTD_bitWeight(stat)
    }
}

/* ZSTD_bitWeight() :
 * provide estimated "cost" of a stat in full bits only */
fn ZSTD_bitWeight(stat: U32) -> U32 {
    ZSTD_highbit32(stat.wrapping_add(1)).wrapping_mul(BITCOST_MULTIPLIER)
}

/* ZSTD_fracWeight() :
 * provide fractional-bit "cost" of a stat,
 * using linear interpolation approximation */
fn ZSTD_fracWeight(rawStat: U32) -> U32 {
    let stat: U32 = rawStat.wrapping_add(1);
    let hb: U32 = ZSTD_highbit32(stat);
    let BWeight: U32 = hb.wrapping_mul(BITCOST_MULTIPLIER);
    /* Fweight was meant for "Fractional weight"
     * but it's effectively a value between 1 and 2
     * using fixed point arithmetic */
    let FWeight: U32 = (stat << BITCOST_ACCURACY) >> hb;
    BWeight.wrapping_add(FWeight)
}

unsafe fn ZSTD_compressedLiterals(optPtr: *const optState_t) -> c_int {
    ((*optPtr).literalCompressionMode != ZSTD_ps_disable) as c_int
}

unsafe fn ZSTD_setBasePrices(optPtr: *mut optState_t, optLevel: c_int) {
    if ZSTD_compressedLiterals(optPtr) != 0 {
        (*optPtr).litSumBasePrice = WEIGHT((*optPtr).litSum, optLevel);
    }
    (*optPtr).litLengthSumBasePrice = WEIGHT((*optPtr).litLengthSum, optLevel);
    (*optPtr).matchLengthSumBasePrice = WEIGHT((*optPtr).matchLengthSum, optLevel);
    (*optPtr).offCodeSumBasePrice = WEIGHT((*optPtr).offCodeSum, optLevel);
}

unsafe fn sum_u32(table: *const c_uint, nbElts: usize) -> U32 {
    let mut total: U32 = 0;
    let mut n: usize = 0;
    while n < nbElts {
        total = total.wrapping_add(*table.add(n));
        n += 1;
    }
    total
}

type base_directive_e = c_uint;
const base_0possible: base_directive_e = 0;
const base_1guaranteed: base_directive_e = 1;

unsafe fn ZSTD_downscaleStats(
    table: *mut c_uint,
    lastEltIndex: U32,
    shift: U32,
    base1: base_directive_e,
) -> U32 {
    let mut s: U32 = 0;
    let mut sum: U32 = 0;
    while s < lastEltIndex.wrapping_add(1) {
        let base: c_uint = if base1 != 0 {
            1
        } else {
            (*table.add(s as usize) > 0) as c_uint
        };
        let newStat: c_uint = base.wrapping_add(*table.add(s as usize) >> shift);
        sum = sum.wrapping_add(newStat);
        *table.add(s as usize) = newStat;
        s = s.wrapping_add(1);
    }
    sum
}

/* ZSTD_scaleStats() :
 * reduce all elt frequencies in table if sum too large
 * return the resulting sum of elements */
unsafe fn ZSTD_scaleStats(table: *mut c_uint, lastEltIndex: U32, logTarget: U32) -> U32 {
    let prevsum: U32 = sum_u32(table, lastEltIndex.wrapping_add(1) as usize);
    let factor: U32 = prevsum >> logTarget;
    if factor <= 1 {
        return prevsum;
    }
    ZSTD_downscaleStats(table, lastEltIndex, ZSTD_highbit32(factor), base_1guaranteed)
}

static baseLLfreqs: [c_uint; (MaxLL + 1) as usize] = [
    4, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1,
];

static baseOFCfreqs: [c_uint; (MaxOff + 1) as usize] = [
    6, 2, 1, 1, 2, 3, 4, 4, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];

/* ZSTD_rescaleFreqs() :
 * if first block (detected by optPtr->litLengthSum == 0) : init statistics
 *    take hints from dictionary if there is one
 *    and init from zero if there is none,
 *    using src for literals stats, and baseline stats for sequence symbols
 * otherwise downscale existing stats, to be used as seed for next block.
 */
unsafe fn ZSTD_rescaleFreqs(
    optPtr: *mut optState_t,
    src: *const BYTE,
    srcSize: usize,
    optLevel: c_int,
) {
    let compressedLiterals: c_int = ZSTD_compressedLiterals(optPtr);
    (*optPtr).priceType = zop_dynamic;

    if (*optPtr).litLengthSum == 0 {
        /* no literals stats collected -> first block assumed -> init */

        /* heuristic: use pre-defined stats for too small inputs */
        if srcSize <= ZSTD_PREDEF_THRESHOLD {
            (*optPtr).priceType = zop_predef;
        }

        if (*(*optPtr).symbolCosts).huf.repeatMode == HUF_repeat_valid {
            /* huffman stats covering the full value set : table presumed generated by dictionary */
            (*optPtr).priceType = zop_dynamic;

            if compressedLiterals != 0 {
                /* generate literals statistics from huffman table */
                let mut lit: c_uint = 0;
                (*optPtr).litSum = 0;
                while lit <= MaxLit {
                    let scaleLog: U32 = 11; /* scale to 2K */
                    let bitCost: U32 = HUF_getNbBitsFromCTable(
                        addr_of!((*(*optPtr).symbolCosts).huf.CTable) as *const HUF_CElt,
                        lit,
                    );
                    *(*optPtr).litFreq.add(lit as usize) = if bitCost != 0 {
                        1 << (scaleLog - bitCost)
                    } else {
                        1
                    };
                    (*optPtr).litSum = (*optPtr)
                        .litSum
                        .wrapping_add(*(*optPtr).litFreq.add(lit as usize));
                    lit = lit.wrapping_add(1);
                }
            }

            {
                let mut ll: c_uint = 0;
                let mut llstate = FSE_CState_t::default();
                FSE_initCState(
                    &mut llstate,
                    addr_of!((*(*optPtr).symbolCosts).fse.litlengthCTable) as *const FSE_CTable,
                );
                (*optPtr).litLengthSum = 0;
                while ll <= MaxLL {
                    let scaleLog: U32 = 10; /* scale to 1K */
                    let bitCost: U32 = FSE_getMaxNbBits(llstate.symbolTT, ll);
                    *(*optPtr).litLengthFreq.add(ll as usize) = if bitCost != 0 {
                        1 << (scaleLog - bitCost)
                    } else {
                        1
                    };
                    (*optPtr).litLengthSum = (*optPtr)
                        .litLengthSum
                        .wrapping_add(*(*optPtr).litLengthFreq.add(ll as usize));
                    ll = ll.wrapping_add(1);
                }
            }

            {
                let mut ml: c_uint = 0;
                let mut mlstate = FSE_CState_t::default();
                FSE_initCState(
                    &mut mlstate,
                    addr_of!((*(*optPtr).symbolCosts).fse.matchlengthCTable) as *const FSE_CTable,
                );
                (*optPtr).matchLengthSum = 0;
                while ml <= MaxML {
                    let scaleLog: U32 = 10;
                    let bitCost: U32 = FSE_getMaxNbBits(mlstate.symbolTT, ml);
                    *(*optPtr).matchLengthFreq.add(ml as usize) = if bitCost != 0 {
                        1 << (scaleLog - bitCost)
                    } else {
                        1
                    };
                    (*optPtr).matchLengthSum = (*optPtr)
                        .matchLengthSum
                        .wrapping_add(*(*optPtr).matchLengthFreq.add(ml as usize));
                    ml = ml.wrapping_add(1);
                }
            }

            {
                let mut of: c_uint = 0;
                let mut ofstate = FSE_CState_t::default();
                FSE_initCState(
                    &mut ofstate,
                    addr_of!((*(*optPtr).symbolCosts).fse.offcodeCTable) as *const FSE_CTable,
                );
                (*optPtr).offCodeSum = 0;
                while of <= MaxOff {
                    let scaleLog: U32 = 10;
                    let bitCost: U32 = FSE_getMaxNbBits(ofstate.symbolTT, of);
                    *(*optPtr).offCodeFreq.add(of as usize) = if bitCost != 0 {
                        1 << (scaleLog - bitCost)
                    } else {
                        1
                    };
                    (*optPtr).offCodeSum = (*optPtr)
                        .offCodeSum
                        .wrapping_add(*(*optPtr).offCodeFreq.add(of as usize));
                    of = of.wrapping_add(1);
                }
            }
        } else {
            /* first block, no dictionary */

            if compressedLiterals != 0 {
                /* base initial cost of literals on direct frequency within src */
                let mut lit: c_uint = MaxLit;
                HIST_count_simple(
                    (*optPtr).litFreq,
                    &mut lit,
                    src as *const c_void,
                    srcSize,
                ); /* use raw first block to init statistics */
                (*optPtr).litSum =
                    ZSTD_downscaleStats((*optPtr).litFreq, MaxLit, 8, base_0possible);
            }

            {
                ZSTD_memcpy(
                    (*optPtr).litLengthFreq as *mut c_void,
                    baseLLfreqs.as_ptr() as *const c_void,
                    core::mem::size_of_val(&baseLLfreqs),
                );
                (*optPtr).litLengthSum = sum_u32(baseLLfreqs.as_ptr(), (MaxLL + 1) as usize);
            }

            {
                let mut ml: c_uint = 0;
                while ml <= MaxML {
                    *(*optPtr).matchLengthFreq.add(ml as usize) = 1;
                    ml = ml.wrapping_add(1);
                }
            }
            (*optPtr).matchLengthSum = MaxML + 1;

            {
                ZSTD_memcpy(
                    (*optPtr).offCodeFreq as *mut c_void,
                    baseOFCfreqs.as_ptr() as *const c_void,
                    core::mem::size_of_val(&baseOFCfreqs),
                );
                (*optPtr).offCodeSum = sum_u32(baseOFCfreqs.as_ptr(), (MaxOff + 1) as usize);
            }
        }
    } else {
        /* new block : scale down accumulated statistics */

        if compressedLiterals != 0 {
            (*optPtr).litSum = ZSTD_scaleStats((*optPtr).litFreq, MaxLit, 12);
        }
        (*optPtr).litLengthSum = ZSTD_scaleStats((*optPtr).litLengthFreq, MaxLL, 11);
        (*optPtr).matchLengthSum = ZSTD_scaleStats((*optPtr).matchLengthFreq, MaxML, 11);
        (*optPtr).offCodeSum = ZSTD_scaleStats((*optPtr).offCodeFreq, MaxOff, 11);
    }

    ZSTD_setBasePrices(optPtr, optLevel);
}

/* ZSTD_rawLiteralsCost() :
 * price of literals (only) in specified segment (which length can be 0).
 * does not include price of literalLength symbol */
unsafe fn ZSTD_rawLiteralsCost(
    literals: *const BYTE,
    litLength: U32,
    optPtr: *const optState_t,
    optLevel: c_int,
) -> U32 {
    if litLength == 0 {
        return 0;
    }

    if ZSTD_compressedLiterals(optPtr) == 0 {
        /* Uncompressed - 8 bytes per literal. */
        return (litLength << 3).wrapping_mul(BITCOST_MULTIPLIER);
    }

    if (*optPtr).priceType == zop_predef {
        /* 6 bit per literal - no statistic used */
        return litLength.wrapping_mul(6).wrapping_mul(BITCOST_MULTIPLIER);
    }

    /* dynamic statistics */
    {
        let mut price: U32 = (*optPtr).litSumBasePrice.wrapping_mul(litLength);
        let litPriceMax: U32 = (*optPtr).litSumBasePrice.wrapping_sub(BITCOST_MULTIPLIER);
        let mut u: U32 = 0;
        while u < litLength {
            let mut litPrice: U32 = WEIGHT(
                *(*optPtr)
                    .litFreq
                    .add(*literals.add(u as usize) as usize),
                optLevel,
            );
            if litPrice > litPriceMax {
                litPrice = litPriceMax;
            }
            price = price.wrapping_sub(litPrice);
            u = u.wrapping_add(1);
        }
        price
    }
}

/* ZSTD_litLengthPrice() :
 * cost of literalLength symbol */
unsafe fn ZSTD_litLengthPrice(litLength: U32, optPtr: *const optState_t, optLevel: c_int) -> U32 {
    if (*optPtr).priceType == zop_predef {
        return WEIGHT(litLength, optLevel);
    }

    /* ZSTD_LLcode() can't compute litLength price for sizes >= ZSTD_BLOCKSIZE_MAX
     * because it isn't representable in the zstd format.
     * So instead just pretend it would cost 1 bit more than ZSTD_BLOCKSIZE_MAX - 1.
     * In such a case, the block would be all literals.
     */
    if litLength == ZSTD_BLOCKSIZE_MAX as U32 {
        return BITCOST_MULTIPLIER.wrapping_add(ZSTD_litLengthPrice(
            ZSTD_BLOCKSIZE_MAX as U32 - 1,
            optPtr,
            optLevel,
        ));
    }

    /* dynamic statistics */
    {
        let llCode: U32 = ZSTD_LLcode(litLength);
        ((LL_bits[llCode as usize] as U32).wrapping_mul(BITCOST_MULTIPLIER))
            .wrapping_add((*optPtr).litLengthSumBasePrice)
            .wrapping_sub(WEIGHT(
                *(*optPtr).litLengthFreq.add(llCode as usize),
                optLevel,
            ))
    }
}

/* ZSTD_getMatchPrice() :
 * Provides the cost of the match part (offset + matchLength) of a sequence.
 * Must be combined with ZSTD_fullLiteralsCost() to get the full cost of a sequence.
 * @offBase : sumtype, representing an offset or a repcode, and using numeric representation of ZSTD_storeSeq()
 * @optLevel: when <2, favors small offset for decompression speed (improved cache efficiency)
 */
unsafe fn ZSTD_getMatchPrice(
    offBase: U32,
    matchLength: U32,
    optPtr: *const optState_t,
    optLevel: c_int,
) -> U32 {
    let mut price: U32;
    let offCode: U32 = ZSTD_highbit32(offBase);
    let mlBase: U32 = matchLength.wrapping_sub(MINMATCH as U32);

    if (*optPtr).priceType == zop_predef {
        /* fixed scheme, does not use statistics */
        return WEIGHT(mlBase, optLevel).wrapping_add(
            (16u32.wrapping_add(offCode)).wrapping_mul(BITCOST_MULTIPLIER), /* emulated offset cost */
        );
    }

    /* dynamic statistics */
    price = (offCode.wrapping_mul(BITCOST_MULTIPLIER)).wrapping_add(
        (*optPtr)
            .offCodeSumBasePrice
            .wrapping_sub(WEIGHT(*(*optPtr).offCodeFreq.add(offCode as usize), optLevel)),
    );
    if (optLevel < 2) && offCode >= 20 {
        /* handicap for long distance offsets, favor decompression speed */
        price = price.wrapping_add(
            offCode
                .wrapping_sub(19)
                .wrapping_mul(2)
                .wrapping_mul(BITCOST_MULTIPLIER),
        );
    }

    /* match Length */
    {
        let mlCode: U32 = ZSTD_MLcode(mlBase);
        price = price.wrapping_add(
            ((ML_bits[mlCode as usize] as U32).wrapping_mul(BITCOST_MULTIPLIER)).wrapping_add(
                (*optPtr).matchLengthSumBasePrice.wrapping_sub(WEIGHT(
                    *(*optPtr).matchLengthFreq.add(mlCode as usize),
                    optLevel,
                )),
            ),
        );
    }

    /* heuristic : make matches a bit more costly to favor less sequences -> faster decompression speed */
    price = price.wrapping_add(BITCOST_MULTIPLIER / 5);

    price
}

/* ZSTD_updateStats() :
 * assumption : literals + litLength <= iend */
unsafe fn ZSTD_updateStats(
    optPtr: *mut optState_t,
    litLength: U32,
    literals: *const BYTE,
    offBase: U32,
    matchLength: U32,
) {
    /* literals */
    if ZSTD_compressedLiterals(optPtr) != 0 {
        let mut u: U32 = 0;
        while u < litLength {
            let idx = *literals.add(u as usize) as usize;
            *(*optPtr).litFreq.add(idx) =
                (*(*optPtr).litFreq.add(idx)).wrapping_add(ZSTD_LITFREQ_ADD);
            u = u.wrapping_add(1);
        }
        (*optPtr).litSum = (*optPtr)
            .litSum
            .wrapping_add(litLength.wrapping_mul(ZSTD_LITFREQ_ADD));
    }

    /* literal Length */
    {
        let llCode: U32 = ZSTD_LLcode(litLength);
        *(*optPtr).litLengthFreq.add(llCode as usize) =
            (*(*optPtr).litLengthFreq.add(llCode as usize)).wrapping_add(1);
        (*optPtr).litLengthSum = (*optPtr).litLengthSum.wrapping_add(1);
    }

    /* offset code : follows storeSeq() numeric representation */
    {
        let offCode: U32 = ZSTD_highbit32(offBase);
        *(*optPtr).offCodeFreq.add(offCode as usize) =
            (*(*optPtr).offCodeFreq.add(offCode as usize)).wrapping_add(1);
        (*optPtr).offCodeSum = (*optPtr).offCodeSum.wrapping_add(1);
    }

    /* match Length */
    {
        let mlBase: U32 = matchLength.wrapping_sub(MINMATCH as U32);
        let mlCode: U32 = ZSTD_MLcode(mlBase);
        *(*optPtr).matchLengthFreq.add(mlCode as usize) =
            (*(*optPtr).matchLengthFreq.add(mlCode as usize)).wrapping_add(1);
        (*optPtr).matchLengthSum = (*optPtr).matchLengthSum.wrapping_add(1);
    }
}

/* ZSTD_readMINMATCH() :
 * function safe only for comparisons
 * assumption : memPtr must be at least 4 bytes before end of buffer */
unsafe fn ZSTD_readMINMATCH(memPtr: *const c_void, length: U32) -> U32 {
    match length {
        3 => {
            if MEM_isLittleEndian() != 0 {
                MEM_read32(memPtr) << 8
            } else {
                MEM_read32(memPtr) >> 8
            }
        }
        _ => MEM_read32(memPtr),
    }
}

/* Update hashTable3 up to ip (excluded)
Assumption : always within prefix (i.e. not within extDict) */
unsafe fn ZSTD_insertAndFindFirstIndexHash3(
    ms: *const ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
) -> U32 {
    let hashTable3: *mut U32 = (*ms).hashTable3;
    let hashLog3: U32 = (*ms).hashLog3;
    let base: *const BYTE = (*ms).window.base;
    let mut idx: U32 = *nextToUpdate3;
    let target: U32 = (ip as usize).wrapping_sub(base as usize) as U32;
    let hash3: usize = ZSTD_hash3Ptr(ip as *const c_void, hashLog3);

    while idx < target {
        *hashTable3.add(ZSTD_hash3Ptr(
            base.wrapping_add(idx as usize) as *const c_void,
            hashLog3,
        )) = idx;
        idx = idx.wrapping_add(1);
    }

    *nextToUpdate3 = target;
    *hashTable3.add(hash3)
}

/*-*************************************
*  Binary Tree search
***************************************/
/** ZSTD_insertBt1() : add one or multiple positions to tree.
 * @param ip assumed <= iend-8 .
 * @param target The target of ZSTD_updateTree_internal() - we are filling to this position
 * @return : nb of positions added */
unsafe fn ZSTD_insertBt1(
    ms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    target: U32,
    mls: U32,
    extDict: c_int,
) -> U32 {
    let cParams: *const ZSTD_compressionParameters = addr_of!((*ms).cParams);
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;
    let h: usize = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog.wrapping_sub(1);
    let btMask: U32 = (1u32 << btLog).wrapping_sub(1);
    let mut matchIndex: U32 = *hashTable.add(h);
    let mut commonLengthSmaller: usize = 0;
    let mut commonLengthLarger: usize = 0;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
    let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
    let mut match_: *const BYTE;
    let curr: U32 = (ip as usize).wrapping_sub(base as usize) as U32;
    let btLow: U32 = if btMask >= curr {
        0
    } else {
        curr.wrapping_sub(btMask)
    };
    let mut smallerPtr: *mut U32 = bt.add((2u32.wrapping_mul(curr & btMask)) as usize);
    let mut largerPtr: *mut U32 = smallerPtr.add(1);
    let mut dummy32: U32 = 0; /* to be nullified at the end */
    /* windowLow is based on target because
     * we only need positions that will be in the window at the end of the tree update.
     */
    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, target, (*cParams).windowLog);
    let mut matchEndIdx: U32 = curr.wrapping_add(8).wrapping_add(1);
    let mut bestLength: usize = 8;
    let mut nbCompares: U32 = 1u32 << (*cParams).searchLog;

    *hashTable.add(h) = curr; /* Update Hash Table */

    while nbCompares != 0 && matchIndex >= windowLow {
        let nextPtr: *mut U32 = bt.add((2u32.wrapping_mul(matchIndex & btMask)) as usize);
        /* guaranteed minimum nb of common bytes */
        let mut matchLength: usize = if commonLengthSmaller < commonLengthLarger {
            commonLengthSmaller
        } else {
            commonLengthLarger
        };

        if extDict == 0
            || ((matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize)
        {
            match_ = base.wrapping_add(matchIndex as usize);
            matchLength = matchLength.wrapping_add(ZSTD_count(
                ip.wrapping_add(matchLength),
                match_.wrapping_add(matchLength),
                iend,
            ));
        } else {
            match_ = dictBase.wrapping_add(matchIndex as usize);
            matchLength = matchLength.wrapping_add(ZSTD_count_2segments(
                ip.wrapping_add(matchLength),
                match_.wrapping_add(matchLength),
                iend,
                dictEnd,
                prefixStart,
            ));
            if (matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize {
                /* to prepare for next usage of match[matchLength] */
                match_ = base.wrapping_add(matchIndex as usize);
            }
        }

        if matchLength > bestLength {
            bestLength = matchLength;
            if matchLength > matchEndIdx.wrapping_sub(matchIndex) as usize {
                matchEndIdx = matchIndex.wrapping_add(matchLength as U32);
            }
        }

        if ip.wrapping_add(matchLength) == iend {
            /* equal : no way to know if inf or sup */
            break; /* drop , to guarantee consistency ; miss a bit of compression, but other solutions can corrupt tree */
        }

        if *match_.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
            /* match is smaller than current */
            *smallerPtr = matchIndex; /* update smaller idx */
            commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
            if matchIndex <= btLow {
                smallerPtr = addr_of_mut!(dummy32);
                break;
            } /* beyond tree size, stop searching */
            smallerPtr = nextPtr.add(1); /* new "candidate" => larger than match, which was smaller than target */
            matchIndex = *nextPtr.add(1); /* new matchIndex, larger than previous and closer to current */
        } else {
            /* match is larger than current */
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = addr_of_mut!(dummy32);
                break;
            } /* beyond tree size, stop searching */
            largerPtr = nextPtr;
            matchIndex = *nextPtr.add(0);
        }

        nbCompares = nbCompares.wrapping_sub(1);
    }

    *largerPtr = 0;
    *smallerPtr = 0;
    {
        let mut positions: U32 = 0;
        if bestLength > 384 {
            let d: U32 = bestLength.wrapping_sub(384) as U32;
            positions = if 192 < d { 192 } else { d }; /* speed optimization */
        }
        let forward: U32 = matchEndIdx.wrapping_sub(curr.wrapping_add(8));
        if positions > forward {
            positions
        } else {
            forward
        }
    }
}

unsafe fn ZSTD_updateTree_internal(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) {
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = (ip as usize).wrapping_sub(base as usize) as U32;
    let mut idx: U32 = (*ms).nextToUpdate;

    while idx < target {
        let forward: U32 = ZSTD_insertBt1(
            ms,
            base.wrapping_add(idx as usize),
            iend,
            target,
            mls,
            (dictMode == ZSTD_extDict) as c_int,
        );
        idx = idx.wrapping_add(forward);
    }
    (*ms).nextToUpdate = target;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_updateTree(
    ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
) {
    ZSTD_updateTree_internal(ms, ip, iend, (*ms).cParams.minMatch, ZSTD_noDict);
}

unsafe fn ZSTD_insertBtAndGetAllMatches(
    matches: *mut ZSTD_match_t, /* store result (found matches) in this table (presumed large enough) */
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iLimit: *const BYTE,
    dictMode: ZSTD_dictMode_e,
    rep: *const U32,
    ll0: U32, /* tells if associated literal length is 0 or not. This value must be 0 or 1 */
    lengthToBeat: U32,
    mls: U32, /* template */
) -> U32 {
    let cParams: *const ZSTD_compressionParameters = addr_of!((*ms).cParams);
    let sufficient_len: U32 = if (*cParams).targetLength < (ZSTD_OPT_NUM - 1) as U32 {
        (*cParams).targetLength
    } else {
        (ZSTD_OPT_NUM - 1) as U32
    };
    let base: *const BYTE = (*ms).window.base;
    let curr: U32 = (ip as usize).wrapping_sub(base as usize) as U32;
    let hashLog: U32 = (*cParams).hashLog;
    let minMatch: U32 = if mls == 3 { 3 } else { 4 };
    let hashTable: *mut U32 = (*ms).hashTable;
    let h: usize = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut matchIndex: U32 = *hashTable.add(h);
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog.wrapping_sub(1);
    let btMask: U32 = (1u32 << btLog).wrapping_sub(1);
    let mut commonLengthSmaller: usize = 0;
    let mut commonLengthLarger: usize = 0;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
    let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
    let btLow: U32 = if btMask >= curr {
        0
    } else {
        curr.wrapping_sub(btMask)
    };
    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, (*cParams).windowLog);
    let matchLow: U32 = if windowLow != 0 { windowLow } else { 1 };
    let mut smallerPtr: *mut U32 = bt.add((2u32.wrapping_mul(curr & btMask)) as usize);
    let mut largerPtr: *mut U32 =
        bt.add((2u32.wrapping_mul(curr & btMask).wrapping_add(1)) as usize);
    /* farthest referenced position of any match => detects repetitive patterns */
    let mut matchEndIdx: U32 = curr.wrapping_add(8).wrapping_add(1);
    let mut dummy32: U32 = 0; /* to be nullified at the end */
    let mut mnum: U32 = 0;
    let mut nbCompares: U32 = 1u32 << (*cParams).searchLog;

    let dms: *const ZSTD_MatchState_t = if dictMode == ZSTD_dictMatchState {
        (*ms).dictMatchState
    } else {
        core::ptr::null()
    };
    let dmsCParams: *const ZSTD_compressionParameters = if dictMode == ZSTD_dictMatchState {
        addr_of!((*dms).cParams)
    } else {
        core::ptr::null()
    };
    let dmsBase: *const BYTE = if dictMode == ZSTD_dictMatchState {
        (*dms).window.base
    } else {
        core::ptr::null()
    };
    let dmsEnd: *const BYTE = if dictMode == ZSTD_dictMatchState {
        (*dms).window.nextSrc
    } else {
        core::ptr::null()
    };
    let dmsHighLimit: U32 = if dictMode == ZSTD_dictMatchState {
        (dmsEnd as usize).wrapping_sub(dmsBase as usize) as U32
    } else {
        0
    };
    let dmsLowLimit: U32 = if dictMode == ZSTD_dictMatchState {
        (*dms).window.lowLimit
    } else {
        0
    };
    let dmsIndexDelta: U32 = if dictMode == ZSTD_dictMatchState {
        windowLow.wrapping_sub(dmsHighLimit)
    } else {
        0
    };
    let dmsHashLog: U32 = if dictMode == ZSTD_dictMatchState {
        (*dmsCParams).hashLog
    } else {
        hashLog
    };
    let dmsBtLog: U32 = if dictMode == ZSTD_dictMatchState {
        (*dmsCParams).chainLog.wrapping_sub(1)
    } else {
        btLog
    };
    let dmsBtMask: U32 = if dictMode == ZSTD_dictMatchState {
        (1u32 << dmsBtLog).wrapping_sub(1)
    } else {
        0
    };
    let dmsBtLow: U32 = if dictMode == ZSTD_dictMatchState
        && dmsBtMask < dmsHighLimit.wrapping_sub(dmsLowLimit)
    {
        dmsHighLimit.wrapping_sub(dmsBtMask)
    } else {
        dmsLowLimit
    };

    let mut bestLength: usize = lengthToBeat.wrapping_sub(1) as usize;

    /* check repCode */
    {
        let lastR: U32 = (ZSTD_REP_NUM as U32).wrapping_add(ll0);
        let mut repCode: U32 = ll0;
        while repCode < lastR {
            let repOffset: U32 = if repCode == ZSTD_REP_NUM as U32 {
                (*rep.add(0)).wrapping_sub(1)
            } else {
                *rep.add(repCode as usize)
            };
            let repIndex: U32 = curr.wrapping_sub(repOffset);
            let mut repLen: U32 = 0;
            /* intentional overflow, discards 0 and -1 ;
             * equivalent to `curr > repIndex >= dictLimit` */
            if repOffset.wrapping_sub(1) < curr.wrapping_sub(dictLimit) {
                /* We must validate the repcode offset because when we're using a dictionary the
                 * valid offset range shrinks when the dictionary goes out of bounds.
                 */
                if (repIndex >= windowLow)
                    & (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(
                            ip.wrapping_sub(repOffset as usize) as *const c_void,
                            minMatch,
                        ))
                {
                    repLen = (ZSTD_count(
                        ip.wrapping_add(minMatch as usize),
                        ip.wrapping_add(minMatch as usize)
                            .wrapping_sub(repOffset as usize),
                        iLimit,
                    ) as U32)
                        .wrapping_add(minMatch);
                }
            } else {
                /* repIndex < dictLimit || repIndex >= curr */
                let repMatch: *const BYTE = if dictMode == ZSTD_dictMatchState {
                    dmsBase
                        .wrapping_add(repIndex as usize)
                        .wrapping_sub(dmsIndexDelta as usize)
                } else {
                    dictBase.wrapping_add(repIndex as usize)
                };
                if dictMode == ZSTD_extDict
                    && ((repOffset.wrapping_sub(1) < curr.wrapping_sub(windowLow))
                        & (ZSTD_index_overlap_check(dictLimit, repIndex) != 0))
                    && (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(repMatch as *const c_void, minMatch))
                {
                    repLen = (ZSTD_count_2segments(
                        ip.wrapping_add(minMatch as usize),
                        repMatch.wrapping_add(minMatch as usize),
                        iLimit,
                        dictEnd,
                        prefixStart,
                    ) as U32)
                        .wrapping_add(minMatch);
                }
                if dictMode == ZSTD_dictMatchState
                    && ((repOffset.wrapping_sub(1)
                        < curr.wrapping_sub(dmsLowLimit.wrapping_add(dmsIndexDelta)))
                        & (ZSTD_index_overlap_check(dictLimit, repIndex) != 0))
                    && (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(repMatch as *const c_void, minMatch))
                {
                    repLen = (ZSTD_count_2segments(
                        ip.wrapping_add(minMatch as usize),
                        repMatch.wrapping_add(minMatch as usize),
                        iLimit,
                        dmsEnd,
                        prefixStart,
                    ) as U32)
                        .wrapping_add(minMatch);
                }
            }
            /* save longer solution */
            if (repLen as usize) > bestLength {
                bestLength = repLen as usize;
                /* expect value between 1 and 3 */
                (*matches.add(mnum as usize)).off =
                    REPCODE_TO_OFFBASE(repCode.wrapping_sub(ll0).wrapping_add(1));
                (*matches.add(mnum as usize)).len = repLen;
                mnum = mnum.wrapping_add(1);
                if (repLen > sufficient_len) | (ip.wrapping_add(repLen as usize) == iLimit) {
                    /* best possible */
                    return mnum;
                }
            }
            repCode = repCode.wrapping_add(1);
        }
    }

    /* HC3 match finder */
    if (mls == 3) && (bestLength < mls as usize) {
        let matchIndex3: U32 = ZSTD_insertAndFindFirstIndexHash3(ms, nextToUpdate3, ip);
        if (matchIndex3 >= matchLow)
            & (curr.wrapping_sub(matchIndex3) < (1u32 << 18))
        /* heuristic : longer distance likely too expensive */
        {
            let mlen: usize;
            if (dictMode == ZSTD_noDict)
                || (dictMode == ZSTD_dictMatchState)
                || (matchIndex3 >= dictLimit)
            {
                let match_: *const BYTE = base.wrapping_add(matchIndex3 as usize);
                mlen = ZSTD_count(ip, match_, iLimit);
            } else {
                let match_: *const BYTE = dictBase.wrapping_add(matchIndex3 as usize);
                mlen = ZSTD_count_2segments(ip, match_, iLimit, dictEnd, prefixStart);
            }

            /* save best solution */
            if mlen >= mls as usize {
                bestLength = mlen;
                (*matches.add(0)).off = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex3));
                (*matches.add(0)).len = mlen as U32;
                mnum = 1;
                if (mlen > sufficient_len as usize) | (ip.wrapping_add(mlen) == iLimit) {
                    /* best possible length */
                    (*ms).nextToUpdate = curr.wrapping_add(1); /* skip insertion */
                    return 1;
                }
            }
        }
        /* no dictMatchState lookup: dicts don't have a populated HC3 table */
    } /* if (mls == 3) */

    *hashTable.add(h) = curr; /* Update Hash Table */

    while nbCompares != 0 && matchIndex >= matchLow {
        let nextPtr: *mut U32 = bt.add((2u32.wrapping_mul(matchIndex & btMask)) as usize);
        let mut match_: *const BYTE;
        /* guaranteed minimum nb of common bytes */
        let mut matchLength: usize = if commonLengthSmaller < commonLengthLarger {
            commonLengthSmaller
        } else {
            commonLengthLarger
        };

        if (dictMode == ZSTD_noDict)
            || (dictMode == ZSTD_dictMatchState)
            || ((matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize)
        {
            match_ = base.wrapping_add(matchIndex as usize);
            matchLength = matchLength.wrapping_add(ZSTD_count(
                ip.wrapping_add(matchLength),
                match_.wrapping_add(matchLength),
                iLimit,
            ));
        } else {
            match_ = dictBase.wrapping_add(matchIndex as usize);
            matchLength = matchLength.wrapping_add(ZSTD_count_2segments(
                ip.wrapping_add(matchLength),
                match_.wrapping_add(matchLength),
                iLimit,
                dictEnd,
                prefixStart,
            ));
            if (matchIndex as usize).wrapping_add(matchLength) >= dictLimit as usize {
                match_ = base.wrapping_add(matchIndex as usize); /* prepare for match[matchLength] read */
            }
        }

        if matchLength > bestLength {
            if matchLength > matchEndIdx.wrapping_sub(matchIndex) as usize {
                matchEndIdx = matchIndex.wrapping_add(matchLength as U32);
            }
            bestLength = matchLength;
            (*matches.add(mnum as usize)).off = OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex));
            (*matches.add(mnum as usize)).len = matchLength as U32;
            mnum = mnum.wrapping_add(1);
            if (matchLength > ZSTD_OPT_NUM) | (ip.wrapping_add(matchLength) == iLimit) {
                /* equal : no way to know if inf or sup */
                if dictMode == ZSTD_dictMatchState {
                    nbCompares = 0; /* break should also skip searching dms */
                }
                break; /* drop, to preserve bt consistency (miss a little bit of compression) */
            }
        }

        if *match_.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
            /* match smaller than current */
            *smallerPtr = matchIndex; /* update smaller idx */
            commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
            if matchIndex <= btLow {
                smallerPtr = addr_of_mut!(dummy32);
                break;
            } /* beyond tree size, stop the search */
            smallerPtr = nextPtr.add(1); /* new candidate => larger than match, which was smaller than current */
            matchIndex = *nextPtr.add(1); /* new matchIndex, larger than previous, closer to current */
        } else {
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = addr_of_mut!(dummy32);
                break;
            } /* beyond tree size, stop the search */
            largerPtr = nextPtr;
            matchIndex = *nextPtr.add(0);
        }

        nbCompares = nbCompares.wrapping_sub(1);
    }

    *largerPtr = 0;
    *smallerPtr = 0;

    if dictMode == ZSTD_dictMatchState && nbCompares != 0 {
        let dmsH: usize = ZSTD_hashPtr(ip as *const c_void, dmsHashLog, mls);
        let mut dictMatchIndex: U32 = *(*dms).hashTable.add(dmsH);
        let dmsBt: *const U32 = (*dms).chainTable as *const U32;
        commonLengthSmaller = 0;
        commonLengthLarger = 0;
        while nbCompares != 0 && dictMatchIndex > dmsLowLimit {
            let nextPtr: *const U32 =
                dmsBt.add((2u32.wrapping_mul(dictMatchIndex & dmsBtMask)) as usize);
            /* guaranteed minimum nb of common bytes */
            let mut matchLength: usize = if commonLengthSmaller < commonLengthLarger {
                commonLengthSmaller
            } else {
                commonLengthLarger
            };
            let mut match_: *const BYTE = dmsBase.wrapping_add(dictMatchIndex as usize);
            matchLength = matchLength.wrapping_add(ZSTD_count_2segments(
                ip.wrapping_add(matchLength),
                match_.wrapping_add(matchLength),
                iLimit,
                dmsEnd,
                prefixStart,
            ));
            if (dictMatchIndex as usize).wrapping_add(matchLength) >= dmsHighLimit as usize {
                /* to prepare for next usage of match[matchLength] */
                match_ = base
                    .wrapping_add(dictMatchIndex as usize)
                    .wrapping_add(dmsIndexDelta as usize);
            }

            if matchLength > bestLength {
                matchIndex = dictMatchIndex.wrapping_add(dmsIndexDelta);
                if matchLength > matchEndIdx.wrapping_sub(matchIndex) as usize {
                    matchEndIdx = matchIndex.wrapping_add(matchLength as U32);
                }
                bestLength = matchLength;
                (*matches.add(mnum as usize)).off =
                    OFFSET_TO_OFFBASE(curr.wrapping_sub(matchIndex));
                (*matches.add(mnum as usize)).len = matchLength as U32;
                mnum = mnum.wrapping_add(1);
                if (matchLength > ZSTD_OPT_NUM) | (ip.wrapping_add(matchLength) == iLimit) {
                    /* equal : no way to know if inf or sup */
                    break; /* drop, to guarantee consistency (miss a little bit of compression) */
                }
            }

            if dictMatchIndex <= dmsBtLow {
                break;
            } /* beyond tree size, stop the search */
            if *match_.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
                commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
                dictMatchIndex = *nextPtr.add(1); /* new matchIndex larger than previous (closer to current) */
            } else {
                /* match is larger than current */
                commonLengthLarger = matchLength;
                dictMatchIndex = *nextPtr.add(0);
            }

            nbCompares = nbCompares.wrapping_sub(1);
        }
    } /* if (dictMode == ZSTD_dictMatchState) */

    (*ms).nextToUpdate = matchEndIdx.wrapping_sub(8); /* skip repetitive patterns */
    mnum
}

type ZSTD_getAllMatchesFn = unsafe extern "C" fn(
    *mut ZSTD_match_t,
    *mut ZSTD_MatchState_t,
    *mut U32,
    *const BYTE,
    *const BYTE,
    *const U32,
    U32,
    U32,
) -> U32;

unsafe fn ZSTD_btGetAllMatches_internal(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
    dictMode: ZSTD_dictMode_e,
    mls: U32,
) -> U32 {
    if ip < (*ms).window.base.wrapping_add((*ms).nextToUpdate as usize) {
        return 0; /* skipped area */
    }
    ZSTD_updateTree_internal(ms, ip, iHighLimit, mls, dictMode);
    ZSTD_insertBtAndGetAllMatches(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        dictMode,
        rep,
        ll0,
        lengthToBeat,
        mls,
    )
}

/* Expansion of GEN_ZSTD_BT_GET_ALL_MATCHES(noDict/extDict/dictMatchState) */

unsafe extern "C" fn ZSTD_btGetAllMatches_noDict_3(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_noDict,
        3,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_noDict_4(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_noDict,
        4,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_noDict_5(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_noDict,
        5,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_noDict_6(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_noDict,
        6,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_extDict_3(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_extDict,
        3,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_extDict_4(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_extDict,
        4,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_extDict_5(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_extDict,
        5,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_extDict_6(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_extDict,
        6,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_dictMatchState_3(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_dictMatchState,
        3,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_dictMatchState_4(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_dictMatchState,
        4,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_dictMatchState_5(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_dictMatchState,
        5,
    )
}

unsafe extern "C" fn ZSTD_btGetAllMatches_dictMatchState_6(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iHighLimit: *const BYTE,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
) -> U32 {
    ZSTD_btGetAllMatches_internal(
        matches,
        ms,
        nextToUpdate3,
        ip,
        iHighLimit,
        rep,
        ll0,
        lengthToBeat,
        ZSTD_dictMatchState,
        6,
    )
}

static getAllMatchesFns: [[ZSTD_getAllMatchesFn; 4]; 3] = [
    [
        ZSTD_btGetAllMatches_noDict_3,
        ZSTD_btGetAllMatches_noDict_4,
        ZSTD_btGetAllMatches_noDict_5,
        ZSTD_btGetAllMatches_noDict_6,
    ],
    [
        ZSTD_btGetAllMatches_extDict_3,
        ZSTD_btGetAllMatches_extDict_4,
        ZSTD_btGetAllMatches_extDict_5,
        ZSTD_btGetAllMatches_extDict_6,
    ],
    [
        ZSTD_btGetAllMatches_dictMatchState_3,
        ZSTD_btGetAllMatches_dictMatchState_4,
        ZSTD_btGetAllMatches_dictMatchState_5,
        ZSTD_btGetAllMatches_dictMatchState_6,
    ],
];

unsafe fn ZSTD_selectBtGetAllMatches(
    ms: *const ZSTD_MatchState_t,
    dictMode: ZSTD_dictMode_e,
) -> ZSTD_getAllMatchesFn {
    /* BOUNDED(3, ms->cParams.minMatch, 6) */
    let minMatch: U32 = (*ms).cParams.minMatch;
    let mls: U32 = {
        let lo = if minMatch < 6 { minMatch } else { 6 };
        if 3 > lo {
            3
        } else {
            lo
        }
    };
    getAllMatchesFns[dictMode as usize][(mls - 3) as usize]
}

/*************************
*  LDM helper functions  *
*************************/

/* Struct containing info needed to make decision about ldm inclusion */
#[repr(C)]
#[derive(Copy, Clone)]
struct ZSTD_optLdm_t {
    /// External match candidates store for this block
    seqStore: RawSeqStore_t,
    /// Start position of the current match candidate
    startPosInBlock: U32,
    /// End position of the current match candidate
    endPosInBlock: U32,
    /// Offset of the match candidate
    offset: U32,
}

/* ZSTD_optLdm_skipRawSeqStoreBytes():
 * Moves forward in @rawSeqStore by @nbBytes,
 * which will update the fields 'pos' and 'posInSequence'.
 */
unsafe fn ZSTD_optLdm_skipRawSeqStoreBytes(rawSeqStore: *mut RawSeqStore_t, nbBytes: usize) {
    let mut currPos: U32 = ((*rawSeqStore).posInSequence.wrapping_add(nbBytes)) as U32;
    while currPos != 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let currSeq: rawSeq = *(*rawSeqStore).seq.add((*rawSeqStore).pos);
        if currPos >= currSeq.litLength.wrapping_add(currSeq.matchLength) {
            currPos =
                currPos.wrapping_sub(currSeq.litLength.wrapping_add(currSeq.matchLength));
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

/* ZSTD_opt_getNextMatchAndUpdateSeqStore():
 * Calculates the beginning and end of the next match in the current block.
 * Updates 'pos' and 'posInSequence' of the ldmSeqStore.
 */
unsafe fn ZSTD_opt_getNextMatchAndUpdateSeqStore(
    optLdm: *mut ZSTD_optLdm_t,
    currPosInBlock: U32,
    blockBytesRemaining: U32,
) {
    let currSeq: rawSeq;
    let currBlockEndPos: U32;
    let literalsBytesRemaining: U32;
    let matchBytesRemaining: U32;

    /* Setting match end position to MAX to ensure we never use an LDM during this block */
    if (*optLdm).seqStore.size == 0 || (*optLdm).seqStore.pos >= (*optLdm).seqStore.size {
        (*optLdm).startPosInBlock = u32::MAX;
        (*optLdm).endPosInBlock = u32::MAX;
        return;
    }
    /* Calculate appropriate bytes left in matchLength and litLength
     * after adjusting based on ldmSeqStore->posInSequence */
    currSeq = *(*optLdm).seqStore.seq.add((*optLdm).seqStore.pos);
    currBlockEndPos = currPosInBlock.wrapping_add(blockBytesRemaining);
    literalsBytesRemaining = if (*optLdm).seqStore.posInSequence < currSeq.litLength as usize {
        currSeq
            .litLength
            .wrapping_sub((*optLdm).seqStore.posInSequence as U32)
    } else {
        0
    };
    matchBytesRemaining = if literalsBytesRemaining == 0 {
        currSeq.matchLength.wrapping_sub(
            ((*optLdm).seqStore.posInSequence as U32).wrapping_sub(currSeq.litLength),
        )
    } else {
        currSeq.matchLength
    };

    /* If there are more literal bytes than bytes remaining in block, no ldm is possible */
    if literalsBytesRemaining >= blockBytesRemaining {
        (*optLdm).startPosInBlock = u32::MAX;
        (*optLdm).endPosInBlock = u32::MAX;
        ZSTD_optLdm_skipRawSeqStoreBytes(
            addr_of_mut!((*optLdm).seqStore),
            blockBytesRemaining as usize,
        );
        return;
    }

    /* Matches may be < minMatch by this process. In that case, we will reject them
    when we are deciding whether or not to add the ldm */
    (*optLdm).startPosInBlock = currPosInBlock.wrapping_add(literalsBytesRemaining);
    (*optLdm).endPosInBlock = (*optLdm).startPosInBlock.wrapping_add(matchBytesRemaining);
    (*optLdm).offset = currSeq.offset;

    if (*optLdm).endPosInBlock > currBlockEndPos {
        /* Match ends after the block ends, we can't use the whole match */
        (*optLdm).endPosInBlock = currBlockEndPos;
        ZSTD_optLdm_skipRawSeqStoreBytes(
            addr_of_mut!((*optLdm).seqStore),
            currBlockEndPos.wrapping_sub(currPosInBlock) as usize,
        );
    } else {
        /* Consume nb of bytes equal to size of sequence left */
        ZSTD_optLdm_skipRawSeqStoreBytes(
            addr_of_mut!((*optLdm).seqStore),
            literalsBytesRemaining.wrapping_add(matchBytesRemaining) as usize,
        );
    }
}

/* ZSTD_optLdm_maybeAddMatch():
 * Adds a match if it's long enough,
 * based on it's 'matchStartPosInBlock' and 'matchEndPosInBlock',
 * into 'matches'. Maintains the correct ordering of 'matches'.
 */
unsafe fn ZSTD_optLdm_maybeAddMatch(
    matches: *mut ZSTD_match_t,
    nbMatches: *mut U32,
    optLdm: *const ZSTD_optLdm_t,
    currPosInBlock: U32,
    minMatch: U32,
) {
    let posDiff: U32 = currPosInBlock.wrapping_sub((*optLdm).startPosInBlock);
    /* Note: ZSTD_match_t actually contains offBase and matchLength (before subtracting MINMATCH) */
    let candidateMatchLength: U32 = (*optLdm)
        .endPosInBlock
        .wrapping_sub((*optLdm).startPosInBlock)
        .wrapping_sub(posDiff);

    /* Ensure that current block position is not outside of the match */
    if currPosInBlock < (*optLdm).startPosInBlock
        || currPosInBlock >= (*optLdm).endPosInBlock
        || candidateMatchLength < minMatch
    {
        return;
    }

    if *nbMatches == 0
        || ((candidateMatchLength > (*matches.add((*nbMatches).wrapping_sub(1) as usize)).len)
            && (*nbMatches) < ZSTD_OPT_NUM as U32)
    {
        let candidateOffBase: U32 = OFFSET_TO_OFFBASE((*optLdm).offset);
        (*matches.add(*nbMatches as usize)).len = candidateMatchLength;
        (*matches.add(*nbMatches as usize)).off = candidateOffBase;
        *nbMatches = (*nbMatches).wrapping_add(1);
    }
}

/* ZSTD_optLdm_processMatchCandidate():
 * Wrapper function to update ldm seq store and call ldm functions as necessary.
 */
unsafe fn ZSTD_optLdm_processMatchCandidate(
    optLdm: *mut ZSTD_optLdm_t,
    matches: *mut ZSTD_match_t,
    nbMatches: *mut U32,
    currPosInBlock: U32,
    remainingBytes: U32,
    minMatch: U32,
) {
    if (*optLdm).seqStore.size == 0 || (*optLdm).seqStore.pos >= (*optLdm).seqStore.size {
        return;
    }

    if currPosInBlock >= (*optLdm).endPosInBlock {
        if currPosInBlock > (*optLdm).endPosInBlock {
            /* The position at which ZSTD_optLdm_processMatchCandidate() is called is not necessarily
             * at the end of a match from the ldm seq store, and will often be some bytes
             * over beyond matchEndPosInBlock. As such, we need to correct for these "overshoots"
             */
            let posOvershoot: U32 = currPosInBlock.wrapping_sub((*optLdm).endPosInBlock);
            ZSTD_optLdm_skipRawSeqStoreBytes(
                addr_of_mut!((*optLdm).seqStore),
                posOvershoot as usize,
            );
        }
        ZSTD_opt_getNextMatchAndUpdateSeqStore(optLdm, currPosInBlock, remainingBytes);
    }
    ZSTD_optLdm_maybeAddMatch(matches, nbMatches, optLdm, currPosInBlock, minMatch);
}

/*-*******************************
*  Optimal parser
*********************************/

unsafe fn ZSTD_compressBlock_opt_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    optLevel: c_int,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    let optStatePtr: *mut optState_t = addr_of_mut!((*ms).opt);
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(8);
    let base: *const BYTE = (*ms).window.base;
    let prefixStart: *const BYTE = base.wrapping_add((*ms).window.dictLimit as usize);
    let cParams: *const ZSTD_compressionParameters = addr_of!((*ms).cParams);

    let getAllMatches: ZSTD_getAllMatchesFn = ZSTD_selectBtGetAllMatches(ms, dictMode);

    let sufficient_len: U32 = if (*cParams).targetLength < (ZSTD_OPT_NUM - 1) as U32 {
        (*cParams).targetLength
    } else {
        (ZSTD_OPT_NUM - 1) as U32
    };
    let minMatch: U32 = if (*cParams).minMatch == 3 { 3 } else { 4 };
    let mut nextToUpdate3: U32 = (*ms).nextToUpdate;

    let opt: *mut ZSTD_optimal_t = (*optStatePtr).priceTable;
    let matches: *mut ZSTD_match_t = (*optStatePtr).matchTable;
    let mut lastStretch = ZSTD_optimal_t::default();
    let mut optLdm = ZSTD_optLdm_t {
        seqStore: RawSeqStore_t::default(),
        startPosInBlock: 0,
        endPosInBlock: 0,
        offset: 0,
    };

    ZSTD_memset(
        addr_of_mut!(lastStretch) as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_optimal_t>(),
    );

    /* `LIT_PRICE`/`LL_PRICE`/`LL_INCPRICE` macros */
    let LIT_PRICE = |p: *const BYTE| -> c_int {
        ZSTD_rawLiteralsCost(p, 1, optStatePtr, optLevel) as c_int
    };
    let LL_PRICE =
        |l: U32| -> c_int { ZSTD_litLengthPrice(l, optStatePtr, optLevel) as c_int };
    let LL_INCPRICE =
        |l: U32| -> c_int { LL_PRICE(l).wrapping_sub(LL_PRICE(l.wrapping_sub(1))) };

    optLdm.seqStore = if !(*ms).ldmSeqStore.is_null() {
        *(*ms).ldmSeqStore
    } else {
        kNullRawSeqStore
    };
    optLdm.offset = 0;
    optLdm.startPosInBlock = 0;
    optLdm.endPosInBlock = 0;
    ZSTD_opt_getNextMatchAndUpdateSeqStore(
        addr_of_mut!(optLdm),
        (ip as usize).wrapping_sub(istart as usize) as U32,
        (iend as usize).wrapping_sub(ip as usize) as U32,
    );

    /* init */
    ZSTD_rescaleFreqs(optStatePtr, src as *const BYTE, srcSize, optLevel);
    ip = ip.wrapping_add((ip == prefixStart) as usize);

    /* Match Loop */
    'outer: while ip < ilimit {
        let mut cur: U32 = 0;
        let mut last_pos: U32 = 0;

        'shortestPath: {
            /* find first match */
            {
                let litlen: U32 = (ip as usize).wrapping_sub(anchor as usize) as U32;
                let ll0: U32 = (litlen == 0) as U32;
                let mut nbMatches: U32 = getAllMatches(
                    matches,
                    ms,
                    addr_of_mut!(nextToUpdate3),
                    ip,
                    iend,
                    rep as *const U32,
                    ll0,
                    minMatch,
                );
                ZSTD_optLdm_processMatchCandidate(
                    addr_of_mut!(optLdm),
                    matches,
                    addr_of_mut!(nbMatches),
                    (ip as usize).wrapping_sub(istart as usize) as U32,
                    (iend as usize).wrapping_sub(ip as usize) as U32,
                    minMatch,
                );
                if nbMatches == 0 {
                    ip = ip.wrapping_add(1);
                    continue 'outer;
                }

                /* Match found: let's store this solution, and eventually find more candidates.
                 * During this forward pass, @opt is used to store stretches,
                 * defined as "a match followed by N literals".
                 * Note how this is different from a Sequence, which is "N literals followed by a match".
                 * Storing stretches allows us to store different match predecessors
                 * for each literal position part of a literals run. */

                /* initialize opt[0] */
                (*opt.add(0)).mlen = 0; /* there are only literals so far */
                (*opt.add(0)).litlen = litlen;
                /* No need to include the actual price of the literals before the first match
                 * because it is static for the duration of the forward pass, and is included
                 * in every subsequent price. But, we include the literal length because
                 * the cost variation of litlen depends on the value of litlen.
                 */
                (*opt.add(0)).price = LL_PRICE(litlen);
                ZSTD_memcpy(
                    addr_of_mut!((*opt.add(0)).rep) as *mut c_void,
                    rep as *const c_void,
                    core::mem::size_of::<[U32; ZSTD_REP_NUM]>(),
                );

                /* large match -> immediate encoding */
                {
                    let maxML: U32 = (*matches.add(nbMatches.wrapping_sub(1) as usize)).len;
                    let maxOffBase: U32 = (*matches.add(nbMatches.wrapping_sub(1) as usize)).off;

                    if maxML > sufficient_len {
                        lastStretch.litlen = 0;
                        lastStretch.mlen = maxML;
                        lastStretch.off = maxOffBase;
                        cur = 0;
                        last_pos = maxML;
                        break 'shortestPath;
                    }
                }

                /* set prices for first matches starting position == 0 */
                {
                    let mut pos: U32 = 1;
                    while pos < minMatch {
                        (*opt.add(pos as usize)).price = ZSTD_MAX_PRICE;
                        (*opt.add(pos as usize)).mlen = 0;
                        (*opt.add(pos as usize)).litlen = litlen.wrapping_add(pos);
                        pos = pos.wrapping_add(1);
                    }
                    let mut matchNb: U32 = 0;
                    while matchNb < nbMatches {
                        let offBase: U32 = (*matches.add(matchNb as usize)).off;
                        let end: U32 = (*matches.add(matchNb as usize)).len;
                        while pos <= end {
                            let matchPrice: c_int =
                                ZSTD_getMatchPrice(offBase, pos, optStatePtr, optLevel) as c_int;
                            let sequencePrice: c_int =
                                (*opt.add(0)).price.wrapping_add(matchPrice);
                            (*opt.add(pos as usize)).mlen = pos;
                            (*opt.add(pos as usize)).off = offBase;
                            (*opt.add(pos as usize)).litlen = 0; /* end of match */
                            (*opt.add(pos as usize)).price =
                                sequencePrice.wrapping_add(LL_PRICE(0));
                            pos = pos.wrapping_add(1);
                        }
                        matchNb = matchNb.wrapping_add(1);
                    }
                    last_pos = pos.wrapping_sub(1);
                    (*opt.add(pos as usize)).price = ZSTD_MAX_PRICE;
                }
            }

            /* check further positions */
            cur = 1;
            'curLoop: while cur <= last_pos {
                'nextCur: {
                    let inr: *const BYTE = ip.wrapping_add(cur as usize);

                    /* Fix current position with one literal if cheaper */
                    {
                        let litlen: U32 =
                            (*opt.add(cur.wrapping_sub(1) as usize)).litlen.wrapping_add(1);
                        let price: c_int = (*opt.add(cur.wrapping_sub(1) as usize))
                            .price
                            .wrapping_add(LIT_PRICE(ip.wrapping_add(cur.wrapping_sub(1) as usize)))
                            .wrapping_add(LL_INCPRICE(litlen));
                        if price <= (*opt.add(cur as usize)).price {
                            let prevMatch: ZSTD_optimal_t = *opt.add(cur as usize);
                            *opt.add(cur as usize) = *opt.add(cur.wrapping_sub(1) as usize);
                            (*opt.add(cur as usize)).litlen = litlen;
                            (*opt.add(cur as usize)).price = price;
                            if (optLevel >= 1) /* additional check only for higher modes */
                                && (prevMatch.litlen == 0) /* replace a match */
                                && (LL_INCPRICE(1) < 0) /* ll1 is cheaper than ll0 */
                                && (ip.wrapping_add(cur as usize) < iend)
                            {
                                /* check next position, in case it would be cheaper */
                                let with1literal: c_int = prevMatch
                                    .price
                                    .wrapping_add(LIT_PRICE(ip.wrapping_add(cur as usize)))
                                    .wrapping_add(LL_INCPRICE(1));
                                let withMoreLiterals: c_int = price
                                    .wrapping_add(LIT_PRICE(ip.wrapping_add(cur as usize)))
                                    .wrapping_add(LL_INCPRICE(litlen.wrapping_add(1)));
                                if (with1literal < withMoreLiterals)
                                    && (with1literal
                                        < (*opt.add(cur.wrapping_add(1) as usize)).price)
                                {
                                    /* update offset history - before it disappears */
                                    let prev: U32 = cur.wrapping_sub(prevMatch.mlen);
                                    let newReps: Repcodes_t = ZSTD_newRep(
                                        addr_of!((*opt.add(prev as usize)).rep) as *const U32,
                                        prevMatch.off,
                                        ((*opt.add(prev as usize)).litlen == 0) as U32,
                                    );
                                    *opt.add(cur.wrapping_add(1) as usize) = prevMatch; /* mlen & offbase */
                                    ZSTD_memcpy(
                                        addr_of_mut!((*opt.add(cur.wrapping_add(1) as usize)).rep)
                                            as *mut c_void,
                                        addr_of!(newReps) as *const c_void,
                                        core::mem::size_of::<Repcodes_t>(),
                                    );
                                    (*opt.add(cur.wrapping_add(1) as usize)).litlen = 1;
                                    (*opt.add(cur.wrapping_add(1) as usize)).price = with1literal;
                                    if last_pos < cur.wrapping_add(1) {
                                        last_pos = cur.wrapping_add(1);
                                    }
                                }
                            }
                        }
                    }

                    /* Offset history is not updated during match comparison.
                     * Do it here, now that the match is selected and confirmed.
                     */
                    if (*opt.add(cur as usize)).litlen == 0 {
                        /* just finished a match => alter offset history */
                        let prev: U32 = cur.wrapping_sub((*opt.add(cur as usize)).mlen);
                        let newReps: Repcodes_t = ZSTD_newRep(
                            addr_of!((*opt.add(prev as usize)).rep) as *const U32,
                            (*opt.add(cur as usize)).off,
                            ((*opt.add(prev as usize)).litlen == 0) as U32,
                        );
                        ZSTD_memcpy(
                            addr_of_mut!((*opt.add(cur as usize)).rep) as *mut c_void,
                            addr_of!(newReps) as *const c_void,
                            core::mem::size_of::<Repcodes_t>(),
                        );
                    }

                    /* last match must start at a minimum distance of 8 from oend */
                    if inr > ilimit {
                        break 'nextCur;
                    }

                    if cur == last_pos {
                        break 'curLoop;
                    }

                    if (optLevel == 0)
                        && ((*opt.add(cur.wrapping_add(1) as usize)).price
                            <= (*opt.add(cur as usize))
                                .price
                                .wrapping_add((BITCOST_MULTIPLIER / 2) as c_int))
                    {
                        /* skip unpromising positions; about ~+6% speed, -0.01 ratio */
                        break 'nextCur;
                    }

                    {
                        let ll0: U32 = ((*opt.add(cur as usize)).litlen == 0) as U32;
                        let previousPrice: c_int = (*opt.add(cur as usize)).price;
                        let basePrice: c_int = previousPrice.wrapping_add(LL_PRICE(0));
                        let mut nbMatches: U32 = getAllMatches(
                            matches,
                            ms,
                            addr_of_mut!(nextToUpdate3),
                            inr,
                            iend,
                            addr_of!((*opt.add(cur as usize)).rep) as *const U32,
                            ll0,
                            minMatch,
                        );

                        ZSTD_optLdm_processMatchCandidate(
                            addr_of_mut!(optLdm),
                            matches,
                            addr_of_mut!(nbMatches),
                            (inr as usize).wrapping_sub(istart as usize) as U32,
                            (iend as usize).wrapping_sub(inr as usize) as U32,
                            minMatch,
                        );

                        if nbMatches == 0 {
                            break 'nextCur;
                        }

                        {
                            let longestML: U32 =
                                (*matches.add(nbMatches.wrapping_sub(1) as usize)).len;

                            if (longestML > sufficient_len)
                                || ((cur as usize).wrapping_add(longestML as usize)
                                    >= ZSTD_OPT_NUM)
                                || (ip
                                    .wrapping_add(cur as usize)
                                    .wrapping_add(longestML as usize)
                                    >= iend)
                            {
                                lastStretch.mlen = longestML;
                                lastStretch.off =
                                    (*matches.add(nbMatches.wrapping_sub(1) as usize)).off;
                                lastStretch.litlen = 0;
                                last_pos = cur.wrapping_add(longestML);
                                break 'shortestPath;
                            }
                        }

                        /* set prices using matches found at position == cur */
                        let mut matchNb: U32 = 0;
                        while matchNb < nbMatches {
                            let offset: U32 = (*matches.add(matchNb as usize)).off;
                            let lastML: U32 = (*matches.add(matchNb as usize)).len;
                            let startML: U32 = if matchNb > 0 {
                                (*matches.add(matchNb.wrapping_sub(1) as usize))
                                    .len
                                    .wrapping_add(1)
                            } else {
                                minMatch
                            };

                            let mut mlen: U32 = lastML;
                            while mlen >= startML {
                                /* scan downward */
                                let pos: U32 = cur.wrapping_add(mlen);
                                let price: c_int = basePrice.wrapping_add(ZSTD_getMatchPrice(
                                    offset,
                                    mlen,
                                    optStatePtr,
                                    optLevel,
                                ) as c_int);

                                if (pos > last_pos) || (price < (*opt.add(pos as usize)).price) {
                                    while last_pos < pos {
                                        /* fill empty positions, for future comparisons */
                                        last_pos = last_pos.wrapping_add(1);
                                        (*opt.add(last_pos as usize)).price = ZSTD_MAX_PRICE;
                                        /* just needs to be != 0, to mean "not an end of match" */
                                        (*opt.add(last_pos as usize)).litlen = 1;
                                    }
                                    (*opt.add(pos as usize)).mlen = mlen;
                                    (*opt.add(pos as usize)).off = offset;
                                    (*opt.add(pos as usize)).litlen = 0;
                                    (*opt.add(pos as usize)).price = price;
                                } else {
                                    if optLevel == 0 {
                                        /* early update abort; gets ~+10% speed for about -0.01 ratio loss */
                                        break;
                                    }
                                }
                                mlen = mlen.wrapping_sub(1);
                            }
                            matchNb = matchNb.wrapping_add(1);
                        }
                    }
                    (*opt.add(last_pos.wrapping_add(1) as usize)).price = ZSTD_MAX_PRICE;
                }
                cur = cur.wrapping_add(1);
            } /* for (cur = 1; cur <= last_pos; cur++) */

            lastStretch = *opt.add(last_pos as usize);
            cur = last_pos.wrapping_sub(lastStretch.mlen);
        }

        /* _shortestPath:   cur, last_pos, best_mlen, best_off have to be set */

        if lastStretch.mlen == 0 {
            /* no solution : all matches have been converted into literals */
            ip = ip.wrapping_add(last_pos as usize);
            continue 'outer;
        }

        /* Update offset history */
        if lastStretch.litlen == 0 {
            /* finishing on a match : update offset history */
            let reps: Repcodes_t = ZSTD_newRep(
                addr_of!((*opt.add(cur as usize)).rep) as *const U32,
                lastStretch.off,
                ((*opt.add(cur as usize)).litlen == 0) as U32,
            );
            ZSTD_memcpy(
                rep as *mut c_void,
                addr_of!(reps) as *const c_void,
                core::mem::size_of::<Repcodes_t>(),
            );
        } else {
            ZSTD_memcpy(
                rep as *mut c_void,
                lastStretch.rep.as_ptr() as *const c_void,
                core::mem::size_of::<Repcodes_t>(),
            );
            cur = cur.wrapping_sub(lastStretch.litlen);
        }

        /* Let's write the shortest path solution.
         * It is stored in @opt in reverse order,
         * starting from @storeEnd (==cur+2),
         * effectively partially @opt overwriting.
         * Content is changed too:
         * - So far, @opt stored stretches, aka a match followed by literals
         * - Now, it will store sequences, aka literals followed by a match
         */
        {
            let storeEnd: U32 = cur.wrapping_add(2);
            let mut storeStart: U32 = storeEnd;
            let mut stretchPos: U32 = cur;

            if lastStretch.litlen > 0 {
                /* last "sequence" is unfinished: just a bunch of literals */
                (*opt.add(storeEnd as usize)).litlen = lastStretch.litlen;
                (*opt.add(storeEnd as usize)).mlen = 0;
                storeStart = storeEnd.wrapping_sub(1);
                *opt.add(storeStart as usize) = lastStretch;
            }
            /* NOTE: the C source has `} {` here (not `} else {`), so this block
             * runs unconditionally. Transliterated verbatim. */
            {
                *opt.add(storeEnd as usize) = lastStretch; /* note: litlen will be fixed */
                storeStart = storeEnd;
            }
            loop {
                let nextStretch: ZSTD_optimal_t = *opt.add(stretchPos as usize);
                (*opt.add(storeStart as usize)).litlen = nextStretch.litlen;
                if nextStretch.mlen == 0 {
                    /* reaching beginning of segment */
                    break;
                }
                storeStart = storeStart.wrapping_sub(1);
                *opt.add(storeStart as usize) = nextStretch; /* note: litlen will be fixed */
                stretchPos = stretchPos
                    .wrapping_sub(nextStretch.litlen.wrapping_add(nextStretch.mlen));
            }

            /* save sequences */
            {
                let mut storePos: U32 = storeStart;
                while storePos <= storeEnd {
                    let llen: U32 = (*opt.add(storePos as usize)).litlen;
                    let mlen: U32 = (*opt.add(storePos as usize)).mlen;
                    let offBase: U32 = (*opt.add(storePos as usize)).off;
                    let advance: U32 = llen.wrapping_add(mlen);

                    if mlen == 0 {
                        /* only literals => must be last "sequence", actually starting a new stream of sequences */
                        /* last "sequence" is a bunch of literals => don't progress anchor */
                        ip = anchor.wrapping_add(llen as usize);
                        storePos = storePos.wrapping_add(1);
                        continue; /* will finish */
                    }

                    ZSTD_updateStats(optStatePtr, llen, anchor, offBase, mlen);
                    ZSTD_storeSeq(
                        seqStore,
                        llen as usize,
                        anchor,
                        iend,
                        offBase,
                        mlen as usize,
                    );
                    anchor = anchor.wrapping_add(advance as usize);
                    ip = anchor;

                    storePos = storePos.wrapping_add(1);
                }
            }

            /* update all costs */
            ZSTD_setBasePrices(optStatePtr, optLevel);
        }
    } /* while (ip < ilimit) */

    /* Return the last literals size */
    (iend as usize).wrapping_sub(anchor as usize)
}

unsafe fn ZSTD_compressBlock_opt0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    ZSTD_compressBlock_opt_generic(ms, seqStore, rep, src, srcSize, 0 /* optLevel */, dictMode)
}

unsafe fn ZSTD_compressBlock_opt2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    ZSTD_compressBlock_opt_generic(ms, seqStore, rep, src, srcSize, 2 /* optLevel */, dictMode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btopt(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_opt0(ms, seqStore, rep, src, srcSize, ZSTD_noDict)
}

/* ZSTD_initStats_ultra():
 * make a first compression pass, just to seed stats with more accurate starting values.
 * only works on first block, with no dictionary and no ldm.
 * this function cannot error out, its narrow contract must be respected.
 */
unsafe fn ZSTD_initStats_ultra(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) {
    let mut tmpRep: [U32; ZSTD_REP_NUM] = [0; ZSTD_REP_NUM]; /* updated rep codes will sink here */
    ZSTD_memcpy(
        tmpRep.as_mut_ptr() as *mut c_void,
        rep as *const c_void,
        core::mem::size_of::<[U32; ZSTD_REP_NUM]>(),
    );

    /* generate stats into ms->opt */
    ZSTD_compressBlock_opt2(
        ms,
        seqStore,
        tmpRep.as_mut_ptr(),
        src,
        srcSize,
        ZSTD_noDict,
    );

    /* invalidate first scan from history, only keep entropy stats */
    ZSTD_resetSeqStore(seqStore);
    (*ms).window.base = (*ms).window.base.wrapping_sub(srcSize);
    (*ms).window.dictLimit = (*ms).window.dictLimit.wrapping_add(srcSize as U32);
    (*ms).window.lowLimit = (*ms).window.dictLimit;
    (*ms).nextToUpdate = (*ms).window.dictLimit;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btultra(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_opt2(ms, seqStore, rep, src, srcSize, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btultra2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let curr: U32 = ((src as *const BYTE as usize).wrapping_sub((*ms).window.base as usize)) as U32;

    /* 2-passes strategy:
     * this strategy makes a first pass over first block to collect statistics
     * in order to seed next round's statistics with it.
     * After 1st pass, function forgets history, and starts a new block.
     * Consequently, this can only work if no data has been previously loaded in tables,
     * aka, no dictionary, no prefix, no ldm preprocessing.
     * The compression ratio gain is generally small (~0.5% on first block),
     * the cost is 2x cpu time on first block. */
    if ((*ms).opt.litLengthSum == 0) /* first block */
        && ((*seqStore).sequences == (*seqStore).sequencesStart) /* no ldm */
        && ((*ms).window.dictLimit == (*ms).window.lowLimit) /* no dictionary */
        && (curr == (*ms).window.dictLimit) /* start of frame, nothing already loaded nor skipped */
        && (srcSize > ZSTD_PREDEF_THRESHOLD)
    /* input large enough to not employ default stats */
    {
        ZSTD_initStats_ultra(ms, seqStore, rep, src, srcSize);
    }

    ZSTD_compressBlock_opt2(ms, seqStore, rep, src, srcSize, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btopt_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_opt0(ms, seqStore, rep, src, srcSize, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btopt_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_opt0(ms, seqStore, rep, src, srcSize, ZSTD_extDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btultra_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_opt2(ms, seqStore, rep, src, srcSize, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btultra_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_opt2(ms, seqStore, rep, src, srcSize, ZSTD_extDict)
}

/* note : no btultra2 variant for extDict nor dictMatchState,
 * because btultra2 is not meant to work with dictionaries
 * and is only specific for the first block (no prefix) */
