//! Translation of legacy/zstd_v03.c (zstd v0.3 decompressor).
//! Self-contained: defines its own FSE/HUF/BIT/ZSTD-decoder internals.
//! Target: little-endian 64-bit. Byte-identical reproduction of the C source.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_parens)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::common::allocations::{free, malloc, memcpy, memmove, memset};

/* ****************************************************************
 * Error codes (local to this legacy file; values differ from the
 * modern zstd error enum, so they are reproduced here exactly).
 * enum: No_Error=0, GENERIC=1, dstSize_tooSmall=2, srcSize_wrong=3,
 *       prefix_unknown=4, corruption_detected=5, tableLog_tooLarge=6,
 *       maxSymbolValue_tooLarge=7, maxSymbolValue_tooSmall=8, maxCode=9
 * ************************************************************** */
// NOTE: legacy C includes ../common/error_private.h first, guarding out the
// file-local enum, so ERROR/ERR_isError use MODERN zstd_errors.h values.
mod ec {
    pub const No_Error: usize = 0;
    pub const GENERIC: usize = 1;
    pub const dstSize_tooSmall: usize = 70;
    pub const srcSize_wrong: usize = 72;
    pub const prefix_unknown: usize = 10;
    pub const corruption_detected: usize = 20;
    pub const tableLog_tooLarge: usize = 44;
    pub const maxSymbolValue_tooLarge: usize = 46;
    pub const maxSymbolValue_tooSmall: usize = 48;
    pub const maxCode: usize = 120;
}

#[inline]
fn ERROR(code: usize) -> usize {
    (0usize).wrapping_sub(code)
}

#[inline]
fn ERR_isError(code: usize) -> c_uint {
    (code > ERROR(ec::maxCode)) as c_uint
}

/* ****************************************************************
 * mem.h : low-level memory access (little-endian 64-bit)
 * ************************************************************** */
#[inline]
fn MEM_32bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 4) as c_uint
}
#[inline]
fn MEM_64bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 8) as c_uint
}

#[inline]
unsafe fn MEM_read16(memPtr: *const c_void) -> u16 {
    core::ptr::read_unaligned(memPtr as *const u16)
}
#[inline]
unsafe fn MEM_read32(memPtr: *const c_void) -> u32 {
    core::ptr::read_unaligned(memPtr as *const u32)
}
#[inline]
unsafe fn MEM_read64(memPtr: *const c_void) -> u64 {
    core::ptr::read_unaligned(memPtr as *const u64)
}
#[inline]
unsafe fn MEM_write16(memPtr: *mut c_void, value: u16) {
    core::ptr::write_unaligned(memPtr as *mut u16, value);
}
#[inline]
unsafe fn MEM_readLE16(memPtr: *const c_void) -> u16 {
    MEM_read16(memPtr)
}
#[inline]
unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: u16) {
    MEM_write16(memPtr, val);
}
#[inline]
unsafe fn MEM_readLE24(memPtr: *const c_void) -> u32 {
    MEM_readLE16(memPtr) as u32 + ((*(memPtr as *const u8).add(2) as u32) << 16)
}
#[inline]
unsafe fn MEM_readLE32(memPtr: *const c_void) -> u32 {
    MEM_read32(memPtr)
}
#[inline]
unsafe fn MEM_readLE64(memPtr: *const c_void) -> u64 {
    MEM_read64(memPtr)
}
#[inline]
unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    if MEM_32bits() != 0 {
        MEM_readLE32(memPtr) as usize
    } else {
        MEM_readLE64(memPtr) as usize
    }
}

/* ****************************************************************
 * bitstream : read backward
 * ************************************************************** */
#[repr(C)]
struct BIT_DStream_t {
    bitContainer: usize,
    bitsConsumed: c_uint,
    ptr: *const c_char,
    start: *const c_char,
}

const BIT_DStream_unfinished: c_int = 0;
const BIT_DStream_endOfBuffer: c_int = 1;
const BIT_DStream_completed: c_int = 2;
const BIT_DStream_overflow: c_int = 3;
type BIT_DStream_status = c_int;

#[inline]
fn BIT_highbit32(val: u32) -> c_uint {
    val.leading_zeros() ^ 31
}

unsafe fn BIT_initDStream(
    bitD: *mut BIT_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BIT_DStream_t>());
        return ERROR(ec::srcSize_wrong);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        /* normal case */
        let contain32: u32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr =
            (srcBuffer as *const c_char).add(srcSize - core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        contain32 = *(srcBuffer as *const u8).add(srcSize - 1) as u32;
        if contain32 == 0 {
            return ERROR(ec::GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BIT_highbit32(contain32);
    } else {
        let contain32: u32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const u8) as usize;
        let sptr = (*bitD).start as *const u8;
        let szbits = core::mem::size_of::<usize>() * 8;
        match srcSize {
            7 => {
                (*bitD).bitContainer += (*sptr.add(6) as usize) << (szbits - 16);
                (*bitD).bitContainer += (*sptr.add(5) as usize) << (szbits - 24);
                (*bitD).bitContainer += (*sptr.add(4) as usize) << (szbits - 32);
                (*bitD).bitContainer += (*sptr.add(3) as usize) << 24;
                (*bitD).bitContainer += (*sptr.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sptr.add(1) as usize) << 8;
            }
            6 => {
                (*bitD).bitContainer += (*sptr.add(5) as usize) << (szbits - 24);
                (*bitD).bitContainer += (*sptr.add(4) as usize) << (szbits - 32);
                (*bitD).bitContainer += (*sptr.add(3) as usize) << 24;
                (*bitD).bitContainer += (*sptr.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sptr.add(1) as usize) << 8;
            }
            5 => {
                (*bitD).bitContainer += (*sptr.add(4) as usize) << (szbits - 32);
                (*bitD).bitContainer += (*sptr.add(3) as usize) << 24;
                (*bitD).bitContainer += (*sptr.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sptr.add(1) as usize) << 8;
            }
            4 => {
                (*bitD).bitContainer += (*sptr.add(3) as usize) << 24;
                (*bitD).bitContainer += (*sptr.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sptr.add(1) as usize) << 8;
            }
            3 => {
                (*bitD).bitContainer += (*sptr.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sptr.add(1) as usize) << 8;
            }
            2 => {
                (*bitD).bitContainer += (*sptr.add(1) as usize) << 8;
            }
            _ => {}
        }
        contain32 = *(srcBuffer as *const u8).add(srcSize - 1) as u32;
        if contain32 == 0 {
            return ERROR(ec::GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BIT_highbit32(contain32);
        (*bitD).bitsConsumed += ((core::mem::size_of::<usize>() - srcSize) as u32) * 8;
    }

    srcSize
}

#[inline]
unsafe fn BIT_lookBits(bitD: *const BIT_DStream_t, nbBits: u32) -> usize {
    let bitMask: u32 = (core::mem::size_of::<usize>() * 8) as u32 - 1;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask - nbBits) & bitMask)
}

#[inline]
unsafe fn BIT_lookBitsFast(bitD: *const BIT_DStream_t, nbBits: u32) -> usize {
    let bitMask: u32 = (core::mem::size_of::<usize>() * 8) as u32 - 1;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1) - nbBits) & bitMask)
}

#[inline]
unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: u32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
unsafe fn BIT_readBits(bitD: *mut BIT_DStream_t, nbBits: u32) -> usize {
    let value = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

#[inline]
unsafe fn BIT_readBitsFast(bitD: *mut BIT_DStream_t, nbBits: u32) -> usize {
    let value = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

unsafe fn BIT_reloadDStream(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as u32 {
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD).ptr.offset(-(((*bitD).bitsConsumed >> 3) as isize));
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as u32 {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes: u32 = (*bitD).bitsConsumed >> 3;
        let mut result: BIT_DStream_status = BIT_DStream_unfinished;
        if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize - (*bitD).start as usize) as u32;
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return result;
    }
}

#[inline]
unsafe fn BIT_endOfDStream(dstream: *const BIT_DStream_t) -> c_uint {
    (((*dstream).ptr == (*dstream).start)
        && ((*dstream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as u32)) as c_uint
}

/* ****************************************************************
 * FSE types
 * ************************************************************** */
type FSE_DTable = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DTableHeader {
    tableLog: u16,
    fastMode: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_decode_t {
    newState: u16,
    symbol: u8,
    nbBits: u8,
}

#[repr(C)]
struct FSE_DState_t {
    state: usize,
    table: *const c_void,
}

const FSE_MAX_MEMORY_USAGE: usize = 14;
const FSE_DEFAULT_MEMORY_USAGE: usize = 13;
const FSE_MAX_SYMBOL_VALUE: usize = 255;
const FSE_MAX_TABLELOG: usize = FSE_MAX_MEMORY_USAGE - 2;
const FSE_MAX_TABLESIZE: usize = 1 << FSE_MAX_TABLELOG;
const FSE_MAXTABLESIZE_MASK: usize = FSE_MAX_TABLESIZE - 1;
const FSE_DEFAULT_TABLELOG: usize = FSE_DEFAULT_MEMORY_USAGE - 2;
const FSE_MIN_TABLELOG: usize = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: usize = 15;

#[inline]
unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
    dt: *const FSE_DTable,
) {
    let DTableH = core::ptr::read_unaligned(dt as *const FSE_DTableHeader);
    (*DStatePtr).state = BIT_readBits(bitD, DTableH.tableLog as u32);
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline]
unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> u8 {
    let DInfo =
        *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as u32;
    let symbol = DInfo.symbol;
    let lowBits = BIT_readBits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> u8 {
    let DInfo =
        *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as u32;
    let symbol = DInfo.symbol;
    let lowBits = BIT_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> c_uint {
    ((*DStatePtr).state == 0) as c_uint
}

/* ****************************************************************
 * FSE decompression
 * ************************************************************** */
type DTable_max_t = [u32; 1 + (1 << FSE_MAX_TABLELOG)];

#[inline]
fn FSE_tableStep(tableSize: u32) -> u32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let ptr = dt.add(1) as *mut c_void;
    let mut DTableH = FSE_DTableHeader { tableLog: 0, fastMode: 0 };
    let tableDecode = ptr as *mut FSE_decode_t;
    let tableSize: u32 = 1 << tableLog;
    let tableMask: u32 = tableSize - 1;
    let step: u32 = FSE_tableStep(tableSize);
    let mut symbolNext: [u16; FSE_MAX_SYMBOL_VALUE + 1] = [0; FSE_MAX_SYMBOL_VALUE + 1];
    let mut position: u32 = 0;
    let mut highThreshold: u32 = tableSize - 1;
    let largeLimit: i16 = (1 << (tableLog - 1)) as i16;
    let mut noLarge: u32 = 1;
    let mut s: u32;

    /* Sanity Checks */
    if maxSymbolValue as usize > FSE_MAX_SYMBOL_VALUE {
        return ERROR(ec::maxSymbolValue_tooLarge);
    }
    if tableLog as usize > FSE_MAX_TABLELOG {
        return ERROR(ec::tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    DTableH.tableLog = tableLog as u16;
    s = 0;
    while s <= maxSymbolValue {
        if *normalizedCounter.add(s as usize) == -1 {
            (*tableDecode.add(highThreshold as usize)).symbol = s as u8;
            highThreshold -= 1;
            symbolNext[s as usize] = 1;
        } else {
            if *normalizedCounter.add(s as usize) >= largeLimit {
                noLarge = 0;
            }
            symbolNext[s as usize] = *normalizedCounter.add(s as usize) as u16;
        }
        s += 1;
    }

    /* Spread symbols */
    s = 0;
    while s <= maxSymbolValue {
        let mut i: c_int = 0;
        while i < *normalizedCounter.add(s as usize) as c_int {
            (*tableDecode.add(position as usize)).symbol = s as u8;
            position = (position + step) & tableMask;
            while position > highThreshold {
                position = (position + step) & tableMask;
            }
            i += 1;
        }
        s += 1;
    }

    if position != 0 {
        return ERROR(ec::GENERIC);
    }

    /* Build Decoding table */
    {
        let mut i: u32 = 0;
        while i < tableSize {
            let symbol = (*tableDecode.add(i as usize)).symbol;
            let nextState = symbolNext[symbol as usize];
            symbolNext[symbol as usize] += 1;
            (*tableDecode.add(i as usize)).nbBits =
                (tableLog - BIT_highbit32(nextState as u32)) as u8;
            (*tableDecode.add(i as usize)).newState =
                (((nextState as u32) << (*tableDecode.add(i as usize)).nbBits) - tableSize) as u16;
            i += 1;
        }
    }

    DTableH.fastMode = noLarge as u16;
    core::ptr::write_unaligned(dt as *mut FSE_DTableHeader, DTableH);
    0
}

#[inline]
fn FSE_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[inline]
fn FSE_abs(a: i16) -> i16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

unsafe fn FSE_readNCount(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const u8;
    let iend = istart.add(hbSize);
    let mut ip = istart;
    let mut nbBits: c_int;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bitStream: u32;
    let mut bitCount: c_int;
    let mut charnum: c_uint = 0;
    let mut previous0: c_int = 0;

    if hbSize < 4 {
        return ERROR(ec::srcSize_wrong);
    }
    bitStream = MEM_readLE32(ip as *const c_void);
    nbBits = (bitStream & 0xF) as c_int + FSE_MIN_TABLELOG as c_int;
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as c_int {
        return ERROR(ec::tableLog_tooLarge);
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
                if ip < iend.offset(-5) {
                    ip = ip.add(2);
                    bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
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
                return ERROR(ec::maxSymbolValue_tooSmall);
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
                bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: i16 = ((2 * threshold - 1) - remaining) as i16;
            let mut count: i16;

            if (bitStream & (threshold - 1) as u32) < max as u32 {
                count = (bitStream & (threshold - 1) as u32) as i16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold - 1) as u32) as i16;
                if count >= threshold as i16 {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1;
            remaining -= FSE_abs(count) as c_int;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as c_int;
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
                    bitCount -= (8 * (iend as isize - 4 - ip as isize)) as c_int;
                    ip = iend.offset(-4);
                }
                bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return ERROR(ec::GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    if (ip as usize - istart as usize) > hbSize {
        return ERROR(ec::srcSize_wrong);
    }
    ip as usize - istart as usize
}

unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: u8) -> usize {
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

unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: c_uint) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let dinfo = (ptr as *mut FSE_decode_t).add(1);
    let tableSize: c_uint = 1 << nbBits;
    let tableMask: c_uint = tableSize - 1;
    let maxSymbolValue: c_uint = tableMask;
    let mut s: c_uint;

    if nbBits < 1 {
        return ERROR(ec::GENERIC);
    }

    (*DTableH).tableLog = nbBits as u16;
    (*DTableH).fastMode = 1;
    s = 0;
    while s <= maxSymbolValue {
        (*dinfo.add(s as usize)).newState = 0;
        (*dinfo.add(s as usize)).symbol = s as u8;
        (*dinfo.add(s as usize)).nbBits = nbBits as u8;
        s += 1;
    }

    0
}

unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: c_uint,
) -> usize {
    let ostart = dst as *mut u8;
    let mut op = ostart;
    let omax = op.add(maxDstSize);
    let olimit = omax.offset(-3);

    let mut bitD: BIT_DStream_t = core::mem::zeroed();
    let mut state1: FSE_DState_t = core::mem::zeroed();
    let mut state2: FSE_DState_t = core::mem::zeroed();
    let errorCode: usize;

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

    /* 4 symbols per loop */
    while (BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit) {
        *op.add(0) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > core::mem::size_of::<usize>() * 8 {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(1) = FSE_GETSYMBOL!(&mut state2);

        if FSE_MAX_TABLELOG * 4 + 7 > core::mem::size_of::<usize>() * 8 {
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > core::mem::size_of::<usize>() * 8 {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(3) = FSE_GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    /* tail */
    loop {
        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0 && (fast != 0 || FSE_endOfDState(&state1) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL!(&mut state1);
        op = op.add(1);

        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0 && (fast != 0 || FSE_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL!(&mut state2);
        op = op.add(1);
    }

    /* end ? */
    if BIT_endOfDStream(&bitD) != 0
        && FSE_endOfDState(&state1) != 0
        && FSE_endOfDState(&state2) != 0
    {
        return op as usize - ostart as usize;
    }

    if op == omax {
        return ERROR(ec::dstSize_tooSmall);
    }

    ERROR(ec::corruption_detected)
}

unsafe fn FSE_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
) -> usize {
    let DTableH = core::ptr::read_unaligned(dt as *const FSE_DTableHeader);

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
    let istart = cSrc as *const u8;
    let mut ip = istart;
    let mut counting: [i16; FSE_MAX_SYMBOL_VALUE + 1] = [0; FSE_MAX_SYMBOL_VALUE + 1];
    let mut dt: DTable_max_t = [0; 1 + (1 << FSE_MAX_TABLELOG)];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSE_MAX_SYMBOL_VALUE as c_uint;
    let mut errorCode: usize;

    if cSrcSize < 2 {
        return ERROR(ec::srcSize_wrong);
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
        return ERROR(ec::srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSE_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

/* ****************************************************************
 * Huff0 : Huffman block decompression
 * ************************************************************** */
#[inline]
fn HUF_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

const HUF_ABSOLUTEMAX_TABLELOG: usize = 16;
const HUF_MAX_TABLELOG: usize = 12;
const HUF_DEFAULT_TABLELOG: usize = HUF_MAX_TABLELOG;
const HUF_MAX_SYMBOL_VALUE: usize = 255;
const HUF_DTABLE_SIZE_MAX: usize = 1 + (1 << HUF_MAX_TABLELOG);

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX2 {
    byte: u8,
    nbBits: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX4 {
    sequence: u16,
    nbBits: u8,
    length: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct sortedSymbol_t {
    symbol: u8,
    weight: u8,
}

unsafe fn HUF_readStats(
    huffWeight: *mut u8,
    hwSize: usize,
    rankStats: *mut u32,
    nbSymbolsPtr: *mut u32,
    tableLogPtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightTotal: u32;
    let mut tableLog: u32;
    let mut ip = src as *const u8;
    let mut iSize: usize;
    let mut oSize: usize;
    let mut n: u32;

    if srcSize == 0 {
        return ERROR(ec::srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            static L: [c_int; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = L[iSize - 242] as usize;
            memset(huffWeight as *mut c_void, 1, hwSize);
            iSize = 0;
        } else {
            /* Incompressible */
            oSize = iSize - 127;
            iSize = (oSize + 1) / 2;
            if iSize + 1 > srcSize {
                return ERROR(ec::srcSize_wrong);
            }
            if oSize >= hwSize {
                return ERROR(ec::corruption_detected);
            }
            ip = ip.add(1);
            n = 0;
            while (n as usize) < oSize {
                *huffWeight.add(n as usize) = *ip.add(n as usize / 2) >> 4;
                *huffWeight.add(n as usize + 1) = *ip.add(n as usize / 2) & 15;
                n += 2;
            }
        }
    } else {
        /* header compressed with FSE */
        if iSize + 1 > srcSize {
            return ERROR(ec::srcSize_wrong);
        }
        oSize = FSE_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.add(1) as *const c_void,
            iSize,
        );
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankStats as *mut c_void,
        0,
        (HUF_ABSOLUTEMAX_TABLELOG + 1) * core::mem::size_of::<u32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if (*huffWeight.add(n as usize) as usize) >= HUF_ABSOLUTEMAX_TABLELOG {
            return ERROR(ec::corruption_detected);
        }
        *rankStats.add(*huffWeight.add(n as usize) as usize) += 1;
        weightTotal += (1u32 << *huffWeight.add(n as usize)) >> 1;
        n += 1;
    }
    if weightTotal == 0 {
        return ERROR(ec::corruption_detected);
    }

    /* get last non-null symbol weight */
    tableLog = BIT_highbit32(weightTotal) + 1;
    if tableLog as usize > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(ec::corruption_detected);
    }
    {
        let total: u32 = 1 << tableLog;
        let rest: u32 = total - weightTotal;
        let verif: u32 = 1 << BIT_highbit32(rest);
        let lastWeight: u32 = BIT_highbit32(rest) + 1;
        if verif != rest {
            return ERROR(ec::corruption_detected);
        }
        *huffWeight.add(oSize) = lastWeight as u8;
        *rankStats.add(lastWeight as usize) += 1;
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ec::corruption_detected);
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as u32;
    *tableLogPtr = tableLog;
    iSize + 1
}

/**************************/
/* single-symbol decoding */
/**************************/

unsafe fn HUF_readDTableX2(DTable: *mut u16, src: *const c_void, srcSize: usize) -> usize {
    let mut huffWeight: [u8; HUF_MAX_SYMBOL_VALUE + 1] = [0; HUF_MAX_SYMBOL_VALUE + 1];
    let mut rankVal: [u32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1];
    let mut tableLog: u32 = 0;
    let _ip = src as *const u8;
    let mut iSize: usize;
    let mut nbSymbols: u32 = 0;
    let mut n: u32;
    let mut nextRankStart: u32;
    let ptr = DTable.add(1) as *mut c_void;
    let dt = ptr as *mut HUF_DEltX2;

    iSize = HUF_readStats(
        huffWeight.as_mut_ptr(),
        HUF_MAX_SYMBOL_VALUE + 1,
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
    if tableLog > *DTable.add(0) as u32 {
        return ERROR(ec::tableLog_tooLarge);
    }
    *DTable.add(0) = tableLog as u16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current = nextRankStart;
        nextRankStart += rankVal[n as usize] << (n - 1);
        rankVal[n as usize] = current;
        n += 1;
    }

    /* fill DTable */
    n = 0;
    while n < nbSymbols {
        let w = huffWeight[n as usize] as u32;
        let length = (1u32 << w) >> 1;
        let mut i: u32;
        let mut d = HUF_DEltX2 { byte: 0, nbBits: 0 };
        d.byte = n as u8;
        d.nbBits = (tableLog + 1 - w) as u8;
        i = rankVal[w as usize];
        while i < rankVal[w as usize] + length {
            *dt.add(i as usize) = d;
            i += 1;
        }
        rankVal[w as usize] += length;
        n += 1;
    }

    iSize
}

#[inline]
unsafe fn HUF_decodeSymbolX2(
    dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: u32,
) -> u8 {
    let val = BIT_lookBitsFast(dstream, dtLog);
    let c = (*dt.add(val)).byte;
    BIT_skipBits(dstream, (*dt.add(val)).nbBits as u32);
    c
}

macro_rules! HUF_DECODE_SYMBOLX2_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUF_decodeSymbolX2($DStreamPtr, $dt, $dtLog);
        $ptr = $ptr.add(1);
    }};
}
macro_rules! HUF_DECODE_SYMBOLX2_1 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUF_MAX_TABLELOG <= 12) {
            HUF_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    }};
}
macro_rules! HUF_DECODE_SYMBOLX2_2 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            HUF_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    }};
}

unsafe fn HUF_decodeStreamX2(
    mut p: *mut u8,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut u8,
    dt: *const HUF_DEltX2,
    dtLog: u32,
) -> usize {
    let pStart = p;

    /* up to 4 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.offset(-4)) {
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd) {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* no more data to retrieve from bitstream */
    while p < pEnd {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    pEnd as usize - pStart as usize
}

unsafe fn HUF_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const u16,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ec::corruption_detected);
    }

    {
        let istart = cSrc as *const u8;
        let ostart = dst as *mut u8;
        let oend = ostart.add(dstSize);

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DEltX2).add(1);
        let dtLog = *DTable.add(0) as u32;
        let mut errorCode: usize;

        let mut bitD1: BIT_DStream_t = core::mem::zeroed();
        let mut bitD2: BIT_DStream_t = core::mem::zeroed();
        let mut bitD3: BIT_DStream_t = core::mem::zeroed();
        let mut bitD4: BIT_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart as *const c_void) as usize;
        let length2 = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3 = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.add(6);
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segmentSize = (dstSize + 3) / 4;
        let opStart2 = ostart.add(segmentSize);
        let opStart3 = opStart2.add(segmentSize);
        let opStart4 = opStart3.add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: u32;

        length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
        if length4 > cSrcSize {
            return ERROR(ec::corruption_detected);
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

        /* 16-32 symbols per loop */
        endSignal = (BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4)) as u32;
        while (endSignal == BIT_DStream_unfinished as u32) && (op4 < oend.offset(-7)) {
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

            endSignal = (BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4)) as u32;
        }

        /* check corruption */
        if op1 > opStart2 {
            return ERROR(ec::corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ec::corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ec::corruption_detected);
        }

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
            return ERROR(ec::corruption_detected);
        }

        dstSize
    }
}

unsafe fn HUF_decompress4X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [u16; HUF_DTABLE_SIZE_MAX] = [0; HUF_DTABLE_SIZE_MAX];
    DTable[0] = HUF_MAX_TABLELOG as u16;
    let mut ip = cSrc as *const u8;
    let errorCode: usize;

    errorCode = HUF_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ec::srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUF_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/***************************/
/* double-symbols decoding */
/***************************/

unsafe fn HUF_fillDTableX4Level2(
    DTable: *mut HUF_DEltX4,
    sizeLog: u32,
    consumed: u32,
    rankValOrigin: *const u32,
    minWeight: c_int,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: u32,
    nbBitsBaseline: u32,
    baseSeq: u16,
) {
    let mut DElt = HUF_DEltX4 { sequence: 0, nbBits: 0, length: 0 };
    let mut rankVal: [u32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1];
    let mut s: u32;

    /* get pre-calculated rankVal */
    core::ptr::copy_nonoverlapping(
        rankValOrigin,
        rankVal.as_mut_ptr(),
        HUF_ABSOLUTEMAX_TABLELOG + 1,
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: u32;
        let skipSize = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut u16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as u8;
        DElt.length = 1;
        i = 0;
        while i < skipSize {
            *DTable.add(i as usize) = DElt;
            i += 1;
        }
    }

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol = (*sortedSymbols.add(s as usize)).symbol as u32;
        let weight = (*sortedSymbols.add(s as usize)).weight as u32;
        let nbBits = nbBitsBaseline - weight;
        let length = 1u32 << (sizeLog - nbBits);
        let start = rankVal[weight as usize];
        let mut i = start;
        let end = start + length;

        MEM_writeLE16(
            &mut DElt.sequence as *mut u16 as *mut c_void,
            (baseSeq as u32 + (symbol << 8)) as u16,
        );
        DElt.nbBits = (nbBits + consumed) as u8;
        DElt.length = 2;
        loop {
            *DTable.add(i as usize) = DElt;
            i += 1;
            if i >= end {
                break;
            }
        }

        rankVal[weight as usize] += length;
        s += 1;
    }
}

type rankVal_t = [[u32; HUF_ABSOLUTEMAX_TABLELOG + 1]; HUF_ABSOLUTEMAX_TABLELOG];

unsafe fn HUF_fillDTableX4(
    DTable: *mut HUF_DEltX4,
    targetLog: u32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: u32,
    rankStart: *const u32,
    rankValOrigin: *mut rankVal_t,
    maxWeight: u32,
    nbBitsBaseline: u32,
) {
    let mut rankVal: [u32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1];
    let scaleLog: c_int = nbBitsBaseline as c_int - targetLog as c_int;
    let minBits: u32 = nbBitsBaseline - maxWeight;
    let mut s: u32;

    core::ptr::copy_nonoverlapping(
        (*rankValOrigin)[0].as_ptr(),
        rankVal.as_mut_ptr(),
        HUF_ABSOLUTEMAX_TABLELOG + 1,
    );

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol = (*sortedList.add(s as usize)).symbol as u16;
        let weight = (*sortedList.add(s as usize)).weight as u32;
        let nbBits = nbBitsBaseline - weight;
        let start = rankVal[weight as usize];
        let length = 1u32 << (targetLog - nbBits);

        if targetLog - nbBits >= minBits {
            /* enough room for a second symbol */
            let sortedRank: u32;
            let mut minWeight: c_int = nbBits as c_int + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUF_fillDTableX4Level2(
                DTable.add(start as usize),
                targetLog - nbBits,
                nbBits,
                (*rankValOrigin)[nbBits as usize].as_ptr(),
                minWeight,
                sortedList.add(sortedRank as usize),
                sortedListSize - sortedRank,
                nbBitsBaseline,
                symbol,
            );
        } else {
            let mut i: u32;
            let end = start + length;
            let mut DElt = HUF_DEltX4 { sequence: 0, nbBits: 0, length: 0 };

            MEM_writeLE16(&mut DElt.sequence as *mut u16 as *mut c_void, symbol);
            DElt.nbBits = nbBits as u8;
            DElt.length = 1;
            i = start;
            while i < end {
                *DTable.add(i as usize) = DElt;
                i += 1;
            }
        }
        rankVal[weight as usize] += length;
        s += 1;
    }
}

unsafe fn HUF_readDTableX4(DTable: *mut u32, src: *const c_void, srcSize: usize) -> usize {
    let mut weightList: [u8; HUF_MAX_SYMBOL_VALUE + 1] = [0; HUF_MAX_SYMBOL_VALUE + 1];
    let mut sortedSymbol: [sortedSymbol_t; HUF_MAX_SYMBOL_VALUE + 1] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; HUF_MAX_SYMBOL_VALUE + 1];
    let mut rankStats: [u32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1];
    let mut rankStart0: [u32; HUF_ABSOLUTEMAX_TABLELOG + 2] = [0; HUF_ABSOLUTEMAX_TABLELOG + 2];
    let rankStart = rankStart0.as_mut_ptr().add(1);
    let mut rankVal: rankVal_t = [[0; HUF_ABSOLUTEMAX_TABLELOG + 1]; HUF_ABSOLUTEMAX_TABLELOG];
    let mut tableLog: u32 = 0;
    let mut maxW: u32;
    let mut sizeOfSort: u32 = 0;
    let mut nbSymbols: u32 = 0;
    let memLog = *DTable.add(0);
    let _ip = src as *const u8;
    let mut iSize: usize;
    let ptr = DTable as *mut c_void;
    let dt = (ptr as *mut HUF_DEltX4).add(1);

    if memLog as usize > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(ec::tableLog_tooLarge);
    }

    iSize = HUF_readStats(
        weightList.as_mut_ptr(),
        HUF_MAX_SYMBOL_VALUE + 1,
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
        return ERROR(ec::tableLog_tooLarge);
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        if maxW == 0 {
            return ERROR(ec::GENERIC);
        }
        maxW -= 1;
    }

    /* Get start index of each weight */
    {
        let mut w: u32;
        let mut nextRankStart: u32 = 0;
        w = 1;
        while w <= maxW {
            let current = nextRankStart;
            nextRankStart += rankStats[w as usize];
            *rankStart.add(w as usize) = current;
            w += 1;
        }
        *rankStart.add(0) = nextRankStart;
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: u32;
        s = 0;
        while s < nbSymbols {
            let w = weightList[s as usize] as u32;
            let r = *rankStart.add(w as usize);
            *rankStart.add(w as usize) += 1;
            sortedSymbol[r as usize].symbol = s as u8;
            sortedSymbol[r as usize].weight = w as u8;
            s += 1;
        }
        *rankStart.add(0) = 0;
    }

    /* Build rankVal */
    {
        let minBits: u32 = tableLog + 1 - maxW;
        let mut nextRankVal: u32 = 0;
        let mut w: u32;
        let mut consumed: u32;
        let rescale: c_int = (memLog as c_int - tableLog as c_int) - 1;
        w = 1;
        while w <= maxW {
            let current = nextRankVal;
            nextRankVal += rankStats[w as usize] << (w as c_int + rescale);
            rankVal[0][w as usize] = current;
            w += 1;
        }
        consumed = minBits;
        while consumed <= memLog - minBits {
            w = 1;
            while w <= maxW {
                rankVal[consumed as usize][w as usize] = rankVal[0][w as usize] >> consumed;
                w += 1;
            }
            consumed += 1;
        }
    }

    HUF_fillDTableX4(
        dt,
        memLog,
        sortedSymbol.as_ptr(),
        sizeOfSort,
        rankStart0.as_ptr(),
        &mut rankVal as *mut rankVal_t,
        maxW,
        tableLog + 1,
    );

    iSize
}

#[inline]
unsafe fn HUF_decodeSymbolX4(
    op: *mut c_void,
    dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: u32,
) -> u32 {
    let val = BIT_lookBitsFast(dstream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 2);
    BIT_skipBits(dstream, (*dt.add(val)).nbBits as u32);
    (*dt.add(val)).length as u32
}

#[inline]
unsafe fn HUF_decodeLastSymbolX4(
    op: *mut c_void,
    dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: u32,
) -> u32 {
    let val = BIT_lookBitsFast(dstream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BIT_skipBits(dstream, (*dt.add(val)).nbBits as u32);
    } else {
        if (*dstream).bitsConsumed < (core::mem::size_of::<usize>() * 8) as u32 {
            BIT_skipBits(dstream, (*dt.add(val)).nbBits as u32);
            if (*dstream).bitsConsumed > (core::mem::size_of::<usize>() * 8) as u32 {
                (*dstream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as u32;
            }
        }
    }
    1
}

macro_rules! HUF_DECODE_SYMBOLX4_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.add(HUF_decodeSymbolX4($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize);
    }};
}
macro_rules! HUF_DECODE_SYMBOLX4_1 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUF_MAX_TABLELOG <= 12) {
            $ptr = $ptr
                .add(HUF_decodeSymbolX4($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize);
        }
    }};
}
macro_rules! HUF_DECODE_SYMBOLX4_2 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            $ptr = $ptr
                .add(HUF_decodeSymbolX4($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize);
        }
    }};
}

unsafe fn HUF_decodeStreamX4(
    mut p: *mut u8,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut u8,
    dt: *const HUF_DEltX4,
    dtLog: u32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd.offset(-7)) {
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.offset(-2)) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p <= pEnd.offset(-2) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    if p < pEnd {
        p = p.add(HUF_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    p as usize - pStart as usize
}

unsafe fn HUF_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const u32,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ec::corruption_detected);
    }

    {
        let istart = cSrc as *const u8;
        let ostart = dst as *mut u8;
        let oend = ostart.add(dstSize);

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DEltX4).add(1);
        let dtLog = *DTable.add(0);
        let mut errorCode: usize;

        let mut bitD1: BIT_DStream_t = core::mem::zeroed();
        let mut bitD2: BIT_DStream_t = core::mem::zeroed();
        let mut bitD3: BIT_DStream_t = core::mem::zeroed();
        let mut bitD4: BIT_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart as *const c_void) as usize;
        let length2 = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3 = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.add(6);
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segmentSize = (dstSize + 3) / 4;
        let opStart2 = ostart.add(segmentSize);
        let opStart3 = opStart2.add(segmentSize);
        let opStart4 = opStart3.add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: u32;

        length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
        if length4 > cSrcSize {
            return ERROR(ec::corruption_detected);
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

        /* 16-32 symbols per loop */
        endSignal = (BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4)) as u32;
        while (endSignal == BIT_DStream_unfinished as u32) && (op4 < oend.offset(-7)) {
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

            endSignal = (BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4)) as u32;
        }

        /* check corruption */
        if op1 > opStart2 {
            return ERROR(ec::corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ec::corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ec::corruption_detected);
        }

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
            return ERROR(ec::corruption_detected);
        }

        dstSize
    }
}

unsafe fn HUF_decompress4X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [u32; HUF_DTABLE_SIZE_MAX] = [0; HUF_DTABLE_SIZE_MAX];
    DTable[0] = HUF_MAX_TABLELOG as u32;
    let mut ip = cSrc as *const u8;

    let hSize = HUF_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ec::srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/**********************************/
/* Generic decompression selector */
/**********************************/

#[repr(C)]
#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: u32,
    decode256Time: u32,
}

static algoTime: [[algo_time_t; 3]; 16] = [
    [at(0, 0), at(1, 1), at(2, 2)],
    [at(0, 0), at(1, 1), at(2, 2)],
    [at(38, 130), at(1313, 74), at(2151, 38)],
    [at(448, 128), at(1353, 74), at(2238, 41)],
    [at(556, 128), at(1353, 74), at(2238, 47)],
    [at(714, 128), at(1418, 74), at(2436, 53)],
    [at(883, 128), at(1437, 74), at(2464, 61)],
    [at(897, 128), at(1515, 75), at(2622, 68)],
    [at(926, 128), at(1613, 75), at(2730, 75)],
    [at(947, 128), at(1729, 77), at(3359, 77)],
    [at(1107, 128), at(2083, 81), at(4006, 84)],
    [at(1177, 128), at(2379, 87), at(4785, 88)],
    [at(1242, 128), at(2415, 93), at(5155, 84)],
    [at(1349, 128), at(2644, 106), at(5260, 106)],
    [at(1455, 128), at(2422, 124), at(4174, 124)],
    [at(722, 128), at(1891, 145), at(1936, 146)],
];

const fn at(tableTime: u32, decode256Time: u32) -> algo_time_t {
    algo_time_t {
        tableTime,
        decode256Time,
    }
}

unsafe fn HUF_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* estimate decompression time */
    let Q: u32;
    let D256: u32 = (dstSize >> 8) as u32;
    let mut Dtime: [u32; 3] = [0; 3];
    let mut algoNb: u32 = 0;
    let mut n: c_int;

    /* validation checks */
    if dstSize == 0 {
        return ERROR(ec::dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ec::corruption_detected);
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const u8) as c_int, dstSize);
        return dstSize;
    }

    /* decoder timing evaluation */
    Q = (cSrcSize * 16 / dstSize) as u32;
    n = 0;
    while n < 3 {
        Dtime[n as usize] = algoTime[Q as usize][n as usize].tableTime
            + (algoTime[Q as usize][n as usize].decode256Time * D256);
        n += 1;
    }

    Dtime[1] += Dtime[1] >> 4;
    Dtime[2] += Dtime[2] >> 3;

    if Dtime[1] < Dtime[0] {
        algoNb = 1;
    }

    if algoNb == 0 {
        HUF_decompress4X2(dst, dstSize, cSrc, cSrcSize)
    } else {
        HUF_decompress4X4(dst, dstSize, cSrc, cSrcSize)
    }
}

/* ****************************************************************
 * zstd decompression
 * ************************************************************** */
const ZSTD_MAGICNUMBER: u32 = 0xFD2FB523;

const FSE_DTABLE_SIZE_U32_LLFSELog: usize = 1 + (1 << LLFSELog);
const FSE_DTABLE_SIZE_U32_OffFSELog: usize = 1 + (1 << OffFSELog);
const FSE_DTABLE_SIZE_U32_MLFSELog: usize = 1 + (1 << MLFSELog);

const HASH_LOG: usize = ZSTD_MEMORY_USAGE - 2;
const HASH_TABLESIZE: usize = 1 << HASH_LOG;
const HASH_MASK: usize = HASH_TABLESIZE - 1;
const ZSTD_MEMORY_USAGE: usize = 17;

const BLOCKSIZE: usize = 128 * (1 << 10);
const MIN_SEQUENCES_SIZE: usize = 2 + 2 + 3 + 1;
const MIN_CBLOCK_SIZE: usize = 3 + MIN_SEQUENCES_SIZE;
const IS_RAW: u8 = 1; /* BIT0 */
const IS_RLE: u8 = 2; /* BIT1 */

const MINMATCH: usize = 4;
const MLbits: u32 = 7;
const LLbits: u32 = 6;
const Offbits: u32 = 5;
const MaxML: usize = (1 << MLbits) - 1;
const MaxLL: usize = (1 << LLbits) - 1;
const MaxOff: usize = 31;
const LitFSELog: usize = 11;
const MLFSELog: usize = 10;
const LLFSELog: usize = 10;
const OffFSELog: usize = 9;

const ZSTD_CONTENTSIZE_ERROR: c_ulonglong = 0u64.wrapping_sub(2);

const ZSTD_blockHeaderSize: usize = 3;
const ZSTD_frameHeaderSize: usize = 4;

#[inline]
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}
#[inline]
unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const u8;
    let mut op = dst as *mut u8;
    let oend = op.offset(length);
    loop {
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if op >= oend {
            break;
        }
    }
}

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
    origSize: u32,
}

#[inline]
fn ZSTD_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/* Decompression section */
#[repr(C)]
pub struct ZSTDv03_Dctx_s {
    LLTable: [u32; FSE_DTABLE_SIZE_U32_LLFSELog],
    OffTable: [u32; FSE_DTABLE_SIZE_U32_OffFSELog],
    MLTable: [u32; FSE_DTABLE_SIZE_U32_MLFSELog],
    previousDstEnd: *mut c_void,
    base: *mut c_void,
    expected: usize,
    bType: blockType_t,
    phase: u32,
    litPtr: *const u8,
    litSize: usize,
    litBuffer: [u8; BLOCKSIZE + 8],
}

type ZSTD_DCtx = ZSTDv03_Dctx_s;

unsafe fn ZSTD_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_ = src as *const u8;
    let headerFlags: u8;
    let cSize: u32;

    if srcSize < 3 {
        return ERROR(ec::srcSize_wrong);
    }

    headerFlags = *in_;
    cSize = *in_.add(2) as u32 + ((*in_.add(1) as u32) << 8) + (((*in_.add(0) as u32) & 7) << 16);

    (*bpPtr).blockType = core::mem::transmute::<u32, blockType_t>((headerFlags >> 6) as u32);
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
        return ERROR(ec::dstSize_tooSmall);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

unsafe fn ZSTD_decompressLiterals(
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip = src as *const u8;

    let litSize = (MEM_readLE32(src) & 0x1FFFFF) as usize >> 2;
    let litCSize = (MEM_readLE32(ip.add(2) as *const c_void) & 0xFFFFFF) as usize >> 5;

    if litSize > *maxDstSizePtr {
        return ERROR(ec::corruption_detected);
    }
    if litCSize + 5 > srcSize {
        return ERROR(ec::corruption_detected);
    }

    if HUF_isError(HUF_decompress(dst, litSize, ip.add(5) as *const c_void, litCSize)) != 0 {
        return ERROR(ec::corruption_detected);
    }

    *maxDstSizePtr = litSize;
    litCSize + 5
}

unsafe fn ZSTD_decodeLiteralsBlock(
    ctx: *mut c_void,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let dctx = ctx as *mut ZSTD_DCtx;
    let istart = src as *const u8;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ec::corruption_detected);
    }

    match *istart & 3 {
        x if x == IS_RAW => {
            let litSize = (MEM_readLE32(istart as *const c_void) & 0xFFFFFF) as usize >> 2;
            if litSize > srcSize - 11 {
                if litSize > BLOCKSIZE {
                    return ERROR(ec::corruption_detected);
                }
                if litSize > srcSize - 3 {
                    return ERROR(ec::corruption_detected);
                }
                memcpy(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    istart as *const c_void,
                    litSize,
                );
                (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
                (*dctx).litSize = litSize;
                memset(
                    (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                    0,
                    8,
                );
                return litSize + 3;
            }
            (*dctx).litPtr = istart.add(3);
            (*dctx).litSize = litSize;
            litSize + 3
        }
        x if x == IS_RLE => {
            let litSize = (MEM_readLE32(istart as *const c_void) & 0xFFFFFF) as usize >> 2;
            if litSize > BLOCKSIZE {
                return ERROR(ec::corruption_detected);
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
        _ => {
            /* default / case 0 */
            let mut litSize = BLOCKSIZE;
            let readSize =
                ZSTD_decompressLiterals((*dctx).litBuffer.as_mut_ptr() as *mut c_void, &mut litSize, src, srcSize);
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                8,
            );
            readSize
        }
    }
}

unsafe fn ZSTD_decodeSeqHeaders(
    nbSeq: *mut c_int,
    dumpsPtr: *mut *const u8,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut FSE_DTable,
    DTableML: *mut FSE_DTable,
    DTableOffb: *mut FSE_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const u8;
    let mut ip = istart;
    let iend = istart.add(srcSize);
    let LLtype: u32;
    let Offtype: u32;
    let MLtype: u32;
    let mut LLlog: u32 = 0;
    let mut Offlog: u32 = 0;
    let mut MLlog: u32 = 0;
    let dumpsLength: usize;

    if srcSize < 5 {
        return ERROR(ec::srcSize_wrong);
    }

    /* SeqHead */
    *nbSeq = MEM_readLE16(ip as *const c_void) as c_int;
    ip = ip.add(2);
    LLtype = (*ip >> 6) as u32;
    Offtype = ((*ip >> 4) & 3) as u32;
    MLtype = ((*ip >> 2) & 3) as u32;
    if (*ip & 2) != 0 {
        dumpsLength = *ip.add(2) as usize;
        let dl = dumpsLength + ((*ip.add(1) as usize) << 8);
        ip = ip.add(3);
        *dumpsPtr = ip;
        ip = ip.add(dl);
        *dumpsLengthPtr = dl;
    } else {
        dumpsLength = *ip.add(1) as usize;
        let dl = dumpsLength + (((*ip.add(0) as usize) & 1) << 8);
        ip = ip.add(2);
        *dumpsPtr = ip;
        ip = ip.add(dl);
        *dumpsLengthPtr = dl;
    }

    /* check */
    if ip > iend.offset(-3) {
        return ERROR(ec::srcSize_wrong);
    }

    /* sequences */
    {
        let mut norm: [i16; MaxML + 1] = [0; MaxML + 1];
        let mut headerSize: usize;

        /* Build DTables */
        match LLtype {
            x if x == blockType_t::bt_rle as u32 => {
                LLlog = 0;
                FSE_buildDTable_rle(DTableLL, *ip);
                ip = ip.add(1);
            }
            x if x == blockType_t::bt_raw as u32 => {
                LLlog = LLbits;
                FSE_buildDTable_raw(DTableLL, LLbits);
            }
            _ => {
                let mut max: c_uint = MaxLL as c_uint;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut LLlog,
                    ip as *const c_void,
                    iend as usize - ip as usize,
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR(ec::GENERIC);
                }
                if LLlog as usize > LLFSELog {
                    return ERROR(ec::corruption_detected);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        match Offtype {
            x if x == blockType_t::bt_rle as u32 => {
                Offlog = 0;
                if ip > iend.offset(-2) {
                    return ERROR(ec::srcSize_wrong);
                }
                FSE_buildDTable_rle(DTableOffb, *ip & MaxOff as u8);
                ip = ip.add(1);
            }
            x if x == blockType_t::bt_raw as u32 => {
                Offlog = Offbits;
                FSE_buildDTable_raw(DTableOffb, Offbits);
            }
            _ => {
                let mut max: c_uint = MaxOff as c_uint;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut Offlog,
                    ip as *const c_void,
                    iend as usize - ip as usize,
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR(ec::GENERIC);
                }
                if Offlog as usize > OffFSELog {
                    return ERROR(ec::corruption_detected);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
            }
        }

        match MLtype {
            x if x == blockType_t::bt_rle as u32 => {
                MLlog = 0;
                if ip > iend.offset(-2) {
                    return ERROR(ec::srcSize_wrong);
                }
                FSE_buildDTable_rle(DTableML, *ip);
                ip = ip.add(1);
            }
            x if x == blockType_t::bt_raw as u32 => {
                MLlog = MLbits;
                FSE_buildDTable_raw(DTableML, MLbits);
            }
            _ => {
                let mut max: c_uint = MaxML as c_uint;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut MLlog,
                    ip as *const c_void,
                    iend as usize - ip as usize,
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR(ec::GENERIC);
                }
                if MLlog as usize > MLFSELog {
                    return ERROR(ec::corruption_detected);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
            }
        }
    }

    ip as usize - istart as usize
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
    DStream: BIT_DStream_t,
    stateLL: FSE_DState_t,
    stateOffb: FSE_DState_t,
    stateML: FSE_DState_t,
    prevOffset: usize,
    dumps: *const u8,
    dumpsEnd: *const u8,
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
    if litLength == MaxLL {
        let add: u32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.add(1);
            v as u32
        } else {
            0
        };
        if add < 255 {
            litLength += add as usize;
        } else if dumps.add(3) <= de {
            litLength = MEM_readLE24(dumps as *const c_void) as usize;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.offset(-1);
        }
    }

    /* Offset */
    {
        static offsetPrefix: [usize; MaxOff + 1] = [
            1, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
            131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, 1, 1,
            1, 1, 1,
        ];
        let offsetCode: u32;
        let mut nbBits: u32;
        offsetCode =
            FSE_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream) as u32;
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        nbBits = offsetCode - 1;
        if offsetCode == 0 {
            nbBits = 0;
        }
        offset = offsetPrefix[offsetCode as usize]
            + BIT_readBits(&mut (*seqState).DStream, nbBits);
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset;
        }
    }

    /* MatchLength */
    matchLength = FSE_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as usize;
    if matchLength == MaxML {
        let add: u32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.add(1);
            v as u32
        } else {
            0
        };
        if add < 255 {
            matchLength += add as usize;
        } else if dumps.add(3) <= de {
            matchLength = MEM_readLE24(dumps as *const c_void) as usize;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.offset(-1);
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
    mut op: *mut u8,
    sequence: seq_t,
    litPtr: *mut *const u8,
    litLimit: *const u8,
    base: *mut u8,
    oend: *mut u8,
) -> usize {
    static dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
    static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
    let ostart = op;
    let oLitEnd = op.add(sequence.litLength);
    let oMatchEnd = op.add(sequence.litLength + sequence.matchLength);
    let oend_8 = oend.offset(-8);
    let litEnd = (*litPtr).add(sequence.litLength);

    /* checks */
    let seqLength: usize = sequence.litLength + sequence.matchLength;

    if seqLength > (oend as usize - op as usize) {
        return ERROR(ec::dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize - *litPtr as usize) {
        return ERROR(ec::corruption_detected);
    }
    if oLitEnd > oend_8 {
        return ERROR(ec::dstSize_tooSmall);
    }
    if sequence.offset > (oLitEnd as usize - base as usize) as u32 as usize {
        return ERROR(ec::corruption_detected);
    }

    if oMatchEnd > oend {
        return ERROR(ec::dstSize_tooSmall);
    }
    if litEnd > litLimit {
        return ERROR(ec::corruption_detected);
    }

    /* copy Literals */
    ZSTD_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = litEnd;

    /* copy Match */
    {
        let mut match_ = op.offset(-(sequence.offset as isize));

        if sequence.offset > op as usize {
            return ERROR(ec::corruption_detected);
        }
        if match_ < base {
            return ERROR(ec::corruption_detected);
        }

        /* close range match, overlap */
        if sequence.offset < 8 {
            let dec64 = dec64table[sequence.offset];
            *op.add(0) = *match_.add(0);
            *op.add(1) = *match_.add(1);
            *op.add(2) = *match_.add(2);
            *op.add(3) = *match_.add(3);
            match_ = match_.add(dec32table[sequence.offset] as usize);
            ZSTD_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
            match_ = match_.offset(-(dec64 as isize));
        } else {
            ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
        }
        op = op.add(8);
        match_ = match_.add(8);

        if oMatchEnd > oend.offset(-((16 - MINMATCH) as isize)) {
            if op < oend_8 {
                ZSTD_wildcopy(
                    op as *mut c_void,
                    match_ as *const c_void,
                    oend_8 as isize - op as isize,
                );
                match_ = match_.add(oend_8 as usize - op as usize);
                op = oend_8;
            }
            while op < oMatchEnd {
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
    }

    oMatchEnd as usize - ostart as usize
}

unsafe fn ZSTD_decompressSequences(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
) -> usize {
    let dctx = ctx as *mut ZSTD_DCtx;
    let mut ip = seqStart as *const u8;
    let iend = ip.add(seqSize);
    let ostart = dst as *mut u8;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut errorCode: usize;
    let mut dumpsLength: usize = 0;
    let mut litPtr = (*dctx).litPtr;
    let litEnd = litPtr.add((*dctx).litSize);
    let mut nbSeq: c_int = 0;
    let mut dumps: *const u8 = core::ptr::null();
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *mut u8;

    /* Build Decoding Tables */
    errorCode = ZSTD_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        iend as usize - ip as usize,
    );
    if ZSTD_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.add(errorCode);

    /* Regen sequences */
    {
        let mut sequence: seq_t = core::mem::zeroed();
        let mut seqState: seqState_t = core::mem::zeroed();

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.add(dumpsLength);
        sequence.offset = 4;
        seqState.prevOffset = sequence.offset;
        errorCode = BIT_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            iend as usize - ip as usize,
        );
        if ERR_isError(errorCode) != 0 {
            return ERROR(ec::corruption_detected);
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
            op = op.add(oneSeqSize);
        }

        /* check if reached exact end */
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ec::corruption_detected);
        }
        if nbSeq < 0 {
            return ERROR(ec::corruption_detected);
        }

        /* last literal segment */
        {
            let lastLLSize = litEnd as usize - litPtr as usize;
            if litPtr > litEnd {
                return ERROR(ec::corruption_detected);
            }
            if op.add(lastLLSize) > oend {
                return ERROR(ec::dstSize_tooSmall);
            }
            if lastLLSize > 0 {
                if op != litPtr as *mut u8 {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.add(lastLLSize);
            }
        }
    }

    op as usize - ostart as usize
}

unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    let mut ip = src as *const u8;

    /* Decode literals sub-block */
    let litCSize = ZSTD_decodeLiteralsBlock(ctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.add(litCSize);
    srcSize -= litCSize;

    ZSTD_decompressSequences(ctx, dst, maxDstSize, ip as *const c_void, srcSize)
}

unsafe fn ZSTD_decompressDCtx(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const u8;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut u8;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut remainingSize = srcSize;
    let magicNumber: u32;
    let mut blockProperties = blockProperties_t {
        blockType: blockType_t::bt_compressed,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        return ERROR(ec::srcSize_wrong);
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_MAGICNUMBER {
        return ERROR(ec::prefix_unknown);
    }
    ip = ip.add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    /* Loop on each block */
    loop {
        let mut decodedSize: usize = 0;
        let cBlockSize = ZSTD_getcBlockSize(
            ip as *const c_void,
            iend as usize - ip as usize,
            &mut blockProperties,
        );
        if ZSTD_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(ec::srcSize_wrong);
        }

        match blockProperties.blockType {
            blockType_t::bt_compressed => {
                decodedSize = ZSTD_decompressBlock(
                    ctx,
                    op as *mut c_void,
                    oend as usize - op as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            blockType_t::bt_raw => {
                decodedSize = ZSTD_copyUncompressedBlock(
                    op as *mut c_void,
                    oend as usize - op as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            blockType_t::bt_rle => {
                return ERROR(ec::GENERIC);
            }
            blockType_t::bt_end => {
                /* end of frame */
                if remainingSize != 0 {
                    return ERROR(ec::srcSize_wrong);
                }
            }
        }
        if cBlockSize == 0 {
            break;
        }

        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op as usize - ostart as usize
}

unsafe fn ZSTD_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ctx: ZSTD_DCtx = core::mem::zeroed();
    ctx.base = dst;
    ZSTD_decompressDCtx(&mut ctx as *mut ZSTD_DCtx as *mut c_void, dst, maxDstSize, src, srcSize)
}

#[inline]
unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
    ret: usize,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
) {
    let mut ip = src as *const u8;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;
    let magicNumber: u32;
    let mut blockProperties = blockProperties_t {
        blockType: blockType_t::bt_compressed,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::srcSize_wrong));
        return;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_MAGICNUMBER {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::prefix_unknown));
        return;
    }
    ip = ip.add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    /* Loop on each block */
    loop {
        let cBlockSize =
            ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break;
        }

        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = ip as usize - src as usize;
    *dBound = (nbBlocks * BLOCKSIZE) as c_ulonglong;
}

/* Streaming Decompression API */

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
        return ERROR(ec::srcSize_wrong);
    }
    if dst != (*ctx).previousDstEnd {
        (*ctx).base = dst;
    }

    /* Decompress : frame header */
    if (*ctx).phase == 0 {
        let magicNumber: u32 = MEM_readLE32(src);
        if magicNumber != ZSTD_MAGICNUMBER {
            return ERROR(ec::prefix_unknown);
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
        let blockSize = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
        if ZSTD_isError(blockSize) != 0 {
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
                return ERROR(ec::GENERIC);
            }
            blockType_t::bt_end => {
                rSize = 0;
            }
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTD_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = (dst as *mut c_char).add(rSize) as *mut c_void;
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
pub unsafe extern "C" fn ZSTDv03_createDCtx() -> *mut ZSTDv03_Dctx_s {
    ZSTD_createDCtx() as *mut ZSTDv03_Dctx_s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_freeDCtx(dctx: *mut ZSTDv03_Dctx_s) -> usize {
    ZSTD_freeDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_resetDCtx(dctx: *mut ZSTDv03_Dctx_s) -> usize {
    ZSTD_resetDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_nextSrcSizeToDecompress(dctx: *mut ZSTDv03_Dctx_s) -> usize {
    ZSTD_nextSrcSizeToDecompress(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_decompressContinue(
    dctx: *mut ZSTDv03_Dctx_s,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompressContinue(dctx as *mut ZSTD_DCtx, dst, maxDstSize, src, srcSize)
}




