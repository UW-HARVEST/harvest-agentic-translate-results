// Translation of c_src/src/legacy/zstd_v02.c
//
// This file is entirely self-contained: it carries its own private copies of the
// v0.2 FSE / Huff0 / bitstream / error-code / memory helpers, exactly like the C
// source does (everything there is `static`).

use crate::libc::{free, malloc, memcpy, memmove, memset};
use core::ffi::c_void;

/* ******************************************************************
 *  mem.h - Basic Types
 ****************************************************************** */

pub type BYTE = u8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

/* ******************************************************************
 *  Error codes.
 *
 *  NOTE: `zstd_v02.c` #includes `../common/error_private.h` at its very top,
 *  which defines the `ERROR_H_MODULE` guard.  Its own private error block is
 *  therefore wrapped in `#ifndef ERROR_H_MODULE` and is *preprocessed away*.
 *  The compiled C code consequently uses the **modern** `ZSTD_ErrorCode`
 *  values from `zstd_errors.h` (prefix_unknown == 10, not 4) and the modern
 *  `ERR_isError(code) == code > (size_t)-ZSTD_error_maxCode` with maxCode 120.
 *  These values are what the reference build returns, so they are used here.
 ****************************************************************** */

const ZSTD_error_No_Error: u32 = 0;
const ZSTD_error_GENERIC: u32 = 1;
const ZSTD_error_dstSize_tooSmall: u32 = 70;
const ZSTD_error_srcSize_wrong: u32 = 72;
const ZSTD_error_prefix_unknown: u32 = 10;
const ZSTD_error_corruption_detected: u32 = 20;
const ZSTD_error_tableLog_tooLarge: u32 = 44;
const ZSTD_error_maxSymbolValue_tooLarge: u32 = 46;
const ZSTD_error_maxSymbolValue_tooSmall: u32 = 48;
const ZSTD_error_maxCode: u32 = 120;

/* #define ERROR(name) (size_t)-PREFIX(name) */
const fn ERROR(name: u32) -> usize {
    (0usize).wrapping_sub(name as usize)
}

static ERR_strings: [&str; 10] = [
    "ZSTD_error_No_Error\0",
    "ZSTD_error_GENERIC\0",
    "ZSTD_error_dstSize_tooSmall\0",
    "ZSTD_error_srcSize_wrong\0",
    "ZSTD_error_prefix_unknown\0",
    "ZSTD_error_corruption_detected\0",
    "ZSTD_error_tableLog_tooLarge\0",
    "ZSTD_error_maxSymbolValue_tooLarge\0",
    "ZSTD_error_maxSymbolValue_tooSmall\0",
    "ZSTD_error_maxCode\0",
];

fn ERR_isError(code: usize) -> u32 {
    (code > ERROR(ZSTD_error_maxCode)) as u32
}

/* In the compiled C this is `error_private.h`'s inline, which forwards to the
 * shared `ERR_getErrorString()` (the v0.2-private string table is preprocessed
 * away together with the private enum). It is never called from this
 * translation unit — v0.2 exports no `getErrorName` — but keep it faithful. */
unsafe fn ERR_getErrorName(code: usize) -> *const i8 {
    let _ = &ERR_strings;
    crate::common::error_private::ERR_getErrorName(code) as *const i8
}

/* ******************************************************************
 *  mem.h - Memory I/O
 ****************************************************************** */

fn MEM_32bits() -> u32 {
    (core::mem::size_of::<*const c_void>() == 4) as u32
}

fn MEM_64bits() -> u32 {
    (core::mem::size_of::<*const c_void>() == 8) as u32
}

fn MEM_isLittleEndian() -> u32 {
    /* target is little endian */
    1
}

unsafe fn MEM_read16(memPtr: *const c_void) -> U16 {
    (memPtr as *const U16).read_unaligned()
}

unsafe fn MEM_read32(memPtr: *const c_void) -> U32 {
    (memPtr as *const U32).read_unaligned()
}

unsafe fn MEM_read64(memPtr: *const c_void) -> U64 {
    (memPtr as *const U64).read_unaligned()
}

unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    (memPtr as *mut U16).write_unaligned(value);
}

unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        MEM_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}

unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let p = memPtr as *mut BYTE;
        *p.add(0) = val as BYTE;
        *p.add(1) = (val >> 8) as BYTE;
    }
}

unsafe fn MEM_readLE24(memPtr: *const c_void) -> U32 {
    (MEM_readLE16(memPtr) as U32).wrapping_add(((*(memPtr as *const BYTE).add(2)) as U32) << 16)
}

unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        MEM_read32(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U32)
            .wrapping_add((*p.add(1) as U32) << 8)
            .wrapping_add((*p.add(2) as U32) << 16)
            .wrapping_add((*p.add(3) as U32) << 24)
    }
}

unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        MEM_read64(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U64)
            .wrapping_add((*p.add(1) as U64) << 8)
            .wrapping_add((*p.add(2) as U64) << 16)
            .wrapping_add((*p.add(3) as U64) << 24)
            .wrapping_add((*p.add(4) as U64) << 32)
            .wrapping_add((*p.add(5) as U64) << 40)
            .wrapping_add((*p.add(6) as U64) << 48)
            .wrapping_add((*p.add(7) as U64) << 56)
    }
}

unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    if MEM_32bits() != 0 {
        MEM_readLE32(memPtr) as usize
    } else {
        MEM_readLE64(memPtr) as usize
    }
}

/* ******************************************************************
 *  bitstream  (read backward)
 ****************************************************************** */

#[repr(C)]
#[derive(Clone, Copy)]
struct BIT_DStream_t {
    bitContainer: usize,
    bitsConsumed: u32,
    ptr: *const BYTE,
    start: *const BYTE,
}

type BIT_DStream_status = u32;
const BIT_DStream_unfinished: BIT_DStream_status = 0;
const BIT_DStream_endOfBuffer: BIT_DStream_status = 1;
const BIT_DStream_completed: BIT_DStream_status = 2;
const BIT_DStream_overflow: BIT_DStream_status = 3;

fn BIT_highbit32(val: U32) -> u32 {
    /* __builtin_clz(val) ^ 31 */
    val.leading_zeros() ^ 31
}

unsafe fn BIT_initDStream(
    bitD: *mut BIT_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BIT_DStream_t>());
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        /* normal case */
        let contain32: U32;
        (*bitD).start = srcBuffer as *const BYTE;
        (*bitD).ptr = (srcBuffer as *const BYTE)
            .wrapping_add(srcSize)
            .wrapping_sub(core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        contain32 = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); /* endMark not present */
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const BYTE;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start) as usize;
        /* C switch(srcSize) with fall-through from 7 down to 2 */
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*((*bitD).start).wrapping_add(6) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 16),
            );
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*((*bitD).start).wrapping_add(5) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 24),
            );
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*((*bitD).start).wrapping_add(4) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 32),
            );
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*((*bitD).start).wrapping_add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*((*bitD).start).wrapping_add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*((*bitD).start).wrapping_add(1) as usize) << 8);
        }
        contain32 = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); /* endMark not present */
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
        (*bitD).bitsConsumed = (*bitD)
            .bitsConsumed
            .wrapping_add(((core::mem::size_of::<usize>() - srcSize) as U32).wrapping_mul(8));
    }

    srcSize
}

unsafe fn BIT_lookBits(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> (bitMask.wrapping_sub(nbBits) & bitMask)
}

/* unsafe version; only works if nbBits >= 1 */
unsafe fn BIT_lookBitsFast(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((bitMask.wrapping_add(1).wrapping_sub(nbBits)) & bitMask)
}

unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

unsafe fn BIT_readBits(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

/* unsafe version; only works if nbBits >= 1 */
unsafe fn BIT_readBitsFast(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

unsafe fn BIT_reloadDStream(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
        /* should never happen */
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD)
            .ptr
            .wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize) < core::mem::size_of::<usize>() * 8 {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BIT_DStream_status = BIT_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize).wrapping_sub((*bitD).start as usize) as U32; /* ptr > start */
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

unsafe fn BIT_endOfDStream(DStream: *const BIT_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed as usize == core::mem::size_of::<usize>() * 8)) as u32
}

/* ******************************************************************
 *  FSE  (v0.2 private copy)
 ****************************************************************** */

type FSE_CTable = u32;
type FSE_DTable = u32;

const FSE_NCOUNTBOUND: usize = 512;

/* FSE_DTABLE_SIZE_U32(maxTableLog) == (1 + (1<<maxTableLog)) */
const fn FSE_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

const FSE_MAX_MEMORY_USAGE: u32 = 14;
const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;

const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

/* typedef U32 DTable_max_t[FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)]; */
const DTABLE_MAX_SIZE: usize = 1 + (1usize << 12);

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DState_t {
    state: usize,
    table: *const c_void, /* precise table may vary, depending on U16 */
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DTableHeader {
    tableLog: U16,
    fastMode: U16,
} /* sizeof U32 */

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_decode_t {
    newState: u16,
    symbol: u8,
    nbBits: u8,
} /* size == U32 */

unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
    dt: *const FSE_DTable,
) {
    let mut DTableH = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    memcpy(
        &mut DTableH as *mut FSE_DTableHeader as *mut c_void,
        dt as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    (*DStatePtr).state = BIT_readBits(bitD, DTableH.tableLog as U32);
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1)
        .wrapping_add(tableSize >> 3)
        .wrapping_add(3)
}

unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    let ptr = dt.wrapping_add(1) as *mut c_void;
    let tableDecode = ptr as *mut FSE_decode_t;
    let mut DTableH = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tableSize: U32 = 1u32 << tableLog;
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);
    let largeLimit: S16 = (1i32 << (tableLog.wrapping_sub(1))) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    /* Sanity Checks */
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    DTableH.tableLog = tableLog as U16;
    s = 0;
    while s <= maxSymbolValue {
        if *normalizedCounter.wrapping_add(s as usize) == -1 {
            (*tableDecode.wrapping_add(highThreshold as usize)).symbol = s as BYTE;
            highThreshold = highThreshold.wrapping_sub(1);
            symbolNext[s as usize] = 1;
        } else {
            if *normalizedCounter.wrapping_add(s as usize) >= largeLimit {
                noLarge = 0;
            }
            symbolNext[s as usize] = *normalizedCounter.wrapping_add(s as usize) as U16;
        }
        s = s.wrapping_add(1);
    }

    /* Spread symbols */
    s = 0;
    while s <= maxSymbolValue {
        let mut i: i32 = 0;
        while i < *normalizedCounter.wrapping_add(s as usize) as i32 {
            (*tableDecode.wrapping_add(position as usize)).symbol = s as BYTE;
            position = position.wrapping_add(step) & tableMask;
            while position > highThreshold {
                position = position.wrapping_add(step) & tableMask; /* lowprob area */
            }
            i += 1;
        }
        s = s.wrapping_add(1);
    }

    if position != 0 {
        return ERROR(ZSTD_error_GENERIC); /* position must reach all cells once */
    }

    /* Build Decoding table */
    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.wrapping_add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.wrapping_add(i as usize)).nbBits =
                tableLog.wrapping_sub(BIT_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.wrapping_add(i as usize)).newState = (((nextState as U32)
                << (*tableDecode.wrapping_add(i as usize)).nbBits)
                .wrapping_sub(tableSize)) as U16;
            i = i.wrapping_add(1);
        }
    }

    DTableH.fastMode = noLarge as U16;
    memcpy(
        dt as *mut c_void,
        &DTableH as *const FSE_DTableHeader as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    0
}

fn FSE_isError(code: usize) -> u32 {
    ERR_isError(code)
}

fn FSE_abs(a: i16) -> i16 {
    (if a < 0 { a.wrapping_neg() } else { a }) as i16
}

unsafe fn FSE_readNCount(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const BYTE;
    let iend = istart.wrapping_add(hbSize);
    let mut ip = istart;
    let mut nbBits: i32;
    let mut remaining: i32;
    let mut threshold: i32;
    let mut bitStream: U32;
    let mut bitCount: i32;
    let mut charnum: u32 = 0;
    let mut previous0: i32 = 0;

    if hbSize < 4 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    bitStream = MEM_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) as i32) + FSE_MIN_TABLELOG as i32; /* extract tableLog */
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as i32 {
        return ERROR(ZSTD_error_tableLog_tooLarge);
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
                n0 = n0.wrapping_add(24);
                if ip < iend.wrapping_sub(5) {
                    ip = ip.wrapping_add(2);
                    bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
                } else {
                    bitStream >>= 16;
                    bitCount += 16;
                }
            }
            while (bitStream & 3) == 3 {
                n0 = n0.wrapping_add(3);
                bitStream >>= 2;
                bitCount += 2;
            }
            n0 = n0.wrapping_add(bitStream & 3);
            bitCount += 2;
            if n0 > *maxSVPtr {
                return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
            }
            while charnum < n0 {
                *normalizedCounter.wrapping_add(charnum as usize) = 0;
                charnum = charnum.wrapping_add(1);
            }
            if (ip <= iend.wrapping_sub(7))
                || (ip.wrapping_offset((bitCount >> 3) as isize) <= iend.wrapping_sub(4))
            {
                ip = ip.wrapping_offset((bitCount >> 3) as isize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: i16 = ((2 * threshold - 1) - remaining) as i16;
            let mut count: i16;

            if (bitStream & ((threshold - 1) as U32)) < (max as U32) {
                count = (bitStream & ((threshold - 1) as U32)) as i16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & ((2 * threshold - 1) as U32)) as i16;
                if count as i32 >= threshold {
                    count = count.wrapping_sub(max);
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining -= FSE_abs(count) as i32;
            *normalizedCounter.wrapping_add(charnum as usize) = count;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as i32;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            {
                if (ip <= iend.wrapping_sub(7))
                    || (ip.wrapping_offset((bitCount >> 3) as isize) <= iend.wrapping_sub(4))
                {
                    ip = ip.wrapping_offset((bitCount >> 3) as isize);
                    bitCount &= 7;
                } else {
                    bitCount -= (8i64
                        * ((iend as isize)
                            .wrapping_sub(4)
                            .wrapping_sub(ip as isize) as i64))
                        as i32;
                    ip = iend.wrapping_sub(4);
                }
                bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    *maxSVPtr = charnum.wrapping_sub(1);

    ip = ip.wrapping_offset(((bitCount + 7) >> 3) as isize);
    if ((ip as usize).wrapping_sub(istart as usize)) > hbSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    (ip as usize).wrapping_sub(istart as usize)
}

unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let cell = (ptr as *mut FSE_decode_t).wrapping_add(1); /* because dt is unsigned */

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: u32) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let dinfo = (ptr as *mut FSE_decode_t).wrapping_add(1); /* because dt is unsigned */
    let tableSize: u32 = 1u32 << nbBits;
    let tableMask: u32 = tableSize.wrapping_sub(1);
    let maxSymbolValue: u32 = tableMask;
    let mut s: u32;

    /* Sanity checks */
    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC); /* min size */
    }

    /* Build Decoding Table */
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    s = 0;
    while s <= maxSymbolValue {
        (*dinfo.wrapping_add(s as usize)).newState = 0;
        (*dinfo.wrapping_add(s as usize)).symbol = s as BYTE;
        (*dinfo.wrapping_add(s as usize)).nbBits = nbBits as BYTE;
        s = s.wrapping_add(1);
    }

    0
}

unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: u32,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.wrapping_add(maxDstSize);
    let olimit = omax.wrapping_sub(3);

    let mut bitD = BIT_DStream_t {
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
    errorCode = BIT_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    macro_rules! FSE_GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSE_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSE_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    /* 4 symbols per loop.
       Note: FSE_MAX_TABLELOG*2+7 == 31 and FSE_MAX_TABLELOG*4+7 == 55, both
       <= sizeof(size_t)*8 == 64, so the intermediate reloads are compiled out. */
    while (BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit) {
        *op.wrapping_add(0) = FSE_GETSYMBOL!(&mut state1);
        *op.wrapping_add(1) = FSE_GETSYMBOL!(&mut state2);
        *op.wrapping_add(2) = FSE_GETSYMBOL!(&mut state1);
        *op.wrapping_add(3) = FSE_GETSYMBOL!(&mut state2);
        op = op.wrapping_add(4);
    }

    /* tail */
    loop {
        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0
                && (fast != 0 || FSE_endOfDState(&state1) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL!(&mut state1);
        op = op.wrapping_add(1);

        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0
                && (fast != 0 || FSE_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL!(&mut state2);
        op = op.wrapping_add(1);
    }

    /* end ? */
    if BIT_endOfDStream(&bitD) != 0
        && FSE_endOfDState(&state1) != 0
        && FSE_endOfDState(&state2) != 0
    {
        return (op as usize).wrapping_sub(ostart as usize);
    }

    if op == omax {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */
    }

    ERROR(ZSTD_error_corruption_detected)
}

unsafe fn FSE_decompress_usingDTable(
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
    mut cSrcSize: usize,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [i16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [U32; DTABLE_MAX_SIZE] = [0; DTABLE_MAX_SIZE];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE;
    let mut errorCode: usize;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
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
        return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
    }
    ip = ip.wrapping_add(errorCode);
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
    FSE_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const c_void,
        cSrcSize,
        dt.as_ptr(),
    )
}

/* ******************************************************************
 *  Huff0  (v0.2 private copy)
 ****************************************************************** */

const HUF_CTABLEBOUND: usize = 129;

/* HUF_DTABLE_SIZE(maxTableLog) == 1 + (1<<maxTableLog) */
const fn HUF_DTABLE_SIZE(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

fn HUF_isError(code: usize) -> u32 {
    ERR_isError(code)
}

const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16;
const HUF_MAX_TABLELOG: u32 = 12;
const HUF_DEFAULT_TABLELOG: u32 = HUF_MAX_TABLELOG;
const HUF_MAX_SYMBOL_VALUE: u32 = 255;

/* static allocation sizes derived from HUF_MAX_TABLELOG == 12 */
const HUF_DTABLEX2_SIZE: usize = 1 + (1usize << 12); /* 4097 U16 */
const HUF_DTABLEX4_SIZE: usize = 1 + (1usize << 12); /* 4097 U32 */
const HUF_DTABLEX6_SIZE: usize = (1 + (1usize << 12)) * 3 / 2; /* 6145 U32 */

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX2 {
    byte: BYTE,
    nbBits: BYTE,
} /* single-symbol decoding */

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX4 {
    sequence: U16,
    nbBits: BYTE,
    length: BYTE,
} /* double-symbols decoding */

#[repr(C)]
#[derive(Clone, Copy)]
struct sortedSymbol_t {
    symbol: BYTE,
    weight: BYTE,
}

unsafe fn HUF_readStats(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightTotal: U32;
    let tableLog: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: usize;
    let oSize: usize;
    let mut n: U32;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.wrapping_add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            static l: [i32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = l[iSize - 242] as usize;
            memset(huffWeight as *mut c_void, 1, hwSize);
            iSize = 0;
        } else {
            /* Incompressible */
            oSize = iSize - 127;
            iSize = (oSize + 1) / 2;
            if iSize + 1 > srcSize {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if oSize >= hwSize {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(1);
            n = 0;
            while (n as usize) < oSize {
                *huffWeight.wrapping_add(n as usize) =
                    *ip.wrapping_add((n / 2) as usize) >> 4;
                *huffWeight.wrapping_add((n + 1) as usize) =
                    *ip.wrapping_add((n / 2) as usize) & 15;
                n = n.wrapping_add(2);
            }
        }
    } else {
        /* header compressed with FSE (normal case) */
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        oSize = FSE_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.wrapping_add(1) as *const c_void,
            iSize,
        ); /* max (hwSize-1) values decoded, as last one is implied */
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankStats as *mut c_void,
        0,
        ((HUF_ABSOLUTEMAX_TABLELOG + 1) as usize) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if (*huffWeight.wrapping_add(n as usize) as U32) >= HUF_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        let w = *huffWeight.wrapping_add(n as usize) as usize;
        *rankStats.wrapping_add(w) = (*rankStats.wrapping_add(w)).wrapping_add(1);
        weightTotal = weightTotal.wrapping_add((1u32 << w) >> 1);
        n = n.wrapping_add(1);
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    tableLog = BIT_highbit32(weightTotal).wrapping_add(1);
    if tableLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_corruption_detected);
    }
    {
        let total: U32 = 1u32 << tableLog;
        let rest: U32 = total.wrapping_sub(weightTotal);
        let verif: U32 = 1u32 << BIT_highbit32(rest);
        let lastWeight: U32 = BIT_highbit32(rest).wrapping_add(1);
        if verif != rest {
            return ERROR(ZSTD_error_corruption_detected); /* last value must be a clean power of 2 */
        }
        *huffWeight.wrapping_add(oSize) = lastWeight as BYTE;
        *rankStats.wrapping_add(lastWeight as usize) =
            (*rankStats.wrapping_add(lastWeight as usize)).wrapping_add(1);
    }

    /* check tree construction validity */
    if (*rankStats.wrapping_add(1) < 2) || ((*rankStats.wrapping_add(1) & 1) != 0) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    *tableLogPtr = tableLog;
    iSize + 1
}

/**************************/
/* single-symbol decoding */
/**************************/

unsafe fn HUF_readDTableX2(DTable: *mut U16, src: *const c_void, srcSize: usize) -> usize {
    let mut huffWeight: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut tableLog: U32 = 0;
    let ip = src as *const BYTE;
    let mut iSize: usize = *ip.wrapping_add(0) as usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let ptr = DTable.wrapping_add(1) as *mut c_void;
    let dt = ptr as *mut HUF_DEltX2;

    iSize = HUF_readStats(
        huffWeight.as_mut_ptr(),
        (HUF_MAX_SYMBOL_VALUE + 1) as usize,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > *DTable.wrapping_add(0) as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable is too small */
    }
    *DTable.wrapping_add(0) = tableLog as U16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current: U32 = nextRankStart;
        nextRankStart = nextRankStart.wrapping_add(rankVal[n as usize] << (n - 1));
        rankVal[n as usize] = current;
        n = n.wrapping_add(1);
    }

    /* fill DTable */
    n = 0;
    while n < nbSymbols {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32 << w) >> 1;
        let mut i: U32;
        let mut D = HUF_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = (tableLog.wrapping_add(1).wrapping_sub(w)) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize].wrapping_add(length) {
            *dt.wrapping_add(i as usize) = D;
            i = i.wrapping_add(1);
        }
        rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
        n = n.wrapping_add(1);
    }

    iSize
}

unsafe fn HUF_decodeSymbolX2(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BIT_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.wrapping_add(val)).byte;
    BIT_skipBits(Dstream, (*dt.wrapping_add(val)).nbBits as U32);
    c
}

unsafe fn HUF_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;

    macro_rules! DEC0 {
        () => {{
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }};
    }

    /* up to 4 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(4)) {
        DEC0!(); /* _2 : MEM_64bits() */
        DEC0!(); /* _1 */
        DEC0!(); /* _2 */
        DEC0!(); /* _0 */
    }

    /* closer to the end */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd) {
        DEC0!();
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        DEC0!();
    }

    (pEnd as usize).wrapping_sub(pStart as usize)
}

unsafe fn HUF_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DEltX2).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0) as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = bitD1;
        let mut bitD3 = bitD1;
        let mut bitD4 = bitD1;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.wrapping_add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.wrapping_add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.wrapping_add(6); /* jumpTable */
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = ostart.wrapping_add(segmentSize);
        let opStart3 = opStart2.wrapping_add(segmentSize);
        let opStart4 = opStart3.wrapping_add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: U32;

        length4 = cSrcSize.wrapping_sub(
            length1
                .wrapping_add(length2)
                .wrapping_add(length3)
                .wrapping_add(6),
        );
        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        errorCode = BIT_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }

        macro_rules! DEC {
            ($p:ident, $bitD:expr) => {{
                *$p = HUF_decodeSymbolX2($bitD, dt, dtLog);
                $p = $p.wrapping_add(1);
            }};
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUF_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BIT_endOfDStream(&bitD1)
            & BIT_endOfDStream(&bitD2)
            & BIT_endOfDStream(&bitD3)
            & BIT_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
        dstSize
    }
}

unsafe fn HUF_decompress4X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [u16; HUF_DTABLEX2_SIZE] = [0; HUF_DTABLEX2_SIZE];
    DTable[0] = HUF_MAX_TABLELOG as u16;
    let mut ip = cSrc as *const BYTE;
    let errorCode: usize;

    errorCode = HUF_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    HUF_decompress4X2_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
}

/***************************/
/* double-symbols decoding */
/***************************/

unsafe fn HUF_fillDTableX4Level2(
    DTable: *mut HUF_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: i32,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    let mut DElt = HUF_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut s: U32;

    /* get pre-calculated rankVal */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: U32;
        let skipSize: U32 = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1;
        i = 0;
        while i < skipSize {
            *DTable.wrapping_add(i as usize) = DElt;
            i = i.wrapping_add(1);
        }
    }

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        /* note : sortedSymbols already skipped */
        let symbol: U32 = (*sortedSymbols.wrapping_add(s as usize)).symbol as U32;
        let weight: U32 = (*sortedSymbols.wrapping_add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let length: U32 = 1u32 << (sizeLog.wrapping_sub(nbBits));
        let start: U32 = rankVal[weight as usize];
        let mut i: U32 = start;
        let end: U32 = start.wrapping_add(length);

        MEM_writeLE16(
            &mut DElt.sequence as *mut U16 as *mut c_void,
            (baseSeq as U32).wrapping_add(symbol << 8) as U16,
        );
        DElt.nbBits = nbBits.wrapping_add(consumed) as BYTE;
        DElt.length = 2;
        loop {
            *DTable.wrapping_add(i as usize) = DElt;
            i = i.wrapping_add(1);
            if !(i < end) {
                break;
            }
        } /* since length >= 1 */

        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s = s.wrapping_add(1);
    }
}

/* typedef U32 rankVal_t[HUF_ABSOLUTEMAX_TABLELOG][HUF_ABSOLUTEMAX_TABLELOG + 1]; */
type rankVal_row_t = [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
type rankVal_t = [rankVal_row_t; HUF_ABSOLUTEMAX_TABLELOG as usize];

unsafe fn HUF_fillDTableX4(
    DTable: *mut HUF_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const rankVal_row_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: i32 = (nbBitsBaseline as i32).wrapping_sub(targetLog as i32); /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.wrapping_add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.wrapping_add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32 << (targetLog.wrapping_sub(nbBits));

        if targetLog.wrapping_sub(nbBits) >= minBits {
            /* enough room for a second symbol */
            let sortedRank: U32;
            let mut minWeight: i32 = (nbBits as i32).wrapping_add(scaleLog);
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.wrapping_add(minWeight as usize);
            HUF_fillDTableX4Level2(
                DTable.wrapping_add(start as usize),
                targetLog.wrapping_sub(nbBits),
                nbBits,
                rankValOrigin.wrapping_add(nbBits as usize) as *const U32,
                minWeight,
                sortedList.wrapping_add(sortedRank as usize),
                sortedListSize.wrapping_sub(sortedRank),
                nbBitsBaseline,
                symbol,
            );
        } else {
            let mut i: U32;
            let end: U32 = start.wrapping_add(length);
            let mut DElt = HUF_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };

            MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1;
            i = start;
            while i < end {
                *DTable.wrapping_add(i as usize) = DElt;
                i = i.wrapping_add(1);
            }
        }
        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s = s.wrapping_add(1);
    }
}

unsafe fn HUF_readDTableX4(DTable: *mut U32, src: *const c_void, srcSize: usize) -> usize {
    let mut weightList: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t {
            symbol: 0,
            weight: 0,
        }; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let rankStart = rankStart0.as_mut_ptr().wrapping_add(1);
    let mut rankVal: rankVal_t = [[0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
        HUF_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.wrapping_add(0);
    let ip = src as *const BYTE;
    let mut iSize: usize = *ip.wrapping_add(0) as usize;
    let ptr = DTable as *mut c_void;
    let dt = (ptr as *mut HUF_DEltX4).wrapping_add(1);

    if memLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUF_readStats(
        weightList.as_mut_ptr(),
        (HUF_MAX_SYMBOL_VALUE + 1) as usize,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > memLog {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable can't fit code depth */
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        if maxW == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
        maxW = maxW.wrapping_sub(1);
    }

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankStats[w as usize]);
            *rankStart.wrapping_add(w as usize) = current;
            w = w.wrapping_add(1);
        }
        *rankStart.wrapping_add(0) = nextRankStart; /* put all 0w symbols at the end of sorted list */
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = *rankStart.wrapping_add(w as usize);
            *rankStart.wrapping_add(w as usize) = r.wrapping_add(1);
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s = s.wrapping_add(1);
        }
        *rankStart.wrapping_add(0) = 0; /* forget 0w symbols; beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: i32 = (memLog as i32).wrapping_sub(tableLog as i32) - 1; /* tableLog <= memLog */
        let rankVal0 = rankVal[0].as_mut_ptr();
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankVal;
            nextRankVal = nextRankVal.wrapping_add(
                rankStats[w as usize] << (((w as i32).wrapping_add(rescale)) as u32 & 31),
            );
            *rankVal0.wrapping_add(w as usize) = current;
            w = w.wrapping_add(1);
        }
        consumed = minBits;
        while consumed <= memLog.wrapping_sub(minBits) {
            let rankValPtr = rankVal[consumed as usize].as_mut_ptr();
            w = 1;
            while w <= maxW {
                *rankValPtr.wrapping_add(w as usize) =
                    *rankVal0.wrapping_add(w as usize) >> (consumed & 31);
                w = w.wrapping_add(1);
            }
            consumed = consumed.wrapping_add(1);
        }
    }

    HUF_fillDTableX4(
        dt,
        memLog,
        sortedSymbol.as_ptr(),
        sizeOfSort,
        rankStart0.as_ptr(),
        rankVal.as_ptr(),
        maxW,
        tableLog.wrapping_add(1),
    );

    iSize
}

unsafe fn HUF_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 2);
    BIT_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    (*dt.wrapping_add(val)).length as U32
}

unsafe fn HUF_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 1);
    if (*dt.wrapping_add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    } else {
        if ((*DStream).bitsConsumed as usize) < (core::mem::size_of::<usize>() * 8) {
            BIT_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
            if (*DStream).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as u32;
                /* ugly hack; works only because it's the last symbol */
            }
        }
    }
    1
}

unsafe fn HUF_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;

    macro_rules! DEC0 {
        () => {{
            p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        }};
    }

    /* up to 8 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd.wrapping_sub(7)) {
        DEC0!();
        DEC0!();
        DEC0!();
        DEC0!();
    }

    /* closer to the end */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(2)) {
        DEC0!();
    }

    while p <= pEnd.wrapping_sub(2) {
        DEC0!(); /* no need to reload : reached the end of DStream */
    }

    if p < pEnd {
        p = p.wrapping_add(
            HUF_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
        );
    }

    (p as usize).wrapping_sub(pStart as usize)
}

unsafe fn HUF_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U32,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DEltX4).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0);
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = bitD1;
        let mut bitD3 = bitD1;
        let mut bitD4 = bitD1;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.wrapping_add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.wrapping_add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.wrapping_add(6); /* jumpTable */
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = ostart.wrapping_add(segmentSize);
        let opStart3 = opStart2.wrapping_add(segmentSize);
        let opStart4 = opStart3.wrapping_add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: U32;

        length4 = cSrcSize.wrapping_sub(
            length1
                .wrapping_add(length2)
                .wrapping_add(length3)
                .wrapping_add(6),
        );
        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        errorCode = BIT_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }

        macro_rules! DEC {
            ($p:ident, $bitD:expr) => {{
                $p = $p.wrapping_add(HUF_decodeSymbolX4(
                    $p as *mut c_void,
                    $bitD,
                    dt,
                    dtLog,
                ) as usize);
            }};
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUF_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BIT_endOfDStream(&bitD1)
            & BIT_endOfDStream(&bitD2)
            & BIT_endOfDStream(&bitD3)
            & BIT_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
        dstSize
    }
}

unsafe fn HUF_decompress4X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [u32; HUF_DTABLEX4_SIZE] = [0; HUF_DTABLEX4_SIZE];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize: usize = HUF_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X4_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
}

/**********************************/
/* quad-symbol decoding           */
/**********************************/

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DDescX6 {
    nbBits: BYTE,
    nbBytes: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
union HUF_DSeqX6 {
    byte: [BYTE; 4],
    sequence: U32,
}

/* recursive, up to level 3 */
unsafe fn HUF_fillDTableX6LevelN(
    DDescription: *mut HUF_DDescX6,
    DSequence: *mut HUF_DSeqX6,
    sizeLog: i32,
    rankValOrigin: *const rankVal_row_t,
    consumed: U32,
    minWeight: i32,
    maxWeight: U32,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    nbBitsBaseline: U32,
    mut baseSeq: HUF_DSeqX6,
    mut DDesc: HUF_DDescX6,
) {
    let scaleLog: i32 = (nbBitsBaseline as i32).wrapping_sub(sizeLog); /* note : targetLog >= (nbBitsBaseline-1), hence scaleLog <= 1 */
    let minBits: i32 = (nbBitsBaseline as i32).wrapping_sub(maxWeight as i32);
    let level: U32 = DDesc.nbBytes as U32;
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let symbolStartPos: U32;
    let mut s: U32;

    /* local rankVal, will be modified */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin.wrapping_add(consumed as usize) as *const c_void,
        core::mem::size_of::<[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: U32;
        let skipSize: U32 = rankVal[minWeight as usize];
        i = 0;
        while i < skipSize {
            *DSequence.wrapping_add(i as usize) = baseSeq;
            *DDescription.wrapping_add(i as usize) = DDesc;
            i = i.wrapping_add(1);
        }
    }

    /* fill DTable */
    DDesc.nbBytes = DDesc.nbBytes.wrapping_add(1);
    symbolStartPos = *rankStart.wrapping_add(minWeight as usize);
    s = symbolStartPos;
    while s < sortedListSize {
        let symbol: BYTE = (*sortedSymbols.wrapping_add(s as usize)).symbol;
        let weight: U32 = (*sortedSymbols.wrapping_add(s as usize)).weight as U32; /* >= 1 (sorted) */
        let nbBits: i32 = (nbBitsBaseline as i32).wrapping_sub(weight as i32); /* >= 1 (by construction) */
        let totalBits: i32 = (consumed as i32).wrapping_add(nbBits);
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32 << ((sizeLog.wrapping_sub(nbBits)) as u32 & 31);
        baseSeq.byte[level as usize] = symbol;
        DDesc.nbBits = totalBits as BYTE;

        if (level < 3) && (sizeLog.wrapping_sub(totalBits) >= minBits) {
            /* enough room for another symbol */
            let mut nextMinWeight: i32 = totalBits.wrapping_add(scaleLog);
            if nextMinWeight < 1 {
                nextMinWeight = 1;
            }
            HUF_fillDTableX6LevelN(
                DDescription.wrapping_add(start as usize),
                DSequence.wrapping_add(start as usize),
                sizeLog.wrapping_sub(nbBits),
                rankValOrigin,
                totalBits as U32,
                nextMinWeight,
                maxWeight,
                sortedSymbols,
                sortedListSize,
                rankStart,
                nbBitsBaseline,
                baseSeq,
                DDesc,
            ); /* recursive (max : level 3) */
        } else {
            let mut i: U32;
            let end: U32 = start.wrapping_add(length);
            i = start;
            while i < end {
                *DDescription.wrapping_add(i as usize) = DDesc;
                *DSequence.wrapping_add(i as usize) = baseSeq;
                i = i.wrapping_add(1);
            }
        }
        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s = s.wrapping_add(1);
    }
}

/* note : same preparation as X4 */
unsafe fn HUF_readDTableX6(DTable: *mut U32, src: *const c_void, srcSize: usize) -> usize {
    let mut weightList: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t {
            symbol: 0,
            weight: 0,
        }; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let rankStart = rankStart0.as_mut_ptr().wrapping_add(1);
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let mut rankVal: rankVal_t = [[0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
        HUF_ABSOLUTEMAX_TABLELOG as usize];
    let memLog: U32 = *DTable.wrapping_add(0);
    let ip = src as *const BYTE;
    let mut iSize: usize = *ip.wrapping_add(0) as usize;

    if memLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUF_readStats(
        weightList.as_mut_ptr(),
        (HUF_MAX_SYMBOL_VALUE + 1) as usize,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > memLog {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable is too small */
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        if maxW == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
        maxW = maxW.wrapping_sub(1);
    }

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankStats[w as usize]);
            *rankStart.wrapping_add(w as usize) = current;
            w = w.wrapping_add(1);
        }
        *rankStart.wrapping_add(0) = nextRankStart; /* put all 0w symbols at the end of sorted list */
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = *rankStart.wrapping_add(w as usize);
            *rankStart.wrapping_add(w as usize) = r.wrapping_add(1);
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s = s.wrapping_add(1);
        }
        *rankStart.wrapping_add(0) = 0; /* forget 0w symbols; beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: i32 = (memLog as i32).wrapping_sub(tableLog as i32) - 1; /* tableLog <= memLog */
        let rankVal0 = rankVal[0].as_mut_ptr();
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankVal;
            nextRankVal = nextRankVal.wrapping_add(
                rankStats[w as usize] << (((w as i32).wrapping_add(rescale)) as u32 & 31),
            );
            *rankVal0.wrapping_add(w as usize) = current;
            w = w.wrapping_add(1);
        }
        consumed = minBits;
        while consumed <= memLog.wrapping_sub(minBits) {
            let rankValPtr = rankVal[consumed as usize].as_mut_ptr();
            w = 1;
            while w <= maxW {
                *rankValPtr.wrapping_add(w as usize) =
                    *rankVal0.wrapping_add(w as usize) >> (consumed & 31);
                w = w.wrapping_add(1);
            }
            consumed = consumed.wrapping_add(1);
        }
    }

    /* fill tables */
    {
        let ptr = DTable.wrapping_add(1) as *mut c_void;
        let DDescription = ptr as *mut HUF_DDescX6;
        let dSeqStart = DTable.wrapping_add(1 + (1usize << (memLog - 1))) as *mut c_void;
        let DSequence = dSeqStart as *mut HUF_DSeqX6;
        let mut DSeq = HUF_DSeqX6 { sequence: 0 };
        let mut DDesc = HUF_DDescX6 {
            nbBits: 0,
            nbBytes: 0,
        };
        DSeq.sequence = 0;
        DDesc.nbBits = 0;
        DDesc.nbBytes = 0;
        HUF_fillDTableX6LevelN(
            DDescription,
            DSequence,
            memLog as i32,
            rankVal.as_ptr(),
            0,
            1,
            maxW,
            sortedSymbol.as_ptr(),
            sizeOfSort,
            rankStart0.as_ptr(),
            tableLog.wrapping_add(1),
            DSeq,
            DDesc,
        );
    }

    iSize
}

unsafe fn HUF_decodeSymbolX6(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dd: *const HUF_DDescX6,
    ds: *const HUF_DSeqX6,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(
        op,
        ds.wrapping_add(val) as *const c_void,
        core::mem::size_of::<HUF_DSeqX6>(),
    );
    BIT_skipBits(DStream, (*dd.wrapping_add(val)).nbBits as U32);
    (*dd.wrapping_add(val)).nbBytes as U32
}

unsafe fn HUF_decodeLastSymbolsX6(
    op: *mut c_void,
    maxL: U32,
    DStream: *mut BIT_DStream_t,
    dd: *const HUF_DDescX6,
    ds: *const HUF_DSeqX6,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    let length: U32 = (*dd.wrapping_add(val)).nbBytes as U32;
    if length <= maxL {
        memcpy(op, ds.wrapping_add(val) as *const c_void, length as usize);
        BIT_skipBits(DStream, (*dd.wrapping_add(val)).nbBits as U32);
        return length;
    }
    memcpy(op, ds.wrapping_add(val) as *const c_void, maxL as usize);
    if ((*DStream).bitsConsumed as usize) < (core::mem::size_of::<usize>() * 8) {
        BIT_skipBits(DStream, (*dd.wrapping_add(val)).nbBits as U32);
        if (*DStream).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
            (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as u32;
            /* ugly hack; works only because it's the last symbol */
        }
    }
    maxL
}

unsafe fn HUF_decodeStreamX6(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    DTable: *const U32,
    dtLog: U32,
) -> usize {
    let ddPtr = DTable.wrapping_add(1) as *const c_void;
    let dd = ddPtr as *const HUF_DDescX6;
    let dsPtr = DTable.wrapping_add(1 + (1usize << (dtLog.wrapping_sub(1) & 63))) as *const c_void;
    let ds = dsPtr as *const HUF_DSeqX6;
    let pStart = p;

    macro_rules! DEC0 {
        () => {{
            p = p.wrapping_add(
                HUF_decodeSymbolX6(p as *mut c_void, bitDPtr, dd, ds, dtLog) as usize,
            );
        }};
    }

    /* up to 16 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(16)) {
        DEC0!();
        DEC0!();
        DEC0!();
        DEC0!();
    }

    /* closer to the end, up to 4 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(4)) {
        DEC0!();
    }

    while p <= pEnd.wrapping_sub(4) {
        DEC0!(); /* no need to reload : reached the end of DStream */
    }

    while p < pEnd {
        p = p.wrapping_add(HUF_decodeLastSymbolsX6(
            p as *mut c_void,
            (pEnd as usize).wrapping_sub(p as usize) as U32,
            bitDPtr,
            dd,
            ds,
            dtLog,
        ) as usize);
    }

    (p as usize).wrapping_sub(pStart as usize)
}

unsafe fn HUF_decompress4X6_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U32,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);

        let dtLog: U32 = *DTable.wrapping_add(0);
        let ddPtr = DTable.wrapping_add(1) as *const c_void;
        let dd = ddPtr as *const HUF_DDescX6;
        let dsPtr =
            DTable.wrapping_add(1 + (1usize << (dtLog.wrapping_sub(1) & 63))) as *const c_void;
        let ds = dsPtr as *const HUF_DSeqX6;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = bitD1;
        let mut bitD3 = bitD1;
        let mut bitD4 = bitD1;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.wrapping_add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.wrapping_add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.wrapping_add(6); /* jumpTable */
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = ostart.wrapping_add(segmentSize);
        let opStart3 = opStart2.wrapping_add(segmentSize);
        let opStart4 = opStart3.wrapping_add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: U32;

        length4 = cSrcSize.wrapping_sub(
            length1
                .wrapping_add(length2)
                .wrapping_add(length3)
                .wrapping_add(6),
        );
        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        errorCode = BIT_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BIT_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUF_isError(errorCode) != 0 {
            return errorCode;
        }

        macro_rules! DEC {
            ($p:ident, $bitD:expr) => {{
                $p = $p.wrapping_add(HUF_decodeSymbolX6(
                    $p as *mut c_void,
                    $bitD,
                    dd,
                    ds,
                    dtLog,
                ) as usize);
            }};
        }

        /* 16-64 symbols per loop (4-16 symbols per stream) */
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (op3 <= opStart4)
            && (endSignal == BIT_DStream_unfinished)
            && (op4 <= oend.wrapping_sub(16))
        {
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUF_decodeStreamX6(op1, &mut bitD1, opStart2, DTable, dtLog);
        HUF_decodeStreamX6(op2, &mut bitD2, opStart3, DTable, dtLog);
        HUF_decodeStreamX6(op3, &mut bitD3, opStart4, DTable, dtLog);
        HUF_decodeStreamX6(op4, &mut bitD4, oend, DTable, dtLog);

        /* check */
        endSignal = BIT_endOfDStream(&bitD1)
            & BIT_endOfDStream(&bitD2)
            & BIT_endOfDStream(&bitD3)
            & BIT_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
        dstSize
    }
}

unsafe fn HUF_decompress4X6(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [u32; HUF_DTABLEX6_SIZE] = [0; HUF_DTABLEX6_SIZE];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize: usize = HUF_readDTableX6(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X6_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
}

/**********************************/
/* Generic decompression selector */
/**********************************/

#[repr(C)]
#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
}

const fn at(tableTime: U32, decode256Time: U32) -> algo_time_t {
    algo_time_t {
        tableTime,
        decode256Time,
    }
}

static algoTime: [[algo_time_t; 3]; 16] = [
    /* single, double, quad */
    [at(0, 0), at(1, 1), at(2, 2)],             /* Q==0 : impossible */
    [at(0, 0), at(1, 1), at(2, 2)],             /* Q==1 : impossible */
    [at(38, 130), at(1313, 74), at(2151, 38)],  /* Q == 2 : 12-18% */
    [at(448, 128), at(1353, 74), at(2238, 41)], /* Q == 3 : 18-25% */
    [at(556, 128), at(1353, 74), at(2238, 47)], /* Q == 4 : 25-32% */
    [at(714, 128), at(1418, 74), at(2436, 53)], /* Q == 5 : 32-38% */
    [at(883, 128), at(1437, 74), at(2464, 61)], /* Q == 6 : 38-44% */
    [at(897, 128), at(1515, 75), at(2622, 68)], /* Q == 7 : 44-50% */
    [at(926, 128), at(1613, 75), at(2730, 75)], /* Q == 8 : 50-56% */
    [at(947, 128), at(1729, 77), at(3359, 77)], /* Q == 9 : 56-62% */
    [at(1107, 128), at(2083, 81), at(4006, 84)], /* Q ==10 : 62-69% */
    [at(1177, 128), at(2379, 87), at(4785, 88)], /* Q ==11 : 69-75% */
    [at(1242, 128), at(2415, 93), at(5155, 84)], /* Q ==12 : 75-81% */
    [at(1349, 128), at(2644, 106), at(5260, 106)], /* Q ==13 : 81-87% */
    [at(1455, 128), at(2422, 124), at(4174, 124)], /* Q ==14 : 87-93% */
    [at(722, 128), at(1891, 145), at(1936, 146)], /* Q ==15 : 93-99% */
];

type decompressionAlgo = unsafe fn(*mut c_void, usize, *const c_void, usize) -> usize;

unsafe fn HUF_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    static decompress: [decompressionAlgo; 3] =
        [HUF_decompress4X2, HUF_decompress4X4, HUF_decompress4X6];
    /* estimate decompression time */
    let Q: U32;
    let D256: U32 = (dstSize >> 8) as U32;
    let mut Dtime: [U32; 3] = [0; 3];
    let mut algoNb: U32 = 0;
    let mut n: i32;

    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected); /* invalid */
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize; /* not compressed */
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as i32, dstSize);
        return dstSize; /* RLE */
    }

    /* decoder timing evaluation */
    Q = (cSrcSize.wrapping_mul(16) / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
    n = 0;
    while n < 3 {
        Dtime[n as usize] = algoTime[Q as usize][n as usize].tableTime.wrapping_add(
            algoTime[Q as usize][n as usize]
                .decode256Time
                .wrapping_mul(D256),
        );
        n += 1;
    }

    Dtime[1] = Dtime[1].wrapping_add(Dtime[1] >> 4);
    Dtime[2] = Dtime[2].wrapping_add(Dtime[2] >> 3); /* advantage to algorithms using less memory */

    if Dtime[1] < Dtime[0] {
        algoNb = 1;
    }
    if Dtime[2] < Dtime[algoNb as usize] {
        algoNb = 2;
    }

    (decompress[algoNb as usize])(dst, dstSize, cSrc, cSrcSize)
}

/* ******************************************************************
 *  zstd v0.2 - decompression
 ****************************************************************** */

const ZSTD_VERSION_MAJOR: u32 = 0;
const ZSTD_VERSION_MINOR: u32 = 2;
const ZSTD_VERSION_RELEASE: u32 = 2;
const ZSTD_VERSION_NUMBER: u32 =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;

const ZSTD_magicNumber: U32 = 0xFD2FB522; /* v0.2 (current) */

const ZSTD_MEMORY_USAGE: u32 = 17;
const HASH_LOG: u32 = ZSTD_MEMORY_USAGE - 2;
const HASH_TABLESIZE: u32 = 1 << HASH_LOG;
const HASH_MASK: u32 = HASH_TABLESIZE - 1;

const KNUTH: u32 = 2654435761;

const BIT7: u32 = 128;
const BIT6: u32 = 64;
const BIT5: u32 = 32;
const BIT4: u32 = 16;
const BIT1: u32 = 2;
const BIT0: u32 = 1;

const BLOCKSIZE: usize = 128 * (1 << 10); /* define, for static allocation */
const MIN_SEQUENCES_SIZE: usize = 2 /*seqNb*/ + 2 /*dumps*/ + 3 /*seqTables*/ + 1 /*bitStream*/;
const MIN_CBLOCK_SIZE: usize = 3 /*litCSize*/ + MIN_SEQUENCES_SIZE;
const IS_RAW: u32 = BIT0;
const IS_RLE: u32 = BIT1;

const WORKPLACESIZE: usize = BLOCKSIZE * 3;
const MINMATCH: usize = 4;
const MLbits: u32 = 7;
const LLbits: u32 = 6;
const Offbits: u32 = 5;
const MaxML: u32 = (1 << MLbits) - 1;
const MaxLL: u32 = (1 << LLbits) - 1;
const MaxOff: u32 = 31;
const LitFSELog: u32 = 11;
const MLFSELog: u32 = 10;
const LLFSELog: u32 = 10;
const OffFSELog: u32 = 9;
const MaxSeq: u32 = if MaxLL < MaxML { MaxML } else { MaxLL };

const LITERAL_NOENTROPY: u32 = 63;
const COMMAND_NOENTROPY: u32 = 7; /* to remove */

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

static ZSTD_blockHeaderSize: usize = 3;
static ZSTD_frameHeaderSize: usize = 4;

/* *******************************************************
*  Memory operations
**********************************************************/
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

/* ZSTD_wildcopy : custom version of memcpy(), can copy up to 7-8 bytes too many */
unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.wrapping_offset(length);
    loop {
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
        if !(op < oend) {
            break;
        }
    }
}

/* **************************************
*  Local structures
****************************************/
type blockType_t = u32;
const bt_compressed: blockType_t = 0;
const bt_raw: blockType_t = 1;
const bt_rle: blockType_t = 2;
const bt_end: blockType_t = 3;

#[repr(C)]
#[derive(Clone, Copy)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SeqStore_t {
    buffer: *mut c_void,
    offsetStart: *mut U32,
    offset: *mut U32,
    offCodeStart: *mut BYTE,
    offCode: *mut BYTE,
    litStart: *mut BYTE,
    lit: *mut BYTE,
    litLengthStart: *mut BYTE,
    litLength: *mut BYTE,
    matchLengthStart: *mut BYTE,
    matchLength: *mut BYTE,
    dumpsStart: *mut BYTE,
    dumps: *mut BYTE,
}

/* *************************************
*  Error Management
***************************************/
fn ZSTD_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/* *************************************************************
*   Decompression section
***************************************************************/

#[repr(C)]
pub struct ZSTDv02_Dctx_s {
    LLTable: [U32; 1 + (1 << LLFSELog)],
    OffTable: [U32; 1 + (1 << OffFSELog)],
    MLTable: [U32; 1 + (1 << MLFSELog)],
    previousDstEnd: *mut c_void,
    base: *mut c_void,
    expected: usize,
    bType: blockType_t,
    phase: U32,
    litPtr: *const BYTE,
    litSize: usize,
    litBuffer: [BYTE; BLOCKSIZE + 8 /* margin for wildcopy */],
}

type ZSTD_DCtx = ZSTDv02_Dctx_s;
pub type ZSTDv02_Dctx = ZSTDv02_Dctx_s;

unsafe fn ZSTD_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_: *const BYTE = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    headerFlags = *in_;
    cSize = (*in_.wrapping_add(2) as U32)
        .wrapping_add((*in_.wrapping_add(1) as U32) << 8)
        .wrapping_add(((*in_.wrapping_add(0) as U32) & 7) << 16);

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

unsafe fn ZSTD_copyUncompressedBlock(
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

/** ZSTD_decompressLiterals
    @return : nb of bytes read from src, or an error code */
unsafe fn ZSTD_decompressLiterals(
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip = src as *const BYTE;

    let litSize: usize = ((MEM_readLE32(src) & 0x1FFFFF) >> 2) as usize; /* no buffer issue : srcSize >= MIN_CBLOCK_SIZE */
    let litCSize: usize =
        ((MEM_readLE32(ip.wrapping_add(2) as *const c_void) & 0xFFFFFF) >> 5) as usize; /* no buffer issue : srcSize >= MIN_CBLOCK_SIZE */

    if litSize > *maxDstSizePtr {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if litCSize.wrapping_add(5) > srcSize {
        return ERROR(ZSTD_error_corruption_detected);
    }

    if HUF_isError(HUF_decompress(
        dst,
        litSize,
        ip.wrapping_add(5) as *const c_void,
        litCSize,
    )) != 0
    {
        return ERROR(ZSTD_error_corruption_detected);
    }

    *maxDstSizePtr = litSize;
    litCSize.wrapping_add(5)
}

/** ZSTD_decodeLiteralsBlock
    @return : nb of bytes read from src (< srcSize ) */
unsafe fn ZSTD_decodeLiteralsBlock(ctx: *mut c_void, src: *const c_void, srcSize: usize) -> usize {
    let dctx = ctx as *mut ZSTD_DCtx;
    let istart: *const BYTE = src as *const BYTE;

    /* any compressed block with literals segment must be at least this size */
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart as u32) & 3 {
        x if x == IS_RAW => {
            let litSize: usize = ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as usize; /* no buffer issue : srcSize >= MIN_CBLOCK_SIZE */
            if litSize > srcSize.wrapping_sub(11) {
                /* risk of reading too far with wildcopy */
                if litSize > BLOCKSIZE {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                if litSize > srcSize.wrapping_sub(3) {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                memcpy(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    istart as *const c_void,
                    litSize,
                );
                (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
                (*dctx).litSize = litSize;
                memset(
                    (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                    0,
                    8,
                );
                return litSize.wrapping_add(3);
            }
            /* direct reference into compressed stream */
            (*dctx).litPtr = istart.wrapping_add(3);
            (*dctx).litSize = litSize;
            litSize.wrapping_add(3)
        }
        x if x == IS_RLE => {
            let litSize: usize = ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as usize; /* no buffer issue : srcSize >= MIN_CBLOCK_SIZE */
            if litSize > BLOCKSIZE {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.wrapping_add(3) as i32,
                litSize.wrapping_add(8),
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            4
        }
        /* default: and case 0: */
        _ => {
            let mut litSize: usize = BLOCKSIZE;
            let readSize: usize = ZSTD_decompressLiterals(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                &mut litSize,
                src,
                srcSize,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                8,
            );
            readSize /* works if it's an error too */
        }
    }
}

unsafe fn ZSTD_decodeSeqHeaders(
    nbSeq: *mut i32,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut FSE_DTable,
    DTableML: *mut FSE_DTable,
    DTableOffb: *mut FSE_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: U32 = 0;
    let mut Offlog: U32 = 0;
    let mut MLlog: U32 = 0;
    let dumpsLength: usize;

    /* check */
    if srcSize < 5 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    *nbSeq = MEM_readLE16(ip as *const c_void) as i32;
    ip = ip.wrapping_add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = (*ip.wrapping_add(2) as usize)
            .wrapping_add((*ip.wrapping_add(1) as usize) << 8);
        ip = ip.wrapping_add(3);
    } else {
        dumpsLength = (*ip.wrapping_add(1) as usize)
            .wrapping_add(((*ip.wrapping_add(0) as usize) & 1) << 8);
        ip = ip.wrapping_add(2);
    }
    *dumpsPtr = ip;
    ip = ip.wrapping_add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    /* check */
    if ip > iend.wrapping_sub(3) {
        return ERROR(ZSTD_error_srcSize_wrong); /* min : all 3 are "raw" */
    }

    /* sequences */
    {
        let mut norm: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize]; /* assumption : MaxML >= MaxLL and MaxOff */
        let mut headerSize: usize;

        /* Build DTables */
        if LLtype == bt_rle {
            LLlog = 0;
            FSE_buildDTable_rle(DTableLL, *ip);
            ip = ip.wrapping_add(1);
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
                (iend as usize).wrapping_sub(ip as usize),
            );
            if FSE_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if LLlog > LLFSELog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(headerSize);
            FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
        }

        if Offtype == bt_rle {
            Offlog = 0;
            if ip > iend.wrapping_sub(2) {
                return ERROR(ZSTD_error_srcSize_wrong); /* min : "raw", hence no header */
            }
            FSE_buildDTable_rle(DTableOffb, *ip & (MaxOff as BYTE)); /* if *ip > MaxOff, data is corrupted */
            ip = ip.wrapping_add(1);
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
                (iend as usize).wrapping_sub(ip as usize),
            );
            if FSE_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if Offlog > OffFSELog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(headerSize);
            FSE_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
        }

        if MLtype == bt_rle {
            MLlog = 0;
            if ip > iend.wrapping_sub(2) {
                return ERROR(ZSTD_error_srcSize_wrong); /* min : "raw", hence no header */
            }
            FSE_buildDTable_rle(DTableML, *ip);
            ip = ip.wrapping_add(1);
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
                (iend as usize).wrapping_sub(ip as usize),
            );
            if FSE_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if MLlog > MLFSELog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(headerSize);
            FSE_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
        }
    }

    (ip as usize).wrapping_sub(istart as usize)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct seq_t {
    litLength: usize,
    offset: usize,
    matchLength: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct seqState_t {
    DStream: BIT_DStream_t,
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
    let mut dumps: *const BYTE = (*seqState).dumps;
    let de: *const BYTE = (*seqState).dumpsEnd;

    /* Literal length */
    litLength = FSE_decodeSymbol(
        &mut (*seqState).stateLL,
        &mut (*seqState).DStream,
    ) as usize;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    (*seqState).prevOffset = (*seq).offset;
    if litLength == MaxLL as usize {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.wrapping_add(1);
            v
        } else {
            0
        };
        if add < 255 {
            litLength = litLength.wrapping_add(add as usize);
        } else if dumps.wrapping_add(3) <= de {
            litLength = MEM_readLE24(dumps as *const c_void) as usize;
            dumps = dumps.wrapping_add(3);
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1); /* late correction, to avoid read overflow */
        }
    }

    /* Offset */
    {
        static offsetPrefix: [usize; (MaxOff + 1) as usize] = [
            /* note : size_t faster than U32 */
            1, /*fake*/ 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384,
            32768, 65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216,
            33554432, /*fake*/ 1, 1, 1, 1, 1,
        ];
        let offsetCode: U32;
        let mut nbBits: U32;
        offsetCode = FSE_decodeSymbol(
            &mut (*seqState).stateOffb,
            &mut (*seqState).DStream,
        ) as U32; /* <= maxOff, by table construction */
        /* MEM_32bits() == 0 : no reload */
        nbBits = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0; /* cmove */
        }
        offset = offsetPrefix[offsetCode as usize]
            .wrapping_add(BIT_readBits(&mut (*seqState).DStream, nbBits));
        /* MEM_32bits() == 0 : no reload */
        if offsetCode == 0 {
            offset = prevOffset; /* cmove */
        }
    }

    /* MatchLength */
    matchLength = FSE_decodeSymbol(
        &mut (*seqState).stateML,
        &mut (*seqState).DStream,
    ) as usize;
    if matchLength == MaxML as usize {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.wrapping_add(1);
            v
        } else {
            0
        };
        if add < 255 {
            matchLength = matchLength.wrapping_add(add as usize);
        } else if dumps.wrapping_add(3) <= de {
            matchLength = MEM_readLE24(dumps as *const c_void) as usize;
            dumps = dumps.wrapping_add(3);
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1); /* late correction, to avoid read overflow */
        }
    }
    matchLength = matchLength.wrapping_add(MINMATCH);

    /* save result */
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> usize {
    static dec32table: [i32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
    static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */
    let ostart: *const BYTE = op;
    let oLitEnd: *mut BYTE = op.wrapping_add(sequence.litLength);
    let oMatchEnd: *mut BYTE = op
        .wrapping_add(sequence.litLength)
        .wrapping_add(sequence.matchLength); /* risk : address space overflow (32-bits) */
    let oend_8: *mut BYTE = oend.wrapping_sub(8);
    let litEnd: *const BYTE = (*litPtr).wrapping_add(sequence.litLength);

    /* checks */
    let seqLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > ((oend as usize).wrapping_sub(op as usize)) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > ((litLimit as usize).wrapping_sub(*litPtr as usize)) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Now we know there are no overflow in literal nor match lengths, can use the pointer check */
    if oLitEnd > oend_8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.offset
        > (((oLitEnd as isize).wrapping_sub(base as isize)) as U32) as usize
    {
        return ERROR(ZSTD_error_corruption_detected);
    }

    if oMatchEnd > oend {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* overwrite beyond dst buffer */
    }
    if litEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected); /* overRead beyond lit buffer */
    }

    /* copy Literals */
    ZSTD_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    ); /* note : oLitEnd <= oend-8 : no risk of overwrite beyond oend */
    op = oLitEnd;
    *litPtr = litEnd; /* update for next sequence */

    /* copy Match */
    {
        let mut match_: *const BYTE = op.wrapping_sub(sequence.offset);

        /* check */
        if sequence.offset > (op as usize) {
            return ERROR(ZSTD_error_corruption_detected); /* address space overflow test */
        }
        if match_ < base {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* close range match, overlap */
        if sequence.offset < 8 {
            let dec64: i32 = dec64table[sequence.offset];
            *op.wrapping_add(0) = *match_.wrapping_add(0);
            *op.wrapping_add(1) = *match_.wrapping_add(1);
            *op.wrapping_add(2) = *match_.wrapping_add(2);
            *op.wrapping_add(3) = *match_.wrapping_add(3);
            match_ = match_.wrapping_offset(dec32table[sequence.offset] as isize);
            ZSTD_copy4(
                op.wrapping_add(4) as *mut c_void,
                match_ as *const c_void,
            );
            match_ = match_.wrapping_offset(-(dec64 as isize));
        } else {
            ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
        }
        op = op.wrapping_add(8);
        match_ = match_.wrapping_add(8);

        if oMatchEnd > oend.wrapping_sub(16 - MINMATCH) {
            if op < oend_8 {
                ZSTD_wildcopy(
                    op as *mut c_void,
                    match_ as *const c_void,
                    (oend_8 as isize).wrapping_sub(op as isize),
                );
                match_ = match_
                    .wrapping_offset((oend_8 as isize).wrapping_sub(op as isize));
                op = oend_8;
            }
            while op < oMatchEnd {
                *op = *match_;
                op = op.wrapping_add(1);
                match_ = match_.wrapping_add(1);
            }
        } else {
            ZSTD_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                (sequence.matchLength as isize).wrapping_sub(8),
            ); /* works even if matchLength < 8 */
        }
    }

    (oMatchEnd as usize).wrapping_sub(ostart as usize)
}

unsafe fn ZSTD_decompressSequences(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
) -> usize {
    let dctx = ctx as *mut ZSTD_DCtx;
    let mut ip = seqStart as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(seqSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut errorCode: usize;
    let mut dumpsLength: usize = 0;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.wrapping_add((*dctx).litSize);
    let mut nbSeq: i32 = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL: *mut U32 = (*dctx).LLTable.as_mut_ptr();
    let DTableML: *mut U32 = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb: *mut U32 = (*dctx).OffTable.as_mut_ptr();
    let base: *mut BYTE = (*dctx).base as *mut BYTE;

    /* Build Decoding Tables */
    errorCode = ZSTD_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        (iend as usize).wrapping_sub(ip as usize),
    );
    if ZSTD_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.wrapping_add(errorCode);

    /* Regen sequences */
    {
        let mut sequence = seq_t {
            litLength: 0,
            offset: 0,
            matchLength: 0,
        };
        let mut seqState = seqState_t {
            DStream: BIT_DStream_t {
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

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.wrapping_add(dumpsLength);
        seqState.prevOffset = 1;
        errorCode = BIT_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
        );
        if ERR_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSE_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSE_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSE_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BIT_reloadDStream(&mut seqState.DStream) <= BIT_DStream_completed) && (nbSeq > 0) {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(op, sequence, &mut litPtr, litEnd, base, oend);
            if ZSTD_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.wrapping_add(oneSeqSize);
        }

        /* check if reached exact end */
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected); /* requested too much : data is corrupted */
        }
        if nbSeq < 0 {
            return ERROR(ZSTD_error_corruption_detected); /* requested too many sequences */
        }

        /* last literal segment */
        {
            let lastLLSize: usize = (litEnd as usize).wrapping_sub(litPtr as usize);
            if litPtr > litEnd {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if op.wrapping_add(lastLLSize) > oend {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            if lastLLSize > 0 {
                if op as *const BYTE != litPtr {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.wrapping_add(lastLLSize);
            }
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip = src as *const BYTE;

    /* Decode literals sub-block */
    let litCSize: usize = ZSTD_decodeLiteralsBlock(ctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.wrapping_add(litCSize);
    srcSize = srcSize.wrapping_sub(litCSize);

    ZSTD_decompressSequences(ctx, dst, maxDstSize, ip as *const c_void, srcSize)
}

unsafe fn ZSTD_decompressDCtx(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut remainingSize: usize = srcSize;
    let magicNumber: U32;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_magicNumber {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    ip = ip.wrapping_add(ZSTD_frameHeaderSize);
    remainingSize = remainingSize.wrapping_sub(ZSTD_frameHeaderSize);

    /* Loop on each block */
    loop {
        let mut decodedSize: usize = 0;
        let cBlockSize: usize = ZSTD_getcBlockSize(
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
            &mut blockProperties,
        );
        if ZSTD_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.wrapping_add(ZSTD_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTD_blockHeaderSize);
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                decodedSize = ZSTD_decompressBlock(
                    ctx,
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_raw => {
                decodedSize = ZSTD_copyUncompressedBlock(
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_rle => {
                return ERROR(ZSTD_error_GENERIC); /* not yet supported */
            }
            x if x == bt_end => {
                /* end of frame */
                if remainingSize != 0 {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC); /* impossible */
            }
        }
        if cBlockSize == 0 {
            break; /* bt_end */
        }

        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.wrapping_add(decodedSize);
        ip = ip.wrapping_add(cBlockSize);
        remainingSize = remainingSize.wrapping_sub(cBlockSize);
    }

    (op as usize).wrapping_sub(ostart as usize)
}

unsafe fn ZSTD_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ctx_storage = core::mem::MaybeUninit::<ZSTD_DCtx>::uninit();
    let ctx = ctx_storage.as_mut_ptr();
    (*ctx).base = dst;
    ZSTD_decompressDCtx(ctx as *mut c_void, dst, maxDstSize, src, srcSize)
}

/* ZSTD_errorFrameSizeInfoLegacy() :
   assumes `cSize` and `dBound` are _not_ NULL */
unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut u64, ret: usize) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;
    let magicNumber: U32;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_magicNumber {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
        return;
    }
    ip = ip.wrapping_add(ZSTD_frameHeaderSize);
    remainingSize = remainingSize.wrapping_sub(ZSTD_frameHeaderSize);

    /* Loop on each block */
    loop {
        let cBlockSize: usize =
            ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTD_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTD_blockHeaderSize);
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break; /* bt_end */
        }

        ip = ip.wrapping_add(cBlockSize);
        remainingSize = remainingSize.wrapping_sub(cBlockSize);
        nbBlocks = nbBlocks.wrapping_add(1);
    }

    *cSize = (ip as usize).wrapping_sub(src as usize);
    *dBound = (nbBlocks as u64).wrapping_mul(BLOCKSIZE as u64);
}

/*******************************
*  Streaming Decompression API
*******************************/

unsafe fn ZSTD_resetDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0;
    (*dctx).previousDstEnd = core::ptr::null_mut();
    (*dctx).base = core::ptr::null_mut();
    0
}

unsafe fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    let dctx = malloc(core::mem::size_of::<ZSTD_DCtx>()) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_resetDCtx(dctx);
    dctx
}

unsafe fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    free(dctx as *mut c_void);
    0
}

unsafe fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected
}

unsafe fn ZSTD_decompressContinue(
    ctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* Sanity check */
    if srcSize != (*ctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dst != (*ctx).previousDstEnd {
        /* not contiguous */
        (*ctx).base = dst;
    }

    /* Decompress : frame header */
    if (*ctx).phase == 0 {
        /* Check frame magic header */
        let magicNumber: U32 = MEM_readLE32(src);
        if magicNumber != ZSTD_magicNumber {
            return ERROR(ZSTD_error_prefix_unknown);
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        return 0;
    }

    /* Decompress : block header */
    if (*ctx).phase == 1 {
        let mut bp = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let blockSize: usize = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
        if ZSTD_isError(blockSize) != 0 {
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

    /* Decompress : block content */
    {
        let rSize: usize;
        match (*ctx).bType {
            x if x == bt_compressed => {
                rSize = ZSTD_decompressBlock(ctx as *mut c_void, dst, maxDstSize, src, srcSize);
            }
            x if x == bt_raw => {
                rSize = ZSTD_copyUncompressedBlock(dst, maxDstSize, src, srcSize);
            }
            x if x == bt_rle => {
                return ERROR(ZSTD_error_GENERIC); /* not yet handled */
            }
            x if x == bt_end => {
                /* should never happen (filtered at phase 1) */
                rSize = 0;
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTD_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = (dst as *mut i8).wrapping_add(rSize) as *mut c_void;
        rSize
    }
}

/* wrapper layer */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_isError(code: usize) -> u32 {
    ZSTD_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_decompress(
    dst: *mut c_void,
    maxOriginalSize: usize,
    src: *const c_void,
    compressedSize: usize,
) -> usize {
    ZSTD_decompress(dst, maxOriginalSize, src, compressedSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_createDCtx() -> *mut ZSTDv02_Dctx {
    ZSTD_createDCtx() as *mut ZSTDv02_Dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_freeDCtx(dctx: *mut ZSTDv02_Dctx) -> usize {
    ZSTD_freeDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_resetDCtx(dctx: *mut ZSTDv02_Dctx) -> usize {
    ZSTD_resetDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_nextSrcSizeToDecompress(dctx: *mut ZSTDv02_Dctx) -> usize {
    ZSTD_nextSrcSizeToDecompress(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_decompressContinue(
    dctx: *mut ZSTDv02_Dctx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompressContinue(dctx as *mut ZSTD_DCtx, dst, maxDstSize, src, srcSize)
}
