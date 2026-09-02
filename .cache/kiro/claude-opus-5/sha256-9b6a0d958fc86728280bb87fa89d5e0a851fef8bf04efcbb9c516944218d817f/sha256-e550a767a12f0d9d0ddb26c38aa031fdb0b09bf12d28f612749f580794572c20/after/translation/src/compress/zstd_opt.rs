//! Optimal parser (btopt / btultra / btultra2).
//!
//! Literal, semantics-preserving transliteration of `zstd_opt.c` + `zstd_opt.h`.
//! Build configuration: `DYNAMIC_BMI2=0`, no `ZSTD_MULTITHREAD`, `DEBUGLEVEL 0`
//! (asserts / DEBUGLOG dropped). None of the build exclusion macros
//! (`ZSTD_EXCLUDE_BT*_BLOCK_COMPRESSOR`) are defined, so all bodies compile.
//!
//! WEIGHT macro resolves to the `else` branch of the #if ladder:
//!   BITCOST_ACCURACY=8, BITCOST_MULTIPLIER=256,
//!   WEIGHT(stat,opt) = opt ? ZSTD_fracWeight(stat) : ZSTD_bitWeight(stat).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_assignments)]

use core::ffi::{c_int, c_uint, c_void};

use crate::common::bits::*;
use crate::common::fse::{FSE_CState_t, FSE_getMaxNbBits, FSE_initCState};
use crate::common::huf::HUF_repeat_valid;
use crate::common::mem::*;
use crate::common::zstd_h::{ZSTD_compressionParameters, ZSTD_ps_disable, ZSTD_BLOCKSIZE_MAX};
use crate::common::zstd_internal::*;

use crate::compress::hist::HIST_count_simple;
use crate::compress::huf_compress::HUF_getNbBitsFromCTable;
use crate::compress::zstd_compress_internal::*;

const ZSTD_LITFREQ_ADD: U32 = 2; /* scaling factor for litFreq, so that frequencies adapt faster to new stats */
const ZSTD_MAX_PRICE: c_int = 1 << 30;

const ZSTD_PREDEF_THRESHOLD: size_t = 8; /* if srcSize < ZSTD_PREDEF_THRESHOLD, symbols' cost is assumed static, directly determined by pre-defined distributions */

/*-*************************************
*  Price functions for optimal parser
***************************************/

const BITCOST_ACCURACY: U32 = 8;
const BITCOST_MULTIPLIER: U32 = 1 << BITCOST_ACCURACY;

/* WEIGHT(stat,opt) = (opt) ? ZSTD_fracWeight(stat) : ZSTD_bitWeight(stat) */
#[inline(always)]
unsafe fn WEIGHT(stat: U32, opt: c_int) -> U32 {
    if opt != 0 {
        ZSTD_fracWeight(stat)
    } else {
        ZSTD_bitWeight(stat)
    }
}

/* ZSTD_bitWeight() :
 * provide estimated "cost" of a stat in full bits only */
unsafe fn ZSTD_bitWeight(stat: U32) -> U32 {
    ZSTD_highbit32(stat.wrapping_add(1)).wrapping_mul(BITCOST_MULTIPLIER)
}

/* ZSTD_fracWeight() :
 * provide fractional-bit "cost" of a stat,
 * using linear interpolation approximation */
unsafe fn ZSTD_fracWeight(rawStat: U32) -> U32 {
    let stat: U32 = rawStat.wrapping_add(1);
    let hb: U32 = ZSTD_highbit32(stat);
    let BWeight: U32 = hb.wrapping_mul(BITCOST_MULTIPLIER);
    /* Fweight was meant for "Fractional weight"
     * but it's effectively a value between 1 and 2
     * using fixed point arithmetic */
    let FWeight: U32 = (stat << BITCOST_ACCURACY) >> hb;
    let weight: U32 = BWeight.wrapping_add(FWeight);
    weight
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

unsafe fn sum_u32(table: *const c_uint, nbElts: size_t) -> U32 {
    let mut n: size_t;
    let mut total: U32 = 0;
    n = 0;
    while n < nbElts {
        total = total.wrapping_add(*table.wrapping_add(n));
        n += 1;
    }
    total
}

pub type base_directive_e = c_uint;
pub const base_0possible: base_directive_e = 0;
pub const base_1guaranteed: base_directive_e = 1;

unsafe fn ZSTD_downscaleStats(
    table: *mut c_uint,
    lastEltIndex: U32,
    shift: U32,
    base1: base_directive_e,
) -> U32 {
    let mut s: U32;
    let mut sum: U32 = 0;
    s = 0;
    while s < lastEltIndex + 1 {
        let base: c_uint = if base1 != 0 {
            1
        } else {
            (*table.wrapping_add(s as usize) > 0) as c_uint
        };
        let newStat: c_uint = base.wrapping_add(*table.wrapping_add(s as usize) >> shift);
        sum = sum.wrapping_add(newStat);
        *table.wrapping_add(s as usize) = newStat;
        s += 1;
    }
    sum
}

/* ZSTD_scaleStats() :
 * reduce all elt frequencies in table if sum too large
 * return the resulting sum of elements */
unsafe fn ZSTD_scaleStats(table: *mut c_uint, lastEltIndex: U32, logTarget: U32) -> U32 {
    let prevsum: U32 = sum_u32(table, (lastEltIndex + 1) as size_t);
    let factor: U32 = prevsum >> logTarget;
    if factor <= 1 {
        return prevsum;
    }
    ZSTD_downscaleStats(table, lastEltIndex, ZSTD_highbit32(factor), base_1guaranteed)
}

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
    srcSize: size_t,
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
                let mut lit: c_uint;
                (*optPtr).litSum = 0;
                lit = 0;
                while lit <= MaxLit as c_uint {
                    let scaleLog: U32 = 11; /* scale to 2K */
                    let bitCost: U32 =
                        HUF_getNbBitsFromCTable((*(*optPtr).symbolCosts).huf.CTable.as_ptr(), lit);
                    *(*optPtr).litFreq.wrapping_add(lit as usize) = if bitCost != 0 {
                        1 << (scaleLog - bitCost)
                    } else {
                        1 /*minimum to calculate cost*/
                    };
                    (*optPtr).litSum = (*optPtr)
                        .litSum
                        .wrapping_add(*(*optPtr).litFreq.wrapping_add(lit as usize));
                    lit += 1;
                }
            }

            {
                let mut ll: c_uint;
                let mut llstate: FSE_CState_t = core::mem::zeroed();
                FSE_initCState(
                    &mut llstate,
                    (*(*optPtr).symbolCosts).fse.litlengthCTable.as_ptr(),
                );
                (*optPtr).litLengthSum = 0;
                ll = 0;
                while ll <= MaxLL as c_uint {
                    let scaleLog: U32 = 10; /* scale to 1K */
                    let bitCost: U32 = FSE_getMaxNbBits(llstate.symbolTT, ll);
                    *(*optPtr).litLengthFreq.wrapping_add(ll as usize) = if bitCost != 0 {
                        1 << (scaleLog - bitCost)
                    } else {
                        1 /*minimum to calculate cost*/
                    };
                    (*optPtr).litLengthSum = (*optPtr)
                        .litLengthSum
                        .wrapping_add(*(*optPtr).litLengthFreq.wrapping_add(ll as usize));
                    ll += 1;
                }
            }

            {
                let mut ml: c_uint;
                let mut mlstate: FSE_CState_t = core::mem::zeroed();
                FSE_initCState(
                    &mut mlstate,
                    (*(*optPtr).symbolCosts).fse.matchlengthCTable.as_ptr(),
                );
                (*optPtr).matchLengthSum = 0;
                ml = 0;
                while ml <= MaxML as c_uint {
                    let scaleLog: U32 = 10;
                    let bitCost: U32 = FSE_getMaxNbBits(mlstate.symbolTT, ml);
                    *(*optPtr).matchLengthFreq.wrapping_add(ml as usize) = if bitCost != 0 {
                        1 << (scaleLog - bitCost)
                    } else {
                        1 /*minimum to calculate cost*/
                    };
                    (*optPtr).matchLengthSum = (*optPtr)
                        .matchLengthSum
                        .wrapping_add(*(*optPtr).matchLengthFreq.wrapping_add(ml as usize));
                    ml += 1;
                }
            }

            {
                let mut of: c_uint;
                let mut ofstate: FSE_CState_t = core::mem::zeroed();
                FSE_initCState(
                    &mut ofstate,
                    (*(*optPtr).symbolCosts).fse.offcodeCTable.as_ptr(),
                );
                (*optPtr).offCodeSum = 0;
                of = 0;
                while of <= MaxOff as c_uint {
                    let scaleLog: U32 = 10;
                    let bitCost: U32 = FSE_getMaxNbBits(ofstate.symbolTT, of);
                    *(*optPtr).offCodeFreq.wrapping_add(of as usize) = if bitCost != 0 {
                        1 << (scaleLog - bitCost)
                    } else {
                        1 /*minimum to calculate cost*/
                    };
                    (*optPtr).offCodeSum = (*optPtr)
                        .offCodeSum
                        .wrapping_add(*(*optPtr).offCodeFreq.wrapping_add(of as usize));
                    of += 1;
                }
            }
        } else {
            /* first block, no dictionary */

            if compressedLiterals != 0 {
                /* base initial cost of literals on direct frequency within src */
                let mut lit: c_uint = MaxLit as c_uint;
                HIST_count_simple(
                    (*optPtr).litFreq,
                    &mut lit,
                    src as *const c_void,
                    srcSize,
                ); /* use raw first block to init statistics */
                (*optPtr).litSum =
                    ZSTD_downscaleStats((*optPtr).litFreq, MaxLit as U32, 8, base_0possible);
            }

            {
                let baseLLfreqs: [c_uint; (MaxLL + 1) as usize] = [
                    4, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1, 1, 1,
                ];
                ZSTD_memcpy(
                    (*optPtr).litLengthFreq as *mut u8,
                    baseLLfreqs.as_ptr() as *const u8,
                    core::mem::size_of_val(&baseLLfreqs) as size_t,
                );
                (*optPtr).litLengthSum = sum_u32(baseLLfreqs.as_ptr(), (MaxLL + 1) as size_t);
            }

            {
                let mut ml: c_uint;
                ml = 0;
                while ml <= MaxML as c_uint {
                    *(*optPtr).matchLengthFreq.wrapping_add(ml as usize) = 1;
                    ml += 1;
                }
            }
            (*optPtr).matchLengthSum = (MaxML + 1) as U32;

            {
                let baseOFCfreqs: [c_uint; (MaxOff + 1) as usize] = [
                    6, 2, 1, 1, 2, 3, 4, 4, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1,
                ];
                ZSTD_memcpy(
                    (*optPtr).offCodeFreq as *mut u8,
                    baseOFCfreqs.as_ptr() as *const u8,
                    core::mem::size_of_val(&baseOFCfreqs) as size_t,
                );
                (*optPtr).offCodeSum = sum_u32(baseOFCfreqs.as_ptr(), (MaxOff + 1) as size_t);
            }
        }
    } else {
        /* new block : scale down accumulated statistics */

        if compressedLiterals != 0 {
            (*optPtr).litSum = ZSTD_scaleStats((*optPtr).litFreq, MaxLit as U32, 12);
        }
        (*optPtr).litLengthSum = ZSTD_scaleStats((*optPtr).litLengthFreq, MaxLL as U32, 11);
        (*optPtr).matchLengthSum = ZSTD_scaleStats((*optPtr).matchLengthFreq, MaxML as U32, 11);
        (*optPtr).offCodeSum = ZSTD_scaleStats((*optPtr).offCodeFreq, MaxOff as U32, 11);
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
        return (litLength << 3).wrapping_mul(BITCOST_MULTIPLIER); /* Uncompressed - 8 bytes per literal. */
    }

    if (*optPtr).priceType == zop_predef {
        return (litLength.wrapping_mul(6)).wrapping_mul(BITCOST_MULTIPLIER); /* 6 bit per literal - no statistic used */
    }

    /* dynamic statistics */
    {
        let mut price: U32 = (*optPtr).litSumBasePrice.wrapping_mul(litLength);
        let litPriceMax: U32 = (*optPtr).litSumBasePrice.wrapping_sub(BITCOST_MULTIPLIER);
        let mut u: U32;
        u = 0;
        while u < litLength {
            let mut litPrice: U32 = WEIGHT(
                *(*optPtr)
                    .litFreq
                    .wrapping_add(*literals.wrapping_add(u as usize) as usize),
                optLevel,
            );
            if litPrice > litPriceMax {
                litPrice = litPriceMax;
            }
            price = price.wrapping_sub(litPrice);
            u += 1;
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
     * because it isn't representable in the zstd format. */
    if litLength == ZSTD_BLOCKSIZE_MAX {
        return BITCOST_MULTIPLIER
            .wrapping_add(ZSTD_litLengthPrice(ZSTD_BLOCKSIZE_MAX - 1, optPtr, optLevel));
    }

    /* dynamic statistics */
    {
        let llCode: U32 = ZSTD_LLcode(litLength);
        (LL_bits[llCode as usize] as U32)
            .wrapping_mul(BITCOST_MULTIPLIER)
            .wrapping_add((*optPtr).litLengthSumBasePrice)
            .wrapping_sub(WEIGHT(
                *(*optPtr).litLengthFreq.wrapping_add(llCode as usize),
                optLevel,
            ))
    }
}

/* ZSTD_getMatchPrice() :
 * Provides the cost of the match part (offset + matchLength) of a sequence.
 * @offBase : sumtype, representing an offset or a repcode.
 * @optLevel: when <2, favors small offset for decompression speed. */
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
        return WEIGHT(mlBase, optLevel)
            .wrapping_add((16u32.wrapping_add(offCode)).wrapping_mul(BITCOST_MULTIPLIER));
        /* emulated offset cost */
    }

    /* dynamic statistics */
    price = (offCode.wrapping_mul(BITCOST_MULTIPLIER)).wrapping_add(
        (*optPtr).offCodeSumBasePrice.wrapping_sub(WEIGHT(
            *(*optPtr).offCodeFreq.wrapping_add(offCode as usize),
            optLevel,
        )),
    );
    if (optLevel < 2) /*static*/ && offCode >= 20 {
        price = price.wrapping_add(
            (offCode - 19).wrapping_mul(2).wrapping_mul(BITCOST_MULTIPLIER),
        ); /* handicap for long distance offsets, favor decompression speed */
    }

    /* match Length */
    {
        let mlCode: U32 = ZSTD_MLcode(mlBase);
        price = price.wrapping_add(
            (ML_bits[mlCode as usize] as U32)
                .wrapping_mul(BITCOST_MULTIPLIER)
                .wrapping_add((*optPtr).matchLengthSumBasePrice.wrapping_sub(WEIGHT(
                    *(*optPtr).matchLengthFreq.wrapping_add(mlCode as usize),
                    optLevel,
                ))),
        );
    }

    price = price.wrapping_add(BITCOST_MULTIPLIER / 5); /* heuristic : make matches a bit more costly */

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
        let mut u: U32;
        u = 0;
        while u < litLength {
            let idx = *literals.wrapping_add(u as usize) as usize;
            *(*optPtr).litFreq.wrapping_add(idx) =
                (*(*optPtr).litFreq.wrapping_add(idx)).wrapping_add(ZSTD_LITFREQ_ADD);
            u += 1;
        }
        (*optPtr).litSum = (*optPtr)
            .litSum
            .wrapping_add(litLength.wrapping_mul(ZSTD_LITFREQ_ADD));
    }

    /* literal Length */
    {
        let llCode: U32 = ZSTD_LLcode(litLength);
        *(*optPtr).litLengthFreq.wrapping_add(llCode as usize) =
            (*(*optPtr).litLengthFreq.wrapping_add(llCode as usize)).wrapping_add(1);
        (*optPtr).litLengthSum = (*optPtr).litLengthSum.wrapping_add(1);
    }

    /* offset code : follows storeSeq() numeric representation */
    {
        let offCode: U32 = ZSTD_highbit32(offBase);
        *(*optPtr).offCodeFreq.wrapping_add(offCode as usize) =
            (*(*optPtr).offCodeFreq.wrapping_add(offCode as usize)).wrapping_add(1);
        (*optPtr).offCodeSum = (*optPtr).offCodeSum.wrapping_add(1);
    }

    /* match Length */
    {
        let mlBase: U32 = matchLength.wrapping_sub(MINMATCH as U32);
        let mlCode: U32 = ZSTD_MLcode(mlBase);
        *(*optPtr).matchLengthFreq.wrapping_add(mlCode as usize) =
            (*(*optPtr).matchLengthFreq.wrapping_add(mlCode as usize)).wrapping_add(1);
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
                MEM_read32(memPtr as *const u8) << 8
            } else {
                MEM_read32(memPtr as *const u8) >> 8
            }
        }
        /* default and case 4 */
        _ => MEM_read32(memPtr as *const u8),
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
    let target: U32 = ip.offset_from(base) as U32;
    let hash3: size_t = ZSTD_hash3Ptr(ip as *const c_void, hashLog3);

    while idx < target {
        *hashTable3.wrapping_add(ZSTD_hash3Ptr(
            base.wrapping_add(idx as usize) as *const c_void,
            hashLog3,
        ) as usize) = idx;
        idx += 1;
    }

    *nextToUpdate3 = target;
    *hashTable3.wrapping_add(hash3 as usize)
}

/*-*************************************
*  Binary Tree search
***************************************/
/** ZSTD_insertBt1() : add one or multiple positions to tree.
 * @return : nb of positions added */
unsafe fn ZSTD_insertBt1(
    ms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    target: U32,
    mls: U32,
    extDict: c_int,
) -> U32 {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog;
    let h: size_t = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog - 1;
    let btMask: U32 = (1 << btLog) - 1;
    let mut matchIndex: U32 = *hashTable.wrapping_add(h as usize);
    let mut commonLengthSmaller: size_t = 0;
    let mut commonLengthLarger: size_t = 0;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
    let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
    let mut r#match: *const BYTE;
    let curr: U32 = ip.offset_from(base) as U32;
    let btLow: U32 = if btMask >= curr { 0 } else { curr - btMask };
    let mut smallerPtr: *mut U32 = bt.wrapping_add((2 * (curr & btMask)) as usize);
    let mut largerPtr: *mut U32 = smallerPtr.wrapping_add(1);
    let mut dummy32: U32 = 0; /* to be nullified at the end */
    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, target, (*cParams).windowLog);
    let mut matchEndIdx: U32 = curr + 8 + 1;
    let mut bestLength: size_t = 8;
    let mut nbCompares: U32 = 1u32 << (*cParams).searchLog;

    *hashTable.wrapping_add(h as usize) = curr; /* Update Hash Table */

    while nbCompares != 0 && (matchIndex >= windowLow) {
        let nextPtr: *mut U32 = bt.wrapping_add((2 * (matchIndex & btMask)) as usize);
        let mut matchLength: size_t = MIN(commonLengthSmaller, commonLengthLarger); /* guaranteed minimum nb of common bytes */

        if extDict == 0 || (matchIndex + matchLength as U32 >= dictLimit) {
            r#match = base.wrapping_add(matchIndex as usize);
            matchLength += ZSTD_count(
                ip.wrapping_add(matchLength),
                r#match.wrapping_add(matchLength),
                iend,
            );
        } else {
            r#match = dictBase.wrapping_add(matchIndex as usize);
            matchLength += ZSTD_count_2segments(
                ip.wrapping_add(matchLength),
                r#match.wrapping_add(matchLength),
                iend,
                dictEnd,
                prefixStart,
            );
            if matchIndex + matchLength as U32 >= dictLimit {
                r#match = base.wrapping_add(matchIndex as usize); /* to prepare for next usage of match[matchLength] */
            }
        }

        if matchLength > bestLength {
            bestLength = matchLength;
            if matchLength as U32 > matchEndIdx - matchIndex {
                matchEndIdx = matchIndex + matchLength as U32;
            }
        }

        if ip.wrapping_add(matchLength) == iend {
            /* equal : no way to know if inf or sup */
            break; /* drop , to guarantee consistency ; miss a bit of compression, but other solutions can corrupt tree */
        }

        if *r#match.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
            /* necessarily within buffer */
            /* match is smaller than current */
            *smallerPtr = matchIndex; /* update smaller idx */
            commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
            if matchIndex <= btLow {
                smallerPtr = &mut dummy32;
                break;
            } /* beyond tree size, stop searching */
            smallerPtr = nextPtr.wrapping_add(1); /* new "candidate" => larger than match, which was smaller than target */
            matchIndex = *nextPtr.wrapping_add(1); /* new matchIndex, larger than previous and closer to current */
        } else {
            /* match is larger than current */
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &mut dummy32;
                break;
            } /* beyond tree size, stop searching */
            largerPtr = nextPtr;
            matchIndex = *nextPtr.wrapping_add(0);
        }
        nbCompares -= 1;
    }

    *smallerPtr = 0;
    *largerPtr = 0;
    {
        let mut positions: U32 = 0;
        if bestLength > 384 {
            positions = MIN(192u32, (bestLength - 384) as U32); /* speed optimization */
        }
        MAX(positions, matchEndIdx - (curr + 8))
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
    let target: U32 = ip.offset_from(base) as U32;
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
        idx += forward;
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
    matches: *mut ZSTD_match_t, /* store result (found matches) in this table */
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const BYTE,
    iLimit: *const BYTE,
    dictMode: ZSTD_dictMode_e,
    rep: *const U32, /* rep[ZSTD_REP_NUM] */
    ll0: U32,        /* tells if associated literal length is 0 or not. */
    lengthToBeat: U32,
    mls: U32, /* template */
) -> U32 {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let sufficient_len: U32 = MIN((*cParams).targetLength, ZSTD_OPT_NUM - 1);
    let base: *const BYTE = (*ms).window.base;
    let curr: U32 = ip.offset_from(base) as U32;
    let hashLog: U32 = (*cParams).hashLog;
    let minMatch: U32 = if mls == 3 { 3 } else { 4 };
    let hashTable: *mut U32 = (*ms).hashTable;
    let h: size_t = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut matchIndex: U32 = *hashTable.wrapping_add(h as usize);
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = (*cParams).chainLog - 1;
    let btMask: U32 = (1u32 << btLog) - 1;
    let mut commonLengthSmaller: size_t = 0;
    let mut commonLengthLarger: size_t = 0;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let dictEnd: *const BYTE = dictBase.wrapping_add(dictLimit as usize);
    let prefixStart: *const BYTE = base.wrapping_add(dictLimit as usize);
    let btLow: U32 = if btMask >= curr { 0 } else { curr - btMask };
    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, (*cParams).windowLog);
    let matchLow: U32 = if windowLow != 0 { windowLow } else { 1 };
    let mut smallerPtr: *mut U32 = bt.wrapping_add((2 * (curr & btMask)) as usize);
    let mut largerPtr: *mut U32 = bt.wrapping_add((2 * (curr & btMask) + 1) as usize);
    let mut matchEndIdx: U32 = curr + 8 + 1; /* farthest referenced position of any match => detects repetitive patterns */
    let mut dummy32: U32 = 0; /* to be nullified at the end */
    let mut mnum: U32 = 0;
    let mut nbCompares: U32 = 1u32 << (*cParams).searchLog;

    let dms: *const ZSTD_MatchState_t = if dictMode == ZSTD_dictMatchState {
        (*ms).dictMatchState
    } else {
        core::ptr::null()
    };
    let dmsCParams: *const ZSTD_compressionParameters = if dictMode == ZSTD_dictMatchState {
        &(*dms).cParams
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
        dmsEnd.offset_from(dmsBase) as U32
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
        (*dmsCParams).chainLog - 1
    } else {
        btLog
    };
    let dmsBtMask: U32 = if dictMode == ZSTD_dictMatchState {
        (1u32 << dmsBtLog) - 1
    } else {
        0
    };
    let dmsBtLow: U32 =
        if dictMode == ZSTD_dictMatchState && dmsBtMask < dmsHighLimit - dmsLowLimit {
            dmsHighLimit - dmsBtMask
        } else {
            dmsLowLimit
        };

    let mut bestLength: size_t = (lengthToBeat - 1) as size_t;

    /* check repCode */
    {
        let lastR: U32 = ZSTD_REP_NUM as U32 + ll0;
        let mut repCode: U32;
        repCode = ll0;
        while repCode < lastR {
            let repOffset: U32 = if repCode == ZSTD_REP_NUM as U32 {
                *rep.wrapping_add(0) - 1
            } else {
                *rep.wrapping_add(repCode as usize)
            };
            let repIndex: U32 = curr.wrapping_sub(repOffset);
            let mut repLen: U32 = 0;
            if repOffset.wrapping_sub(1) /* intentional overflow, discards 0 and -1 */ < curr - dictLimit {
                /* equivalent to `curr > repIndex >= dictLimit` */
                if (repIndex >= windowLow)
                    & (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(
                            ip.wrapping_sub(repOffset as usize) as *const c_void,
                            minMatch,
                        ))
                {
                    repLen = ZSTD_count(
                        ip.wrapping_add(minMatch as usize),
                        ip.wrapping_add(minMatch as usize)
                            .wrapping_sub(repOffset as usize),
                        iLimit,
                    ) as U32
                        + minMatch;
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
                    && ((repOffset.wrapping_sub(1) /*intentional overflow*/ < curr - windowLow)
                        & (ZSTD_index_overlap_check(dictLimit, repIndex) != 0))
                    && (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(repMatch as *const c_void, minMatch))
                {
                    repLen = ZSTD_count_2segments(
                        ip.wrapping_add(minMatch as usize),
                        repMatch.wrapping_add(minMatch as usize),
                        iLimit,
                        dictEnd,
                        prefixStart,
                    ) as U32
                        + minMatch;
                }
                if dictMode == ZSTD_dictMatchState
                    && ((repOffset.wrapping_sub(1) /*intentional overflow*/
                        < curr - (dmsLowLimit + dmsIndexDelta))
                        & (ZSTD_index_overlap_check(dictLimit, repIndex) != 0))
                    && (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(repMatch as *const c_void, minMatch))
                {
                    repLen = ZSTD_count_2segments(
                        ip.wrapping_add(minMatch as usize),
                        repMatch.wrapping_add(minMatch as usize),
                        iLimit,
                        dmsEnd,
                        prefixStart,
                    ) as U32
                        + minMatch;
                }
            }
            /* save longer solution */
            if repLen as size_t > bestLength {
                bestLength = repLen as size_t;
                (*matches.wrapping_add(mnum as usize)).off =
                    REPCODE_TO_OFFBASE(repCode - ll0 + 1); /* expect value between 1 and 3 */
                (*matches.wrapping_add(mnum as usize)).len = repLen;
                mnum += 1;
                if (repLen as size_t > sufficient_len as size_t)
                    | (ip.wrapping_add(repLen as usize) == iLimit)
                {
                    /* best possible */
                    return mnum;
                }
            }
            repCode += 1;
        }
    }

    /* HC3 match finder */
    if (mls == 3) /*static*/ && (bestLength < mls as size_t) {
        let matchIndex3: U32 = ZSTD_insertAndFindFirstIndexHash3(ms, nextToUpdate3, ip);
        if (matchIndex3 >= matchLow) & ((curr - matchIndex3 < (1 << 18)) as u32 != 0)
        /*heuristic : longer distance likely too expensive*/
        {
            let mlen: size_t;
            if (dictMode == ZSTD_noDict) /*static*/
                || (dictMode == ZSTD_dictMatchState) /*static*/
                || (matchIndex3 >= dictLimit)
            {
                let r#match: *const BYTE = base.wrapping_add(matchIndex3 as usize);
                mlen = ZSTD_count(ip, r#match, iLimit);
            } else {
                let r#match: *const BYTE = dictBase.wrapping_add(matchIndex3 as usize);
                mlen = ZSTD_count_2segments(ip, r#match, iLimit, dictEnd, prefixStart);
            }

            /* save best solution */
            if mlen >= mls as size_t
            /* == 3 > bestLength */
            {
                bestLength = mlen;
                (*matches.wrapping_add(0)).off = OFFSET_TO_OFFBASE(curr - matchIndex3);
                (*matches.wrapping_add(0)).len = mlen as U32;
                mnum = 1;
                if (mlen > sufficient_len as size_t) | (ip.wrapping_add(mlen) == iLimit) {
                    /* best possible length */
                    (*ms).nextToUpdate = curr + 1; /* skip insertion */
                    return 1;
                }
            }
        }
        /* no dictMatchState lookup: dicts don't have a populated HC3 table */
    } /* if (mls == 3) */

    *hashTable.wrapping_add(h as usize) = curr; /* Update Hash Table */

    while nbCompares != 0 && (matchIndex >= matchLow) {
        let nextPtr: *mut U32 = bt.wrapping_add((2 * (matchIndex & btMask)) as usize);
        let mut r#match: *const BYTE;
        let mut matchLength: size_t = MIN(commonLengthSmaller, commonLengthLarger); /* guaranteed minimum nb of common bytes */

        if (dictMode == ZSTD_noDict)
            || (dictMode == ZSTD_dictMatchState)
            || (matchIndex + matchLength as U32 >= dictLimit)
        {
            r#match = base.wrapping_add(matchIndex as usize);
            matchLength += ZSTD_count(
                ip.wrapping_add(matchLength),
                r#match.wrapping_add(matchLength),
                iLimit,
            );
        } else {
            r#match = dictBase.wrapping_add(matchIndex as usize);
            matchLength += ZSTD_count_2segments(
                ip.wrapping_add(matchLength),
                r#match.wrapping_add(matchLength),
                iLimit,
                dictEnd,
                prefixStart,
            );
            if matchIndex + matchLength as U32 >= dictLimit {
                r#match = base.wrapping_add(matchIndex as usize); /* prepare for match[matchLength] read */
            }
        }

        if matchLength > bestLength {
            if matchLength as U32 > matchEndIdx - matchIndex {
                matchEndIdx = matchIndex + matchLength as U32;
            }
            bestLength = matchLength;
            (*matches.wrapping_add(mnum as usize)).off = OFFSET_TO_OFFBASE(curr - matchIndex);
            (*matches.wrapping_add(mnum as usize)).len = matchLength as U32;
            mnum += 1;
            if (matchLength > ZSTD_OPT_NUM as size_t)
                | (ip.wrapping_add(matchLength) == iLimit)
            /* equal : no way to know if inf or sup */
            {
                if dictMode == ZSTD_dictMatchState {
                    nbCompares = 0; /* break should also skip searching dms */
                }
                break; /* drop, to preserve bt consistency */
            }
        }

        if *r#match.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
            /* match smaller than current */
            *smallerPtr = matchIndex; /* update smaller idx */
            commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
            if matchIndex <= btLow {
                smallerPtr = &mut dummy32;
                break;
            } /* beyond tree size, stop the search */
            smallerPtr = nextPtr.wrapping_add(1); /* new candidate => larger than match, which was smaller than current */
            matchIndex = *nextPtr.wrapping_add(1); /* new matchIndex, larger than previous, closer to current */
        } else {
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &mut dummy32;
                break;
            } /* beyond tree size, stop the search */
            largerPtr = nextPtr;
            matchIndex = *nextPtr.wrapping_add(0);
        }
        nbCompares -= 1;
    }

    *smallerPtr = 0;
    *largerPtr = 0;

    if dictMode == ZSTD_dictMatchState && nbCompares != 0 {
        let dmsH: size_t = ZSTD_hashPtr(ip as *const c_void, dmsHashLog, mls);
        let mut dictMatchIndex: U32 = *(*dms).hashTable.wrapping_add(dmsH as usize);
        let dmsBt: *const U32 = (*dms).chainTable;
        commonLengthSmaller = 0;
        commonLengthLarger = 0;
        while nbCompares != 0 && (dictMatchIndex > dmsLowLimit) {
            let nextPtr: *const U32 = dmsBt.wrapping_add((2 * (dictMatchIndex & dmsBtMask)) as usize);
            let mut matchLength: size_t = MIN(commonLengthSmaller, commonLengthLarger); /* guaranteed minimum nb of common bytes */
            let mut r#match: *const BYTE = dmsBase.wrapping_add(dictMatchIndex as usize);
            matchLength += ZSTD_count_2segments(
                ip.wrapping_add(matchLength),
                r#match.wrapping_add(matchLength),
                iLimit,
                dmsEnd,
                prefixStart,
            );
            if dictMatchIndex + matchLength as U32 >= dmsHighLimit {
                r#match = base
                    .wrapping_add(dictMatchIndex as usize)
                    .wrapping_add(dmsIndexDelta as usize); /* to prepare for next usage of match[matchLength] */
            }

            if matchLength > bestLength {
                matchIndex = dictMatchIndex.wrapping_add(dmsIndexDelta);
                if matchLength as U32 > matchEndIdx - matchIndex {
                    matchEndIdx = matchIndex + matchLength as U32;
                }
                bestLength = matchLength;
                (*matches.wrapping_add(mnum as usize)).off = OFFSET_TO_OFFBASE(curr - matchIndex);
                (*matches.wrapping_add(mnum as usize)).len = matchLength as U32;
                mnum += 1;
                if (matchLength > ZSTD_OPT_NUM as size_t)
                    | (ip.wrapping_add(matchLength) == iLimit)
                /* equal : no way to know if inf or sup */
                {
                    break; /* drop, to guarantee consistency */
                }
            }

            if dictMatchIndex <= dmsBtLow {
                break;
            } /* beyond tree size, stop the search */
            if *r#match.wrapping_add(matchLength) < *ip.wrapping_add(matchLength) {
                commonLengthSmaller = matchLength; /* all smaller will now have at least this guaranteed common length */
                dictMatchIndex = *nextPtr.wrapping_add(1); /* new matchIndex larger than previous (closer to current) */
            } else {
                /* match is larger than current */
                commonLengthLarger = matchLength;
                dictMatchIndex = *nextPtr.wrapping_add(0);
            }
            nbCompares -= 1;
        }
    } /* if (dictMode == ZSTD_dictMatchState) */

    (*ms).nextToUpdate = matchEndIdx - 8; /* skip repetitive patterns */
    mnum
}

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

pub type ZSTD_getAllMatchesFn = unsafe fn(
    *mut ZSTD_match_t,
    *mut ZSTD_MatchState_t,
    *mut U32,
    *const BYTE,
    *const BYTE,
    *const U32, /* rep[ZSTD_REP_NUM] */
    U32,        /* ll0 */
    U32,        /* lengthToBeat */
) -> U32;

/* GEN_ZSTD_BT_GET_ALL_MATCHES: expands into one fn per (dictMode, mls) pair.
 * Each is a thin wrapper over ZSTD_btGetAllMatches_internal with the template
 * arguments fixed. Named ZSTD_btGetAllMatches_<dictMode>_<mls>. */
macro_rules! GEN_ZSTD_BT_GET_ALL_MATCHES_ {
    ($name:ident, $dictMode:expr, $mls:expr) => {
        unsafe fn $name(
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
                $dictMode,
                $mls,
            )
        }
    };
}

GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_noDict_3, ZSTD_noDict, 3);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_noDict_4, ZSTD_noDict, 4);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_noDict_5, ZSTD_noDict, 5);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_noDict_6, ZSTD_noDict, 6);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_extDict_3, ZSTD_extDict, 3);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_extDict_4, ZSTD_extDict, 4);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_extDict_5, ZSTD_extDict, 5);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_extDict_6, ZSTD_extDict, 6);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_dictMatchState_3, ZSTD_dictMatchState, 3);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_dictMatchState_4, ZSTD_dictMatchState, 4);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_dictMatchState_5, ZSTD_dictMatchState, 5);
GEN_ZSTD_BT_GET_ALL_MATCHES_!(ZSTD_btGetAllMatches_dictMatchState_6, ZSTD_dictMatchState, 6);

unsafe fn ZSTD_selectBtGetAllMatches(
    ms: *const ZSTD_MatchState_t,
    dictMode: ZSTD_dictMode_e,
) -> ZSTD_getAllMatchesFn {
    let getAllMatchesFns: [[ZSTD_getAllMatchesFn; 4]; 3] = [
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
    let mls: U32 = BOUNDED(3, (*ms).cParams.minMatch, 6);
    getAllMatchesFns[dictMode as usize][(mls - 3) as usize]
}

/*************************
*  LDM helper functions  *
*************************/

/* Struct containing info needed to make decision about ldm inclusion */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_optLdm_t {
    pub seqStore: RawSeqStore_t, /* External match candidates store for this block */
    pub startPosInBlock: U32,    /* Start position of the current match candidate */
    pub endPosInBlock: U32,      /* End position of the current match candidate */
    pub offset: U32,             /* Offset of the match candidate */
}

/* ZSTD_optLdm_skipRawSeqStoreBytes():
 * Moves forward in @rawSeqStore by @nbBytes,
 * which will update the fields 'pos' and 'posInSequence'. */
unsafe fn ZSTD_optLdm_skipRawSeqStoreBytes(rawSeqStore: *mut RawSeqStore_t, nbBytes: size_t) {
    let mut currPos: U32 = ((*rawSeqStore).posInSequence + nbBytes) as U32;
    while currPos != 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let currSeq: rawSeq = *(*rawSeqStore).seq.wrapping_add((*rawSeqStore).pos);
        if currPos >= currSeq.litLength + currSeq.matchLength {
            currPos -= currSeq.litLength + currSeq.matchLength;
            (*rawSeqStore).pos += 1;
        } else {
            (*rawSeqStore).posInSequence = currPos as size_t;
            break;
        }
    }
    if currPos == 0 || (*rawSeqStore).pos == (*rawSeqStore).size {
        (*rawSeqStore).posInSequence = 0;
    }
}

/* ZSTD_opt_getNextMatchAndUpdateSeqStore():
 * Calculates the beginning and end of the next match in the current block.
 * Updates 'pos' and 'posInSequence' of the ldmSeqStore. */
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
    currSeq = *(*optLdm).seqStore.seq.wrapping_add((*optLdm).seqStore.pos);
    currBlockEndPos = currPosInBlock + blockBytesRemaining;
    literalsBytesRemaining = if ((*optLdm).seqStore.posInSequence as U32) < currSeq.litLength {
        currSeq.litLength - (*optLdm).seqStore.posInSequence as U32
    } else {
        0
    };
    matchBytesRemaining = if literalsBytesRemaining == 0 {
        currSeq
            .matchLength
            .wrapping_sub((*optLdm).seqStore.posInSequence as U32 - currSeq.litLength)
    } else {
        currSeq.matchLength
    };

    /* If there are more literal bytes than bytes remaining in block, no ldm is possible */
    if literalsBytesRemaining >= blockBytesRemaining {
        (*optLdm).startPosInBlock = u32::MAX;
        (*optLdm).endPosInBlock = u32::MAX;
        ZSTD_optLdm_skipRawSeqStoreBytes(&mut (*optLdm).seqStore, blockBytesRemaining as size_t);
        return;
    }

    /* Matches may be < minMatch by this process. In that case, we will reject them
       when we are deciding whether or not to add the ldm */
    (*optLdm).startPosInBlock = currPosInBlock + literalsBytesRemaining;
    (*optLdm).endPosInBlock = (*optLdm).startPosInBlock + matchBytesRemaining;
    (*optLdm).offset = currSeq.offset;

    if (*optLdm).endPosInBlock > currBlockEndPos {
        /* Match ends after the block ends, we can't use the whole match */
        (*optLdm).endPosInBlock = currBlockEndPos;
        ZSTD_optLdm_skipRawSeqStoreBytes(
            &mut (*optLdm).seqStore,
            (currBlockEndPos - currPosInBlock) as size_t,
        );
    } else {
        /* Consume nb of bytes equal to size of sequence left */
        ZSTD_optLdm_skipRawSeqStoreBytes(
            &mut (*optLdm).seqStore,
            (literalsBytesRemaining + matchBytesRemaining) as size_t,
        );
    }
}

/* ZSTD_optLdm_maybeAddMatch():
 * Adds a match if it's long enough, into 'matches'. Maintains the correct ordering of 'matches'. */
unsafe fn ZSTD_optLdm_maybeAddMatch(
    matches: *mut ZSTD_match_t,
    nbMatches: *mut U32,
    optLdm: *const ZSTD_optLdm_t,
    currPosInBlock: U32,
    minMatch: U32,
) {
    let posDiff: U32 = currPosInBlock - (*optLdm).startPosInBlock;
    /* Note: ZSTD_match_t actually contains offBase and matchLength (before subtracting MINMATCH) */
    let candidateMatchLength: U32 =
        (*optLdm).endPosInBlock - (*optLdm).startPosInBlock - posDiff;

    /* Ensure that current block position is not outside of the match */
    if currPosInBlock < (*optLdm).startPosInBlock
        || currPosInBlock >= (*optLdm).endPosInBlock
        || candidateMatchLength < minMatch
    {
        return;
    }

    if *nbMatches == 0
        || ((candidateMatchLength > (*matches.wrapping_add((*nbMatches - 1) as usize)).len)
            && *nbMatches < ZSTD_OPT_NUM)
    {
        let candidateOffBase: U32 = OFFSET_TO_OFFBASE((*optLdm).offset);
        (*matches.wrapping_add(*nbMatches as usize)).len = candidateMatchLength;
        (*matches.wrapping_add(*nbMatches as usize)).off = candidateOffBase;
        *nbMatches += 1;
    }
}

/* ZSTD_optLdm_processMatchCandidate():
 * Wrapper function to update ldm seq store and call ldm functions as necessary. */
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
            /* correct for "overshoots" */
            let posOvershoot: U32 = currPosInBlock - (*optLdm).endPosInBlock;
            ZSTD_optLdm_skipRawSeqStoreBytes(&mut (*optLdm).seqStore, posOvershoot as size_t);
        }
        ZSTD_opt_getNextMatchAndUpdateSeqStore(optLdm, currPosInBlock, remainingBytes);
    }
    ZSTD_optLdm_maybeAddMatch(matches, nbMatches, optLdm, currPosInBlock, minMatch);
}

/*-*******************************
*  Optimal parser
*********************************/

/* LIT_PRICE / LL_PRICE / LL_INCPRICE macros as inline helpers */
#[inline(always)]
unsafe fn LIT_PRICE(p: *const BYTE, optStatePtr: *const optState_t, optLevel: c_int) -> c_int {
    ZSTD_rawLiteralsCost(p, 1, optStatePtr, optLevel) as c_int
}
#[inline(always)]
unsafe fn LL_PRICE(l: U32, optStatePtr: *const optState_t, optLevel: c_int) -> c_int {
    ZSTD_litLengthPrice(l, optStatePtr, optLevel) as c_int
}
#[inline(always)]
unsafe fn LL_INCPRICE(l: U32, optStatePtr: *const optState_t, optLevel: c_int) -> c_int {
    LL_PRICE(l, optStatePtr, optLevel) - LL_PRICE(l - 1, optStatePtr, optLevel)
}

unsafe fn ZSTD_compressBlock_opt_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32, /* rep[ZSTD_REP_NUM] */
    src: *const c_void,
    srcSize: size_t,
    optLevel: c_int,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let optStatePtr: *mut optState_t = &mut (*ms).opt;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(8);
    let base: *const BYTE = (*ms).window.base;
    let prefixStart: *const BYTE = base.wrapping_add((*ms).window.dictLimit as usize);
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;

    let getAllMatches: ZSTD_getAllMatchesFn = ZSTD_selectBtGetAllMatches(ms, dictMode);

    let sufficient_len: U32 = MIN((*cParams).targetLength, ZSTD_OPT_NUM - 1);
    let minMatch: U32 = if (*cParams).minMatch == 3 { 3 } else { 4 };
    let mut nextToUpdate3: U32 = (*ms).nextToUpdate;

    let opt: *mut ZSTD_optimal_t = (*optStatePtr).priceTable;
    let matches: *mut ZSTD_match_t = (*optStatePtr).matchTable;
    let mut lastStretch: ZSTD_optimal_t = core::mem::zeroed();
    let mut optLdm: ZSTD_optLdm_t = core::mem::zeroed();

    ZSTD_memset(
        &mut lastStretch as *mut ZSTD_optimal_t as *mut u8,
        0,
        core::mem::size_of::<ZSTD_optimal_t>() as size_t,
    );

    optLdm.seqStore = if (*ms).ldmSeqStore != core::ptr::null() {
        *(*ms).ldmSeqStore
    } else {
        kNullRawSeqStore
    };
    optLdm.endPosInBlock = 0;
    optLdm.startPosInBlock = 0;
    optLdm.offset = 0;
    ZSTD_opt_getNextMatchAndUpdateSeqStore(
        &mut optLdm,
        ip.offset_from(istart) as U32,
        iend.offset_from(ip) as U32,
    );

    /* init */
    ZSTD_rescaleFreqs(optStatePtr, src as *const BYTE, srcSize, optLevel);
    ip = ip.wrapping_add((ip == prefixStart) as usize);

    /* Match Loop */
    'matchLoop: while ip < ilimit {
        let mut cur: U32;
        let mut last_pos: U32 = 0;

        /* `'toShortestPath` is a labeled block that the three C `goto _shortestPath`
         * sites break out of. Falling off the end of the block corresponds to the
         * C fall-through into `_shortestPath:`. After the block, the shortest-path
         * body runs, then the outer `'matchLoop` continues (matching C's `continue`). */
        'toShortestPath: {

        /* find first match */
        {
            let litlen: U32 = ip.offset_from(anchor) as U32;
            let ll0: U32 = (litlen == 0) as U32;
            let mut nbMatches: U32 = getAllMatches(
                matches,
                ms,
                &mut nextToUpdate3,
                ip,
                iend,
                rep,
                ll0,
                minMatch,
            );
            ZSTD_optLdm_processMatchCandidate(
                &mut optLdm,
                matches,
                &mut nbMatches,
                ip.offset_from(istart) as U32,
                iend.offset_from(ip) as U32,
                minMatch,
            );
            if nbMatches == 0 {
                ip = ip.wrapping_add(1);
                continue 'matchLoop;
            }

            /* initialize opt[0] */
            (*opt.wrapping_add(0)).mlen = 0; /* there are only literals so far */
            (*opt.wrapping_add(0)).litlen = litlen;
            (*opt.wrapping_add(0)).price = LL_PRICE(litlen, optStatePtr, optLevel);
            ZSTD_memcpy(
                (*opt.wrapping_add(0)).rep.as_mut_ptr() as *mut u8,
                rep as *const u8,
                core::mem::size_of_val(&(*opt.wrapping_add(0)).rep) as size_t,
            );

            /* large match -> immediate encoding */
            {
                let maxML: U32 = (*matches.wrapping_add((nbMatches - 1) as usize)).len;
                let maxOffBase: U32 = (*matches.wrapping_add((nbMatches - 1) as usize)).off;

                if maxML > sufficient_len {
                    lastStretch.litlen = 0;
                    lastStretch.mlen = maxML;
                    lastStretch.off = maxOffBase;
                    cur = 0;
                    last_pos = maxML;
                    break 'toShortestPath; /* goto _shortestPath */
                }
            }

            /* set prices for first matches starting position == 0 */
            {
                let mut pos: U32;
                let mut matchNb: U32;
                pos = 1;
                while pos < minMatch {
                    (*opt.wrapping_add(pos as usize)).price = ZSTD_MAX_PRICE;
                    (*opt.wrapping_add(pos as usize)).mlen = 0;
                    (*opt.wrapping_add(pos as usize)).litlen = litlen + pos;
                    pos += 1;
                }
                matchNb = 0;
                while matchNb < nbMatches {
                    let offBase: U32 = (*matches.wrapping_add(matchNb as usize)).off;
                    let end: U32 = (*matches.wrapping_add(matchNb as usize)).len;
                    while pos <= end {
                        let matchPrice: c_int =
                            ZSTD_getMatchPrice(offBase, pos, optStatePtr, optLevel) as c_int;
                        let sequencePrice: c_int =
                            (*opt.wrapping_add(0)).price + matchPrice;
                        (*opt.wrapping_add(pos as usize)).mlen = pos;
                        (*opt.wrapping_add(pos as usize)).off = offBase;
                        (*opt.wrapping_add(pos as usize)).litlen = 0; /* end of match */
                        (*opt.wrapping_add(pos as usize)).price =
                            sequencePrice + LL_PRICE(0, optStatePtr, optLevel);
                        pos += 1;
                    }
                    matchNb += 1;
                }
                last_pos = pos - 1;
                (*opt.wrapping_add(pos as usize)).price = ZSTD_MAX_PRICE;
            }
        }

        /* check further positions */
        cur = 1;
        'curLoop: while cur <= last_pos {
            let inr: *const BYTE = ip.wrapping_add(cur as usize);

            /* Fix current position with one literal if cheaper */
            {
                let litlen: U32 = (*opt.wrapping_add((cur - 1) as usize)).litlen + 1;
                let price: c_int = (*opt.wrapping_add((cur - 1) as usize)).price
                    + LIT_PRICE(ip.wrapping_add((cur - 1) as usize), optStatePtr, optLevel)
                    + LL_INCPRICE(litlen, optStatePtr, optLevel);
                if price <= (*opt.wrapping_add(cur as usize)).price {
                    let prevMatch: ZSTD_optimal_t = *opt.wrapping_add(cur as usize);
                    *opt.wrapping_add(cur as usize) = *opt.wrapping_add((cur - 1) as usize);
                    (*opt.wrapping_add(cur as usize)).litlen = litlen;
                    (*opt.wrapping_add(cur as usize)).price = price;
                    if (optLevel >= 1) /* additional check only for higher modes */
                        && (prevMatch.litlen == 0) /* replace a match */
                        && (LL_INCPRICE(1, optStatePtr, optLevel) < 0) /* ll1 is cheaper than ll0 */
                        && (ip.wrapping_add(cur as usize) < iend)
                    {
                        /* check next position, in case it would be cheaper */
                        let with1literal: c_int = prevMatch.price
                            + LIT_PRICE(ip.wrapping_add(cur as usize), optStatePtr, optLevel)
                            + LL_INCPRICE(1, optStatePtr, optLevel);
                        let withMoreLiterals: c_int = price
                            + LIT_PRICE(ip.wrapping_add(cur as usize), optStatePtr, optLevel)
                            + LL_INCPRICE(litlen + 1, optStatePtr, optLevel);
                        if (with1literal < withMoreLiterals)
                            && (with1literal < (*opt.wrapping_add((cur + 1) as usize)).price)
                        {
                            /* update offset history - before it disappears */
                            let prev: U32 = cur - prevMatch.mlen;
                            let newReps: Repcodes_t = ZSTD_newRep(
                                (*opt.wrapping_add(prev as usize)).rep.as_ptr(),
                                prevMatch.off,
                                ((*opt.wrapping_add(prev as usize)).litlen == 0) as U32,
                            );
                            *opt.wrapping_add((cur + 1) as usize) = prevMatch; /* mlen & offbase */
                            ZSTD_memcpy(
                                (*opt.wrapping_add((cur + 1) as usize)).rep.as_mut_ptr()
                                    as *mut u8,
                                &newReps as *const Repcodes_t as *const u8,
                                core::mem::size_of::<Repcodes_t>() as size_t,
                            );
                            (*opt.wrapping_add((cur + 1) as usize)).litlen = 1;
                            (*opt.wrapping_add((cur + 1) as usize)).price = with1literal;
                            if last_pos < cur + 1 {
                                last_pos = cur + 1;
                            }
                        }
                    }
                }
            }

            /* Offset history is not updated during match comparison.
             * Do it here, now that the match is selected and confirmed. */
            if (*opt.wrapping_add(cur as usize)).litlen == 0 {
                /* just finished a match => alter offset history */
                let prev: U32 = cur - (*opt.wrapping_add(cur as usize)).mlen;
                let newReps: Repcodes_t = ZSTD_newRep(
                    (*opt.wrapping_add(prev as usize)).rep.as_ptr(),
                    (*opt.wrapping_add(cur as usize)).off,
                    ((*opt.wrapping_add(prev as usize)).litlen == 0) as U32,
                );
                ZSTD_memcpy(
                    (*opt.wrapping_add(cur as usize)).rep.as_mut_ptr() as *mut u8,
                    &newReps as *const Repcodes_t as *const u8,
                    core::mem::size_of::<Repcodes_t>() as size_t,
                );
            }

            /* last match must start at a minimum distance of 8 from oend */
            if inr > ilimit {
                cur += 1;
                continue 'curLoop;
            }

            if cur == last_pos {
                break 'curLoop;
            }

            if (optLevel == 0) /*static_test*/
                && ((*opt.wrapping_add((cur + 1) as usize)).price
                    <= (*opt.wrapping_add(cur as usize)).price
                        + (BITCOST_MULTIPLIER / 2) as c_int)
            {
                cur += 1;
                continue 'curLoop; /* skip unpromising positions */
            }

            {
                let ll0: U32 = ((*opt.wrapping_add(cur as usize)).litlen == 0) as U32;
                let previousPrice: c_int = (*opt.wrapping_add(cur as usize)).price;
                let basePrice: c_int = previousPrice + LL_PRICE(0, optStatePtr, optLevel);
                let mut nbMatches: U32 = getAllMatches(
                    matches,
                    ms,
                    &mut nextToUpdate3,
                    inr,
                    iend,
                    (*opt.wrapping_add(cur as usize)).rep.as_ptr(),
                    ll0,
                    minMatch,
                );
                let mut matchNb: U32;

                ZSTD_optLdm_processMatchCandidate(
                    &mut optLdm,
                    matches,
                    &mut nbMatches,
                    inr.offset_from(istart) as U32,
                    iend.offset_from(inr) as U32,
                    minMatch,
                );

                if nbMatches == 0 {
                    cur += 1;
                    continue 'curLoop;
                }

                {
                    let longestML: U32 = (*matches.wrapping_add((nbMatches - 1) as usize)).len;

                    if (longestML > sufficient_len)
                        || (cur + longestML >= ZSTD_OPT_NUM)
                        || (ip.wrapping_add((cur + longestML) as usize) >= iend)
                    {
                        lastStretch.mlen = longestML;
                        lastStretch.off = (*matches.wrapping_add((nbMatches - 1) as usize)).off;
                        lastStretch.litlen = 0;
                        last_pos = cur + longestML;
                        break 'toShortestPath; /* goto _shortestPath */
                    }
                }

                /* set prices using matches found at position == cur */
                matchNb = 0;
                while matchNb < nbMatches {
                    let offset: U32 = (*matches.wrapping_add(matchNb as usize)).off;
                    let lastML: U32 = (*matches.wrapping_add(matchNb as usize)).len;
                    let startML: U32 = if matchNb > 0 {
                        (*matches.wrapping_add((matchNb - 1) as usize)).len + 1
                    } else {
                        minMatch
                    };
                    let mut mlen: U32;

                    mlen = lastML;
                    while mlen >= startML {
                        /* scan downward */
                        let pos: U32 = cur + mlen;
                        let price: c_int = basePrice
                            + ZSTD_getMatchPrice(offset, mlen, optStatePtr, optLevel) as c_int;

                        if (pos > last_pos) || (price < (*opt.wrapping_add(pos as usize)).price) {
                            while last_pos < pos {
                                /* fill empty positions, for future comparisons */
                                last_pos += 1;
                                (*opt.wrapping_add(last_pos as usize)).price = ZSTD_MAX_PRICE;
                                (*opt.wrapping_add(last_pos as usize)).litlen = 1; /* just needs to be != 0 */
                            }
                            (*opt.wrapping_add(pos as usize)).mlen = mlen;
                            (*opt.wrapping_add(pos as usize)).off = offset;
                            (*opt.wrapping_add(pos as usize)).litlen = 0;
                            (*opt.wrapping_add(pos as usize)).price = price;
                        } else {
                            if optLevel == 0 {
                                break; /* early update abort */
                            }
                        }
                        mlen -= 1;
                    }
                    matchNb += 1;
                }
            }
            (*opt.wrapping_add((last_pos + 1) as usize)).price = ZSTD_MAX_PRICE;
            cur += 1;
        } /* for (cur = 1; cur <= last_pos; cur++) */

        lastStretch = *opt.wrapping_add(last_pos as usize);
        cur = last_pos - lastStretch.mlen;

        } /* 'toShortestPath block (fall-through == C fall into _shortestPath) */

        /* _shortestPath:   cur, last_pos, best_mlen, best_off have to be set */

        if lastStretch.mlen == 0 {
            /* no solution : all matches have been converted into literals */
            ip = ip.wrapping_add(last_pos as usize);
            continue 'matchLoop;
        }

        /* Update offset history */
        if lastStretch.litlen == 0 {
            /* finishing on a match : update offset history */
            let reps: Repcodes_t = ZSTD_newRep(
                (*opt.wrapping_add(cur as usize)).rep.as_ptr(),
                lastStretch.off,
                ((*opt.wrapping_add(cur as usize)).litlen == 0) as U32,
            );
            ZSTD_memcpy(
                rep as *mut u8,
                &reps as *const Repcodes_t as *const u8,
                core::mem::size_of::<Repcodes_t>() as size_t,
            );
        } else {
            ZSTD_memcpy(
                rep as *mut u8,
                lastStretch.rep.as_ptr() as *const u8,
                core::mem::size_of::<Repcodes_t>() as size_t,
            );
            cur -= lastStretch.litlen;
        }

        /* Let's write the shortest path solution.
         * It is stored in @opt in reverse order, starting from @storeEnd (==cur+2). */
        {
            let storeEnd: U32 = cur + 2;
            let mut storeStart: U32; // = storeEnd; (set below)
            let mut stretchPos: U32 = cur;

            if lastStretch.litlen > 0 {
                /* last "sequence" is unfinished: just a bunch of literals */
                (*opt.wrapping_add(storeEnd as usize)).litlen = lastStretch.litlen;
                (*opt.wrapping_add(storeEnd as usize)).mlen = 0;
                storeStart = storeEnd - 1;
                *opt.wrapping_add(storeStart as usize) = lastStretch;
            }
            {
                *opt.wrapping_add(storeEnd as usize) = lastStretch; /* note: litlen will be fixed */
                storeStart = storeEnd;
            }
            loop {
                let nextStretch: ZSTD_optimal_t = *opt.wrapping_add(stretchPos as usize);
                (*opt.wrapping_add(storeStart as usize)).litlen = nextStretch.litlen;
                if nextStretch.mlen == 0 {
                    /* reaching beginning of segment */
                    break;
                }
                storeStart -= 1;
                *opt.wrapping_add(storeStart as usize) = nextStretch; /* note: litlen will be fixed */
                stretchPos -= nextStretch.litlen + nextStretch.mlen;
            }

            /* save sequences */
            {
                let mut storePos: U32;
                storePos = storeStart;
                while storePos <= storeEnd {
                    let llen: U32 = (*opt.wrapping_add(storePos as usize)).litlen;
                    let mlen: U32 = (*opt.wrapping_add(storePos as usize)).mlen;
                    let offBase: U32 = (*opt.wrapping_add(storePos as usize)).off;
                    let advance: U32 = llen + mlen;

                    if mlen == 0 {
                        /* only literals => must be last "sequence", starting a new stream */
                        ip = anchor.wrapping_add(llen as usize); /* don't progress anchor */
                        storePos += 1;
                        continue; /* will finish */
                    }

                    ZSTD_updateStats(optStatePtr, llen, anchor, offBase, mlen);
                    ZSTD_storeSeq(
                        seqStore,
                        llen as size_t,
                        anchor,
                        iend,
                        offBase,
                        mlen as size_t,
                    );
                    anchor = anchor.wrapping_add(advance as usize);
                    ip = anchor;
                    storePos += 1;
                }
            }

            /* update all costs */
            ZSTD_setBasePrices(optStatePtr, optLevel);
        }
    } /* while (ip < ilimit) */

    /* Return the last literals size */
    (iend.offset_from(anchor)) as size_t
}

unsafe fn ZSTD_compressBlock_opt0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    ZSTD_compressBlock_opt_generic(ms, seqStore, rep, src, srcSize, 0 /* optLevel */, dictMode)
}

unsafe fn ZSTD_compressBlock_opt2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    ZSTD_compressBlock_opt_generic(ms, seqStore, rep, src, srcSize, 2 /* optLevel */, dictMode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btopt(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_opt0(ms, seqStore, rep, src, srcSize, ZSTD_noDict)
}

/* ZSTD_initStats_ultra():
 * make a first compression pass, just to seed stats with more accurate starting values.
 * only works on first block, with no dictionary and no ldm. */
unsafe fn ZSTD_initStats_ultra(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) {
    let mut tmpRep: [U32; ZSTD_REP_NUM] = [0; ZSTD_REP_NUM]; /* updated rep codes will sink here */
    ZSTD_memcpy(
        tmpRep.as_mut_ptr() as *mut u8,
        rep as *const u8,
        core::mem::size_of_val(&tmpRep) as size_t,
    );

    ZSTD_compressBlock_opt2(ms, seqStore, tmpRep.as_mut_ptr(), src, srcSize, ZSTD_noDict); /* generate stats into ms->opt*/

    /* invalidate first scan from history, only keep entropy stats */
    ZSTD_resetSeqStore(seqStore);
    (*ms).window.base = (*ms).window.base.wrapping_sub(srcSize);
    (*ms).window.dictLimit += srcSize as U32;
    (*ms).window.lowLimit = (*ms).window.dictLimit;
    (*ms).nextToUpdate = (*ms).window.dictLimit;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btultra(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_opt2(ms, seqStore, rep, src, srcSize, ZSTD_noDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btultra2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let curr: U32 = (src as *const BYTE).offset_from((*ms).window.base) as U32;

    /* 2-passes strategy */
    if ((*ms).opt.litLengthSum == 0)   /* first block */
        && ((*seqStore).sequences == (*seqStore).sequencesStart)  /* no ldm */
        && ((*ms).window.dictLimit == (*ms).window.lowLimit)   /* no dictionary */
        && (curr == (*ms).window.dictLimit)    /* start of frame, nothing already loaded nor skipped */
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
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_opt0(ms, seqStore, rep, src, srcSize, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btopt_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_opt0(ms, seqStore, rep, src, srcSize, ZSTD_extDict)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btultra_dictMatchState(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_opt2(ms, seqStore, rep, src, srcSize, ZSTD_dictMatchState)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btultra_extDict(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_opt2(ms, seqStore, rep, src, srcSize, ZSTD_extDict)
}
