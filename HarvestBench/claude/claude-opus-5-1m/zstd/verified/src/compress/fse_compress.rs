//! Translation of `compress/fse_compress.c`
#![allow(dead_code)]

use crate::common::bits::ZSTD_highbit32;
use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::mem::*;
use core::ffi::{c_int, c_void};

/* **************************************************************
*  Error Management
****************************************************************/
/* #define FSE_isError ERR_isError */

/* FSE_FUNCTION_TYPE == BYTE (from fse.h) */

/* Function templates */

/* FSE_buildCTable_wksp() :
 * Same as FSE_buildCTable(), but using an externally allocated scratch buffer (`workSpace`).
 * wkspSize should be sized to handle worst case situation, which is `1<<max_tableLog * sizeof(FSE_FUNCTION_TYPE)`
 * workSpace must also be properly aligned with FSE_FUNCTION_TYPE requirements
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_wksp(
    ct: *mut FSE_CTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let tableSize: U32 = 1u32.wrapping_shl(tableLog);
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let ptr = ct as *mut c_void;
    let tableU16: *mut U16 = (ptr as *mut U16).add(2);
    let FSCT = (ptr as *mut U32).add(
        1 /* header */ + if tableLog != 0 { (tableSize >> 1) as usize } else { 1 },
    ) as *mut c_void;
    let symbolTT: *mut FSE_symbolCompressionTransform =
        FSCT as *mut FSE_symbolCompressionTransform;
    let step: U32 = FSE_TABLESTEP(tableSize);
    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);

    let cumul: *mut U16 = workSpace as *mut U16; /* size = maxSV1 */
    let tableSymbol: *mut BYTE = cumul.add(maxSV1 as usize + 1) as *mut BYTE; /* size = tableSize */

    let mut highThreshold: U32 = tableSize.wrapping_sub(1);

    if FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog) > wkspSize {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    /* CTable header */
    *tableU16.offset(-2) = tableLog as U16;
    *tableU16.offset(-1) = maxSymbolValue as U16;

    /* For explanations on how to distribute symbol values over the table :
     * https://fastcompression.blogspot.fr/2014/02/fse-distributing-symbol-values.html */

    /* symbol start positions */
    {
        let mut u: U32;
        *cumul.add(0) = 0;
        u = 1;
        while u <= maxSV1 {
            if *normalizedCounter.add(u as usize - 1) == -1 {
                /* Low proba symbol */
                *cumul.add(u as usize) = (*cumul.add(u as usize - 1)).wrapping_add(1);
                *tableSymbol.add(highThreshold as usize) = (u.wrapping_sub(1)) as BYTE;
                highThreshold = highThreshold.wrapping_sub(1);
            } else {
                *cumul.add(u as usize) = (*cumul.add(u as usize - 1))
                    .wrapping_add(*normalizedCounter.add(u as usize - 1) as U16);
            }
            u += 1;
        }
        *cumul.add(maxSV1 as usize) = tableSize.wrapping_add(1) as U16;
    }

    /* Spread symbols */
    if highThreshold == tableSize.wrapping_sub(1) {
        /* Case for no low prob count symbols. Lay down 8 bytes at a time
         * to reduce branch misses since we are operating on a small block
         */
        let spread: *mut BYTE = tableSymbol.add(tableSize as usize); /* size = tableSize + 8 (may write beyond tableSize) */
        {
            let add: U64 = 0x0101010101010101u64;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let mut i: c_int;
                let n: c_int = *normalizedCounter.add(s as usize) as c_int;
                MEM_write64(spread.add(pos) as *mut c_void, sv);
                i = 8;
                while i < n {
                    MEM_write64(
                        spread.add(pos).offset(i as isize) as *mut c_void,
                        sv,
                    );
                    i += 8;
                }
                pos = pos.wrapping_add(n as usize);
                s += 1;
                sv = sv.wrapping_add(add);
            }
        }
        /* Spread symbols across the table. Lack of lowprob symbols means that
         * we don't need variable sized inner loop, so we can unroll the loop and
         * reduce branch misses.
         */
        {
            let mut position: usize = 0;
            let mut s: usize;
            let unroll: usize = 2; /* Experimentally determined optimal unroll */
            s = 0;
            while s < tableSize as usize {
                let mut u: usize = 0;
                while u < unroll {
                    let uPosition: usize =
                        (position.wrapping_add(u.wrapping_mul(step as usize))) & tableMask as usize;
                    *tableSymbol.add(uPosition) = *spread.add(s.wrapping_add(u));
                    u += 1;
                }
                position = (position.wrapping_add(unroll.wrapping_mul(step as usize)))
                    & tableMask as usize;
                s = s.wrapping_add(unroll);
            }
        }
    } else {
        let mut position: U32 = 0;
        let mut symbol: U32 = 0;
        while symbol < maxSV1 {
            let mut nbOccurrences: c_int;
            let freq: c_int = *normalizedCounter.add(symbol as usize) as c_int;
            nbOccurrences = 0;
            while nbOccurrences < freq {
                *tableSymbol.add(position as usize) = symbol as BYTE;
                position = (position.wrapping_add(step)) & tableMask;
                while position > highThreshold {
                    position = (position.wrapping_add(step)) & tableMask;
                    /* Low proba area */
                }
                nbOccurrences += 1;
            }
            symbol += 1;
        }
    }

    /* Build table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let s: BYTE = *tableSymbol.add(u as usize);
            *tableU16.add(*cumul.add(s as usize) as usize) = tableSize.wrapping_add(u) as U16; /* TableU16 : sorted by symbol order; gives next state value */
            *cumul.add(s as usize) = (*cumul.add(s as usize)).wrapping_add(1);
            u += 1;
        }
    }

    /* Build Symbol Transformation Table */
    {
        let mut total: u32 = 0;
        let mut s: u32 = 0;
        while s <= maxSymbolValue {
            match *normalizedCounter.add(s as usize) as c_int {
                0 => {
                    /* filling nonetheless, for compatibility with FSE_getMaxNbBits() */
                    (*symbolTT.add(s as usize)).deltaNbBits = (tableLog.wrapping_add(1) << 16)
                        .wrapping_sub(1u32.wrapping_shl(tableLog));
                }
                -1 | 1 => {
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        (tableLog << 16).wrapping_sub(1u32.wrapping_shl(tableLog));
                    (*symbolTT.add(s as usize)).deltaFindState = total.wrapping_sub(1) as c_int;
                    total = total.wrapping_add(1);
                }
                _ => {
                    let maxBitsOut: U32 = tableLog.wrapping_sub(ZSTD_highbit32(
                        (*normalizedCounter.add(s as usize) as i32 as U32).wrapping_sub(1),
                    ));
                    let minStatePlus: U32 = (*normalizedCounter.add(s as usize) as i32 as U32)
                        .wrapping_shl(maxBitsOut);
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        (maxBitsOut << 16).wrapping_sub(minStatePlus);
                    (*symbolTT.add(s as usize)).deltaFindState = total
                        .wrapping_sub(*normalizedCounter.add(s as usize) as i32 as U32)
                        as c_int;
                    total = total.wrapping_add(*normalizedCounter.add(s as usize) as i32 as U32);
                }
            }
            s += 1;
        }
    }

    0
}

/*-**************************************************************
*  FSE NCount encoding
****************************************************************/
#[unsafe(no_mangle)]
pub extern "C" fn FSE_NCountWriteBound(maxSymbolValue: u32, tableLog: u32) -> usize {
    let maxHeaderSize: usize = (((maxSymbolValue as usize + 1) * tableLog as usize
        + 4 /* bitCount initialized at 4 */
        + 2/* first two symbols may use one additional bit each */)
        / 8)
        + 1 /* round up to whole nb bytes */
        + 2 /* additional two bytes for bitstream flush */;
    if maxSymbolValue != 0 {
        maxHeaderSize
    } else {
        FSE_NCOUNTBOUND
    } /* maxSymbolValue==0 ? use default */
}

unsafe fn FSE_writeNCount_generic(
    header: *mut c_void,
    headerBufferSize: usize,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
    writeIsSafe: u32,
) -> usize {
    let ostart: *mut BYTE = header as *mut BYTE;
    let mut out: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(headerBufferSize);
    let mut nbBits: c_int;
    let tableSize: c_int = 1i32.wrapping_shl(tableLog);
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bitStream: U32 = 0;
    let mut bitCount: c_int = 0;
    let mut symbol: u32 = 0;
    let alphabetSize: u32 = maxSymbolValue.wrapping_add(1);
    let mut previousIs0: c_int = 0;

    /* Table Size */
    bitStream = bitStream
        .wrapping_add((tableLog.wrapping_sub(FSE_MIN_TABLELOG)).wrapping_shl(bitCount as u32));
    bitCount += 4;

    /* Init */
    remaining = tableSize + 1; /* +1 for extra accuracy */
    threshold = tableSize;
    nbBits = tableLog as c_int + 1;

    while (symbol < alphabetSize) && (remaining > 1) {
        /* stops at 1 */
        if previousIs0 != 0 {
            let mut start: u32 = symbol;
            while (symbol < alphabetSize) && (*normalizedCounter.add(symbol as usize) == 0) {
                symbol += 1;
            }
            if symbol == alphabetSize {
                break;
            } /* incorrect distribution */
            while symbol >= start.wrapping_add(24) {
                start = start.wrapping_add(24);
                bitStream = bitStream.wrapping_add(0xFFFFu32.wrapping_shl(bitCount as u32));
                if (writeIsSafe == 0) && (out > oend.wrapping_sub(2)) {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                } /* Buffer overflow */
                *out.add(0) = bitStream as BYTE;
                *out.add(1) = (bitStream >> 8) as BYTE;
                out = out.add(2);
                bitStream >>= 16;
            }
            while symbol >= start.wrapping_add(3) {
                start = start.wrapping_add(3);
                bitStream = bitStream.wrapping_add(3u32.wrapping_shl(bitCount as u32));
                bitCount += 2;
            }
            bitStream = bitStream
                .wrapping_add((symbol.wrapping_sub(start)).wrapping_shl(bitCount as u32));
            bitCount += 2;
            if bitCount > 16 {
                if (writeIsSafe == 0) && (out > oend.wrapping_sub(2)) {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                } /* Buffer overflow */
                *out.add(0) = bitStream as BYTE;
                *out.add(1) = (bitStream >> 8) as BYTE;
                out = out.add(2);
                bitStream >>= 16;
                bitCount -= 16;
            }
        }
        {
            let mut count: c_int = *normalizedCounter.add(symbol as usize) as c_int;
            symbol += 1;
            let max: c_int = (2 * threshold - 1) - remaining;
            remaining -= if count < 0 { -count } else { count };
            count += 1; /* +1 for extra accuracy */
            if count >= threshold {
                count += max;
            } /* [0..max[ [max..threshold[ (...) [threshold+max 2*threshold[ */
            bitStream = bitStream.wrapping_add((count as U32).wrapping_shl(bitCount as u32));
            bitCount += nbBits;
            bitCount -= (count < max) as c_int;
            previousIs0 = (count == 1) as c_int;
            if remaining < 1 {
                return ERROR(ZSTD_error_GENERIC);
            }
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }
        }
        if bitCount > 16 {
            if (writeIsSafe == 0) && (out > oend.wrapping_sub(2)) {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            } /* Buffer overflow */
            *out.add(0) = bitStream as BYTE;
            *out.add(1) = (bitStream >> 8) as BYTE;
            out = out.add(2);
            bitStream >>= 16;
            bitCount -= 16;
        }
    }

    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC);
    } /* incorrect normalized distribution */

    /* flush remaining bitStream */
    if (writeIsSafe == 0) && (out > oend.wrapping_sub(2)) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    } /* Buffer overflow */
    *out.add(0) = bitStream as BYTE;
    *out.add(1) = (bitStream >> 8) as BYTE;
    out = out.offset(((bitCount + 7) / 8) as isize);

    out.offset_from(ostart) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_writeNCount(
    buffer: *mut c_void,
    bufferSize: usize,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    } /* Unsupported */
    if tableLog < FSE_MIN_TABLELOG {
        return ERROR(ZSTD_error_GENERIC);
    } /* Unsupported */

    if bufferSize < FSE_NCountWriteBound(maxSymbolValue, tableLog) {
        return FSE_writeNCount_generic(
            buffer,
            bufferSize,
            normalizedCounter,
            maxSymbolValue,
            tableLog,
            0,
        );
    }

    FSE_writeNCount_generic(
        buffer,
        bufferSize,
        normalizedCounter,
        maxSymbolValue,
        tableLog,
        1, /* write in buffer is safe */
    )
}

/*-**************************************************************
*  FSE Compression Code
****************************************************************/

/* provides the minimum logSize to safely represent a distribution */
unsafe fn FSE_minTableLog(srcSize: usize, maxSymbolValue: u32) -> u32 {
    let minBitsSrc: U32 = ZSTD_highbit32(srcSize as U32).wrapping_add(1);
    let minBitsSymbols: U32 = ZSTD_highbit32(maxSymbolValue).wrapping_add(2);
    let minBits: U32 = if minBitsSrc < minBitsSymbols {
        minBitsSrc
    } else {
        minBitsSymbols
    };
    minBits
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_optimalTableLog_internal(
    maxTableLog: u32,
    srcSize: usize,
    maxSymbolValue: u32,
    minus: u32,
) -> u32 {
    let maxBitsSrc: U32 = ZSTD_highbit32(srcSize.wrapping_sub(1) as U32).wrapping_sub(minus);
    let mut tableLog: U32 = maxTableLog;
    let minBits: U32 = FSE_minTableLog(srcSize, maxSymbolValue);
    if tableLog == 0 {
        tableLog = FSE_DEFAULT_TABLELOG;
    }
    if maxBitsSrc < tableLog {
        tableLog = maxBitsSrc;
    } /* Accuracy can be reduced */
    if minBits > tableLog {
        tableLog = minBits;
    } /* Need a minimum to safely represent all symbol values */
    if tableLog < FSE_MIN_TABLELOG {
        tableLog = FSE_MIN_TABLELOG;
    }
    if tableLog > FSE_MAX_TABLELOG {
        tableLog = FSE_MAX_TABLELOG;
    }
    tableLog
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_optimalTableLog(
    maxTableLog: u32,
    srcSize: usize,
    maxSymbolValue: u32,
) -> u32 {
    FSE_optimalTableLog_internal(maxTableLog, srcSize, maxSymbolValue, 2)
}

/* Secondary normalization method.
To be used when primary method fails. */

unsafe fn FSE_normalizeM2(
    norm: *mut i16,
    tableLog: U32,
    count: *const u32,
    mut total: usize,
    maxSymbolValue: U32,
    lowProbCount: i16,
) -> usize {
    let NOT_YET_ASSIGNED: i16 = -2;
    let mut s: U32;
    let mut distributed: U32 = 0;
    let mut ToDistribute: U32;

    /* Init */
    let lowThreshold: U32 = (total >> tableLog) as U32;
    let mut lowOne: U32 = ((total.wrapping_mul(3)) >> (tableLog + 1)) as U32;

    s = 0;
    while s <= maxSymbolValue {
        if *count.add(s as usize) == 0 {
            *norm.add(s as usize) = 0;
            s += 1;
            continue;
        }
        if *count.add(s as usize) <= lowThreshold {
            *norm.add(s as usize) = lowProbCount;
            distributed = distributed.wrapping_add(1);
            total = total.wrapping_sub(*count.add(s as usize) as usize);
            s += 1;
            continue;
        }
        if *count.add(s as usize) <= lowOne {
            *norm.add(s as usize) = 1;
            distributed = distributed.wrapping_add(1);
            total = total.wrapping_sub(*count.add(s as usize) as usize);
            s += 1;
            continue;
        }

        *norm.add(s as usize) = NOT_YET_ASSIGNED;
        s += 1;
    }
    ToDistribute = (1u32.wrapping_shl(tableLog)).wrapping_sub(distributed);

    if ToDistribute == 0 {
        return 0;
    }

    if (total / ToDistribute as usize) > lowOne as usize {
        /* risk of rounding to zero */
        lowOne = ((total.wrapping_mul(3)) / (ToDistribute.wrapping_mul(2)) as usize) as U32;
        s = 0;
        while s <= maxSymbolValue {
            if (*norm.add(s as usize) == NOT_YET_ASSIGNED)
                && (*count.add(s as usize) <= lowOne)
            {
                *norm.add(s as usize) = 1;
                distributed = distributed.wrapping_add(1);
                total = total.wrapping_sub(*count.add(s as usize) as usize);
                s += 1;
                continue;
            }
            s += 1;
        }
        ToDistribute = (1u32.wrapping_shl(tableLog)).wrapping_sub(distributed);
    }

    if distributed == maxSymbolValue.wrapping_add(1) {
        /* all values are pretty poor;
        probably incompressible data (should have already been detected);
        find max, then give all remaining points to max */
        let mut maxV: U32 = 0;
        let mut maxC: U32 = 0;
        s = 0;
        while s <= maxSymbolValue {
            if *count.add(s as usize) > maxC {
                maxV = s;
                maxC = *count.add(s as usize);
            }
            s += 1;
        }
        *norm.add(maxV as usize) =
            (*norm.add(maxV as usize)).wrapping_add(ToDistribute as i16);
        return 0;
    }

    if total == 0 {
        /* all of the symbols were low enough for the lowOne or lowThreshold */
        s = 0;
        while ToDistribute > 0 {
            if *norm.add(s as usize) > 0 {
                ToDistribute -= 1;
                *norm.add(s as usize) = (*norm.add(s as usize)).wrapping_add(1);
            }
            s = (s.wrapping_add(1)) % (maxSymbolValue.wrapping_add(1));
        }
        return 0;
    }

    {
        let vStepLog: U64 = (62u32.wrapping_sub(tableLog)) as U64;
        let mid: U64 = (1u64 << (vStepLog - 1)).wrapping_sub(1);
        let rStep: U64 = ((1u64 << vStepLog).wrapping_mul(ToDistribute as U64)).wrapping_add(mid)
            / (total as U32) as U64; /* scale on remaining */
        let mut tmpTotal: U64 = mid;
        s = 0;
        while s <= maxSymbolValue {
            if *norm.add(s as usize) == NOT_YET_ASSIGNED {
                let end: U64 =
                    tmpTotal.wrapping_add((*count.add(s as usize) as U64).wrapping_mul(rStep));
                let sStart: U32 = (tmpTotal >> vStepLog) as U32;
                let sEnd: U32 = (end >> vStepLog) as U32;
                let weight: U32 = sEnd.wrapping_sub(sStart);
                if weight < 1 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                *norm.add(s as usize) = weight as i16;
                tmpTotal = end;
            }
            s += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_normalizeCount(
    normalizedCounter: *mut i16,
    mut tableLog: u32,
    count: *const u32,
    total: usize,
    maxSymbolValue: u32,
    useLowProbCount: u32,
) -> usize {
    /* Sanity checks */
    if tableLog == 0 {
        tableLog = FSE_DEFAULT_TABLELOG;
    }
    if tableLog < FSE_MIN_TABLELOG {
        return ERROR(ZSTD_error_GENERIC);
    } /* Unsupported size */
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    } /* Unsupported size */
    if tableLog < FSE_minTableLog(total, maxSymbolValue) {
        return ERROR(ZSTD_error_GENERIC);
    } /* Too small tableLog, compression potentially impossible */

    {
        static rtbTable: [U32; 8] = [0, 473195, 504333, 520860, 550000, 700000, 750000, 830000];
        let lowProbCount: i16 = if useLowProbCount != 0 { -1 } else { 1 };
        let scale: U64 = (62u32.wrapping_sub(tableLog)) as U64;
        let step: U64 = (1u64 << 62) / (total as U32) as U64; /* <== here, one division ! */
        let vStep: U64 = 1u64 << (scale - 20);
        let mut stillToDistribute: c_int = 1i32.wrapping_shl(tableLog);
        let mut s: u32;
        let mut largest: u32 = 0;
        let mut largestP: i16 = 0;
        let lowThreshold: U32 = (total >> tableLog) as U32;

        s = 0;
        while s <= maxSymbolValue {
            if *count.add(s as usize) as usize == total {
                return 0;
            } /* rle special case */
            if *count.add(s as usize) == 0 {
                *normalizedCounter.add(s as usize) = 0;
                s += 1;
                continue;
            }
            if *count.add(s as usize) <= lowThreshold {
                *normalizedCounter.add(s as usize) = lowProbCount;
                stillToDistribute -= 1;
            } else {
                let mut proba: i16 =
                    (((*count.add(s as usize) as U64).wrapping_mul(step)) >> scale) as i16;
                if proba < 8 {
                    let restToBeat: U64 = vStep.wrapping_mul(rtbTable[proba as usize] as U64);
                    proba = proba.wrapping_add(
                        ((((*count.add(s as usize) as U64).wrapping_mul(step))
                            .wrapping_sub((proba as i64 as U64) << scale))
                            > restToBeat) as i16,
                    );
                }
                if proba > largestP {
                    largestP = proba;
                    largest = s;
                }
                *normalizedCounter.add(s as usize) = proba;
                stillToDistribute -= proba as c_int;
            }
            s += 1;
        }
        if (-stillToDistribute) >= ((*normalizedCounter.add(largest as usize) as c_int) >> 1) {
            /* corner case, need another normalization method */
            let errorCode: usize = FSE_normalizeM2(
                normalizedCounter,
                tableLog,
                count,
                total,
                maxSymbolValue,
                lowProbCount,
            );
            if ERR_isError(errorCode) != 0 {
                return errorCode;
            }
        } else {
            *normalizedCounter.add(largest as usize) = (*normalizedCounter
                .add(largest as usize))
            .wrapping_add(stillToDistribute as i16);
        }
    }

    tableLog as usize
}

/* fake FSE_CTable, for rle input (always same symbol) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_rle(ct: *mut FSE_CTable, symbolValue: BYTE) -> usize {
    let ptr = ct as *mut c_void;
    let tableU16: *mut U16 = (ptr as *mut U16).add(2);
    let FSCTptr = (ptr as *mut U32).add(2) as *mut c_void;
    let symbolTT: *mut FSE_symbolCompressionTransform =
        FSCTptr as *mut FSE_symbolCompressionTransform;

    /* header */
    *tableU16.offset(-2) = 0u16;
    *tableU16.offset(-1) = symbolValue as U16;

    /* Build table */
    *tableU16.add(0) = 0;
    *tableU16.add(1) = 0; /* just in case */

    /* Build Symbol Transformation Table */
    (*symbolTT.add(symbolValue as usize)).deltaNbBits = 0;
    (*symbolTT.add(symbolValue as usize)).deltaFindState = 0;

    0
}

unsafe fn FSE_compress_usingCTable_generic(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    mut srcSize: usize,
    ct: *const FSE_CTable,
    fast: u32,
) -> usize {
    const BITCONTAINER_BITS: usize = core::mem::size_of::<BitContainerType>() * 8;
    const FSE_MAX_TABLELOG_X4_P7: usize = (FSE_MAX_TABLELOG as usize) * 4 + 7;
    const FSE_MAX_TABLELOG_X2_P7: usize = (FSE_MAX_TABLELOG as usize) * 2 + 7;

    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let mut ip: *const BYTE = iend;

    let mut bitC = BIT_CStream_t {
        bitContainer: 0,
        bitPos: 0,
        startPtr: core::ptr::null_mut(),
        ptr: core::ptr::null_mut(),
        endPtr: core::ptr::null_mut(),
    };
    let mut CState1 = FSE_CState_t::default();
    let mut CState2 = FSE_CState_t::default();

    /* init */
    if srcSize <= 2 {
        return 0;
    }
    {
        let initError: usize = BIT_initCStream(&mut bitC, dst, dstSize);
        if ERR_isError(initError) != 0 {
            return 0;
        } /* not enough space available to write a bitstream */
    }

    /* FSE_FLUSHBITS(s) == (fast ? BIT_flushBitsFast(s) : BIT_flushBits(s)) */

    if srcSize & 1 != 0 {
        ip = ip.sub(1);
        FSE_initCState2(&mut CState1, ct, *ip as U32);
        ip = ip.sub(1);
        FSE_initCState2(&mut CState2, ct, *ip as U32);
        ip = ip.sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as U32);
        if fast != 0 {
            BIT_flushBitsFast(&mut bitC);
        } else {
            BIT_flushBits(&mut bitC);
        }
    } else {
        ip = ip.sub(1);
        FSE_initCState2(&mut CState2, ct, *ip as U32);
        ip = ip.sub(1);
        FSE_initCState2(&mut CState1, ct, *ip as U32);
    }

    /* join to mod 4 */
    srcSize -= 2;
    if (BITCONTAINER_BITS > FSE_MAX_TABLELOG_X4_P7) && (srcSize & 2 != 0) {
        /* test bit 2 */
        ip = ip.sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as U32);
        ip = ip.sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as U32);
        if fast != 0 {
            BIT_flushBitsFast(&mut bitC);
        } else {
            BIT_flushBits(&mut bitC);
        }
    }

    /* 2 or 4 encoding per loop */
    while ip > istart {
        ip = ip.sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as U32);

        if BITCONTAINER_BITS < FSE_MAX_TABLELOG_X2_P7 {
            /* this test must be static */
            if fast != 0 {
                BIT_flushBitsFast(&mut bitC);
            } else {
                BIT_flushBits(&mut bitC);
            }
        }

        ip = ip.sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as U32);

        if BITCONTAINER_BITS > FSE_MAX_TABLELOG_X4_P7 {
            /* this test must be static */
            ip = ip.sub(1);
            FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as U32);
            ip = ip.sub(1);
            FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as U32);
        }

        if fast != 0 {
            BIT_flushBitsFast(&mut bitC);
        } else {
            BIT_flushBits(&mut bitC);
        }
    }

    FSE_flushCState(&mut bitC, &CState2);
    FSE_flushCState(&mut bitC, &CState1);
    BIT_closeCStream(&mut bitC)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_compress_usingCTable(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    ct: *const FSE_CTable,
) -> usize {
    let fast: u32 = (dstSize >= FSE_BLOCKBOUND(srcSize)) as u32;

    if fast != 0 {
        FSE_compress_usingCTable_generic(dst, dstSize, src, srcSize, ct, 1)
    } else {
        FSE_compress_usingCTable_generic(dst, dstSize, src, srcSize, ct, 0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_compressBound(size: usize) -> usize {
    FSE_COMPRESSBOUND(size)
}
