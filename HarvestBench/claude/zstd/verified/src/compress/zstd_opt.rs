//! Faithful translation of compress/zstd_opt.c — the optimal parser.
//!
//! Build config: DYNAMIC_BMI2=0, single-threaded, LE 64-bit. Byte-identical output.
//! The price model MUST be byte-identical or compressed output differs.
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use core::ffi::c_void;

use crate::common::bits::highbit32 as ZSTD_highbit32;
use crate::common::fse::{fse_get_max_nb_bits as FSE_getMaxNbBits, fse_init_cstate, FSE_CState_t};
use crate::common::mem::{mem_is_little_endian, mem_read32 as MEM_read32, U32};
use crate::common::zstd_internal::{
    LL_bits, ML_bits, MaxLL, MaxLit, MaxML, MaxOff, MINMATCH, ZSTD_OPT_NUM, ZSTD_REP_NUM,
};
use crate::compress::hist::HIST_count_simple;
use crate::compress::huf_compress::HUF_getNbBitsFromCTable;
use crate::compress::zstd_compress_internal::{
    kNullRawSeqStore, optState_t, rawSeq, zop_dynamic, zop_predef, RawSeqStore_t,
    SeqStore_t, ZSTD_MatchState_t, ZSTD_count, ZSTD_count_2segments, ZSTD_dictMatchState,
    ZSTD_dictMode_e, ZSTD_extDict, ZSTD_getLowestMatchIndex, ZSTD_hash3Ptr, ZSTD_hashPtr,
    ZSTD_index_overlap_check, ZSTD_match_t, ZSTD_newRep, ZSTD_noDict, ZSTD_optimal_t,
    ZSTD_storeSeq, OFFSET_TO_OFFBASE, REPCODE_TO_OFFBASE, ZSTD_LLcode, ZSTD_MLcode,
    ZSTD_ps_disable,
};
use crate::zstd_h::ZSTD_BLOCKSIZE_MAX;

// ZSTD_resetSeqStore is defined in zstd_compress.c (not yet translated here).
extern "C" {
    fn ZSTD_resetSeqStore(ssPtr: *mut SeqStore_t);
}

const ZSTD_LITFREQ_ADD: U32 = 2;
const ZSTD_MAX_PRICE: i32 = 1 << 30;
const ZSTD_PREDEF_THRESHOLD: usize = 8;

const BITCOST_ACCURACY: u32 = 8;
const BITCOST_MULTIPLIER: U32 = 1 << BITCOST_ACCURACY;

const HUF_repeat_valid: u32 = 2;

/*-*************************************
*  Price functions for optimal parser
***************************************/

/* ZSTD_bitWeight() : estimated "cost" of a stat in full bits only */
#[inline]
fn ZSTD_bitWeight(stat: U32) -> U32 {
    ZSTD_highbit32(stat + 1) * BITCOST_MULTIPLIER
}

/* ZSTD_fracWeight() : fractional-bit "cost" of a stat, linear interpolation */
#[inline]
fn ZSTD_fracWeight(rawStat: U32) -> U32 {
    let stat = rawStat + 1;
    let hb = ZSTD_highbit32(stat);
    let BWeight = hb * BITCOST_MULTIPLIER;
    let FWeight = (stat << BITCOST_ACCURACY) >> hb;
    let weight = BWeight + FWeight;
    debug_assert!(hb + BITCOST_ACCURACY < 31);
    weight
}

#[inline]
fn WEIGHT(stat: U32, opt: i32) -> U32 {
    if opt != 0 {
        ZSTD_fracWeight(stat)
    } else {
        ZSTD_bitWeight(stat)
    }
}

fn ZSTD_compressedLiterals(optPtr: *const optState_t) -> i32 {
    (unsafe { (*optPtr).literalCompressionMode } != ZSTD_ps_disable) as i32
}

unsafe fn ZSTD_setBasePrices(optPtr: *mut optState_t, optLevel: i32) {
    if ZSTD_compressedLiterals(optPtr) != 0 {
        (*optPtr).litSumBasePrice = WEIGHT((*optPtr).litSum, optLevel);
    }
    (*optPtr).litLengthSumBasePrice = WEIGHT((*optPtr).litLengthSum, optLevel);
    (*optPtr).matchLengthSumBasePrice = WEIGHT((*optPtr).matchLengthSum, optLevel);
    (*optPtr).offCodeSumBasePrice = WEIGHT((*optPtr).offCodeSum, optLevel);
}

unsafe fn sum_u32(table: *const u32, nbElts: usize) -> U32 {
    let mut total: U32 = 0;
    let mut n = 0usize;
    while n < nbElts {
        total += *table.add(n);
        n += 1;
    }
    total
}

type base_directive_e = u32;
const base_0possible: base_directive_e = 0;
const base_1guaranteed: base_directive_e = 1;

unsafe fn ZSTD_downscaleStats(
    table: *mut u32,
    lastEltIndex: U32,
    shift: U32,
    base1: base_directive_e,
) -> U32 {
    let mut sum: U32 = 0;
    debug_assert!(shift < 30);
    let mut s: U32 = 0;
    while s < lastEltIndex + 1 {
        let base = if base1 != 0 {
            1u32
        } else {
            (*table.add(s as usize) > 0) as u32
        };
        let newStat = base + (*table.add(s as usize) >> shift);
        sum += newStat;
        *table.add(s as usize) = newStat;
        s += 1;
    }
    sum
}

/* ZSTD_scaleStats() : reduce all elt frequencies if sum too large */
unsafe fn ZSTD_scaleStats(table: *mut u32, lastEltIndex: U32, logTarget: U32) -> U32 {
    let prevsum = sum_u32(table, (lastEltIndex + 1) as usize);
    let factor = prevsum >> logTarget;
    debug_assert!(logTarget < 30);
    if factor <= 1 {
        return prevsum;
    }
    ZSTD_downscaleStats(table, lastEltIndex, ZSTD_highbit32(factor), base_1guaranteed)
}

/* ZSTD_rescaleFreqs() :
 * if first block (detected by optPtr->litLengthSum == 0) : init statistics
 * otherwise downscale existing stats, to be used as seed for next block. */
unsafe fn ZSTD_rescaleFreqs(
    optPtr: *mut optState_t,
    src: *const u8,
    srcSize: usize,
    optLevel: i32,
) {
    let compressedLiterals = ZSTD_compressedLiterals(optPtr);
    (*optPtr).priceType = zop_dynamic;

    if (*optPtr).litLengthSum == 0 {
        /* no literals stats collected -> first block assumed -> init */

        /* heuristic: use pre-defined stats for too small inputs */
        if srcSize <= ZSTD_PREDEF_THRESHOLD {
            (*optPtr).priceType = zop_predef;
        }

        debug_assert!(!(*optPtr).symbolCosts.is_null());
        if (*(*optPtr).symbolCosts).huf.repeatMode == HUF_repeat_valid {
            /* huffman stats covering the full value set : table presumed generated by dictionary */
            (*optPtr).priceType = zop_dynamic;

            if compressedLiterals != 0 {
                /* generate literals statistics from huffman table */
                debug_assert!(!(*optPtr).litFreq.is_null());
                (*optPtr).litSum = 0;
                let mut lit: u32 = 0;
                while lit <= MaxLit {
                    let scaleLog: U32 = 11; /* scale to 2K */
                    let bitCost =
                        HUF_getNbBitsFromCTable((*(*optPtr).symbolCosts).huf.CTable.as_ptr(), lit);
                    debug_assert!(bitCost <= scaleLog);
                    *(*optPtr).litFreq.add(lit as usize) =
                        if bitCost != 0 { 1 << (scaleLog - bitCost) } else { 1 };
                    (*optPtr).litSum += *(*optPtr).litFreq.add(lit as usize);
                    lit += 1;
                }
            }

            {
                let mut llstate: FSE_CState_t = core::mem::zeroed();
                fse_init_cstate(
                    &mut llstate,
                    (*(*optPtr).symbolCosts).fse.litlengthCTable.as_ptr(),
                );
                (*optPtr).litLengthSum = 0;
                let mut ll: u32 = 0;
                while ll <= MaxLL {
                    let scaleLog: U32 = 10; /* scale to 1K */
                    let bitCost = FSE_getMaxNbBits(llstate.symbolTT, ll);
                    debug_assert!(bitCost < scaleLog);
                    *(*optPtr).litLengthFreq.add(ll as usize) =
                        if bitCost != 0 { 1 << (scaleLog - bitCost) } else { 1 };
                    (*optPtr).litLengthSum += *(*optPtr).litLengthFreq.add(ll as usize);
                    ll += 1;
                }
            }

            {
                let mut mlstate: FSE_CState_t = core::mem::zeroed();
                fse_init_cstate(
                    &mut mlstate,
                    (*(*optPtr).symbolCosts).fse.matchlengthCTable.as_ptr(),
                );
                (*optPtr).matchLengthSum = 0;
                let mut ml: u32 = 0;
                while ml <= MaxML {
                    let scaleLog: U32 = 10;
                    let bitCost = FSE_getMaxNbBits(mlstate.symbolTT, ml);
                    debug_assert!(bitCost < scaleLog);
                    *(*optPtr).matchLengthFreq.add(ml as usize) =
                        if bitCost != 0 { 1 << (scaleLog - bitCost) } else { 1 };
                    (*optPtr).matchLengthSum += *(*optPtr).matchLengthFreq.add(ml as usize);
                    ml += 1;
                }
            }

            {
                let mut ofstate: FSE_CState_t = core::mem::zeroed();
                fse_init_cstate(
                    &mut ofstate,
                    (*(*optPtr).symbolCosts).fse.offcodeCTable.as_ptr(),
                );
                (*optPtr).offCodeSum = 0;
                let mut of: u32 = 0;
                while of <= MaxOff {
                    let scaleLog: U32 = 10;
                    let bitCost = FSE_getMaxNbBits(ofstate.symbolTT, of);
                    debug_assert!(bitCost < scaleLog);
                    *(*optPtr).offCodeFreq.add(of as usize) =
                        if bitCost != 0 { 1 << (scaleLog - bitCost) } else { 1 };
                    (*optPtr).offCodeSum += *(*optPtr).offCodeFreq.add(of as usize);
                    of += 1;
                }
            }
        } else {
            /* first block, no dictionary */
            debug_assert!(!(*optPtr).litFreq.is_null());
            if compressedLiterals != 0 {
                /* base initial cost of literals on direct frequency within src */
                let mut lit: u32 = MaxLit;
                HIST_count_simple(
                    (*optPtr).litFreq,
                    &mut lit,
                    src as *const c_void,
                    srcSize,
                );
                (*optPtr).litSum =
                    ZSTD_downscaleStats((*optPtr).litFreq, MaxLit, 8, base_0possible);
            }

            {
                let baseLLfreqs: [u32; (MaxLL + 1) as usize] = [
                    4, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                ];
                core::ptr::copy_nonoverlapping(
                    baseLLfreqs.as_ptr(),
                    (*optPtr).litLengthFreq,
                    (MaxLL + 1) as usize,
                );
                (*optPtr).litLengthSum = sum_u32(baseLLfreqs.as_ptr(), (MaxLL + 1) as usize);
            }

            {
                let mut ml: u32 = 0;
                while ml <= MaxML {
                    *(*optPtr).matchLengthFreq.add(ml as usize) = 1;
                    ml += 1;
                }
            }
            (*optPtr).matchLengthSum = MaxML + 1;

            {
                let baseOFCfreqs: [u32; (MaxOff + 1) as usize] = [
                    6, 2, 1, 1, 2, 3, 4, 4, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1,
                ];
                core::ptr::copy_nonoverlapping(
                    baseOFCfreqs.as_ptr(),
                    (*optPtr).offCodeFreq,
                    (MaxOff + 1) as usize,
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

/* ZSTD_rawLiteralsCost() : price of literals (only) in specified segment.
 * does not include price of literalLength symbol */
unsafe fn ZSTD_rawLiteralsCost(
    literals: *const u8,
    litLength: U32,
    optPtr: *const optState_t,
    optLevel: i32,
) -> U32 {
    if litLength == 0 {
        return 0;
    }

    if ZSTD_compressedLiterals(optPtr) == 0 {
        return (litLength << 3) * BITCOST_MULTIPLIER; /* Uncompressed - 8 bytes per literal. */
    }

    if (*optPtr).priceType == zop_predef {
        return (litLength * 6) * BITCOST_MULTIPLIER; /* 6 bit per literal - no statistic used */
    }

    /* dynamic statistics */
    {
        let mut price: U32 = (*optPtr).litSumBasePrice * litLength;
        let litPriceMax = (*optPtr).litSumBasePrice - BITCOST_MULTIPLIER;
        debug_assert!((*optPtr).litSumBasePrice >= BITCOST_MULTIPLIER);
        let mut u: U32 = 0;
        while u < litLength {
            let mut litPrice = WEIGHT((*(*optPtr).litFreq.add(*literals.add(u as usize) as usize)), optLevel);
            if litPrice > litPriceMax {
                litPrice = litPriceMax;
            }
            price -= litPrice;
            u += 1;
        }
        price
    }
}

/* ZSTD_litLengthPrice() : cost of literalLength symbol */
unsafe fn ZSTD_litLengthPrice(litLength: U32, optPtr: *const optState_t, optLevel: i32) -> U32 {
    debug_assert!(litLength as usize <= ZSTD_BLOCKSIZE_MAX);
    if (*optPtr).priceType == zop_predef {
        return WEIGHT(litLength, optLevel);
    }

    /* ZSTD_LLcode() can't compute litLength price for sizes >= ZSTD_BLOCKSIZE_MAX */
    if litLength as usize == ZSTD_BLOCKSIZE_MAX {
        return BITCOST_MULTIPLIER
            + ZSTD_litLengthPrice((ZSTD_BLOCKSIZE_MAX - 1) as U32, optPtr, optLevel);
    }

    /* dynamic statistics */
    {
        let llCode = ZSTD_LLcode(litLength);
        (LL_bits[llCode as usize] as U32 * BITCOST_MULTIPLIER)
            + (*optPtr).litLengthSumBasePrice
            - WEIGHT(*(*optPtr).litLengthFreq.add(llCode as usize), optLevel)
    }
}

/* ZSTD_getMatchPrice() : cost of the match part (offset + matchLength) of a sequence. */
#[inline]
unsafe fn ZSTD_getMatchPrice(
    offBase: U32,
    matchLength: U32,
    optPtr: *const optState_t,
    optLevel: i32,
) -> U32 {
    let mut price: U32;
    let offCode = ZSTD_highbit32(offBase);
    let mlBase = matchLength - MINMATCH;
    debug_assert!(matchLength >= MINMATCH);

    if (*optPtr).priceType == zop_predef {
        /* fixed scheme, does not use statistics */
        return WEIGHT(mlBase, optLevel) + ((16 + offCode) * BITCOST_MULTIPLIER);
    }

    /* dynamic statistics */
    price = (offCode * BITCOST_MULTIPLIER)
        + ((*optPtr).offCodeSumBasePrice - WEIGHT(*(*optPtr).offCodeFreq.add(offCode as usize), optLevel));
    if (optLevel < 2) && offCode >= 20 {
        price += (offCode - 19) * 2 * BITCOST_MULTIPLIER; /* handicap for long distance offsets */
    }

    /* match Length */
    {
        let mlCode = ZSTD_MLcode(mlBase);
        price += (ML_bits[mlCode as usize] as U32 * BITCOST_MULTIPLIER)
            + ((*optPtr).matchLengthSumBasePrice - WEIGHT(*(*optPtr).matchLengthFreq.add(mlCode as usize), optLevel));
    }

    price += BITCOST_MULTIPLIER / 5; /* heuristic : make matches a bit more costly */

    price
}

/* ZSTD_updateStats() : assumption : literals + litLength <= iend */
unsafe fn ZSTD_updateStats(
    optPtr: *mut optState_t,
    litLength: U32,
    literals: *const u8,
    offBase: U32,
    matchLength: U32,
) {
    /* literals */
    if ZSTD_compressedLiterals(optPtr) != 0 {
        let mut u: U32 = 0;
        while u < litLength {
            *(*optPtr).litFreq.add(*literals.add(u as usize) as usize) += ZSTD_LITFREQ_ADD;
            u += 1;
        }
        (*optPtr).litSum += litLength * ZSTD_LITFREQ_ADD;
    }

    /* literal Length */
    {
        let llCode = ZSTD_LLcode(litLength);
        *(*optPtr).litLengthFreq.add(llCode as usize) += 1;
        (*optPtr).litLengthSum += 1;
    }

    /* offset code : follows storeSeq() numeric representation */
    {
        let offCode = ZSTD_highbit32(offBase);
        debug_assert!(offCode <= MaxOff);
        *(*optPtr).offCodeFreq.add(offCode as usize) += 1;
        (*optPtr).offCodeSum += 1;
    }

    /* match Length */
    {
        let mlBase = matchLength - MINMATCH;
        let mlCode = ZSTD_MLcode(mlBase);
        *(*optPtr).matchLengthFreq.add(mlCode as usize) += 1;
        (*optPtr).matchLengthSum += 1;
    }
}

/* ZSTD_readMINMATCH() : function safe only for comparisons */
#[inline]
unsafe fn ZSTD_readMINMATCH(memPtr: *const c_void, length: U32) -> U32 {
    match length {
        3 => {
            if mem_is_little_endian() != 0 {
                MEM_read32(memPtr) << 8
            } else {
                MEM_read32(memPtr) >> 8
            }
        }
        _ => MEM_read32(memPtr),
    }
}

/* Update hashTable3 up to ip (excluded). Assumption : always within prefix. */
unsafe fn ZSTD_insertAndFindFirstIndexHash3(
    ms: *const ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const u8,
) -> U32 {
    let hashTable3 = (*ms).hashTable3;
    let hashLog3 = (*ms).hashLog3;
    let base = (*ms).window.base;
    let mut idx = *nextToUpdate3;
    let target = ip.offset_from(base) as U32;
    let hash3 = ZSTD_hash3Ptr(ip as *const c_void, hashLog3);
    debug_assert!(hashLog3 > 0);

    while idx < target {
        *hashTable3.add(ZSTD_hash3Ptr(base.add(idx as usize) as *const c_void, hashLog3)) = idx;
        idx += 1;
    }

    *nextToUpdate3 = target;
    *hashTable3.add(hash3)
}

/*-*************************************
*  Binary Tree search
***************************************/
/** ZSTD_insertBt1() : add one or multiple positions to tree.
 * @return : nb of positions added */
unsafe fn ZSTD_insertBt1(
    ms: *const ZSTD_MatchState_t,
    ip: *const u8,
    iend: *const u8,
    target: U32,
    mls: U32,
    extDict: i32,
) -> U32 {
    let cParams = &(*ms).cParams;
    let hashTable = (*ms).hashTable;
    let hashLog = cParams.hashLog;
    let h = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let bt = (*ms).chainTable;
    let btLog = cParams.chainLog - 1;
    let btMask = (1u32 << btLog) - 1;
    let mut matchIndex = *hashTable.add(h);
    let mut commonLengthSmaller: usize = 0;
    let mut commonLengthLarger: usize = 0;
    let base = (*ms).window.base;
    let dictBase = (*ms).window.dictBase;
    let dictLimit = (*ms).window.dictLimit;
    let dictEnd = dictBase.add(dictLimit as usize);
    let prefixStart = base.add(dictLimit as usize);
    let mut r#match: *const u8;
    let curr = ip.offset_from(base) as U32;
    let btLow = if btMask >= curr { 0 } else { curr - btMask };
    let mut smallerPtr = bt.add(2 * (curr & btMask) as usize);
    let mut largerPtr = smallerPtr.add(1);
    let mut dummy32: U32 = 0;
    let windowLow = ZSTD_getLowestMatchIndex(ms, target, cParams.windowLog);
    let mut matchEndIdx = curr + 8 + 1;
    let mut bestLength: usize = 8;
    let mut nbCompares = 1u32 << cParams.searchLog;

    debug_assert!(curr <= target);
    debug_assert!(ip <= iend.offset(-8));
    *hashTable.add(h) = curr; /* Update Hash Table */

    debug_assert!(windowLow > 0);
    while nbCompares != 0 && matchIndex >= windowLow {
        let nextPtr = bt.add(2 * (matchIndex & btMask) as usize);
        let mut matchLength = commonLengthSmaller.min(commonLengthLarger);
        debug_assert!(matchIndex < curr);

        if extDict == 0 || (matchIndex as usize + matchLength >= dictLimit as usize) {
            debug_assert!(matchIndex as usize + matchLength >= dictLimit as usize);
            r#match = base.add(matchIndex as usize);
            matchLength += ZSTD_count(
                ip.add(matchLength),
                r#match.add(matchLength),
                iend,
            );
        } else {
            r#match = dictBase.add(matchIndex as usize);
            matchLength += ZSTD_count_2segments(
                ip.add(matchLength),
                r#match.add(matchLength),
                iend,
                dictEnd,
                prefixStart,
            );
            if matchIndex as usize + matchLength >= dictLimit as usize {
                r#match = base.add(matchIndex as usize);
            }
        }

        if matchLength > bestLength {
            bestLength = matchLength;
            if matchLength as u32 > matchEndIdx - matchIndex {
                matchEndIdx = matchIndex + matchLength as U32;
            }
        }

        if ip.add(matchLength) == iend {
            /* equal : no way to know if inf or sup */
            break;
        }

        if *r#match.add(matchLength) < *ip.add(matchLength) {
            /* match is smaller than current */
            *smallerPtr = matchIndex;
            commonLengthSmaller = matchLength;
            if matchIndex <= btLow {
                smallerPtr = &mut dummy32;
                break;
            }
            smallerPtr = nextPtr.add(1);
            matchIndex = *nextPtr.add(1);
        } else {
            /* match is larger than current */
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &mut dummy32;
                break;
            }
            largerPtr = nextPtr;
            matchIndex = *nextPtr.add(0);
        }
        nbCompares -= 1;
    }

    *smallerPtr = 0;
    *largerPtr = 0;
    {
        let mut positions: U32 = 0;
        if bestLength > 384 {
            positions = 192u32.min((bestLength - 384) as U32);
        }
        debug_assert!(matchEndIdx > curr + 8);
        positions.max(matchEndIdx - (curr + 8))
    }
}

#[inline]
unsafe fn ZSTD_updateTree_internal(
    ms: *mut ZSTD_MatchState_t,
    ip: *const u8,
    iend: *const u8,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) {
    let base = (*ms).window.base;
    let target = ip.offset_from(base) as U32;
    let mut idx = (*ms).nextToUpdate;

    while idx < target {
        let forward = ZSTD_insertBt1(
            ms,
            base.add(idx as usize),
            iend,
            target,
            mls,
            (dictMode == ZSTD_extDict) as i32,
        );
        debug_assert!(idx < idx.wrapping_add(forward));
        idx += forward;
    }
    (*ms).nextToUpdate = target;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_updateTree(
    ms: *mut ZSTD_MatchState_t,
    ip: *const u8,
    iend: *const u8,
) {
    ZSTD_updateTree_internal(ms, ip, iend, (*ms).cParams.minMatch, ZSTD_noDict);
}

#[inline]
unsafe fn ZSTD_insertBtAndGetAllMatches(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const u8,
    iLimit: *const u8,
    dictMode: ZSTD_dictMode_e,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
    mls: U32,
) -> U32 {
    let cParams = &(*ms).cParams;
    let sufficient_len = cParams.targetLength.min((ZSTD_OPT_NUM - 1) as U32);
    let base = (*ms).window.base;
    let curr = ip.offset_from(base) as U32;
    let hashLog = cParams.hashLog;
    let minMatch = if mls == 3 { 3u32 } else { 4u32 };
    let hashTable = (*ms).hashTable;
    let h = ZSTD_hashPtr(ip as *const c_void, hashLog, mls);
    let mut matchIndex = *hashTable.add(h);
    let bt = (*ms).chainTable;
    let btLog = cParams.chainLog - 1;
    let btMask = (1u32 << btLog) - 1;
    let mut commonLengthSmaller: usize = 0;
    let mut commonLengthLarger: usize = 0;
    let dictBase = (*ms).window.dictBase;
    let dictLimit = (*ms).window.dictLimit;
    let dictEnd = dictBase.add(dictLimit as usize);
    let prefixStart = base.add(dictLimit as usize);
    let btLow = if btMask >= curr { 0 } else { curr - btMask };
    let windowLow = ZSTD_getLowestMatchIndex(ms, curr, cParams.windowLog);
    let matchLow = if windowLow != 0 { windowLow } else { 1 };
    let mut smallerPtr = bt.add(2 * (curr & btMask) as usize);
    let mut largerPtr = bt.add(2 * (curr & btMask) as usize + 1);
    let mut matchEndIdx = curr + 8 + 1;
    let mut dummy32: U32 = 0;
    let mut mnum: U32 = 0;
    let mut nbCompares = 1u32 << cParams.searchLog;

    let dms: *const ZSTD_MatchState_t = if dictMode == ZSTD_dictMatchState {
        (*ms).dictMatchState
    } else {
        core::ptr::null()
    };
    let dmsBase: *const u8 = if dictMode == ZSTD_dictMatchState {
        (*dms).window.base
    } else {
        core::ptr::null()
    };
    let dmsEnd: *const u8 = if dictMode == ZSTD_dictMatchState {
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
        (*dms).cParams.hashLog
    } else {
        hashLog
    };
    let dmsBtLog: U32 = if dictMode == ZSTD_dictMatchState {
        (*dms).cParams.chainLog - 1
    } else {
        btLog
    };
    let dmsBtMask: U32 = if dictMode == ZSTD_dictMatchState {
        (1u32 << dmsBtLog) - 1
    } else {
        0
    };
    let dmsBtLow: U32 = if dictMode == ZSTD_dictMatchState && dmsBtMask < dmsHighLimit - dmsLowLimit
    {
        dmsHighLimit - dmsBtMask
    } else {
        dmsLowLimit
    };

    let mut bestLength: usize = (lengthToBeat - 1) as usize;

    /* check repCode */
    debug_assert!(ll0 <= 1);
    {
        let lastR = ZSTD_REP_NUM as U32 + ll0;
        let mut repCode = ll0;
        while repCode < lastR {
            let repOffset = if repCode == ZSTD_REP_NUM as U32 {
                *rep.add(0) - 1
            } else {
                *rep.add(repCode as usize)
            };
            let repIndex = curr - repOffset;
            let mut repLen: U32 = 0;
            debug_assert!(curr >= dictLimit);
            if repOffset.wrapping_sub(1) < curr - dictLimit {
                /* equivalent to `curr > repIndex >= dictLimit` */
                if (repIndex >= windowLow)
                    && (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(
                            ip.offset(-(repOffset as isize)) as *const c_void,
                            minMatch,
                        ))
                {
                    repLen = ZSTD_count(
                        ip.add(minMatch as usize),
                        ip.add(minMatch as usize).offset(-(repOffset as isize)),
                        iLimit,
                    ) as U32
                        + minMatch;
                }
            } else {
                /* repIndex < dictLimit || repIndex >= curr */
                let repMatch: *const u8 = if dictMode == ZSTD_dictMatchState {
                    dmsBase.offset(repIndex as isize - dmsIndexDelta as isize)
                } else {
                    dictBase.add(repIndex as usize)
                };
                debug_assert!(curr >= windowLow);
                if dictMode == ZSTD_extDict
                    && ((repOffset.wrapping_sub(1) < curr - windowLow)
                        && (ZSTD_index_overlap_check(dictLimit, repIndex) != 0))
                    && (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(repMatch as *const c_void, minMatch))
                {
                    repLen = ZSTD_count_2segments(
                        ip.add(minMatch as usize),
                        repMatch.add(minMatch as usize),
                        iLimit,
                        dictEnd,
                        prefixStart,
                    ) as U32
                        + minMatch;
                }
                if dictMode == ZSTD_dictMatchState
                    && ((repOffset.wrapping_sub(1) < curr - (dmsLowLimit + dmsIndexDelta))
                        && (ZSTD_index_overlap_check(dictLimit, repIndex) != 0))
                    && (ZSTD_readMINMATCH(ip as *const c_void, minMatch)
                        == ZSTD_readMINMATCH(repMatch as *const c_void, minMatch))
                {
                    repLen = ZSTD_count_2segments(
                        ip.add(minMatch as usize),
                        repMatch.add(minMatch as usize),
                        iLimit,
                        dmsEnd,
                        prefixStart,
                    ) as U32
                        + minMatch;
                }
            }
            /* save longer solution */
            if repLen > bestLength as U32 {
                bestLength = repLen as usize;
                (*matches.add(mnum as usize)).off = REPCODE_TO_OFFBASE(repCode - ll0 + 1);
                (*matches.add(mnum as usize)).len = repLen;
                mnum += 1;
                if (repLen > sufficient_len) || (ip.add(repLen as usize) == iLimit) {
                    return mnum;
                }
            }
            repCode += 1;
        }
    }

    /* HC3 match finder */
    if (mls == 3) && (bestLength < mls as usize) {
        let matchIndex3 = ZSTD_insertAndFindFirstIndexHash3(ms, nextToUpdate3, ip);
        if (matchIndex3 >= matchLow) && (curr - matchIndex3 < (1 << 18)) {
            let mlen: usize;
            if (dictMode == ZSTD_noDict)
                || (dictMode == ZSTD_dictMatchState)
                || (matchIndex3 >= dictLimit)
            {
                let r#match = base.add(matchIndex3 as usize);
                mlen = ZSTD_count(ip, r#match, iLimit);
            } else {
                let r#match = dictBase.add(matchIndex3 as usize);
                mlen = ZSTD_count_2segments(ip, r#match, iLimit, dictEnd, prefixStart);
            }

            /* save best solution */
            if mlen >= mls as usize {
                bestLength = mlen;
                debug_assert!(curr > matchIndex3);
                debug_assert!(mnum == 0);
                (*matches.add(0)).off = OFFSET_TO_OFFBASE(curr - matchIndex3);
                (*matches.add(0)).len = mlen as U32;
                mnum = 1;
                if (mlen > sufficient_len as usize) || (ip.add(mlen) == iLimit) {
                    (*ms).nextToUpdate = curr + 1; /* skip insertion */
                    return 1;
                }
            }
        }
    }

    *hashTable.add(h) = curr; /* Update Hash Table */

    while nbCompares != 0 && matchIndex >= matchLow {
        let nextPtr = bt.add(2 * (matchIndex & btMask) as usize);
        let mut r#match: *const u8;
        let mut matchLength = commonLengthSmaller.min(commonLengthLarger);
        debug_assert!(curr > matchIndex);

        if (dictMode == ZSTD_noDict)
            || (dictMode == ZSTD_dictMatchState)
            || (matchIndex as usize + matchLength >= dictLimit as usize)
        {
            debug_assert!(matchIndex as usize + matchLength >= dictLimit as usize);
            r#match = base.add(matchIndex as usize);
            matchLength += ZSTD_count(ip.add(matchLength), r#match.add(matchLength), iLimit);
        } else {
            r#match = dictBase.add(matchIndex as usize);
            matchLength += ZSTD_count_2segments(
                ip.add(matchLength),
                r#match.add(matchLength),
                iLimit,
                dictEnd,
                prefixStart,
            );
            if matchIndex as usize + matchLength >= dictLimit as usize {
                r#match = base.add(matchIndex as usize);
            }
        }

        if matchLength > bestLength {
            debug_assert!(matchEndIdx > matchIndex);
            if matchLength as U32 > matchEndIdx - matchIndex {
                matchEndIdx = matchIndex + matchLength as U32;
            }
            bestLength = matchLength;
            (*matches.add(mnum as usize)).off = OFFSET_TO_OFFBASE(curr - matchIndex);
            (*matches.add(mnum as usize)).len = matchLength as U32;
            mnum += 1;
            if (matchLength > ZSTD_OPT_NUM) || (ip.add(matchLength) == iLimit) {
                if dictMode == ZSTD_dictMatchState {
                    nbCompares = 0; /* break should also skip searching dms */
                }
                break;
            }
        }

        if *r#match.add(matchLength) < *ip.add(matchLength) {
            /* match smaller than current */
            *smallerPtr = matchIndex;
            commonLengthSmaller = matchLength;
            if matchIndex <= btLow {
                smallerPtr = &mut dummy32;
                break;
            }
            smallerPtr = nextPtr.add(1);
            matchIndex = *nextPtr.add(1);
        } else {
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &mut dummy32;
                break;
            }
            largerPtr = nextPtr;
            matchIndex = *nextPtr.add(0);
        }
        nbCompares -= 1;
    }

    *smallerPtr = 0;
    *largerPtr = 0;

    if dictMode == ZSTD_dictMatchState && nbCompares != 0 {
        let dmsH = ZSTD_hashPtr(ip as *const c_void, dmsHashLog, mls);
        let mut dictMatchIndex = *(*dms).hashTable.add(dmsH);
        let dmsBt = (*dms).chainTable;
        commonLengthSmaller = 0;
        commonLengthLarger = 0;
        while nbCompares != 0 && dictMatchIndex > dmsLowLimit {
            let nextPtr = dmsBt.add(2 * (dictMatchIndex & dmsBtMask) as usize);
            let mut matchLength = commonLengthSmaller.min(commonLengthLarger);
            let mut r#match = dmsBase.add(dictMatchIndex as usize);
            matchLength += ZSTD_count_2segments(
                ip.add(matchLength),
                r#match.add(matchLength),
                iLimit,
                dmsEnd,
                prefixStart,
            );
            if dictMatchIndex as usize + matchLength >= dmsHighLimit as usize {
                r#match = base.add(dictMatchIndex as usize + dmsIndexDelta as usize);
            }

            if matchLength > bestLength {
                matchIndex = dictMatchIndex + dmsIndexDelta;
                if matchLength as U32 > matchEndIdx - matchIndex {
                    matchEndIdx = matchIndex + matchLength as U32;
                }
                bestLength = matchLength;
                (*matches.add(mnum as usize)).off = OFFSET_TO_OFFBASE(curr - matchIndex);
                (*matches.add(mnum as usize)).len = matchLength as U32;
                mnum += 1;
                if (matchLength > ZSTD_OPT_NUM) || (ip.add(matchLength) == iLimit) {
                    break;
                }
            }

            if dictMatchIndex <= dmsBtLow {
                break;
            }
            if *r#match.add(matchLength) < *ip.add(matchLength) {
                commonLengthSmaller = matchLength;
                dictMatchIndex = *nextPtr.add(1);
            } else {
                commonLengthLarger = matchLength;
                dictMatchIndex = *nextPtr.add(0);
            }
            nbCompares -= 1;
        }
    }

    debug_assert!(matchEndIdx > curr + 8);
    (*ms).nextToUpdate = matchEndIdx - 8; /* skip repetitive patterns */
    mnum
}

type ZSTD_getAllMatchesFn = unsafe fn(
    *mut ZSTD_match_t,
    *mut ZSTD_MatchState_t,
    *mut U32,
    *const u8,
    *const u8,
    *const U32,
    U32,
    U32,
) -> U32;

#[inline]
unsafe fn ZSTD_btGetAllMatches_internal(
    matches: *mut ZSTD_match_t,
    ms: *mut ZSTD_MatchState_t,
    nextToUpdate3: *mut U32,
    ip: *const u8,
    iHighLimit: *const u8,
    rep: *const U32,
    ll0: U32,
    lengthToBeat: U32,
    dictMode: ZSTD_dictMode_e,
    mls: U32,
) -> U32 {
    if ip < (*ms).window.base.add((*ms).nextToUpdate as usize) {
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

macro_rules! gen_bt_get_all_matches {
    ($name:ident, $dictMode:expr, $mls:expr) => {
        unsafe fn $name(
            matches: *mut ZSTD_match_t,
            ms: *mut ZSTD_MatchState_t,
            nextToUpdate3: *mut U32,
            ip: *const u8,
            iHighLimit: *const u8,
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

gen_bt_get_all_matches!(ZSTD_btGetAllMatches_noDict_3, ZSTD_noDict, 3);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_noDict_4, ZSTD_noDict, 4);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_noDict_5, ZSTD_noDict, 5);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_noDict_6, ZSTD_noDict, 6);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_extDict_3, ZSTD_extDict, 3);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_extDict_4, ZSTD_extDict, 4);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_extDict_5, ZSTD_extDict, 5);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_extDict_6, ZSTD_extDict, 6);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_dictMatchState_3, ZSTD_dictMatchState, 3);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_dictMatchState_4, ZSTD_dictMatchState, 4);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_dictMatchState_5, ZSTD_dictMatchState, 5);
gen_bt_get_all_matches!(ZSTD_btGetAllMatches_dictMatchState_6, ZSTD_dictMatchState, 6);

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
    let mls = (*ms).cParams.minMatch.clamp(3, 6);
    debug_assert!((dictMode as u32) < 3);
    debug_assert!(mls - 3 < 4);
    getAllMatchesFns[dictMode as usize][(mls - 3) as usize]
}

/*************************
*  LDM helper functions  *
*************************/

/* Struct containing info needed to make decision about ldm inclusion */
struct ZSTD_optLdm_t {
    seqStore: RawSeqStore_t,
    startPosInBlock: U32,
    endPosInBlock: U32,
    offset: U32,
}

/* ZSTD_optLdm_skipRawSeqStoreBytes():
 * Moves forward in @rawSeqStore by @nbBytes. */
unsafe fn ZSTD_optLdm_skipRawSeqStoreBytes(rawSeqStore: *mut RawSeqStore_t, nbBytes: usize) {
    let mut currPos = ((*rawSeqStore).posInSequence + nbBytes) as U32;
    while currPos != 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let currSeq: rawSeq = *(*rawSeqStore).seq.add((*rawSeqStore).pos);
        if currPos >= currSeq.litLength + currSeq.matchLength {
            currPos -= currSeq.litLength + currSeq.matchLength;
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
 * Calculates the beginning and end of the next match in the current block. */
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
    currSeq = *(*optLdm).seqStore.seq.add((*optLdm).seqStore.pos);
    debug_assert!(
        (*optLdm).seqStore.posInSequence as U32 <= currSeq.litLength + currSeq.matchLength
    );
    currBlockEndPos = currPosInBlock + blockBytesRemaining;
    literalsBytesRemaining = if ((*optLdm).seqStore.posInSequence as U32) < currSeq.litLength {
        currSeq.litLength - (*optLdm).seqStore.posInSequence as U32
    } else {
        0
    };
    matchBytesRemaining = if literalsBytesRemaining == 0 {
        currSeq.matchLength - ((*optLdm).seqStore.posInSequence as U32 - currSeq.litLength)
    } else {
        currSeq.matchLength
    };

    /* If there are more literal bytes than bytes remaining in block, no ldm is possible */
    if literalsBytesRemaining >= blockBytesRemaining {
        (*optLdm).startPosInBlock = u32::MAX;
        (*optLdm).endPosInBlock = u32::MAX;
        ZSTD_optLdm_skipRawSeqStoreBytes(&mut (*optLdm).seqStore, blockBytesRemaining as usize);
        return;
    }

    (*optLdm).startPosInBlock = currPosInBlock + literalsBytesRemaining;
    (*optLdm).endPosInBlock = (*optLdm).startPosInBlock + matchBytesRemaining;
    (*optLdm).offset = currSeq.offset;

    if (*optLdm).endPosInBlock > currBlockEndPos {
        /* Match ends after the block ends, we can't use the whole match */
        (*optLdm).endPosInBlock = currBlockEndPos;
        ZSTD_optLdm_skipRawSeqStoreBytes(
            &mut (*optLdm).seqStore,
            (currBlockEndPos - currPosInBlock) as usize,
        );
    } else {
        /* Consume nb of bytes equal to size of sequence left */
        ZSTD_optLdm_skipRawSeqStoreBytes(
            &mut (*optLdm).seqStore,
            (literalsBytesRemaining + matchBytesRemaining) as usize,
        );
    }
}

/* ZSTD_optLdm_maybeAddMatch():
 * Adds a match if it's long enough into 'matches'. */
unsafe fn ZSTD_optLdm_maybeAddMatch(
    matches: *mut ZSTD_match_t,
    nbMatches: *mut U32,
    optLdm: *const ZSTD_optLdm_t,
    currPosInBlock: U32,
    minMatch: U32,
) {
    let posDiff = currPosInBlock - (*optLdm).startPosInBlock;
    let candidateMatchLength =
        (*optLdm).endPosInBlock - (*optLdm).startPosInBlock - posDiff;

    /* Ensure that current block position is not outside of the match */
    if currPosInBlock < (*optLdm).startPosInBlock
        || currPosInBlock >= (*optLdm).endPosInBlock
        || candidateMatchLength < minMatch
    {
        return;
    }

    if *nbMatches == 0
        || ((candidateMatchLength > (*matches.add((*nbMatches - 1) as usize)).len)
            && *nbMatches < ZSTD_OPT_NUM as U32)
    {
        let candidateOffBase = OFFSET_TO_OFFBASE((*optLdm).offset);
        (*matches.add(*nbMatches as usize)).len = candidateMatchLength;
        (*matches.add(*nbMatches as usize)).off = candidateOffBase;
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
            let posOvershoot = currPosInBlock - (*optLdm).endPosInBlock;
            ZSTD_optLdm_skipRawSeqStoreBytes(&mut (*optLdm).seqStore, posOvershoot as usize);
        }
        ZSTD_opt_getNextMatchAndUpdateSeqStore(optLdm, currPosInBlock, remainingBytes);
    }
    ZSTD_optLdm_maybeAddMatch(matches, nbMatches, optLdm, currPosInBlock, minMatch);
}

/*-*******************************
*  Optimal parser
*********************************/

#[inline]
unsafe fn ZSTD_compressBlock_opt_generic(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    optLevel: i32,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    let optStatePtr: *mut optState_t = &mut (*ms).opt;
    let istart = src as *const u8;
    let mut ip = istart;
    let mut anchor = istart;
    let iend = istart.add(srcSize);
    let ilimit = iend.offset(-8);
    let base = (*ms).window.base;
    let prefixStart = base.add((*ms).window.dictLimit as usize);
    let cParams = &(*ms).cParams;

    let getAllMatches = ZSTD_selectBtGetAllMatches(ms, dictMode);

    let sufficient_len = cParams.targetLength.min((ZSTD_OPT_NUM - 1) as U32);
    let minMatch: U32 = if cParams.minMatch == 3 { 3 } else { 4 };
    let mut nextToUpdate3: U32 = (*ms).nextToUpdate;

    let opt: *mut ZSTD_optimal_t = (*optStatePtr).priceTable;
    let matches: *mut ZSTD_match_t = (*optStatePtr).matchTable;
    let mut lastStretch: ZSTD_optimal_t = core::mem::zeroed();
    let mut optLdm: ZSTD_optLdm_t = ZSTD_optLdm_t {
        seqStore: kNullRawSeqStore,
        startPosInBlock: 0,
        endPosInBlock: 0,
        offset: 0,
    };

    // LIT_PRICE / LL_PRICE / LL_INCPRICE macros
    macro_rules! LIT_PRICE {
        ($p:expr) => {
            ZSTD_rawLiteralsCost($p, 1, optStatePtr, optLevel) as i32
        };
    }
    macro_rules! LL_PRICE {
        ($l:expr) => {
            ZSTD_litLengthPrice($l, optStatePtr, optLevel) as i32
        };
    }
    macro_rules! LL_INCPRICE {
        ($l:expr) => {
            (LL_PRICE!($l) - LL_PRICE!($l - 1))
        };
    }

    optLdm.seqStore = if !(*ms).ldmSeqStore.is_null() {
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
    debug_assert!(optLevel <= 2);
    ZSTD_rescaleFreqs(optStatePtr, src as *const u8, srcSize, optLevel);
    ip = ip.add((ip == prefixStart) as usize);

    /* Match Loop */
    'matchLoop: while ip < ilimit {
        let mut cur: U32;
        let mut last_pos: U32 = 0;

        'find: {
            /* find first match */
            {
                let litlen = ip.offset_from(anchor) as U32;
                let ll0 = (litlen == 0) as U32;
                let mut nbMatches =
                    getAllMatches(matches, ms, &mut nextToUpdate3, ip, iend, rep, ll0, minMatch);
                ZSTD_optLdm_processMatchCandidate(
                    &mut optLdm,
                    matches,
                    &mut nbMatches,
                    ip.offset_from(istart) as U32,
                    iend.offset_from(ip) as U32,
                    minMatch,
                );
                if nbMatches == 0 {
                    ip = ip.add(1);
                    continue 'matchLoop;
                }

                /* initialize opt[0] */
                (*opt.add(0)).mlen = 0;
                (*opt.add(0)).litlen = litlen;
                (*opt.add(0)).price = LL_PRICE!(litlen);
                core::ptr::copy_nonoverlapping(
                    rep,
                    (*opt.add(0)).rep.as_mut_ptr(),
                    ZSTD_REP_NUM,
                );

                /* large match -> immediate encoding */
                {
                    let maxML = (*matches.add((nbMatches - 1) as usize)).len;
                    let maxOffBase = (*matches.add((nbMatches - 1) as usize)).off;

                    if maxML > sufficient_len {
                        lastStretch.litlen = 0;
                        lastStretch.mlen = maxML;
                        lastStretch.off = maxOffBase;
                        cur = 0;
                        last_pos = maxML;
                        break 'find; /* goto _shortestPath */
                    }
                }

                /* set prices for first matches starting position == 0 */
                debug_assert!((*opt.add(0)).price >= 0);
                {
                    let mut pos: U32;
                    let mut matchNb: U32;
                    pos = 1;
                    while pos < minMatch {
                        (*opt.add(pos as usize)).price = ZSTD_MAX_PRICE;
                        (*opt.add(pos as usize)).mlen = 0;
                        (*opt.add(pos as usize)).litlen = litlen + pos;
                        pos += 1;
                    }
                    matchNb = 0;
                    while matchNb < nbMatches {
                        let offBase = (*matches.add(matchNb as usize)).off;
                        let end = (*matches.add(matchNb as usize)).len;
                        while pos <= end {
                            let matchPrice =
                                ZSTD_getMatchPrice(offBase, pos, optStatePtr, optLevel) as i32;
                            let sequencePrice = (*opt.add(0)).price + matchPrice;
                            (*opt.add(pos as usize)).mlen = pos;
                            (*opt.add(pos as usize)).off = offBase;
                            (*opt.add(pos as usize)).litlen = 0; /* end of match */
                            (*opt.add(pos as usize)).price = sequencePrice + LL_PRICE!(0);
                            pos += 1;
                        }
                        matchNb += 1;
                    }
                    last_pos = pos - 1;
                    (*opt.add(pos as usize)).price = ZSTD_MAX_PRICE;
                }
            }

            /* check further positions */
            cur = 1;
            'curloop: while cur <= last_pos {
                let inr = ip.add(cur as usize);
                debug_assert!(cur as usize <= ZSTD_OPT_NUM);

                /* Fix current position with one literal if cheaper */
                {
                    let litlen = (*opt.add((cur - 1) as usize)).litlen + 1;
                    let price = (*opt.add((cur - 1) as usize)).price
                        + LIT_PRICE!(ip.add((cur - 1) as usize))
                        + LL_INCPRICE!(litlen);
                    debug_assert!(price < 1000000000);
                    if price <= (*opt.add(cur as usize)).price {
                        let prevMatch = *opt.add(cur as usize);
                        *opt.add(cur as usize) = *opt.add((cur - 1) as usize);
                        (*opt.add(cur as usize)).litlen = litlen;
                        (*opt.add(cur as usize)).price = price;
                        if (optLevel >= 1)
                            && (prevMatch.litlen == 0)
                            && (LL_INCPRICE!(1) < 0)
                            && (ip.add(cur as usize) < iend)
                        {
                            /* check next position, in case it would be cheaper */
                            let with1literal =
                                prevMatch.price + LIT_PRICE!(ip.add(cur as usize)) + LL_INCPRICE!(1);
                            let withMoreLiterals = price
                                + LIT_PRICE!(ip.add(cur as usize))
                                + LL_INCPRICE!(litlen + 1);
                            if (with1literal < withMoreLiterals)
                                && (with1literal < (*opt.add((cur + 1) as usize)).price)
                            {
                                /* update offset history - before it disappears */
                                let prev = cur - prevMatch.mlen;
                                let newReps = ZSTD_newRep(
                                    (*opt.add(prev as usize)).rep.as_ptr(),
                                    prevMatch.off,
                                    ((*opt.add(prev as usize)).litlen == 0) as U32,
                                );
                                debug_assert!(cur >= prevMatch.mlen);
                                *opt.add((cur + 1) as usize) = prevMatch; /* mlen & offbase */
                                core::ptr::copy_nonoverlapping(
                                    newReps.rep.as_ptr(),
                                    (*opt.add((cur + 1) as usize)).rep.as_mut_ptr(),
                                    ZSTD_REP_NUM,
                                );
                                (*opt.add((cur + 1) as usize)).litlen = 1;
                                (*opt.add((cur + 1) as usize)).price = with1literal;
                                if last_pos < cur + 1 {
                                    last_pos = cur + 1;
                                }
                            }
                        }
                    }
                }

                /* Offset history is not updated during match comparison. Do it here. */
                debug_assert!(cur >= (*opt.add(cur as usize)).mlen);
                if (*opt.add(cur as usize)).litlen == 0 {
                    /* just finished a match => alter offset history */
                    let prev = cur - (*opt.add(cur as usize)).mlen;
                    let newReps = ZSTD_newRep(
                        (*opt.add(prev as usize)).rep.as_ptr(),
                        (*opt.add(cur as usize)).off,
                        ((*opt.add(prev as usize)).litlen == 0) as U32,
                    );
                    core::ptr::copy_nonoverlapping(
                        newReps.rep.as_ptr(),
                        (*opt.add(cur as usize)).rep.as_mut_ptr(),
                        ZSTD_REP_NUM,
                    );
                }

                /* last match must start at a minimum distance of 8 from oend */
                if inr > ilimit {
                    cur += 1;
                    continue 'curloop;
                }

                if cur == last_pos {
                    break 'curloop;
                }

                if (optLevel == 0)
                    && ((*opt.add((cur + 1) as usize)).price
                        <= (*opt.add(cur as usize)).price + (BITCOST_MULTIPLIER as i32 / 2))
                {
                    /* skip unpromising positions */
                    cur += 1;
                    continue 'curloop;
                }

                debug_assert!((*opt.add(cur as usize)).price >= 0);
                {
                    let ll0 = ((*opt.add(cur as usize)).litlen == 0) as U32;
                    let previousPrice = (*opt.add(cur as usize)).price;
                    let basePrice = previousPrice + LL_PRICE!(0);
                    let mut nbMatches = getAllMatches(
                        matches,
                        ms,
                        &mut nextToUpdate3,
                        inr,
                        iend,
                        (*opt.add(cur as usize)).rep.as_ptr(),
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
                        continue 'curloop;
                    }

                    {
                        let longestML = (*matches.add((nbMatches - 1) as usize)).len;

                        if (longestML > sufficient_len)
                            || (cur + longestML >= ZSTD_OPT_NUM as U32)
                            || (ip.add((cur + longestML) as usize) >= iend)
                        {
                            lastStretch.mlen = longestML;
                            lastStretch.off = (*matches.add((nbMatches - 1) as usize)).off;
                            lastStretch.litlen = 0;
                            last_pos = cur + longestML;
                            break 'find; /* goto _shortestPath */
                        }
                    }

                    /* set prices using matches found at position == cur */
                    matchNb = 0;
                    while matchNb < nbMatches {
                        let offset = (*matches.add(matchNb as usize)).off;
                        let lastML = (*matches.add(matchNb as usize)).len;
                        let startML = if matchNb > 0 {
                            (*matches.add((matchNb - 1) as usize)).len + 1
                        } else {
                            minMatch
                        };
                        let mut mlen: U32;

                        mlen = lastML;
                        while mlen >= startML {
                            /* scan downward */
                            let pos = cur + mlen;
                            let price = basePrice
                                + ZSTD_getMatchPrice(offset, mlen, optStatePtr, optLevel) as i32;

                            if (pos > last_pos) || (price < (*opt.add(pos as usize)).price) {
                                while last_pos < pos {
                                    /* fill empty positions */
                                    last_pos += 1;
                                    (*opt.add(last_pos as usize)).price = ZSTD_MAX_PRICE;
                                    (*opt.add(last_pos as usize)).litlen = 1; /* != 0 */
                                }
                                (*opt.add(pos as usize)).mlen = mlen;
                                (*opt.add(pos as usize)).off = offset;
                                (*opt.add(pos as usize)).litlen = 0;
                                (*opt.add(pos as usize)).price = price;
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
                (*opt.add((last_pos + 1) as usize)).price = ZSTD_MAX_PRICE;

                cur += 1;
            } /* for (cur = 1; cur <= last_pos; cur++) */

            lastStretch = *opt.add(last_pos as usize);
            debug_assert!(cur >= lastStretch.mlen);
            cur = last_pos - lastStretch.mlen;
        } // 'find:  _shortestPath

        /* _shortestPath: cur, last_pos, lastStretch have to be set */
        debug_assert!((*opt.add(0)).mlen == 0);
        debug_assert!(last_pos >= lastStretch.mlen);
        debug_assert!(cur == last_pos - lastStretch.mlen);

        if lastStretch.mlen == 0 {
            /* no solution : all matches have been converted into literals */
            debug_assert!(lastStretch.litlen == (ip.offset_from(anchor) as U32) + last_pos);
            ip = ip.add(last_pos as usize);
            continue;
        }
        debug_assert!(lastStretch.off > 0);

        /* Update offset history */
        if lastStretch.litlen == 0 {
            /* finishing on a match : update offset history */
            let reps = ZSTD_newRep(
                (*opt.add(cur as usize)).rep.as_ptr(),
                lastStretch.off,
                ((*opt.add(cur as usize)).litlen == 0) as U32,
            );
            core::ptr::copy_nonoverlapping(reps.rep.as_ptr(), rep, ZSTD_REP_NUM);
        } else {
            core::ptr::copy_nonoverlapping(lastStretch.rep.as_ptr(), rep, ZSTD_REP_NUM);
            debug_assert!(cur >= lastStretch.litlen);
            cur -= lastStretch.litlen;
        }

        /* Let's write the shortest path solution. */
        {
            let storeEnd = cur + 2;
            let mut storeStart = storeEnd;
            let mut stretchPos = cur;

            debug_assert!((storeEnd as usize) < ZSTD_OPT_NUM + 3);
            if lastStretch.litlen > 0 {
                /* last "sequence" is unfinished: just a bunch of literals */
                (*opt.add(storeEnd as usize)).litlen = lastStretch.litlen;
                (*opt.add(storeEnd as usize)).mlen = 0;
                storeStart = storeEnd - 1;
                *opt.add(storeStart as usize) = lastStretch;
            }
            {
                *opt.add(storeEnd as usize) = lastStretch; /* note: litlen will be fixed */
                storeStart = storeEnd;
            }
            loop {
                let nextStretch = *opt.add(stretchPos as usize);
                (*opt.add(storeStart as usize)).litlen = nextStretch.litlen;
                if nextStretch.mlen == 0 {
                    /* reaching beginning of segment */
                    break;
                }
                storeStart -= 1;
                *opt.add(storeStart as usize) = nextStretch; /* note: litlen will be fixed */
                debug_assert!(nextStretch.litlen + nextStretch.mlen <= stretchPos);
                stretchPos -= nextStretch.litlen + nextStretch.mlen;
            }

            /* save sequences */
            {
                let mut storePos = storeStart;
                while storePos <= storeEnd {
                    let llen = (*opt.add(storePos as usize)).litlen;
                    let mlen = (*opt.add(storePos as usize)).mlen;
                    let offBase = (*opt.add(storePos as usize)).off;
                    let advance = llen + mlen;

                    if mlen == 0 {
                        /* only literals => must be last "sequence" */
                        debug_assert!(storePos == storeEnd);
                        ip = anchor.add(llen as usize);
                        storePos += 1;
                        continue;
                    }

                    debug_assert!(anchor.add(llen as usize) <= iend);
                    ZSTD_updateStats(optStatePtr, llen, anchor, offBase, mlen);
                    ZSTD_storeSeq(
                        seqStore,
                        llen as usize,
                        anchor,
                        iend,
                        offBase,
                        mlen as usize,
                    );
                    anchor = anchor.add(advance as usize);
                    ip = anchor;
                    storePos += 1;
                }
            }

            /* update all costs */
            ZSTD_setBasePrices(optStatePtr, optLevel);
        }
    } /* while (ip < ilimit) */

    /* Return the last literals size */
    iend.offset_from(anchor) as usize
}

unsafe fn ZSTD_compressBlock_opt0(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    ZSTD_compressBlock_opt_generic(ms, seqStore, rep, src, srcSize, 0, dictMode)
}

unsafe fn ZSTD_compressBlock_opt2(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
    dictMode: ZSTD_dictMode_e,
) -> usize {
    ZSTD_compressBlock_opt_generic(ms, seqStore, rep, src, srcSize, 2, dictMode)
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

/* ZSTD_initStats_ultra(): make a first compression pass to seed stats. */
unsafe fn ZSTD_initStats_ultra(
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    src: *const c_void,
    srcSize: usize,
) {
    let mut tmpRep: [U32; ZSTD_REP_NUM] = [0; ZSTD_REP_NUM];
    core::ptr::copy_nonoverlapping(rep, tmpRep.as_mut_ptr(), ZSTD_REP_NUM);

    debug_assert!((*ms).opt.litLengthSum == 0); /* first block */
    debug_assert!((*seqStore).sequences == (*seqStore).sequencesStart); /* no ldm */
    debug_assert!((*ms).window.dictLimit == (*ms).window.lowLimit); /* no dictionary */
    debug_assert!((*ms).window.dictLimit.wrapping_sub((*ms).nextToUpdate) <= 1); /* no prefix */

    ZSTD_compressBlock_opt2(ms, seqStore, tmpRep.as_mut_ptr(), src, srcSize, ZSTD_noDict);

    /* invalidate first scan from history, only keep entropy stats */
    ZSTD_resetSeqStore(seqStore);
    (*ms).window.base = (*ms).window.base.offset(-(srcSize as isize));
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
    let curr = (src as *const u8).offset_from((*ms).window.base) as U32;

    /* 2-passes strategy */
    debug_assert!(srcSize <= ZSTD_BLOCKSIZE_MAX);
    if ((*ms).opt.litLengthSum == 0) /* first block */
        && ((*seqStore).sequences == (*seqStore).sequencesStart) /* no ldm */
        && ((*ms).window.dictLimit == (*ms).window.lowLimit) /* no dictionary */
        && (curr == (*ms).window.dictLimit) /* start of frame */
        && (srcSize > ZSTD_PREDEF_THRESHOLD)
    /* input large enough */
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
