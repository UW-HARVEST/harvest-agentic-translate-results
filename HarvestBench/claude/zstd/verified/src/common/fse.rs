//! Translation of common/fse.h inline functions, fse_decompress.c, and the
//! FSE portions of entropy_common.c.
#![allow(dead_code)]
use super::bitstream::*;
use super::bits::highbit32;
use super::error::{code, error, err_is_error};
use super::mem::*;
use core::ffi::c_void;

pub const FSE_MAX_MEMORY_USAGE: u32 = 14;
pub const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSE_MAX_SYMBOL_VALUE: u32 = 255;
pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
pub const FSE_MAX_TABLESIZE: u32 = 1 << FSE_MAX_TABLELOG;
pub const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
pub const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
pub const FSE_MIN_TABLELOG: u32 = 5;
pub const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

#[inline]
pub fn fse_tablestep(table_size: u32) -> u32 {
    (table_size >> 1) + (table_size >> 3) + 3
}

pub const FSE_NCOUNTBOUND: usize = 512;
#[inline]
pub fn fse_blockbound(size: usize) -> usize {
    size + (size >> 7) + 4 + core::mem::size_of::<usize>()
}
#[inline]
pub fn fse_compressbound(size: usize) -> usize {
    FSE_NCOUNTBOUND + fse_blockbound(size)
}

pub type FSE_CTable = u32;
pub type FSE_DTable = u32;

pub const FSE_repeat_none: u32 = 0;
pub const FSE_repeat_check: u32 = 1;
pub const FSE_repeat_valid: u32 = 2;
pub type FSE_repeat = u32;

// table size macros
#[inline]
pub fn fse_ctable_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    (1 + (1 << (max_table_log - 1)) + ((max_symbol_value + 1) * 2)) as usize
}
#[inline]
pub fn fse_dtable_size_u32(max_table_log: u32) -> usize {
    (1 + (1 << max_table_log)) as usize
}
#[inline]
pub fn fse_dtable_size(max_table_log: u32) -> usize {
    fse_dtable_size_u32(max_table_log) * core::mem::size_of::<FSE_DTable>()
}
#[inline]
pub fn fse_build_dtable_wksp_size(max_table_log: u32, max_symbol_value: u32) -> usize {
    core::mem::size_of::<i16>() * (max_symbol_value as usize + 1)
        + (1usize << max_table_log)
        + 8
}
#[inline]
pub fn fse_decompress_wksp_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    fse_dtable_size_u32(max_table_log)
        + 1
        + (fse_build_dtable_wksp_size(max_table_log, max_symbol_value) + core::mem::size_of::<u32>()
            - 1)
            / core::mem::size_of::<u32>()
        + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2
        + 1
}
#[inline]
pub fn fse_decompress_wksp_size(max_table_log: u32, max_symbol_value: u32) -> usize {
    fse_decompress_wksp_size_u32(max_table_log, max_symbol_value) * core::mem::size_of::<u32>()
}

// ===== State structs =====
#[repr(C)]
pub struct FSE_CState_t {
    pub value: isize,
    pub stateTable: *const c_void,
    pub symbolTT: *const c_void,
    pub stateLog: u32,
}

#[repr(C)]
pub struct FSE_DState_t {
    pub state: usize,
    pub table: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_symbolCompressionTransform {
    pub deltaFindState: i32,
    pub deltaNbBits: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_DTableHeader {
    pub tableLog: u16,
    pub fastMode: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
}

// ===== CState inline functions =====
#[inline]
pub unsafe fn fse_init_cstate(statePtr: *mut FSE_CState_t, ct: *const FSE_CTable) {
    let ptr = ct as *const c_void;
    let tableLog = mem_read16(ptr) as u32;
    (*statePtr).value = (1isize) << tableLog;
    (*statePtr).stateTable = (ct as *const u16).add(2) as *const c_void;
    (*statePtr).symbolTT = ct.add(1 + if tableLog != 0 { 1 << (tableLog - 1) } else { 1 }) as *const c_void;
    (*statePtr).stateLog = tableLog;
}

#[inline]
pub unsafe fn fse_init_cstate2(statePtr: *mut FSE_CState_t, ct: *const FSE_CTable, symbol: u32) {
    fse_init_cstate(statePtr, ct);
    let symbolTT = *((*statePtr).symbolTT as *const FSE_symbolCompressionTransform).add(symbol as usize);
    let stateTable = (*statePtr).stateTable as *const u16;
    let nbBitsOut = (symbolTT.deltaNbBits + (1 << 15)) >> 16;
    (*statePtr).value = ((nbBitsOut << 16) as i32 - symbolTT.deltaNbBits as i32) as isize;
    (*statePtr).value = *stateTable
        .offset(((*statePtr).value >> nbBitsOut) as isize + symbolTT.deltaFindState as isize)
        as isize;
}

#[inline]
pub unsafe fn fse_encode_symbol(bitC: *mut BIT_CStream_t, statePtr: *mut FSE_CState_t, symbol: u32) {
    let symbolTT = *((*statePtr).symbolTT as *const FSE_symbolCompressionTransform).add(symbol as usize);
    let stateTable = (*statePtr).stateTable as *const u16;
    let nbBitsOut = (((*statePtr).value as i64 + symbolTT.deltaNbBits as i64) >> 16) as u32;
    bit_add_bits(bitC, (*statePtr).value as BitContainerType, nbBitsOut);
    (*statePtr).value = *stateTable
        .offset(((*statePtr).value >> nbBitsOut) + symbolTT.deltaFindState as isize)
        as isize;
}

#[inline]
pub unsafe fn fse_flush_cstate(bitC: *mut BIT_CStream_t, statePtr: *const FSE_CState_t) {
    bit_add_bits(bitC, (*statePtr).value as BitContainerType, (*statePtr).stateLog);
    bit_flush_bits(bitC);
}

#[inline]
pub unsafe fn fse_get_max_nb_bits(symbolTTPtr: *const c_void, symbolValue: u32) -> u32 {
    let symbolTT = symbolTTPtr as *const FSE_symbolCompressionTransform;
    ((*symbolTT.add(symbolValue as usize)).deltaNbBits + ((1 << 16) - 1)) >> 16
}

#[inline]
pub unsafe fn fse_bit_cost(symbolTTPtr: *const c_void, tableLog: u32, symbolValue: u32, accuracyLog: u32) -> u32 {
    let symbolTT = symbolTTPtr as *const FSE_symbolCompressionTransform;
    let minNbBits = (*symbolTT.add(symbolValue as usize)).deltaNbBits >> 16;
    let threshold = (minNbBits + 1) << 16;
    let tableSize = 1u32 << tableLog;
    let deltaFromThreshold = threshold - ((*symbolTT.add(symbolValue as usize)).deltaNbBits + tableSize);
    let normalizedDeltaFromThreshold = (deltaFromThreshold << accuracyLog) >> tableLog;
    let bitMultiplier = 1u32 << accuracyLog;
    (minNbBits + 1) * bitMultiplier - normalizedDeltaFromThreshold
}

// ===== DState inline functions =====
#[inline]
pub unsafe fn fse_init_dstate(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t, dt: *const FSE_DTable) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSE_DTableHeader;
    (*DStatePtr).state = bit_read_bits(bitD, (*DTableH).tableLog as u32);
    bit_reload_dstream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline]
pub unsafe fn fse_peek_symbol(DStatePtr: *const FSE_DState_t) -> u8 {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

#[inline]
pub unsafe fn fse_update_state(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as u32;
    let lowBits = bit_read_bits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
}

#[inline]
pub unsafe fn fse_decode_symbol(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> u8 {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as u32;
    let symbol = DInfo.symbol;
    let lowBits = bit_read_bits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
pub unsafe fn fse_decode_symbol_fast(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> u8 {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as u32;
    let symbol = DInfo.symbol;
    let lowBits = bit_read_bits_fast(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
pub unsafe fn fse_end_of_dstate(DStatePtr: *const FSE_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

// ===================================================================
// fse_decompress.c
// ===================================================================
pub unsafe fn fse_build_dtable_internal(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSE_decode_t;
    let symbolNext = workSpace as *mut u16;
    let spread = symbolNext.add(maxSymbolValue as usize + 1) as *mut u8;

    let maxSV1 = maxSymbolValue + 1;
    let tableSize = 1u32 << tableLog;
    let mut highThreshold = tableSize - 1;

    if fse_build_dtable_wksp_size(tableLog, maxSymbolValue) > wkspSize {
        return error(code::MAXSYMBOLVALUE_TOOLARGE);
    }
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return error(code::MAXSYMBOLVALUE_TOOLARGE);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return error(code::TABLELOG_TOOLARGE);
    }

    let mut DTableH = FSE_DTableHeader { tableLog: tableLog as u16, fastMode: 1 };
    {
        let largeLimit = (1i16) << (tableLog - 1);
        for s in 0..maxSV1 {
            let nc = *normalizedCounter.add(s as usize);
            if nc == -1 {
                (*tableDecode.add(highThreshold as usize)).symbol = s as u8;
                highThreshold -= 1;
                *symbolNext.add(s as usize) = 1;
            } else {
                if nc >= largeLimit {
                    DTableH.fastMode = 0;
                }
                *symbolNext.add(s as usize) = nc as u16;
            }
        }
    }
    core::ptr::copy_nonoverlapping(
        &DTableH as *const FSE_DTableHeader as *const u8,
        dt as *mut u8,
        core::mem::size_of::<FSE_DTableHeader>(),
    );

    if highThreshold == tableSize - 1 {
        let tableMask = (tableSize - 1) as usize;
        let step = fse_tablestep(tableSize) as usize;
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
                    let uPosition = (position + (u * step)) & tableMask;
                    (*tableDecode.add(uPosition)).symbol = *spread.add(s + u);
                }
                position = (position + (unroll * step)) & tableMask;
                s += unroll;
            }
        }
    } else {
        let tableMask = tableSize - 1;
        let step = fse_tablestep(tableSize);
        let mut position: u32 = 0;
        for s in 0..maxSV1 {
            let nc = *normalizedCounter.add(s as usize) as i32;
            for _ in 0..nc {
                (*tableDecode.add(position as usize)).symbol = s as u8;
                position = (position + step) & tableMask;
                while position > highThreshold {
                    position = (position + step) & tableMask;
                }
            }
        }
        if position != 0 {
            return error(code::GENERIC);
        }
    }

    {
        for u in 0..tableSize {
            let symbol = (*tableDecode.add(u as usize)).symbol;
            let nextState = *symbolNext.add(symbol as usize);
            *symbolNext.add(symbol as usize) = nextState + 1;
            (*tableDecode.add(u as usize)).nbBits = (tableLog - highbit32(nextState as u32)) as u8;
            (*tableDecode.add(u as usize)).newState =
                (((nextState as u32) << (*tableDecode.add(u as usize)).nbBits) - tableSize) as u16;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildDTable_wksp(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    fse_build_dtable_internal(dt, normalizedCounter, maxSymbolValue, tableLog, workSpace, wkspSize)
}

unsafe fn fse_decompress_using_dtable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: u32,
) -> usize {
    let ostart = dst as *mut u8;
    let mut op = ostart;
    let omax = op.add(maxDstSize);
    let olimit = omax.sub(3);

    let mut bitD: BIT_DStream_t = core::mem::zeroed();
    let mut state1: FSE_DState_t = core::mem::zeroed();
    let mut state2: FSE_DState_t = core::mem::zeroed();

    let e = bit_init_dstream(&mut bitD, cSrc, cSrcSize);
    if err_is_error(e) != 0 {
        return e;
    }

    fse_init_dstate(&mut state1, &mut bitD, dt);
    fse_init_dstate(&mut state2, &mut bitD, dt);

    macro_rules! getsym {
        ($s:expr) => {
            if fast != 0 {
                fse_decode_symbol_fast($s, &mut bitD)
            } else {
                fse_decode_symbol($s, &mut bitD)
            }
        };
    }

    if bit_reload_dstream(&mut bitD) == BIT_DStream_overflow {
        return error(code::CORRUPTION_DETECTED);
    }

    // FSE_MAX_TABLELOG*2+7 = 31 <= 64, so those static reloads happen.
    while (bit_reload_dstream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit) {
        *op.add(0) = getsym!(&mut state1);
        // FSE_MAX_TABLELOG*2+7 <= 64
        *op.add(1) = getsym!(&mut state2);
        // FSE_MAX_TABLELOG*4+7 = 55 <= 64 is false? 12*4+7=55 <=64 true
        *op.add(2) = getsym!(&mut state1);
        *op.add(3) = getsym!(&mut state2);
        op = op.add(4);
    }

    loop {
        if op > omax.sub(2) {
            return error(code::DSTSIZE_TOOSMALL);
        }
        *op = getsym!(&mut state1);
        op = op.add(1);
        if bit_reload_dstream(&mut bitD) == BIT_DStream_overflow {
            *op = getsym!(&mut state2);
            op = op.add(1);
            break;
        }
        if op > omax.sub(2) {
            return error(code::DSTSIZE_TOOSMALL);
        }
        *op = getsym!(&mut state2);
        op = op.add(1);
        if bit_reload_dstream(&mut bitD) == BIT_DStream_overflow {
            *op = getsym!(&mut state1);
            op = op.add(1);
            break;
        }
    }
    (op as usize) - (ostart as usize)
}

#[repr(C)]
struct FSE_DecompressWksp {
    ncount: [i16; FSE_MAX_SYMBOL_VALUE as usize + 1],
}

unsafe fn fse_decompress_wksp_body(
    dst: *mut c_void,
    dstCapacity: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    maxLog: u32,
    mut workSpace: *mut c_void,
    mut wkspSize: usize,
    bmi2: i32,
) -> usize {
    let istart = cSrc as *const u8;
    let mut ip = istart;
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE;
    let wksp = workSpace as *mut FSE_DecompressWksp;
    let dtablePos = core::mem::size_of::<FSE_DecompressWksp>() / core::mem::size_of::<FSE_DTable>();
    let dtable = (workSpace as *mut FSE_DTable).add(dtablePos);

    if wkspSize < core::mem::size_of::<FSE_DecompressWksp>() {
        return error(code::GENERIC);
    }

    {
        let NCountLength = FSE_readNCount_bmi2(
            (*wksp).ncount.as_mut_ptr(),
            &mut maxSymbolValue,
            &mut tableLog,
            istart as *const c_void,
            cSrcSize,
            bmi2,
        );
        if err_is_error(NCountLength) != 0 {
            return NCountLength;
        }
        if tableLog > maxLog {
            return error(code::TABLELOG_TOOLARGE);
        }
        ip = ip.add(NCountLength);
        cSrcSize -= NCountLength;
    }

    if fse_decompress_wksp_size(tableLog, maxSymbolValue) > wkspSize {
        return error(code::TABLELOG_TOOLARGE);
    }
    workSpace = (workSpace as *mut u8)
        .add(core::mem::size_of::<FSE_DecompressWksp>() + fse_dtable_size(tableLog))
        as *mut c_void;
    wkspSize -= core::mem::size_of::<FSE_DecompressWksp>() + fse_dtable_size(tableLog);

    let e = fse_build_dtable_internal(
        dtable,
        (*wksp).ncount.as_ptr(),
        maxSymbolValue,
        tableLog,
        workSpace,
        wkspSize,
    );
    if err_is_error(e) != 0 {
        return e;
    }

    let DTableH = dtable as *const FSE_DTableHeader;
    let fastMode = (*DTableH).fastMode;
    if fastMode != 0 {
        fse_decompress_using_dtable_generic(dst, dstCapacity, ip as *const c_void, cSrcSize, dtable, 1)
    } else {
        fse_decompress_using_dtable_generic(dst, dstCapacity, ip as *const c_void, cSrcSize, dtable, 0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_decompress_wksp_bmi2(
    dst: *mut c_void,
    dstCapacity: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    maxLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
    bmi2: i32,
) -> usize {
    fse_decompress_wksp_body(dst, dstCapacity, cSrc, cSrcSize, maxLog, workSpace, wkspSize, bmi2)
}

// ===================================================================
// entropy_common.c — FSE_versionNumber, isError/getErrorName, readNCount
// ===================================================================
#[unsafe(no_mangle)]
pub extern "C" fn FSE_versionNumber() -> u32 {
    super::zstd_common::FSE_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_isError(code: usize) -> u32 {
    err_is_error(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_getErrorName(code: usize) -> *const core::ffi::c_char {
    super::error::err_get_error_name(code)
}

use super::bits::count_trailing_zeros32;

unsafe fn fse_read_ncount_body(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const u8;
    let iend = istart.add(hbSize);
    let mut ip = istart;
    let mut nbBits: i32;
    let mut remaining: i32;
    let mut threshold: i32;
    let mut bitStream: u32;
    let mut bitCount: i32;
    let mut charnum: u32 = 0;
    let maxSV1 = *maxSVPtr + 1;
    let mut previous0 = false;

    if hbSize < 8 {
        let mut buffer = [0u8; 8];
        core::ptr::copy_nonoverlapping(headerBuffer as *const u8, buffer.as_mut_ptr(), hbSize);
        let countSize = FSE_readNCount(
            normalizedCounter,
            maxSVPtr,
            tableLogPtr,
            buffer.as_ptr() as *const c_void,
            8,
        );
        if err_is_error(countSize) != 0 {
            return countSize;
        }
        if countSize > hbSize {
            return error(code::CORRUPTION_DETECTED);
        }
        return countSize;
    }

    core::ptr::write_bytes(normalizedCounter, 0, (*maxSVPtr + 1) as usize);
    bitStream = mem_read_le32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) + FSE_MIN_TABLELOG) as i32;
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as i32 {
        return error(code::TABLELOG_TOOLARGE);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as u32;
    remaining = (1 << nbBits) + 1;
    threshold = 1 << nbBits;
    nbBits += 1;

    loop {
        if previous0 {
            let mut repeats = (count_trailing_zeros32(!bitStream | 0x80000000) >> 1) as i32;
            while repeats >= 12 {
                charnum += 3 * 12;
                if ip <= iend.sub(7) {
                    ip = ip.add(3);
                } else {
                    bitCount -= (8 * (iend as isize - 7 - ip as isize)) as i32;
                    bitCount &= 31;
                    ip = iend.sub(4);
                }
                bitStream = mem_read_le32(ip as *const c_void) >> bitCount;
                repeats = (count_trailing_zeros32(!bitStream | 0x80000000) >> 1) as i32;
            }
            charnum += 3 * repeats as u32;
            bitStream >>= 2 * repeats;
            bitCount += 2 * repeats;

            charnum += bitStream & 3;
            bitCount += 2;

            if charnum >= maxSV1 {
                break;
            }

            if (ip <= iend.sub(7)) || (ip.add((bitCount >> 3) as usize) <= iend.sub(4)) {
                ip = ip.add((bitCount >> 3) as usize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend as isize - 4 - ip as isize)) as i32;
                bitCount &= 31;
                ip = iend.sub(4);
            }
            bitStream = mem_read_le32(ip as *const c_void) >> bitCount;
        }
        {
            let max = (2 * threshold - 1) - remaining;
            let mut count: i32;
            if (bitStream & (threshold as u32 - 1)) < (max as u32) {
                count = (bitStream & (threshold as u32 - 1)) as i32;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold as u32 - 1)) as i32;
                if count >= threshold {
                    count -= max;
                }
                bitCount += nbBits;
            }
            count -= 1;
            if count >= 0 {
                remaining -= count;
            } else {
                remaining += count;
            }
            *normalizedCounter.add(charnum as usize) = count as i16;
            charnum += 1;
            previous0 = count == 0;

            if remaining < threshold {
                if remaining <= 1 {
                    break;
                }
                nbBits = (highbit32(remaining as u32) + 1) as i32;
                threshold = 1 << (nbBits - 1);
            }
            if charnum >= maxSV1 {
                break;
            }

            if (ip <= iend.sub(7)) || (ip.add((bitCount >> 3) as usize) <= iend.sub(4)) {
                ip = ip.add((bitCount >> 3) as usize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend as isize - 4 - ip as isize)) as i32;
                bitCount &= 31;
                ip = iend.sub(4);
            }
            bitStream = mem_read_le32(ip as *const c_void) >> bitCount;
        }
    }
    if remaining != 1 {
        return error(code::CORRUPTION_DETECTED);
    }
    if charnum > maxSV1 {
        return error(code::MAXSYMBOLVALUE_TOOSMALL);
    }
    if bitCount > 32 {
        return error(code::CORRUPTION_DETECTED);
    }
    *maxSVPtr = charnum - 1;
    ip = ip.add(((bitCount + 7) >> 3) as usize);
    (ip as usize) - (istart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount_bmi2(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: usize,
    _bmi2: i32,
) -> usize {
    fse_read_ncount_body(normalizedCounter, maxSVPtr, tableLogPtr, headerBuffer, hbSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    rBuffer: *const c_void,
    rBuffSize: usize,
) -> usize {
    FSE_readNCount_bmi2(normalizedCounter, maxSVPtr, tableLogPtr, rBuffer, rBuffSize, 0)
}
