//! Transliteration of `legacy/zstd_v07.c` (+ `zstd_v07.h`).
//!
//! Self-contained legacy v0.7 decoder: bundles FSEv07, HUFv07, ZSTDv07 and ZBUFFv07.
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
use crate::xxhash::{
    XXH64_state_t, ZSTD_XXH64 as XXH64, ZSTD_XXH64_digest as XXH64_digest,
    ZSTD_XXH64_reset as XXH64_reset, ZSTD_XXH64_update as XXH64_update,
};

/* ***  Basic types (mem.h) *** */
pub type BYTE = u8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

/* *************************************
*  zstd_v07.h : Constants
***************************************/
pub const ZSTDv07_MAGICNUMBER: U32 = 0xFD2FB527;

/* ====  ZSTDv07_STATIC_LINKING_ONLY  ==== */
pub const ZSTDv07_MAGIC_SKIPPABLE_START: U32 = 0x184D2A50;

pub const ZSTDv07_WINDOWLOG_MAX_32: U32 = 25;
pub const ZSTDv07_WINDOWLOG_MAX_64: U32 = 27;
#[inline(always)]
pub fn ZSTDv07_WINDOWLOG_MAX() -> U32 {
    if MEM_32bits() != 0 {
        ZSTDv07_WINDOWLOG_MAX_32
    } else {
        ZSTDv07_WINDOWLOG_MAX_64
    }
}
pub const ZSTDv07_WINDOWLOG_MIN: U32 = 18;
pub const ZSTDv07_CHAINLOG_MIN: U32 = 4;
pub const ZSTDv07_HASHLOG_MIN: U32 = 12;
pub const ZSTDv07_HASHLOG3_MAX: U32 = 17;
pub const ZSTDv07_SEARCHLOG_MIN: U32 = 1;
pub const ZSTDv07_SEARCHLENGTH_MAX: U32 = 7;
pub const ZSTDv07_SEARCHLENGTH_MIN: U32 = 3;
pub const ZSTDv07_TARGETLENGTH_MIN: U32 = 4;
pub const ZSTDv07_TARGETLENGTH_MAX: U32 = 999;

pub const ZSTDv07_FRAMEHEADERSIZE_MAX: usize = 18;
pub static ZSTDv07_frameHeaderSize_min: usize = 5;
pub static ZSTDv07_frameHeaderSize_max: usize = ZSTDv07_FRAMEHEADERSIZE_MAX;
pub static ZSTDv07_skippableHeaderSize: usize = 8;

pub const ZSTDv07_BLOCKSIZE_ABSOLUTEMAX: usize = 128 * 1024;

/* custom memory allocation functions */
pub type ZSTDv07_allocFunction = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type ZSTDv07_freeFunction = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTDv07_customMem {
    pub customAlloc: ZSTDv07_allocFunction,
    pub customFree: ZSTDv07_freeFunction,
    pub opaque: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTDv07_frameParams {
    pub frameContentSize: u64,
    pub windowSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
}

/* ******************************************************************
*  mem.h : low-level memory access routines
********************************************************************/
#[inline(always)]
pub fn MEM_32bits() -> c_uint {
    (core::mem::size_of::<usize>() == 4) as c_uint
}
#[inline(always)]
pub fn MEM_64bits() -> c_uint {
    (core::mem::size_of::<usize>() == 8) as c_uint
}

#[inline(always)]
pub fn MEM_isLittleEndian() -> c_uint {
    if cfg!(target_endian = "little") {
        1
    } else {
        0
    }
}

#[inline(always)]
pub unsafe fn MEM_read16(memPtr: *const c_void) -> U16 {
    core::ptr::read_unaligned(memPtr as *const U16)
}

#[inline(always)]
pub unsafe fn MEM_read32(memPtr: *const c_void) -> U32 {
    core::ptr::read_unaligned(memPtr as *const U32)
}

#[inline(always)]
pub unsafe fn MEM_read64(memPtr: *const c_void) -> U64 {
    core::ptr::read_unaligned(memPtr as *const U64)
}

#[inline(always)]
pub unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    core::ptr::write_unaligned(memPtr as *mut U16, value)
}

#[inline(always)]
pub fn MEM_swap32(input: U32) -> U32 {
    input.swap_bytes()
}

#[inline(always)]
pub fn MEM_swap64(input: U64) -> U64 {
    input.swap_bytes()
}

/*=== Little endian r/w ===*/

#[inline(always)]
pub unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        MEM_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p as U16).wrapping_add((*p.wrapping_add(1) as U16) << 8)
    }
}

#[inline(always)]
pub unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let p = memPtr as *mut BYTE;
        *p = val as BYTE;
        *p.wrapping_add(1) = (val >> 8) as BYTE;
    }
}

#[inline(always)]
pub unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        MEM_read32(memPtr)
    } else {
        MEM_swap32(MEM_read32(memPtr))
    }
}

#[inline(always)]
pub unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        MEM_read64(memPtr)
    } else {
        MEM_swap64(MEM_read64(memPtr))
    }
}

#[inline(always)]
pub unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    if MEM_32bits() != 0 {
        MEM_readLE32(memPtr) as usize
    } else {
        MEM_readLE64(memPtr) as usize
    }
}

/* ******************************************************************
*  bitstream (BITv07)
********************************************************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BITv07_DStream_t {
    pub bitContainer: usize,
    pub bitsConsumed: c_uint,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

pub type BITv07_DStream_status = U32;
pub const BITv07_DStream_unfinished: BITv07_DStream_status = 0;
pub const BITv07_DStream_endOfBuffer: BITv07_DStream_status = 1;
pub const BITv07_DStream_completed: BITv07_DStream_status = 2;
pub const BITv07_DStream_overflow: BITv07_DStream_status = 3;

#[inline(always)]
pub fn BITv07_highbit32(val: U32) -> c_uint {
    val.leading_zeros() ^ 31
}

/* BITv07_initDStream() */
pub unsafe fn BITv07_initDStream(
    bitD: *mut BITv07_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        memset(
            bitD as *mut c_void,
            0,
            core::mem::size_of::<BITv07_DStream_t>(),
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        /* normal case */
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char)
            .wrapping_add(srcSize)
            .wrapping_sub(core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        {
            let lastByte: BYTE = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1);
            (*bitD).bitsConsumed = if lastByte != 0 {
                8u32.wrapping_sub(BITv07_highbit32(lastByte as U32))
            } else {
                0
            };
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC); /* endMark not present */
            }
        }
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        {
            let p = srcBuffer as *const BYTE;
            /* switch with fall-through, srcSize in [1..7] */
            if srcSize >= 7 {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*p.wrapping_add(6) as usize) << (core::mem::size_of::<usize>() * 8 - 16),
                );
            }
            if srcSize >= 6 {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*p.wrapping_add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24),
                );
            }
            if srcSize >= 5 {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*p.wrapping_add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32),
                );
            }
            if srcSize >= 4 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*p.wrapping_add(3) as usize) << 24);
            }
            if srcSize >= 3 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*p.wrapping_add(2) as usize) << 16);
            }
            if srcSize >= 2 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*p.wrapping_add(1) as usize) << 8);
            }
        }
        {
            let lastByte: BYTE = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1);
            (*bitD).bitsConsumed = if lastByte != 0 {
                8u32.wrapping_sub(BITv07_highbit32(lastByte as U32))
            } else {
                0
            };
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC); /* endMark not present */
            }
        }
        (*bitD).bitsConsumed = (*bitD)
            .bitsConsumed
            .wrapping_add(((core::mem::size_of::<usize>() - srcSize) * 8) as U32);
    }

    srcSize
}

#[inline(always)]
pub unsafe fn BITv07_lookBits(bitD: *const BITv07_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> (bitMask.wrapping_sub(nbBits) & bitMask)
}

/* BITv07_lookBitsFast() : unsafe version; only works if nbBits >= 1 */
#[inline(always)]
pub unsafe fn BITv07_lookBitsFast(bitD: *const BITv07_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((bitMask.wrapping_add(1).wrapping_sub(nbBits)) & bitMask)
}

#[inline(always)]
pub unsafe fn BITv07_skipBits(bitD: *mut BITv07_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

#[inline(always)]
pub unsafe fn BITv07_readBits(bitD: *mut BITv07_DStream_t, nbBits: U32) -> usize {
    let value: usize = BITv07_lookBits(bitD, nbBits);
    BITv07_skipBits(bitD, nbBits);
    value
}

/* BITv07_readBitsFast() : unsafe version; only works if nbBits >= 1 */
#[inline(always)]
pub unsafe fn BITv07_readBitsFast(bitD: *mut BITv07_DStream_t, nbBits: U32) -> usize {
    let value: usize = BITv07_lookBitsFast(bitD, nbBits);
    BITv07_skipBits(bitD, nbBits);
    value
}

pub unsafe fn BITv07_reloadDStream(bitD: *mut BITv07_DStream_t) -> BITv07_DStream_status {
    if (*bitD).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
        /* should not happen => corruption detected */
        return BITv07_DStream_overflow;
    }

    if (*bitD).ptr as usize
        >= ((*bitD).start as usize).wrapping_add(core::mem::size_of::<usize>())
    {
        (*bitD).ptr = (*bitD)
            .ptr
            .wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BITv07_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize) < core::mem::size_of::<usize>() * 8 {
            return BITv07_DStream_endOfBuffer;
        }
        return BITv07_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv07_DStream_status = BITv07_DStream_unfinished;
        if ((*bitD).ptr as usize).wrapping_sub(nbBytes as usize) < (*bitD).start as usize {
            nbBytes = ((*bitD).ptr as usize).wrapping_sub((*bitD).start as usize) as U32; /* ptr > start */
            result = BITv07_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return result;
    }
}

/* BITv07_endOfDStream() */
#[inline(always)]
pub unsafe fn BITv07_endOfDStream(DStream: *const BITv07_DStream_t) -> c_uint {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed as usize == core::mem::size_of::<usize>() * 8)) as c_uint
}

/* ******************************************************************
*  FSEv07
********************************************************************/
pub type FSEv07_DTable = c_uint;

pub const fn FSEv07_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    (1 + (1usize << maxTableLog)) as usize
}

pub const FSEv07_NCOUNTBOUND: usize = 512;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv07_DState_t {
    pub state: usize,
    pub table: *const c_void, /* precise table may vary, depending on U16 */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv07_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
} /* sizeof U32 */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv07_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
} /* size == U32 */

/* ***************************************************************
*  FSE Constants
*****************************************************************/
pub const FSEv07_MAX_MEMORY_USAGE: u32 = 14;
pub const FSEv07_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSEv07_MAX_SYMBOL_VALUE: u32 = 255;

pub const FSEv07_MAX_TABLELOG: u32 = FSEv07_MAX_MEMORY_USAGE - 2;
pub const FSEv07_MAX_TABLESIZE: u32 = 1u32 << FSEv07_MAX_TABLELOG;
pub const FSEv07_MAXTABLESIZE_MASK: u32 = FSEv07_MAX_TABLESIZE - 1;
pub const FSEv07_DEFAULT_TABLELOG: u32 = FSEv07_DEFAULT_MEMORY_USAGE - 2;
pub const FSEv07_MIN_TABLELOG: u32 = 5;

pub const FSEv07_TABLELOG_ABSOLUTE_MAX: u32 = 15;

#[inline(always)]
pub const fn FSEv07_TABLESTEP(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[inline(always)]
pub unsafe fn FSEv07_initDState(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
    dt: *const FSEv07_DTable,
) {
    let ptr: *const c_void = dt as *const c_void;
    let DTableH: *const FSEv07_DTableHeader = ptr as *const FSEv07_DTableHeader;
    (*DStatePtr).state = BITv07_readBits(bitD, (*DTableH).tableLog as U32);
    BITv07_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

#[inline(always)]
pub unsafe fn FSEv07_peekSymbol(DStatePtr: *const FSEv07_DState_t) -> BYTE {
    let DInfo: FSEv07_decode_t = *((*DStatePtr).table as *const FSEv07_decode_t)
        .wrapping_add((*DStatePtr).state);
    DInfo.symbol
}

#[inline(always)]
pub unsafe fn FSEv07_updateState(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
) {
    let DInfo: FSEv07_decode_t = *((*DStatePtr).table as *const FSEv07_decode_t)
        .wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let lowBits: usize = BITv07_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
}

#[inline(always)]
pub unsafe fn FSEv07_decodeSymbol(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
) -> BYTE {
    let DInfo: FSEv07_decode_t = *((*DStatePtr).table as *const FSEv07_decode_t)
        .wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = BITv07_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* FSEv07_decodeSymbolFast() :
unsafe, only works if no symbol has a probability > 50% */
#[inline(always)]
pub unsafe fn FSEv07_decodeSymbolFast(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
) -> BYTE {
    let DInfo: FSEv07_decode_t = *((*DStatePtr).table as *const FSEv07_decode_t)
        .wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = BITv07_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* ******************************************************************
*  HUFv07
********************************************************************/
pub const HUFv07_TABLELOG_ABSOLUTEMAX: usize = 16;
pub const HUFv07_TABLELOG_MAX: u32 = 12;
pub const HUFv07_TABLELOG_DEFAULT: u32 = 11;
pub const HUFv07_SYMBOLVALUE_MAX: usize = 255;
pub const HUFv07_BLOCKSIZE_MAX: usize = 128 * 1024;

pub type HUFv07_DTable = U32;

pub const fn HUFv07_DTABLE_SIZE(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

/*-****************************************
*  FSE Error Management
******************************************/
#[unsafe(no_mangle)]
pub extern "C" fn FSEv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn FSEv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
*  HUF Error Management
****************************************************************/
#[unsafe(no_mangle)]
pub extern "C" fn HUFv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUFv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/*-**************************************************************
*  FSE NCount encoding-decoding
****************************************************************/
pub fn FSEv07_abs(a: i16) -> i16 {
    (if a < 0 { -(a as i32) } else { a as i32 }) as i16
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_readNCount(
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
    nbBits = ((bitStream & 0xF) as c_int) + FSEv07_MIN_TABLELOG as c_int; /* extract tableLog */
    if nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX as c_int {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1i32.wrapping_shl(nbBits as u32)) + 1;
    threshold = 1i32.wrapping_shl(nbBits as u32);
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0: c_uint = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 = n0.wrapping_add(24);
                if (ip as usize) < (iend as usize).wrapping_sub(5) {
                    ip = ip.wrapping_add(2);
                    bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr(bitCount as u32);
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
            if ((ip as usize) <= (iend as usize).wrapping_sub(7))
                || ((ip as usize).wrapping_add((bitCount >> 3) as usize)
                    <= (iend as usize).wrapping_sub(4))
            {
                ip = ip.wrapping_add((bitCount >> 3) as usize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr(bitCount as u32);
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
                if count as c_int >= threshold {
                    count = (count as c_int - max as c_int) as i16;
                }
                bitCount += nbBits;
            }

            count = (count as c_int - 1) as i16; /* extra accuracy */
            remaining -= FSEv07_abs(count) as c_int;
            *normalizedCounter.wrapping_add(charnum as usize) = count;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as c_int;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            if ((ip as usize) <= (iend as usize).wrapping_sub(7))
                || ((ip as usize).wrapping_add((bitCount >> 3) as usize)
                    <= (iend as usize).wrapping_sub(4))
            {
                ip = ip.wrapping_add((bitCount >> 3) as usize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * ((iend as isize) - 4 - (ip as isize))) as c_int;
                ip = iend.wrapping_sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr((bitCount & 31) as u32);
        }
    } /* while ((remaining>1) && (charnum<=*maxSVPtr)) */
    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    *maxSVPtr = charnum.wrapping_sub(1);

    ip = ip.wrapping_add(((bitCount + 7) >> 3) as usize);
    if ((ip as usize).wrapping_sub(istart as usize)) > hbSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    (ip as usize).wrapping_sub(istart as usize)
}

/* function-local `static const U32 l[14]` of HUFv07_readStats */
pub static HUFv07_readStats_l: [U32; 14] =
    [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];

/* HUFv07_readStats() :
Read compact Huffman tree, saved by HUFv07_writeCTable(). */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readStats(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightTotal: U32;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: usize;
    let mut oSize: usize;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.wrapping_add(0) as usize;
    /* memset(huffWeight, 0, hwSize); */ /* is not necessary */

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            oSize = HUFv07_readStats_l[iSize - 242] as usize;
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
            {
                let mut n: U32 = 0;
                while (n as usize) < oSize {
                    *huffWeight.wrapping_add(n as usize) =
                        *ip.wrapping_add((n / 2) as usize) >> 4;
                    *huffWeight.wrapping_add((n + 1) as usize) =
                        *ip.wrapping_add((n / 2) as usize) & 15;
                    n += 2;
                }
            }
        }
    } else {
        /* header compressed with FSE (normal case) */
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        oSize = FSEv07_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.wrapping_add(1) as *const c_void,
            iSize,
        ); /* max (hwSize-1) values decoded, as last one is implied */
        if FSEv07_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankStats as *mut c_void,
        0,
        (HUFv07_TABLELOG_ABSOLUTEMAX + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    {
        let mut n: U32 = 0;
        while (n as usize) < oSize {
            if (*huffWeight.wrapping_add(n as usize) as usize) >= HUFv07_TABLELOG_ABSOLUTEMAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            let w = *huffWeight.wrapping_add(n as usize) as usize;
            *rankStats.wrapping_add(w) = (*rankStats.wrapping_add(w)).wrapping_add(1);
            weightTotal = weightTotal.wrapping_add((1u32.wrapping_shl(w as u32)) >> 1);
            n += 1;
        }
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    {
        let tableLog: U32 = BITv07_highbit32(weightTotal).wrapping_add(1);
        if tableLog as usize > HUFv07_TABLELOG_ABSOLUTEMAX {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *tableLogPtr = tableLog;
        /* determine last weight */
        {
            let total: U32 = 1u32.wrapping_shl(tableLog);
            let rest: U32 = total.wrapping_sub(weightTotal);
            let verif: U32 = 1u32.wrapping_shl(BITv07_highbit32(rest));
            let lastWeight: U32 = BITv07_highbit32(rest).wrapping_add(1);
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected); /* last value must be a clean power of 2 */
            }
            *huffWeight.wrapping_add(oSize) = lastWeight as BYTE;
            *rankStats.wrapping_add(lastWeight as usize) =
                (*rankStats.wrapping_add(lastWeight as usize)).wrapping_add(1);
        }
    }

    /* check tree construction validity */
    if (*rankStats.wrapping_add(1) < 2) || ((*rankStats.wrapping_add(1) & 1) != 0) {
        return ERROR(ZSTD_error_corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

/* ******************************************************************
*  FSE : Finite State Entropy decoder
********************************************************************/

/* Function templates */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_createDTable(mut tableLog: c_uint) -> *mut FSEv07_DTable {
    if tableLog > FSEv07_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv07_TABLELOG_ABSOLUTE_MAX;
    }
    malloc(FSEv07_DTABLE_SIZE_U32(tableLog) * core::mem::size_of::<U32>()) as *mut FSEv07_DTable
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_freeDTable(dt: *mut FSEv07_DTable) {
    free(dt as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable(
    dt: *mut FSEv07_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let tdPtr: *mut c_void = dt.wrapping_add(1) as *mut c_void;
    let tableDecode: *mut FSEv07_decode_t = tdPtr as *mut FSEv07_decode_t;
    let mut symbolNext: [U16; FSEv07_MAX_SYMBOL_VALUE as usize + 1] =
        [0; FSEv07_MAX_SYMBOL_VALUE as usize + 1];

    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);
    let tableSize: U32 = 1u32.wrapping_shl(tableLog);
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);

    /* Sanity Checks */
    if maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSEv07_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    {
        let mut DTableH = FSEv07_DTableHeader {
            tableLog: 0,
            fastMode: 0,
        };
        DTableH.tableLog = tableLog as U16;
        DTableH.fastMode = 1;
        {
            let largeLimit: i16 = (1i32.wrapping_shl(tableLog.wrapping_sub(1))) as i16;
            let mut s: U32 = 0;
            while s < maxSV1 {
                if *normalizedCounter.wrapping_add(s as usize) == -1 {
                    (*tableDecode.wrapping_add(highThreshold as usize)).symbol = s as BYTE;
                    highThreshold = highThreshold.wrapping_sub(1);
                    symbolNext[s as usize] = 1;
                } else {
                    if *normalizedCounter.wrapping_add(s as usize) >= largeLimit {
                        DTableH.fastMode = 0;
                    }
                    symbolNext[s as usize] =
                        *normalizedCounter.wrapping_add(s as usize) as U16;
                }
                s += 1;
            }
        }
        memcpy(
            dt as *mut c_void,
            &DTableH as *const FSEv07_DTableHeader as *const c_void,
            core::mem::size_of::<FSEv07_DTableHeader>(),
        );
    }

    /* Spread symbols */
    {
        let tableMask: U32 = tableSize.wrapping_sub(1);
        let step: U32 = FSEv07_TABLESTEP(tableSize);
        let mut s: U32 = 0;
        let mut position: U32 = 0;
        while s < maxSV1 {
            let mut i: c_int = 0;
            while i < *normalizedCounter.wrapping_add(s as usize) as c_int {
                (*tableDecode.wrapping_add(position as usize)).symbol = s as BYTE;
                position = position.wrapping_add(step) & tableMask;
                while position > highThreshold {
                    position = position.wrapping_add(step) & tableMask; /* lowprob area */
                }
                i += 1;
            }
            s += 1;
        }

        if position != 0 {
            return ERROR(ZSTD_error_GENERIC); /* position must reach all cells once */
        }
    }

    /* Build Decoding table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol: BYTE = (*tableDecode.wrapping_add(u as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.wrapping_add(u as usize)).nbBits =
                tableLog.wrapping_sub(BITv07_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.wrapping_add(u as usize)).newState = ((nextState as U32)
                .wrapping_shl((*tableDecode.wrapping_add(u as usize)).nbBits as U32)
                .wrapping_sub(tableSize)) as U16;
            u += 1;
        }
    }

    0
}

/*-*******************************************************
*  Decompression (Byte symbols)
*********************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable_rle(
    dt: *mut FSEv07_DTable,
    symbolValue: BYTE,
) -> usize {
    let ptr: *mut c_void = dt as *mut c_void;
    let DTableH: *mut FSEv07_DTableHeader = ptr as *mut FSEv07_DTableHeader;
    let dPtr: *mut c_void = dt.wrapping_add(1) as *mut c_void;
    let cell: *mut FSEv07_decode_t = dPtr as *mut FSEv07_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable_raw(
    dt: *mut FSEv07_DTable,
    nbBits: c_uint,
) -> usize {
    let ptr: *mut c_void = dt as *mut c_void;
    let DTableH: *mut FSEv07_DTableHeader = ptr as *mut FSEv07_DTableHeader;
    let dPtr: *mut c_void = dt.wrapping_add(1) as *mut c_void;
    let dinfo: *mut FSEv07_decode_t = dPtr as *mut FSEv07_decode_t;
    let tableSize: c_uint = 1u32.wrapping_shl(nbBits);
    let tableMask: c_uint = tableSize.wrapping_sub(1);
    let maxSV1: c_uint = tableMask.wrapping_add(1);
    let mut s: c_uint;

    /* Sanity checks */
    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC); /* min size */
    }

    /* Build Decoding Table */
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    s = 0;
    while s < maxSV1 {
        (*dinfo.wrapping_add(s as usize)).newState = 0;
        (*dinfo.wrapping_add(s as usize)).symbol = s as BYTE;
        (*dinfo.wrapping_add(s as usize)).nbBits = nbBits as BYTE;
        s += 1;
    }

    0
}

pub unsafe fn FSEv07_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv07_DTable,
    fast: c_uint,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.wrapping_add(maxDstSize);
    let olimit: *mut BYTE = omax.wrapping_sub(3);

    let mut bitD = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1 = FSEv07_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2 = FSEv07_DState_t {
        state: 0,
        table: core::ptr::null(),
    };

    /* Init */
    {
        let errorCode: usize = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if FSEv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv07_initDState(&mut state1, &mut bitD, dt);
    FSEv07_initDState(&mut state2, &mut bitD, dt);

    /* 4 symbols per loop */
    loop {
        if !((BITv07_reloadDStream(&mut bitD) == BITv07_DStream_unfinished)
            && ((op as usize) < olimit as usize))
        {
            break;
        }
        *op.wrapping_add(0) = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state1, &mut bitD)
        };

        if (FSEv07_MAX_TABLELOG * 2 + 7) as usize > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            BITv07_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(1) = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state2, &mut bitD)
        };

        if (FSEv07_MAX_TABLELOG * 4 + 7) as usize > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            if BITv07_reloadDStream(&mut bitD) > BITv07_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.wrapping_add(2) = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state1, &mut bitD)
        };

        if (FSEv07_MAX_TABLELOG * 2 + 7) as usize > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            BITv07_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(3) = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state2, &mut bitD)
        };

        op = op.wrapping_add(4);
    }

    /* tail */
    loop {
        if op as usize > omax.wrapping_sub(2) as usize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state1, &mut bitD)
        };
        op = op.wrapping_add(1);

        if BITv07_reloadDStream(&mut bitD) == BITv07_DStream_overflow {
            *op = if fast != 0 {
                FSEv07_decodeSymbolFast(&mut state2, &mut bitD)
            } else {
                FSEv07_decodeSymbol(&mut state2, &mut bitD)
            };
            op = op.wrapping_add(1);
            break;
        }

        if op as usize > omax.wrapping_sub(2) as usize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state2, &mut bitD)
        };
        op = op.wrapping_add(1);

        if BITv07_reloadDStream(&mut bitD) == BITv07_DStream_overflow {
            *op = if fast != 0 {
                FSEv07_decodeSymbolFast(&mut state1, &mut bitD)
            } else {
                FSEv07_decodeSymbol(&mut state1, &mut bitD)
            };
            op = op.wrapping_add(1);
            break;
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv07_DTable,
) -> usize {
    let ptr: *const c_void = dt as *const c_void;
    let DTableH: *const FSEv07_DTableHeader = ptr as *const FSEv07_DTableHeader;
    let fastMode: U32 = (*DTableH).fastMode as U32;

    /* select fast mode (static) */
    if fastMode != 0 {
        return FSEv07_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSEv07_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut counting: [i16; FSEv07_MAX_SYMBOL_VALUE as usize + 1] =
        [0; FSEv07_MAX_SYMBOL_VALUE as usize + 1];
    /* DTable_max_t dt */
    let mut dt: [U32; FSEv07_DTABLE_SIZE_U32(FSEv07_MAX_TABLELOG)] =
        [0; FSEv07_DTABLE_SIZE_U32(FSEv07_MAX_TABLELOG)];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv07_MAX_SYMBOL_VALUE;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
    }

    /* normal FSE decoding mode */
    {
        let NCountLength: usize = FSEv07_readNCount(
            counting.as_mut_ptr(),
            &mut maxSymbolValue,
            &mut tableLog,
            istart as *const c_void,
            cSrcSize,
        );
        if FSEv07_isError(NCountLength) != 0 {
            return NCountLength;
        }
        if NCountLength >= cSrcSize {
            return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
        }
        ip = ip.wrapping_add(NCountLength);
        cSrcSize -= NCountLength;
    }

    {
        let errorCode: usize = FSEv07_buildDTable(
            dt.as_mut_ptr(),
            counting.as_ptr(),
            maxSymbolValue,
            tableLog,
        );
        if FSEv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv07_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const c_void,
        cSrcSize,
        dt.as_ptr(),
    ) /* always return, even if it is an error code */
}

/* ******************************************************************
   Huffman decoder, part of New Generation Entropy library
********************************************************************/

/*-***************************/
/*  generic DTableDesc       */
/*-***************************/

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DTableDesc {
    pub maxTableLog: BYTE,
    pub tableType: BYTE,
    pub tableLog: BYTE,
    pub reserved: BYTE,
}

pub unsafe fn HUFv07_getDTableDesc(table: *const HUFv07_DTable) -> DTableDesc {
    let mut dtd = DTableDesc {
        maxTableLog: 0,
        tableType: 0,
        tableLog: 0,
        reserved: 0,
    };
    memcpy(
        &mut dtd as *mut DTableDesc as *mut c_void,
        table as *const c_void,
        core::mem::size_of::<DTableDesc>(),
    );
    dtd
}

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUFv07_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
} /* single-symbol decoding */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX2(
    DTable: *mut HUFv07_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut huffWeight: [BYTE; HUFv07_SYMBOLVALUE_MAX + 1] = [0; HUFv07_SYMBOLVALUE_MAX + 1];
    let mut rankVal: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1] =
        [0; HUFv07_TABLELOG_ABSOLUTEMAX + 1]; /* large enough for values from 0 to 16 */
    let mut tableLog: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let mut iSize: usize;
    let dtPtr: *mut c_void = DTable.wrapping_add(1) as *mut c_void;
    let dt: *mut HUFv07_DEltX2 = dtPtr as *mut HUFv07_DEltX2;

    iSize = HUFv07_readStats(
        huffWeight.as_mut_ptr(),
        HUFv07_SYMBOLVALUE_MAX + 1,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }

    /* Table header */
    {
        let mut dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
        if tableLog > (dtd.maxTableLog as U32).wrapping_add(1) {
            return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable too small */
        }
        dtd.tableType = 0;
        dtd.tableLog = tableLog as BYTE;
        memcpy(
            DTable as *mut c_void,
            &dtd as *const DTableDesc as *const c_void,
            core::mem::size_of::<DTableDesc>(),
        );
    }

    /* Prepare ranks */
    {
        let mut n: U32 = 1;
        let mut nextRankStart: U32 = 0;
        while n < tableLog.wrapping_add(1) {
            let current: U32 = nextRankStart;
            nextRankStart =
                nextRankStart.wrapping_add(rankVal[n as usize].wrapping_shl(n.wrapping_sub(1)));
            rankVal[n as usize] = current;
            n += 1;
        }
    }

    /* fill DTable */
    {
        let mut n: U32 = 0;
        while n < nbSymbols {
            let w: U32 = huffWeight[n as usize] as U32;
            let length: U32 = (1u32.wrapping_shl(w)) >> 1;
            let mut i: U32;
            let mut D = HUFv07_DEltX2 { byte: 0, nbBits: 0 };
            D.byte = n as BYTE;
            D.nbBits = tableLog.wrapping_add(1).wrapping_sub(w) as BYTE;
            i = rankVal[w as usize];
            while i < rankVal[w as usize].wrapping_add(length) {
                *dt.wrapping_add(i as usize) = D;
                i += 1;
            }
            rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
            n += 1;
        }
    }

    iSize
}

pub unsafe fn HUFv07_decodeSymbolX2(
    Dstream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val: usize = BITv07_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.wrapping_add(val)).byte;
    BITv07_skipBits(Dstream, (*dt.wrapping_add(val)).nbBits as U32);
    c
}

/* HUFv07_DECODE_SYMBOLX2_0 */
macro_rules! HUFv07_DECODE_SYMBOLX2_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUFv07_decodeSymbolX2($DStreamPtr, $dt, $dtLog);
        $ptr = $ptr.wrapping_add(1);
    }};
}

/* HUFv07_DECODE_SYMBOLX2_1 */
macro_rules! HUFv07_DECODE_SYMBOLX2_1 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUFv07_TABLELOG_MAX <= 12) {
            HUFv07_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog)
        }
    }};
}

/* HUFv07_DECODE_SYMBOLX2_2 */
macro_rules! HUFv07_DECODE_SYMBOLX2_2 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            HUFv07_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog)
        }
    }};
}

pub unsafe fn HUFv07_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart: *mut BYTE = p;

    /* up to 4 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished)
        && (p as usize <= pEnd.wrapping_sub(4) as usize)
    {
        HUFv07_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX2_1!(p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished)
        && ((p as usize) < pEnd as usize)
    {
        HUFv07_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while (p as usize) < pEnd as usize {
        HUFv07_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    (pEnd as usize).wrapping_sub(pStart as usize)
}

pub unsafe fn HUFv07_decompress1X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_add(dstSize);
    let dtPtr: *const c_void = DTable.wrapping_add(1) as *const c_void;
    let dt: *const HUFv07_DEltX2 = dtPtr as *const HUFv07_DEltX2;
    let mut bitD = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    let dtLog: U32 = dtd.tableLog as U32;

    {
        let errorCode: usize = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv07_decodeStreamX2(op, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv07_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
    HUFv07_decompress1X2_usingDTable_internal(dst, dstSize, cSrc, cSrcSize, DTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X2_DCtx(
    DCtx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUFv07_readDTableX2(DCtx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv07_decompress1X2_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* HUFv07_CREATE_STATIC_DTABLEX2(DTable, HUFv07_TABLELOG_MAX) */
    let mut DTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)] =
        [0; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)];
    DTable[0] = (HUFv07_TABLELOG_MAX - 1).wrapping_mul(0x1000001);
    HUFv07_decompress1X2_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

pub unsafe fn HUFv07_decompress4X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    /* Check */
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }

    {
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let dtPtr: *const c_void = DTable.wrapping_add(1) as *const c_void;
        let dt: *const HUFv07_DEltX2 = dtPtr as *const HUFv07_DEltX2;

        /* Init */
        let mut bitD1 = BITv07_DStream_t {
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
        let length4: usize = cSrcSize.wrapping_sub(
            length1
                .wrapping_add(length2)
                .wrapping_add(length3)
                .wrapping_add(6),
        );
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
        let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        {
            let errorCode: usize =
                BITv07_initDStream(&mut bitD1, istart1 as *const c_void, length1);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode: usize =
                BITv07_initDStream(&mut bitD2, istart2 as *const c_void, length2);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode: usize =
                BITv07_initDStream(&mut bitD3, istart3 as *const c_void, length3);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode: usize =
                BITv07_initDStream(&mut bitD4, istart4 as *const c_void, length4);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv07_reloadDStream(&mut bitD1)
            | BITv07_reloadDStream(&mut bitD2)
            | BITv07_reloadDStream(&mut bitD3)
            | BITv07_reloadDStream(&mut bitD4);
        while (endSignal == BITv07_DStream_unfinished)
            && ((op4 as usize) < oend.wrapping_sub(7) as usize)
        {
            HUFv07_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_1!(op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_1!(op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_1!(op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_1!(op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_0!(op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_0!(op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_0!(op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_0!(op4, &mut bitD4, dt, dtLog);
            endSignal = BITv07_reloadDStream(&mut bitD1)
                | BITv07_reloadDStream(&mut bitD2)
                | BITv07_reloadDStream(&mut bitD3)
                | BITv07_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 as usize > opStart2 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 as usize > opStart3 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 as usize > opStart4 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUFv07_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv07_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv07_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv07_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BITv07_endOfDStream(&bitD1)
            & BITv07_endOfDStream(&bitD2)
            & BITv07_endOfDStream(&bitD3)
            & BITv07_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
        return dstSize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
    HUFv07_decompress4X2_usingDTable_internal(dst, dstSize, cSrc, cSrcSize, DTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X2_DCtx(
    dctx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUFv07_readDTableX2(dctx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv07_decompress4X2_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* HUFv07_CREATE_STATIC_DTABLEX2(DTable, HUFv07_TABLELOG_MAX) */
    let mut DTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)] =
        [0; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)];
    DTable[0] = (HUFv07_TABLELOG_MAX - 1).wrapping_mul(0x1000001);
    HUFv07_decompress4X2_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

/* *************************/
/* double-symbols decoding */
/* *************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUFv07_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
} /* double-symbols decoding */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
    pub weight: BYTE,
}

pub unsafe fn HUFv07_fillDTableX4Level2(
    DTable: *mut HUFv07_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: c_int,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    let mut DElt = HUFv07_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1] =
        [0; HUFv07_TABLELOG_ABSOLUTEMAX + 1];

    /* get pre-calculated rankVal */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: U32;
        let skipSize: U32 = rankVal[minWeight as usize];
        MEM_writeLE16(
            core::ptr::addr_of_mut!(DElt.sequence) as *mut c_void,
            baseSeq,
        );
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1;
        i = 0;
        while i < skipSize {
            *DTable.wrapping_add(i as usize) = DElt;
            i += 1;
        }
    }

    /* fill DTable */
    {
        let mut s: U32 = 0;
        while s < sortedListSize {
            /* note : sortedSymbols already skipped */
            let symbol: U32 = (*sortedSymbols.wrapping_add(s as usize)).symbol as U32;
            let weight: U32 = (*sortedSymbols.wrapping_add(s as usize)).weight as U32;
            let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
            let length: U32 = 1u32.wrapping_shl(sizeLog.wrapping_sub(nbBits));
            let start: U32 = rankVal[weight as usize];
            let mut i: U32 = start;
            let end: U32 = start.wrapping_add(length);

            MEM_writeLE16(
                core::ptr::addr_of_mut!(DElt.sequence) as *mut c_void,
                (baseSeq as U32).wrapping_add(symbol << 8) as U16,
            );
            DElt.nbBits = nbBits.wrapping_add(consumed) as BYTE;
            DElt.length = 2;
            loop {
                *DTable.wrapping_add(i as usize) = DElt;
                i += 1;
                if !(i < end) {
                    break;
                }
            } /* since length >= 1 */

            rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
            s += 1;
        }
    }
}

/* typedef U32 rankVal_t[HUFv07_TABLELOG_ABSOLUTEMAX][HUFv07_TABLELOG_ABSOLUTEMAX + 1]; */
pub type rankVal_t = [[U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1]; HUFv07_TABLELOG_ABSOLUTEMAX];

pub unsafe fn HUFv07_fillDTableX4(
    DTable: *mut HUFv07_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1] =
        [0; HUFv07_TABLELOG_ABSOLUTEMAX + 1];
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int; /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1]>(),
    );

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.wrapping_add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.wrapping_add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32.wrapping_shl(targetLog.wrapping_sub(nbBits));

        if targetLog.wrapping_sub(nbBits) >= minBits {
            /* enough room for a second symbol */
            let sortedRank: U32;
            let mut minWeight: c_int = (nbBits as c_int).wrapping_add(scaleLog);
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.wrapping_add(minWeight as usize);
            HUFv07_fillDTableX4Level2(
                DTable.wrapping_add(start as usize),
                targetLog.wrapping_sub(nbBits),
                nbBits,
                (*rankValOrigin.wrapping_add(nbBits as usize)).as_ptr(),
                minWeight,
                sortedList.wrapping_add(sortedRank as usize),
                sortedListSize.wrapping_sub(sortedRank),
                nbBitsBaseline,
                symbol,
            );
        } else {
            let mut DElt = HUFv07_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };
            MEM_writeLE16(
                core::ptr::addr_of_mut!(DElt.sequence) as *mut c_void,
                symbol,
            );
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1;
            {
                let mut u: U32;
                let end: U32 = start.wrapping_add(length);
                u = start;
                while u < end {
                    *DTable.wrapping_add(u as usize) = DElt;
                    u += 1;
                }
            }
        }
        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX4(
    DTable: *mut HUFv07_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightList: [BYTE; HUFv07_SYMBOLVALUE_MAX + 1] = [0; HUFv07_SYMBOLVALUE_MAX + 1];
    let mut sortedSymbol: [sortedSymbol_t; HUFv07_SYMBOLVALUE_MAX + 1] =
        [sortedSymbol_t {
            symbol: 0,
            weight: 0,
        }; HUFv07_SYMBOLVALUE_MAX + 1];
    let mut rankStats: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1] =
        [0; HUFv07_TABLELOG_ABSOLUTEMAX + 1];
    let mut rankStart0: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 2] =
        [0; HUFv07_TABLELOG_ABSOLUTEMAX + 2];
    let rankStart: *mut U32 = rankStart0.as_mut_ptr().wrapping_add(1);
    let mut rankVal: rankVal_t = [[0; HUFv07_TABLELOG_ABSOLUTEMAX + 1]; HUFv07_TABLELOG_ABSOLUTEMAX];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let mut sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let mut dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    let maxTableLog: U32 = dtd.maxTableLog as U32;
    let mut iSize: usize;
    let dtPtr: *mut c_void = DTable.wrapping_add(1) as *mut c_void; /* force compiler to avoid strict-aliasing */
    let dt: *mut HUFv07_DEltX4 = dtPtr as *mut HUFv07_DEltX4;

    if maxTableLog as usize > HUFv07_TABLELOG_ABSOLUTEMAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    /* memset(weightList, 0, sizeof(weightList)); */ /* is not necessary */

    iSize = HUFv07_readStats(
        weightList.as_mut_ptr(),
        HUFv07_SYMBOLVALUE_MAX + 1,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > maxTableLog {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable can't fit code depth */
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW = maxW.wrapping_sub(1);
    } /* necessarily finds a solution before 0 */

    /* Get start index of each weight */
    {
        let mut w: U32 = 1;
        let mut nextRankStart: U32 = 0;
        while w < maxW.wrapping_add(1) {
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankStats[w as usize]);
            *rankStart.wrapping_add(w as usize) = current;
            w += 1;
        }
        *rankStart.wrapping_add(0) = nextRankStart; /* put all 0w symbols at the end of sorted list*/
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = *rankStart.wrapping_add(w as usize);
            *rankStart.wrapping_add(w as usize) =
                (*rankStart.wrapping_add(w as usize)).wrapping_add(1);
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        *rankStart.wrapping_add(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let rankVal0: *mut U32 = rankVal[0].as_mut_ptr();
        {
            let rescale: c_int = (maxTableLog.wrapping_sub(tableLog) as c_int) - 1; /* tableLog <= maxTableLog */
            let mut nextRankVal: U32 = 0;
            let mut w: U32 = 1;
            while w < maxW.wrapping_add(1) {
                let current: U32 = nextRankVal;
                nextRankVal = nextRankVal.wrapping_add(
                    rankStats[w as usize].wrapping_shl(w.wrapping_add(rescale as U32)),
                );
                *rankVal0.wrapping_add(w as usize) = current;
                w += 1;
            }
        }
        {
            let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
            let mut consumed: U32 = minBits;
            while consumed < maxTableLog.wrapping_sub(minBits).wrapping_add(1) {
                let rankValPtr: *mut U32 = rankVal[consumed as usize].as_mut_ptr();
                let mut w: U32 = 1;
                while w < maxW.wrapping_add(1) {
                    *rankValPtr.wrapping_add(w as usize) =
                        *rankVal0.wrapping_add(w as usize) >> consumed;
                    w += 1;
                }
                consumed += 1;
            }
        }
    }

    HUFv07_fillDTableX4(
        dt,
        maxTableLog,
        sortedSymbol.as_ptr(),
        sizeOfSort,
        rankStart0.as_ptr(),
        rankVal.as_ptr(),
        maxW,
        tableLog.wrapping_add(1),
    );

    dtd.tableLog = maxTableLog as BYTE;
    dtd.tableType = 1;
    memcpy(
        DTable as *mut c_void,
        &dtd as *const DTableDesc as *const c_void,
        core::mem::size_of::<DTableDesc>(),
    );
    iSize
}

pub unsafe fn HUFv07_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BITv07_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 2);
    BITv07_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    (*dt.wrapping_add(val)).length as U32
}

pub unsafe fn HUFv07_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BITv07_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 1);
    if (*dt.wrapping_add(val)).length == 1 {
        BITv07_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    } else {
        if ((*DStream).bitsConsumed as usize) < (core::mem::size_of::<usize>() * 8) {
            BITv07_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
            if (*DStream).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as c_uint;
                /* ugly hack; works only because it's the last symbol */
            }
        }
    }
    1
}

macro_rules! HUFv07_DECODE_SYMBOLX4_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.wrapping_add(HUFv07_decodeSymbolX4(
            $ptr as *mut c_void,
            $DStreamPtr,
            $dt,
            $dtLog,
        ) as usize);
    }};
}

macro_rules! HUFv07_DECODE_SYMBOLX4_1 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUFv07_TABLELOG_MAX <= 12) {
            HUFv07_DECODE_SYMBOLX4_0!($ptr, $DStreamPtr, $dt, $dtLog)
        }
    }};
}

macro_rules! HUFv07_DECODE_SYMBOLX4_2 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            HUFv07_DECODE_SYMBOLX4_0!($ptr, $DStreamPtr, $dt, $dtLog)
        }
    }};
}

pub unsafe fn HUFv07_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart: *mut BYTE = p;

    /* up to 8 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished)
        && ((p as usize) < pEnd.wrapping_sub(7) as usize)
    {
        HUFv07_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX4_1!(p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to end : up to 2 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished)
        && (p as usize <= pEnd.wrapping_sub(2) as usize)
    {
        HUFv07_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p as usize <= pEnd.wrapping_sub(2) as usize {
        HUFv07_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog); /* no need to reload */
    }

    if (p as usize) < pEnd as usize {
        p = p.wrapping_add(
            HUFv07_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
        );
    }

    (p as usize).wrapping_sub(pStart as usize)
}

pub unsafe fn HUFv07_decompress1X4_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let mut bitD = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };

    /* Init */
    {
        let errorCode: usize = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    /* decode */
    {
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let dtPtr: *const c_void = DTable.wrapping_add(1) as *const c_void;
        let dt: *const HUFv07_DEltX4 = dtPtr as *const HUFv07_DEltX4;
        let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
        HUFv07_decodeStreamX4(ostart, &mut bitD, oend, dt, dtd.tableLog as U32);
    }

    /* check */
    if BITv07_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* decoded size */
    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    HUFv07_decompress1X4_usingDTable_internal(dst, dstSize, cSrc, cSrcSize, DTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X4_DCtx(
    DCtx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUFv07_readDTableX4(DCtx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv07_decompress1X4_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* HUFv07_CREATE_STATIC_DTABLEX4(DTable, HUFv07_TABLELOG_MAX) */
    let mut DTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)] =
        [0; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)];
    DTable[0] = HUFv07_TABLELOG_MAX.wrapping_mul(0x1000001);
    HUFv07_decompress1X4_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

pub unsafe fn HUFv07_decompress4X4_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }

    {
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let dtPtr: *const c_void = DTable.wrapping_add(1) as *const c_void;
        let dt: *const HUFv07_DEltX4 = dtPtr as *const HUFv07_DEltX4;

        /* Init */
        let mut bitD1 = BITv07_DStream_t {
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
        let length4: usize = cSrcSize.wrapping_sub(
            length1
                .wrapping_add(length2)
                .wrapping_add(length3)
                .wrapping_add(6),
        );
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
        let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        {
            let errorCode: usize =
                BITv07_initDStream(&mut bitD1, istart1 as *const c_void, length1);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode: usize =
                BITv07_initDStream(&mut bitD2, istart2 as *const c_void, length2);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode: usize =
                BITv07_initDStream(&mut bitD3, istart3 as *const c_void, length3);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode: usize =
                BITv07_initDStream(&mut bitD4, istart4 as *const c_void, length4);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv07_reloadDStream(&mut bitD1)
            | BITv07_reloadDStream(&mut bitD2)
            | BITv07_reloadDStream(&mut bitD3)
            | BITv07_reloadDStream(&mut bitD4);
        while (endSignal == BITv07_DStream_unfinished)
            && ((op4 as usize) < oend.wrapping_sub(7) as usize)
        {
            HUFv07_DECODE_SYMBOLX4_2!(op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2!(op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2!(op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2!(op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_1!(op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_1!(op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_1!(op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_1!(op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2!(op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2!(op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2!(op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2!(op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_0!(op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_0!(op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_0!(op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_0!(op4, &mut bitD4, dt, dtLog);

            endSignal = BITv07_reloadDStream(&mut bitD1)
                | BITv07_reloadDStream(&mut bitD2)
                | BITv07_reloadDStream(&mut bitD3)
                | BITv07_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 as usize > opStart2 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 as usize > opStart3 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 as usize > opStart4 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUFv07_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv07_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv07_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv07_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        {
            let endCheck: U32 = BITv07_endOfDStream(&bitD1)
                & BITv07_endOfDStream(&bitD2)
                & BITv07_endOfDStream(&bitD3)
                & BITv07_endOfDStream(&bitD4);
            if endCheck == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }

        /* decoded size */
        return dstSize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    HUFv07_decompress4X4_usingDTable_internal(dst, dstSize, cSrc, cSrcSize, DTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X4_DCtx(
    dctx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let mut hSize: usize = HUFv07_readDTableX4(dctx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv07_decompress4X4_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* HUFv07_CREATE_STATIC_DTABLEX4(DTable, HUFv07_TABLELOG_MAX) */
    let mut DTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)] =
        [0; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)];
    DTable[0] = HUFv07_TABLELOG_MAX.wrapping_mul(0x1000001);
    HUFv07_decompress4X4_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

/* ********************************/
/* Generic decompression selector */
/* ********************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        HUFv07_decompress1X4_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    } else {
        HUFv07_decompress1X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        HUFv07_decompress4X4_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    } else {
        HUFv07_decompress4X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}

pub static algoTime: [[algo_time_t; 3]; 16] = {
    macro_rules! at {
        ($a:expr, $b:expr) => {
            algo_time_t {
                tableTime: $a,
                decode256Time: $b,
            }
        };
    }
    [
        /* single, double, quad */
        [at!(0, 0), at!(1, 1), at!(2, 2)], /* Q==0 : impossible */
        [at!(0, 0), at!(1, 1), at!(2, 2)], /* Q==1 : impossible */
        [at!(38, 130), at!(1313, 74), at!(2151, 38)], /* Q == 2 : 12-18% */
        [at!(448, 128), at!(1353, 74), at!(2238, 41)], /* Q == 3 : 18-25% */
        [at!(556, 128), at!(1353, 74), at!(2238, 47)], /* Q == 4 : 25-32% */
        [at!(714, 128), at!(1418, 74), at!(2436, 53)], /* Q == 5 : 32-38% */
        [at!(883, 128), at!(1437, 74), at!(2464, 61)], /* Q == 6 : 38-44% */
        [at!(897, 128), at!(1515, 75), at!(2622, 68)], /* Q == 7 : 44-50% */
        [at!(926, 128), at!(1613, 75), at!(2730, 75)], /* Q == 8 : 50-56% */
        [at!(947, 128), at!(1729, 77), at!(3359, 77)], /* Q == 9 : 56-62% */
        [at!(1107, 128), at!(2083, 81), at!(4006, 84)], /* Q ==10 : 62-69% */
        [at!(1177, 128), at!(2379, 87), at!(4785, 88)], /* Q ==11 : 69-75% */
        [at!(1242, 128), at!(2415, 93), at!(5155, 84)], /* Q ==12 : 75-81% */
        [at!(1349, 128), at!(2644, 106), at!(5260, 106)], /* Q ==13 : 81-87% */
        [at!(1455, 128), at!(2422, 124), at!(4174, 124)], /* Q ==14 : 87-93% */
        [at!(722, 128), at!(1891, 145), at!(1936, 146)], /* Q ==15 : 93-99% */
    ]
};

/** HUFv07_selectDecoder() */
#[unsafe(no_mangle)]
pub extern "C" fn HUFv07_selectDecoder(dstSize: usize, cSrcSize: usize) -> U32 {
    /* decoder timing evaluation */
    let Q: U32 = (cSrcSize.wrapping_mul(16) / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
    let D256: U32 = (dstSize >> 8) as U32;
    let DTime0: U32 = algoTime[Q as usize][0]
        .tableTime
        .wrapping_add(algoTime[Q as usize][0].decode256Time.wrapping_mul(D256));
    let mut DTime1: U32 = algoTime[Q as usize][1]
        .tableTime
        .wrapping_add(algoTime[Q as usize][1].decode256Time.wrapping_mul(D256));
    DTime1 = DTime1.wrapping_add(DTime1 >> 3); /* advantage to algorithm using less memory */

    (DTime1 < DTime0) as U32
}

pub type decompressionAlgo =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;

/* function-local `static const decompressionAlgo decompress[2]` of HUFv07_decompress */
pub static HUFv07_decompress_decompress: [decompressionAlgo; 2] =
    [HUFv07_decompress4X2, HUFv07_decompress4X4];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
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

    {
        let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize);
        return HUFv07_decompress_decompress[algoNb as usize](dst, dstSize, cSrc, cSrcSize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X_DCtx(
    dctx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
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

    {
        let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize);
        return if algoNb != 0 {
            HUFv07_decompress4X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv07_decompress4X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X_hufOnly(
    dctx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if (cSrcSize >= dstSize) || (cSrcSize <= 1) {
        return ERROR(ZSTD_error_corruption_detected); /* invalid */
    }

    {
        let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize);
        return if algoNb != 0 {
            HUFv07_decompress4X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv07_decompress4X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X_DCtx(
    dctx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
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

    {
        let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize);
        return if algoNb != 0 {
            HUFv07_decompress1X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv07_decompress1X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        };
    }
}

/*
    Common functions of Zstd compression library
*/

/*-****************************************
*  ZSTD Error Management
******************************************/
/* ! ZSTDv07_isError() : tells if a return value is an error code */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/* ! ZSTDv07_getErrorName() */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
*  ZBUFF Error Management
****************************************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv07_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv07_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

/* `extern` (implicit "C" ABI) — these two are `static` in the C source and are
 * only ever reached through the C function pointers in `ZSTDv07_customMem`. */
#[allow(missing_abi)]
pub unsafe extern fn ZSTDv07_defaultAllocFunction(
    opaque: *mut c_void,
    size: usize,
) -> *mut c_void {
    let address: *mut c_void = malloc(size);
    let _ = opaque;
    /* printf("alloc %p, %d opaque=%p \n", address, (int)size, opaque); */
    address
}

#[allow(missing_abi)]
pub unsafe extern fn ZSTDv07_defaultFreeFunction(opaque: *mut c_void, address: *mut c_void) {
    let _ = opaque;
    /* if (address) printf("free %p opaque=%p \n", address, opaque); */
    free(address);
}

/*
    zstd_internal - common functions to include
*/

/*-*************************************
*  Common macros
***************************************/
#[inline(always)]
pub fn MIN_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}
#[inline(always)]
pub fn MAX_u32(a: U32, b: U32) -> U32 {
    if a > b {
        a
    } else {
        b
    }
}

/*-*************************************
*  Common constants
***************************************/
pub const ZSTDv07_OPT_NUM: u32 = 1 << 12;
pub const ZSTDv07_DICT_MAGIC: U32 = 0xEC30A437; /* v0.7 */

pub const ZSTDv07_REP_NUM: usize = 3;
pub const ZSTDv07_REP_INIT: usize = ZSTDv07_REP_NUM;
pub const ZSTDv07_REP_MOVE: usize = ZSTDv07_REP_NUM - 1;
pub static repStartValue: [U32; ZSTDv07_REP_NUM] = [1, 4, 8];

pub const BIT7: u32 = 128;
pub const BIT6: u32 = 64;
pub const BIT5: u32 = 32;
pub const BIT4: u32 = 16;
pub const BIT1: u32 = 2;
pub const BIT0: u32 = 1;

pub const ZSTDv07_WINDOWLOG_ABSOLUTEMIN: u32 = 10;
pub static ZSTDv07_fcs_fieldSize: [usize; 4] = [0, 2, 4, 8];
pub static ZSTDv07_did_fieldSize: [usize; 4] = [0, 1, 2, 4];

pub const ZSTDv07_BLOCKHEADERSIZE: usize = 3;
pub static ZSTDv07_blockHeaderSize: usize = ZSTDv07_BLOCKHEADERSIZE;

pub type blockType_t = c_int;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

pub const MIN_SEQUENCES_SIZE: usize = 1; /* nbSeq==0 */
pub const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

pub type litBlockType_t = c_int;
pub const lbt_huffman: litBlockType_t = 0;
pub const lbt_repeat: litBlockType_t = 1;
pub const lbt_raw: litBlockType_t = 2;
pub const lbt_rle: litBlockType_t = 3;

pub const LONGNBSEQ: u32 = 0x7F00;

pub const MINMATCH: usize = 3;
pub const EQUAL_READ32: usize = 4;

pub const Litbits: u32 = 8;
pub const MaxLit: usize = (1 << Litbits) - 1;
pub const MaxML: usize = 52;
pub const MaxLL: usize = 35;
pub const MaxOff: usize = 28;
pub const MaxSeq: usize = if MaxLL > MaxML { MaxLL } else { MaxML };
pub const MLFSELog: u32 = 9;
pub const LLFSELog: u32 = 9;
pub const OffFSELog: u32 = 8;

pub const FSEv07_ENCODING_RAW: U32 = 0;
pub const FSEv07_ENCODING_RLE: U32 = 1;
pub const FSEv07_ENCODING_STATIC: U32 = 2;
pub const FSEv07_ENCODING_DYNAMIC: U32 = 3;

pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

pub static LL_bits: [U32; MaxLL + 1] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
pub static LL_defaultNorm: [S16; MaxLL + 1] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
pub static LL_defaultNormLog: U32 = 6;

pub static ML_bits: [U32; MaxML + 1] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
pub static ML_defaultNorm: [S16; MaxML + 1] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
pub static ML_defaultNormLog: U32 = 6;

pub static OF_defaultNorm: [S16; MaxOff + 1] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
pub static OF_defaultNormLog: U32 = 5;

/*-*******************************************
*  Shared functions to include for inlining
*********************************************/
#[inline(always)]
pub unsafe fn ZSTDv07_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

pub const WILDCOPY_OVERLENGTH: usize = 8;

#[inline(always)]
pub unsafe fn ZSTDv07_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_offset(length);
    loop {
        ZSTDv07_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
        if !((op as usize) < oend as usize) {
            break;
        }
    }
}

/* custom memory allocation functions */
pub const defaultCustomMem: ZSTDv07_customMem = ZSTDv07_customMem {
    customAlloc: Some(ZSTDv07_defaultAllocFunction),
    customFree: Some(ZSTDv07_defaultFreeFunction),
    opaque: core::ptr::null_mut(),
};

/*
    zstd - standard compression library
*/

/*_*******************************************************
*  Memory operations
**********************************************************/
pub unsafe fn ZSTDv07_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

/*-*************************************************************
*   Context management
***************************************************************/
pub type ZSTDv07_dStage = c_int;
pub const ZSTDds_getFrameHeaderSize: ZSTDv07_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTDv07_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTDv07_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTDv07_dStage = 3;
pub const ZSTDds_decodeSkippableHeader: ZSTDv07_dStage = 4;
pub const ZSTDds_skipFrame: ZSTDv07_dStage = 5;

#[repr(C)]
pub struct ZSTDv07_DCtx {
    pub LLTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(LLFSELog)],
    pub OffTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(OffFSELog)],
    pub MLTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(MLFSELog)],
    pub hufTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: usize,
    pub rep: [U32; 3],
    pub fParams: ZSTDv07_frameParams,
    pub bType: blockType_t,
    pub stage: ZSTDv07_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: usize,
    pub dictID: U32,
    pub litPtr: *const BYTE,
    pub customMem: ZSTDv07_customMem,
    pub litSize: usize,
    pub litBuffer: [BYTE; ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [BYTE; ZSTDv07_FRAMEHEADERSIZE_MAX],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_sizeofDCtx(dctx: *const ZSTDv07_DCtx) -> usize {
    core::mem::size_of::<ZSTDv07_DCtx>()
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv07_estimateDCtxSize() -> usize {
    core::mem::size_of::<ZSTDv07_DCtx>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBegin(dctx: *mut ZSTDv07_DCtx) -> usize {
    (*dctx).expected = ZSTDv07_frameHeaderSize_min;
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).hufTable[0] = (ZSTD_HUFFDTABLE_CAPACITY_LOG.wrapping_mul(0x1000001)) as HUFv07_DTable;
    (*dctx).fseEntropy = 0;
    (*dctx).litEntropy = (*dctx).fseEntropy;
    (*dctx).dictID = 0;
    {
        let mut i: c_int = 0;
        while i < ZSTDv07_REP_NUM as c_int {
            (*dctx).rep[i as usize] = repStartValue[i as usize];
            i += 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDCtx_advanced(
    mut customMem: ZSTDv07_customMem,
) -> *mut ZSTDv07_DCtx {
    let dctx: *mut ZSTDv07_DCtx;

    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem;
    }

    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    dctx = (customMem.customAlloc.unwrap())(
        customMem.opaque,
        core::mem::size_of::<ZSTDv07_DCtx>(),
    ) as *mut ZSTDv07_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    memcpy(
        core::ptr::addr_of_mut!((*dctx).customMem) as *mut c_void,
        &customMem as *const ZSTDv07_customMem as *const c_void,
        core::mem::size_of::<ZSTDv07_customMem>(),
    );
    ZSTDv07_decompressBegin(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDCtx() -> *mut ZSTDv07_DCtx {
    ZSTDv07_createDCtx_advanced(defaultCustomMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_freeDCtx(dctx: *mut ZSTDv07_DCtx) -> usize {
    if dctx.is_null() {
        return 0; /* support free on NULL */
    }
    ((*dctx).customMem.customFree.unwrap())((*dctx).customMem.opaque, dctx as *mut c_void);
    0 /* reserved as a potential error code in the future */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_copyDCtx(
    dstDCtx: *mut ZSTDv07_DCtx,
    srcDCtx: *const ZSTDv07_DCtx,
) {
    memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv07_DCtx>()
            - (ZSTDv07_BLOCKSIZE_ABSOLUTEMAX
                + WILDCOPY_OVERLENGTH
                + ZSTDv07_frameHeaderSize_max),
    ); /* no need to copy workspace */
}

/*-*************************************************************
*   Decompression section
***************************************************************/

/** ZSTDv07_frameHeaderSize() */
pub unsafe fn ZSTDv07_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let fhd: BYTE = *(src as *const BYTE).wrapping_add(4);
        let dictID: U32 = (fhd & 3) as U32;
        let directMode: U32 = ((fhd >> 5) & 1) as U32;
        let fcsId: U32 = (fhd >> 6) as U32;
        return ZSTDv07_frameHeaderSize_min
            + ((directMode == 0) as usize)
            + ZSTDv07_did_fieldSize[dictID as usize]
            + ZSTDv07_fcs_fieldSize[fcsId as usize]
            + ((directMode != 0 && ZSTDv07_fcs_fieldSize[fcsId as usize] == 0) as usize);
    }
}

/** ZSTDv07_getFrameParams() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getFrameParams(
    fparamsPtr: *mut ZSTDv07_frameParams,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip: *const BYTE = src as *const BYTE;

    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ZSTDv07_frameHeaderSize_min;
    }
    memset(
        fparamsPtr as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv07_frameParams>(),
    );
    if MEM_readLE32(src) != ZSTDv07_MAGICNUMBER {
        if (MEM_readLE32(src) & 0xFFFFFFF0u32) == ZSTDv07_MAGIC_SKIPPABLE_START {
            if srcSize < ZSTDv07_skippableHeaderSize {
                return ZSTDv07_skippableHeaderSize; /* magic number + skippable frame length */
            }
            (*fparamsPtr).frameContentSize =
                MEM_readLE32((src as *const c_char).wrapping_add(4) as *const c_void) as u64;
            (*fparamsPtr).windowSize = 0; /* windowSize==0 means a frame is skippable */
            return 0;
        }
        return ERROR(ZSTD_error_prefix_unknown);
    }

    /* ensure there is enough `srcSize` to fully read/decode frame header */
    {
        let fhsize: usize = ZSTDv07_frameHeaderSize(src, srcSize);
        if srcSize < fhsize {
            return fhsize;
        }
    }

    {
        let fhdByte: BYTE = *ip.wrapping_add(4);
        let mut pos: usize = 5;
        let dictIDSizeCode: U32 = (fhdByte & 3) as U32;
        let checksumFlag: U32 = ((fhdByte >> 2) & 1) as U32;
        let directMode: U32 = ((fhdByte >> 5) & 1) as U32;
        let fcsID: U32 = (fhdByte >> 6) as U32;
        let windowSizeMax: U32 = 1u32 << ZSTDv07_WINDOWLOG_MAX();
        let mut windowSize: U32 = 0;
        let mut dictID: U32 = 0;
        let mut frameContentSize: U64 = 0;
        if (fhdByte & 0x08) != 0 {
            /* reserved bits, which must be zero */
            return ERROR(ZSTD_error_frameParameter_unsupported);
        }
        if directMode == 0 {
            let wlByte: BYTE = *ip.wrapping_add(pos);
            pos += 1;
            let windowLog: U32 = ((wlByte >> 3) as U32).wrapping_add(ZSTDv07_WINDOWLOG_ABSOLUTEMIN);
            if windowLog > ZSTDv07_WINDOWLOG_MAX() {
                return ERROR(ZSTD_error_frameParameter_unsupported);
            }
            windowSize = 1u32 << windowLog;
            windowSize =
                windowSize.wrapping_add((windowSize >> 3).wrapping_mul((wlByte & 7) as U32));
        }

        match dictIDSizeCode {
            1 => {
                dictID = *ip.wrapping_add(pos) as U32;
                pos += 1;
            }
            2 => {
                dictID = MEM_readLE16(ip.wrapping_add(pos) as *const c_void) as U32;
                pos += 2;
            }
            3 => {
                dictID = MEM_readLE32(ip.wrapping_add(pos) as *const c_void);
                pos += 4;
            }
            /* default / case 0 */
            _ => {}
        }
        match fcsID {
            1 => {
                frameContentSize =
                    (MEM_readLE16(ip.wrapping_add(pos) as *const c_void) as U64).wrapping_add(256);
            }
            2 => {
                frameContentSize = MEM_readLE32(ip.wrapping_add(pos) as *const c_void) as U64;
            }
            3 => {
                frameContentSize = MEM_readLE64(ip.wrapping_add(pos) as *const c_void);
            }
            /* default / case 0 */
            _ => {
                if directMode != 0 {
                    frameContentSize = *ip.wrapping_add(pos) as U64;
                }
            }
        }
        if windowSize == 0 {
            windowSize = frameContentSize as U32;
        }
        if windowSize > windowSizeMax {
            return ERROR(ZSTD_error_frameParameter_unsupported);
        }
        (*fparamsPtr).frameContentSize = frameContentSize;
        (*fparamsPtr).windowSize = windowSize;
        (*fparamsPtr).dictID = dictID;
        (*fparamsPtr).checksumFlag = checksumFlag;
    }
    0
}

/** ZSTDv07_getDecompressedSize() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getDecompressedSize(src: *const c_void, srcSize: usize) -> u64 {
    let mut fparams = ZSTDv07_frameParams::default();
    let frResult: usize = ZSTDv07_getFrameParams(&mut fparams, src, srcSize);
    if frResult != 0 {
        return 0;
    }
    fparams.frameContentSize
}

/** ZSTDv07_decodeFrameHeader() */
pub unsafe fn ZSTDv07_decodeFrameHeader(
    dctx: *mut ZSTDv07_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result: usize =
        ZSTDv07_getFrameParams(core::ptr::addr_of_mut!((*dctx).fParams), src, srcSize);
    if (*dctx).fParams.dictID != 0 && ((*dctx).dictID != (*dctx).fParams.dictID) {
        return ERROR(ZSTD_error_dictionary_wrong);
    }
    if (*dctx).fParams.checksumFlag != 0 {
        XXH64_reset(core::ptr::addr_of_mut!((*dctx).xxhState), 0);
    }
    result
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

/* ! ZSTDv07_getcBlockSize() */
pub unsafe fn ZSTDv07_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_: *const BYTE = src as *const BYTE;
    let cSize: U32;

    if srcSize < ZSTDv07_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (*bpPtr).blockType = ((*in_) >> 6) as blockType_t;
    cSize = (*in_.wrapping_add(2) as U32)
        .wrapping_add((*in_.wrapping_add(1) as U32) << 8)
        .wrapping_add(((*in_.wrapping_add(0) & 7) as U32) << 16);
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as usize
}

pub unsafe fn ZSTDv07_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

/* ! ZSTDv07_decodeLiteralsBlock() :
@return : nb of bytes read from src (< srcSize ) */
pub unsafe fn ZSTDv07_decodeLiteralsBlock(
    dctx: *mut ZSTDv07_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart.wrapping_add(0) >> 6) as litBlockType_t {
        lbt_huffman => {
            let litSize: usize;
            let litCSize: usize;
            let mut singleStream: usize = 0;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) >> 4) & 3) as U32;
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3 */
            }
            match lhSize {
                2 => {
                    /* 2 - 2 - 14 - 14 */
                    lhSize = 4;
                    litSize = (((*istart.wrapping_add(0) & 15) as usize) << 10)
                        + ((*istart.wrapping_add(1) as usize) << 2)
                        + ((*istart.wrapping_add(2) >> 6) as usize);
                    litCSize = (((*istart.wrapping_add(2) & 63) as usize) << 8)
                        + (*istart.wrapping_add(3) as usize);
                }
                3 => {
                    /* 2 - 2 - 18 - 18 */
                    lhSize = 5;
                    litSize = (((*istart.wrapping_add(0) & 15) as usize) << 14)
                        + ((*istart.wrapping_add(1) as usize) << 6)
                        + ((*istart.wrapping_add(2) >> 2) as usize);
                    litCSize = (((*istart.wrapping_add(2) & 3) as usize) << 16)
                        + ((*istart.wrapping_add(3) as usize) << 8)
                        + (*istart.wrapping_add(4) as usize);
                }
                /* case 0: case 1: default: */
                _ => {
                    /* 2 - 2 - 10 - 10 */
                    lhSize = 3;
                    singleStream = (*istart.wrapping_add(0) & 16) as usize;
                    litSize = (((*istart.wrapping_add(0) & 15) as usize) << 6)
                        + ((*istart.wrapping_add(1) >> 2) as usize);
                    litCSize = (((*istart.wrapping_add(1) & 3) as usize) << 8)
                        + (*istart.wrapping_add(2) as usize);
                }
            }
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if litCSize + (lhSize as usize) > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            if HUFv07_isError(if singleStream != 0 {
                HUFv07_decompress1X2_DCtx(
                    core::ptr::addr_of_mut!((*dctx).hufTable) as *mut HUFv07_DTable,
                    core::ptr::addr_of_mut!((*dctx).litBuffer) as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            } else {
                HUFv07_decompress4X_hufOnly(
                    core::ptr::addr_of_mut!((*dctx).hufTable) as *mut HUFv07_DTable,
                    core::ptr::addr_of_mut!((*dctx).litBuffer) as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            }) != 0
            {
                return ERROR(ZSTD_error_corruption_detected);
            }

            (*dctx).litPtr = core::ptr::addr_of_mut!((*dctx).litBuffer) as *const BYTE;
            (*dctx).litSize = litSize;
            (*dctx).litEntropy = 1;
            memset(
                (core::ptr::addr_of_mut!((*dctx).litBuffer) as *mut BYTE)
                    .wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            return litCSize + lhSize as usize;
        }
        lbt_repeat => {
            let litSize: usize;
            let litCSize: usize;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) >> 4) & 3) as U32;
            if lhSize != 1 {
                /* only case supported for now : small litSize, single stream */
                return ERROR(ZSTD_error_corruption_detected);
            }
            if (*dctx).litEntropy == 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }

            /* 2 - 2 - 10 - 10 */
            lhSize = 3;
            litSize = (((*istart.wrapping_add(0) & 15) as usize) << 6)
                + ((*istart.wrapping_add(1) >> 2) as usize);
            litCSize = (((*istart.wrapping_add(1) & 3) as usize) << 8)
                + (*istart.wrapping_add(2) as usize);
            if litCSize + (lhSize as usize) > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            {
                let errorCode: usize = HUFv07_decompress1X4_usingDTable(
                    core::ptr::addr_of_mut!((*dctx).litBuffer) as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                    core::ptr::addr_of!((*dctx).hufTable) as *const HUFv07_DTable,
                );
                if HUFv07_isError(errorCode) != 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            (*dctx).litPtr = core::ptr::addr_of_mut!((*dctx).litBuffer) as *const BYTE;
            (*dctx).litSize = litSize;
            memset(
                (core::ptr::addr_of_mut!((*dctx).litBuffer) as *mut BYTE)
                    .wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            return litCSize + lhSize as usize;
        }
        lbt_raw => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) >> 4) & 3) as U32;
            match lhSize {
                2 => {
                    litSize = (((*istart.wrapping_add(0) & 15) as usize) << 8)
                        + (*istart.wrapping_add(1) as usize);
                }
                3 => {
                    litSize = (((*istart.wrapping_add(0) & 15) as usize) << 16)
                        + ((*istart.wrapping_add(1) as usize) << 8)
                        + (*istart.wrapping_add(2) as usize);
                }
                /* case 0: case 1: default: */
                _ => {
                    lhSize = 1;
                    litSize = (*istart.wrapping_add(0) & 31) as usize;
                }
            }

            if (lhSize as usize) + litSize + WILDCOPY_OVERLENGTH > srcSize {
                /* risk reading beyond src buffer with wildcopy */
                if litSize + (lhSize as usize) > srcSize {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                memcpy(
                    core::ptr::addr_of_mut!((*dctx).litBuffer) as *mut c_void,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litSize,
                );
                (*dctx).litPtr = core::ptr::addr_of_mut!((*dctx).litBuffer) as *const BYTE;
                (*dctx).litSize = litSize;
                memset(
                    (core::ptr::addr_of_mut!((*dctx).litBuffer) as *mut BYTE)
                        .wrapping_add((*dctx).litSize) as *mut c_void,
                    0,
                    WILDCOPY_OVERLENGTH,
                );
                return (lhSize as usize) + litSize;
            }
            /* direct reference into compressed stream */
            (*dctx).litPtr = istart.wrapping_add(lhSize as usize);
            (*dctx).litSize = litSize;
            return (lhSize as usize) + litSize;
        }
        lbt_rle => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) >> 4) & 3) as U32;
            match lhSize {
                2 => {
                    litSize = (((*istart.wrapping_add(0) & 15) as usize) << 8)
                        + (*istart.wrapping_add(1) as usize);
                }
                3 => {
                    litSize = (((*istart.wrapping_add(0) & 15) as usize) << 16)
                        + ((*istart.wrapping_add(1) as usize) << 8)
                        + (*istart.wrapping_add(2) as usize);
                    if srcSize < 4 {
                        return ERROR(ZSTD_error_corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3 */
                    }
                }
                /* case 0: case 1: default: */
                _ => {
                    lhSize = 1;
                    litSize = (*istart.wrapping_add(0) & 31) as usize;
                }
            }
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memset(
                core::ptr::addr_of_mut!((*dctx).litBuffer) as *mut c_void,
                *istart.wrapping_add(lhSize as usize) as c_int,
                litSize + WILDCOPY_OVERLENGTH,
            );
            (*dctx).litPtr = core::ptr::addr_of_mut!((*dctx).litBuffer) as *const BYTE;
            (*dctx).litSize = litSize;
            return (lhSize as usize) + 1;
        }
        _ => {
            return ERROR(ZSTD_error_corruption_detected); /* impossible */
        }
    }
}

/* ! ZSTDv07_buildSeqTable() */
pub unsafe fn ZSTDv07_buildSeqTable(
    DTable: *mut FSEv07_DTable,
    type_: U32,
    mut max: U32,
    maxLog: U32,
    src: *const c_void,
    srcSize: usize,
    defaultNorm: *const S16,
    defaultLog: U32,
    flagRepeatTable: U32,
) -> usize {
    if type_ == FSEv07_ENCODING_RLE {
        if srcSize == 0 {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if (*(src as *const BYTE) as U32) > max {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSEv07_buildDTable_rle(DTable, *(src as *const BYTE)); /* if *src > max, data is corrupted */
        return 1;
    } else if type_ == FSEv07_ENCODING_RAW {
        FSEv07_buildDTable(DTable, defaultNorm, max, defaultLog);
        return 0;
    } else if type_ == FSEv07_ENCODING_STATIC {
        if flagRepeatTable == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        return 0;
    } else {
        /* default / FSEv07_ENCODING_DYNAMIC */
        let mut tableLog: U32 = 0;
        let mut norm: [S16; MaxSeq + 1] = [0; MaxSeq + 1];
        let headerSize: usize = FSEv07_readNCount(
            norm.as_mut_ptr(),
            &mut max,
            &mut tableLog,
            src,
            srcSize,
        );
        if FSEv07_isError(headerSize) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if tableLog > maxLog {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSEv07_buildDTable(DTable, norm.as_ptr(), max, tableLog);
        return headerSize;
    }
}

pub unsafe fn ZSTDv07_decodeSeqHeaders(
    nbSeqPtr: *mut c_int,
    DTableLL: *mut FSEv07_DTable,
    DTableML: *mut FSEv07_DTable,
    DTableOffb: *mut FSEv07_DTable,
    flagRepeatTable: U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let mut ip: *const BYTE = istart;

    /* check */
    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    {
        let mut nbSeq: c_int = *ip as c_int;
        ip = ip.wrapping_add(1);
        if nbSeq == 0 {
            *nbSeqPtr = 0;
            return 1;
        }
        if nbSeq > 0x7F {
            if nbSeq == 0xFF {
                if ip.wrapping_add(2) as usize > iend as usize {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = (MEM_readLE16(ip as *const c_void) as U32).wrapping_add(LONGNBSEQ) as c_int;
                ip = ip.wrapping_add(2);
            } else {
                if ip as usize >= iend as usize {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = ((nbSeq - 0x80) << 8) + (*ip as c_int);
                ip = ip.wrapping_add(1);
            }
        }
        *nbSeqPtr = nbSeq;
    }

    /* FSE table descriptors */
    if ip.wrapping_add(4) as usize > iend as usize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let LLtype: U32 = (*ip >> 6) as U32;
        let OFtype: U32 = ((*ip >> 4) & 3) as U32;
        let MLtype: U32 = ((*ip >> 2) & 3) as U32;
        ip = ip.wrapping_add(1);

        /* Build DTables */
        {
            let llhSize: usize = ZSTDv07_buildSeqTable(
                DTableLL,
                LLtype,
                MaxLL as U32,
                LLFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(llhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(llhSize);
        }
        {
            let ofhSize: usize = ZSTDv07_buildSeqTable(
                DTableOffb,
                OFtype,
                MaxOff as U32,
                OffFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(ofhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(ofhSize);
        }
        {
            let mlhSize: usize = ZSTDv07_buildSeqTable(
                DTableML,
                MLtype,
                MaxML as U32,
                MLFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(mlhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(mlhSize);
        }
    }

    (ip as usize).wrapping_sub(istart as usize)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct seq_t {
    pub litLength: usize,
    pub matchLength: usize,
    pub offset: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct seqState_t {
    pub DStream: BITv07_DStream_t,
    pub stateLL: FSEv07_DState_t,
    pub stateOffb: FSEv07_DState_t,
    pub stateML: FSEv07_DState_t,
    pub prevOffset: [usize; ZSTDv07_REP_INIT],
}

/* function-local `static const` tables of ZSTDv07_decodeSequence */
pub static LL_base: [U32; MaxLL + 1] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

pub static ML_base: [U32; MaxML + 1] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
    0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003,
];

pub static OF_base: [U32; MaxOff + 1] = [
    0, 1, 1, 5, 0xD, 0x1D, 0x3D, 0x7D, 0xFD, 0x1FD, 0x3FD, 0x7FD, 0xFFD, 0x1FFD, 0x3FFD, 0x7FFD,
    0xFFFD, 0x1FFFD, 0x3FFFD, 0x7FFFD, 0xFFFFD, 0x1FFFFD, 0x3FFFFD, 0x7FFFFD, 0xFFFFFD, 0x1FFFFFD,
    0x3FFFFFD, 0x7FFFFFD, 0xFFFFFFD,
];

pub unsafe fn ZSTDv07_decodeSequence(seqState: *mut seqState_t) -> seq_t {
    let mut seq = seq_t {
        litLength: 0,
        matchLength: 0,
        offset: 0,
    };

    let llCode: U32 = FSEv07_peekSymbol(core::ptr::addr_of!((*seqState).stateLL)) as U32;
    let mlCode: U32 = FSEv07_peekSymbol(core::ptr::addr_of!((*seqState).stateML)) as U32;
    let ofCode: U32 = FSEv07_peekSymbol(core::ptr::addr_of!((*seqState).stateOffb)) as U32; /* <= maxOff, by table construction */

    let llBits: U32 = LL_bits[llCode as usize];
    let mlBits: U32 = ML_bits[mlCode as usize];
    let ofBits: U32 = ofCode;
    let totalBits: U32 = llBits.wrapping_add(mlBits).wrapping_add(ofBits);

    /* sequence */
    {
        let mut offset: usize;
        if ofCode == 0 {
            offset = 0;
        } else {
            offset = (OF_base[ofCode as usize] as usize).wrapping_add(BITv07_readBits(
                core::ptr::addr_of_mut!((*seqState).DStream),
                ofBits,
            )); /* <=  (ZSTDv07_WINDOWLOG_MAX-1) bits */
            if MEM_32bits() != 0 {
                BITv07_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
            }
        }

        if ofCode <= 1 {
            if ((llCode == 0) as c_int & (offset <= 1) as c_int) != 0 {
                offset = 1usize.wrapping_sub(offset);
            }
            if offset != 0 {
                let temp: usize = (*seqState).prevOffset[offset];
                if offset != 1 {
                    (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
                }
                (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
                offset = temp;
                (*seqState).prevOffset[0] = offset;
            } else {
                offset = (*seqState).prevOffset[0];
            }
        } else {
            (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
            (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
            (*seqState).prevOffset[0] = offset;
        }
        seq.offset = offset;
    }

    seq.matchLength = (ML_base[mlCode as usize] as usize).wrapping_add(if mlCode > 31 {
        BITv07_readBits(core::ptr::addr_of_mut!((*seqState).DStream), mlBits)
    } else {
        0
    }); /* <=  16 bits */
    if MEM_32bits() != 0 && (mlBits.wrapping_add(llBits) > 24) {
        BITv07_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
    }

    seq.litLength = (LL_base[llCode as usize] as usize).wrapping_add(if llCode > 15 {
        BITv07_readBits(core::ptr::addr_of_mut!((*seqState).DStream), llBits)
    } else {
        0
    }); /* <=  16 bits */
    if MEM_32bits() != 0
        || (totalBits > (64 - 7 - (LLFSELog + MLFSELog + OffFSELog)))
    {
        BITv07_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
    }

    /* ANS state update */
    FSEv07_updateState(
        core::ptr::addr_of_mut!((*seqState).stateLL),
        core::ptr::addr_of_mut!((*seqState).DStream),
    ); /* <=  9 bits */
    FSEv07_updateState(
        core::ptr::addr_of_mut!((*seqState).stateML),
        core::ptr::addr_of_mut!((*seqState).DStream),
    ); /* <=  9 bits */
    if MEM_32bits() != 0 {
        BITv07_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
    } /* <= 18 bits */
    FSEv07_updateState(
        core::ptr::addr_of_mut!((*seqState).stateOffb),
        core::ptr::addr_of_mut!((*seqState).DStream),
    ); /* <=  8 bits */

    seq
}

/* function-local `static const` tables of ZSTDv07_execSequence */
pub static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
pub static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */

pub unsafe fn ZSTDv07_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let oLitEnd: *mut BYTE = op.wrapping_add(sequence.litLength);
    let sequenceLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd: *mut BYTE = op.wrapping_add(sequenceLength); /* risk : address space overflow (32-bits) */
    let oend_w: *mut BYTE = oend.wrapping_sub(WILDCOPY_OVERLENGTH);
    let iLitEnd: *const BYTE = (*litPtr).wrapping_add(sequence.litLength);
    let mut match_: *const BYTE = (oLitEnd as *const BYTE).wrapping_sub(sequence.offset);

    /* check */
    if sequence.litLength.wrapping_add(WILDCOPY_OVERLENGTH)
        > (oend as usize).wrapping_sub(op as usize)
    {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequenceLength > (oend as usize).wrapping_sub(op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize).wrapping_sub(*litPtr as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* copy Literals */
    ZSTDv07_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    ); /* note : since oLitEnd <= oend-WILDCOPY_OVERLENGTH, no risk of overwrite beyond oend */
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* copy Match */
    if sequence.offset > (oLitEnd as usize).wrapping_sub(base as usize) {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as usize).wrapping_sub(vBase as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_sub((base as usize).wrapping_sub(match_ as usize));
        if match_.wrapping_add(sequence.matchLength) as usize <= dictEnd as usize {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = (dictEnd as usize).wrapping_sub(match_ as usize);
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.wrapping_add(length1);
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            match_ = base;
            if op as usize > oend_w as usize || sequence.matchLength < MINMATCH {
                while (op as usize) < oMatchEnd as usize {
                    *op = *match_;
                    op = op.wrapping_add(1);
                    match_ = match_.wrapping_add(1);
                }
                return sequenceLength;
            }
        }
    }
    /* Requirement: op <= oend_w */

    /* match within prefix */
    if sequence.offset < 8 {
        /* close range match, overlap */
        let sub2: c_int = dec64table[sequence.offset];
        *op.wrapping_add(0) = *match_.wrapping_add(0);
        *op.wrapping_add(1) = *match_.wrapping_add(1);
        *op.wrapping_add(2) = *match_.wrapping_add(2);
        *op.wrapping_add(3) = *match_.wrapping_add(3);
        match_ = match_.wrapping_add(dec32table[sequence.offset] as usize);
        ZSTDv07_copy4(
            op.wrapping_add(4) as *mut c_void,
            match_ as *const c_void,
        );
        match_ = match_.wrapping_offset(-(sub2 as isize));
    } else {
        ZSTDv07_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.wrapping_add(8);
    match_ = match_.wrapping_add(8);

    if oMatchEnd as usize > oend.wrapping_sub(16 - MINMATCH) as usize {
        if (op as usize) < oend_w as usize {
            ZSTDv07_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                (oend_w as isize).wrapping_sub(op as isize),
            );
            match_ = match_.wrapping_add((oend_w as usize).wrapping_sub(op as usize));
            op = oend_w;
        }
        while (op as usize) < oMatchEnd as usize {
            *op = *match_;
            op = op.wrapping_add(1);
            match_ = match_.wrapping_add(1);
        }
    } else {
        ZSTDv07_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            (sequence.matchLength as isize).wrapping_sub(8),
        ); /* works even if matchLength < 8 */
    }
    sequenceLength
}

pub unsafe fn ZSTDv07_decompressSequences(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    mut seqSize: usize,
) -> usize {
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(seqSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.wrapping_add((*dctx).litSize);
    let DTableLL: *mut FSEv07_DTable =
        core::ptr::addr_of_mut!((*dctx).LLTable) as *mut FSEv07_DTable;
    let DTableML: *mut FSEv07_DTable =
        core::ptr::addr_of_mut!((*dctx).MLTable) as *mut FSEv07_DTable;
    let DTableOffb: *mut FSEv07_DTable =
        core::ptr::addr_of_mut!((*dctx).OffTable) as *mut FSEv07_DTable;
    let base: *const BYTE = (*dctx).base as *const BYTE;
    let vBase: *const BYTE = (*dctx).vBase as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: c_int = 0;

    /* Build Decoding Tables */
    {
        let seqHSize: usize = ZSTDv07_decodeSeqHeaders(
            &mut nbSeq,
            DTableLL,
            DTableML,
            DTableOffb,
            (*dctx).fseEntropy,
            ip as *const c_void,
            seqSize,
        );
        if ZSTDv07_isError(seqHSize) != 0 {
            return seqHSize;
        }
        ip = ip.wrapping_add(seqHSize);
    }

    /* Regen sequences */
    if nbSeq != 0 {
        let mut seqState = seqState_t {
            DStream: BITv07_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            },
            stateLL: FSEv07_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateOffb: FSEv07_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateML: FSEv07_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            prevOffset: [0; ZSTDv07_REP_INIT],
        };
        (*dctx).fseEntropy = 1;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTDv07_REP_INIT {
                seqState.prevOffset[i as usize] = (*dctx).rep[i as usize] as usize;
                i += 1;
            }
        }
        {
            let errorCode: usize = BITv07_initDStream(
                &mut seqState.DStream,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }
        FSEv07_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSEv07_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSEv07_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BITv07_reloadDStream(&mut seqState.DStream) <= BITv07_DStream_completed)
            && nbSeq != 0
        {
            nbSeq -= 1;
            {
                let sequence: seq_t = ZSTDv07_decodeSequence(&mut seqState);
                let oneSeqSize: usize = ZSTDv07_execSequence(
                    op,
                    oend,
                    sequence,
                    &mut litPtr,
                    litEnd,
                    base,
                    vBase,
                    dictEnd,
                );
                if ZSTDv07_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.wrapping_add(oneSeqSize);
            }
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* save reps for next block */
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTDv07_REP_INIT {
                (*dctx).rep[i as usize] = seqState.prevOffset[i as usize] as U32;
                i += 1;
            }
        }
    }

    /* last literal segment */
    {
        let lastLLSize: usize = (litEnd as usize).wrapping_sub(litPtr as usize);
        if lastLLSize > (oend as usize).wrapping_sub(op as usize) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if lastLLSize > 0 {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

pub unsafe fn ZSTDv07_checkContinuity(dctx: *mut ZSTDv07_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = ((dst as isize).wrapping_sub(
            ((*dctx).previousDstEnd as isize).wrapping_sub((*dctx).base as isize),
        )) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

pub unsafe fn ZSTDv07_decompressBlock_internal(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip: *const BYTE = src as *const BYTE;

    if srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals sub-block */
    {
        let litCSize: usize = ZSTDv07_decodeLiteralsBlock(dctx, src, srcSize);
        if ZSTDv07_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.wrapping_add(litCSize);
        srcSize -= litCSize;
    }
    ZSTDv07_decompressSequences(dctx, dst, dstCapacity, ip as *const c_void, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBlock(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let dSize: usize;
    ZSTDv07_checkContinuity(dctx, dst as *const c_void);
    dSize = ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
    (*dctx).previousDstEnd = (dst as *mut c_char).wrapping_add(dSize) as *const c_void;
    dSize
}

/** ZSTDv07_insertBlock() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_insertBlock(
    dctx: *mut ZSTDv07_DCtx,
    blockStart: *const c_void,
    blockSize: usize,
) -> usize {
    ZSTDv07_checkContinuity(dctx, blockStart);
    (*dctx).previousDstEnd =
        (blockStart as *const c_char).wrapping_add(blockSize) as *const c_void;
    blockSize
}

pub unsafe fn ZSTDv07_generateNxBytes(
    dst: *mut c_void,
    dstCapacity: usize,
    byte: BYTE,
    length: usize,
) -> usize {
    if length > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if length > 0 {
        memset(dst, byte as c_int, length);
    }
    length
}

/* ! ZSTDv07_decompressFrame() : `dctx` must be properly initialized */
pub unsafe fn ZSTDv07_decompressFrame(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let mut remainingSize: usize = srcSize;

    /* check */
    if srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Frame Header */
    {
        let frameHeaderSize: usize = ZSTDv07_frameHeaderSize(src, ZSTDv07_frameHeaderSize_min);
        if ZSTDv07_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv07_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if ZSTDv07_decodeFrameHeader(dctx, src, frameHeaderSize) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    /* Loop on each block */
    loop {
        let mut decodedSize: usize;
        let mut blockProperties = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let cBlockSize: usize = ZSTDv07_getcBlockSize(
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
            &mut blockProperties,
        );
        if ZSTDv07_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.wrapping_add(ZSTDv07_blockHeaderSize);
        remainingSize -= ZSTDv07_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        if blockProperties.blockType == bt_compressed {
            decodedSize = ZSTDv07_decompressBlock_internal(
                dctx,
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                ip as *const c_void,
                cBlockSize,
            );
        } else if blockProperties.blockType == bt_raw {
            decodedSize = ZSTDv07_copyRawBlock(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                ip as *const c_void,
                cBlockSize,
            );
        } else if blockProperties.blockType == bt_rle {
            decodedSize = ZSTDv07_generateNxBytes(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                *ip,
                blockProperties.origSize as usize,
            );
        } else if blockProperties.blockType == bt_end {
            /* end of frame */
            if remainingSize != 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            decodedSize = 0;
        } else {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
        if blockProperties.blockType == bt_end {
            break; /* bt_end */
        }

        if ZSTDv07_isError(decodedSize) != 0 {
            return decodedSize;
        }
        if (*dctx).fParams.checksumFlag != 0 {
            XXH64_update(
                core::ptr::addr_of_mut!((*dctx).xxhState),
                op as *const c_void,
                decodedSize,
            );
        }
        op = op.wrapping_add(decodedSize);
        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    (op as usize).wrapping_sub(ostart as usize)
}

/* ! ZSTDv07_decompress_usingPreparedDCtx() */
pub unsafe fn ZSTDv07_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv07_DCtx,
    refDCtx: *const ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv07_copyDCtx(dctx, refDCtx);
    ZSTDv07_checkContinuity(dctx, dst as *const c_void);
    ZSTDv07_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompress_usingDict(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTDv07_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv07_checkContinuity(dctx, dst as *const c_void);
    ZSTDv07_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressDCtx(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv07_decompress_usingDict(
        dctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        core::ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* ZSTDv07_HEAPMODE == 1 */
    let regenSize: usize;
    let dctx: *mut ZSTDv07_DCtx = ZSTDv07_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv07_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv07_freeDCtx(dctx);
    regenSize
}

/* ZSTD_errorFrameSizeInfoLegacy() :
assumes `cSize` and `dBound` are _not_ NULL */
pub unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut usize,
    dBound: *mut u64,
    ret: usize,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;

    /* check */
    if srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }

    /* Frame Header */
    {
        let frameHeaderSize: usize = ZSTDv07_frameHeaderSize(src, srcSize);
        if ZSTDv07_isError(frameHeaderSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, frameHeaderSize);
            return;
        }
        if MEM_readLE32(src) != ZSTDv07_MAGICNUMBER {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
            return;
        }
        if srcSize < frameHeaderSize + ZSTDv07_blockHeaderSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    /* Loop on each block */
    loop {
        let mut blockProperties = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let cBlockSize: usize =
            ZSTDv07_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv07_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTDv07_blockHeaderSize);
        remainingSize -= ZSTDv07_blockHeaderSize;

        if blockProperties.blockType == bt_end {
            break;
        }

        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = (ip as usize).wrapping_sub(src as usize);
    *dBound = (nbBlocks * ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) as u64;
}

/*_******************************
*  Streaming Decompression API
********************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_nextSrcSizeToDecompress(dctx: *mut ZSTDv07_DCtx) -> usize {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isSkipFrame(dctx: *mut ZSTDv07_DCtx) -> c_int {
    ((*dctx).stage == ZSTDds_skipFrame) as c_int
}

/** ZSTDv07_decompressContinue() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressContinue(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* Sanity check */
    if srcSize != (*dctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dstCapacity != 0 {
        ZSTDv07_checkContinuity(dctx, dst as *const c_void);
    }

    let mut stage: ZSTDv07_dStage = (*dctx).stage;

    if stage == ZSTDds_getFrameHeaderSize {
        if srcSize != ZSTDv07_frameHeaderSize_min {
            return ERROR(ZSTD_error_srcSize_wrong); /* impossible */
        }
        if (MEM_readLE32(src) & 0xFFFFFFF0u32) == ZSTDv07_MAGIC_SKIPPABLE_START {
            memcpy(
                core::ptr::addr_of_mut!((*dctx).headerBuffer) as *mut c_void,
                src,
                ZSTDv07_frameHeaderSize_min,
            );
            (*dctx).expected = ZSTDv07_skippableHeaderSize - ZSTDv07_frameHeaderSize_min; /* magic number + skippable frame length */
            (*dctx).stage = ZSTDds_decodeSkippableHeader;
            return 0;
        }
        (*dctx).headerSize = ZSTDv07_frameHeaderSize(src, ZSTDv07_frameHeaderSize_min);
        if ZSTDv07_isError((*dctx).headerSize) != 0 {
            return (*dctx).headerSize;
        }
        memcpy(
            core::ptr::addr_of_mut!((*dctx).headerBuffer) as *mut c_void,
            src,
            ZSTDv07_frameHeaderSize_min,
        );
        if (*dctx).headerSize > ZSTDv07_frameHeaderSize_min {
            (*dctx).expected = (*dctx).headerSize - ZSTDv07_frameHeaderSize_min;
            (*dctx).stage = ZSTDds_decodeFrameHeader;
            return 0;
        }
        (*dctx).expected = 0; /* not necessary to copy more */
        /* fall-through */
        stage = ZSTDds_decodeFrameHeader;
    }

    if stage == ZSTDds_decodeFrameHeader {
        let result: usize;
        memcpy(
            (core::ptr::addr_of_mut!((*dctx).headerBuffer) as *mut BYTE)
                .wrapping_add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
            src,
            (*dctx).expected,
        );
        result = ZSTDv07_decodeFrameHeader(
            dctx,
            core::ptr::addr_of!((*dctx).headerBuffer) as *const c_void,
            (*dctx).headerSize,
        );
        if ZSTDv07_isError(result) != 0 {
            return result;
        }
        (*dctx).expected = ZSTDv07_blockHeaderSize;
        (*dctx).stage = ZSTDds_decodeBlockHeader;
        return 0;
    }

    if stage == ZSTDds_decodeBlockHeader {
        let mut bp = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let cBlockSize: usize = ZSTDv07_getcBlockSize(src, ZSTDv07_blockHeaderSize, &mut bp);
        if ZSTDv07_isError(cBlockSize) != 0 {
            return cBlockSize;
        }
        if bp.blockType == bt_end {
            if (*dctx).fParams.checksumFlag != 0 {
                let h64: U64 = XXH64_digest(core::ptr::addr_of!((*dctx).xxhState));
                let h32: U32 = ((h64 >> 11) as U32) & ((1u32 << 22) - 1);
                let ip: *const BYTE = src as *const BYTE;
                let check32: U32 = (*ip.wrapping_add(2) as U32)
                    .wrapping_add((*ip.wrapping_add(1) as U32) << 8)
                    .wrapping_add(((*ip.wrapping_add(0) & 0x3F) as U32) << 16);
                if check32 != h32 {
                    return ERROR(ZSTD_error_checksum_wrong);
                }
            }
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
        } else {
            (*dctx).expected = cBlockSize;
            (*dctx).bType = bp.blockType;
            (*dctx).stage = ZSTDds_decompressBlock;
        }
        return 0;
    }

    if stage == ZSTDds_decompressBlock {
        let rSize: usize;
        if (*dctx).bType == bt_compressed {
            rSize = ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
        } else if (*dctx).bType == bt_raw {
            rSize = ZSTDv07_copyRawBlock(dst, dstCapacity, src, srcSize);
        } else if (*dctx).bType == bt_rle {
            return ERROR(ZSTD_error_GENERIC); /* not yet handled */
        } else if (*dctx).bType == bt_end {
            /* should never happen (filtered at phase 1) */
            rSize = 0;
        } else {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
        (*dctx).stage = ZSTDds_decodeBlockHeader;
        (*dctx).expected = ZSTDv07_blockHeaderSize;
        if ZSTDv07_isError(rSize) != 0 {
            return rSize;
        }
        (*dctx).previousDstEnd = (dst as *mut c_char).wrapping_add(rSize) as *const c_void;
        if (*dctx).fParams.checksumFlag != 0 {
            XXH64_update(
                core::ptr::addr_of_mut!((*dctx).xxhState),
                dst as *const c_void,
                rSize,
            );
        }
        return rSize;
    }

    if stage == ZSTDds_decodeSkippableHeader {
        memcpy(
            (core::ptr::addr_of_mut!((*dctx).headerBuffer) as *mut BYTE)
                .wrapping_add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
            src,
            (*dctx).expected,
        );
        (*dctx).expected = MEM_readLE32(
            (core::ptr::addr_of!((*dctx).headerBuffer) as *const BYTE).wrapping_add(4)
                as *const c_void,
        ) as usize;
        (*dctx).stage = ZSTDds_skipFrame;
        return 0;
    }

    if stage == ZSTDds_skipFrame {
        (*dctx).expected = 0;
        (*dctx).stage = ZSTDds_getFrameHeaderSize;
        return 0;
    }

    ERROR(ZSTD_error_GENERIC) /* impossible */
}

pub unsafe fn ZSTDv07_refDictContent(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = ((dict as isize).wrapping_sub(
        ((*dctx).previousDstEnd as isize).wrapping_sub((*dctx).base as isize),
    )) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const c_char).wrapping_add(dictSize) as *const c_void;
    0
}

pub unsafe fn ZSTDv07_loadEntropy(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut dictPtr: *const BYTE = dict as *const BYTE;
    let dictEnd: *const BYTE = dictPtr.wrapping_add(dictSize);

    {
        let hSize: usize = HUFv07_readDTableX4(
            core::ptr::addr_of_mut!((*dctx).hufTable) as *mut HUFv07_DTable,
            dict,
            dictSize,
        );
        if HUFv07_isError(hSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dictPtr = dictPtr.wrapping_add(hSize);
    }

    {
        let mut offcodeNCount: [i16; MaxOff + 1] = [0; MaxOff + 1];
        let mut offcodeMaxValue: c_uint = MaxOff as c_uint;
        let mut offcodeLog: c_uint = 0;
        let offcodeHeaderSize: usize = FSEv07_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
        );
        if FSEv07_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode: usize = FSEv07_buildDTable(
                core::ptr::addr_of_mut!((*dctx).OffTable) as *mut FSEv07_DTable,
                offcodeNCount.as_ptr(),
                offcodeMaxValue,
                offcodeLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dictPtr = dictPtr.wrapping_add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount: [i16; MaxML + 1] = [0; MaxML + 1];
        let mut matchlengthMaxValue: c_uint = MaxML as c_uint;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize: usize = FSEv07_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
        );
        if FSEv07_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode: usize = FSEv07_buildDTable(
                core::ptr::addr_of_mut!((*dctx).MLTable) as *mut FSEv07_DTable,
                matchlengthNCount.as_ptr(),
                matchlengthMaxValue,
                matchlengthLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dictPtr = dictPtr.wrapping_add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount: [i16; MaxLL + 1] = [0; MaxLL + 1];
        let mut litlengthMaxValue: c_uint = MaxLL as c_uint;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize: usize = FSEv07_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
        );
        if FSEv07_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode: usize = FSEv07_buildDTable(
                core::ptr::addr_of_mut!((*dctx).LLTable) as *mut FSEv07_DTable,
                litlengthNCount.as_ptr(),
                litlengthMaxValue,
                litlengthLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dictPtr = dictPtr.wrapping_add(litlengthHeaderSize);
    }

    if dictPtr.wrapping_add(12) as usize > dictEnd as usize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[0] = MEM_readLE32(dictPtr.wrapping_add(0) as *const c_void);
    if (*dctx).rep[0] == 0 || (*dctx).rep[0] as usize >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[1] = MEM_readLE32(dictPtr.wrapping_add(4) as *const c_void);
    if (*dctx).rep[1] == 0 || (*dctx).rep[1] as usize >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[2] = MEM_readLE32(dictPtr.wrapping_add(8) as *const c_void);
    if (*dctx).rep[2] == 0 || (*dctx).rep[2] as usize >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dictPtr = dictPtr.wrapping_add(12);

    (*dctx).fseEntropy = 1;
    (*dctx).litEntropy = (*dctx).fseEntropy;
    (dictPtr as usize).wrapping_sub(dict as usize)
}

pub unsafe fn ZSTDv07_decompress_insertDictionary(
    dctx: *mut ZSTDv07_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    if dictSize < 8 {
        return ZSTDv07_refDictContent(dctx, dict, dictSize);
    }
    {
        let magic: U32 = MEM_readLE32(dict);
        if magic != ZSTDv07_DICT_MAGIC {
            return ZSTDv07_refDictContent(dctx, dict, dictSize); /* pure content mode */
        }
    }
    (*dctx).dictID = MEM_readLE32((dict as *const c_char).wrapping_add(4) as *const c_void);

    /* load entropy tables */
    dict = (dict as *const c_char).wrapping_add(8) as *const c_void;
    dictSize -= 8;
    {
        let eSize: usize = ZSTDv07_loadEntropy(dctx, dict, dictSize);
        if ZSTDv07_isError(eSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dict = (dict as *const c_char).wrapping_add(eSize) as *const c_void;
        dictSize -= eSize;
    }

    /* reference dictionary content */
    ZSTDv07_refDictContent(dctx, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBegin_usingDict(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    {
        let errorCode: usize = ZSTDv07_decompressBegin(dctx);
        if ZSTDv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    if !dict.is_null() && dictSize != 0 {
        let errorCode: usize = ZSTDv07_decompress_insertDictionary(dctx, dict, dictSize);
        if ZSTDv07_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }

    0
}

#[repr(C)]
pub struct ZSTDv07_DDict {
    pub dict: *mut c_void,
    pub dictSize: usize,
    pub refContext: *mut ZSTDv07_DCtx,
}

pub unsafe fn ZSTDv07_createDDict_advanced(
    dict: *const c_void,
    dictSize: usize,
    mut customMem: ZSTDv07_customMem,
) -> *mut ZSTDv07_DDict {
    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem;
    }

    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    {
        let ddict: *mut ZSTDv07_DDict = (customMem.customAlloc.unwrap())(
            customMem.opaque,
            core::mem::size_of::<ZSTDv07_DDict>(),
        ) as *mut ZSTDv07_DDict;
        let dictContent: *mut c_void =
            (customMem.customAlloc.unwrap())(customMem.opaque, dictSize);
        let dctx: *mut ZSTDv07_DCtx = ZSTDv07_createDCtx_advanced(customMem);

        if dictContent.is_null() || ddict.is_null() || dctx.is_null() {
            (customMem.customFree.unwrap())(customMem.opaque, dictContent);
            (customMem.customFree.unwrap())(customMem.opaque, ddict as *mut c_void);
            (customMem.customFree.unwrap())(customMem.opaque, dctx as *mut c_void);
            return core::ptr::null_mut();
        }

        memcpy(dictContent, dict, dictSize);
        {
            let errorCode: usize =
                ZSTDv07_decompressBegin_usingDict(dctx, dictContent as *const c_void, dictSize);
            if ZSTDv07_isError(errorCode) != 0 {
                (customMem.customFree.unwrap())(customMem.opaque, dictContent);
                (customMem.customFree.unwrap())(customMem.opaque, ddict as *mut c_void);
                (customMem.customFree.unwrap())(customMem.opaque, dctx as *mut c_void);
                return core::ptr::null_mut();
            }
        }

        (*ddict).dict = dictContent;
        (*ddict).dictSize = dictSize;
        (*ddict).refContext = dctx;
        return ddict;
    }
}

/* ! ZSTDv07_createDDict() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDDict(
    dict: *const c_void,
    dictSize: usize,
) -> *mut ZSTDv07_DDict {
    let allocator = ZSTDv07_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTDv07_createDDict_advanced(dict, dictSize, allocator)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_freeDDict(ddict: *mut ZSTDv07_DDict) -> usize {
    let cFree: ZSTDv07_freeFunction = (*(*ddict).refContext).customMem.customFree;
    let opaque: *mut c_void = (*(*ddict).refContext).customMem.opaque;
    ZSTDv07_freeDCtx((*ddict).refContext);
    (cFree.unwrap())(opaque, (*ddict).dict);
    (cFree.unwrap())(opaque, ddict as *mut c_void);
    0
}

/* ! ZSTDv07_decompress_usingDDict() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompress_usingDDict(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    ddict: *const ZSTDv07_DDict,
) -> usize {
    ZSTDv07_decompress_usingPreparedDCtx(
        dctx,
        (*ddict).refContext,
        dst,
        dstCapacity,
        src,
        srcSize,
    )
}

/*
    Buffered version of Zstd compression library
*/

pub type ZBUFFv07_dStage = c_int;
pub const ZBUFFds_init: ZBUFFv07_dStage = 0;
pub const ZBUFFds_loadHeader: ZBUFFv07_dStage = 1;
pub const ZBUFFds_read: ZBUFFv07_dStage = 2;
pub const ZBUFFds_load: ZBUFFv07_dStage = 3;
pub const ZBUFFds_flush: ZBUFFv07_dStage = 4;

/* *** Resource management *** */
#[repr(C)]
pub struct ZBUFFv07_DCtx {
    pub zd: *mut ZSTDv07_DCtx,
    pub fParams: ZSTDv07_frameParams,
    pub stage: ZBUFFv07_dStage,
    pub inBuff: *mut c_char,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub outBuff: *mut c_char,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub blockSize: usize,
    pub headerBuffer: [BYTE; ZSTDv07_FRAMEHEADERSIZE_MAX],
    pub lhSize: usize,
    pub customMem: ZSTDv07_customMem,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_createDCtx() -> *mut ZBUFFv07_DCtx {
    ZBUFFv07_createDCtx_advanced(defaultCustomMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_createDCtx_advanced(
    mut customMem: ZSTDv07_customMem,
) -> *mut ZBUFFv07_DCtx {
    let zbd: *mut ZBUFFv07_DCtx;

    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem;
    }

    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    zbd = (customMem.customAlloc.unwrap())(
        customMem.opaque,
        core::mem::size_of::<ZBUFFv07_DCtx>(),
    ) as *mut ZBUFFv07_DCtx;
    if zbd.is_null() {
        return core::ptr::null_mut();
    }
    memset(
        zbd as *mut c_void,
        0,
        core::mem::size_of::<ZBUFFv07_DCtx>(),
    );
    memcpy(
        core::ptr::addr_of_mut!((*zbd).customMem) as *mut c_void,
        &customMem as *const ZSTDv07_customMem as *const c_void,
        core::mem::size_of::<ZSTDv07_customMem>(),
    );
    (*zbd).zd = ZSTDv07_createDCtx_advanced(customMem);
    if (*zbd).zd.is_null() {
        ZBUFFv07_freeDCtx(zbd);
        return core::ptr::null_mut();
    }
    (*zbd).stage = ZBUFFds_init;
    zbd
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_freeDCtx(zbd: *mut ZBUFFv07_DCtx) -> usize {
    if zbd.is_null() {
        return 0; /* support free on null */
    }
    ZSTDv07_freeDCtx((*zbd).zd);
    if !(*zbd).inBuff.is_null() {
        ((*zbd).customMem.customFree.unwrap())(
            (*zbd).customMem.opaque,
            (*zbd).inBuff as *mut c_void,
        );
    }
    if !(*zbd).outBuff.is_null() {
        ((*zbd).customMem.customFree.unwrap())(
            (*zbd).customMem.opaque,
            (*zbd).outBuff as *mut c_void,
        );
    }
    ((*zbd).customMem.customFree.unwrap())((*zbd).customMem.opaque, zbd as *mut c_void);
    0
}

/* *** Initialization *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInitDictionary(
    zbd: *mut ZBUFFv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*zbd).stage = ZBUFFds_loadHeader;
    (*zbd).outEnd = 0;
    (*zbd).outStart = (*zbd).outEnd;
    (*zbd).inPos = (*zbd).outStart;
    (*zbd).lhSize = (*zbd).inPos;
    ZSTDv07_decompressBegin_usingDict((*zbd).zd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInit(zbd: *mut ZBUFFv07_DCtx) -> usize {
    ZBUFFv07_decompressInitDictionary(zbd, core::ptr::null(), 0)
}

/* internal util function */
#[inline(always)]
pub unsafe fn ZBUFFv07_limitCopy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length: usize = MIN_usize(dstCapacity, srcSize);
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

/* *** Decompression *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressContinue(
    zbd: *mut ZBUFFv07_DCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart: *const c_char = src as *const c_char;
    let iend: *const c_char = istart.wrapping_add(*srcSizePtr);
    let mut ip: *const c_char = istart;
    let ostart: *mut c_char = dst as *mut c_char;
    let oend: *mut c_char = ostart.wrapping_add(*dstCapacityPtr);
    let mut op: *mut c_char = ostart;
    let mut notDone: U32 = 1;

    while notDone != 0 {
        let mut st: ZBUFFv07_dStage = (*zbd).stage;
        if st != ZBUFFds_init
            && st != ZBUFFds_loadHeader
            && st != ZBUFFds_read
            && st != ZBUFFds_load
            && st != ZBUFFds_flush
        {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
        /* `break 'sw` emulates C's `break` out of the switch statement */
        'sw: loop {
            if st == ZBUFFds_init {
                return ERROR(ZSTD_error_init_missing);
            }

            if st == ZBUFFds_loadHeader {
                {
                    let hSize: usize = ZSTDv07_getFrameParams(
                        core::ptr::addr_of_mut!((*zbd).fParams),
                        core::ptr::addr_of!((*zbd).headerBuffer) as *const c_void,
                        (*zbd).lhSize,
                    );
                    if ZSTDv07_isError(hSize) != 0 {
                        return hSize;
                    }
                    if hSize != 0 {
                        let toLoad: usize = hSize.wrapping_sub((*zbd).lhSize); /* if hSize!=0, hSize > zbd->lhSize */
                        if toLoad > (iend as usize).wrapping_sub(ip as usize) {
                            /* not enough input to load full header */
                            if !ip.is_null() {
                                memcpy(
                                    (core::ptr::addr_of_mut!((*zbd).headerBuffer) as *mut BYTE)
                                        .wrapping_add((*zbd).lhSize) as *mut c_void,
                                    ip as *const c_void,
                                    (iend as usize).wrapping_sub(ip as usize),
                                );
                            }
                            (*zbd).lhSize = (*zbd)
                                .lhSize
                                .wrapping_add((iend as usize).wrapping_sub(ip as usize));
                            *dstCapacityPtr = 0;
                            return hSize
                                .wrapping_sub((*zbd).lhSize)
                                .wrapping_add(ZSTDv07_blockHeaderSize); /* remaining header bytes + next block header */
                        }
                        memcpy(
                            (core::ptr::addr_of_mut!((*zbd).headerBuffer) as *mut BYTE)
                                .wrapping_add((*zbd).lhSize) as *mut c_void,
                            ip as *const c_void,
                            toLoad,
                        );
                        (*zbd).lhSize = hSize;
                        ip = ip.wrapping_add(toLoad);
                        break 'sw;
                    }
                }

                /* Consume header */
                {
                    let h1Size: usize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd); /* == ZSTDv07_frameHeaderSize_min */
                    let h1Result: usize = ZSTDv07_decompressContinue(
                        (*zbd).zd,
                        core::ptr::null_mut(),
                        0,
                        core::ptr::addr_of!((*zbd).headerBuffer) as *const c_void,
                        h1Size,
                    );
                    if ZSTDv07_isError(h1Result) != 0 {
                        return h1Result;
                    }
                    if h1Size < (*zbd).lhSize {
                        /* long header */
                        let h2Size: usize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                        let h2Result: usize = ZSTDv07_decompressContinue(
                            (*zbd).zd,
                            core::ptr::null_mut(),
                            0,
                            (core::ptr::addr_of!((*zbd).headerBuffer) as *const BYTE)
                                .wrapping_add(h1Size) as *const c_void,
                            h2Size,
                        );
                        if ZSTDv07_isError(h2Result) != 0 {
                            return h2Result;
                        }
                    }
                }

                (*zbd).fParams.windowSize = MAX_u32(
                    (*zbd).fParams.windowSize,
                    1u32 << ZSTDv07_WINDOWLOG_ABSOLUTEMIN,
                );

                /* Frame header instruct buffer sizes */
                {
                    let blockSize: usize = MIN_usize(
                        (*zbd).fParams.windowSize as usize,
                        ZSTDv07_BLOCKSIZE_ABSOLUTEMAX,
                    );
                    (*zbd).blockSize = blockSize;
                    if (*zbd).inBuffSize < blockSize {
                        ((*zbd).customMem.customFree.unwrap())(
                            (*zbd).customMem.opaque,
                            (*zbd).inBuff as *mut c_void,
                        );
                        (*zbd).inBuffSize = blockSize;
                        (*zbd).inBuff = ((*zbd).customMem.customAlloc.unwrap())(
                            (*zbd).customMem.opaque,
                            blockSize,
                        ) as *mut c_char;
                        if (*zbd).inBuff.is_null() {
                            return ERROR(ZSTD_error_memory_allocation);
                        }
                    }
                    {
                        let neededOutSize: usize = ((*zbd).fParams.windowSize as usize)
                            .wrapping_add(blockSize)
                            .wrapping_add(WILDCOPY_OVERLENGTH * 2);
                        if (*zbd).outBuffSize < neededOutSize {
                            ((*zbd).customMem.customFree.unwrap())(
                                (*zbd).customMem.opaque,
                                (*zbd).outBuff as *mut c_void,
                            );
                            (*zbd).outBuffSize = neededOutSize;
                            (*zbd).outBuff = ((*zbd).customMem.customAlloc.unwrap())(
                                (*zbd).customMem.opaque,
                                neededOutSize,
                            ) as *mut c_char;
                            if (*zbd).outBuff.is_null() {
                                return ERROR(ZSTD_error_memory_allocation);
                            }
                        }
                    }
                }
                (*zbd).stage = ZBUFFds_read;
                /* pass-through */
                /* fall-through */
                st = ZBUFFds_read;
            }

            if st == ZBUFFds_read {
                {
                    let neededInSize: usize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                    if neededInSize == 0 {
                        /* end of frame */
                        (*zbd).stage = ZBUFFds_init;
                        notDone = 0;
                        break 'sw;
                    }
                    if (iend as usize).wrapping_sub(ip as usize) >= neededInSize {
                        /* decode directly from src */
                        let isSkipFrame: c_int = ZSTDv07_isSkipFrame((*zbd).zd);
                        let decodedSize: usize = ZSTDv07_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.wrapping_add((*zbd).outStart) as *mut c_void,
                            if isSkipFrame != 0 {
                                0
                            } else {
                                (*zbd).outBuffSize.wrapping_sub((*zbd).outStart)
                            },
                            ip as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv07_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        ip = ip.wrapping_add(neededInSize);
                        if decodedSize == 0 && isSkipFrame == 0 {
                            break 'sw; /* this was just a header */
                        }
                        (*zbd).outEnd = (*zbd).outStart.wrapping_add(decodedSize);
                        (*zbd).stage = ZBUFFds_flush;
                        break 'sw;
                    }
                    if ip == iend {
                        notDone = 0;
                        break 'sw;
                    } /* no more input */
                    (*zbd).stage = ZBUFFds_load;
                }
                /* fall-through */
                st = ZBUFFds_load;
            }

            if st == ZBUFFds_load {
                {
                    let neededInSize: usize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                    let toLoad: usize = neededInSize.wrapping_sub((*zbd).inPos); /* should always be <= remaining space within inBuff */
                    let loadedSize: usize;
                    if toLoad > (*zbd).inBuffSize.wrapping_sub((*zbd).inPos) {
                        return ERROR(ZSTD_error_corruption_detected); /* should never happen */
                    }
                    loadedSize = ZBUFFv07_limitCopy(
                        (*zbd).inBuff.wrapping_add((*zbd).inPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        (iend as usize).wrapping_sub(ip as usize),
                    );
                    ip = ip.wrapping_add(loadedSize);
                    (*zbd).inPos = (*zbd).inPos.wrapping_add(loadedSize);
                    if loadedSize < toLoad {
                        notDone = 0;
                        break 'sw;
                    } /* not enough input, wait for more */

                    /* decode loaded input */
                    {
                        let isSkipFrame: c_int = ZSTDv07_isSkipFrame((*zbd).zd);
                        let decodedSize: usize = ZSTDv07_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.wrapping_add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize.wrapping_sub((*zbd).outStart),
                            (*zbd).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv07_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        (*zbd).inPos = 0; /* input is consumed */
                        if decodedSize == 0 && isSkipFrame == 0 {
                            (*zbd).stage = ZBUFFds_read;
                            break 'sw;
                        } /* this was just a header */
                        (*zbd).outEnd = (*zbd).outStart.wrapping_add(decodedSize);
                        (*zbd).stage = ZBUFFds_flush;
                        /* break; */
                        /* pass-through */
                    }
                }
                /* fall-through */
                st = ZBUFFds_flush;
            }

            if st == ZBUFFds_flush {
                {
                    let toFlushSize: usize = (*zbd).outEnd.wrapping_sub((*zbd).outStart);
                    let flushedSize: usize = ZBUFFv07_limitCopy(
                        op as *mut c_void,
                        (oend as usize).wrapping_sub(op as usize),
                        (*zbd).outBuff.wrapping_add((*zbd).outStart) as *const c_void,
                        toFlushSize,
                    );
                    op = op.wrapping_add(flushedSize);
                    (*zbd).outStart = (*zbd).outStart.wrapping_add(flushedSize);
                    if flushedSize == toFlushSize {
                        (*zbd).stage = ZBUFFds_read;
                        if (*zbd).outStart.wrapping_add((*zbd).blockSize) > (*zbd).outBuffSize {
                            (*zbd).outEnd = 0;
                            (*zbd).outStart = (*zbd).outEnd;
                        }
                        break 'sw;
                    }
                    /* cannot flush everything */
                    notDone = 0;
                    break 'sw;
                }
            }

            break 'sw;
        }
    }

    /* result */
    *srcSizePtr = (ip as usize).wrapping_sub(istart as usize);
    *dstCapacityPtr = (op as usize).wrapping_sub(ostart as usize);
    {
        let mut nextSrcSizeHint: usize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbd).inPos); /* already loaded*/
        return nextSrcSizeHint;
    }
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv07_recommendedDInSize() -> usize {
    ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + ZSTDv07_blockHeaderSize /* block header size*/
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv07_recommendedDOutSize() -> usize {
    ZSTDv07_BLOCKSIZE_ABSOLUTEMAX
}
