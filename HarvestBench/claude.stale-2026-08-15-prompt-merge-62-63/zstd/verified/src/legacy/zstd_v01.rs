//! Translation of legacy/zstd_v01.c — decompression of ZSTD v0.1.x frames.
//! Self-contained: defines its own internal FSE / Huff0 decode code.
//! Target: little-endian 64-bit.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_parens)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::common::allocations::{free, malloc, memcpy, memmove, memset};
use crate::common::error::code as ec;
use crate::common::error::{err_is_error, error as zstd_error};

/* ---- Basic types ---- */
type BYTE = u8;
type U16 = u16;
type S16 = i16;
type U32 = u32;
type S32 = i32;
type U64 = u64;
type S64 = i64;

/* ---- FSE error codes (local enum FSE_errorCodes) ---- */
const FSE_ERROR_GENERIC: usize = 1;
const FSE_ERROR_tableLog_tooLarge: usize = 2;
const FSE_ERROR_maxSymbolValue_tooLarge: usize = 3;
const FSE_ERROR_maxSymbolValue_tooSmall: usize = 4;
const FSE_ERROR_dstSize_tooSmall: usize = 5;
const FSE_ERROR_srcSize_wrong: usize = 6;
const FSE_ERROR_corruptionDetected: usize = 7;
const FSE_ERROR_maxCode: usize = 8;

#[inline]
fn fse_error(c: usize) -> usize {
    0usize.wrapping_sub(c)
}

/* ---- FSE tuning constants ---- */
const FSE_MAX_MEMORY_USAGE: u32 = 14;
const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;

const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2; /* 12 */
const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

/* FSE_DTABLE_SIZE_U32(maxTableLog) = 1 + (1<<maxTableLog) */
const fn fse_dtable_size_u32(max_table_log: u32) -> usize {
    (1 + (1u32 << max_table_log)) as usize
}

/* ---- FSE stream / state / decode structures ---- */
#[repr(C)]
struct FSE_CStream_t {
    bitContainer: usize,
    bitPos: c_int,
    startPtr: *mut c_char,
    ptr: *mut c_char,
    endPtr: *mut c_char,
}

#[repr(C)]
struct FSE_CState_t {
    value: isize,
    stateTable: *const c_void,
    symbolTT: *const c_void,
    stateLog: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DStream_t {
    bitContainer: usize,
    bitsConsumed: c_uint,
    ptr: *const c_char,
    start: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DState_t {
    state: usize,
    table: *const c_void,
}

/* FSE_DStream_status */
const FSE_DStream_unfinished: u32 = 0;
const FSE_DStream_endOfBuffer: u32 = 1;
const FSE_DStream_completed: u32 = 2;
const FSE_DStream_tooFar: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_decode_t {
    newState: U16,
    symbol: u8,
    nbBits: u8,
}

#[repr(C)]
struct FSE_symbolCompressionTransform {
    deltaFindState: c_int,
    deltaNbBits: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DTableHeader {
    tableLog: U16,
    fastMode: U16,
}

/* DTable_max_t = U32[FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)] */
const DTABLE_MAX_LEN: usize = 1 + (1usize << 12); /* fse_dtable_size_u32(FSE_MAX_TABLELOG) */

/****************************************************************
*  Memory I/O
****************************************************************/
#[inline]
fn FSE_32bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 4) as c_uint
}

#[inline]
fn FSE_isLittleEndian() -> c_uint {
    cfg!(target_endian = "little") as c_uint
}

#[inline]
unsafe fn FSE_read16(memPtr: *const c_void) -> U16 {
    let mut val: U16 = 0;
    memcpy(
        &mut val as *mut U16 as *mut c_void,
        memPtr,
        core::mem::size_of::<U16>(),
    );
    val
}

#[inline]
unsafe fn FSE_read32(memPtr: *const c_void) -> U32 {
    let mut val: U32 = 0;
    memcpy(
        &mut val as *mut U32 as *mut c_void,
        memPtr,
        core::mem::size_of::<U32>(),
    );
    val
}

#[inline]
unsafe fn FSE_read64(memPtr: *const c_void) -> U64 {
    let mut val: U64 = 0;
    memcpy(
        &mut val as *mut U64 as *mut c_void,
        memPtr,
        core::mem::size_of::<U64>(),
    );
    val
}

#[inline]
unsafe fn FSE_readLE16(memPtr: *const c_void) -> U16 {
    if FSE_isLittleEndian() != 0 {
        FSE_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}

#[inline]
unsafe fn FSE_readLE32(memPtr: *const c_void) -> U32 {
    if FSE_isLittleEndian() != 0 {
        FSE_read32(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p as U32)
            .wrapping_add((*p.add(1) as U32) << 8)
            .wrapping_add((*p.add(2) as U32) << 16)
            .wrapping_add((*p.add(3) as U32) << 24)
    }
}

#[inline]
unsafe fn FSE_readLE64(memPtr: *const c_void) -> U64 {
    if FSE_isLittleEndian() != 0 {
        FSE_read64(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p as U64)
            .wrapping_add((*p.add(1) as U64) << 8)
            .wrapping_add((*p.add(2) as U64) << 16)
            .wrapping_add((*p.add(3) as U64) << 24)
            .wrapping_add((*p.add(4) as U64) << 32)
            .wrapping_add((*p.add(5) as U64) << 40)
            .wrapping_add((*p.add(6) as U64) << 48)
            .wrapping_add((*p.add(7) as U64) << 56)
    }
}

#[inline]
unsafe fn FSE_readLEST(memPtr: *const c_void) -> usize {
    if FSE_32bits() != 0 {
        FSE_readLE32(memPtr) as usize
    } else {
        FSE_readLE64(memPtr) as usize
    }
}

/****************************************************************
*  Internal functions
****************************************************************/
#[inline]
fn FSE_highbit32(val: U32) -> c_uint {
    /* __builtin_clz(val) ^ 31 == 31 - clz(val) for val != 0 */
    (31 ^ val.leading_zeros()) as c_uint
}

#[inline]
fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

unsafe fn FSE_buildDTable(
    dt: *mut U32,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
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

    /* Sanity Checks */
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return fse_error(FSE_ERROR_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return fse_error(FSE_ERROR_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    (*DTableH).tableLog = tableLog as U16;
    s = 0;
    while s <= maxSymbolValue {
        if *normalizedCounter.add(s as usize) == -1 {
            (*tableDecode.offset(highThreshold as isize)).symbol = s as BYTE;
            highThreshold = highThreshold.wrapping_sub(1);
            symbolNext[s as usize] = 1;
        } else {
            if *normalizedCounter.add(s as usize) >= largeLimit {
                noLarge = 0;
            }
            symbolNext[s as usize] = *normalizedCounter.add(s as usize) as U16;
        }
        s += 1;
    }

    /* Spread symbols */
    s = 0;
    while s <= maxSymbolValue {
        let mut i: c_int = 0;
        while i < *normalizedCounter.add(s as usize) as c_int {
            (*tableDecode.offset(position as isize)).symbol = s as BYTE;
            position = (position + step) & tableMask;
            while position > highThreshold {
                position = (position + step) & tableMask; /* lowprob area */
            }
            i += 1;
        }
        s += 1;
    }

    if position != 0 {
        return fse_error(FSE_ERROR_GENERIC); /* position must reach all cells once */
    }

    /* Build Decoding table */
    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.offset(i as isize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.offset(i as isize)).nbBits =
                (tableLog - FSE_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.offset(i as isize)).newState =
                ((nextState as U32) << (*tableDecode.offset(i as isize)).nbBits)
                    .wrapping_sub(tableSize) as U16;
            i += 1;
        }
    }

    (*DTableH).fastMode = noLarge as U16;
    0
}

#[inline]
fn FSE_isError(code: usize) -> c_uint {
    (code > fse_error(FSE_ERROR_maxCode)) as c_uint
}

#[inline]
fn FSE_abs(a: S16) -> S16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

/****************************************************************
*  Header bitstream management
****************************************************************/
unsafe fn FSE_readNCount(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const BYTE;
    let iend = istart.add(hbSize);
    let mut ip = istart;
    let mut nbBits: c_int;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bitStream: U32;
    let mut bitCount: c_int;
    let mut charnum: c_uint = 0;
    let mut previous0: c_int = 0;

    if hbSize < 4 {
        return fse_error(FSE_ERROR_srcSize_wrong);
    }
    bitStream = FSE_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) + FSE_MIN_TABLELOG) as c_int; /* extract tableLog */
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as c_int {
        return fse_error(FSE_ERROR_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1 << nbBits) + 1;
    threshold = 1 << nbBits;
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0: c_uint = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 += 24;
                if ip < iend.sub(5) {
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
                return fse_error(FSE_ERROR_maxSymbolValue_tooSmall);
            }
            while charnum < n0 {
                *normalizedCounter.add(charnum as usize) = 0;
                charnum += 1;
            }
            if (ip <= iend.sub(7)) || (ip.offset((bitCount >> 3) as isize) <= iend.sub(4)) {
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

            if (bitStream & (threshold as U32 - 1)) < (max as U32) {
                count = (bitStream & (threshold as U32 - 1)) as S16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold as U32 - 1)) as S16;
                if count >= threshold as S16 {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1; /* extra accuracy */
            remaining -= FSE_abs(count) as c_int;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as c_int;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            {
                if (ip <= iend.sub(7)) || (ip.offset((bitCount >> 3) as isize) <= iend.sub(4)) {
                    ip = ip.offset((bitCount >> 3) as isize);
                    bitCount &= 7;
                } else {
                    bitCount -= (8 * (iend as isize - 4 - ip as isize)) as c_int;
                    ip = iend.sub(4);
                }
                bitStream = FSE_readLE32(ip as *const c_void) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return fse_error(FSE_ERROR_GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    if (ip as usize - istart as usize) > hbSize {
        return fse_error(FSE_ERROR_srcSize_wrong);
    }
    (ip as usize) - (istart as usize)
}

/*********************************************************
*  Decompression (Byte symbols)
*********************************************************/
unsafe fn FSE_buildDTable_rle(dt: *mut U32, symbolValue: BYTE) -> usize {
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

unsafe fn FSE_buildDTable_raw(dt: *mut U32, nbBits: c_uint) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let dinfo = (ptr as *mut FSE_decode_t).add(1);
    let tableSize: c_uint = 1u32 << nbBits;
    let tableMask: c_uint = tableSize - 1;
    let maxSymbolValue: c_uint = tableMask;
    let mut s: c_uint;

    /* Sanity checks */
    if nbBits < 1 {
        return fse_error(FSE_ERROR_GENERIC); /* min size */
    }

    /* Build Decoding Table */
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    s = 0;
    while s <= maxSymbolValue {
        (*dinfo.offset(s as isize)).newState = 0;
        (*dinfo.offset(s as isize)).symbol = s as BYTE;
        (*dinfo.offset(s as isize)).nbBits = nbBits as BYTE;
        s += 1;
    }

    0
}

/* FSE_initDStream */
unsafe fn FSE_initDStream(
    bitD: *mut FSE_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        return fse_error(FSE_ERROR_srcSize_wrong);
    }

    let stsize = core::mem::size_of::<usize>();
    if srcSize >= stsize {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char).add(srcSize - stsize);
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return fse_error(FSE_ERROR_GENERIC); /* stop bit not present */
        }
        (*bitD).bitsConsumed = 8 - FSE_highbit32(contain32);
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let start = (*bitD).start as *const BYTE;
        match srcSize {
            7 => {
                (*bitD).bitContainer +=
                    (*(start.add(6)) as usize) << (stsize * 8 - 16);
                (*bitD).bitContainer +=
                    (*(start.add(5)) as usize) << (stsize * 8 - 24);
                (*bitD).bitContainer +=
                    (*(start.add(4)) as usize) << (stsize * 8 - 32);
                (*bitD).bitContainer += (*(start.add(3)) as usize) << 24;
                (*bitD).bitContainer += (*(start.add(2)) as usize) << 16;
                (*bitD).bitContainer += (*(start.add(1)) as usize) << 8;
            }
            6 => {
                (*bitD).bitContainer +=
                    (*(start.add(5)) as usize) << (stsize * 8 - 24);
                (*bitD).bitContainer +=
                    (*(start.add(4)) as usize) << (stsize * 8 - 32);
                (*bitD).bitContainer += (*(start.add(3)) as usize) << 24;
                (*bitD).bitContainer += (*(start.add(2)) as usize) << 16;
                (*bitD).bitContainer += (*(start.add(1)) as usize) << 8;
            }
            5 => {
                (*bitD).bitContainer +=
                    (*(start.add(4)) as usize) << (stsize * 8 - 32);
                (*bitD).bitContainer += (*(start.add(3)) as usize) << 24;
                (*bitD).bitContainer += (*(start.add(2)) as usize) << 16;
                (*bitD).bitContainer += (*(start.add(1)) as usize) << 8;
            }
            4 => {
                (*bitD).bitContainer += (*(start.add(3)) as usize) << 24;
                (*bitD).bitContainer += (*(start.add(2)) as usize) << 16;
                (*bitD).bitContainer += (*(start.add(1)) as usize) << 8;
            }
            3 => {
                (*bitD).bitContainer += (*(start.add(2)) as usize) << 16;
                (*bitD).bitContainer += (*(start.add(1)) as usize) << 8;
            }
            2 => {
                (*bitD).bitContainer += (*(start.add(1)) as usize) << 8;
            }
            _ => {}
        }
        let contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return fse_error(FSE_ERROR_GENERIC); /* stop bit not present */
        }
        (*bitD).bitsConsumed = 8 - FSE_highbit32(contain32);
        (*bitD).bitsConsumed += ((stsize - srcSize) * 8) as c_uint;
    }

    srcSize
}

#[inline]
unsafe fn FSE_lookBits(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask - nbBits) & bitMask)
}

#[inline]
unsafe fn FSE_lookBitsFast(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1) - nbBits) & bitMask)
}

#[inline]
unsafe fn FSE_skipBits(bitD: *mut FSE_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
unsafe fn FSE_readBits(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let value = FSE_lookBits(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    value
}

#[inline]
unsafe fn FSE_readBitsFast(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let value = FSE_lookBitsFast(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    value
}

unsafe fn FSE_reloadDStream(bitD: *mut FSE_DStream_t) -> c_uint {
    let stsize = core::mem::size_of::<usize>();
    if (*bitD).bitsConsumed as usize > (stsize * 8) {
        return FSE_DStream_tooFar;
    }

    if (*bitD).ptr >= (*bitD).start.add(stsize) {
        (*bitD).ptr = (*bitD).ptr.offset(-(((*bitD).bitsConsumed >> 3) as isize));
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        return FSE_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize) < stsize * 8 {
            return FSE_DStream_endOfBuffer;
        }
        return FSE_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: U32 = FSE_DStream_unfinished;
        if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize - (*bitD).start as usize) as U32;
            result = FSE_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut FSE_DStream_t,
    dt: *const U32,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSE_DTableHeader;
    (*DStatePtr).state = FSE_readBits(bitD, (*DTableH).tableLog as U32);
    FSE_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline]
unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut FSE_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = FSE_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

#[inline]
unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut FSE_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = FSE_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

#[inline]
unsafe fn FSE_endOfDStream(bitD: *const FSE_DStream_t) -> c_uint {
    (((*bitD).ptr == (*bitD).start)
        && ((*bitD).bitsConsumed as usize == core::mem::size_of::<usize>() * 8)) as c_uint
}

#[inline]
unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> c_uint {
    ((*DStatePtr).state == 0) as c_uint
}

unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const U32,
    fast: c_uint,
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
    let mut errorCode: usize;

    /* Init */
    errorCode = FSE_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    let stbits = (core::mem::size_of::<usize>() * 8) as u32;

    macro_rules! GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSE_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSE_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    /* 4 symbols per loop */
    while (FSE_reloadDStream(&mut bitD) == FSE_DStream_unfinished) && (op < olimit) {
        *op.offset(0) = GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > stbits {
            FSE_reloadDStream(&mut bitD);
        }

        *op.offset(1) = GETSYMBOL!(&mut state2);

        if FSE_MAX_TABLELOG * 4 + 7 > stbits {
            if FSE_reloadDStream(&mut bitD) > FSE_DStream_unfinished {
                op = op.offset(2);
                break;
            }
        }

        *op.offset(2) = GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > stbits {
            FSE_reloadDStream(&mut bitD);
        }

        *op.offset(3) = GETSYMBOL!(&mut state2);

        op = op.offset(4);
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

    /* end ? */
    if FSE_endOfDStream(&bitD) != 0
        && FSE_endOfDState(&state1) != 0
        && FSE_endOfDState(&state2) != 0
    {
        return (op as usize) - (ostart as usize);
    }

    if op == omax {
        return fse_error(FSE_ERROR_dstSize_tooSmall);
    }

    fse_error(FSE_ERROR_corruptionDetected)
}

unsafe fn FSE_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const U32,
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

    /* select fast mode (static) */
    if DTableH.fastMode != 0 {
        return FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

unsafe fn FSE_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [S16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [U32; DTABLE_MAX_LEN] = [0; DTABLE_MAX_LEN];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSE_MAX_SYMBOL_VALUE;
    let mut errorCode: usize;
    let mut cSrcSize = cSrcSize;

    if cSrcSize < 2 {
        return fse_error(FSE_ERROR_srcSize_wrong); /* too small input size */
    }

    /* normal FSE decoding mode */
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
        return fse_error(FSE_ERROR_srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSE_buildDTable(
        dt.as_mut_ptr(),
        counting.as_ptr(),
        maxSymbolValue,
        tableLog,
    );
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    /* always return, even if it is an error code */
    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

/* *******************************************************
*  Huff0 : Huffman block compression / decompression
*********************************************************/
const HUF_MAX_SYMBOL_VALUE: u32 = 255;
const HUF_DEFAULT_TABLELOG: u32 = 12;
const HUF_MAX_TABLELOG: u32 = 12;
const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16;

/* HUF_DTABLE_SIZE_U16(maxTableLog) = 1 + (1<<maxTableLog) */
const HUF_DTABLE_LEN: usize = 1 + (1usize << 12); /* HUF_MAX_TABLELOG */

#[repr(C)]
struct HUF_CElt {
    val: U16,
    nbBits: BYTE,
}

#[repr(C)]
struct nodeElt {
    count: U32,
    parent: U16,
    byte: BYTE,
    nbBits: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DElt {
    byte: BYTE,
    nbBits: BYTE,
}

unsafe fn HUF_readDTable(DTable: *mut U16, src: *const c_void, srcSize: usize) -> usize {
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
        return fse_error(FSE_ERROR_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            let l: [c_int; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = l[iSize - 242] as usize;
            memset(
                huffWeight.as_mut_ptr() as *mut c_void,
                1,
                core::mem::size_of_val(&huffWeight),
            );
            iSize = 0;
        } else {
            /* Incompressible */
            oSize = iSize - 127;
            iSize = (oSize + 1) / 2;
            if iSize + 1 > srcSize {
                return fse_error(FSE_ERROR_srcSize_wrong);
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
        /* header compressed with FSE (normal case) */
        if iSize + 1 > srcSize {
            return fse_error(FSE_ERROR_srcSize_wrong);
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
            return fse_error(FSE_ERROR_corruptionDetected);
        }
        rankVal[huffWeight[n as usize] as usize] += 1;
        weightTotal += (1u32 << huffWeight[n as usize]) >> 1;
        n += 1;
    }
    if weightTotal == 0 {
        return fse_error(FSE_ERROR_corruptionDetected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    maxBits = FSE_highbit32(weightTotal) + 1;
    if maxBits > *DTable.add(0) as U32 {
        return fse_error(FSE_ERROR_tableLog_tooLarge);
    }
    *DTable.add(0) = maxBits as U16;
    {
        let total: U32 = 1u32 << maxBits;
        let rest: U32 = total - weightTotal;
        let verif: U32 = 1u32 << FSE_highbit32(rest);
        let lastWeight: U32 = FSE_highbit32(rest) + 1;
        if verif != rest {
            return fse_error(FSE_ERROR_corruptionDetected);
        }
        huffWeight[oSize] = lastWeight as BYTE;
        rankVal[lastWeight as usize] += 1;
    }

    /* check tree construction validity */
    if (rankVal[1] < 2) || (rankVal[1] & 1) != 0 {
        return fse_error(FSE_ERROR_corruptionDetected);
    }

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= maxBits {
        let current: U32 = nextRankStart;
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
        let mut D = HUF_DElt { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = (maxBits + 1 - w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize] + length {
            *dt.offset(i as isize) = D;
            i += 1;
        }
        rankVal[w as usize] += length;
        n += 1;
    }

    iSize + 1
}

#[inline]
unsafe fn HUF_decodeSymbol(Dstream: *mut FSE_DStream_t, dt: *const HUF_DElt, dtLog: U32) -> BYTE {
    let val = FSE_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c = (*dt.add(val)).byte;
    FSE_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

unsafe fn HUF_decompress_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    if cSrcSize < 6 {
        return fse_error(FSE_ERROR_srcSize_wrong);
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

        /* Init */
        let jumpTable = cSrc as *const U16;
        let length1 = FSE_readLE16(jumpTable as *const c_void) as usize;
        let length2 = FSE_readLE16(jumpTable.add(1) as *const c_void) as usize;
        let length3 = FSE_readLE16(jumpTable.add(2) as *const c_void) as usize;
        let length4 = cSrcSize
            .wrapping_sub(6)
            .wrapping_sub(length1)
            .wrapping_sub(length2)
            .wrapping_sub(length3);
        let start1 = (cSrc as *const c_char).add(6);
        let start2 = start1.add(length1);
        let start3 = start2.add(length2);
        let start4 = start3.add(length3);
        let mut bitD1 = FSE_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = bitD1;
        let mut bitD3 = bitD1;
        let mut bitD4 = bitD1;

        if length1 + length2 + length3 + 6 >= cSrcSize {
            return fse_error(FSE_ERROR_srcSize_wrong);
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

        macro_rules! DECODE_0 {
            ($n:expr, $D:expr) => {
                *op.offset($n) = HUF_decodeSymbol(&mut $D, dt, dtLog);
            };
        }
        macro_rules! DECODE_1 {
            ($n:expr, $D:expr) => {
                *op.offset($n) = HUF_decodeSymbol(&mut $D, dt, dtLog);
                if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                    FSE_reloadDStream(&mut $D);
                }
            };
        }
        macro_rules! DECODE_2 {
            ($n:expr, $D:expr) => {
                *op.offset($n) = HUF_decodeSymbol(&mut $D, dt, dtLog);
                if FSE_32bits() != 0 {
                    FSE_reloadDStream(&mut $D);
                }
            };
        }

        /* 16 symbols per loop */
        while (reloadStatus < FSE_DStream_completed) && (op < olimit) {
            DECODE_1!(0, bitD1);
            DECODE_1!(1, bitD2);
            DECODE_1!(2, bitD3);
            DECODE_1!(3, bitD4);
            DECODE_2!(4, bitD1);
            DECODE_2!(5, bitD2);
            DECODE_2!(6, bitD3);
            DECODE_2!(7, bitD4);
            DECODE_1!(8, bitD1);
            DECODE_1!(9, bitD2);
            DECODE_1!(10, bitD3);
            DECODE_1!(11, bitD4);
            DECODE_0!(12, bitD1);
            DECODE_0!(13, bitD2);
            DECODE_0!(14, bitD3);
            DECODE_0!(15, bitD4);

            op = op.offset(16);
            reloadStatus = FSE_reloadDStream(&mut bitD2)
                | FSE_reloadDStream(&mut bitD3)
                | FSE_reloadDStream(&mut bitD4);
            FSE_reloadDStream(&mut bitD1);
        }

        if reloadStatus != FSE_DStream_completed {
            return fse_error(FSE_ERROR_corruptionDetected);
        }

        /* tail */
        {
            let mut bitTail = FSE_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            };
            bitTail.ptr = bitD1.ptr;
            bitTail.bitsConsumed = bitD1.bitsConsumed;
            bitTail.bitContainer = bitD1.bitContainer;
            bitTail.start = start1;
            while (FSE_reloadDStream(&mut bitTail) < FSE_DStream_completed) && (op < omax) {
                DECODE_0!(0, bitTail);
                op = op.add(1);
            }

            if FSE_endOfDStream(&bitTail) != 0 {
                return (op as usize) - (ostart as usize);
            }
        }

        if op == omax {
            return fse_error(FSE_ERROR_dstSize_tooSmall);
        }

        fse_error(FSE_ERROR_corruptionDetected)
    }
}

unsafe fn HUF_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* HUF_CREATE_STATIC_DTABLE(DTable, HUF_MAX_TABLELOG) = U16[..] = { maxTableLog } */
    let mut DTable: [U16; HUF_DTABLE_LEN] = [0; HUF_DTABLE_LEN];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut errorCode: usize;
    let mut cSrcSize = cSrcSize;

    errorCode = HUF_readDTable(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return fse_error(FSE_ERROR_srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUF_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/****************************************************************
*  ZSTD constants
****************************************************************/
const ZSTD_MEMORY_USAGE: u32 = 17;

const ZSTD_magicNumber: U32 = 0xFD2FB51E;

const HASH_LOG: u32 = ZSTD_MEMORY_USAGE - 2;
const HASH_TABLESIZE: usize = 1 << HASH_LOG;
const HASH_MASK: usize = HASH_TABLESIZE - 1;

const KNUTH: u32 = 2654435761;

const BIT7: u32 = 128;
const BIT6: u32 = 64;
const BIT5: u32 = 32;
const BIT4: u32 = 16;

const BLOCKSIZE: usize = 128 * (1 << 10);

const WORKPLACESIZE: usize = BLOCKSIZE * 3;
const MINMATCH: usize = 4;
const MLbits: u32 = 7;
const LLbits: u32 = 6;
const Offbits: u32 = 5;
const MaxML: u32 = (1 << MLbits) - 1;
const MaxLL: u32 = (1 << LLbits) - 1;
const MaxOff: u32 = (1 << Offbits) - 1;
const LitFSELog: u32 = 11;
const MLFSELog: u32 = 10;
const LLFSELog: u32 = 10;
const OffFSELog: u32 = 9;
const MaxSeq: u32 = if MaxLL < MaxML { MaxML } else { MaxLL };

const LITERAL_NOENTROPY: u32 = 63;
const COMMAND_NOENTROPY: u32 = 7;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

const ZSTD_blockHeaderSize: usize = 3;
const ZSTD_frameHeaderSize: usize = 4;

/****************************************************************
*  ZSTD Memory operations
****************************************************************/
#[inline]
fn ZSTD_32bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 4) as c_uint
}

#[inline]
fn ZSTD_isLittleEndian() -> c_uint {
    cfg!(target_endian = "little") as c_uint
}

#[inline]
unsafe fn ZSTD_read16(p: *const c_void) -> U16 {
    let mut r: U16 = 0;
    memcpy(
        &mut r as *mut U16 as *mut c_void,
        p,
        core::mem::size_of::<U16>(),
    );
    r
}

#[inline]
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

#[inline]
unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

#[inline]
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

#[inline]
unsafe fn ZSTD_readLE16(memPtr: *const c_void) -> U16 {
    if ZSTD_isLittleEndian() != 0 {
        ZSTD_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}

#[inline]
unsafe fn ZSTD_readLE24(memPtr: *const c_void) -> U32 {
    (ZSTD_readLE16(memPtr) as U32).wrapping_add(((*((memPtr as *const BYTE).add(2))) as U32) << 16)
}

#[inline]
unsafe fn ZSTD_readBE32(memPtr: *const c_void) -> U32 {
    let p = memPtr as *const BYTE;
    ((*p as U32) << 24)
        .wrapping_add((*p.add(1) as U32) << 16)
        .wrapping_add((*p.add(2) as U32) << 8)
        .wrapping_add((*p.add(3) as U32) << 0)
}

/****************************************************************
*  Local structures
****************************************************************/
#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
enum blockType_t {
    bt_compressed = 0,
    bt_raw = 1,
    bt_rle = 2,
    bt_end = 3,
}

#[repr(C)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

/* ZSTD error helper: ERROR(name) in the ZSTD section maps to crate error codes */
#[inline]
fn ERROR_(code: i32) -> usize {
    zstd_error(code)
}

/**************************************
*  Error Management
**************************************/
/* published entry point */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_isError(code: usize) -> c_uint {
    err_is_error(code)
}

#[inline]
fn blockType_from_u32(v: U32) -> blockType_t {
    match v & 3 {
        0 => blockType_t::bt_compressed,
        1 => blockType_t::bt_raw,
        2 => blockType_t::bt_rle,
        _ => blockType_t::bt_end,
    }
}

/**************************************************************
*   Decompression code
**************************************************************/
unsafe fn ZSTDv01_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let inp = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR_(ec::SRCSIZE_WRONG);
    }

    headerFlags = *inp;
    cSize = (*inp.add(2) as U32)
        .wrapping_add((*inp.add(1) as U32) << 8)
        .wrapping_add(((*inp.add(0) as U32) & 7) << 16);

    (*bpPtr).blockType = blockType_from_u32((headerFlags >> 6) as U32);
    (*bpPtr).origSize = if (*bpPtr).blockType == blockType_t::bt_rle {
        cSize
    } else {
        0
    };

    if (*bpPtr).blockType == blockType_t::bt_end {
        return 0;
    }
    if (*bpPtr).blockType == blockType_t::bt_rle {
        return 1;
    }
    cSize as usize
}

unsafe fn ZSTD_copyUncompressedBlock(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize > maxDstSize {
        return ERROR_(ec::DSTSIZE_TOOSMALL);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

unsafe fn ZSTD_decompressLiterals(
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

    /* check : minimum 2, for litSize, +1, for content */
    if srcSize <= 3 {
        return ERROR_(ec::CORRUPTION_DETECTED);
    }

    litSize = (*ip.add(1) as usize) + ((*ip.add(0) as usize) << 8);
    litSize += (((*ip.offset(-3) as usize) >> 3) & 7) << 16; /* mmmmh.... */
    op = oend.offset(-(litSize as isize));

    let _ = ctx;
    if litSize > maxDstSize {
        return ERROR_(ec::DSTSIZE_TOOSMALL);
    }
    let errorCode = HUF_decompress(
        op as *mut c_void,
        litSize,
        ip.add(2) as *const c_void,
        srcSize - 2,
    );
    if FSE_isError(errorCode) != 0 {
        return ERROR_(ec::GENERIC);
    }
    litSize
}

unsafe fn ZSTDv01_decodeLiteralsBlock(
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
        blockType: blockType_t::bt_compressed,
        origSize: 0,
    };

    let litcSize = ZSTDv01_getcBlockSize(src, srcSize, &mut litbp);
    if ZSTDv01_isError(litcSize) != 0 {
        return litcSize;
    }
    if litcSize > srcSize - ZSTD_blockHeaderSize {
        return ERROR_(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(ZSTD_blockHeaderSize);

    match litbp.blockType {
        blockType_t::bt_raw => {
            *litStart = ip;
            ip = ip.add(litcSize);
            *litSize = litcSize;
        }
        blockType_t::bt_rle => {
            let rleSize = litbp.origSize as usize;
            if rleSize > maxDstSize {
                return ERROR_(ec::DSTSIZE_TOOSMALL);
            }
            if srcSize == 0 {
                return ERROR_(ec::SRCSIZE_WRONG);
            }
            if rleSize > 0 {
                memset(
                    oend.offset(-(rleSize as isize)) as *mut c_void,
                    *ip as c_int,
                    rleSize,
                );
            }
            *litStart = oend.offset(-(rleSize as isize));
            *litSize = rleSize;
            ip = ip.add(1);
        }
        blockType_t::bt_compressed => {
            let decodedLitSize =
                ZSTD_decompressLiterals(ctx, dst, maxDstSize, ip as *const c_void, litcSize);
            if ZSTDv01_isError(decodedLitSize) != 0 {
                return decodedLitSize;
            }
            *litStart = oend.offset(-(decodedLitSize as isize));
            *litSize = decodedLitSize;
            ip = ip.add(litcSize);
        }
        blockType_t::bt_end => {
            return ERROR_(ec::GENERIC);
        }
    }

    (ip as usize) - (istart as usize)
}

unsafe fn ZSTDv01_decodeSeqHeaders(
    nbSeq: *mut c_int,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut U32,
    DTableML: *mut U32,
    DTableOffb: *mut U32,
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

    /* check */
    if srcSize < 5 {
        return ERROR_(ec::SRCSIZE_WRONG);
    }

    /* SeqHead */
    *nbSeq = ZSTD_readLE16(ip as *const c_void) as c_int;
    ip = ip.add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = (*ip.add(2) as usize) + ((*ip.add(1) as usize) << 8);
        ip = ip.add(3);
    } else {
        dumpsLength = (*ip.add(1) as usize) + (((*ip.add(0) as usize) & 1) << 8);
        ip = ip.add(2);
    }
    *dumpsPtr = ip;
    ip = ip.add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    /* check */
    if ip > iend.offset(-3) {
        return ERROR_(ec::SRCSIZE_WRONG);
    }

    /* sequences */
    {
        let mut norm: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut headerSize: usize;

        /* Build DTables */
        match blockType_from_u32(LLtype) {
            blockType_t::bt_rle => {
                LLlog = 0;
                let v = *ip;
                ip = ip.add(1);
                FSE_buildDTable_rle(DTableLL, v);
            }
            blockType_t::bt_raw => {
                LLlog = LLbits;
                FSE_buildDTable_raw(DTableLL, LLbits);
            }
            _ => {
                let mut max: U32 = MaxLL;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut LLlog,
                    ip as *const c_void,
                    (iend as usize) - (ip as usize),
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR_(ec::GENERIC);
                }
                if LLlog > LLFSELog {
                    return ERROR_(ec::CORRUPTION_DETECTED);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        match blockType_from_u32(Offtype) {
            blockType_t::bt_rle => {
                Offlog = 0;
                if ip > iend.offset(-2) {
                    return ERROR_(ec::SRCSIZE_WRONG);
                }
                let v = *ip;
                ip = ip.add(1);
                FSE_buildDTable_rle(DTableOffb, v);
            }
            blockType_t::bt_raw => {
                Offlog = Offbits;
                FSE_buildDTable_raw(DTableOffb, Offbits);
            }
            _ => {
                let mut max: U32 = MaxOff;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut Offlog,
                    ip as *const c_void,
                    (iend as usize) - (ip as usize),
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR_(ec::GENERIC);
                }
                if Offlog > OffFSELog {
                    return ERROR_(ec::CORRUPTION_DETECTED);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
            }
        }

        match blockType_from_u32(MLtype) {
            blockType_t::bt_rle => {
                MLlog = 0;
                if ip > iend.offset(-2) {
                    return ERROR_(ec::SRCSIZE_WRONG);
                }
                let v = *ip;
                ip = ip.add(1);
                FSE_buildDTable_rle(DTableML, v);
            }
            blockType_t::bt_raw => {
                MLlog = MLbits;
                FSE_buildDTable_raw(DTableML, MLbits);
            }
            _ => {
                let mut max: U32 = MaxML;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut MLlog,
                    ip as *const c_void,
                    (iend as usize) - (ip as usize),
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR_(ec::GENERIC);
                }
                if MLlog > MLFSELog {
                    return ERROR_(ec::CORRUPTION_DETECTED);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
            }
        }
    }

    (ip as usize) - (istart as usize)
}

#[repr(C)]
#[derive(Clone, Copy)]
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

unsafe fn ZSTD_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
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
            let v = *dumps as U32;
            dumps = dumps.add(1);
            v
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
            nbBits = 0; /* cmove */
        }
        offset = ((1usize) << (nbBits & ((core::mem::size_of::<usize>() * 8) as U32 - 1)))
            .wrapping_add(FSE_readBits(&mut (*seqState).DStream, nbBits));
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
            let v = *dumps as U32;
            dumps = dumps.add(1);
            v
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

unsafe fn ZSTD_execSequence(
    op: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> usize {
    let dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
    let dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
    let ostart = op;
    let oLitEnd = op.add(sequence.litLength);
    let litLength = sequence.litLength;
    let endMatch = op.add(litLength + sequence.matchLength);
    let litEnd = (*litPtr).add(litLength);
    let mut op = op;

    /* checks */
    let seqLength: usize = sequence.litLength + sequence.matchLength;

    if seqLength > (oend as usize).wrapping_sub(op as usize) {
        return ERROR_(ec::DSTSIZE_TOOSMALL);
    }
    if sequence.litLength > (litLimit as usize).wrapping_sub(*litPtr as usize) {
        return ERROR_(ec::CORRUPTION_DETECTED);
    }
    if sequence.offset > (oLitEnd as usize).wrapping_sub(base as usize) as U32 as usize {
        return ERROR_(ec::CORRUPTION_DETECTED);
    }

    if endMatch > oend {
        return ERROR_(ec::DSTSIZE_TOOSMALL); /* overwrite beyond dst buffer */
    }
    if litEnd > litLimit {
        return ERROR_(ec::CORRUPTION_DETECTED); /* overRead beyond lit buffer */
    }
    if sequence.matchLength > (*litPtr as usize).wrapping_sub(op as usize) {
        return ERROR_(ec::DSTSIZE_TOOSMALL); /* overwrite literal segment */
    }

    /* copy Literals */
    memmove(op as *mut c_void, *litPtr as *const c_void, sequence.litLength);

    op = op.add(litLength);
    *litPtr = litEnd; /* update for next sequence */

    /* check : last match must be at a minimum distance of 8 from end of dest buffer */
    if (oend as isize) - (op as isize) < 8 {
        return ERROR_(ec::DSTSIZE_TOOSMALL);
    }

    /* copy Match */
    {
        let overlapRisk: U32 =
            (((litEnd as usize).wrapping_sub(endMatch as usize)) < 12) as U32;
        let mut match_: *const BYTE = op.offset(-(sequence.offset as isize));
        let mut qutt: usize = 12;
        let mut saved: [U64; 2] = [0; 2];

        /* check */
        if match_ < base {
            return ERROR_(ec::CORRUPTION_DETECTED);
        }
        if sequence.offset > base as usize {
            return ERROR_(ec::CORRUPTION_DETECTED);
        }

        /* save beginning of literal sequence, in case of write overlap */
        if overlapRisk != 0 {
            if endMatch.add(qutt) > oend {
                qutt = (oend as usize) - (endMatch as usize);
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
                    (oend.offset(-8) as isize) - (op as isize),
                );
                match_ = match_.offset((oend.offset(-8) as isize) - (op as isize));
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
                (sequence.matchLength as isize) - 8,
            );
        }

        /* restore, in case of overlap */
        if overlapRisk != 0 {
            memcpy(
                endMatch as *mut c_void,
                saved.as_ptr() as *const c_void,
                qutt,
            );
        }
    }

    (endMatch as usize) - (ostart as usize)
}

#[repr(C)]
struct dctx_t {
    LLTable: [U32; fse_dtable_size_u32(LLFSELog)],
    OffTable: [U32; fse_dtable_size_u32(OffFSELog)],
    MLTable: [U32; fse_dtable_size_u32(MLFSELog)],
    previousDstEnd: *mut c_void,
    base: *mut c_void,
    expected: usize,
    bType: blockType_t,
    phase: U32,
}

unsafe fn ZSTD_decompressSequences(
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
    let mut nbSeq: c_int = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *mut BYTE;

    /* Build Decoding Tables */
    errorCode = ZSTDv01_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        (iend as usize) - (ip as usize),
    );
    if ZSTDv01_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.add(errorCode);

    /* Regen sequences */
    {
        let mut sequence = seq_t {
            litLength: 0,
            offset: 0,
            matchLength: 0,
        };
        let mut seqState: seqState_t = core::mem::zeroed();

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.add(dumpsLength);
        seqState.prevOffset = 1;
        errorCode = FSE_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            (iend as usize) - (ip as usize),
        );
        if FSE_isError(errorCode) != 0 {
            return ERROR_(ec::CORRUPTION_DETECTED);
        }
        FSE_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSE_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSE_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (FSE_reloadDStream(&mut seqState.DStream) <= FSE_DStream_completed) && (nbSeq > 0) {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(op, sequence, &mut litPtr, litEnd, base, oend);
            if ZSTDv01_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
        }

        /* check if reached exact end */
        if FSE_endOfDStream(&seqState.DStream) == 0 {
            return ERROR_(ec::CORRUPTION_DETECTED);
        }
        if nbSeq < 0 {
            return ERROR_(ec::CORRUPTION_DETECTED);
        }

        /* last literal segment */
        {
            let lastLLSize: usize = (litEnd as usize) - (litPtr as usize);
            if op.add(lastLLSize) > oend {
                return ERROR_(ec::DSTSIZE_TOOSMALL);
            }
            if lastLLSize > 0 {
                if op != litPtr as *mut BYTE {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.add(lastLLSize);
            }
        }
    }

    (op as usize) - (ostart as usize)
}

unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let mut litPtr: *const BYTE = core::ptr::null();
    let mut litSize: usize = 0;
    let errorCode: usize;
    let mut srcSize = srcSize;

    /* Decode literals sub-block */
    let errorCode = ZSTDv01_decodeLiteralsBlock(
        ctx,
        dst,
        maxDstSize,
        &mut litPtr,
        &mut litSize,
        src,
        srcSize,
    );
    if ZSTDv01_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.add(errorCode);
    srcSize -= errorCode;

    ZSTD_decompressSequences(
        ctx,
        dst,
        maxDstSize,
        ip as *const c_void,
        srcSize,
        litPtr,
        litSize,
    )
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
        blockType: blockType_t::bt_compressed,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        return ERROR_(ec::SRCSIZE_WRONG);
    }
    magicNumber = ZSTD_readBE32(src);
    if magicNumber != ZSTD_magicNumber {
        return ERROR_(ec::PREFIX_UNKNOWN);
    }
    ip = ip.add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    /* Loop on each block */
    loop {
        let blockSize = ZSTDv01_getcBlockSize(
            ip as *const c_void,
            (iend as usize) - (ip as usize),
            &mut blockProperties,
        );
        if ZSTDv01_isError(blockSize) != 0 {
            return blockSize;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if blockSize > remainingSize {
            return ERROR_(ec::SRCSIZE_WRONG);
        }

        match blockProperties.blockType {
            blockType_t::bt_compressed => {
                errorCode = ZSTD_decompressBlock(
                    ctx,
                    op as *mut c_void,
                    (oend as usize) - (op as usize),
                    ip as *const c_void,
                    blockSize,
                );
            }
            blockType_t::bt_raw => {
                errorCode = ZSTD_copyUncompressedBlock(
                    op as *mut c_void,
                    (oend as usize) - (op as usize),
                    ip as *const c_void,
                    blockSize,
                );
            }
            blockType_t::bt_rle => {
                return ERROR_(ec::GENERIC); /* not yet supported */
            }
            blockType_t::bt_end => {
                /* end of frame */
                if remainingSize != 0 {
                    return ERROR_(ec::SRCSIZE_WRONG);
                }
            }
        }
        if blockSize == 0 {
            break; /* bt_end */
        }

        if ZSTDv01_isError(errorCode) != 0 {
            return errorCode;
        }
        op = op.add(errorCode);
        ip = ip.add(blockSize);
        remainingSize -= blockSize;
    }

    (op as usize) - (ostart as usize)
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
    ZSTDv01_decompressDCtx(
        &mut ctx as *mut dctx_t as *mut c_void,
        dst,
        maxDstSize,
        src,
        srcSize,
    )
}

/* ZSTDv01_Dctx == dctx_t (opaque in the public header) */
type ZSTDv01_Dctx = dctx_t;

/* ZSTD_errorFrameSizeInfoLegacy() :
   assumes `cSize` and `dBound` are _not_ NULL */
unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut usize,
    dBound: *mut core::ffi::c_ulonglong,
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
    dBound: *mut core::ffi::c_ulonglong,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;
    let magicNumber: U32;
    let mut blockProperties = blockProperties_t {
        blockType: blockType_t::bt_compressed,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR_(ec::SRCSIZE_WRONG));
        return;
    }
    magicNumber = ZSTD_readBE32(src);
    if magicNumber != ZSTD_magicNumber {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR_(ec::PREFIX_UNKNOWN));
        return;
    }
    ip = ip.add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    /* Loop on each block */
    loop {
        let blockSize =
            ZSTDv01_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv01_isError(blockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, blockSize);
            return;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if blockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR_(ec::SRCSIZE_WRONG));
            return;
        }

        if blockSize == 0 {
            break; /* bt_end */
        }

        ip = ip.add(blockSize);
        remainingSize -= blockSize;
        nbBlocks += 1;
    }

    *cSize = (ip as usize) - (src as usize);
    *dBound = (nbBlocks * BLOCKSIZE) as core::ffi::c_ulonglong;
}

/*******************************
*  Streaming Decompression API
*******************************/
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

    /* Sanity check */
    if srcSize != (*ctx).expected {
        return ERROR_(ec::SRCSIZE_WRONG);
    }
    if dst != (*ctx).previousDstEnd {
        /* not contiguous */
        (*ctx).base = dst;
    }

    /* Decompress : frame header */
    if (*ctx).phase == 0 {
        /* Check frame magic header */
        let magicNumber: U32 = ZSTD_readBE32(src);
        if magicNumber != ZSTD_magicNumber {
            return ERROR_(ec::PREFIX_UNKNOWN);
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        return 0;
    }

    /* Decompress : block header */
    if (*ctx).phase == 1 {
        let mut bp = blockProperties_t {
            blockType: blockType_t::bt_compressed,
            origSize: 0,
        };
        let blockSize = ZSTDv01_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
        if ZSTDv01_isError(blockSize) != 0 {
            return blockSize;
        }
        if bp.blockType == blockType_t::bt_end {
            (*ctx).expected = 0;
            (*ctx).phase = 0;
        } else {
            (*ctx).expected = blockSize;
            (*ctx).bType = bp.blockType;
            (*ctx).phase = 2;
        }

        return 0;
    }

    /* Decompress : block content */
    {
        let rSize: usize;
        match (*ctx).bType {
            blockType_t::bt_compressed => {
                rSize = ZSTD_decompressBlock(ctx as *mut c_void, dst, maxDstSize, src, srcSize);
            }
            blockType_t::bt_raw => {
                rSize = ZSTD_copyUncompressedBlock(dst, maxDstSize, src, srcSize);
            }
            blockType_t::bt_rle => {
                return ERROR_(ec::GENERIC); /* not yet handled */
            }
            blockType_t::bt_end => {
                /* should never happen (filtered at phase 1) */
                rSize = 0;
            }
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTDv01_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = ((dst as *mut c_char).add(rSize)) as *mut c_void;
        rSize
    }
}

