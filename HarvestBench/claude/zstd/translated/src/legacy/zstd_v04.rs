//! Translation of legacy/zstd_v04.c — decompression of zstd v0.4.x legacy frames.
//! Self-contained: defines its own MEM/BIT/FSE/HUF/ZSTD/ZBUFF internals.
//! Target platform: little-endian 64-bit. Only the 17 listed symbols are exported.
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::common::allocations::{free, malloc, memcpy, memmove, memset};
use crate::common::error::{code, err_is_error, error};

// ----------------------------------------------------------------------------
// Basic types (mirroring the C typedefs)
// ----------------------------------------------------------------------------
type BYTE = u8;
type U16 = u16;
type S16 = i16;
type U32 = u32;
type S32 = i32;
type U64 = u64;
type S64 = i64;

#[inline]
const fn MEM_32bits() -> u32 {
    (core::mem::size_of::<usize>() == 4) as u32
}
#[inline]
const fn MEM_64bits() -> u32 {
    (core::mem::size_of::<usize>() == 8) as u32
}
#[inline]
const fn MEM_isLittleEndian() -> u32 {
    cfg!(target_endian = "little") as u32
}

#[inline]
unsafe fn MEM_read16(memPtr: *const c_void) -> U16 {
    let mut val: U16 = 0;
    core::ptr::copy_nonoverlapping(memPtr as *const u8, &mut val as *mut U16 as *mut u8, 2);
    val
}
#[inline]
unsafe fn MEM_read32(memPtr: *const c_void) -> U32 {
    let mut val: U32 = 0;
    core::ptr::copy_nonoverlapping(memPtr as *const u8, &mut val as *mut U32 as *mut u8, 4);
    val
}
#[inline]
unsafe fn MEM_read64(memPtr: *const c_void) -> U64 {
    let mut val: U64 = 0;
    core::ptr::copy_nonoverlapping(memPtr as *const u8, &mut val as *mut U64 as *mut u8, 8);
    val
}
#[inline]
unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    core::ptr::copy_nonoverlapping(&value as *const U16 as *const u8, memPtr as *mut u8, 2);
}
#[inline]
unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        MEM_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}
#[inline]
unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let p = memPtr as *mut BYTE;
        *p = val as BYTE;
        *p.add(1) = (val >> 8) as BYTE;
    }
}
#[inline]
unsafe fn MEM_readLE24(memPtr: *const c_void) -> U32 {
    (MEM_readLE16(memPtr) as U32).wrapping_add(((*(memPtr as *const BYTE).add(2)) as U32) << 16)
}
#[inline]
unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        MEM_read32(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p as U32)
            .wrapping_add((*p.add(1) as U32) << 8)
            .wrapping_add((*p.add(2) as U32) << 16)
            .wrapping_add((*p.add(3) as U32) << 24)
    }
}
#[inline]
unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        MEM_read64(memPtr)
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
unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    if MEM_32bits() != 0 {
        MEM_readLE32(memPtr) as usize
    } else {
        MEM_readLE64(memPtr) as usize
    }
}

// ----------------------------------------------------------------------------
// Common constants
// ----------------------------------------------------------------------------
const ZSTD_MAGICNUMBER: U32 = 0xFD2FB524;
const BLOCKSIZE: usize = 128 * 1024;
const ZSTD_blockHeaderSize: usize = 3;
const ZSTD_frameHeaderSize_min: usize = 5;
const ZSTD_frameHeaderSize_max: usize = 5;

const BIT7: u32 = 128;
const BIT6: u32 = 64;
const BIT5: u32 = 32;
const BIT4: u32 = 16;
const BIT1: u32 = 2;
const BIT0: u32 = 1;

const IS_RAW: u32 = BIT0;
const IS_RLE: u32 = BIT1;

const MINMATCH: usize = 4;
const REPCODE_STARTVALUE: usize = 4;

const MLbits: u32 = 7;
const LLbits: u32 = 6;
const Offbits: u32 = 5;
const MaxML: u32 = (1 << MLbits) - 1;
const MaxLL: u32 = (1 << LLbits) - 1;
const MaxOff: u32 = (1 << Offbits) - 1;
const MLFSELog: u32 = 10;
const LLFSELog: u32 = 10;
const OffFSELog: u32 = 9;

const MIN_SEQUENCES_SIZE: usize = 2 + 2 + 3 + 1;
const MIN_CBLOCK_SIZE: usize = 3 + MIN_SEQUENCES_SIZE;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

const ZSTD_WINDOWLOG_ABSOLUTEMIN: u32 = 11;

// blockType_t
type blockType_t = u32;
const bt_compressed: u32 = 0;
const bt_raw: u32 = 1;
const bt_rle: u32 = 2;
const bt_end: u32 = 3;

#[inline]
unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

/// ZSTD_wildcopy : can copy up to 7-8 bytes too many
#[inline]
unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.wrapping_offset(length);
    loop {
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if !(op < oend) {
            break;
        }
    }
}

// ----------------------------------------------------------------------------
// bitstream (read backward)
// ----------------------------------------------------------------------------
#[repr(C)]
struct BIT_DStream_t {
    bitContainer: usize,
    bitsConsumed: u32,
    ptr: *const c_char,
    start: *const c_char,
}

const BIT_DStream_unfinished: u32 = 0;
const BIT_DStream_endOfBuffer: u32 = 1;
const BIT_DStream_completed: u32 = 2;
const BIT_DStream_overflow: u32 = 3;

#[inline]
fn BIT_highbit32(val: U32) -> u32 {
    val.leading_zeros() ^ 31
}

unsafe fn BIT_initDStream(bitD: *mut BIT_DStream_t, srcBuffer: *const c_void, srcSize: usize) -> usize {
    if srcSize < 1 {
        core::ptr::write_bytes(bitD as *mut u8, 0, core::mem::size_of::<BIT_DStream_t>());
        return error(code::SRCSIZE_WRONG);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        // normal case
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char).add(srcSize - core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        let contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as u32;
        if contain32 == 0 {
            return error(code::GENERIC);
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let sp = (*bitD).start as *const BYTE;
        let wbits = core::mem::size_of::<usize>() * 8;
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sp.add(6) as usize) << (wbits - 16));
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sp.add(5) as usize) << (wbits - 24));
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sp.add(4) as usize) << (wbits - 32));
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*sp.add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*sp.add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*sp.add(1) as usize) << 8);
        }
        let contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as u32;
        if contain32 == 0 {
            return error(code::GENERIC);
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
        (*bitD).bitsConsumed = (*bitD)
            .bitsConsumed
            .wrapping_add(((core::mem::size_of::<usize>() - srcSize) * 8) as u32);
    }

    srcSize
}

#[inline]
unsafe fn BIT_lookBits(bitD: *const BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask.wrapping_sub(nbBits)) & bitMask)
}

#[inline]
unsafe fn BIT_lookBitsFast(bitD: *const BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1).wrapping_sub(nbBits)) & bitMask)
}

#[inline]
unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

#[inline]
unsafe fn BIT_readBits(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

#[inline]
unsafe fn BIT_readBitsFast(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

unsafe fn BIT_reloadDStream(bitD: *mut BIT_DStream_t) -> u32 {
    let wbits = (core::mem::size_of::<usize>() * 8) as u32;
    if (*bitD).bitsConsumed > wbits {
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < wbits {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes: u32 = (*bitD).bitsConsumed >> 3;
        let mut result: u32 = BIT_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize - (*bitD).start as usize) as u32;
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes * 8);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline]
unsafe fn BIT_endOfDStream(DStream: *const BIT_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as u32)) as u32
}

// ----------------------------------------------------------------------------
// FSE : Finite State Entropy coder
// ----------------------------------------------------------------------------
type FSE_DTable = u32;

const FSE_MAX_MEMORY_USAGE: u32 = 14;
const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;

const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
const FSE_MAX_TABLESIZE: u32 = 1 << FSE_MAX_TABLELOG;
const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

// FSE_DTABLE_SIZE_U32(maxTableLog) = 1 + (1<<maxTableLog)
const fn FSE_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

// DTable_max_t = U32[FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)]
const DTABLE_MAX_LEN: usize = 1 + (1 << FSE_MAX_TABLELOG) as usize;

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DTableHeader {
    tableLog: U16,
    fastMode: U16,
} // sizeof U32

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_decode_t {
    newState: u16,
    symbol: u8,
    nbBits: u8,
} // size == U32

#[repr(C)]
struct FSE_DState_t {
    state: usize,
    table: *const c_void,
}

#[inline]
unsafe fn FSE_initDState(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t, dt: *const FSE_DTable) {
    let mut DTableH = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    core::ptr::copy_nonoverlapping(
        dt as *const u8,
        &mut DTableH as *mut FSE_DTableHeader as *mut u8,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    (*DStatePtr).state = BIT_readBits(bitD, DTableH.tableLog as U32);
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline]
unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BIT_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

#[inline]
unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BIT_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

#[inline]
unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

#[inline]
fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    let mut DTableH = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSE_decode_t;
    let tableSize: U32 = 1 << tableLog;
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
        return error(code::MAXSYMBOLVALUE_TOOLARGE);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return error(code::TABLELOG_TOOLARGE);
    }

    memset(
        tableDecode as *mut c_void,
        0,
        core::mem::size_of::<FSE_decode_t>() * (maxSymbolValue + 1) as usize,
    );
    DTableH.tableLog = tableLog as U16;
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

    // Spread symbols
    s = 0;
    while s <= maxSymbolValue {
        let mut i: i32 = 0;
        let nc = *normalizedCounter.add(s as usize) as i32;
        while i < nc {
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
        return error(code::GENERIC);
    }

    // Build Decoding table
    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.add(i as usize)).nbBits =
                (tableLog - BIT_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(i as usize)).newState =
                (((nextState as U32) << (*tableDecode.add(i as usize)).nbBits as U32)
                    .wrapping_sub(tableSize)) as U16;
            i += 1;
        }
    }

    DTableH.fastMode = noLarge as U16;
    core::ptr::copy_nonoverlapping(
        &DTableH as *const FSE_DTableHeader as *const u8,
        dt as *mut u8,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    0
}

unsafe fn FSE_isError(code_in: usize) -> u32 {
    err_is_error(code_in)
}

#[inline]
fn FSE_abs(a: S16) -> S16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

unsafe fn FSE_readNCount(
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
        return error(code::SRCSIZE_WRONG);
    }
    bitStream = MEM_readLE32(ip as *const c_void);
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

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0 = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 += 24;
                if ip < iend.wrapping_sub(5) {
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
                return error(code::MAXSYMBOLVALUE_TOOSMALL);
            }
            while charnum < n0 {
                *normalizedCounter.add(charnum as usize) = 0;
                charnum += 1;
            }
            if (ip <= iend.wrapping_sub(7))
                || (ip.wrapping_add((bitCount >> 3) as usize) <= iend.wrapping_sub(4))
            {
                ip = ip.add((bitCount >> 3) as usize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
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
                if (ip <= iend.wrapping_sub(7))
                    || (ip.wrapping_add((bitCount >> 3) as usize) <= iend.wrapping_sub(4))
                {
                    ip = ip.add((bitCount >> 3) as usize);
                    bitCount &= 7;
                } else {
                    bitCount -= (8 * (iend as isize - 4 - ip as isize)) as i32;
                    ip = iend.wrapping_sub(4);
                }
                bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return error(code::GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.add(((bitCount + 7) >> 3) as usize);
    if (ip as usize - istart as usize) > hbSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip as usize - istart as usize
}

unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let cell = dPtr as *mut FSE_decode_t;

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
    let dPtr = dt.add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSE_decode_t;
    let tableSize: u32 = 1 << nbBits;
    let tableMask: u32 = tableSize - 1;
    let maxSymbolValue: u32 = tableMask;
    let mut s: u32;

    if nbBits < 1 {
        return error(code::GENERIC);
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
    let omax = op.add(maxDstSize);
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

    // 4 symbols per loop
    while (BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit) {
        *op.add(0) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(1) = FSE_GETSYMBOL!(&mut state2);

        if FSE_MAX_TABLELOG * 4 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(3) = FSE_GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    // tail
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

    if BIT_endOfDStream(&bitD) != 0 && FSE_endOfDState(&state1) != 0 && FSE_endOfDState(&state2) != 0
    {
        return op as usize - ostart as usize;
    }

    if op == omax {
        return error(code::DSTSIZE_TOOSMALL);
    }

    error(code::CORRUPTION_DETECTED)
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
    core::ptr::copy_nonoverlapping(
        dt as *const u8,
        &mut DTableH as *mut FSE_DTableHeader as *mut u8,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    let fastMode = DTableH.fastMode as U32;

    if fastMode != 0 {
        return FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

unsafe fn FSE_decompress(dst: *mut c_void, maxDstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [S16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [FSE_DTable; DTABLE_MAX_LEN] = [0; DTABLE_MAX_LEN];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE;
    let mut errorCode: usize;
    let mut cSrcSize = cSrcSize;

    if cSrcSize < 2 {
        return error(code::SRCSIZE_WRONG);
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
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSE_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

// ----------------------------------------------------------------------------
// Huff0 : Huffman coder
// ----------------------------------------------------------------------------
const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16;
const HUF_MAX_TABLELOG: u32 = 12;
const HUF_DEFAULT_TABLELOG: u32 = HUF_MAX_TABLELOG;
const HUF_MAX_SYMBOL_VALUE: u32 = 255;

// HUF_DTABLE_SIZE(maxTableLog) = 1 + (1<<maxTableLog)
const HUF_DTABLE_LEN_MAX: usize = 1 + (1 << HUF_MAX_TABLELOG) as usize;

unsafe fn HUF_isError(code_in: usize) -> u32 {
    err_is_error(code_in)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX2 {
    byte: BYTE,
    nbBits: BYTE,
} // single-symbol decoding, sizeof == U16

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX4 {
    sequence: U16,
    nbBits: BYTE,
    length: BYTE,
} // double-symbols decoding, sizeof == U32

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
        return error(code::SRCSIZE_WRONG);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        // special header
        if iSize >= 242 {
            // RLE
            static L: [i32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = L[iSize - 242] as usize;
            memset(huffWeight as *mut c_void, 1, hwSize);
            iSize = 0;
        } else {
            // Incompressible
            oSize = iSize - 127;
            iSize = (oSize + 1) / 2;
            if iSize + 1 > srcSize {
                return error(code::SRCSIZE_WRONG);
            }
            if oSize >= hwSize {
                return error(code::CORRUPTION_DETECTED);
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
        // header compressed with FSE (normal case)
        if iSize + 1 > srcSize {
            return error(code::SRCSIZE_WRONG);
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

    // collect weight stats
    memset(
        rankStats as *mut c_void,
        0,
        (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        let w = *huffWeight.add(n as usize) as U32;
        if w >= HUF_ABSOLUTEMAX_TABLELOG {
            return error(code::CORRUPTION_DETECTED);
        }
        *rankStats.add(w as usize) += 1;
        weightTotal = weightTotal.wrapping_add((1u32 << w) >> 1);
        n += 1;
    }
    if weightTotal == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    // get last non-null symbol weight
    tableLog = BIT_highbit32(weightTotal) + 1;
    if tableLog > HUF_ABSOLUTEMAX_TABLELOG {
        return error(code::CORRUPTION_DETECTED);
    }
    {
        let total: U32 = 1 << tableLog;
        let rest: U32 = total - weightTotal;
        let verif: U32 = 1 << BIT_highbit32(rest);
        let lastWeight: U32 = BIT_highbit32(rest) + 1;
        if verif != rest {
            return error(code::CORRUPTION_DETECTED);
        }
        *huffWeight.add(oSize) = lastWeight as BYTE;
        *rankStats.add(lastWeight as usize) += 1;
    }

    // check tree construction validity
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    *nbSymbolsPtr = (oSize + 1) as U32;
    *tableLogPtr = tableLog;
    iSize + 1
}

// --- single-symbol decoding ---

unsafe fn HUF_readDTableX2(DTable: *mut U16, src: *const c_void, srcSize: usize) -> usize {
    let mut huffWeight: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUF_DEltX2;

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

    if tableLog > *DTable.add(0) as U32 {
        return error(code::TABLELOG_TOOLARGE);
    }
    *DTable.add(0) = tableLog as U16;

    // Prepare ranks
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current = nextRankStart;
        nextRankStart += rankVal[n as usize] << (n - 1);
        rankVal[n as usize] = current;
        n += 1;
    }

    // fill DTable
    n = 0;
    while n < nbSymbols {
        let w = huffWeight[n as usize] as U32;
        let length = (1u32 << w) >> 1;
        let mut i: U32;
        let mut D = HUF_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = (tableLog + 1 - w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize] + length {
            *dt.add(i as usize) = D;
            i += 1;
        }
        rankVal[w as usize] += length;
        n += 1;
    }

    iSize
}

#[inline]
unsafe fn HUF_decodeSymbolX2(Dstream: *mut BIT_DStream_t, dt: *const HUF_DEltX2, dtLog: U32) -> BYTE {
    let val = BIT_lookBitsFast(Dstream, dtLog);
    let c = (*dt.add(val)).byte;
    BIT_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
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

    // up to 4 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(4)) {
        // SYMBOLX2_2 (MEM_64bits)
        if MEM_64bits() != 0 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.add(1);
        }
        // SYMBOLX2_1 (MEM_64bits || HUF_MAX_TABLELOG<=12)
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.add(1);
        }
        // SYMBOLX2_2
        if MEM_64bits() != 0 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.add(1);
        }
        // SYMBOLX2_0
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    // closer to the end
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd) {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    // no more data to retrieve from bitstream
    while p < pEnd {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    pEnd as usize - pStart as usize
}

unsafe fn HUF_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    if cSrcSize < 10 {
        return error(code::CORRUPTION_DETECTED);
    }

    let istart = cSrc as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dstSize);
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUF_DEltX2).add(1);
    let dtLog = *DTable.add(0) as U32;
    let mut errorCode: usize;

    let mut bitD1 = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut bitD2 = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut bitD3 = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut bitD4 = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
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
    let mut endSignal: U32;

    length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
    if length4 > cSrcSize {
        return error(code::CORRUPTION_DETECTED);
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

    endSignal = BIT_reloadDStream(&mut bitD1)
        | BIT_reloadDStream(&mut bitD2)
        | BIT_reloadDStream(&mut bitD3)
        | BIT_reloadDStream(&mut bitD4);
    while (endSignal == BIT_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
        macro_rules! X2_2 {
            ($op:expr, $bd:expr) => {
                if MEM_64bits() != 0 {
                    *$op = HUF_decodeSymbolX2($bd, dt, dtLog);
                    $op = $op.add(1);
                }
            };
        }
        macro_rules! X2_1 {
            ($op:expr, $bd:expr) => {
                if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                    *$op = HUF_decodeSymbolX2($bd, dt, dtLog);
                    $op = $op.add(1);
                }
            };
        }
        macro_rules! X2_0 {
            ($op:expr, $bd:expr) => {{
                *$op = HUF_decodeSymbolX2($bd, dt, dtLog);
                $op = $op.add(1);
            }};
        }
        X2_2!(op1, &mut bitD1);
        X2_2!(op2, &mut bitD2);
        X2_2!(op3, &mut bitD3);
        X2_2!(op4, &mut bitD4);
        X2_1!(op1, &mut bitD1);
        X2_1!(op2, &mut bitD2);
        X2_1!(op3, &mut bitD3);
        X2_1!(op4, &mut bitD4);
        X2_2!(op1, &mut bitD1);
        X2_2!(op2, &mut bitD2);
        X2_2!(op3, &mut bitD3);
        X2_2!(op4, &mut bitD4);
        X2_0!(op1, &mut bitD1);
        X2_0!(op2, &mut bitD2);
        X2_0!(op3, &mut bitD3);
        X2_0!(op4, &mut bitD4);

        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
    }

    // check corruption
    if op1 > opStart2 {
        return error(code::CORRUPTION_DETECTED);
    }
    if op2 > opStart3 {
        return error(code::CORRUPTION_DETECTED);
    }
    if op3 > opStart4 {
        return error(code::CORRUPTION_DETECTED);
    }

    // finish bitStreams one by one
    HUF_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
    HUF_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
    HUF_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
    HUF_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

    // check
    endSignal = BIT_endOfDStream(&bitD1)
        & BIT_endOfDStream(&bitD2)
        & BIT_endOfDStream(&bitD3)
        & BIT_endOfDStream(&bitD4);
    if endSignal == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    dstSize
}

unsafe fn HUF_decompress4X2(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    let mut DTable: [U16; HUF_DTABLE_LEN_MAX] = [0; HUF_DTABLE_LEN_MAX];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;
    let errorCode: usize;

    errorCode = HUF_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUF_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// --- double-symbols decoding ---

const RANKVAL_ROW: usize = (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize;
const RANKVAL_ROWS: usize = HUF_ABSOLUTEMAX_TABLELOG as usize;
// rankVal_t = U32[HUF_ABSOLUTEMAX_TABLELOG][HUF_ABSOLUTEMAX_TABLELOG + 1]

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
    let mut rankVal: [U32; RANKVAL_ROW] = [0; RANKVAL_ROW];
    let mut s: U32;

    // get pre-calculated rankVal
    core::ptr::copy_nonoverlapping(rankValOrigin, rankVal.as_mut_ptr(), RANKVAL_ROW);

    // fill skipped values
    if minWeight > 1 {
        let mut i: U32;
        let skipSize = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1;
        i = 0;
        while i < skipSize {
            *DTable.add(i as usize) = DElt;
            i += 1;
        }
    }

    // fill DTable
    s = 0;
    while s < sortedListSize {
        let symbol = (*sortedSymbols.add(s as usize)).symbol as U32;
        let weight = (*sortedSymbols.add(s as usize)).weight as U32;
        let nbBits = nbBitsBaseline - weight;
        let length = 1u32 << (sizeLog - nbBits);
        let start = rankVal[weight as usize];
        let mut i = start;
        let end = start + length;

        MEM_writeLE16(
            &mut DElt.sequence as *mut U16 as *mut c_void,
            (baseSeq as U32 + (symbol << 8)) as U16,
        );
        DElt.nbBits = (nbBits + consumed) as BYTE;
        DElt.length = 2;
        loop {
            *DTable.add(i as usize) = DElt;
            i += 1;
            if !(i < end) {
                break;
            }
        }

        rankVal[weight as usize] += length;
        s += 1;
    }
}

unsafe fn HUF_fillDTableX4(
    DTable: *mut HUF_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const [U32; RANKVAL_ROW],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; RANKVAL_ROW] = [0; RANKVAL_ROW];
    let scaleLog: i32 = nbBitsBaseline as i32 - targetLog as i32;
    let minBits: U32 = nbBitsBaseline - maxWeight;
    let mut s: U32;

    core::ptr::copy_nonoverlapping(
        rankValOrigin as *const U32,
        rankVal.as_mut_ptr(),
        RANKVAL_ROW,
    );

    s = 0;
    while s < sortedListSize {
        let symbol = (*sortedList.add(s as usize)).symbol as U16;
        let weight = (*sortedList.add(s as usize)).weight as U32;
        let nbBits = nbBitsBaseline - weight;
        let start = rankVal[weight as usize];
        let length = 1u32 << (targetLog - nbBits);

        if targetLog - nbBits >= minBits {
            // enough room for a second symbol
            let sortedRank: U32;
            let mut minWeight = nbBits as i32 + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUF_fillDTableX4Level2(
                DTable.add(start as usize),
                targetLog - nbBits,
                nbBits,
                (*rankValOrigin.add(nbBits as usize)).as_ptr(),
                minWeight,
                sortedList.add(sortedRank as usize),
                sortedListSize - sortedRank,
                nbBitsBaseline,
                symbol,
            );
        } else {
            let mut i: U32;
            let end = start + length;
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
                *DTable.add(i as usize) = DElt;
                i += 1;
            }
        }
        rankVal[weight as usize] += length;
        s += 1;
    }
}

unsafe fn HUF_readDTableX4(DTable: *mut U32, src: *const c_void, srcSize: usize) -> usize {
    let mut weightList: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize];
    // rankStart = rankStart0 + 1
    let mut rankVal: [[U32; RANKVAL_ROW]; RANKVAL_ROWS] = [[0; RANKVAL_ROW]; RANKVAL_ROWS];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog = *DTable.add(0);
    let iSize: usize;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUF_DEltX4).add(1);

    if memLog > HUF_ABSOLUTEMAX_TABLELOG {
        return error(code::TABLELOG_TOOLARGE);
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

    if tableLog > memLog {
        return error(code::TABLELOG_TOOLARGE);
    }

    // find maxWeight
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        if maxW == 0 {
            return error(code::GENERIC);
        }
        maxW -= 1;
    }

    // Get start index of each weight (rankStart = rankStart0+1)
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w <= maxW {
            let current = nextRankStart;
            nextRankStart += rankStats[w as usize];
            rankStart0[(w + 1) as usize] = current; // rankStart[w]
            w += 1;
        }
        rankStart0[1] = nextRankStart; // rankStart[0]
        sizeOfSort = nextRankStart;
    }

    // sort symbols by weight
    {
        let mut s: U32;
        s = 0;
        while s < nbSymbols {
            let w = weightList[s as usize] as U32;
            let r = rankStart0[(w + 1) as usize]; // rankStart[w]++
            rankStart0[(w + 1) as usize] = r + 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        rankStart0[1] = 0; // rankStart[0] = 0
    }

    // Build rankVal
    {
        let minBits: U32 = tableLog + 1 - maxW;
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: i32 = (memLog as i32 - tableLog as i32) - 1;
        // rankVal0 = rankVal[0]
        w = 1;
        while w <= maxW {
            let current = nextRankVal;
            nextRankVal += rankStats[w as usize] << ((w as i32 + rescale) as u32);
            rankVal[0][w as usize] = current;
            w += 1;
        }
        consumed = minBits;
        while consumed <= memLog - minBits {
            // rankValPtr = rankVal[consumed]
            w = 1;
            while w <= maxW {
                let v = rankVal[0][w as usize] >> consumed;
                rankVal[consumed as usize][w as usize] = v;
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
        rankStart0.as_ptr(), // C passes rankStart0 base (not the +1 offset local)
        rankVal.as_ptr(),
        maxW,
        tableLog + 1,
    );

    iSize
}

#[inline]
unsafe fn HUF_decodeSymbolX4(op: *mut c_void, DStream: *mut BIT_DStream_t, dt: *const HUF_DEltX4, dtLog: U32) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 2);
    BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

#[inline]
unsafe fn HUF_decodeLastSymbolX4(op: *mut c_void, DStream: *mut BIT_DStream_t, dt: *const HUF_DEltX4, dtLog: U32) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        let wbits = (core::mem::size_of::<usize>() * 8) as u32;
        if (*DStream).bitsConsumed < wbits {
            BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > wbits {
                (*DStream).bitsConsumed = wbits;
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

    // up to 8 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd.wrapping_sub(7)) {
        // X4_2
        if MEM_64bits() != 0 {
            p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        }
        // X4_1
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
            p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        }
        // X4_2
        if MEM_64bits() != 0 {
            p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        }
        // X4_0
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    // closer to the end
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(2)) {
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    while p <= pEnd.wrapping_sub(2) {
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
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
    DTable: *const U32,
) -> usize {
    if cSrcSize < 10 {
        return error(code::CORRUPTION_DETECTED);
    }

    let istart = cSrc as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dstSize);
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUF_DEltX4).add(1);
    let dtLog = *DTable.add(0);
    let mut errorCode: usize;

    let mut bitD1 = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut bitD2 = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut bitD3 = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut bitD4 = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
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
    let mut endSignal: U32;

    length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
    if length4 > cSrcSize {
        return error(code::CORRUPTION_DETECTED);
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

    endSignal = BIT_reloadDStream(&mut bitD1)
        | BIT_reloadDStream(&mut bitD2)
        | BIT_reloadDStream(&mut bitD3)
        | BIT_reloadDStream(&mut bitD4);
    while (endSignal == BIT_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
        macro_rules! X4_2 {
            ($op:expr, $bd:expr) => {
                if MEM_64bits() != 0 {
                    $op = $op.add(HUF_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize);
                }
            };
        }
        macro_rules! X4_1 {
            ($op:expr, $bd:expr) => {
                if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                    $op = $op.add(HUF_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize);
                }
            };
        }
        macro_rules! X4_0 {
            ($op:expr, $bd:expr) => {{
                $op = $op.add(HUF_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize);
            }};
        }
        X4_2!(op1, &mut bitD1);
        X4_2!(op2, &mut bitD2);
        X4_2!(op3, &mut bitD3);
        X4_2!(op4, &mut bitD4);
        X4_1!(op1, &mut bitD1);
        X4_1!(op2, &mut bitD2);
        X4_1!(op3, &mut bitD3);
        X4_1!(op4, &mut bitD4);
        X4_2!(op1, &mut bitD1);
        X4_2!(op2, &mut bitD2);
        X4_2!(op3, &mut bitD3);
        X4_2!(op4, &mut bitD4);
        X4_0!(op1, &mut bitD1);
        X4_0!(op2, &mut bitD2);
        X4_0!(op3, &mut bitD3);
        X4_0!(op4, &mut bitD4);

        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
    }

    if op1 > opStart2 {
        return error(code::CORRUPTION_DETECTED);
    }
    if op2 > opStart3 {
        return error(code::CORRUPTION_DETECTED);
    }
    if op3 > opStart4 {
        return error(code::CORRUPTION_DETECTED);
    }

    HUF_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
    HUF_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
    HUF_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
    HUF_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

    endSignal = BIT_endOfDStream(&bitD1)
        & BIT_endOfDStream(&bitD2)
        & BIT_endOfDStream(&bitD3)
        & BIT_endOfDStream(&bitD4);
    if endSignal == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    dstSize
}

unsafe fn HUF_decompress4X4(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    let mut DTable: [U32; HUF_DTABLE_LEN_MAX] = [0; HUF_DTABLE_LEN_MAX];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let hSize = HUF_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// --- Generic decompression selector ---

#[repr(C)]
#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
}

static algoTime: [[algo_time_t; 3]; 16] = {
    const fn a(t: U32, d: U32) -> algo_time_t {
        algo_time_t {
            tableTime: t,
            decode256Time: d,
        }
    }
    [
        [a(0, 0), a(1, 1), a(2, 2)],
        [a(0, 0), a(1, 1), a(2, 2)],
        [a(38, 130), a(1313, 74), a(2151, 38)],
        [a(448, 128), a(1353, 74), a(2238, 41)],
        [a(556, 128), a(1353, 74), a(2238, 47)],
        [a(714, 128), a(1418, 74), a(2436, 53)],
        [a(883, 128), a(1437, 74), a(2464, 61)],
        [a(897, 128), a(1515, 75), a(2622, 68)],
        [a(926, 128), a(1613, 75), a(2730, 75)],
        [a(947, 128), a(1729, 77), a(3359, 77)],
        [a(1107, 128), a(2083, 81), a(4006, 84)],
        [a(1177, 128), a(2379, 87), a(4785, 88)],
        [a(1242, 128), a(2415, 93), a(5155, 84)],
        [a(1349, 128), a(2644, 106), a(5260, 106)],
        [a(1455, 128), a(2422, 124), a(4174, 124)],
        [a(722, 128), a(1891, 145), a(1936, 146)],
    ]
};

unsafe fn HUF_decompress(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    type DecompFn = unsafe fn(*mut c_void, usize, *const c_void, usize) -> usize;
    let decompress: [Option<DecompFn>; 3] =
        [Some(HUF_decompress4X2), Some(HUF_decompress4X4), None];

    let Q: U32;
    let D256: U32 = (dstSize >> 8) as U32;
    let mut Dtime: [U32; 3] = [0; 3];
    let mut algoNb: U32 = 0;
    let mut n: i32;

    if dstSize == 0 {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if cSrcSize > dstSize {
        return error(code::CORRUPTION_DETECTED);
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as i32, dstSize);
        return dstSize;
    }

    Q = (cSrcSize * 16 / dstSize) as U32;
    n = 0;
    while n < 3 {
        Dtime[n as usize] = algoTime[Q as usize][n as usize]
            .tableTime
            .wrapping_add(algoTime[Q as usize][n as usize].decode256Time.wrapping_mul(D256));
        n += 1;
    }

    Dtime[1] = Dtime[1].wrapping_add(Dtime[1] >> 4);
    Dtime[2] = Dtime[2].wrapping_add(Dtime[2] >> 3);

    if Dtime[1] < Dtime[0] {
        algoNb = 1;
    }

    (decompress[algoNb as usize].unwrap())(dst, dstSize, cSrc, cSrcSize)
}

// ----------------------------------------------------------------------------
// ZSTD decompression module for v0.4 legacy format
// ----------------------------------------------------------------------------

// ZSTD_strategy
type ZSTD_strategy = u32;
const ZSTD_fast: u32 = 0;
const ZSTD_greedy: u32 = 1;
const ZSTD_lazy: u32 = 2;
const ZSTD_lazy2: u32 = 3;
const ZSTD_btlazy2: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_parameters {
    srcSize: U64,
    windowLog: U32,
    contentLog: U32,
    hashLog: U32,
    searchLog: U32,
    searchLength: U32,
    strategy: ZSTD_strategy,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

#[inline]
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

unsafe fn ZSTD_isError(code_in: usize) -> u32 {
    err_is_error(code_in)
}

// ZSTD_dStage
type ZSTD_dStage = u32;
const ZSTDds_getFrameHeaderSize: u32 = 0;
const ZSTDds_decodeFrameHeader: u32 = 1;
const ZSTDds_decodeBlockHeader: u32 = 2;
const ZSTDds_decompressBlock: u32 = 3;

#[repr(C)]
pub struct ZSTDv04_Dctx_s {
    LLTable: [U32; FSE_DTABLE_SIZE_U32(LLFSELog)],
    OffTable: [U32; FSE_DTABLE_SIZE_U32(OffFSELog)],
    MLTable: [U32; FSE_DTABLE_SIZE_U32(MLFSELog)],
    previousDstEnd: *const c_void,
    base: *const c_void,
    vBase: *const c_void,
    dictEnd: *const c_void,
    expected: usize,
    headerSize: usize,
    params: ZSTD_parameters,
    bType: blockType_t,
    stage: ZSTD_dStage,
    litPtr: *const BYTE,
    litSize: usize,
    litBuffer: [BYTE; BLOCKSIZE + 8],
    headerBuffer: [BYTE; ZSTD_frameHeaderSize_max],
}

type ZSTD_DCtx = ZSTDv04_Dctx_s;

unsafe fn ZSTD_resetDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected = ZSTD_frameHeaderSize_min;
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
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

unsafe fn ZSTD_decodeFrameHeader_Part1(zc: *mut ZSTD_DCtx, src: *const c_void, srcSize: usize) -> usize {
    let magicNumber: U32;
    if srcSize != ZSTD_frameHeaderSize_min {
        return error(code::SRCSIZE_WRONG);
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_MAGICNUMBER {
        return error(code::PREFIX_UNKNOWN);
    }
    (*zc).headerSize = ZSTD_frameHeaderSize_min;
    (*zc).headerSize
}

unsafe fn ZSTD_getFrameParams(params: *mut ZSTD_parameters, src: *const c_void, srcSize: usize) -> usize {
    let magicNumber: U32;
    if srcSize < ZSTD_frameHeaderSize_min {
        return ZSTD_frameHeaderSize_max;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_MAGICNUMBER {
        return error(code::PREFIX_UNKNOWN);
    }
    memset(params as *mut c_void, 0, core::mem::size_of::<ZSTD_parameters>());
    (*params).windowLog =
        ((*(src as *const BYTE).add(4) & 15) as U32) + ZSTD_WINDOWLOG_ABSOLUTEMIN;
    if (*(src as *const BYTE).add(4) >> 4) != 0 {
        return error(code::FRAMEPARAMETER_UNSUPPORTED);
    }
    0
}

unsafe fn ZSTD_decodeFrameHeader_Part2(zc: *mut ZSTD_DCtx, src: *const c_void, srcSize: usize) -> usize {
    let result: usize;
    if srcSize != (*zc).headerSize {
        return error(code::SRCSIZE_WRONG);
    }
    result = ZSTD_getFrameParams(&mut (*zc).params, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).params.windowLog > 25) {
        return error(code::FRAMEPARAMETER_UNSUPPORTED);
    }
    result
}

unsafe fn ZSTD_getcBlockSize(src: *const c_void, srcSize: usize, bpPtr: *mut blockProperties_t) -> usize {
    let in_ = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return error(code::SRCSIZE_WRONG);
    }

    headerFlags = *in_;
    cSize = (*in_.add(2) as U32) + ((*in_.add(1) as U32) << 8) + (((*in_.add(0) & 7) as U32) << 16);

    (*bpPtr).blockType = (headerFlags >> 6) as blockType_t;
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle {
        cSize
    } else {
        0
    };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as usize
}

unsafe fn ZSTD_copyRawBlock(dst: *mut c_void, maxDstSize: usize, src: *const c_void, srcSize: usize) -> usize {
    if srcSize > maxDstSize {
        return error(code::DSTSIZE_TOOSMALL);
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
    let ip = src as *const BYTE;

    let litSize = ((MEM_readLE32(src) & 0x1FFFFF) >> 2) as usize;
    let litCSize = ((MEM_readLE32(ip.add(2) as *const c_void) & 0xFFFFFF) >> 5) as usize;

    if litSize > *maxDstSizePtr {
        return error(code::CORRUPTION_DETECTED);
    }
    if litCSize + 5 > srcSize {
        return error(code::CORRUPTION_DETECTED);
    }

    if HUF_isError(HUF_decompress(dst, litSize, ip.add(5) as *const c_void, litCSize)) != 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    *maxDstSizePtr = litSize;
    litCSize + 5
}

unsafe fn ZSTD_decodeLiteralsBlock(dctx: *mut ZSTD_DCtx, src: *const c_void, srcSize: usize) -> usize {
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return error(code::CORRUPTION_DETECTED);
    }

    match (*istart & 3) as u32 {
        // compressed
        0 => {
            let mut litSize: usize = BLOCKSIZE;
            let readSize = ZSTD_decompressLiterals(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                &mut litSize,
                src,
                srcSize,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                8,
            );
            readSize
        }
        x if x == IS_RAW => {
            let litSize = ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as usize;
            if litSize > srcSize - 11 {
                if litSize > BLOCKSIZE {
                    return error(code::CORRUPTION_DETECTED);
                }
                if litSize > srcSize - 3 {
                    return error(code::CORRUPTION_DETECTED);
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
            let litSize = ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as usize;
            if litSize > BLOCKSIZE {
                return error(code::CORRUPTION_DETECTED);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.add(3) as i32,
                litSize + 8,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            4
        }
        _ => error(code::CORRUPTION_DETECTED),
    }
}

unsafe fn ZSTD_decodeSeqHeaders(
    nbSeq: *mut c_int,
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
        return error(code::SRCSIZE_WRONG);
    }

    *nbSeq = MEM_readLE16(ip as *const c_void) as c_int;
    ip = ip.add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = (*ip.add(2) as usize) + ((*ip.add(1) as usize) << 8);
        ip = ip.add(3);
    } else {
        dumpsLength = (*ip.add(1) as usize) + (((*ip.add(0) & 1) as usize) << 8);
        ip = ip.add(2);
    }
    *dumpsPtr = ip;
    ip = ip.add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    if ip > iend.wrapping_sub(3) {
        return error(code::SRCSIZE_WRONG);
    }

    // sequences
    {
        let mut norm: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut headerSize: usize;

        // LL
        match LLtype {
            x if x == bt_rle => {
                LLlog = 0;
                let v = *ip;
                ip = ip.add(1);
                FSE_buildDTable_rle(DTableLL, v);
            }
            x if x == bt_raw => {
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
                    iend as usize - ip as usize,
                );
                if FSE_isError(headerSize) != 0 {
                    return error(code::GENERIC);
                }
                if LLlog > LLFSELog {
                    return error(code::CORRUPTION_DETECTED);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        // Off
        match Offtype {
            x if x == bt_rle => {
                Offlog = 0;
                if ip > iend.wrapping_sub(2) {
                    return error(code::SRCSIZE_WRONG);
                }
                let v = *ip & (MaxOff as BYTE);
                ip = ip.add(1);
                FSE_buildDTable_rle(DTableOffb, v);
            }
            x if x == bt_raw => {
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
                    iend as usize - ip as usize,
                );
                if FSE_isError(headerSize) != 0 {
                    return error(code::GENERIC);
                }
                if Offlog > OffFSELog {
                    return error(code::CORRUPTION_DETECTED);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
            }
        }

        // ML
        match MLtype {
            x if x == bt_rle => {
                MLlog = 0;
                if ip > iend.wrapping_sub(2) {
                    return error(code::SRCSIZE_WRONG);
                }
                let v = *ip;
                ip = ip.add(1);
                FSE_buildDTable_rle(DTableML, v);
            }
            x if x == bt_raw => {
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
                    iend as usize - ip as usize,
                );
                if FSE_isError(headerSize) != 0 {
                    return error(code::GENERIC);
                }
                if MLlog > MLFSELog {
                    return error(code::CORRUPTION_DETECTED);
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

    // Literal length
    litLength = FSE_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream) as usize;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
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
        } else if dumps.add(3) <= de {
            litLength = MEM_readLE24(dumps as *const c_void) as usize;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1);
        }
    }

    // Offset
    {
        static offsetPrefix: [U32; (MaxOff + 1) as usize] = [
            1, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
            131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, 1, 1,
            1, 1, 1,
        ];
        let offsetCode: U32;
        let mut nbBits: U32;
        offsetCode = FSE_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream) as U32;
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        nbBits = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0;
        }
        offset = (offsetPrefix[offsetCode as usize] as usize)
            .wrapping_add(BIT_readBits(&mut (*seqState).DStream, nbBits));
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset;
        }
        if (offsetCode != 0) || (litLength == 0) {
            (*seqState).prevOffset = (*seq).offset;
        }
    }

    // MatchLength
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
        } else if dumps.add(3) <= de {
            matchLength = MEM_readLE24(dumps as *const c_void) as usize;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1);
        }
    }
    matchLength += MINMATCH;

    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    static dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
    static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.add(sequenceLength);
    let oend_8 = oend.wrapping_sub(8);
    let litEnd = (*litPtr).add(sequence.litLength);
    let mut match_ = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    let seqLength = sequence.litLength + sequence.matchLength;

    if seqLength > (oend as usize - op as usize) {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if sequence.litLength > (litLimit as usize - *litPtr as usize) {
        return error(code::CORRUPTION_DETECTED);
    }
    if oLitEnd > oend_8 {
        return error(code::DSTSIZE_TOOSMALL);
    }

    if oMatchEnd > oend {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if litEnd > litLimit {
        return error(code::CORRUPTION_DETECTED);
    }

    // copy Literals
    ZSTD_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = litEnd;

    // copy Match
    if sequence.offset > (oLitEnd as usize - base as usize) {
        // offset beyond prefix
        if sequence.offset > (oLitEnd as usize - vBase as usize) {
            return error(code::CORRUPTION_DETECTED);
        }
        match_ = dictEnd.wrapping_sub(base as usize - match_ as usize);
        if match_.add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        // span extDict & currentPrefixSegment
        {
            let length1 = dictEnd as usize - match_ as usize;
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.add(length1);
            sequence.matchLength -= length1;
            match_ = base;
            if op > oend_8 || sequence.matchLength < MINMATCH {
                while op < oMatchEnd {
                    *op = *match_;
                    op = op.add(1);
                    match_ = match_.add(1);
                }
                return sequenceLength;
            }
        }
    }

    // match within prefix
    if sequence.offset < 8 {
        let sub2 = dec64table[sequence.offset];
        *op.add(0) = *match_.add(0);
        *op.add(1) = *match_.add(1);
        *op.add(2) = *match_.add(2);
        *op.add(3) = *match_.add(3);
        match_ = match_.add(dec32table[sequence.offset] as usize);
        ZSTD_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.wrapping_sub(sub2 as usize);
    } else {
        ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.add(8);
    match_ = match_.add(8);

    if oMatchEnd > oend.wrapping_sub(16 - MINMATCH) {
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
    sequenceLength
}

unsafe fn ZSTD_decompressSequences(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
) -> usize {
    let mut ip = seqStart as *const BYTE;
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut errorCode: usize;
    let mut dumpsLength: usize = 0;
    let mut litPtr = (*dctx).litPtr;
    let litEnd = litPtr.add((*dctx).litSize);
    let mut nbSeq: c_int = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;

    // Build Decoding Tables
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

    // Regen sequences
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
        sequence.offset = 4;
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.add(dumpsLength);
        seqState.prevOffset = 4;
        errorCode = BIT_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            iend as usize - ip as usize,
        );
        if err_is_error(errorCode) != 0 {
            return error(code::CORRUPTION_DETECTED);
        }
        FSE_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSE_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSE_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BIT_reloadDStream(&mut seqState.DStream) <= BIT_DStream_completed) && nbSeq != 0 {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(
                op, oend, sequence, &mut litPtr, litEnd, base, vBase, dictEnd,
            );
            if ZSTD_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
        }

        // check if reached exact end
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return error(code::CORRUPTION_DETECTED);
        }

        // last literal segment
        {
            let lastLLSize = litEnd as usize - litPtr as usize;
            if litPtr > litEnd {
                return error(code::CORRUPTION_DETECTED);
            }
            if op.add(lastLLSize) > oend {
                return error(code::DSTSIZE_TOOSMALL);
            }
            if lastLLSize > 0 {
                if op != (litPtr as *mut BYTE) {
                    memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.add(lastLLSize);
            }
        }
    }

    op as usize - ostart as usize
}

unsafe fn ZSTD_checkContinuity(dctx: *mut ZSTD_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char)
            .wrapping_sub((*dctx).previousDstEnd as usize - (*dctx).base as usize)
            as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

unsafe fn ZSTD_decompressBlock_internal(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let litCSize: usize;
    let mut srcSize = srcSize;

    if srcSize > BLOCKSIZE {
        return error(code::CORRUPTION_DETECTED);
    }

    litCSize = ZSTD_decodeLiteralsBlock(dctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.add(litCSize);
    srcSize -= litCSize;

    ZSTD_decompressSequences(dctx, dst, maxDstSize, ip as *const c_void, srcSize)
}

unsafe fn ZSTD_decompress_usingDict(
    ctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut remainingSize = srcSize;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    ZSTD_resetDCtx(ctx);
    if !dict.is_null() {
        ZSTD_decompress_insertDictionary(ctx, dict, dictSize);
        (*ctx).dictEnd = (*ctx).previousDstEnd;
        (*ctx).vBase = (dst as *const c_char)
            .wrapping_sub((*ctx).previousDstEnd as usize - (*ctx).base as usize)
            as *const c_void;
        (*ctx).base = dst;
    } else {
        (*ctx).vBase = dst;
        (*ctx).base = dst;
        (*ctx).dictEnd = dst;
    }

    // Frame Header
    {
        let mut frameHeaderSize: usize;
        if srcSize < ZSTD_frameHeaderSize_min + ZSTD_blockHeaderSize {
            return error(code::SRCSIZE_WRONG);
        }
        frameHeaderSize = ZSTD_decodeFrameHeader_Part1(ctx, src, ZSTD_frameHeaderSize_min);
        if ZSTD_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTD_blockHeaderSize {
            return error(code::SRCSIZE_WRONG);
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
        frameHeaderSize = ZSTD_decodeFrameHeader_Part2(ctx, src, frameHeaderSize);
        if ZSTD_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
    }

    // Loop on each block
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
            return error(code::SRCSIZE_WRONG);
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                decodedSize = ZSTD_decompressBlock_internal(
                    ctx,
                    op as *mut c_void,
                    oend as usize - op as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_raw => {
                decodedSize = ZSTD_copyRawBlock(
                    op as *mut c_void,
                    oend as usize - op as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_rle => {
                return error(code::GENERIC);
            }
            x if x == bt_end => {
                // end of frame
                if remainingSize != 0 {
                    return error(code::SRCSIZE_WRONG);
                }
            }
            _ => {
                return error(code::GENERIC);
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

unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut c_ulonglong, ret: usize) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    if srcSize < ZSTD_frameHeaderSize_min {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, error(code::SRCSIZE_WRONG));
        return;
    }
    if MEM_readLE32(src) != ZSTD_MAGICNUMBER {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, error(code::PREFIX_UNKNOWN));
        return;
    }
    ip = ip.add(ZSTD_frameHeaderSize_min);
    remainingSize -= ZSTD_frameHeaderSize_min;

    loop {
        let cBlockSize = ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, error(code::SRCSIZE_WRONG));
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

// Streaming Decompression API
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
    if srcSize != (*ctx).expected {
        return error(code::SRCSIZE_WRONG);
    }
    ZSTD_checkContinuity(ctx, dst);

    match (*ctx).stage {
        x if x == ZSTDds_getFrameHeaderSize => {
            if srcSize != ZSTD_frameHeaderSize_min {
                return error(code::SRCSIZE_WRONG);
            }
            (*ctx).headerSize = ZSTD_decodeFrameHeader_Part1(ctx, src, ZSTD_frameHeaderSize_min);
            if ZSTD_isError((*ctx).headerSize) != 0 {
                return (*ctx).headerSize;
            }
            memcpy(
                (*ctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTD_frameHeaderSize_min,
            );
            if (*ctx).headerSize > ZSTD_frameHeaderSize_min {
                return error(code::GENERIC);
            }
            (*ctx).expected = 0;
            // fallthrough
            let result = ZSTD_decodeFrameHeader_Part2(
                ctx,
                (*ctx).headerBuffer.as_ptr() as *const c_void,
                (*ctx).headerSize,
            );
            if ZSTD_isError(result) != 0 {
                return result;
            }
            (*ctx).expected = ZSTD_blockHeaderSize;
            (*ctx).stage = ZSTDds_decodeBlockHeader;
            0
        }
        x if x == ZSTDds_decodeFrameHeader => {
            let result = ZSTD_decodeFrameHeader_Part2(
                ctx,
                (*ctx).headerBuffer.as_ptr() as *const c_void,
                (*ctx).headerSize,
            );
            if ZSTD_isError(result) != 0 {
                return result;
            }
            (*ctx).expected = ZSTD_blockHeaderSize;
            (*ctx).stage = ZSTDds_decodeBlockHeader;
            0
        }
        x if x == ZSTDds_decodeBlockHeader => {
            let mut bp = blockProperties_t {
                blockType: 0,
                origSize: 0,
            };
            let blockSize = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
            if ZSTD_isError(blockSize) != 0 {
                return blockSize;
            }
            if bp.blockType == bt_end {
                (*ctx).expected = 0;
                (*ctx).stage = ZSTDds_getFrameHeaderSize;
            } else {
                (*ctx).expected = blockSize;
                (*ctx).bType = bp.blockType;
                (*ctx).stage = ZSTDds_decompressBlock;
            }
            0
        }
        x if x == ZSTDds_decompressBlock => {
            let rSize: usize;
            match (*ctx).bType {
                y if y == bt_compressed => {
                    rSize = ZSTD_decompressBlock_internal(ctx, dst, maxDstSize, src, srcSize);
                }
                y if y == bt_raw => {
                    rSize = ZSTD_copyRawBlock(dst, maxDstSize, src, srcSize);
                }
                y if y == bt_rle => {
                    return error(code::GENERIC);
                }
                y if y == bt_end => {
                    rSize = 0;
                }
                _ => {
                    return error(code::GENERIC);
                }
            }
            (*ctx).stage = ZSTDds_decodeBlockHeader;
            (*ctx).expected = ZSTD_blockHeaderSize;
            if ZSTD_isError(rSize) != 0 {
                return rSize;
            }
            (*ctx).previousDstEnd = (dst as *const c_char).add(rSize) as *const c_void;
            rSize
        }
        _ => error(code::GENERIC),
    }
}

unsafe fn ZSTD_decompress_insertDictionary(ctx: *mut ZSTD_DCtx, dict: *const c_void, dictSize: usize) {
    (*ctx).dictEnd = (*ctx).previousDstEnd;
    (*ctx).vBase = (dict as *const c_char)
        .wrapping_sub((*ctx).previousDstEnd as usize - (*ctx).base as usize)
        as *const c_void;
    (*ctx).base = dict;
    (*ctx).previousDstEnd = (dict as *const c_char).add(dictSize) as *const c_void;
}

// ----------------------------------------------------------------------------
// Buffered version (ZBUFF)
// ----------------------------------------------------------------------------
#[inline]
fn MIN(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

// ZBUFF_dStage
type ZBUFF_dStage = u32;
const ZBUFFds_init: u32 = 0;
const ZBUFFds_readHeader: u32 = 1;
const ZBUFFds_loadHeader: u32 = 2;
const ZBUFFds_decodeHeader: u32 = 3;
const ZBUFFds_read: u32 = 4;
const ZBUFFds_load: u32 = 5;
const ZBUFFds_flush: u32 = 6;

#[repr(C)]
pub struct ZBUFFv04_DCtx_s {
    zc: *mut ZSTD_DCtx,
    params: ZSTD_parameters,
    inBuff: *mut c_char,
    inBuffSize: usize,
    inPos: usize,
    outBuff: *mut c_char,
    outBuffSize: usize,
    outStart: usize,
    outEnd: usize,
    hPos: usize,
    dict: *const c_char,
    dictSize: usize,
    stage: ZBUFF_dStage,
    headerBuffer: [u8; ZSTD_frameHeaderSize_max],
}

type ZBUFF_DCtx = ZBUFFv04_DCtx_s;

unsafe fn ZBUFF_createDCtx() -> *mut ZBUFF_DCtx {
    let zbc = malloc(core::mem::size_of::<ZBUFF_DCtx>()) as *mut ZBUFF_DCtx;
    if zbc.is_null() {
        return core::ptr::null_mut();
    }
    memset(zbc as *mut c_void, 0, core::mem::size_of::<ZBUFF_DCtx>());
    (*zbc).zc = ZSTD_createDCtx();
    (*zbc).stage = ZBUFFds_init;
    zbc
}

unsafe fn ZBUFF_freeDCtx(zbc: *mut ZBUFF_DCtx) -> usize {
    if zbc.is_null() {
        return 0;
    }
    ZSTD_freeDCtx((*zbc).zc);
    free((*zbc).inBuff as *mut c_void);
    free((*zbc).outBuff as *mut c_void);
    free(zbc as *mut c_void);
    0
}

unsafe fn ZBUFF_decompressInit(zbc: *mut ZBUFF_DCtx) -> usize {
    (*zbc).stage = ZBUFFds_readHeader;
    (*zbc).hPos = 0;
    (*zbc).inPos = 0;
    (*zbc).outStart = 0;
    (*zbc).outEnd = 0;
    (*zbc).dictSize = 0;
    ZSTD_resetDCtx((*zbc).zc)
}

unsafe fn ZBUFF_decompressWithDictionary(zbc: *mut ZBUFF_DCtx, src: *const c_void, srcSize: usize) -> usize {
    (*zbc).dict = src as *const c_char;
    (*zbc).dictSize = srcSize;
    0
}

unsafe fn ZBUFF_limitCopy(dst: *mut c_void, maxDstSize: usize, src: *const c_void, srcSize: usize) -> usize {
    let length = MIN(maxDstSize, srcSize);
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

unsafe fn ZBUFF_decompressContinue(
    zbc: *mut ZBUFF_DCtx,
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart = src as *const c_char;
    let mut ip = istart;
    let iend = istart.add(*srcSizePtr);
    let ostart = dst as *mut c_char;
    let mut op = ostart;
    let oend = ostart.add(*maxDstSizePtr);
    let mut notDone: U32 = 1;

    while notDone != 0 {
        // Emulate C switch(stage) with fall-through. `break 'sw` == C `break`
        // (exit switch, re-run while). Falling to next `if` block == C fall-through.
        let mut st = (*zbc).stage;
        'sw: loop {
            if st == ZBUFFds_init {
                return error(code::INIT_MISSING);
            }

            if st == ZBUFFds_readHeader {
                let headerSize = ZSTD_getFrameParams(&mut (*zbc).params, src, *srcSizePtr);
                if ZSTD_isError(headerSize) != 0 {
                    return headerSize;
                }
                if headerSize != 0 {
                    memcpy(
                        (*zbc).headerBuffer.as_mut_ptr().add((*zbc).hPos) as *mut c_void,
                        src,
                        *srcSizePtr,
                    );
                    (*zbc).hPos += *srcSizePtr;
                    *maxDstSizePtr = 0;
                    (*zbc).stage = ZBUFFds_loadHeader;
                    return headerSize - (*zbc).hPos;
                }
                (*zbc).stage = ZBUFFds_decodeHeader;
                break 'sw;
            }

            if st == ZBUFFds_loadHeader {
                {
                    let mut headerSize = ZBUFF_limitCopy(
                        (*zbc).headerBuffer.as_mut_ptr().add((*zbc).hPos) as *mut c_void,
                        ZSTD_frameHeaderSize_max - (*zbc).hPos,
                        src,
                        *srcSizePtr,
                    );
                    (*zbc).hPos += headerSize;
                    ip = ip.add(headerSize);
                    headerSize = ZSTD_getFrameParams(
                        &mut (*zbc).params,
                        (*zbc).headerBuffer.as_ptr() as *const c_void,
                        (*zbc).hPos,
                    );
                    if ZSTD_isError(headerSize) != 0 {
                        return headerSize;
                    }
                    if headerSize != 0 {
                        *maxDstSizePtr = 0;
                        return headerSize - (*zbc).hPos;
                    }
                }
                st = ZBUFFds_decodeHeader; // intentional fall-through
            }

            if st == ZBUFFds_decodeHeader {
                {
                    let neededOutSize: usize = 1usize << (*zbc).params.windowLog;
                    let neededInSize: usize = BLOCKSIZE;
                    if (*zbc).inBuffSize < neededInSize {
                        free((*zbc).inBuff as *mut c_void);
                        (*zbc).inBuffSize = neededInSize;
                        (*zbc).inBuff = malloc(neededInSize) as *mut c_char;
                        if (*zbc).inBuff.is_null() {
                            return error(code::MEMORY_ALLOCATION);
                        }
                    }
                    if (*zbc).outBuffSize < neededOutSize {
                        free((*zbc).outBuff as *mut c_void);
                        (*zbc).outBuffSize = neededOutSize;
                        (*zbc).outBuff = malloc(neededOutSize) as *mut c_char;
                        if (*zbc).outBuff.is_null() {
                            return error(code::MEMORY_ALLOCATION);
                        }
                    }
                }
                if (*zbc).dictSize != 0 {
                    ZSTD_decompress_insertDictionary(
                        (*zbc).zc,
                        (*zbc).dict as *const c_void,
                        (*zbc).dictSize,
                    );
                }
                if (*zbc).hPos != 0 {
                    memcpy(
                        (*zbc).inBuff as *mut c_void,
                        (*zbc).headerBuffer.as_ptr() as *const c_void,
                        (*zbc).hPos,
                    );
                    (*zbc).inPos = (*zbc).hPos;
                    (*zbc).hPos = 0;
                    (*zbc).stage = ZBUFFds_load;
                    break 'sw;
                }
                (*zbc).stage = ZBUFFds_read;
                st = ZBUFFds_read; // fall-through
            }

            if st == ZBUFFds_read {
                let neededInSize = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
                if neededInSize == 0 {
                    (*zbc).stage = ZBUFFds_init;
                    notDone = 0;
                    break 'sw;
                }
                if (iend as usize - ip as usize) >= neededInSize {
                    let decodedSize = ZSTD_decompressContinue(
                        (*zbc).zc,
                        (*zbc).outBuff.add((*zbc).outStart) as *mut c_void,
                        (*zbc).outBuffSize - (*zbc).outStart,
                        ip as *const c_void,
                        neededInSize,
                    );
                    if ZSTD_isError(decodedSize) != 0 {
                        return decodedSize;
                    }
                    ip = ip.add(neededInSize);
                    if decodedSize == 0 {
                        break 'sw;
                    }
                    (*zbc).outEnd = (*zbc).outStart + decodedSize;
                    (*zbc).stage = ZBUFFds_flush;
                    break 'sw;
                }
                if ip == iend {
                    notDone = 0;
                    break 'sw;
                }
                (*zbc).stage = ZBUFFds_load;
                st = ZBUFFds_load; // fall-through
            }

            if st == ZBUFFds_load {
                let neededInSize = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
                let toLoad = neededInSize - (*zbc).inPos;
                let loadedSize: usize;
                if toLoad > (*zbc).inBuffSize - (*zbc).inPos {
                    return error(code::CORRUPTION_DETECTED);
                }
                loadedSize = ZBUFF_limitCopy(
                    (*zbc).inBuff.add((*zbc).inPos) as *mut c_void,
                    toLoad,
                    ip as *const c_void,
                    iend as usize - ip as usize,
                );
                ip = ip.add(loadedSize);
                (*zbc).inPos += loadedSize;
                if loadedSize < toLoad {
                    notDone = 0;
                    break 'sw;
                }
                {
                    let decodedSize = ZSTD_decompressContinue(
                        (*zbc).zc,
                        (*zbc).outBuff.add((*zbc).outStart) as *mut c_void,
                        (*zbc).outBuffSize - (*zbc).outStart,
                        (*zbc).inBuff as *const c_void,
                        neededInSize,
                    );
                    if ZSTD_isError(decodedSize) != 0 {
                        return decodedSize;
                    }
                    (*zbc).inPos = 0;
                    if decodedSize == 0 {
                        (*zbc).stage = ZBUFFds_read;
                        break 'sw;
                    }
                    (*zbc).outEnd = (*zbc).outStart + decodedSize;
                    (*zbc).stage = ZBUFFds_flush;
                }
                st = ZBUFFds_flush; // fall-through
            }

            if st == ZBUFFds_flush {
                let toFlushSize = (*zbc).outEnd - (*zbc).outStart;
                let flushedSize = ZBUFF_limitCopy(
                    op as *mut c_void,
                    oend as usize - op as usize,
                    (*zbc).outBuff.add((*zbc).outStart) as *const c_void,
                    toFlushSize,
                );
                op = op.add(flushedSize);
                (*zbc).outStart += flushedSize;
                if flushedSize == toFlushSize {
                    (*zbc).stage = ZBUFFds_read;
                    if (*zbc).outStart + BLOCKSIZE > (*zbc).outBuffSize {
                        (*zbc).outStart = 0;
                        (*zbc).outEnd = 0;
                    }
                    break 'sw;
                }
                notDone = 0;
                break 'sw;
            }

            return error(code::GENERIC);
        }
    }

    *srcSizePtr = ip as usize - istart as usize;
    *maxDstSizePtr = op as usize - ostart as usize;

    {
        let mut nextSrcSizeHint = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
        if nextSrcSizeHint > 3 {
            nextSrcSizeHint += 3;
        }
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbc).inPos);
        nextSrcSizeHint
    }
}

// ----------------------------------------------------------------------------
// Tool functions (exported)
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_isError(errorCode: usize) -> c_uint {
    err_is_error(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_getErrorName(errorCode: usize) -> *const c_char {
    crate::common::error::err_get_error_name(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_recommendedDInSize() -> usize {
    BLOCKSIZE + 3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_recommendedDOutSize() -> usize {
    BLOCKSIZE
}

// ----------------------------------------------------------------------------
// Final wrapping stage (exported)
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_decompressDCtx(
    dctx: *mut ZSTDv04_Dctx_s,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompress_usingDict(
        dctx,
        dst,
        maxDstSize,
        src,
        srcSize,
        core::ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    // ZSTD_HEAPMODE == 1
    let regenSize: usize;
    let dctx = ZSTD_createDCtx();
    if dctx.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    regenSize = ZSTDv04_decompressDCtx(dctx, dst, maxDstSize, src, srcSize);
    ZSTD_freeDCtx(dctx);
    regenSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_resetDCtx(dctx: *mut ZSTDv04_Dctx_s) -> usize {
    ZSTD_resetDCtx(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_nextSrcSizeToDecompress(dctx: *mut ZSTDv04_Dctx_s) -> usize {
    ZSTD_nextSrcSizeToDecompress(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_decompressContinue(
    dctx: *mut ZSTDv04_Dctx_s,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompressContinue(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_createDCtx() -> *mut ZBUFFv04_DCtx_s {
    ZBUFF_createDCtx()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_freeDCtx(dctx: *mut ZBUFFv04_DCtx_s) -> usize {
    ZBUFF_freeDCtx(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressInit(dctx: *mut ZBUFFv04_DCtx_s) -> usize {
    ZBUFF_decompressInit(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressWithDictionary(
    dctx: *mut ZBUFFv04_DCtx_s,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZBUFF_decompressWithDictionary(dctx, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressContinue(
    dctx: *mut ZBUFFv04_DCtx_s,
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    ZBUFF_decompressContinue(dctx, dst, maxDstSizePtr, src, srcSizePtr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_createDCtx() -> *mut ZSTDv04_Dctx_s {
    ZSTD_createDCtx()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_freeDCtx(dctx: *mut ZSTDv04_Dctx_s) -> usize {
    ZSTD_freeDCtx(dctx)
}








