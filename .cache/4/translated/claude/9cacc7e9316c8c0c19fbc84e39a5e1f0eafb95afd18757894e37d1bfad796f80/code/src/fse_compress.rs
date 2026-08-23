//! Transliteration of compress/fse_compress.c
//!
//! FSE : Finite State Entropy encoder
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

use crate::bits::*;
use crate::bitstream::*;
use crate::error_private::*;
use crate::fse::*;
use crate::mem::*;

/* **************************************************************
*  Error Management
****************************************************************/
/* #define FSE_isError ERR_isError */
#[inline(always)]
pub fn FSE_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/* zstd_deps.h : `#define ZSTD_div64(dividend, divisor) ((dividend) / (divisor))`
 * (`ZSTD_DEPS_NEED_MATH64`).  `divisor` is a U32 which is promoted to U64 by
 * the usual arithmetic conversions. */
#[inline(always)]
pub fn ZSTD_div64(dividend: U64, divisor: U32) -> U64 {
    dividend / (divisor as U64)
}

/* **************************************************************
*  Templates
****************************************************************/
/* fse.h : `#define FSE_FUNCTION_TYPE BYTE` and an empty `FSE_FUNCTION_EXTENSION`. */

/* FSE_buildCTable_wksp() :
 * Same as FSE_buildCTable(), but using an externally allocated scratch buffer (`workSpace`).
 * wkspSize should be sized to handle worst case situation, which is `1<<max_tableLog * sizeof(FSE_FUNCTION_TYPE)`
 * workSpace must also be properly aligned with FSE_FUNCTION_TYPE requirements
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_wksp(
    ct: *mut FSE_CTable,
    normalizedCounter: *const core::ffi::c_short,
    maxSymbolValue: core::ffi::c_uint,
    tableLog: core::ffi::c_uint,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
) -> usize {
    let tableSize: U32 = 1u32.wrapping_shl(tableLog);
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let ptr: *mut core::ffi::c_void = ct as *mut core::ffi::c_void;
    let tableU16: *mut U16 = (ptr as *mut U16).wrapping_add(2);
    let FSCT: *mut core::ffi::c_void = (ptr as *mut U32).wrapping_add(
        1 /* header */ + (if tableLog != 0 { (tableSize >> 1) as usize } else { 1 }),
    ) as *mut core::ffi::c_void;
    let symbolTT: *mut FSE_symbolCompressionTransform =
        FSCT as *mut FSE_symbolCompressionTransform;
    let step: U32 = FSE_TABLESTEP(tableSize);
    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);

    let cumul: *mut U16 = workSpace as *mut U16; /* size = maxSV1 */
    let tableSymbol: *mut BYTE =
        cumul.wrapping_add(maxSV1.wrapping_add(1) as usize) as *mut BYTE; /* size = tableSize */

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
            if *normalizedCounter.add((u - 1) as usize) == -1 {
                /* Low proba symbol */
                *cumul.add(u as usize) = (*cumul.add((u - 1) as usize)).wrapping_add(1);
                *tableSymbol.add(highThreshold as usize) = (u - 1) as BYTE;
                highThreshold = highThreshold.wrapping_sub(1);
            } else {
                *cumul.add(u as usize) = (*cumul.add((u - 1) as usize))
                    .wrapping_add(*normalizedCounter.add((u - 1) as usize) as U16);
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
        let spread: *mut BYTE = tableSymbol.wrapping_add(tableSize as usize); /* size = tableSize + 8 (may write beyond tableSize) */
        {
            let add: U64 = 0x0101010101010101u64;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let mut i: core::ffi::c_int;
                let n: core::ffi::c_int =
                    *normalizedCounter.add(s as usize) as core::ffi::c_int;
                MEM_write64(spread.wrapping_add(pos), sv);
                i = 8;
                while i < n {
                    MEM_write64(spread.wrapping_add(pos).wrapping_offset(i as isize), sv);
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
                let mut u: usize;
                u = 0;
                while u < unroll {
                    let uPosition: usize = (position.wrapping_add(u.wrapping_mul(step as usize)))
                        & (tableMask as usize);
                    *tableSymbol.add(uPosition) = *spread.wrapping_add(s.wrapping_add(u));
                    u += 1;
                }
                position = (position.wrapping_add(unroll.wrapping_mul(step as usize)))
                    & (tableMask as usize);
                s = s.wrapping_add(unroll);
            }
        }
    } else {
        let mut position: U32 = 0;
        let mut symbol: U32;
        symbol = 0;
        while symbol < maxSV1 {
            let mut nbOccurrences: core::ffi::c_int;
            let freq: core::ffi::c_int =
                *normalizedCounter.add(symbol as usize) as core::ffi::c_int;
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
        let mut u: U32;
        u = 0;
        while u < tableSize {
            let s: BYTE = *tableSymbol.add(u as usize); /* note : static analyzer may not understand tableSymbol is properly initialized */
            let idx: U16 = *cumul.add(s as usize);
            *cumul.add(s as usize) = idx.wrapping_add(1);
            *tableU16.add(idx as usize) = tableSize.wrapping_add(u) as U16; /* TableU16 : sorted by symbol order; gives next state value */
            u += 1;
        }
    }

    /* Build Symbol Transformation Table */
    {
        let mut total: core::ffi::c_uint = 0;
        let mut s: core::ffi::c_uint;
        s = 0;
        while s <= maxSymbolValue {
            match *normalizedCounter.add(s as usize) as core::ffi::c_int {
                0 => {
                    /* filling nonetheless, for compatibility with FSE_getMaxNbBits() */
                    (*symbolTT.add(s as usize)).deltaNbBits = (tableLog.wrapping_add(1) << 16)
                        .wrapping_sub(1u32.wrapping_shl(tableLog));
                }
                -1 | 1 => {
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        (tableLog << 16).wrapping_sub(1u32.wrapping_shl(tableLog));
                    (*symbolTT.add(s as usize)).deltaFindState =
                        total.wrapping_sub(1) as core::ffi::c_int;
                    total = total.wrapping_add(1);
                }
                _ => {
                    let maxBitsOut: U32 = tableLog.wrapping_sub(ZSTD_highbit32(
                        (*normalizedCounter.add(s as usize) as U32).wrapping_sub(1),
                    ));
                    let minStatePlus: U32 = (*normalizedCounter.add(s as usize) as U32)
                        .wrapping_shl(maxBitsOut);
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        (maxBitsOut << 16).wrapping_sub(minStatePlus);
                    (*symbolTT.add(s as usize)).deltaFindState = total
                        .wrapping_sub(*normalizedCounter.add(s as usize) as U32)
                        as core::ffi::c_int;
                    total =
                        total.wrapping_add(*normalizedCounter.add(s as usize) as U32);
                }
            }
            s += 1;
        }
    }

    0
}

/* #ifndef FSE_COMMONDEFS_ONLY  (FSE_COMMONDEFS_ONLY is not defined) */

/*-**************************************************************
*  FSE NCount encoding
****************************************************************/
#[unsafe(no_mangle)]
pub extern "C" fn FSE_NCountWriteBound(
    maxSymbolValue: core::ffi::c_uint,
    tableLog: core::ffi::c_uint,
) -> usize {
    let maxHeaderSize: usize = ((((maxSymbolValue.wrapping_add(1)).wrapping_mul(tableLog))
        .wrapping_add(4 /* bitCount initialized at 4 */)
        .wrapping_add(2 /* first two symbols may use one additional bit each */)
        / 8)
        .wrapping_add(1 /* round up to whole nb bytes */)
        .wrapping_add(2 /* additional two bytes for bitstream flush */))
        as usize;
    if maxSymbolValue != 0 {
        maxHeaderSize
    } else {
        FSE_NCOUNTBOUND
    } /* maxSymbolValue==0 ? use default */
}

pub unsafe fn FSE_writeNCount_generic(
    header: *mut core::ffi::c_void,
    headerBufferSize: usize,
    normalizedCounter: *const core::ffi::c_short,
    maxSymbolValue: core::ffi::c_uint,
    tableLog: core::ffi::c_uint,
    writeIsSafe: core::ffi::c_uint,
) -> usize {
    let ostart: *mut BYTE = header as *mut BYTE;
    let mut out: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(headerBufferSize);
    let mut nbBits: core::ffi::c_int;
    let tableSize: core::ffi::c_int = (1u32.wrapping_shl(tableLog)) as core::ffi::c_int;
    let mut remaining: core::ffi::c_int;
    let mut threshold: core::ffi::c_int;
    let mut bitStream: U32 = 0;
    let mut bitCount: core::ffi::c_int = 0;
    let mut symbol: core::ffi::c_uint = 0;
    let alphabetSize: core::ffi::c_uint = maxSymbolValue.wrapping_add(1);
    let mut previousIs0: core::ffi::c_int = 0;

    /* Table Size */
    bitStream = bitStream.wrapping_add((tableLog.wrapping_sub(FSE_MIN_TABLELOG)) << bitCount);
    bitCount += 4;

    /* Init */
    remaining = tableSize.wrapping_add(1); /* +1 for extra accuracy */
    threshold = tableSize;
    nbBits = (tableLog as core::ffi::c_int).wrapping_add(1);

    while (symbol < alphabetSize) && (remaining > 1) {
        /* stops at 1 */
        if previousIs0 != 0 {
            let mut start: core::ffi::c_uint = symbol;
            while (symbol < alphabetSize) && (*normalizedCounter.add(symbol as usize) == 0) {
                symbol = symbol.wrapping_add(1);
            }
            if symbol == alphabetSize {
                break; /* incorrect distribution */
            }
            while symbol >= start.wrapping_add(24) {
                start = start.wrapping_add(24);
                bitStream = bitStream.wrapping_add(0xFFFFu32 << bitCount);
                if (writeIsSafe == 0) && (out > oend.wrapping_sub(2)) {
                    return ERROR(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
                }
                *out.add(0) = bitStream as BYTE;
                *out.add(1) = (bitStream >> 8) as BYTE;
                out = out.wrapping_add(2);
                bitStream >>= 16;
            }
            while symbol >= start.wrapping_add(3) {
                start = start.wrapping_add(3);
                bitStream = bitStream.wrapping_add(3u32 << bitCount);
                bitCount += 2;
            }
            bitStream = bitStream.wrapping_add((symbol.wrapping_sub(start)) << bitCount);
            bitCount += 2;
            if bitCount > 16 {
                if (writeIsSafe == 0) && (out > oend.wrapping_sub(2)) {
                    return ERROR(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
                }
                *out.add(0) = bitStream as BYTE;
                *out.add(1) = (bitStream >> 8) as BYTE;
                out = out.wrapping_add(2);
                bitStream >>= 16;
                bitCount -= 16;
            }
        }
        {
            let mut count: core::ffi::c_int =
                *normalizedCounter.add(symbol as usize) as core::ffi::c_int;
            symbol = symbol.wrapping_add(1);
            let max: core::ffi::c_int = (2i32.wrapping_mul(threshold).wrapping_sub(1))
                .wrapping_sub(remaining);
            remaining = remaining.wrapping_sub(if count < 0 { count.wrapping_neg() } else { count });
            count = count.wrapping_add(1); /* +1 for extra accuracy */
            if count >= threshold {
                count = count.wrapping_add(max); /* [0..max[ [max..threshold[ (...) [threshold+max 2*threshold[ */
            }
            bitStream = bitStream.wrapping_add((count as U32) << bitCount);
            bitCount += nbBits;
            bitCount -= (count < max) as core::ffi::c_int;
            previousIs0 = (count == 1) as core::ffi::c_int;
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
                return ERROR(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
            }
            *out.add(0) = bitStream as BYTE;
            *out.add(1) = (bitStream >> 8) as BYTE;
            out = out.wrapping_add(2);
            bitStream >>= 16;
            bitCount -= 16;
        }
    }

    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC); /* incorrect normalized distribution */
    }

    /* flush remaining bitStream */
    if (writeIsSafe == 0) && (out > oend.wrapping_sub(2)) {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
    }
    *out.add(0) = bitStream as BYTE;
    *out.add(1) = (bitStream >> 8) as BYTE;
    out = out.wrapping_offset(((bitCount + 7) / 8) as isize);

    (out as usize).wrapping_sub(ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_writeNCount(
    buffer: *mut core::ffi::c_void,
    bufferSize: usize,
    normalizedCounter: *const core::ffi::c_short,
    maxSymbolValue: core::ffi::c_uint,
    tableLog: core::ffi::c_uint,
) -> usize {
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
pub fn FSE_minTableLog(srcSize: usize, maxSymbolValue: core::ffi::c_uint) -> core::ffi::c_uint {
    let minBitsSrc: U32 = ZSTD_highbit32(srcSize as U32).wrapping_add(1);
    /* `maxSymbolValue == 0` makes the C `ZSTD_highbit32()` evaluate
     * `31 - __builtin_clz(0)`, which is undefined behaviour. In the reference
     * build gcc folds this site to `31 - 0`, i.e. `ZSTD_highbit32(0) == 31`,
     * so `minBitsSymbols` becomes 33 and never wins the `min` below.
     * Reproduced verbatim; note this input is unreachable from the public
     * compression path, which routes single-symbol data to RLE first. */
    let minBitsSymbols: U32 = if maxSymbolValue == 0 {
        31u32.wrapping_add(2)
    } else {
        ZSTD_highbit32(maxSymbolValue).wrapping_add(2)
    };
    let minBits: U32 = if minBitsSrc < minBitsSymbols {
        minBitsSrc
    } else {
        minBitsSymbols
    };
    minBits
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_optimalTableLog_internal(
    maxTableLog: core::ffi::c_uint,
    srcSize: usize,
    maxSymbolValue: core::ffi::c_uint,
    minus: core::ffi::c_uint,
) -> core::ffi::c_uint {
    let maxBitsSrc: U32 =
        ZSTD_highbit32((srcSize.wrapping_sub(1)) as U32).wrapping_sub(minus);
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
pub extern "C" fn FSE_optimalTableLog(
    maxTableLog: core::ffi::c_uint,
    srcSize: usize,
    maxSymbolValue: core::ffi::c_uint,
) -> core::ffi::c_uint {
    FSE_optimalTableLog_internal(maxTableLog, srcSize, maxSymbolValue, 2)
}

/* Secondary normalization method.
   To be used when primary method fails. */

pub unsafe fn FSE_normalizeM2(
    norm: *mut core::ffi::c_short,
    tableLog: U32,
    count: *const core::ffi::c_uint,
    mut total: usize,
    maxSymbolValue: U32,
    lowProbCount: core::ffi::c_short,
) -> usize {
    let NOT_YET_ASSIGNED: core::ffi::c_short = -2;
    let mut s: U32;
    let mut distributed: U32 = 0;
    let mut ToDistribute: U32;

    /* Init */
    let lowThreshold: U32 = (total >> tableLog) as U32;
    let mut lowOne: U32 = ((total.wrapping_mul(3)) >> (tableLog.wrapping_add(1))) as U32;

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

    if (total / (ToDistribute as usize)) > (lowOne as usize) {
        /* risk of rounding to zero */
        lowOne = ((total.wrapping_mul(3)) / ((ToDistribute.wrapping_mul(2)) as usize)) as U32;
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
        *norm.add(maxV as usize) = (*norm.add(maxV as usize))
            .wrapping_add(ToDistribute as core::ffi::c_short);
        return 0;
    }

    if total == 0 {
        /* all of the symbols were low enough for the lowOne or lowThreshold */
        s = 0;
        while ToDistribute > 0 {
            if *norm.add(s as usize) > 0 {
                ToDistribute = ToDistribute.wrapping_sub(1);
                *norm.add(s as usize) = (*norm.add(s as usize)).wrapping_add(1);
            }
            s = (s.wrapping_add(1)) % (maxSymbolValue.wrapping_add(1));
        }
        return 0;
    }

    {
        let vStepLog: U64 = (62u32.wrapping_sub(tableLog)) as U64;
        let mid: U64 = (1u64.wrapping_shl((vStepLog.wrapping_sub(1)) as u32)).wrapping_sub(1);
        let rStep: U64 = ZSTD_div64(
            ((1u64.wrapping_shl(vStepLog as u32)).wrapping_mul(ToDistribute as U64))
                .wrapping_add(mid),
            total as U32,
        ); /* scale on remaining */
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
                *norm.add(s as usize) = weight as core::ffi::c_short;
                tmpTotal = end;
            }
            s += 1;
        }
    }

    0
}

/* `static U32 const rtbTable[]` local to FSE_normalizeCount() */
static rtbTable: [U32; 8] = [0, 473195, 504333, 520860, 550000, 700000, 750000, 830000];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_normalizeCount(
    normalizedCounter: *mut core::ffi::c_short,
    mut tableLog: core::ffi::c_uint,
    count: *const core::ffi::c_uint,
    total: usize,
    maxSymbolValue: core::ffi::c_uint,
    useLowProbCount: core::ffi::c_uint,
) -> usize {
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
        let lowProbCount: core::ffi::c_short = if useLowProbCount != 0 { -1 } else { 1 };
        let scale: U64 = (62u32.wrapping_sub(tableLog)) as U64;
        let step: U64 = ZSTD_div64(1u64 << 62, total as U32); /* <== here, one division ! */
        let vStep: U64 = 1u64.wrapping_shl((scale.wrapping_sub(20)) as u32);
        let mut stillToDistribute: core::ffi::c_int =
            (1u32.wrapping_shl(tableLog)) as core::ffi::c_int;
        let mut s: core::ffi::c_uint;
        let mut largest: core::ffi::c_uint = 0;
        let mut largestP: core::ffi::c_short = 0;
        let lowThreshold: U32 = (total >> tableLog) as U32;

        s = 0;
        while s <= maxSymbolValue {
            if (*count.add(s as usize) as usize) == total {
                return 0; /* rle special case */
            }
            if *count.add(s as usize) == 0 {
                *normalizedCounter.add(s as usize) = 0;
                s += 1;
                continue;
            }
            if *count.add(s as usize) <= lowThreshold {
                *normalizedCounter.add(s as usize) = lowProbCount;
                stillToDistribute = stillToDistribute.wrapping_sub(1);
            } else {
                let mut proba: core::ffi::c_short =
                    (((*count.add(s as usize) as U64).wrapping_mul(step)) >> scale)
                        as core::ffi::c_short;
                if (proba as core::ffi::c_int) < 8 {
                    let restToBeat: U64 = vStep
                        .wrapping_mul(*rtbTable.as_ptr().offset(proba as isize) as U64);
                    proba = proba.wrapping_add(
                        ((((*count.add(s as usize) as U64).wrapping_mul(step))
                            .wrapping_sub((proba as U64) << scale))
                            > restToBeat) as core::ffi::c_short,
                    );
                }
                if (proba as core::ffi::c_int) > (largestP as core::ffi::c_int) {
                    largestP = proba;
                    largest = s;
                }
                *normalizedCounter.add(s as usize) = proba;
                stillToDistribute =
                    stillToDistribute.wrapping_sub(proba as core::ffi::c_int);
            }
            s += 1;
        }
        if stillToDistribute.wrapping_neg()
            >= ((*normalizedCounter.add(largest as usize) as core::ffi::c_int) >> 1)
        {
            /* corner case, need another normalization method */
            let errorCode: usize = FSE_normalizeM2(
                normalizedCounter,
                tableLog,
                count,
                total,
                maxSymbolValue,
                lowProbCount,
            );
            if FSE_isError(errorCode) != 0 {
                return errorCode;
            }
        } else {
            *normalizedCounter.add(largest as usize) = (*normalizedCounter
                .add(largest as usize))
            .wrapping_add(stillToDistribute as core::ffi::c_short);
        }
    }

    tableLog as usize
}

/* fake FSE_CTable, for rle input (always same symbol) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_rle(
    ct: *mut FSE_CTable,
    symbolValue: core::ffi::c_uchar,
) -> usize {
    let ptr: *mut core::ffi::c_void = ct as *mut core::ffi::c_void;
    let tableU16: *mut U16 = (ptr as *mut U16).wrapping_add(2);
    let FSCTptr: *mut core::ffi::c_void =
        (ptr as *mut U32).wrapping_add(2) as *mut core::ffi::c_void;
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

pub unsafe fn FSE_compress_usingCTable_generic(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    mut srcSize: usize,
    ct: *const FSE_CTable,
    fast: core::ffi::c_uint,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let mut ip: *const BYTE = iend;

    let mut bitC: BIT_CStream_t = BIT_CStream_t::default();
    let mut CState1: FSE_CState_t = FSE_CState_t::default();
    let mut CState2: FSE_CState_t = FSE_CState_t::default();

    /* init */
    if srcSize <= 2 {
        return 0;
    }
    {
        let initError: usize = BIT_initCStream(&mut bitC, dst as *mut u8, dstSize);
        if FSE_isError(initError) != 0 {
            return 0; /* not enough space available to write a bitstream */
        }
    }

    /* #define FSE_FLUSHBITS(s)  (fast ? BIT_flushBitsFast(s) : BIT_flushBits(s)) */

    if (srcSize & 1) != 0 {
        ip = ip.wrapping_sub(1);
        FSE_initCState2(&mut CState1, ct, *ip as U32);
        ip = ip.wrapping_sub(1);
        FSE_initCState2(&mut CState2, ct, *ip as U32);
        ip = ip.wrapping_sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as core::ffi::c_uint);
        if fast != 0 {
            BIT_flushBitsFast(&mut bitC)
        } else {
            BIT_flushBits(&mut bitC)
        };
    } else {
        ip = ip.wrapping_sub(1);
        FSE_initCState2(&mut CState2, ct, *ip as U32);
        ip = ip.wrapping_sub(1);
        FSE_initCState2(&mut CState1, ct, *ip as U32);
    }

    /* join to mod 4 */
    srcSize = srcSize.wrapping_sub(2);
    if (core::mem::size_of_val(&bitC.bitContainer) * 8
        > (FSE_MAX_TABLELOG * 4 + 7) as usize)
        && ((srcSize & 2) != 0)
    {
        /* test bit 2 */
        ip = ip.wrapping_sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as core::ffi::c_uint);
        ip = ip.wrapping_sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as core::ffi::c_uint);
        if fast != 0 {
            BIT_flushBitsFast(&mut bitC)
        } else {
            BIT_flushBits(&mut bitC)
        };
    }

    /* 2 or 4 encoding per loop */
    while ip > istart {
        ip = ip.wrapping_sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as core::ffi::c_uint);

        if core::mem::size_of_val(&bitC.bitContainer) * 8
            < (FSE_MAX_TABLELOG * 2 + 7) as usize
        {
            /* this test must be static */
            if fast != 0 {
                BIT_flushBitsFast(&mut bitC)
            } else {
                BIT_flushBits(&mut bitC)
            };
        }

        ip = ip.wrapping_sub(1);
        FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as core::ffi::c_uint);

        if core::mem::size_of_val(&bitC.bitContainer) * 8
            > (FSE_MAX_TABLELOG * 4 + 7) as usize
        {
            /* this test must be static */
            ip = ip.wrapping_sub(1);
            FSE_encodeSymbol(&mut bitC, &mut CState2, *ip as core::ffi::c_uint);
            ip = ip.wrapping_sub(1);
            FSE_encodeSymbol(&mut bitC, &mut CState1, *ip as core::ffi::c_uint);
        }

        if fast != 0 {
            BIT_flushBitsFast(&mut bitC)
        } else {
            BIT_flushBits(&mut bitC)
        };
    }

    FSE_flushCState(&mut bitC, &CState2);
    FSE_flushCState(&mut bitC, &CState1);
    BIT_closeCStream(&mut bitC)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_compress_usingCTable(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    ct: *const FSE_CTable,
) -> usize {
    let fast: core::ffi::c_uint = (dstSize >= FSE_BLOCKBOUND(srcSize)) as core::ffi::c_uint;

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

/* #endif   /* FSE_COMMONDEFS_ONLY */ */
