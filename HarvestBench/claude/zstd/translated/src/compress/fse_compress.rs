//! Translation of compress/fse_compress.c — FSE encoder.
#![allow(dead_code)]
use crate::common::bits::highbit32;
use crate::common::bitstream::*;
use crate::common::error::{code, err_is_error, error};
use crate::common::fse::*;
use crate::common::mem::*;
use core::ffi::c_void;

#[inline]
fn fse_build_ctable_workspace_size_u32(maxSymbolValue: u32, tableLog: u32) -> usize {
    (((maxSymbolValue + 2) as u64 + (1u64 << tableLog)) / 2 + (8 / 4)) as usize
}
#[inline]
fn fse_build_ctable_workspace_size(maxSymbolValue: u32, tableLog: u32) -> usize {
    4 * fse_build_ctable_workspace_size_u32(maxSymbolValue, tableLog)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_wksp(
    ct: *mut FSE_CTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let tableSize = 1u32 << tableLog;
    let tableMask = tableSize - 1;
    let ptr = ct as *mut c_void;
    let tableU16 = (ptr as *mut u16).add(2);
    let fsct = (ptr as *mut u32).add(1 + if tableLog != 0 { (tableSize >> 1) as usize } else { 1 });
    let symbolTT = fsct as *mut FSE_symbolCompressionTransform;
    let step = fse_tablestep(tableSize);
    let maxSV1 = maxSymbolValue + 1;

    let cumul = workSpace as *mut u16;
    let tableSymbol = cumul.add(maxSV1 as usize + 1) as *mut u8;

    let mut highThreshold = tableSize - 1;

    if fse_build_ctable_workspace_size(maxSymbolValue, tableLog) > wkspSize {
        return error(code::TABLELOG_TOOLARGE);
    }
    *tableU16.offset(-2) = tableLog as u16;
    *tableU16.offset(-1) = maxSymbolValue as u16;

    // symbol start positions
    {
        *cumul.add(0) = 0;
        for u in 1..=maxSV1 {
            if *normalizedCounter.add((u - 1) as usize) == -1 {
                *cumul.add(u as usize) = *cumul.add((u - 1) as usize) + 1;
                *tableSymbol.add(highThreshold as usize) = (u - 1) as u8;
                highThreshold -= 1;
            } else {
                *cumul.add(u as usize) =
                    *cumul.add((u - 1) as usize) + *normalizedCounter.add((u - 1) as usize) as u16;
            }
        }
        *cumul.add(maxSV1 as usize) = (tableSize + 1) as u16;
    }

    // Spread symbols
    if highThreshold == tableSize - 1 {
        let spread = tableSymbol.add(tableSize as usize);
        {
            let add = 0x0101010101010101u64;
            let mut pos: usize = 0;
            let mut sv: u64 = 0;
            for s in 0..maxSV1 {
                let n = *normalizedCounter.add(s as usize) as i32;
                mem_write64(spread.add(pos) as *mut c_void, sv);
                let mut i = 8;
                while i < n {
                    mem_write64(spread.add(pos + i as usize) as *mut c_void, sv);
                    i += 8;
                }
                pos += n as usize;
                sv = sv.wrapping_add(add);
            }
        }
        {
            let mut position: usize = 0;
            let unroll = 2usize;
            let mut s = 0usize;
            while s < tableSize as usize {
                for u in 0..unroll {
                    let uPosition = (position + (u * step as usize)) & tableMask as usize;
                    *tableSymbol.add(uPosition) = *spread.add(s + u);
                }
                position = (position + (unroll * step as usize)) & tableMask as usize;
                s += unroll;
            }
        }
    } else {
        let mut position: u32 = 0;
        for symbol in 0..maxSV1 {
            let freq = *normalizedCounter.add(symbol as usize) as i32;
            for _ in 0..freq {
                *tableSymbol.add(position as usize) = symbol as u8;
                position = (position + step) & tableMask;
                while position > highThreshold {
                    position = (position + step) & tableMask;
                }
            }
        }
    }

    // Build table
    {
        for u in 0..tableSize {
            let s = *tableSymbol.add(u as usize);
            let c = *cumul.add(s as usize);
            *tableU16.add(c as usize) = (tableSize + u) as u16;
            *cumul.add(s as usize) = c + 1;
        }
    }

    // Build Symbol Transformation Table
    {
        let mut total: u32 = 0;
        for s in 0..=maxSymbolValue {
            let nc = *normalizedCounter.add(s as usize);
            match nc {
                0 => {
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        ((tableLog + 1) << 16).wrapping_sub(1 << tableLog);
                }
                -1 | 1 => {
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        (tableLog << 16).wrapping_sub(1 << tableLog);
                    (*symbolTT.add(s as usize)).deltaFindState = (total.wrapping_sub(1)) as i32;
                    total += 1;
                }
                _ => {
                    let ncu = nc as u32;
                    let maxBitsOut = tableLog - highbit32(ncu - 1);
                    let minStatePlus = ncu << maxBitsOut;
                    (*symbolTT.add(s as usize)).deltaNbBits =
                        (maxBitsOut << 16).wrapping_sub(minStatePlus);
                    (*symbolTT.add(s as usize)).deltaFindState =
                        (total.wrapping_sub(ncu)) as i32;
                    total += ncu;
                }
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_NCountWriteBound(maxSymbolValue: u32, tableLog: u32) -> usize {
    let maxHeaderSize = (((maxSymbolValue + 1) * tableLog + 4 + 2) / 8) as usize + 1 + 2;
    if maxSymbolValue != 0 {
        maxHeaderSize
    } else {
        FSE_NCOUNTBOUND
    }
}

unsafe fn fse_write_ncount_generic(
    header: *mut c_void,
    headerBufferSize: usize,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
    writeIsSafe: u32,
) -> usize {
    let ostart = header as *mut u8;
    let mut out = ostart;
    let oend = ostart.add(headerBufferSize);
    let mut nbBits: i32;
    let tableSize = 1i32 << tableLog;
    let mut remaining: i32;
    let mut threshold: i32;
    let mut bitStream: u32 = 0;
    let mut bitCount: i32 = 0;
    let mut symbol: u32 = 0;
    let alphabetSize = maxSymbolValue + 1;
    let mut previousIs0 = false;

    bitStream += (tableLog - FSE_MIN_TABLELOG) << bitCount;
    bitCount += 4;

    remaining = tableSize + 1;
    threshold = tableSize;
    nbBits = tableLog as i32 + 1;

    while (symbol < alphabetSize) && (remaining > 1) {
        if previousIs0 {
            let mut start = symbol;
            while (symbol < alphabetSize) && *normalizedCounter.add(symbol as usize) == 0 {
                symbol += 1;
            }
            if symbol == alphabetSize {
                break;
            }
            while symbol >= start + 24 {
                start += 24;
                bitStream += 0xFFFFu32 << bitCount;
                if writeIsSafe == 0 && out > oend.sub(2) {
                    return error(code::DSTSIZE_TOOSMALL);
                }
                *out.add(0) = bitStream as u8;
                *out.add(1) = (bitStream >> 8) as u8;
                out = out.add(2);
                bitStream >>= 16;
            }
            while symbol >= start + 3 {
                start += 3;
                bitStream += 3u32 << bitCount;
                bitCount += 2;
            }
            bitStream += (symbol - start) << bitCount;
            bitCount += 2;
            if bitCount > 16 {
                if writeIsSafe == 0 && out > oend.sub(2) {
                    return error(code::DSTSIZE_TOOSMALL);
                }
                *out.add(0) = bitStream as u8;
                *out.add(1) = (bitStream >> 8) as u8;
                out = out.add(2);
                bitStream >>= 16;
                bitCount -= 16;
            }
        }
        {
            let count0 = *normalizedCounter.add(symbol as usize) as i32;
            symbol += 1;
            let max = (2 * threshold - 1) - remaining;
            remaining -= if count0 < 0 { -count0 } else { count0 };
            let mut count = count0 + 1;
            if count >= threshold {
                count += max;
            }
            bitStream += (count as u32) << bitCount;
            bitCount += nbBits;
            bitCount -= (count < max) as i32;
            previousIs0 = count == 1;
            if remaining < 1 {
                return error(code::GENERIC);
            }
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }
        }
        if bitCount > 16 {
            if writeIsSafe == 0 && out > oend.sub(2) {
                return error(code::DSTSIZE_TOOSMALL);
            }
            *out.add(0) = bitStream as u8;
            *out.add(1) = (bitStream >> 8) as u8;
            out = out.add(2);
            bitStream >>= 16;
            bitCount -= 16;
        }
    }

    if remaining != 1 {
        return error(code::GENERIC);
    }

    if writeIsSafe == 0 && out > oend.sub(2) {
        return error(code::DSTSIZE_TOOSMALL);
    }
    *out.add(0) = bitStream as u8;
    *out.add(1) = (bitStream >> 8) as u8;
    out = out.add(((bitCount + 7) / 8) as usize);

    (out as usize) - (ostart as usize)
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
        return error(code::TABLELOG_TOOLARGE);
    }
    if tableLog < FSE_MIN_TABLELOG {
        return error(code::GENERIC);
    }
    if bufferSize < FSE_NCountWriteBound(maxSymbolValue, tableLog) {
        return fse_write_ncount_generic(buffer, bufferSize, normalizedCounter, maxSymbolValue, tableLog, 0);
    }
    fse_write_ncount_generic(buffer, bufferSize, normalizedCounter, maxSymbolValue, tableLog, 1)
}

fn fse_min_table_log(srcSize: usize, maxSymbolValue: u32) -> u32 {
    let minBitsSrc = highbit32(srcSize as u32) + 1;
    let minBitsSymbols = highbit32(maxSymbolValue) + 2;
    if minBitsSrc < minBitsSymbols {
        minBitsSrc
    } else {
        minBitsSymbols
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_optimalTableLog_internal(
    maxTableLog: u32,
    srcSize: usize,
    maxSymbolValue: u32,
    minus: u32,
) -> u32 {
    let maxBitsSrc = highbit32((srcSize - 1) as u32) - minus;
    let mut tableLog = maxTableLog;
    let minBits = fse_min_table_log(srcSize, maxSymbolValue);
    if tableLog == 0 {
        tableLog = FSE_DEFAULT_TABLELOG;
    }
    if maxBitsSrc < tableLog {
        tableLog = maxBitsSrc;
    }
    if minBits > tableLog {
        tableLog = minBits;
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
pub extern "C" fn FSE_optimalTableLog(maxTableLog: u32, srcSize: usize, maxSymbolValue: u32) -> u32 {
    FSE_optimalTableLog_internal(maxTableLog, srcSize, maxSymbolValue, 2)
}

unsafe fn fse_normalize_m2(
    norm: *mut i16,
    tableLog: u32,
    count: *const u32,
    mut total: usize,
    maxSymbolValue: u32,
    lowProbCount: i16,
) -> usize {
    const NOT_YET_ASSIGNED: i16 = -2;
    let mut distributed: u32 = 0;

    let lowThreshold = (total >> tableLog) as u32;
    let mut lowOne = ((total * 3) >> (tableLog + 1)) as u32;

    for s in 0..=maxSymbolValue {
        let cs = *count.add(s as usize);
        if cs == 0 {
            *norm.add(s as usize) = 0;
            continue;
        }
        if cs <= lowThreshold {
            *norm.add(s as usize) = lowProbCount;
            distributed += 1;
            total -= cs as usize;
            continue;
        }
        if cs <= lowOne {
            *norm.add(s as usize) = 1;
            distributed += 1;
            total -= cs as usize;
            continue;
        }
        *norm.add(s as usize) = NOT_YET_ASSIGNED;
    }
    let mut ToDistribute = (1u32 << tableLog) - distributed;

    if ToDistribute == 0 {
        return 0;
    }

    if (total as u32 / ToDistribute) > lowOne {
        lowOne = ((total * 3) / (ToDistribute as usize * 2)) as u32;
        for s in 0..=maxSymbolValue {
            if *norm.add(s as usize) == NOT_YET_ASSIGNED && *count.add(s as usize) <= lowOne {
                *norm.add(s as usize) = 1;
                distributed += 1;
                total -= *count.add(s as usize) as usize;
            }
        }
        ToDistribute = (1u32 << tableLog) - distributed;
    }

    if distributed == maxSymbolValue + 1 {
        let mut maxV = 0u32;
        let mut maxC = 0u32;
        for s in 0..=maxSymbolValue {
            if *count.add(s as usize) > maxC {
                maxV = s;
                maxC = *count.add(s as usize);
            }
        }
        *norm.add(maxV as usize) += ToDistribute as i16;
        return 0;
    }

    if total == 0 {
        let mut s = 0u32;
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
        let vStepLog = 62 - tableLog as u64;
        let mid = (1u64 << (vStepLog - 1)) - 1;
        let rStep = (((1u64 << vStepLog) * ToDistribute as u64) + mid) / (total as u64);
        let mut tmpTotal = mid;
        for s in 0..=maxSymbolValue {
            if *norm.add(s as usize) == NOT_YET_ASSIGNED {
                let end = tmpTotal + (*count.add(s as usize) as u64 * rStep);
                let sStart = (tmpTotal >> vStepLog) as u32;
                let sEnd = (end >> vStepLog) as u32;
                let weight = sEnd - sStart;
                if weight < 1 {
                    return error(code::GENERIC);
                }
                *norm.add(s as usize) = weight as i16;
                tmpTotal = end;
            }
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
    if tableLog == 0 {
        tableLog = FSE_DEFAULT_TABLELOG;
    }
    if tableLog < FSE_MIN_TABLELOG {
        return error(code::GENERIC);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return error(code::TABLELOG_TOOLARGE);
    }
    if tableLog < fse_min_table_log(total, maxSymbolValue) {
        return error(code::GENERIC);
    }

    {
        static RTB_TABLE: [u32; 8] =
            [0, 473195, 504333, 520860, 550000, 700000, 750000, 830000];
        let lowProbCount: i16 = if useLowProbCount != 0 { -1 } else { 1 };
        let scale = 62 - tableLog as u64;
        let step = (1u64 << 62) / (total as u64);
        let vStep = 1u64 << (scale - 20);
        let mut stillToDistribute: i32 = 1 << tableLog;
        let mut largest: u32 = 0;
        let mut largestP: i16 = 0;
        let lowThreshold = (total >> tableLog) as u32;

        for s in 0..=maxSymbolValue {
            let cs = *count.add(s as usize);
            if cs as usize == total {
                return 0;
            }
            if cs == 0 {
                *normalizedCounter.add(s as usize) = 0;
                continue;
            }
            if cs <= lowThreshold {
                *normalizedCounter.add(s as usize) = lowProbCount;
                stillToDistribute -= 1;
            } else {
                let mut proba = ((cs as u64 * step) >> scale) as i16;
                if proba < 8 {
                    let restToBeat = vStep * RTB_TABLE[proba as usize] as u64;
                    proba += ((cs as u64 * step) - ((proba as u64) << scale) > restToBeat) as i16;
                }
                if proba > largestP {
                    largestP = proba;
                    largest = s;
                }
                *normalizedCounter.add(s as usize) = proba;
                stillToDistribute -= proba as i32;
            }
        }
        if -stillToDistribute >= (*normalizedCounter.add(largest as usize) as i32 >> 1) {
            let errorCode =
                fse_normalize_m2(normalizedCounter, tableLog, count, total, maxSymbolValue, lowProbCount);
            if err_is_error(errorCode) != 0 {
                return errorCode;
            }
        } else {
            *normalizedCounter.add(largest as usize) += stillToDistribute as i16;
        }
    }
    tableLog as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_rle(ct: *mut FSE_CTable, symbolValue: u8) -> usize {
    let ptr = ct as *mut c_void;
    let tableU16 = (ptr as *mut u16).add(2);
    let fsctptr = (ptr as *mut u32).add(2);
    let symbolTT = fsctptr as *mut FSE_symbolCompressionTransform;

    *tableU16.offset(-2) = 0;
    *tableU16.offset(-1) = symbolValue as u16;

    *tableU16.add(0) = 0;
    *tableU16.add(1) = 0;

    (*symbolTT.add(symbolValue as usize)).deltaNbBits = 0;
    (*symbolTT.add(symbolValue as usize)).deltaFindState = 0;
    0
}

unsafe fn fse_compress_using_ctable_generic(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    mut srcSize: usize,
    ct: *const FSE_CTable,
    fast: u32,
) -> usize {
    let istart = src as *const u8;
    let iend = istart.add(srcSize);
    let mut ip = iend;

    let mut bitC: BIT_CStream_t = core::mem::zeroed();
    let mut cstate1: FSE_CState_t = core::mem::zeroed();
    let mut cstate2: FSE_CState_t = core::mem::zeroed();

    if srcSize <= 2 {
        return 0;
    }
    {
        let initError = bit_init_cstream(&mut bitC, dst, dstSize);
        if err_is_error(initError) != 0 {
            return 0;
        }
    }

    macro_rules! flushbits {
        ($s:expr) => {
            if fast != 0 {
                bit_flush_bits_fast($s)
            } else {
                bit_flush_bits($s)
            }
        };
    }

    if srcSize & 1 != 0 {
        ip = ip.sub(1);
        fse_init_cstate2(&mut cstate1, ct, *ip as u32);
        ip = ip.sub(1);
        fse_init_cstate2(&mut cstate2, ct, *ip as u32);
        ip = ip.sub(1);
        fse_encode_symbol(&mut bitC, &mut cstate1, *ip as u32);
        flushbits!(&mut bitC);
    } else {
        ip = ip.sub(1);
        fse_init_cstate2(&mut cstate2, ct, *ip as u32);
        ip = ip.sub(1);
        fse_init_cstate2(&mut cstate1, ct, *ip as u32);
    }

    srcSize -= 2;
    // sizeof(bitContainer)*8 = 64 > FSE_MAX_TABLELOG*4+7 = 55 : true
    if (64 > FSE_MAX_TABLELOG * 4 + 7) && (srcSize & 2 != 0) {
        ip = ip.sub(1);
        fse_encode_symbol(&mut bitC, &mut cstate2, *ip as u32);
        ip = ip.sub(1);
        fse_encode_symbol(&mut bitC, &mut cstate1, *ip as u32);
        flushbits!(&mut bitC);
    }

    while ip > istart {
        ip = ip.sub(1);
        fse_encode_symbol(&mut bitC, &mut cstate2, *ip as u32);
        // 64 < 31 : false, skip static flush
        ip = ip.sub(1);
        fse_encode_symbol(&mut bitC, &mut cstate1, *ip as u32);
        // 64 > 55 : true
        ip = ip.sub(1);
        fse_encode_symbol(&mut bitC, &mut cstate2, *ip as u32);
        ip = ip.sub(1);
        fse_encode_symbol(&mut bitC, &mut cstate1, *ip as u32);
        flushbits!(&mut bitC);
    }

    fse_flush_cstate(&mut bitC, &cstate2);
    fse_flush_cstate(&mut bitC, &cstate1);
    bit_close_cstream(&mut bitC)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_compress_usingCTable(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    ct: *const FSE_CTable,
) -> usize {
    let fast = (dstSize >= fse_blockbound(srcSize)) as u32;
    if fast != 0 {
        fse_compress_using_ctable_generic(dst, dstSize, src, srcSize, ct, 1)
    } else {
        fse_compress_using_ctable_generic(dst, dstSize, src, srcSize, ct, 0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_compressBound(size: usize) -> usize {
    fse_compressbound(size)
}
