//! Literal translation of `c_src/src/legacy/zstd_v01.c` (+ zstd_v01.h).
//!
//! Self-contained: bundles its own FSE / Huff0 / bitstream decoders. All
//! internal functions are `pub unsafe fn` (no `#[no_mangle]`); only the nine
//! `ZSTDv01_*` public entry points are exported.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_parens)]

use core::ffi::c_void;

use crate::common::error_private::{ERR_isError, ERROR};
use crate::common::error_private::{
    ZSTD_error_GENERIC, ZSTD_error_corruption_detected, ZSTD_error_dstSize_tooSmall,
    ZSTD_error_prefix_unknown, ZSTD_error_srcSize_wrong,
};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memset(d: *mut c_void, c: i32, n: usize) -> *mut c_void;
}

// ==== Basic types ====
type BYTE = u8;
type U16 = u16;
type S16 = i16;
type U32 = u32;
type S32 = i32;
type U64 = u64;
type S64 = i64;

// ==== FSE error codes (from THIS file's enum) ====
const FSE_OK_NoError: u32 = 0;
const FSE_ERROR_GENERIC: u32 = 1;
const FSE_ERROR_tableLog_tooLarge: u32 = 2;
const FSE_ERROR_maxSymbolValue_tooLarge: u32 = 3;
const FSE_ERROR_maxSymbolValue_tooSmall: u32 = 4;
const FSE_ERROR_dstSize_tooSmall: u32 = 5;
const FSE_ERROR_srcSize_wrong: u32 = 6;
const FSE_ERROR_corruptionDetected: u32 = 7;
const FSE_ERROR_maxCode: u32 = 8;

#[inline(always)]
fn FSE_ERR(code: u32) -> usize {
    (0i64 - code as i64) as usize
}

// ==== FSE tuning parameters ====
const FSE_MAX_MEMORY_USAGE: u32 = 14;
const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;

const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

fn FSE_DTABLE_SIZE_U32(maxTableLog: u32) -> u32 {
    1 + (1u32 << maxTableLog)
}

type FSE_DTable = U32;

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_decode_t {
    newState: u16,
    symbol: u8,
    nbBits: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DTableHeader {
    tableLog: U16,
    fastMode: U16,
}

#[repr(C)]
struct FSE_DStream_t {
    bitContainer: usize,
    bitsConsumed: u32,
    ptr: *const i8,
    start: *const i8,
}

#[repr(C)]
struct FSE_DState_t {
    state: usize,
    table: *const c_void,
}

const FSE_DStream_unfinished: u32 = 0;
const FSE_DStream_endOfBuffer: u32 = 1;
const FSE_DStream_completed: u32 = 2;
const FSE_DStream_tooFar: u32 = 3;

// ==== Memory I/O ====
#[inline(always)]
unsafe fn FSE_32bits() -> u32 {
    (core::mem::size_of::<*const c_void>() == 4) as u32
}

#[inline(always)]
unsafe fn FSE_read16(memPtr: *const c_void) -> U16 {
    (memPtr as *const U16).read_unaligned()
}
#[inline(always)]
unsafe fn FSE_read32(memPtr: *const c_void) -> U32 {
    (memPtr as *const U32).read_unaligned()
}
#[inline(always)]
unsafe fn FSE_read64(memPtr: *const c_void) -> U64 {
    (memPtr as *const U64).read_unaligned()
}
#[inline(always)]
unsafe fn FSE_readLE16(memPtr: *const c_void) -> U16 {
    U16::from_le(FSE_read16(memPtr))
}
#[inline(always)]
unsafe fn FSE_readLE32(memPtr: *const c_void) -> U32 {
    U32::from_le(FSE_read32(memPtr))
}
#[inline(always)]
unsafe fn FSE_readLE64(memPtr: *const c_void) -> U64 {
    U64::from_le(FSE_read64(memPtr))
}
#[inline(always)]
unsafe fn FSE_readLEST(memPtr: *const c_void) -> usize {
    if FSE_32bits() != 0 {
        FSE_readLE32(memPtr) as usize
    } else {
        FSE_readLE64(memPtr) as usize
    }
}

#[inline(always)]
pub unsafe fn FSE_highbit32(val: U32) -> u32 {
    // __builtin_clz(val) ^ 31 ; val is assumed non-zero as in C usage
    (val.leading_zeros()) ^ 31
}

#[inline(always)]
unsafe fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

// ==== FSE_buildDTable ====
pub unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let tableDecode = (ptr as *mut FSE_decode_t).add(1);
    let tableSize: U32 = 1u32 << tableLog;
    let tableMask: U32 = tableSize - 1;
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize - 1;
    let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return FSE_ERR(FSE_ERROR_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return FSE_ERR(FSE_ERROR_tableLog_tooLarge);
    }

    (*DTableH).tableLog = tableLog as U16;
    s = 0;
    while s <= maxSymbolValue {
        let nc = *normalizedCounter.add(s as usize);
        if nc == -1 {
            (*tableDecode.add(highThreshold as usize)).symbol = s as BYTE;
            highThreshold = highThreshold.wrapping_sub(1);
            symbolNext[s as usize] = 1;
        } else {
            if nc >= largeLimit {
                noLarge = 0;
            }
            symbolNext[s as usize] = nc as U16;
        }
        s += 1;
    }

    /* Spread symbols */
    s = 0;
    while s <= maxSymbolValue {
        let mut i: i32 = 0;
        while i < *normalizedCounter.add(s as usize) as i32 {
            (*tableDecode.add(position as usize)).symbol = s as BYTE;
            position = (position + step) & tableMask;
            while position > highThreshold {
                position = (position + step) & tableMask;
            }
            i += 1;
        }
        s += 1;
    }

    if position != 0 {
        return FSE_ERR(FSE_ERROR_GENERIC);
    }

    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.add(i as usize)).nbBits =
                (tableLog - FSE_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(i as usize)).newState =
                (((nextState as U32) << (*tableDecode.add(i as usize)).nbBits).wrapping_sub(tableSize))
                    as U16;
            i += 1;
        }
    }

    (*DTableH).fastMode = noLarge as U16;
    0
}

pub unsafe fn FSE_isError(code: usize) -> u32 {
    (code > (FSE_ERR(FSE_ERROR_maxCode))) as u32
}

pub unsafe fn FSE_abs(a: S16) -> S16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

// ==== FSE_readNCount ====
pub unsafe fn FSE_readNCount(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const BYTE;
    let iend = istart.add(hbSize);
    let mut ip = istart;
    let mut nbBits: i32;
    let mut remaining: i32;
    let mut threshold: i32;
    let mut bitStream: U32;
    let mut bitCount: i32;
    let mut charnum: u32 = 0;
    let mut previous0: i32 = 0;

    if hbSize < 4 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    bitStream = FSE_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) + FSE_MIN_TABLELOG) as i32;
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as i32 {
        return FSE_ERR(FSE_ERROR_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as u32;
    remaining = (1i32 << nbBits) + 1;
    threshold = 1i32 << nbBits;
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0: u32 = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 += 24;
                if ip < iend.offset(-5) {
                    ip = ip.add(2);
                    bitStream = FSE_readLE32(ip as *const c_void) >> bitCount;
                } else {
                    bitStream >>= 16;
                    bitCount += 16;
                }
            }
            while (bitStream & 3) == 3 {
                n0 += 3;
                bitStream >>= 2;
                bitCount += 2;
            }
            n0 += bitStream & 3;
            bitCount += 2;
            if n0 > *maxSVPtr {
                return FSE_ERR(FSE_ERROR_maxSymbolValue_tooSmall);
            }
            while charnum < n0 {
                *normalizedCounter.add(charnum as usize) = 0;
                charnum += 1;
            }
            if (ip <= iend.offset(-7))
                || (ip.offset((bitCount >> 3) as isize) <= iend.offset(-4))
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
                bitStream = FSE_readLE32(ip as *const c_void) >> bitCount;
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: S16 = ((2 * threshold - 1) - remaining) as S16;
            let mut count: S16;

            if (bitStream & (threshold - 1) as U32) < (max as U32) {
                count = (bitStream & (threshold - 1) as U32) as S16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold - 1) as U32) as S16;
                if count >= threshold as S16 {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1;
            remaining -= FSE_abs(count) as i32;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as i32;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            {
                if (ip <= iend.offset(-7))
                    || (ip.offset((bitCount >> 3) as isize) <= iend.offset(-4))
                {
                    ip = ip.offset((bitCount >> 3) as isize);
                    bitCount &= 7;
                } else {
                    bitCount -= (8 * (iend.offset(-4).offset_from(ip))) as i32;
                    ip = iend.offset(-4);
                }
                bitStream = FSE_readLE32(ip as *const c_void) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return FSE_ERR(FSE_ERROR_GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    if (ip.offset_from(istart) as usize) > hbSize {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    ip.offset_from(istart) as usize
}

// ==== FSE decompression (byte symbols) ====
pub unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let cell = (ptr as *mut FSE_decode_t).add(1);

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

pub unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: u32) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let dinfo = (ptr as *mut FSE_decode_t).add(1);
    let tableSize: u32 = 1u32 << nbBits;
    let tableMask: u32 = tableSize - 1;
    let maxSymbolValue: u32 = tableMask;
    let mut s: u32;

    if nbBits < 1 {
        return FSE_ERR(FSE_ERROR_GENERIC);
    }

    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    s = 0;
    while s <= maxSymbolValue {
        (*dinfo.add(s as usize)).newState = 0;
        (*dinfo.add(s as usize)).symbol = s as BYTE;
        (*dinfo.add(s as usize)).nbBits = nbBits as BYTE;
        s += 1;
    }

    0
}

pub unsafe fn FSE_initDStream(
    bitD: *mut FSE_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }

    let szst = core::mem::size_of::<usize>();
    if srcSize >= szst {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const i8;
        (*bitD).ptr = (srcBuffer as *const i8).add(srcSize - szst);
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
        if contain32 == 0 {
            return FSE_ERR(FSE_ERROR_GENERIC);
        }
        (*bitD).bitsConsumed = 8 - FSE_highbit32(contain32);
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const i8;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let start = (*bitD).start as *const BYTE;
        // switch with fallthrough
        if srcSize == 7 {
            (*bitD).bitContainer +=
                (*start.add(6) as usize) << (szst * 8 - 16);
        }
        if srcSize >= 6 {
            (*bitD).bitContainer +=
                (*start.add(5) as usize) << (szst * 8 - 24);
        }
        if srcSize >= 5 {
            (*bitD).bitContainer +=
                (*start.add(4) as usize) << (szst * 8 - 32);
        }
        if srcSize >= 4 {
            (*bitD).bitContainer += (*start.add(3) as usize) << 24;
        }
        if srcSize >= 3 {
            (*bitD).bitContainer += (*start.add(2) as usize) << 16;
        }
        if srcSize >= 2 {
            (*bitD).bitContainer += (*start.add(1) as usize) << 8;
        }
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
        if contain32 == 0 {
            return FSE_ERR(FSE_ERROR_GENERIC);
        }
        (*bitD).bitsConsumed = 8 - FSE_highbit32(contain32);
        (*bitD).bitsConsumed += ((szst - srcSize) * 8) as u32;
    }

    srcSize
}

pub unsafe fn FSE_lookBits(bitD: *const FSE_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask - nbBits) & bitMask)
}

pub unsafe fn FSE_lookBitsFast(bitD: *const FSE_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1) - nbBits) & bitMask)
}

pub unsafe fn FSE_skipBits(bitD: *mut FSE_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed += nbBits;
}

pub unsafe fn FSE_readBits(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let value = FSE_lookBits(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    value
}

pub unsafe fn FSE_readBitsFast(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let value = FSE_lookBitsFast(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    value
}

pub unsafe fn FSE_reloadDStream(bitD: *mut FSE_DStream_t) -> u32 {
    let szbc = core::mem::size_of::<usize>();
    if (*bitD).bitsConsumed > (szbc * 8) as u32 {
        return FSE_DStream_tooFar;
    }

    if (*bitD).ptr >= (*bitD).start.add(szbc) {
        (*bitD).ptr = (*bitD).ptr.offset(-(((*bitD).bitsConsumed >> 3) as isize));
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        return FSE_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (szbc * 8) as u32 {
            return FSE_DStream_endOfBuffer;
        }
        return FSE_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: U32 = FSE_DStream_unfinished;
        if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32;
            result = FSE_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        return result;
    }
}

pub unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut FSE_DStream_t,
    dt: *const FSE_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSE_DTableHeader;
    (*DStatePtr).state = FSE_readBits(bitD, (*DTableH).tableLog as U32);
    FSE_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

pub unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut FSE_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = FSE_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut FSE_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = FSE_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSE_endOfDStream(bitD: *const FSE_DStream_t) -> u32 {
    (((*bitD).ptr == (*bitD).start)
        && ((*bitD).bitsConsumed == (core::mem::size_of::<usize>() * 8) as u32)) as u32
}

pub unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

pub unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: u32,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.add(maxDstSize);
    let olimit = omax.offset(-3);

    let mut bitD = FSE_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1 = FSE_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2 = FSE_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let errorCode: usize;

    errorCode = FSE_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    macro_rules! GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSE_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSE_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    let szbits = core::mem::size_of::<usize>() * 8;

    /* 4 symbols per loop */
    while (FSE_reloadDStream(&mut bitD) == FSE_DStream_unfinished) && (op < olimit) {
        *op.add(0) = GETSYMBOL!(&mut state1);

        if (FSE_MAX_TABLELOG * 2 + 7) as usize > szbits {
            FSE_reloadDStream(&mut bitD);
        }

        *op.add(1) = GETSYMBOL!(&mut state2);

        if (FSE_MAX_TABLELOG * 4 + 7) as usize > szbits {
            if FSE_reloadDStream(&mut bitD) > FSE_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = GETSYMBOL!(&mut state1);

        if (FSE_MAX_TABLELOG * 2 + 7) as usize > szbits {
            FSE_reloadDStream(&mut bitD);
        }

        *op.add(3) = GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    /* tail */
    loop {
        if (FSE_reloadDStream(&mut bitD) > FSE_DStream_completed)
            || (op == omax)
            || (FSE_endOfDStream(&bitD) != 0
                && (fast != 0 || FSE_endOfDState(&state1) != 0))
        {
            break;
        }

        *op = GETSYMBOL!(&mut state1);
        op = op.add(1);

        if (FSE_reloadDStream(&mut bitD) > FSE_DStream_completed)
            || (op == omax)
            || (FSE_endOfDStream(&bitD) != 0
                && (fast != 0 || FSE_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = GETSYMBOL!(&mut state2);
        op = op.add(1);
    }

    if FSE_endOfDStream(&bitD) != 0
        && FSE_endOfDState(&state1) != 0
        && FSE_endOfDState(&state2) != 0
    {
        return op.offset_from(ostart) as usize;
    }

    if op == omax {
        return FSE_ERR(FSE_ERROR_dstSize_tooSmall);
    }

    FSE_ERR(FSE_ERROR_corruptionDetected)
}

pub unsafe fn FSE_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
) -> usize {
    let mut DTableH = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    memcpy(
        &mut DTableH as *mut FSE_DTableHeader as *mut c_void,
        dt as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );

    if DTableH.fastMode != 0 {
        return FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

pub unsafe fn FSE_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [S16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [U32; (1 + (1usize << FSE_MAX_TABLELOG))] =
        [0; (1 + (1usize << FSE_MAX_TABLELOG))];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE;
    let mut errorCode: usize;
    let mut cSrcSize = cSrcSize;

    if cSrcSize < 2 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }

    errorCode = FSE_readNCount(
        counting.as_mut_ptr(),
        &mut maxSymbolValue,
        &mut tableLog,
        istart as *const c_void,
        cSrcSize,
    );
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSE_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

// ==== Huff0 : Huffman block decompression ====
const HUF_MAX_SYMBOL_VALUE: u32 = 255;
const HUF_DEFAULT_TABLELOG: u32 = 12;
const HUF_MAX_TABLELOG: u32 = 12;
const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16;

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DElt {
    byte: BYTE,
    nbBits: BYTE,
}

pub unsafe fn HUF_readDTable(DTable: *mut U16, src: *const c_void, srcSize: usize) -> usize {
    let mut huffWeight: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut weightTotal: U32;
    let mut maxBits: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: usize;
    let mut oSize: usize;
    let mut n: U32;
    let mut nextRankStart: U32;
    let ptr = DTable.add(1) as *mut c_void;
    let dt = ptr as *mut HUF_DElt;

    if srcSize == 0 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        if iSize >= 242 {
            static L: [i32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = L[iSize - 242] as usize;
            memset(
                huffWeight.as_mut_ptr() as *mut c_void,
                1,
                core::mem::size_of_val(&huffWeight),
            );
            iSize = 0;
        } else {
            oSize = iSize - 127;
            iSize = (oSize + 1) / 2;
            if iSize + 1 > srcSize {
                return FSE_ERR(FSE_ERROR_srcSize_wrong);
            }
            ip = ip.add(1);
            n = 0;
            while (n as usize) < oSize {
                huffWeight[n as usize] = *ip.add((n / 2) as usize) >> 4;
                huffWeight[(n + 1) as usize] = *ip.add((n / 2) as usize) & 15;
                n += 2;
            }
        }
    } else {
        if iSize + 1 > srcSize {
            return FSE_ERR(FSE_ERROR_srcSize_wrong);
        }
        oSize = FSE_decompress(
            huffWeight.as_mut_ptr() as *mut c_void,
            HUF_MAX_SYMBOL_VALUE as usize,
            ip.add(1) as *const c_void,
            iSize,
        );
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankVal.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of_val(&rankVal),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if huffWeight[n as usize] as U32 >= HUF_ABSOLUTEMAX_TABLELOG {
            return FSE_ERR(FSE_ERROR_corruptionDetected);
        }
        rankVal[huffWeight[n as usize] as usize] += 1;
        weightTotal += (1u32 << huffWeight[n as usize]) >> 1;
        n += 1;
    }
    if weightTotal == 0 {
        return FSE_ERR(FSE_ERROR_corruptionDetected);
    }

    /* get last non-null symbol weight */
    maxBits = FSE_highbit32(weightTotal) + 1;
    if maxBits > *DTable.add(0) as U32 {
        return FSE_ERR(FSE_ERROR_tableLog_tooLarge);
    }
    *DTable.add(0) = maxBits as U16;
    {
        let total: U32 = 1u32 << maxBits;
        let rest: U32 = total - weightTotal;
        let verif: U32 = 1u32 << FSE_highbit32(rest);
        let lastWeight: U32 = FSE_highbit32(rest) + 1;
        if verif != rest {
            return FSE_ERR(FSE_ERROR_corruptionDetected);
        }
        huffWeight[oSize] = lastWeight as BYTE;
        rankVal[lastWeight as usize] += 1;
    }

    /* check tree construction validity */
    if (rankVal[1] < 2) || (rankVal[1] & 1) != 0 {
        return FSE_ERR(FSE_ERROR_corruptionDetected);
    }

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= maxBits {
        let current = nextRankStart;
        nextRankStart += rankVal[n as usize] << (n - 1);
        rankVal[n as usize] = current;
        n += 1;
    }

    /* fill DTable */
    n = 0;
    while (n as usize) <= oSize {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32 << w) >> 1;
        let mut i: U32;
        let D = HUF_DElt {
            byte: n as BYTE,
            nbBits: (maxBits + 1 - w) as BYTE,
        };
        i = rankVal[w as usize];
        while i < rankVal[w as usize] + length {
            *dt.add(i as usize) = D;
            i += 1;
        }
        rankVal[w as usize] += length;
        n += 1;
    }

    iSize + 1
}

pub unsafe fn HUF_decodeSymbol(
    Dstream: *mut FSE_DStream_t,
    dt: *const HUF_DElt,
    dtLog: U32,
) -> BYTE {
    let val = FSE_lookBitsFast(Dstream, dtLog);
    let c = (*dt.add(val)).byte;
    FSE_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

pub unsafe fn HUF_decompress_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    if cSrcSize < 6 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    {
        let ostart = dst as *mut BYTE;
        let mut op = ostart;
        let omax = op.add(maxDstSize);
        let olimit = if maxDstSize < 15 { op } else { omax.offset(-15) };

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DElt).add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;
        let mut reloadStatus: U32;

        let jumpTable = cSrc as *const U16;
        let length1 = FSE_readLE16(jumpTable as *const c_void) as usize;
        let length2 = FSE_readLE16(jumpTable.add(1) as *const c_void) as usize;
        let length3 = FSE_readLE16(jumpTable.add(2) as *const c_void) as usize;
        let length4 = cSrcSize
            .wrapping_sub(6)
            .wrapping_sub(length1)
            .wrapping_sub(length2)
            .wrapping_sub(length3);
        let start1 = (cSrc as *const i8).add(6);
        let start2 = start1.add(length1);
        let start3 = start2.add(length2);
        let start4 = start3.add(length3);
        let mut bitD1 = FSE_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = FSE_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD3 = FSE_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD4 = FSE_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };

        if length1 + length2 + length3 + 6 >= cSrcSize {
            return FSE_ERR(FSE_ERROR_srcSize_wrong);
        }

        errorCode = FSE_initDStream(&mut bitD1, start1 as *const c_void, length1);
        if FSE_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = FSE_initDStream(&mut bitD2, start2 as *const c_void, length2);
        if FSE_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = FSE_initDStream(&mut bitD3, start3 as *const c_void, length3);
        if FSE_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = FSE_initDStream(&mut bitD4, start4 as *const c_void, length4);
        if FSE_isError(errorCode) != 0 {
            return errorCode;
        }

        reloadStatus = FSE_reloadDStream(&mut bitD2);

        macro_rules! HUF_DECODE_SYMBOL_0 {
            ($n:expr, $ds:expr) => {
                *op.add($n) = HUF_decodeSymbol(&mut $ds, dt, dtLog);
            };
        }
        macro_rules! HUF_DECODE_SYMBOL_1 {
            ($n:expr, $ds:expr) => {
                *op.add($n) = HUF_decodeSymbol(&mut $ds, dt, dtLog);
                if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                    FSE_reloadDStream(&mut $ds);
                }
            };
        }
        macro_rules! HUF_DECODE_SYMBOL_2 {
            ($n:expr, $ds:expr) => {
                *op.add($n) = HUF_decodeSymbol(&mut $ds, dt, dtLog);
                if FSE_32bits() != 0 {
                    FSE_reloadDStream(&mut $ds);
                }
            };
        }

        /* 16 symbols per loop */
        while (reloadStatus < FSE_DStream_completed) && (op < olimit) {
            HUF_DECODE_SYMBOL_1!(0, bitD1);
            HUF_DECODE_SYMBOL_1!(1, bitD2);
            HUF_DECODE_SYMBOL_1!(2, bitD3);
            HUF_DECODE_SYMBOL_1!(3, bitD4);
            HUF_DECODE_SYMBOL_2!(4, bitD1);
            HUF_DECODE_SYMBOL_2!(5, bitD2);
            HUF_DECODE_SYMBOL_2!(6, bitD3);
            HUF_DECODE_SYMBOL_2!(7, bitD4);
            HUF_DECODE_SYMBOL_1!(8, bitD1);
            HUF_DECODE_SYMBOL_1!(9, bitD2);
            HUF_DECODE_SYMBOL_1!(10, bitD3);
            HUF_DECODE_SYMBOL_1!(11, bitD4);
            HUF_DECODE_SYMBOL_0!(12, bitD1);
            HUF_DECODE_SYMBOL_0!(13, bitD2);
            HUF_DECODE_SYMBOL_0!(14, bitD3);
            HUF_DECODE_SYMBOL_0!(15, bitD4);

            op = op.add(16);
            reloadStatus = FSE_reloadDStream(&mut bitD2)
                | FSE_reloadDStream(&mut bitD3)
                | FSE_reloadDStream(&mut bitD4);
            FSE_reloadDStream(&mut bitD1);
        }

        if reloadStatus != FSE_DStream_completed {
            return FSE_ERR(FSE_ERROR_corruptionDetected);
        }

        /* tail */
        {
            let mut bitTail = FSE_DStream_t {
                bitContainer: bitD1.bitContainer,
                bitsConsumed: bitD1.bitsConsumed,
                ptr: bitD1.ptr,
                start: start1,
            };
            while (FSE_reloadDStream(&mut bitTail) < FSE_DStream_completed) && (op < omax) {
                HUF_DECODE_SYMBOL_0!(0, bitTail);
                op = op.add(1);
            }

            if FSE_endOfDStream(&bitTail) != 0 {
                return op.offset_from(ostart) as usize;
            }
        }

        if op == omax {
            return FSE_ERR(FSE_ERROR_dstSize_tooSmall);
        }

        FSE_ERR(FSE_ERROR_corruptionDetected)
    }
}

pub unsafe fn HUF_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    // HUF_CREATE_STATIC_DTABLE(DTable, HUF_MAX_TABLELOG)
    // unsigned short DTable[HUF_DTABLE_SIZE_U16(maxTableLog)] = { maxTableLog };
    let mut DTable: [U16; (1 + (1usize << HUF_MAX_TABLELOG))] =
        [0; (1 + (1usize << HUF_MAX_TABLELOG))];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut errorCode: usize;
    let mut cSrcSize = cSrcSize;

    errorCode = HUF_readDTable(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUF_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// ==== ZSTD layer ====
const ZSTD_magicNumber: U32 = 0xFD2FB51E;

const BLOCKSIZE: usize = 128 * 1024;
const WORKPLACESIZE: usize = BLOCKSIZE * 3;
const MINMATCH: usize = 4;
const MLbits: u32 = 7;
const LLbits: u32 = 6;
const Offbits: u32 = 5;
const MaxML: u32 = (1u32 << MLbits) - 1;
const MaxLL: u32 = (1u32 << LLbits) - 1;
const MaxOff: u32 = (1u32 << Offbits) - 1;
const LitFSELog: u32 = 11;
const MLFSELog: u32 = 10;
const LLFSELog: u32 = 10;
const OffFSELog: u32 = 9;
const MaxSeq: u32 = if MaxLL < MaxML { MaxML } else { MaxLL };

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

const ZSTD_blockHeaderSize: usize = 3;
const ZSTD_frameHeaderSize: usize = 4;

// blockType_t : bt_compressed, bt_raw, bt_rle, bt_end
type blockType_t = u32;
const bt_compressed: blockType_t = 0;
const bt_raw: blockType_t = 1;
const bt_rle: blockType_t = 2;
const bt_end: blockType_t = 3;

#[repr(C)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

#[inline(always)]
unsafe fn ZSTD_read16(p: *const c_void) -> U16 {
    (p as *const U16).read_unaligned()
}

#[inline(always)]
unsafe fn ZSTD_32bits() -> u32 {
    (core::mem::size_of::<*const c_void>() == 4) as u32
}

#[inline(always)]
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

#[inline(always)]
unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.offset(length);
    while op < oend {
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
    }
}

#[inline(always)]
unsafe fn ZSTD_readLE16(memPtr: *const c_void) -> U16 {
    U16::from_le(ZSTD_read16(memPtr))
}

#[inline(always)]
unsafe fn ZSTD_readLE24(memPtr: *const c_void) -> U32 {
    ZSTD_readLE16(memPtr) as U32 + ((*(memPtr as *const BYTE).add(2) as U32) << 16)
}

#[inline(always)]
unsafe fn ZSTD_readBE32(memPtr: *const c_void) -> U32 {
    let p = memPtr as *const BYTE;
    ((*p.add(0) as U32) << 24)
        + ((*p.add(1) as U32) << 16)
        + ((*p.add(2) as U32) << 8)
        + ((*p.add(3) as U32) << 0)
}

/* published entry point */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_isError(code: usize) -> u32 {
    ERR_isError(code)
}

pub unsafe fn ZSTDv01_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let inp = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    headerFlags = *inp;
    cSize = *inp.add(2) as U32 + ((*inp.add(1) as U32) << 8) + (((*inp.add(0) as U32) & 7) << 16);

    (*bpPtr).blockType = (headerFlags >> 6) as blockType_t;
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as usize
}

pub unsafe fn ZSTD_copyUncompressedBlock(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

pub unsafe fn ZSTD_decompressLiterals(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut op = dst as *mut BYTE;
    let oend = op.add(maxDstSize);
    let ip = src as *const BYTE;
    let errorCode: usize;
    let mut litSize: usize;

    if srcSize <= 3 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    litSize = *ip.add(1) as usize + ((*ip.add(0) as usize) << 8);
    litSize += (((*ip.offset(-3) as usize) >> 3) & 7) << 16;
    op = oend.offset(-(litSize as isize));

    if litSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    errorCode = HUF_decompress(
        op as *mut c_void,
        litSize,
        ip.add(2) as *const c_void,
        srcSize - 2,
    );
    if FSE_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
    litSize
}

pub unsafe fn ZSTDv01_decodeLiteralsBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    litStart: *mut *const BYTE,
    litSize: *mut usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;
    let mut ip = istart;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(maxDstSize);
    let mut litbp = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    let litcSize = ZSTDv01_getcBlockSize(src, srcSize, &mut litbp);
    if ZSTDv01_isError(litcSize) != 0 {
        return litcSize;
    }
    if litcSize > srcSize - ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(ZSTD_blockHeaderSize);

    match litbp.blockType {
        x if x == bt_raw => {
            *litStart = ip;
            ip = ip.add(litcSize);
            *litSize = litcSize;
        }
        x if x == bt_rle => {
            let rleSize = litbp.origSize as usize;
            if rleSize > maxDstSize {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            if srcSize == 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if rleSize > 0 {
                memset(
                    oend.offset(-(rleSize as isize)) as *mut c_void,
                    *ip as i32,
                    rleSize,
                );
            }
            *litStart = oend.offset(-(rleSize as isize));
            *litSize = rleSize;
            ip = ip.add(1);
        }
        x if x == bt_compressed => {
            let decodedLitSize = ZSTD_decompressLiterals(ctx, dst, maxDstSize, ip as *const c_void, litcSize);
            if ZSTDv01_isError(decodedLitSize) != 0 {
                return decodedLitSize;
            }
            *litStart = oend.offset(-(decodedLitSize as isize));
            *litSize = decodedLitSize;
            ip = ip.add(litcSize);
        }
        _ => {
            // bt_end / default
            return ERROR(ZSTD_error_GENERIC);
        }
    }

    ip.offset_from(istart) as usize
}

pub unsafe fn ZSTDv01_decodeSeqHeaders(
    nbSeq: *mut i32,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut FSE_DTable,
    DTableML: *mut FSE_DTable,
    DTableOffb: *mut FSE_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;
    let mut ip = istart;
    let iend = istart.add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: U32 = 0;
    let mut Offlog: U32 = 0;
    let mut MLlog: U32 = 0;
    let dumpsLength: usize;

    if srcSize < 5 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    *nbSeq = ZSTD_readLE16(ip as *const c_void) as i32;
    ip = ip.add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = *ip.add(2) as usize + ((*ip.add(1) as usize) << 8);
        ip = ip.add(3);
    } else {
        dumpsLength = *ip.add(1) as usize + (((*ip.add(0) as usize) & 1) << 8);
        ip = ip.add(2);
    }
    *dumpsPtr = ip;
    ip = ip.add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    if ip > iend.offset(-3) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    {
        let mut norm: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut headerSize: usize;

        // LL
        if LLtype == bt_rle {
            LLlog = 0;
            let v = *ip;
            ip = ip.add(1);
            FSE_buildDTable_rle(DTableLL, v);
        } else if LLtype == bt_raw {
            LLlog = LLbits;
            FSE_buildDTable_raw(DTableLL, LLbits);
        } else {
            let mut max: U32 = MaxLL;
            headerSize = FSE_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut LLlog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
            );
            if FSE_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if LLlog > LLFSELog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(headerSize);
            FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
        }

        // Off
        if Offtype == bt_rle {
            Offlog = 0;
            if ip > iend.offset(-2) {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            let v = *ip;
            ip = ip.add(1);
            FSE_buildDTable_rle(DTableOffb, v);
        } else if Offtype == bt_raw {
            Offlog = Offbits;
            FSE_buildDTable_raw(DTableOffb, Offbits);
        } else {
            let mut max: U32 = MaxOff;
            headerSize = FSE_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut Offlog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
            );
            if FSE_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if Offlog > OffFSELog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(headerSize);
            FSE_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
        }

        // ML
        if MLtype == bt_rle {
            MLlog = 0;
            if ip > iend.offset(-2) {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            let v = *ip;
            ip = ip.add(1);
            FSE_buildDTable_rle(DTableML, v);
        } else if MLtype == bt_raw {
            MLlog = MLbits;
            FSE_buildDTable_raw(DTableML, MLbits);
        } else {
            let mut max: U32 = MaxML;
            headerSize = FSE_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut MLlog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
            );
            if FSE_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if MLlog > MLFSELog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(headerSize);
            FSE_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
        }
    }

    ip.offset_from(istart) as usize
}

#[repr(C)]
struct seq_t {
    litLength: usize,
    offset: usize,
    matchLength: usize,
}

#[repr(C)]
struct seqState_t {
    DStream: FSE_DStream_t,
    stateLL: FSE_DState_t,
    stateOffb: FSE_DState_t,
    stateML: FSE_DState_t,
    prevOffset: usize,
    dumps: *const BYTE,
    dumpsEnd: *const BYTE,
}

pub unsafe fn ZSTD_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let mut litLength: usize;
    let prevOffset: usize;
    let mut offset: usize;
    let mut matchLength: usize;
    let mut dumps = (*seqState).dumps;
    let de = (*seqState).dumpsEnd;

    /* Literal length */
    litLength = FSE_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream) as usize;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    (*seqState).prevOffset = (*seq).offset;
    if litLength == MaxLL as usize {
        let add: U32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.add(1);
            v as U32
        } else {
            0
        };
        if add < 255 {
            litLength += add as usize;
        } else {
            if dumps <= de.offset(-3) {
                litLength = ZSTD_readLE24(dumps as *const c_void) as usize;
                dumps = dumps.add(3);
            }
        }
    }

    /* Offset */
    {
        let offsetCode: U32;
        let mut nbBits: U32;
        offsetCode = FSE_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream) as U32;
        if ZSTD_32bits() != 0 {
            FSE_reloadDStream(&mut (*seqState).DStream);
        }
        nbBits = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0;
        }
        offset = ((1usize) << (nbBits as usize & (core::mem::size_of::<usize>() * 8 - 1)))
            + FSE_readBits(&mut (*seqState).DStream, nbBits);
        if ZSTD_32bits() != 0 {
            FSE_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset;
        }
    }

    /* MatchLength */
    matchLength = FSE_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as usize;
    if matchLength == MaxML as usize {
        let add: U32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.add(1);
            v as U32
        } else {
            0
        };
        if add < 255 {
            matchLength += add as usize;
        } else {
            if dumps <= de.offset(-3) {
                matchLength = ZSTD_readLE24(dumps as *const c_void) as usize;
                dumps = dumps.add(3);
            }
        }
    }
    matchLength += MINMATCH;

    /* save result */
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

pub unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> usize {
    static dec32table: [i32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
    static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
    let ostart = op;
    let oLitEnd = op.add(sequence.litLength);
    let litLength = sequence.litLength;
    let endMatch = op.add(litLength + sequence.matchLength);
    let litEnd = (*litPtr).add(litLength);

    let seqLength = sequence.litLength + sequence.matchLength;

    if seqLength > (oend.offset_from(op) as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit.offset_from(*litPtr) as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if sequence.offset > (oLitEnd.offset_from(base) as U32 as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    if endMatch > oend {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if litEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if sequence.matchLength > ((*litPtr).offset_from(op) as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    /* copy Literals */
    memmove(op as *mut c_void, *litPtr as *const c_void, sequence.litLength);

    op = op.add(litLength);
    *litPtr = litEnd;

    if oend.offset_from(op) < 8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    /* copy Match */
    {
        let overlapRisk: U32 =
            (((litEnd.offset_from(endMatch)) as usize) < 12) as U32;
        let mut match_: *const BYTE = op.offset(-(sequence.offset as isize));
        let mut qutt: usize = 12;
        let mut saved: [U64; 2] = [0; 2];

        if match_ < base {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if sequence.offset > (base as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }

        if overlapRisk != 0 {
            if endMatch.add(qutt) > oend {
                qutt = oend.offset_from(endMatch) as usize;
            }
            memcpy(
                saved.as_mut_ptr() as *mut c_void,
                endMatch as *const c_void,
                qutt,
            );
        }

        if sequence.offset < 8 {
            let dec64 = dec64table[sequence.offset];
            *op.add(0) = *match_.add(0);
            *op.add(1) = *match_.add(1);
            *op.add(2) = *match_.add(2);
            *op.add(3) = *match_.add(3);
            match_ = match_.offset(dec32table[sequence.offset] as isize);
            ZSTD_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
            match_ = match_.offset(-(dec64 as isize));
        } else {
            ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
        }
        op = op.add(8);
        match_ = match_.add(8);

        if endMatch > oend.offset(-((16 - MINMATCH) as isize)) {
            if op < oend.offset(-8) {
                ZSTD_wildcopy(
                    op as *mut c_void,
                    match_ as *const c_void,
                    oend.offset(-8).offset_from(op),
                );
                match_ = match_.offset(oend.offset(-8).offset_from(op));
                op = oend.offset(-8);
            }
            while op < endMatch {
                *op = *match_;
                op = op.add(1);
                match_ = match_.add(1);
            }
        } else {
            ZSTD_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength as isize - 8,
            );
        }

        if overlapRisk != 0 {
            memcpy(
                endMatch as *mut c_void,
                saved.as_ptr() as *const c_void,
                qutt,
            );
        }
    }

    endMatch.offset_from(ostart) as usize
}

#[repr(C)]
struct dctx_t {
    LLTable: [U32; (1 + (1usize << LLFSELog))],
    OffTable: [U32; (1 + (1usize << OffFSELog))],
    MLTable: [U32; (1 + (1usize << MLFSELog))],
    previousDstEnd: *mut c_void,
    base: *mut c_void,
    expected: usize,
    bType: blockType_t,
    phase: U32,
}

pub type ZSTDv01_Dctx = dctx_t;

pub unsafe fn ZSTD_decompressSequences(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    litStart: *const BYTE,
    litSize: usize,
) -> usize {
    let dctx = ctx as *mut dctx_t;
    let mut ip = seqStart as *const BYTE;
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut errorCode: usize;
    let mut dumpsLength: usize = 0;
    let mut litPtr = litStart;
    let litEnd = litStart.add(litSize);
    let mut nbSeq: i32 = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *mut BYTE;

    errorCode = ZSTDv01_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        iend.offset_from(ip) as usize,
    );
    if ZSTDv01_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.add(errorCode);

    {
        let mut sequence = seq_t {
            litLength: 0,
            offset: 0,
            matchLength: 0,
        };
        let mut seqState = seqState_t {
            DStream: FSE_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            },
            stateLL: FSE_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateOffb: FSE_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateML: FSE_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            prevOffset: 0,
            dumps: core::ptr::null(),
            dumpsEnd: core::ptr::null(),
        };

        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.add(dumpsLength);
        seqState.prevOffset = 1;
        errorCode = FSE_initDStream(&mut seqState.DStream, ip as *const c_void, iend.offset_from(ip) as usize);
        if FSE_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSE_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSE_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSE_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (FSE_reloadDStream(&mut seqState.DStream) <= FSE_DStream_completed) && (nbSeq > 0) {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(
                op,
                seq_t {
                    litLength: sequence.litLength,
                    offset: sequence.offset,
                    matchLength: sequence.matchLength,
                },
                &mut litPtr,
                litEnd,
                base,
                oend,
            );
            if ZSTDv01_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
        }

        if FSE_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if nbSeq < 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        {
            let lastLLSize = litEnd.offset_from(litPtr) as usize;
            if op.add(lastLLSize) > oend {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            if lastLLSize > 0 {
                if op != litPtr as *mut BYTE {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.add(lastLLSize);
            }
        }
    }

    op.offset_from(ostart) as usize
}

pub unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let mut litPtr: *const BYTE = core::ptr::null();
    let mut litSize: usize = 0;
    let errorCode: usize;

    errorCode = ZSTDv01_decodeLiteralsBlock(ctx, dst, maxDstSize, &mut litPtr, &mut litSize, src, srcSize);
    if ZSTDv01_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.add(errorCode);
    srcSize -= errorCode;

    ZSTD_decompressSequences(ctx, dst, maxDstSize, ip as *const c_void, srcSize, litPtr, litSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompressDCtx(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut remainingSize = srcSize;
    let magicNumber: U32;
    let mut errorCode: usize = 0;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    magicNumber = ZSTD_readBE32(src);
    if magicNumber != ZSTD_magicNumber {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    ip = ip.add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    loop {
        let blockSize = ZSTDv01_getcBlockSize(ip as *const c_void, iend.offset_from(ip) as usize, &mut blockProperties);
        if ZSTDv01_isError(blockSize) != 0 {
            return blockSize;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if blockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        if blockProperties.blockType == bt_compressed {
            errorCode = ZSTD_decompressBlock(ctx, op as *mut c_void, oend.offset_from(op) as usize, ip as *const c_void, blockSize);
        } else if blockProperties.blockType == bt_raw {
            errorCode = ZSTD_copyUncompressedBlock(op as *mut c_void, oend.offset_from(op) as usize, ip as *const c_void, blockSize);
        } else if blockProperties.blockType == bt_rle {
            return ERROR(ZSTD_error_GENERIC);
        } else if blockProperties.blockType == bt_end {
            if remainingSize != 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
        } else {
            return ERROR(ZSTD_error_GENERIC);
        }
        if blockSize == 0 {
            break;
        }

        if ZSTDv01_isError(errorCode) != 0 {
            return errorCode;
        }
        op = op.add(errorCode);
        ip = ip.add(blockSize);
        remainingSize -= blockSize;
    }

    op.offset_from(ostart) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ctx: dctx_t = core::mem::zeroed();
    ctx.base = dst;
    ZSTDv01_decompressDCtx(&mut ctx as *mut dctx_t as *mut c_void, dst, maxDstSize, src, srcSize)
}

unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut usize,
    dBound: *mut u64,
    ret: usize,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;
    let magicNumber: U32;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    magicNumber = ZSTD_readBE32(src);
    if magicNumber != ZSTD_magicNumber {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
        return;
    }
    ip = ip.add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    loop {
        let blockSize = ZSTDv01_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv01_isError(blockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, blockSize);
            return;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if blockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if blockSize == 0 {
            break;
        }

        ip = ip.add(blockSize);
        remainingSize -= blockSize;
        nbBlocks += 1;
    }

    *cSize = ip.offset_from(src as *const BYTE) as usize;
    *dBound = (nbBlocks * BLOCKSIZE) as u64;
}

// ==== Streaming Decompression API ====
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_resetDCtx(dctx: *mut ZSTDv01_Dctx) -> usize {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0;
    (*dctx).previousDstEnd = core::ptr::null_mut();
    (*dctx).base = core::ptr::null_mut();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_createDCtx() -> *mut ZSTDv01_Dctx {
    let dctx = malloc(core::mem::size_of::<ZSTDv01_Dctx>()) as *mut ZSTDv01_Dctx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDv01_resetDCtx(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_freeDCtx(dctx: *mut ZSTDv01_Dctx) -> usize {
    free(dctx as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_nextSrcSizeToDecompress(dctx: *mut ZSTDv01_Dctx) -> usize {
    (*(dctx as *mut dctx_t)).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompressContinue(
    dctx: *mut ZSTDv01_Dctx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ctx = dctx as *mut dctx_t;

    if srcSize != (*ctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dst != (*ctx).previousDstEnd {
        (*ctx).base = dst;
    }

    /* frame header */
    if (*ctx).phase == 0 {
        let magicNumber = ZSTD_readBE32(src);
        if magicNumber != ZSTD_magicNumber {
            return ERROR(ZSTD_error_prefix_unknown);
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        return 0;
    }

    /* block header */
    if (*ctx).phase == 1 {
        let mut bp = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let blockSize = ZSTDv01_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
        if ZSTDv01_isError(blockSize) != 0 {
            return blockSize;
        }
        if bp.blockType == bt_end {
            (*ctx).expected = 0;
            (*ctx).phase = 0;
        } else {
            (*ctx).expected = blockSize;
            (*ctx).bType = bp.blockType;
            (*ctx).phase = 2;
        }

        return 0;
    }

    /* block content */
    {
        let rSize: usize;
        if (*ctx).bType == bt_compressed {
            rSize = ZSTD_decompressBlock(ctx as *mut c_void, dst, maxDstSize, src, srcSize);
        } else if (*ctx).bType == bt_raw {
            rSize = ZSTD_copyUncompressedBlock(dst, maxDstSize, src, srcSize);
        } else if (*ctx).bType == bt_rle {
            return ERROR(ZSTD_error_GENERIC);
        } else if (*ctx).bType == bt_end {
            rSize = 0;
        } else {
            return ERROR(ZSTD_error_GENERIC);
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTDv01_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = ((dst as *mut i8).add(rSize)) as *mut c_void;
        rSize
    }
}
