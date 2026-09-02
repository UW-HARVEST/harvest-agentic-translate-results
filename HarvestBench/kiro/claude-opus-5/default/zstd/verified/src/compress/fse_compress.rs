//! Translation of `compress/fse_compress.c` (FSE : Finite State Entropy encoder).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use crate::common::bits::ZSTD_highbit32;
use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::mem::*;

use core::ffi::{c_uint, c_void};

/* FSE template type/extension : FSE_FUNCTION_TYPE == BYTE */

/* FSE_buildCTable_wksp() :
 * Same as FSE_buildCTable(), but using an externally allocated scratch buffer (`workSpace`).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_wksp(
    ct: *mut FSE_CTable,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    let tableSize: U32 = 1u32 << tableLog;
    let tableMask: U32 = tableSize - 1;
    let ptr = ct as *mut c_void;
    let tableU16 = (ptr as *mut U16).add(2);
    let FSCT = (ptr as *mut U32).add(1 /* header */ + if tableLog != 0 { (tableSize >> 1) as usize } else { 1 });
    let symbolTT = FSCT as *mut FSE_symbolCompressionTransform;
    let step: U32 = FSE_TABLESTEP(tableSize);
    let maxSV1: U32 = maxSymbolValue + 1;

    let cumul = workSpace as *mut U16; /* size = maxSV1 */
    let tableSymbol = cumul.add((maxSV1 + 1) as usize) as *mut BYTE; /* size = tableSize */

    let mut highThreshold: U32 = tableSize - 1;

    /* assert(((size_t)workSpace & 1) == 0) dropped */
    if FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog) > wkspSize {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    /* CTable header */
    *tableU16.offset(-2) = tableLog as U16;
    *tableU16.offset(-1) = maxSymbolValue as U16;
    /* assert(tableLog < 16) dropped */

    /* symbol start positions */
    {
        let mut u: U32;
        *cumul.add(0) = 0;
        u = 1;
        while u <= maxSV1 {
            if *normalizedCounter.add((u - 1) as usize) == -1 {
                /* Low proba symbol */
                *cumul.add(u as usize) = *cumul.add((u - 1) as usize) + 1;
                *tableSymbol.add(highThreshold as usize) = (u - 1) as BYTE;
                highThreshold = highThreshold.wrapping_sub(1);
            } else {
                *cumul.add(u as usize) = (*cumul.add((u - 1) as usize))
                    .wrapping_add(*normalizedCounter.add((u - 1) as usize) as U16);
            }
            u += 1;
        }
        *cumul.add(maxSV1 as usize) = (tableSize + 1) as U16;
    }

    /* Spread symbols */
    if highThreshold == tableSize - 1 {
        /* Case for no low prob count symbols. */
        let spread = tableSymbol.add(tableSize as usize); /* size = tableSize + 8 (may write beyond tableSize) */
        {
            let add: U64 = 0x0101010101010101u64;
            let mut pos: size_t = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let mut i: c_int;
                let n: c_int = *normalizedCounter.add(s as usize) as c_int;
                MEM_write64(spread.add(pos), sv);
                i = 8;
                while i < n {
                    MEM_write64(spread.add(pos + i as usize), sv);
                    i += 8;
                }
                pos = pos.wrapping_add(n as size_t);
                s += 1;
                sv = sv.wrapping_add(add);
            }
        }
        /* Spread symbols across the table. */
        {
            let mut position: size_t = 0;
            let mut s: size_t = 0;
            let unroll: size_t = 2; /* Experimentally determined optimal unroll */
            while s < tableSize as size_t {
                let mut u: size_t = 0;
                while u < unroll {
                    let uPosition: size_t =
                        (position + (u * step as size_t)) & tableMask as size_t;
                    *tableSymbol.add(uPosition) = *spread.add(s + u);
                    u += 1;
                }
                position = (position + (unroll * step as size_t)) & tableMask as size_t;
                s += unroll;
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
                position = (position + step) & tableMask;
                while position > highThreshold {
                    position = (position + step) & tableMask; /* Low proba area */
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
            *tableU16.add(*cumul.add(s as usize) as usize) = (tableSize + u) as U16;
            *cumul.add(s as usize) = (*cumul.add(s as usize)).wrapping_add(1);
            u += 1;
        }
    }

    /* Build Symbol Transformation Table */
    {
        let mut total: c_uint = 0;
        let mut s: c_uint = 0;
        while s <= maxSymbolValue {
            let nc = *normalizedCounter.add(s as usize);
            match nc {
                0 => {
                    /* filling nonetheless, for compatibility with FSE_getMaxNbBits() */
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        ((tableLog + 1) << 16).wrapping_sub(1u32 << tableLog);
                }
                -1 | 1 => {
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        (tableLog << 16).wrapping_sub(1u32 << tableLog);
                    (*symbolTT.add(s as usize)).deltaFindState = total.wrapping_sub(1) as c_int;
                    total += 1;
                }
                _ => {
                    let maxBitsOut: U32 =
                        tableLog - ZSTD_highbit32(nc as U32 - 1);
                    let minStatePlus: U32 = (nc as U32) << maxBitsOut;
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        (maxBitsOut << 16).wrapping_sub(minStatePlus);
                    (*symbolTT.add(s as usize)).deltaFindState =
                        total.wrapping_sub(nc as c_uint) as c_int;
                    total += nc as c_uint;
                }
            }
            s += 1;
        }
    }

    0
}

use core::ffi::c_int;

/*-**************************************************************
*  FSE NCount encoding
****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_NCountWriteBound(
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> size_t {
    let maxHeaderSize: size_t = (((maxSymbolValue + 1) * tableLog
        + 4 /* bitCount initialized at 4 */
        + 2 /* first two symbols may use one additional bit each */) as size_t
        / 8)
        + 1 /* round up to whole nb bytes */
        + 2 /* additional two bytes for bitstream flush */;
    if maxSymbolValue != 0 {
        maxHeaderSize
    } else {
        FSE_NCOUNTBOUND
    } /* maxSymbolValue==0 ? use default */
}

pub unsafe fn FSE_writeNCount_generic(
    header: *mut c_void,
    headerBufferSize: size_t,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
    writeIsSafe: c_uint,
) -> size_t {
    let ostart = header as *mut BYTE;
    let mut out = ostart;
    let oend = ostart.add(headerBufferSize);
    let mut nbBits: c_int;
    let tableSize: c_int = 1 << tableLog;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bitStream: U32 = 0;
    let mut bitCount: c_int = 0;
    let mut symbol: c_uint = 0;
    let alphabetSize: c_uint = maxSymbolValue + 1;
    let mut previousIs0: c_int = 0;

    /* Table Size */
    bitStream = bitStream.wrapping_add((tableLog - FSE_MIN_TABLELOG) << bitCount);
    bitCount += 4;

    /* Init */
    remaining = tableSize + 1; /* +1 for extra accuracy */
    threshold = tableSize;
    nbBits = tableLog as c_int + 1;

    while (symbol < alphabetSize) && (remaining > 1) {
        /* stops at 1 */
        if previousIs0 != 0 {
            let mut start: c_uint = symbol;
            while (symbol < alphabetSize) && (*normalizedCounter.add(symbol as usize) == 0) {
                symbol += 1;
            }
            if symbol == alphabetSize {
                break;
            } /* incorrect distribution */
            while symbol >= start + 24 {
                start += 24;
                bitStream = bitStream.wrapping_add(0xFFFFu32 << bitCount);
                if (writeIsSafe == 0) && (out > oend.offset(-2)) {
                    return ERROR(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
                }
                *out.add(0) = bitStream as BYTE;
                *out.add(1) = (bitStream >> 8) as BYTE;
                out = out.add(2);
                bitStream >>= 16;
            }
            while symbol >= start + 3 {
                start += 3;
                bitStream = bitStream.wrapping_add(3u32 << bitCount);
                bitCount += 2;
            }
            bitStream = bitStream.wrapping_add((symbol - start) << bitCount);
            bitCount += 2;
            if bitCount > 16 {
                if (writeIsSafe == 0) && (out > oend.offset(-2)) {
                    return ERROR(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
                }
                *out.add(0) = bitStream as BYTE;
                *out.add(1) = (bitStream >> 8) as BYTE;
                out = out.add(2);
                bitStream >>= 16;
                bitCount -= 16;
            }
        }
        {
            let count: c_int = *normalizedCounter.add(symbol as usize) as c_int;
            symbol += 1;
            let max: c_int = (2 * threshold - 1) - remaining;
            remaining -= if count < 0 { -count } else { count };
            let count = count + 1; /* +1 for extra accuracy */
            let count = if count >= threshold {
                count + max /* [0..max[ [max..threshold[ (...) [threshold+max 2*threshold[ */
            } else {
                count
            };
            bitStream = bitStream.wrapping_add((count as U32) << bitCount);
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
            if (writeIsSafe == 0) && (out > oend.offset(-2)) {
                return ERROR(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
            }
            *out.add(0) = bitStream as BYTE;
            *out.add(1) = (bitStream >> 8) as BYTE;
            out = out.add(2);
            bitStream >>= 16;
            bitCount -= 16;
        }
    }

    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC); /* incorrect normalized distribution */
    }
    /* assert(symbol <= alphabetSize) dropped */

    /* flush remaining bitStream */
    if (writeIsSafe == 0) && (out > oend.offset(-2)) {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
    }
    *out.add(0) = bitStream as BYTE;
    *out.add(1) = (bitStream >> 8) as BYTE;
    out = out.add(((bitCount + 7) / 8) as usize);

    /* assert(out >= ostart) dropped */
    (out as size_t).wrapping_sub(ostart as size_t)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_writeNCount(
    buffer: *mut c_void,
    bufferSize: size_t,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> size_t {
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* Unsupported */
    }
    if tableLog < FSE_MIN_TABLELOG {
        return ERROR(ZSTD_error_GENERIC); /* Unsupported */
    }

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
pub unsafe fn FSE_minTableLog(srcSize: size_t, maxSymbolValue: c_uint) -> c_uint {
    let minBitsSrc: U32 = ZSTD_highbit32(srcSize as U32) + 1;
    let minBitsSymbols: U32 = ZSTD_highbit32(maxSymbolValue) + 2;
    let minBits: U32 = if minBitsSrc < minBitsSymbols {
        minBitsSrc
    } else {
        minBitsSymbols
    };
    minBits
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_optimalTableLog_internal(
    maxTableLog: c_uint,
    srcSize: size_t,
    maxSymbolValue: c_uint,
    minus: c_uint,
) -> c_uint {
    let maxBitsSrc: U32 = ZSTD_highbit32((srcSize as U32).wrapping_sub(1)).wrapping_sub(minus);
    let mut tableLog: U32 = maxTableLog;
    let minBits: U32 = FSE_minTableLog(srcSize, maxSymbolValue);
    if tableLog == 0 {
        tableLog = FSE_DEFAULT_TABLELOG;
    }
    if maxBitsSrc < tableLog {
        tableLog = maxBitsSrc; /* Accuracy can be reduced */
    }
    if minBits > tableLog {
        tableLog = minBits; /* Need a minimum to safely represent all symbol values */
    }
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
    maxTableLog: c_uint,
    srcSize: size_t,
    maxSymbolValue: c_uint,
) -> c_uint {
    FSE_optimalTableLog_internal(maxTableLog, srcSize, maxSymbolValue, 2)
}

/* Secondary normalization method.
To be used when primary method fails. */

pub unsafe fn FSE_normalizeM2(
    norm: *mut i16,
    tableLog: U32,
    count: *const c_uint,
    mut total: size_t,
    maxSymbolValue: U32,
    lowProbCount: i16,
) -> size_t {
    let NOT_YET_ASSIGNED: i16 = -2;
    let mut s: U32;
    let mut distributed: U32 = 0;
    let mut ToDistribute: U32;

    /* Init */
    let lowThreshold: U32 = (total >> tableLog) as U32;
    let mut lowOne: U32 = ((total * 3) >> (tableLog + 1)) as U32;

    s = 0;
    while s <= maxSymbolValue {
        if *count.add(s as usize) == 0 {
            *norm.add(s as usize) = 0;
            s += 1;
            continue;
        }
        if *count.add(s as usize) <= lowThreshold {
            *norm.add(s as usize) = lowProbCount;
            distributed += 1;
            total -= *count.add(s as usize) as size_t;
            s += 1;
            continue;
        }
        if *count.add(s as usize) <= lowOne {
            *norm.add(s as usize) = 1;
            distributed += 1;
            total -= *count.add(s as usize) as size_t;
            s += 1;
            continue;
        }

        *norm.add(s as usize) = NOT_YET_ASSIGNED;
        s += 1;
    }
    ToDistribute = (1u32 << tableLog) - distributed;

    if ToDistribute == 0 {
        return 0;
    }

    if (total as U32 / ToDistribute) > lowOne {
        /* risk of rounding to zero */
        lowOne = ((total * 3) / (ToDistribute as size_t * 2)) as U32;
        s = 0;
        while s <= maxSymbolValue {
            if (*norm.add(s as usize) == NOT_YET_ASSIGNED)
                && (*count.add(s as usize) <= lowOne)
            {
                *norm.add(s as usize) = 1;
                distributed += 1;
                total -= *count.add(s as usize) as size_t;
                s += 1;
                continue;
            }
            s += 1;
        }
        ToDistribute = (1u32 << tableLog) - distributed;
    }

    if distributed == maxSymbolValue + 1 {
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
        *norm.add(maxV as usize) += ToDistribute as i16;
        return 0;
    }

    if total == 0 {
        /* all of the symbols were low enough for the lowOne or lowThreshold */
        s = 0;
        while ToDistribute > 0 {
            if *norm.add(s as usize) > 0 {
                ToDistribute -= 1;
                *norm.add(s as usize) += 1;
            }
            s = (s + 1) % (maxSymbolValue + 1);
        }
        return 0;
    }

    {
        let vStepLog: U64 = 62 - tableLog as U64;
        let mid: U64 = (1u64 << (vStepLog - 1)) - 1;
        let rStep: U64 =
            (((1u64 << vStepLog) * ToDistribute as U64) + mid) / (total as U32) as U64; /* scale on remaining */
        let mut tmpTotal: U64 = mid;
        s = 0;
        while s <= maxSymbolValue {
            if *norm.add(s as usize) == NOT_YET_ASSIGNED {
                let end: U64 = tmpTotal + (*count.add(s as usize) as U64 * rStep);
                let sStart: U32 = (tmpTotal >> vStepLog) as U32;
                let sEnd: U32 = (end >> vStepLog) as U32;
                let weight: U32 = sEnd - sStart;
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
    mut tableLog: c_uint,
    count: *const c_uint,
    total: size_t,
    maxSymbolValue: c_uint,
    useLowProbCount: c_uint,
) -> size_t {
    /* Sanity checks */
    if tableLog == 0 {
        tableLog = FSE_DEFAULT_TABLELOG;
    }
    if tableLog < FSE_MIN_TABLELOG {
        return ERROR(ZSTD_error_GENERIC); /* Unsupported size */
    }
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* Unsupported size */
    }
    if tableLog < FSE_minTableLog(total, maxSymbolValue) {
        return ERROR(ZSTD_error_GENERIC); /* Too small tableLog, compression potentially impossible */
    }

    {
        const rtbTable: [U32; 8] =
            [0, 473195, 504333, 520860, 550000, 700000, 750000, 830000];
        let lowProbCount: i16 = if useLowProbCount != 0 { -1 } else { 1 };
        let scale: U64 = 62 - tableLog as U64;
        let step: U64 = (1u64 << 62) / (total as U32) as U64; /* <== here, one division ! */
        let vStep: U64 = 1u64 << (scale - 20);
        let mut stillToDistribute: c_int = 1i32 << tableLog;
        let mut s: c_uint;
        let mut largest: c_uint = 0;
        let mut largestP: i16 = 0;
        let lowThreshold: U32 = (total >> tableLog) as U32;

        s = 0;
        while s <= maxSymbolValue {
            if *count.add(s as usize) as size_t == total {
                return 0; /* rle special case */
            }
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
                    ((*count.add(s as usize) as U64 * step) >> scale) as i16;
                if proba < 8 {
                    let restToBeat: U64 = vStep * rtbTable[proba as usize] as U64;
                    proba += ((*count.add(s as usize) as U64 * step)
                        .wrapping_sub((proba as U64) << scale)
                        > restToBeat) as i16;
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
        if -stillToDistribute >= (*normalizedCounter.add(largest as usize) as c_int >> 1) {
            /* corner case, need another normalization method */
            let errorCode: size_t = FSE_normalizeM2(
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
            *normalizedCounter.add(largest as usize) += stillToDistribute as i16;
        }
    }

    tableLog as size_t
}

/* fake FSE_CTable, for rle input (always same symbol) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_rle(ct: *mut FSE_CTable, symbolValue: BYTE) -> size_t {
    let ptr = ct as *mut c_void;
    let tableU16 = (ptr as *mut U16).add(2);
    let FSCTptr = (ptr as *mut U32).add(2);
    let symbolTT = FSCTptr as *mut FSE_symbolCompressionTransform;

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

pub unsafe fn FSE_compress_usingCTable_generic(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    mut srcSize: size_t,
    ct: *const FSE_CTable,
    fast: c_uint,
) -> size_t {
    let istart = src as *const BYTE;
    let iend = istart.add(srcSize);
    let mut ip = iend;

    let mut bitC: BIT_CStream_t = core::mem::zeroed();
    let mut CState1: FSE_CState_t = FSE_CState_t::default();
    let mut CState2: FSE_CState_t = FSE_CState_t::default();

    /* init */
    if srcSize <= 2 {
        return 0;
    }
    {
        let initError: size_t = BIT_initCStream(&mut bitC, dst as *mut u8, dstSize);
        if ERR_isError(initError) != 0 {
            return 0; /* not enough space available to write a bitstream */
        }
    }

    /* FSE_FLUSHBITS(s) : (fast ? BIT_flushBitsFast(s) : BIT_flushBits(s)) */

    if srcSize & 1 != 0 {
        ip = ip.offset(-1);
        FSE_initCState2(&mut CState1, ct, *ip as U32);
        ip = ip.offset(-1);
        FSE_initCState2(&mut CState2, ct, *ip as U32);
        ip = ip.offset(-1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as c_uint);
        if fast != 0 {
            BIT_flushBitsFast(&mut bitC);
        } else {
            BIT_flushBits(&mut bitC);
        }
    } else {
        ip = ip.offset(-1);
        FSE_initCState2(&mut CState2, ct, *ip as U32);
        ip = ip.offset(-1);
        FSE_initCState2(&mut CState1, ct, *ip as U32);
    }

    /* join to mod 4 */
    srcSize -= 2;
    if (core::mem::size_of_val(&bitC.bitContainer) * 8 > (FSE_MAX_TABLELOG * 4 + 7) as usize)
        && (srcSize & 2 != 0)
    {
        /* test bit 2 */
        ip = ip.offset(-1);
        FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as c_uint);
        ip = ip.offset(-1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as c_uint);
        if fast != 0 {
            BIT_flushBitsFast(&mut bitC);
        } else {
            BIT_flushBits(&mut bitC);
        }
    }

    /* 2 or 4 encoding per loop */
    while ip > istart {
        ip = ip.offset(-1);
        FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as c_uint);

        if core::mem::size_of_val(&bitC.bitContainer) * 8 < (FSE_MAX_TABLELOG * 2 + 7) as usize {
            /* this test must be static */
            if fast != 0 {
                BIT_flushBitsFast(&mut bitC);
            } else {
                BIT_flushBits(&mut bitC);
            }
        }

        ip = ip.offset(-1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as c_uint);

        if core::mem::size_of_val(&bitC.bitContainer) * 8 > (FSE_MAX_TABLELOG * 4 + 7) as usize {
            /* this test must be static */
            ip = ip.offset(-1);
            FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as c_uint);
            ip = ip.offset(-1);
            FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as c_uint);
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
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    ct: *const FSE_CTable,
) -> size_t {
    let fast: c_uint = (dstSize >= FSE_BLOCKBOUND(srcSize)) as c_uint;

    if fast != 0 {
        FSE_compress_usingCTable_generic(dst, dstSize, src, srcSize, ct, 1)
    } else {
        FSE_compress_usingCTable_generic(dst, dstSize, src, srcSize, ct, 0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_compressBound(size: size_t) -> size_t {
    FSE_COMPRESSBOUND(size)
}
