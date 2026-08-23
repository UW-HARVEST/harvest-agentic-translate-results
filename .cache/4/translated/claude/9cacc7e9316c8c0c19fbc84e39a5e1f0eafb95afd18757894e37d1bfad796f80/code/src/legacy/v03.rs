//! Transliteration of `legacy/zstd_v03.c` + `legacy/zstd_v03.h`.
//!
//! This is a fully self-contained legacy (v0.3) decoder: it carries its own
//! copies of the `mem.h` / `bitstream.h` / FSE / Huff0 helpers, exactly as the
//! C file does.  The only shared pieces are the libc bindings and
//! `error_private.h` (which the C file `#include`s, and whose `ERROR_H_MODULE`
//! guard causes the file's own local error block to be skipped).
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens,
    unused_imports
)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::error_private::*;
use crate::mem::{calloc, free, malloc, memcpy, memmove, memset, qsort};

/* ******************************************************************
 *  mem.h -- basic types
 ****************************************************************** */

pub type BYTE = u8;
pub type U8 = u8;
pub type S8 = i8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

/* ******************************************************************
 *  mem.h -- memory I/O
 ****************************************************************** */

pub fn MEM_32bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 4) as c_uint
}

pub fn MEM_64bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 8) as c_uint
}

pub fn MEM_isLittleEndian() -> c_uint {
    if cfg!(target_endian = "little") {
        1
    } else {
        0
    }
}

pub unsafe fn MEM_read16(memPtr: *const c_void) -> U16 {
    core::ptr::read_unaligned(memPtr as *const U16)
}

pub unsafe fn MEM_read32(memPtr: *const c_void) -> U32 {
    core::ptr::read_unaligned(memPtr as *const U32)
}

pub unsafe fn MEM_read64(memPtr: *const c_void) -> U64 {
    core::ptr::read_unaligned(memPtr as *const U64)
}

pub unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    core::ptr::write_unaligned(memPtr as *mut U16, value)
}

pub unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        MEM_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}

pub unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let p = memPtr as *mut BYTE;
        *p.add(0) = val as BYTE;
        *p.add(1) = (val >> 8) as BYTE;
    }
}

pub unsafe fn MEM_readLE24(memPtr: *const c_void) -> U32 {
    (MEM_readLE16(memPtr) as U32).wrapping_add((*(memPtr as *const BYTE).add(2) as U32) << 16)
}

pub unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
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

pub unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
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

pub unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    if MEM_32bits() != 0 {
        MEM_readLE32(memPtr) as usize
    } else {
        MEM_readLE64(memPtr) as usize
    }
}

/* ******************************************************************
 *  bitstream.h
 ****************************************************************** */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BIT_DStream_t {
    pub bitContainer: usize,
    pub bitsConsumed: c_uint,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

pub type BIT_DStream_status = c_uint;
pub const BIT_DStream_unfinished: BIT_DStream_status = 0;
pub const BIT_DStream_endOfBuffer: BIT_DStream_status = 1;
pub const BIT_DStream_completed: BIT_DStream_status = 2;
pub const BIT_DStream_overflow: BIT_DStream_status = 3;

pub fn BIT_highbit32(val: U32) -> c_uint {
    /* __builtin_clz (val) ^ 31 */
    val.leading_zeros() ^ 31
}

/* !BIT_initDStream
 *  Initialize a BIT_DStream_t. */
pub unsafe fn BIT_initDStream(
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
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char)
            .wrapping_add(srcSize)
            .wrapping_sub(core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); /* endMark not present */
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let s = (*bitD).start as *const BYTE;
        match srcSize {
            7 => {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*s.add(6) as usize) << (core::mem::size_of::<usize>() * 8 - 16));
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*s.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24));
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*s.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32));
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(3) as usize) << 24);
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(2) as usize) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*s.add(1) as usize) << 8);
            }
            6 => {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*s.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24));
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*s.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32));
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(3) as usize) << 24);
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(2) as usize) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*s.add(1) as usize) << 8);
            }
            5 => {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*s.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32));
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(3) as usize) << 24);
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(2) as usize) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*s.add(1) as usize) << 8);
            }
            4 => {
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(3) as usize) << 24);
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(2) as usize) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*s.add(1) as usize) << 8);
            }
            3 => {
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add((*s.add(2) as usize) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*s.add(1) as usize) << 8);
            }
            2 => {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*s.add(1) as usize) << 8);
            }
            _ => {}
        }
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
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

pub unsafe fn BIT_lookBits(bitD: *const BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask.wrapping_sub(nbBits)) & bitMask)
}

/* ! BIT_lookBitsFast : unsafe version; only works if nbBits >= 1 */
pub unsafe fn BIT_lookBitsFast(bitD: *const BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask.wrapping_add(1)).wrapping_sub(nbBits)) & bitMask)
}

pub unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

pub unsafe fn BIT_readBits(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

/* !BIT_readBitsFast : unsafe version; only works if nbBits >= 1 */
pub unsafe fn BIT_readBitsFast(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

pub unsafe fn BIT_reloadDStream(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as c_uint {
        /* should never happen */
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD)
            .ptr
            .wrapping_offset(-(((*bitD).bitsConsumed >> 3) as isize));
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as c_uint {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BIT_DStream_status = BIT_DStream_unfinished;
        if (*bitD).ptr.wrapping_offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize).wrapping_sub((*bitD).start as usize) as U32; /* ptr > start */
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_offset(-(nbBytes as isize));
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return result;
    }
}

/* ! BIT_endOfDStream : @return Tells if DStream has reached its exact end */
pub unsafe fn BIT_endOfDStream(DStream: *const BIT_DStream_t) -> c_uint {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as c_uint))
        as c_uint
}

/* ******************************************************************
 *  FSE_static -- types & inline decoding helpers
 ****************************************************************** */

pub type FSE_CTable = c_uint;
pub type FSE_DTable = c_uint;

/* FSE buffer bounds */
pub const FSE_NCOUNTBOUND: usize = 512;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSE_DState_t {
    pub state: usize,
    pub table: *const c_void, /* precise table may vary, depending on U16 */
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSE_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
} /* sizeof U32 */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSE_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
} /* size == U32 */

pub unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
    dt: *const FSE_DTable,
) {
    let mut DTableH: FSE_DTableHeader = FSE_DTableHeader {
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
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

pub unsafe fn FSE_decodeSymbol(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = BIT_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSE_decodeSymbolFast(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = BIT_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> c_uint {
    ((*DStatePtr).state == 0) as c_uint
}

/* ******************************************************************
 *  FSE : Finite State Entropy coder -- constants
 ****************************************************************** */

pub const FSE_MAX_MEMORY_USAGE: u32 = 14;
pub const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSE_MAX_SYMBOL_VALUE: c_uint = 255;

pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2; /* 12 */
pub const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
pub const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
pub const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
pub const FSE_MIN_TABLELOG: c_int = 5;
pub const FSE_TABLELOG_ABSOLUTE_MAX: c_int = 15;

/* FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG) == 1 + (1<<12) */
pub const DTable_max_t_SIZE: usize = 1 + (1usize << FSE_MAX_TABLELOG);

pub fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1)
        .wrapping_add(tableSize >> 3)
        .wrapping_add(3)
}

pub unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let ptr: *mut c_void = dt.add(1) as *mut c_void;
    let mut DTableH: FSE_DTableHeader = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tableDecode: *mut FSE_decode_t = ptr as *mut FSE_decode_t;
    let tableSize: U32 = 1i32.wrapping_shl(tableLog) as U32;
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);
    let largeLimit: S16 = 1i32.wrapping_shl(tableLog.wrapping_sub(1)) as S16;
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
        if *normalizedCounter.offset(s as isize) == -1 {
            (*tableDecode.offset(highThreshold as isize)).symbol = s as BYTE;
            highThreshold = highThreshold.wrapping_sub(1);
            symbolNext[s as usize] = 1;
        } else {
            if *normalizedCounter.offset(s as isize) >= largeLimit {
                noLarge = 0;
            }
            symbolNext[s as usize] = *normalizedCounter.offset(s as isize) as U16;
        }
        s = s.wrapping_add(1);
    }

    /* Spread symbols */
    s = 0;
    while s <= maxSymbolValue {
        let mut i: c_int = 0;
        while i < *normalizedCounter.offset(s as isize) as c_int {
            (*tableDecode.offset(position as isize)).symbol = s as BYTE;
            position = (position.wrapping_add(step)) & tableMask;
            while position > highThreshold {
                position = (position.wrapping_add(step)) & tableMask; /* lowprob area */
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
            let symbol: BYTE = (*tableDecode.offset(i as isize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = nextState.wrapping_add(1);
            (*tableDecode.offset(i as isize)).nbBits =
                tableLog.wrapping_sub(BIT_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.offset(i as isize)).newState = (nextState as U32)
                .wrapping_shl((*tableDecode.offset(i as isize)).nbBits as U32)
                .wrapping_sub(tableSize) as U16;
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

/******************************************
 *  FSE helper functions
 ******************************************/
pub fn FSE_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/****************************************************************
 *  FSE NCount encoding-decoding
 ****************************************************************/
pub fn FSE_abs(a: i16) -> i16 {
    if a < 0 {
        a.wrapping_neg()
    } else {
        a
    }
}

pub unsafe fn FSE_readNCount(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart: *const BYTE = headerBuffer as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(hbSize);
    let mut ip: *const BYTE = istart;
    let mut nbBits: c_int;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bitStream: U32;
    let mut bitCount: c_int;
    let mut charnum: c_uint = 0;
    let mut previous0: c_int = 0;

    if hbSize < 4 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    bitStream = MEM_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) as c_int) + FSE_MIN_TABLELOG; /* extract tableLog */
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1i32.wrapping_shl(nbBits as u32)).wrapping_add(1);
    threshold = 1i32.wrapping_shl(nbBits as u32);
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0: c_uint = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 = n0.wrapping_add(24);
                if ip < iend.wrapping_sub(5) {
                    ip = ip.wrapping_add(2);
                    bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
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
                *normalizedCounter.offset(charnum as isize) = 0;
                charnum = charnum.wrapping_add(1);
            }
            if (ip <= iend.wrapping_sub(7))
                || (ip.wrapping_offset((bitCount >> 3) as isize) <= iend.wrapping_sub(4))
            {
                ip = ip.wrapping_offset((bitCount >> 3) as isize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: i16 = ((2i32.wrapping_mul(threshold).wrapping_sub(1)).wrapping_sub(remaining))
                as i16;
            let mut count: i16;

            if (bitStream & (threshold.wrapping_sub(1)) as U32) < (max as U32) {
                count = (bitStream & (threshold.wrapping_sub(1)) as U32) as i16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2i32.wrapping_mul(threshold).wrapping_sub(1)) as U32) as i16;
                if count as c_int >= threshold {
                    count = count.wrapping_sub(max);
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining = remaining.wrapping_sub(FSE_abs(count) as c_int);
            *normalizedCounter.offset(charnum as isize) = count;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as c_int;
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
                    bitCount = bitCount.wrapping_sub(
                        (8isize.wrapping_mul(
                            (iend.wrapping_sub(4) as isize).wrapping_sub(ip as isize),
                        )) as c_int,
                    );
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

/*********************************************************
 *  Decompression (Byte symbols)
 *********************************************************/
pub unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> usize {
    let ptr: *mut c_void = dt as *mut c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let cell: *mut FSE_decode_t = (ptr as *mut FSE_decode_t).add(1);

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

pub unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: c_uint) -> usize {
    let ptr: *mut c_void = dt as *mut c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let dinfo: *mut FSE_decode_t = (ptr as *mut FSE_decode_t).add(1);
    let tableSize: c_uint = 1i32.wrapping_shl(nbBits) as c_uint;
    let tableMask: c_uint = tableSize.wrapping_sub(1);
    let maxSymbolValue: c_uint = tableMask;
    let mut s: c_uint;

    /* Sanity checks */
    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC); /* min size */
    }

    /* Build Decoding Table */
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    s = 0;
    while s <= maxSymbolValue {
        (*dinfo.offset(s as isize)).newState = 0;
        (*dinfo.offset(s as isize)).symbol = s as BYTE;
        (*dinfo.offset(s as isize)).nbBits = nbBits as BYTE;
        s = s.wrapping_add(1);
    }

    0
}

pub unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: c_uint,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.wrapping_add(maxDstSize);
    let olimit: *mut BYTE = omax.wrapping_sub(3);

    let mut bitD: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1: FSE_DState_t = FSE_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2: FSE_DState_t = FSE_DState_t {
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

    /* 4 symbols per loop */
    loop {
        if !((BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit)) {
            break;
        }

        *op.offset(0) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.offset(1) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state2, &mut bitD)
        };

        if FSE_MAX_TABLELOG * 4 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            /* This test must be static */
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.offset(2) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.offset(3) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state2, &mut bitD)
        };

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

        *op = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };
        op = op.wrapping_add(1);

        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0
                && (fast != 0 || FSE_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = if fast != 0 {
            FSE_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state2, &mut bitD)
        };
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

pub unsafe fn FSE_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
) -> usize {
    let mut DTableH: FSE_DTableHeader = FSE_DTableHeader {
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

pub unsafe fn FSE_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut counting: [i16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [U32; DTable_max_t_SIZE] = [0; DTable_max_t_SIZE];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSE_MAX_SYMBOL_VALUE;
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
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

    errorCode = FSE_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    /* always return, even if it is an error code */
    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

/* ******************************************************************
 *  Huff0 : Huffman coder
 ****************************************************************** */

pub fn HUF_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

pub const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16;
pub const HUF_MAX_TABLELOG: u32 = 12;
pub const HUF_DEFAULT_TABLELOG: u32 = HUF_MAX_TABLELOG;
pub const HUF_MAX_SYMBOL_VALUE: u32 = 255;

/* HUF_DTABLE_SIZE(HUF_MAX_TABLELOG) */
pub const HUF_DTABLE_SIZE_MAX: usize = 1 + (1usize << HUF_MAX_TABLELOG);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
} /* single-symbol decoding */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
} /* double-symbols decoding */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
    pub weight: BYTE,
}

static HUF_readStats_l: [c_int; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];

/* ! HUF_readStats
 *  Read compact Huffman tree, saved by HUF_writeCTable
 *  @return : size read from `src` */
pub unsafe fn HUF_readStats(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightTotal: U32;
    let mut tableLog: U32;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: usize;
    let mut oSize: usize;
    let mut n: U32;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            oSize = HUF_readStats_l[iSize - 242] as usize;
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
                *huffWeight.offset(n as isize) = *ip.offset((n / 2) as isize) >> 4;
                *huffWeight.offset((n + 1) as isize) = *ip.offset((n / 2) as isize) & 15;
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
        if (*huffWeight.offset(n as isize) as U32) >= HUF_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *rankStats.offset(*huffWeight.offset(n as isize) as isize) =
            (*rankStats.offset(*huffWeight.offset(n as isize) as isize)).wrapping_add(1);
        weightTotal = weightTotal
            .wrapping_add((1u32.wrapping_shl(*huffWeight.offset(n as isize) as U32)) >> 1);
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
        let total: U32 = 1u32.wrapping_shl(tableLog);
        let rest: U32 = total.wrapping_sub(weightTotal);
        let verif: U32 = 1u32.wrapping_shl(BIT_highbit32(rest));
        let lastWeight: U32 = BIT_highbit32(rest).wrapping_add(1);
        if verif != rest {
            return ERROR(ZSTD_error_corruption_detected); /* last value must be a clean power of 2 */
        }
        *huffWeight.offset(oSize as isize) = lastWeight as BYTE;
        *rankStats.offset(lastWeight as isize) =
            (*rankStats.offset(lastWeight as isize)).wrapping_add(1);
    }

    /* check tree construction validity */
    if (*rankStats.offset(1) < 2) || (*rankStats.offset(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected); /* at least 2 elts of rank 1, must be even */
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    *tableLogPtr = tableLog;
    iSize + 1
}

/**************************/
/* single-symbol decoding */
/**************************/

pub unsafe fn HUF_readDTableX2(DTable: *mut U16, src: *const c_void, srcSize: usize) -> usize {
    let mut huffWeight: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]; /* large enough for values from 0 to 16 */
    let mut tableLog: U32 = 0;
    let ip: *const BYTE = src as *const BYTE;
    let mut iSize: usize = *ip.add(0) as usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let ptr: *mut c_void = DTable.add(1) as *mut c_void;
    let dt: *mut HUF_DEltX2 = ptr as *mut HUF_DEltX2;

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
    if tableLog > *DTable.add(0) as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable is too small */
    }
    *DTable.add(0) = tableLog as U16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current: U32 = nextRankStart;
        nextRankStart =
            nextRankStart.wrapping_add(rankVal[n as usize].wrapping_shl(n.wrapping_sub(1)));
        rankVal[n as usize] = current;
        n = n.wrapping_add(1);
    }

    /* fill DTable */
    n = 0;
    while n < nbSymbols {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32.wrapping_shl(w)) >> 1;
        let mut i: U32;
        let mut D: HUF_DEltX2 = HUF_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = tableLog.wrapping_add(1).wrapping_sub(w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize].wrapping_add(length) {
            *dt.offset(i as isize) = D;
            i = i.wrapping_add(1);
        }
        rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
        n = n.wrapping_add(1);
    }

    iSize
}

pub unsafe fn HUF_decodeSymbolX2(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val: usize = BIT_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.add(val)).byte;
    BIT_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

macro_rules! HUF_DECODE_SYMBOLX2_0 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUF_decodeSymbolX2($DStreamPtr, $dt, $dtLog);
        $ptr = $ptr.wrapping_add(1);
    }};
}

macro_rules! HUF_DECODE_SYMBOLX2_1 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {
        if MEM_64bits() != 0 || (HUF_MAX_TABLELOG <= 12) {
            HUF_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    };
}

macro_rules! HUF_DECODE_SYMBOLX2_2 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {
        if MEM_64bits() != 0 {
            HUF_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    };
}

pub unsafe fn HUF_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart: *mut BYTE = p;

    /* up to 4 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(4)) {
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd) {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    (pEnd as usize).wrapping_sub(pStart as usize)
}

pub unsafe fn HUF_decompress4X2_usingDTable(
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
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);

        let ptr: *const c_void = DTable as *const c_void;
        let dt: *const HUF_DEltX2 = (ptr as *const HUF_DEltX2).add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1: BIT_DStream_t = BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2: BIT_DStream_t = bitD1;
        let mut bitD3: BIT_DStream_t = bitD1;
        let mut bitD4: BIT_DStream_t = bitD1;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.wrapping_add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.wrapping_add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1: *const BYTE = istart.wrapping_add(6); /* jumpTable */
        let istart2: *const BYTE = istart1.wrapping_add(length1);
        let istart3: *const BYTE = istart2.wrapping_add(length2);
        let istart4: *const BYTE = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2: *mut BYTE = ostart.wrapping_add(segmentSize);
        let opStart3: *mut BYTE = opStart2.wrapping_add(segmentSize);
        let opStart4: *mut BYTE = opStart3.wrapping_add(segmentSize);
        let mut op1: *mut BYTE = ostart;
        let mut op2: *mut BYTE = opStart2;
        let mut op3: *mut BYTE = opStart3;
        let mut op4: *mut BYTE = opStart4;
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

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            HUF_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX2_1!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX2_1!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX2_1!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX2_1!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX2_0!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX2_0!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX2_0!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX2_0!(op4, &mut bitD4, dt, dtLog);

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

pub unsafe fn HUF_decompress4X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U16; HUF_DTABLE_SIZE_MAX] = [0; HUF_DTABLE_SIZE_MAX];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let mut errorCode: usize;

    errorCode = HUF_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

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

pub unsafe fn HUF_fillDTableX4Level2(
    DTable: *mut HUF_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: c_int,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    let mut DElt: HUF_DEltX4 = HUF_DEltX4 {
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
            *DTable.offset(i as isize) = DElt;
            i = i.wrapping_add(1);
        }
    }

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        /* note : sortedSymbols already skipped */
        let symbol: U32 = (*sortedSymbols.offset(s as isize)).symbol as U32;
        let weight: U32 = (*sortedSymbols.offset(s as isize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let length: U32 = 1u32.wrapping_shl(sizeLog.wrapping_sub(nbBits));
        let start: U32 = rankVal[weight as usize];
        let mut i: U32 = start;
        let end: U32 = start.wrapping_add(length);

        MEM_writeLE16(
            &mut DElt.sequence as *mut U16 as *mut c_void,
            (baseSeq as U32).wrapping_add(symbol.wrapping_shl(8)) as U16,
        );
        DElt.nbBits = nbBits.wrapping_add(consumed) as BYTE;
        DElt.length = 2;
        loop {
            *DTable.offset(i as isize) = DElt;
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
pub type rankVal_t = [[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    HUF_ABSOLUTEMAX_TABLELOG as usize];

pub unsafe fn HUF_fillDTableX4(
    DTable: *mut HUF_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *mut [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int; /* note : targetLog >= srcLog, hence scaleLog <= 1 */
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
        let symbol: U16 = (*sortedList.offset(s as isize)).symbol as U16;
        let weight: U32 = (*sortedList.offset(s as isize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32.wrapping_shl(targetLog.wrapping_sub(nbBits));

        if targetLog.wrapping_sub(nbBits) >= minBits {
            /* enough room for a second symbol */
            let sortedRank: U32;
            let mut minWeight: c_int = nbBits.wrapping_add(scaleLog as U32) as c_int;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.offset(minWeight as isize);
            HUF_fillDTableX4Level2(
                DTable.offset(start as isize),
                targetLog.wrapping_sub(nbBits),
                nbBits,
                (*rankValOrigin.offset(nbBits as isize)).as_ptr(),
                minWeight,
                sortedList.offset(sortedRank as isize),
                sortedListSize.wrapping_sub(sortedRank),
                nbBitsBaseline,
                symbol,
            );
        } else {
            let mut i: U32;
            let end: U32 = start.wrapping_add(length);
            let mut DElt: HUF_DEltX4 = HUF_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };

            MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1;
            i = start;
            while i < end {
                *DTable.offset(i as isize) = DElt;
                i = i.wrapping_add(1);
            }
        }
        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s = s.wrapping_add(1);
    }
}

pub unsafe fn HUF_readDTableX4(DTable: *mut U32, src: *const c_void, srcSize: usize) -> usize {
    let mut weightList: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let rankStart: *mut U32 = rankStart0.as_mut_ptr().wrapping_add(1);
    let mut rankVal: rankVal_t = [[0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
        HUF_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let mut sizeOfSort: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.add(0);
    let ip: *const BYTE = src as *const BYTE;
    let mut iSize: usize = *ip.add(0) as usize;
    let ptr: *mut c_void = DTable as *mut c_void;
    let dt: *mut HUF_DEltX4 = (ptr as *mut HUF_DEltX4).add(1);

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
        } /* necessarily finds a solution before maxW==0 */
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
            *rankStart.offset(w as isize) = current;
            w = w.wrapping_add(1);
        }
        *rankStart.offset(0) = nextRankStart; /* put all 0w symbols at the end of sorted list */
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = *rankStart.offset(w as isize);
            *rankStart.offset(w as isize) = r.wrapping_add(1);
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s = s.wrapping_add(1);
        }
        *rankStart.offset(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: c_int = (memLog.wrapping_sub(tableLog)).wrapping_sub(1) as c_int; /* tableLog <= memLog */
        let rankVal0: *mut U32 = rankVal.as_mut_ptr() as *mut U32;
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankVal;
            nextRankVal = nextRankVal.wrapping_add(
                rankStats[w as usize].wrapping_shl(w.wrapping_add(rescale as U32)),
            );
            *rankVal0.offset(w as isize) = current;
            w = w.wrapping_add(1);
        }
        consumed = minBits;
        while consumed <= memLog.wrapping_sub(minBits) {
            let rankValPtr: *mut U32 =
                (rankVal.as_mut_ptr().wrapping_offset(consumed as isize)) as *mut U32;
            w = 1;
            while w <= maxW {
                *rankValPtr.offset(w as isize) = *rankVal0.offset(w as isize) >> consumed;
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
        rankVal.as_mut_ptr(),
        maxW,
        tableLog.wrapping_add(1),
    );

    iSize
}

pub unsafe fn HUF_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.add(val) as *const c_void, 2);
    BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

pub unsafe fn HUF_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (core::mem::size_of::<usize>() * 8) as c_uint {
            BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (core::mem::size_of::<usize>() * 8) as c_uint {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as c_uint;
                /* ugly hack; works only because it's the last symbol */
            }
        }
    }
    1
}

macro_rules! HUF_DECODE_SYMBOLX4_0 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.wrapping_add(
            HUF_decodeSymbolX4($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize,
        );
    }};
}

macro_rules! HUF_DECODE_SYMBOLX4_1 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {
        if MEM_64bits() != 0 || (HUF_MAX_TABLELOG <= 12) {
            $ptr = $ptr.wrapping_add(
                HUF_decodeSymbolX4($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize,
            );
        }
    };
}

macro_rules! HUF_DECODE_SYMBOLX4_2 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {
        if MEM_64bits() != 0 {
            $ptr = $ptr.wrapping_add(
                HUF_decodeSymbolX4($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize,
            );
        }
    };
}

pub unsafe fn HUF_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart: *mut BYTE = p;

    /* up to 8 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd.wrapping_sub(7)) {
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(2)) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p <= pEnd.wrapping_sub(2) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog); /* no need to reload */
    }

    if p < pEnd {
        p = p.wrapping_add(HUF_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    (p as usize).wrapping_sub(pStart as usize)
}

pub unsafe fn HUF_decompress4X4_usingDTable(
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
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);

        let ptr: *const c_void = DTable as *const c_void;
        let dt: *const HUF_DEltX4 = (ptr as *const HUF_DEltX4).add(1);
        let dtLog: U32 = *DTable.add(0);
        let mut errorCode: usize;

        /* Init */
        let mut bitD1: BIT_DStream_t = BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2: BIT_DStream_t = bitD1;
        let mut bitD3: BIT_DStream_t = bitD1;
        let mut bitD4: BIT_DStream_t = bitD1;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.wrapping_add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.wrapping_add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1: *const BYTE = istart.wrapping_add(6); /* jumpTable */
        let istart2: *const BYTE = istart1.wrapping_add(length1);
        let istart3: *const BYTE = istart2.wrapping_add(length2);
        let istart4: *const BYTE = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2: *mut BYTE = ostart.wrapping_add(segmentSize);
        let opStart3: *mut BYTE = opStart2.wrapping_add(segmentSize);
        let opStart4: *mut BYTE = opStart3.wrapping_add(segmentSize);
        let mut op1: *mut BYTE = ostart;
        let mut op2: *mut BYTE = opStart2;
        let mut op3: *mut BYTE = opStart3;
        let mut op4: *mut BYTE = opStart4;
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

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            HUF_DECODE_SYMBOLX4_2!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX4_1!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX4_1!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX4_1!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX4_1!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX4_0!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX4_0!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX4_0!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX4_0!(op4, &mut bitD4, dt, dtLog);

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

pub unsafe fn HUF_decompress4X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U32; HUF_DTABLE_SIZE_MAX] = [0; HUF_DTABLE_SIZE_MAX];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUF_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize = cSrcSize.wrapping_sub(hSize);

    HUF_decompress4X4_usingDTable(
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
#[derive(Copy, Clone)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}

const fn AT(tableTime: U32, decode256Time: U32) -> algo_time_t {
    algo_time_t {
        tableTime,
        decode256Time,
    }
}

pub static algoTime: [[algo_time_t; 3]; 16] = [
    /* single, double, quad */
    [AT(0, 0), AT(1, 1), AT(2, 2)], /* Q==0 : impossible */
    [AT(0, 0), AT(1, 1), AT(2, 2)], /* Q==1 : impossible */
    [AT(38, 130), AT(1313, 74), AT(2151, 38)], /* Q == 2 : 12-18% */
    [AT(448, 128), AT(1353, 74), AT(2238, 41)], /* Q == 3 : 18-25% */
    [AT(556, 128), AT(1353, 74), AT(2238, 47)], /* Q == 4 : 25-32% */
    [AT(714, 128), AT(1418, 74), AT(2436, 53)], /* Q == 5 : 32-38% */
    [AT(883, 128), AT(1437, 74), AT(2464, 61)], /* Q == 6 : 38-44% */
    [AT(897, 128), AT(1515, 75), AT(2622, 68)], /* Q == 7 : 44-50% */
    [AT(926, 128), AT(1613, 75), AT(2730, 75)], /* Q == 8 : 50-56% */
    [AT(947, 128), AT(1729, 77), AT(3359, 77)], /* Q == 9 : 56-62% */
    [AT(1107, 128), AT(2083, 81), AT(4006, 84)], /* Q ==10 : 62-69% */
    [AT(1177, 128), AT(2379, 87), AT(4785, 88)], /* Q ==11 : 69-75% */
    [AT(1242, 128), AT(2415, 93), AT(5155, 84)], /* Q ==12 : 75-81% */
    [AT(1349, 128), AT(2644, 106), AT(5260, 106)], /* Q ==13 : 81-87% */
    [AT(1455, 128), AT(2422, 124), AT(4174, 124)], /* Q ==14 : 87-93% */
    [AT(722, 128), AT(1891, 145), AT(1936, 146)], /* Q ==15 : 93-99% */
];

pub type decompressionAlgo =
    unsafe fn(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize;

static HUF_decompress_decompress: [Option<decompressionAlgo>; 3] =
    [Some(HUF_decompress4X2), Some(HUF_decompress4X4), None];

pub unsafe fn HUF_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* estimate decompression time */
    let Q: U32;
    let D256: U32 = (dstSize >> 8) as U32;
    let mut Dtime: [U32; 3] = [0; 3];
    let mut algoNb: U32 = 0;
    let mut n: c_int;

    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected); /* invalid */
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    } /* not compressed */
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    } /* RLE */

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

    (HUF_decompress_decompress[algoNb as usize].unwrap())(dst, dstSize, cSrc, cSrcSize)
}

/* ******************************************************************
 *  zstd -- decompression
 ****************************************************************** */

pub const ZSTD_MEMORY_USAGE: u32 = 17;
pub const ZSTD_HEAPMODE: u32 = 1;

pub const HASH_LOG: u32 = ZSTD_MEMORY_USAGE - 2;
pub const HASH_TABLESIZE: u32 = 1u32 << HASH_LOG;
pub const HASH_MASK: u32 = HASH_TABLESIZE - 1;

pub const KNUTH: u32 = 2654435761;

pub const BIT7: u32 = 128;
pub const BIT6: u32 = 64;
pub const BIT5: u32 = 32;
pub const BIT4: u32 = 16;
pub const BIT1: u32 = 2;
pub const BIT0: u32 = 1;

pub const BLOCKSIZE: usize = 128 * (1 << 10); /* define, for static allocation */
pub const MIN_SEQUENCES_SIZE: usize = 2 /*seqNb*/ + 2 /*dumps*/ + 3 /*seqTables*/ + 1 /*bitStream*/;
pub const MIN_CBLOCK_SIZE: usize = 3 /*litCSize*/ + MIN_SEQUENCES_SIZE;
pub const IS_RAW: u32 = BIT0;
pub const IS_RLE: u32 = BIT1;

pub const WORKPLACESIZE: usize = BLOCKSIZE * 3;
pub const MINMATCH: usize = 4;
pub const MLbits: c_uint = 7;
pub const LLbits: c_uint = 6;
pub const Offbits: c_uint = 5;
pub const MaxML: usize = (1usize << MLbits) - 1;
pub const MaxLL: usize = (1usize << LLbits) - 1;
pub const MaxOff: usize = 31;
pub const LitFSELog: u32 = 11;
pub const MLFSELog: u32 = 10;
pub const LLFSELog: u32 = 10;
pub const OffFSELog: u32 = 9;
pub const MaxSeq: usize = if MaxLL < MaxML { MaxML } else { MaxLL };

pub const LITERAL_NOENTROPY: u32 = 63;
pub const COMMAND_NOENTROPY: u32 = 7; /* to remove */

pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

pub static ZSTD_blockHeaderSize: usize = 3;
pub static ZSTD_frameHeaderSize: usize = 4;

/* *******************************************************
 *  Memory operations
 **********************************************************/
pub unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

pub unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

/* ! ZSTD_wildcopy : custom version of memcpy(), can copy up to 7-8 bytes too many */
pub unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_offset(length);
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
pub type blockType_t = c_uint;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SeqStore_t {
    pub buffer: *mut c_void,
    pub offsetStart: *mut U32,
    pub offset: *mut U32,
    pub offCodeStart: *mut BYTE,
    pub offCode: *mut BYTE,
    pub litStart: *mut BYTE,
    pub lit: *mut BYTE,
    pub litLengthStart: *mut BYTE,
    pub litLength: *mut BYTE,
    pub matchLengthStart: *mut BYTE,
    pub matchLength: *mut BYTE,
    pub dumpsStart: *mut BYTE,
    pub dumps: *mut BYTE,
}

/* *************************************
 *  Error Management
 ***************************************/
/* ! ZSTD_isError : tells if a return value is an error code */
pub fn ZSTD_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/* *************************************************************
 *   Decompression section
 ***************************************************************/

/* FSE_DTABLE_SIZE_U32(LLFSELog) */
pub const LLTable_SIZE: usize = 1 + (1usize << LLFSELog);
/* FSE_DTABLE_SIZE_U32(OffFSELog) */
pub const OffTable_SIZE: usize = 1 + (1usize << OffFSELog);
/* FSE_DTABLE_SIZE_U32(MLFSELog) */
pub const MLTable_SIZE: usize = 1 + (1usize << MLFSELog);

#[repr(C)]
pub struct ZSTDv03_Dctx_s {
    pub LLTable: [U32; LLTable_SIZE],
    pub OffTable: [U32; OffTable_SIZE],
    pub MLTable: [U32; MLTable_SIZE],
    pub previousDstEnd: *mut c_void,
    pub base: *mut c_void,
    pub expected: usize,
    pub bType: blockType_t,
    pub phase: U32,
    pub litPtr: *const BYTE,
    pub litSize: usize,
    pub litBuffer: [BYTE; BLOCKSIZE + 8 /* margin for wildcopy */],
}

pub type ZSTDv03_Dctx = ZSTDv03_Dctx_s;
pub type ZSTD_DCtx = ZSTDv03_Dctx_s;

pub unsafe fn ZSTD_getcBlockSize(
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
    cSize = (*in_.add(2) as U32)
        .wrapping_add((*in_.add(1) as U32) << 8)
        .wrapping_add(((*in_.add(0) as U32) & 7) << 16);

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

/** ZSTD_decompressLiterals
 *  @return : nb of bytes read from src, or an error code */
pub unsafe fn ZSTD_decompressLiterals(
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip: *const BYTE = src as *const BYTE;

    let litSize: usize = ((MEM_readLE32(src) & 0x1FFFFF) >> 2) as usize; /* no buffer issue : srcSize >= MIN_CBLOCK_SIZE */
    let litCSize: usize =
        ((MEM_readLE32(ip.wrapping_add(2) as *const c_void) & 0xFFFFFF) >> 5) as usize; /* no buffer issue */

    if litSize > *maxDstSizePtr {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if litCSize + 5 > srcSize {
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
    litCSize + 5
}

/** ZSTD_decodeLiteralsBlock
 *  @return : nb of bytes read from src (< srcSize ) */
pub unsafe fn ZSTD_decodeLiteralsBlock(
    ctx: *mut c_void,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let dctx: *mut ZSTD_DCtx = ctx as *mut ZSTD_DCtx;
    let istart: *const BYTE = src as *const BYTE;

    /* any compressed block with literals segment must be at least this size */
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart & 3) as u32 {
        x if x == IS_RAW => {
            let litSize: usize =
                ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as usize; /* no buffer issue */
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
                return litSize + 3;
            }
            /* direct reference into compressed stream */
            (*dctx).litPtr = istart.wrapping_add(3);
            (*dctx).litSize = litSize;
            litSize + 3
        }
        x if x == IS_RLE => {
            let litSize: usize =
                ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as usize; /* no buffer issue */
            if litSize > BLOCKSIZE {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.add(3) as c_int,
                litSize + 8,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            4
        }
        /* default and case 0 */
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

pub unsafe fn ZSTD_decodeSeqHeaders(
    nbSeq: *mut c_int,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut FSE_DTable,
    DTableML: *mut FSE_DTable,
    DTableOffb: *mut FSE_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: U32 = 0;
    let mut Offlog: U32 = 0;
    let mut MLlog: U32 = 0;
    let mut dumpsLength: usize;

    /* check */
    if srcSize < 5 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    *nbSeq = MEM_readLE16(ip as *const c_void) as c_int;
    ip = ip.wrapping_add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = *ip.add(2) as usize;
        dumpsLength = dumpsLength.wrapping_add((*ip.add(1) as usize) << 8);
        ip = ip.wrapping_add(3);
    } else {
        dumpsLength = *ip.add(1) as usize;
        dumpsLength = dumpsLength.wrapping_add(((*ip.add(0) as usize) & 1) << 8);
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
        let mut norm: [S16; MaxML + 1] = [0; MaxML + 1]; /* assumption : MaxML >= MaxLL and MaxOff */
        let mut headerSize: usize;

        /* Build DTables */
        match LLtype {
            x if x == bt_rle => {
                LLlog = 0;
                let sym = *ip;
                ip = ip.wrapping_add(1);
                FSE_buildDTable_rle(DTableLL, sym);
            }
            x if x == bt_raw => {
                LLlog = LLbits;
                FSE_buildDTable_raw(DTableLL, LLbits);
            }
            _ => {
                let mut max: U32 = MaxLL as U32;
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
        }

        match Offtype {
            x if x == bt_rle => {
                Offlog = 0;
                if ip > iend.wrapping_sub(2) {
                    return ERROR(ZSTD_error_srcSize_wrong); /* min : "raw", hence no header */
                }
                let sym = *ip & (MaxOff as BYTE);
                ip = ip.wrapping_add(1);
                FSE_buildDTable_rle(DTableOffb, sym); /* if *ip > MaxOff, data is corrupted */
            }
            x if x == bt_raw => {
                Offlog = Offbits;
                FSE_buildDTable_raw(DTableOffb, Offbits);
            }
            _ => {
                let mut max: U32 = MaxOff as U32;
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
        }

        match MLtype {
            x if x == bt_rle => {
                MLlog = 0;
                if ip > iend.wrapping_sub(2) {
                    return ERROR(ZSTD_error_srcSize_wrong); /* min : "raw", hence no header */
                }
                let sym = *ip;
                ip = ip.wrapping_add(1);
                FSE_buildDTable_rle(DTableML, sym);
            }
            x if x == bt_raw => {
                MLlog = MLbits;
                FSE_buildDTable_raw(DTableML, MLbits);
            }
            _ => {
                let mut max: U32 = MaxML as U32;
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
    }

    (ip as usize).wrapping_sub(istart as usize)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct seq_t {
    pub litLength: usize,
    pub offset: usize,
    pub matchLength: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct seqState_t {
    pub DStream: BIT_DStream_t,
    pub stateLL: FSE_DState_t,
    pub stateOffb: FSE_DState_t,
    pub stateML: FSE_DState_t,
    pub prevOffset: usize,
    pub dumps: *const BYTE,
    pub dumpsEnd: *const BYTE,
}

/* note : size_t faster than U32 */
static offsetPrefix: [usize; MaxOff + 1] = [
    1, /*fake*/
    1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072,
    262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, /*fake*/ 1, 1, 1,
    1, 1,
];

pub unsafe fn ZSTD_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
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
    if litLength == MaxLL {
        let add: U32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.wrapping_add(1);
            v as U32
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
        let offsetCode: U32;
        let mut nbBits: U32;
        offsetCode = FSE_decodeSymbol(
            &mut (*seqState).stateOffb,
            &mut (*seqState).DStream,
        ) as U32; /* <= maxOff, by table construction */
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        nbBits = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0; /* cmove */
        }
        offset = offsetPrefix[offsetCode as usize]
            .wrapping_add(BIT_readBits(&mut (*seqState).DStream, nbBits));
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset; /* cmove */
        }
    }

    /* MatchLength */
    matchLength = FSE_decodeSymbol(
        &mut (*seqState).stateML,
        &mut (*seqState).DStream,
    ) as usize;
    if matchLength == MaxML {
        let add: U32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.wrapping_add(1);
            v as U32
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

static dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */

pub unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> usize {
    let ostart: *const BYTE = op;
    let oLitEnd: *mut BYTE = op.wrapping_add(sequence.litLength);
    let oMatchEnd: *mut BYTE = op
        .wrapping_add(sequence.litLength)
        .wrapping_add(sequence.matchLength); /* risk : address space overflow (32-bits) */
    let oend_8: *mut BYTE = oend.wrapping_sub(8);
    let litEnd: *const BYTE = (*litPtr).wrapping_add(sequence.litLength);

    /* checks */
    let seqLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > (oend as usize).wrapping_sub(op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize).wrapping_sub(*litPtr as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Now we know there are no overflow in literal nor match lengths, can use pointer checks */
    if oLitEnd > oend_8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.offset > (((oLitEnd as usize).wrapping_sub(base as usize)) as U32) as usize {
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
    ); /* note : oLitEnd <= oend-8 */
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
            let dec64: c_int = dec64table[sequence.offset];
            *op.offset(0) = *match_.offset(0);
            *op.offset(1) = *match_.offset(1);
            *op.offset(2) = *match_.offset(2);
            *op.offset(3) = *match_.offset(3);
            match_ = match_.wrapping_offset(dec32table[sequence.offset] as isize);
            ZSTD_copy4(op.wrapping_add(4) as *mut c_void, match_ as *const c_void);
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
                match_ = match_.wrapping_offset((oend_8 as isize).wrapping_sub(op as isize));
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

pub unsafe fn ZSTD_decompressSequences(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
) -> usize {
    let dctx: *mut ZSTD_DCtx = ctx as *mut ZSTD_DCtx;
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(seqSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut errorCode: usize;
    let mut dumpsLength: usize = 0;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.wrapping_add((*dctx).litSize);
    let mut nbSeq: c_int = 0;
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
        let mut sequence: seq_t = seq_t {
            litLength: 0,
            offset: 0,
            matchLength: 0,
        };
        let mut seqState: seqState_t = seqState_t {
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
        sequence.offset = 4;
        seqState.prevOffset = sequence.offset;
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
            oneSeqSize =
                ZSTD_execSequence(op, sequence, &mut litPtr, litEnd, base, oend);
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
                if (op as *const BYTE) != litPtr {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.wrapping_add(lastLLSize);
            }
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

pub unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip: *const BYTE = src as *const BYTE;

    /* Decode literals sub-block */
    let litCSize: usize = ZSTD_decodeLiteralsBlock(ctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.wrapping_add(litCSize);
    srcSize = srcSize.wrapping_sub(litCSize);

    ZSTD_decompressSequences(ctx, dst, maxDstSize, ip as *const c_void, srcSize)
}

pub unsafe fn ZSTD_decompressDCtx(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut remainingSize: usize = srcSize;
    let magicNumber: U32;
    let mut blockProperties: blockProperties_t = blockProperties_t {
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

pub unsafe fn ZSTD_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ctx_storage = core::mem::MaybeUninit::<ZSTD_DCtx>::uninit();
    let ctx: *mut ZSTD_DCtx = ctx_storage.as_mut_ptr();
    (*ctx).base = dst;
    ZSTD_decompressDCtx(ctx as *mut c_void, dst, maxDstSize, src, srcSize)
}

/* ZSTD_errorFrameSizeInfoLegacy() :
 *  assumes `cSize` and `dBound` are _not_ NULL */
pub unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut u64, ret: usize) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

/* *************************************
 *  Prefix - version detection
 ***************************************/
pub const ZSTD_magicNumber: U32 = 0xFD2FB523; /* v0.3 */
pub const ZSTDv03_magicNumber: U32 = 0xFD2FB523; /* v0.3 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;
    let magicNumber: U32;
    let mut blockProperties: blockProperties_t = blockProperties_t {
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
    *dBound = (nbBlocks.wrapping_mul(BLOCKSIZE)) as u64;
}

/*******************************
 *  Streaming Decompression API
 *******************************/

pub unsafe fn ZSTD_resetDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0;
    (*dctx).previousDstEnd = core::ptr::null_mut();
    (*dctx).base = core::ptr::null_mut();
    0
}

pub unsafe fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    let dctx: *mut ZSTD_DCtx = malloc(core::mem::size_of::<ZSTD_DCtx>()) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_resetDCtx(dctx);
    dctx
}

pub unsafe fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    free(dctx as *mut c_void);
    0
}

pub unsafe fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected
}

pub unsafe fn ZSTD_decompressContinue(
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
        let mut bp: blockProperties_t = blockProperties_t {
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
        (*ctx).previousDstEnd = ((dst as *mut c_char).wrapping_add(rSize)) as *mut c_void;
        return rSize;
    }
}

/* wrapper layer */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_isError(code: usize) -> c_uint {
    ZSTD_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_decompress(
    dst: *mut c_void,
    maxOriginalSize: usize,
    src: *const c_void,
    compressedSize: usize,
) -> usize {
    ZSTD_decompress(dst, maxOriginalSize, src, compressedSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_createDCtx() -> *mut ZSTDv03_Dctx {
    ZSTD_createDCtx() as *mut ZSTDv03_Dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_freeDCtx(dctx: *mut ZSTDv03_Dctx) -> usize {
    ZSTD_freeDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_resetDCtx(dctx: *mut ZSTDv03_Dctx) -> usize {
    ZSTD_resetDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_nextSrcSizeToDecompress(dctx: *mut ZSTDv03_Dctx) -> usize {
    ZSTD_nextSrcSizeToDecompress(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_decompressContinue(
    dctx: *mut ZSTDv03_Dctx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompressContinue(dctx as *mut ZSTD_DCtx, dst, maxDstSize, src, srcSize)
}
