//! Transliteration of the FIRST half of `legacy/zstd_v05.c` (C lines 1 - 2493):
//! the bundled `mem.h`, `zstd_internal.h` (ZSTD_CCOMMON_H_MODULE), `fse.h` /
//! `bitstream.h` / `fse_static.h` / `fse.c` (FSEv05) and `huff0.h` /
//! `huff0_static.h` / `huff0.c` (HUFv05) sections.
//!
//! The `ZSTDv05_*` decompressor proper (C lines 2494 - 4005) lives in the
//! sibling module.
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens,
    unused_imports,
    unused_macros,
    unused_labels
)]

use crate::error_private::*;
use crate::mem::{calloc, free, malloc, memcpy, memmove, memset, qsort};

use core::ffi::{c_char, c_int, c_uint, c_void};

/* ******************************************************************
 *  mem.h -- Basic Types
 *******************************************************************/
pub type BYTE = u8;
pub type U8 = u8;
pub type S8 = i8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

/*-**************************************************************
 *  mem.h -- Memory I/O
 *****************************************************************/

#[inline(always)]
pub fn MEM_32bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 4) as c_uint
}

#[inline(always)]
pub fn MEM_64bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 8) as c_uint
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
pub unsafe fn MEM_write32(memPtr: *mut c_void, value: U32) {
    core::ptr::write_unaligned(memPtr as *mut U32, value)
}

#[inline(always)]
pub unsafe fn MEM_write64(memPtr: *mut c_void, value: U64) {
    core::ptr::write_unaligned(memPtr as *mut U64, value)
}

#[inline(always)]
pub unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        MEM_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        ((*p.add(0) as U32) + ((*p.add(1) as U32) << 8)) as U16
    }
}

#[inline(always)]
pub unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let p = memPtr as *mut BYTE;
        *p.add(0) = val as BYTE;
        *p.add(1) = (val >> 8) as BYTE;
    }
}

#[inline(always)]
pub unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        MEM_read32(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U32)
            + ((*p.add(1) as U32) << 8)
            + ((*p.add(2) as U32) << 16)
            + ((*p.add(3) as U32) << 24)
    }
}

#[inline(always)]
pub unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        MEM_read64(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U64)
            + ((*p.add(1) as U64) << 8)
            + ((*p.add(2) as U64) << 16)
            + ((*p.add(3) as U64) << 24)
            + ((*p.add(4) as U64) << 32)
            + ((*p.add(5) as U64) << 40)
            + ((*p.add(6) as U64) << 48)
            + ((*p.add(7) as U64) << 56)
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

/*-*************************************
 *  zstd_v05_static.h -- Types
 ***************************************/
pub const ZSTDv05_WINDOWLOG_ABSOLUTEMIN: U32 = 11;

/*-*************************************
 *  zstd_internal.h -- Common constants
 ***************************************/
pub const ZSTDv05_DICT_MAGIC: U32 = 0xEC30A435;

pub const BLOCKSIZE: usize = 128 * (1 << 10); /* define, for static allocation */

pub const ZSTDv05_blockHeaderSize: usize = 3;
pub const ZSTDv05_frameHeaderSize_min: usize = 5;
pub const ZSTDv05_frameHeaderSize_max: usize = 5; /* define, for static allocation */

pub const BITv057: U32 = 128;
pub const BITv056: U32 = 64;
pub const BITv055: U32 = 32;
pub const BITv054: U32 = 16;
pub const BITv051: U32 = 2;
pub const BITv050: U32 = 1;

pub const IS_HUFv05: U32 = 0;
pub const IS_PCH: U32 = 1;
pub const IS_RAW: U32 = 2;
pub const IS_RLE: U32 = 3;

pub const MINMATCH: U32 = 4;
pub const REPCODE_STARTVALUE: U32 = 1;

pub const Litbits: U32 = 8;
pub const MLbits: U32 = 7;
pub const LLbits: U32 = 6;
pub const Offbits: U32 = 5;
pub const MaxLit: U32 = (1 << Litbits) - 1;
pub const MaxML: U32 = (1 << MLbits) - 1;
pub const MaxLL: U32 = (1 << LLbits) - 1;
pub const MaxOff: U32 = (1 << Offbits) - 1;
pub const MLFSEv05Log: U32 = 10;
pub const LLFSEv05Log: U32 = 10;
pub const OffFSEv05Log: U32 = 9;
pub const MaxSeq: U32 = if MaxLL > MaxML { MaxLL } else { MaxML };

pub const FSEv05_ENCODING_RAW: U32 = 0;
pub const FSEv05_ENCODING_RLE: U32 = 1;
pub const FSEv05_ENCODING_STATIC: U32 = 2;
pub const FSEv05_ENCODING_DYNAMIC: U32 = 3;

pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: U32 = 12;

pub const MIN_SEQUENCES_SIZE: usize = 1; /* nbSeq==0 */
pub const MIN_CBLOCK_SIZE: usize = 1 /*litCSize*/ + 1 /* RLE or RAW */ + MIN_SEQUENCES_SIZE;

pub const WILDCOPY_OVERLENGTH: usize = 8;

pub const ZSTD_CONTENTSIZE_ERROR: U64 = u64::MAX - 1; /* (0ULL - 2) */

pub type blockType_t = c_int;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

/*-*******************************************
 *  Shared functions to include for inlining
 *********************************************/
pub unsafe fn ZSTDv05_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

/* ! ZSTDv05_wildcopy() :
 *   custom version of memcpy(), can copy up to 7 bytes too many
 *   (8 bytes if length==0) */
pub unsafe fn ZSTDv05_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_offset(length);
    loop {
        /* COPY8(op, ip) */
        ZSTDv05_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
        if !(op < oend) {
            break;
        }
    }
}

/*-*******************************************
 *  Private interfaces
 *********************************************/
#[repr(C)]
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
    /* opt */
    pub matchLengthFreq: *mut U32,
    pub litLengthFreq: *mut U32,
    pub litFreq: *mut U32,
    pub offCodeFreq: *mut U32,
    pub matchLengthSum: U32,
    pub litLengthSum: U32,
    pub litSum: U32,
    pub offCodeSum: U32,
}

/* ******************************************************************
 *  FSEv05 -- public types / static allocation
 *******************************************************************/
pub type FSEv05_DTable = c_uint;

pub const fn FSEv05_DTABLE_SIZE_U32(maxTableLog: U32) -> usize {
    1 + (1usize << maxTableLog)
}

/* ******************************************************************
 *  bitstream -- decoding API (read backward)
 *******************************************************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct BITv05_DStream_t {
    pub bitContainer: usize,
    pub bitsConsumed: c_uint,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

pub type BITv05_DStream_status = c_uint;
pub const BITv05_DStream_unfinished: BITv05_DStream_status = 0;
pub const BITv05_DStream_endOfBuffer: BITv05_DStream_status = 1;
pub const BITv05_DStream_completed: BITv05_DStream_status = 2;
pub const BITv05_DStream_overflow: BITv05_DStream_status = 3;

/*-**************************************************************
 *  Helper functions
 ****************************************************************/
pub fn BITv05_highbit32(val: U32) -> c_uint {
    /* __builtin_clz (val) ^ 31 */
    (val.leading_zeros() ^ 31) as c_uint
}

/*-********************************************************
 * bitStream decoding
 **********************************************************/
/* !BITv05_initDStream
 *  Initialize a BITv05_DStream_t.
 */
pub unsafe fn BITv05_initDStream(
    bitD: *mut BITv05_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BITv05_DStream_t>());
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
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BITv05_highbit32(contain32));
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        /* switch(srcSize) with fall-through, srcSize is 1..7 here */
        let sp = (*bitD).start as *const BYTE;
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*sp.add(6) as usize) << (core::mem::size_of::<usize>() * 8 - 16),
            );
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*sp.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24),
            );
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*sp.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32),
            );
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sp.add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sp.add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sp.add(1) as usize) << 8);
        }
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); /* endMark not present */
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BITv05_highbit32(contain32));
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(
            ((core::mem::size_of::<usize>().wrapping_sub(srcSize)) as U32).wrapping_mul(8),
        );
    }

    srcSize
}

pub unsafe fn BITv05_lookBits(bitD: *mut BITv05_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask.wrapping_sub(nbBits)) & bitMask)
}

/* ! BITv05_lookBitsFast :
 *   unsafe version; only works if nbBits >= 1 */
pub unsafe fn BITv05_lookBitsFast(bitD: *mut BITv05_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((bitMask.wrapping_add(1).wrapping_sub(nbBits)) & bitMask)
}

pub unsafe fn BITv05_skipBits(bitD: *mut BITv05_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

pub unsafe fn BITv05_readBits(bitD: *mut BITv05_DStream_t, nbBits: c_uint) -> usize {
    let value = BITv05_lookBits(bitD, nbBits);
    BITv05_skipBits(bitD, nbBits);
    value
}

/* !BITv05_readBitsFast :
 *  unsafe version; only works if nbBits >= 1 */
pub unsafe fn BITv05_readBitsFast(bitD: *mut BITv05_DStream_t, nbBits: c_uint) -> usize {
    let value = BITv05_lookBitsFast(bitD, nbBits);
    BITv05_skipBits(bitD, nbBits);
    value
}

pub unsafe fn BITv05_reloadDStream(bitD: *mut BITv05_DStream_t) -> BITv05_DStream_status {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as c_uint {
        /* should never happen */
        return BITv05_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BITv05_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as c_uint {
            return BITv05_DStream_endOfBuffer;
        }
        return BITv05_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv05_DStream_status = BITv05_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize).wrapping_sub((*bitD).start as usize) as U32; /* ptr > start */
            result = BITv05_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return result;
    }
}

/* ! BITv05_endOfDStream
 *   @return Tells if DStream has reached its exact end
 */
pub unsafe fn BITv05_endOfDStream(DStream: *const BITv05_DStream_t) -> c_uint {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as c_uint))
        as c_uint
}

/* ******************************************************************
 *  FSEv05 -- symbol decompression API (inlined)
 *******************************************************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv05_DState_t {
    pub state: usize,
    pub table: *const c_void, /* precise table may vary, depending on U16 */
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv05_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
} /* sizeof U32 */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv05_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
} /* size == U32 */

pub unsafe fn FSEv05_initDState(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
    dt: *const FSEv05_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv05_DTableHeader;
    (*DStatePtr).state = BITv05_readBits(bitD, (*DTableH).tableLog as c_uint);
    BITv05_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

pub unsafe fn FSEv05_peakSymbol(DStatePtr: *mut FSEv05_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

pub unsafe fn FSEv05_decodeSymbol(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = BITv05_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSEv05_decodeSymbolFast(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = BITv05_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSEv05_endOfDState(DStatePtr: *const FSEv05_DState_t) -> c_uint {
    ((*DStatePtr).state == 0) as c_uint
}

/* **************************************************************
 *  FSEv05 -- Tuning parameters / constants
 ****************************************************************/
pub const FSEv05_MAX_MEMORY_USAGE: U32 = 14;
pub const FSEv05_DEFAULT_MEMORY_USAGE: U32 = 13;
pub const FSEv05_MAX_SYMBOL_VALUE: U32 = 255;

pub const FSEv05_MAX_TABLELOG: U32 = FSEv05_MAX_MEMORY_USAGE - 2;
pub const FSEv05_MAX_TABLESIZE: U32 = 1u32 << FSEv05_MAX_TABLELOG;
pub const FSEv05_MAXTABLESIZE_MASK: U32 = FSEv05_MAX_TABLESIZE - 1;
pub const FSEv05_DEFAULT_TABLELOG: U32 = FSEv05_DEFAULT_MEMORY_USAGE - 2;
pub const FSEv05_MIN_TABLELOG: U32 = 5;

pub const FSEv05_TABLELOG_ABSOLUTE_MAX: U32 = 15;

/* **************************************************************
 *  FSEv05 -- Complex types
 ****************************************************************/
pub type DTable_max_t = [c_uint; FSEv05_DTABLE_SIZE_U32(FSEv05_MAX_TABLELOG)];

/* **************************************************************
 *  FSEv05 -- Function templates
 ****************************************************************/
pub fn FSEv05_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1).wrapping_add(tableSize >> 3).wrapping_add(3)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_createDTable(mut tableLog: c_uint) -> *mut FSEv05_DTable {
    if tableLog > FSEv05_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv05_TABLELOG_ABSOLUTE_MAX;
    }
    malloc(FSEv05_DTABLE_SIZE_U32(tableLog) * core::mem::size_of::<U32>()) as *mut FSEv05_DTable
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_freeDTable(dt: *mut FSEv05_DTable) {
    free(dt as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable(
    dt: *mut FSEv05_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let mut DTableH = FSEv05_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tdPtr = dt.add(1) as *mut c_void; /* because dt is unsigned, 32-bits aligned on 32-bits */
    let tableDecode = tdPtr as *mut FSEv05_decode_t;
    let tableSize: U32 = 1u32.wrapping_shl(tableLog);
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let step: U32 = FSEv05_tableStep(tableSize);
    let mut symbolNext: [U16; FSEv05_MAX_SYMBOL_VALUE as usize + 1] =
        [0; FSEv05_MAX_SYMBOL_VALUE as usize + 1];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);
    let largeLimit: S16 = (1i32.wrapping_shl(tableLog.wrapping_sub(1))) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    /* Sanity Checks */
    if maxSymbolValue > FSEv05_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSEv05_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    memset(
        tableDecode as *mut c_void,
        0,
        core::mem::size_of::<BYTE>() * (maxSymbolValue as usize + 1),
    );
    DTableH.tableLog = tableLog as U16;
    s = 0;
    while s <= maxSymbolValue {
        if *normalizedCounter.add(s as usize) == -1 {
            (*tableDecode.add(highThreshold as usize)).symbol = s as BYTE;
            highThreshold = highThreshold.wrapping_sub(1);
            symbolNext[s as usize] = 1;
        } else {
            if *normalizedCounter.add(s as usize) >= largeLimit {
                noLarge = 0;
            }
            symbolNext[s as usize] = *normalizedCounter.add(s as usize) as U16;
        }
        s = s.wrapping_add(1);
    }

    /* Spread symbols */
    s = 0;
    while s <= maxSymbolValue {
        let mut i: c_int = 0;
        while i < *normalizedCounter.add(s as usize) as c_int {
            (*tableDecode.add(position as usize)).symbol = s as BYTE;
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
            let symbol: BYTE = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.add(i as usize)).nbBits =
                tableLog.wrapping_sub(BITv05_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(i as usize)).newState = ((nextState as U32)
                .wrapping_shl((*tableDecode.add(i as usize)).nbBits as U32)
                .wrapping_sub(tableSize)) as U16;
            i = i.wrapping_add(1);
        }
    }

    DTableH.fastMode = noLarge as U16;
    memcpy(
        dt as *mut c_void,
        &DTableH as *const FSEv05_DTableHeader as *const c_void,
        core::mem::size_of::<FSEv05_DTableHeader>(),
    );
    0
}

/*-****************************************
 *  FSEv05 helper functions
 ******************************************/
#[unsafe(no_mangle)]
pub extern "C" fn FSEv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn FSEv05_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/*-**************************************************************
 *  FSEv05 NCount encoding-decoding
 ****************************************************************/
pub fn FSEv05_abs(a: S16) -> S16 {
    if a < 0 {
        a.wrapping_neg()
    } else {
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_readNCount(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const BYTE;
    let iend = istart.wrapping_add(hbSize);
    let mut ip = istart;
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
    nbBits = ((bitStream & 0xF).wrapping_add(FSEv05_MIN_TABLELOG)) as c_int; /* extract tableLog */
    if nbBits > FSEv05_TABLELOG_ABSOLUTE_MAX as c_int {
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
                *normalizedCounter.add(charnum as usize) = 0;
                charnum = charnum.wrapping_add(1);
            }
            if (ip <= iend.wrapping_sub(7))
                || (ip.wrapping_offset((bitCount >> 3) as isize) <= iend.wrapping_sub(4))
            {
                ip = ip.wrapping_offset((bitCount >> 3) as isize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr(bitCount as u32);
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: S16 = ((2i32.wrapping_mul(threshold).wrapping_sub(1)).wrapping_sub(remaining))
                as S16;
            let mut count: S16;

            if (bitStream & (threshold.wrapping_sub(1)) as U32) < (max as U32) {
                count = (bitStream & (threshold.wrapping_sub(1)) as U32) as S16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2i32.wrapping_mul(threshold).wrapping_sub(1)) as U32) as S16;
                if (count as c_int) >= threshold {
                    count = count.wrapping_sub(max);
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining = remaining.wrapping_sub(FSEv05_abs(count) as c_int);
            *normalizedCounter.add(charnum as usize) = count;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as c_int;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            if (ip <= iend.wrapping_sub(7))
                || (ip.wrapping_offset((bitCount >> 3) as isize) <= iend.wrapping_sub(4))
            {
                ip = ip.wrapping_offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount = bitCount.wrapping_sub(
                    (8isize.wrapping_mul(
                        (iend as isize).wrapping_sub(4).wrapping_sub(ip as isize),
                    )) as c_int,
                );
                ip = iend.wrapping_sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr((bitCount & 31) as u32);
        }
    }
    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    *maxSVPtr = charnum.wrapping_sub(1);

    ip = ip.wrapping_offset(((bitCount + 7) >> 3) as isize);
    if (ip as usize).wrapping_sub(istart as usize) > hbSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    (ip as usize).wrapping_sub(istart as usize)
}

/*-*******************************************************
 *  FSEv05 Decompression (Byte symbols)
 *********************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable_rle(
    dt: *mut FSEv05_DTable,
    symbolValue: BYTE,
) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv05_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let cell = dPtr as *mut FSEv05_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable_raw(dt: *mut FSEv05_DTable, nbBits: c_uint) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv05_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv05_decode_t;
    let tableSize: c_uint = 1u32.wrapping_shl(nbBits);
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
        (*dinfo.add(s as usize)).newState = 0;
        (*dinfo.add(s as usize)).symbol = s as BYTE;
        (*dinfo.add(s as usize)).nbBits = nbBits as BYTE;
        s = s.wrapping_add(1);
    }

    0
}

macro_rules! FSEv05_GETSYMBOL {
    ($statePtr:expr, $bitD:expr, $fast:expr) => {
        if $fast != 0 {
            FSEv05_decodeSymbolFast($statePtr, $bitD)
        } else {
            FSEv05_decodeSymbol($statePtr, $bitD)
        }
    };
}

pub unsafe fn FSEv05_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv05_DTable,
    fast: c_uint,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.wrapping_add(maxDstSize);
    let olimit = omax.wrapping_sub(3);

    let mut bitD: BITv05_DStream_t = core::mem::zeroed();
    let mut state1: FSEv05_DState_t = core::mem::zeroed();
    let mut state2: FSEv05_DState_t = core::mem::zeroed();
    let bitDp: *mut BITv05_DStream_t = &mut bitD;
    let s1: *mut FSEv05_DState_t = &mut state1;
    let s2: *mut FSEv05_DState_t = &mut state2;
    let mut errorCode: usize;

    /* Init */
    errorCode = BITv05_initDStream(bitDp, cSrc, cSrcSize);
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }

    FSEv05_initDState(s1, bitDp, dt);
    FSEv05_initDState(s2, bitDp, dt);

    /* 4 symbols per loop */
    loop {
        if !((BITv05_reloadDStream(bitDp) == BITv05_DStream_unfinished) && (op < olimit)) {
            break;
        }
        *op.add(0) = FSEv05_GETSYMBOL!(s1, bitDp, fast);

        if FSEv05_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as U32 {
            /* This test must be static */
            BITv05_reloadDStream(bitDp);
        }

        *op.add(1) = FSEv05_GETSYMBOL!(s2, bitDp, fast);

        if FSEv05_MAX_TABLELOG * 4 + 7 > (core::mem::size_of::<usize>() * 8) as U32 {
            /* This test must be static */
            if BITv05_reloadDStream(bitDp) > BITv05_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.add(2) = FSEv05_GETSYMBOL!(s1, bitDp, fast);

        if FSEv05_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as U32 {
            /* This test must be static */
            BITv05_reloadDStream(bitDp);
        }

        *op.add(3) = FSEv05_GETSYMBOL!(s2, bitDp, fast);

        op = op.wrapping_add(4);
    }

    /* tail */
    loop {
        if (BITv05_reloadDStream(bitDp) > BITv05_DStream_completed)
            || (op == omax)
            || ((BITv05_endOfDStream(bitDp) != 0) && ((fast != 0) || (FSEv05_endOfDState(s1) != 0)))
        {
            break;
        }

        *op = FSEv05_GETSYMBOL!(s1, bitDp, fast);
        op = op.wrapping_add(1);

        if (BITv05_reloadDStream(bitDp) > BITv05_DStream_completed)
            || (op == omax)
            || ((BITv05_endOfDStream(bitDp) != 0) && ((fast != 0) || (FSEv05_endOfDState(s2) != 0)))
        {
            break;
        }

        *op = FSEv05_GETSYMBOL!(s2, bitDp, fast);
        op = op.wrapping_add(1);
    }

    /* end ? */
    if (BITv05_endOfDStream(bitDp) != 0)
        && (FSEv05_endOfDState(s1) != 0)
        && (FSEv05_endOfDState(s2) != 0)
    {
        return (op as usize).wrapping_sub(ostart as usize);
    }

    if op == omax {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */
    }

    ERROR(ZSTD_error_corruption_detected)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv05_DTable,
) -> usize {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv05_DTableHeader;
    let fastMode: U32 = (*DTableH).fastMode as U32;

    /* select fast mode (static) */
    if fastMode != 0 {
        return FSEv05_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSEv05_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [S16; FSEv05_MAX_SYMBOL_VALUE as usize + 1] =
        [0; FSEv05_MAX_SYMBOL_VALUE as usize + 1];
    let mut dt: DTable_max_t = [0; FSEv05_DTABLE_SIZE_U32(FSEv05_MAX_TABLELOG)];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv05_MAX_SYMBOL_VALUE;
    let mut errorCode: usize;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
    }

    /* normal FSEv05 decoding mode */
    errorCode = FSEv05_readNCount(
        counting.as_mut_ptr(),
        &mut maxSymbolValue,
        &mut tableLog,
        istart as *const c_void,
        cSrcSize,
    );
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

    errorCode = FSEv05_buildDTable(
        dt.as_mut_ptr(),
        counting.as_ptr(),
        maxSymbolValue,
        tableLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }

    /* always return, even if it is an error code */
    FSEv05_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

/* ******************************************************************
 *  HUFv05 -- static allocation
 *******************************************************************/
pub const fn HUFv05_DTABLE_SIZE(maxTableLog: U32) -> usize {
    1 + (1usize << maxTableLog)
}

/* **************************************************************
 *  HUFv05 -- Constants
 ****************************************************************/
pub const HUFv05_ABSOLUTEMAX_TABLELOG: U32 = 16;
pub const HUFv05_MAX_TABLELOG: U32 = 12;
pub const HUFv05_DEFAULT_TABLELOG: U32 = HUFv05_MAX_TABLELOG;
pub const HUFv05_MAX_SYMBOL_VALUE: U32 = 255;

/* **************************************************************
 *  HUFv05 -- Error Management
 ****************************************************************/
#[unsafe(no_mangle)]
pub extern "C" fn HUFv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUFv05_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* *******************************************************
 *  HUFv05 : Huffman block decompression
 *********************************************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUFv05_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
} /* single-symbol decoding */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUFv05_DEltX4 {
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

/* function-local `static int l[14]` of HUFv05_readStats */
pub static HUFv05_readStats_l: [c_int; 14] =
    [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];

/* ! HUFv05_readStats
    Read compact Huffman tree, saved by HUFv05_writeCTable
    @huffWeight : destination buffer
    @return : size read from `src`
*/
pub unsafe fn HUFv05_readStats(
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
    let mut ip = src as *const BYTE;
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
            oSize = HUFv05_readStats_l[iSize.wrapping_sub(242)] as usize;
            memset(huffWeight as *mut c_void, 1, hwSize);
            iSize = 0;
        } else {
            /* Incompressible */
            oSize = iSize.wrapping_sub(127);
            iSize = (oSize.wrapping_add(1)) / 2;
            if iSize.wrapping_add(1) > srcSize {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if oSize >= hwSize {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(1);
            n = 0;
            while (n as usize) < oSize {
                *huffWeight.add(n as usize) = *ip.add((n as usize) / 2) >> 4;
                *huffWeight.add((n as usize).wrapping_add(1)) = *ip.add((n as usize) / 2) & 15;
                n = n.wrapping_add(2);
            }
        }
    } else {
        /* header compressed with FSEv05 (normal case) */
        if iSize.wrapping_add(1) > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        /* max (hwSize-1) values decoded, as last one is implied */
        oSize = FSEv05_decompress(
            huffWeight as *mut c_void,
            hwSize.wrapping_sub(1),
            ip.add(1) as *const c_void,
            iSize,
        );
        if FSEv05_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankStats as *mut c_void,
        0,
        (HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if (*huffWeight.add(n as usize) as U32) >= HUFv05_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        let idx = *huffWeight.add(n as usize) as usize;
        *rankStats.add(idx) = (*rankStats.add(idx)).wrapping_add(1);
        weightTotal = weightTotal
            .wrapping_add((1u32.wrapping_shl(*huffWeight.add(n as usize) as U32)) >> 1);
        n = n.wrapping_add(1);
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    tableLog = BITv05_highbit32(weightTotal).wrapping_add(1);
    if tableLog > HUFv05_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_corruption_detected);
    }
    {
        /* determine last weight */
        let total: U32 = 1u32.wrapping_shl(tableLog);
        let rest: U32 = total.wrapping_sub(weightTotal);
        let verif: U32 = 1u32.wrapping_shl(BITv05_highbit32(rest));
        let lastWeight: U32 = BITv05_highbit32(rest).wrapping_add(1);
        if verif != rest {
            return ERROR(ZSTD_error_corruption_detected); /* last value must be a clean power of 2 */
        }
        *huffWeight.add(oSize) = lastWeight as BYTE;
        *rankStats.add(lastWeight as usize) = (*rankStats.add(lastWeight as usize)).wrapping_add(1);
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || ((*rankStats.add(1) & 1) != 0) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* results */
    *nbSymbolsPtr = (oSize.wrapping_add(1)) as U32;
    *tableLogPtr = tableLog;
    iSize.wrapping_add(1)
}

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_readDTableX2(
    DTable: *mut U16,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut huffWeight: [BYTE; HUFv05_MAX_SYMBOL_VALUE as usize + 1] =
        [0; HUFv05_MAX_SYMBOL_VALUE as usize + 1];
    let mut rankVal: [U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1]; /* large enough for values from 0 to 16 */
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv05_DEltX2;

    iSize = HUFv05_readStats(
        huffWeight.as_mut_ptr(),
        HUFv05_MAX_SYMBOL_VALUE as usize + 1,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv05_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > (*DTable.add(0) as U32) {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable is too small */
    }
    *DTable.add(0) = tableLog as U16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current: U32 = nextRankStart;
        nextRankStart = nextRankStart
            .wrapping_add(rankVal[n as usize].wrapping_shl(n.wrapping_sub(1)));
        rankVal[n as usize] = current;
        n = n.wrapping_add(1);
    }

    /* fill DTable */
    n = 0;
    while n < nbSymbols {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32.wrapping_shl(w)) >> 1;
        let mut i: U32;
        let mut D = HUFv05_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = tableLog.wrapping_add(1).wrapping_sub(w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize].wrapping_add(length) {
            *dt.add(i as usize) = D;
            i = i.wrapping_add(1);
        }
        rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
        n = n.wrapping_add(1);
    }

    iSize
}

pub unsafe fn HUFv05_decodeSymbolX2(
    Dstream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val: usize = BITv05_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.add(val)).byte;
    BITv05_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

macro_rules! HUFv05_DECODE_SYMBOLX2_0 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUFv05_decodeSymbolX2($DStreamPtr, $dt, $dtLog);
        $ptr = $ptr.wrapping_add(1);
    }};
}

macro_rules! HUFv05_DECODE_SYMBOLX2_1 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {
        if (MEM_64bits() != 0) || (HUFv05_MAX_TABLELOG <= 12) {
            HUFv05_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog)
        }
    };
}

macro_rules! HUFv05_DECODE_SYMBOLX2_2 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {
        if MEM_64bits() != 0 {
            HUFv05_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog)
        }
    };
}

pub unsafe fn HUFv05_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv05_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv05_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 4 symbols at a time */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(4))
    {
        HUFv05_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX2_1!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished) && (p < pEnd) {
        HUFv05_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        HUFv05_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    (pEnd as usize).wrapping_sub(pStart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    let op = dst as *mut BYTE;
    let oend = op.wrapping_add(dstSize);
    let dtLog: U32 = *DTable.add(0) as U32;
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv05_DEltX2).add(1);
    let mut bitD: BITv05_DStream_t = core::mem::zeroed();
    let bitDp: *mut BITv05_DStream_t = &mut bitD;

    if dstSize <= cSrcSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let errorCode: usize = BITv05_initDStream(bitDp, cSrc, cSrcSize);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv05_decodeStreamX2(op, bitDp, oend, dt, dtLog);

    /* check */
    if BITv05_endOfDStream(bitDp) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U16; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)] =
        [0; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut errorCode: usize;

    errorCode = HUFv05_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

    HUFv05_decompress1X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    /* Check */
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }
    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv05_DEltX2).add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv05_DStream_t = core::mem::zeroed();
        let b1: *mut BITv05_DStream_t = &mut bitD1;
        let b2: *mut BITv05_DStream_t = &mut bitD2;
        let b3: *mut BITv05_DStream_t = &mut bitD3;
        let b4: *mut BITv05_DStream_t = &mut bitD4;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.wrapping_add(6); /* jumpTable */
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize.wrapping_add(3)) / 4;
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
        errorCode = BITv05_initDStream(b1, istart1 as *const c_void, length1);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(b2, istart2 as *const c_void, length2);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(b3, istart3 as *const c_void, length3);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(b4, istart4 as *const c_void, length4);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv05_reloadDStream(b1)
            | BITv05_reloadDStream(b2)
            | BITv05_reloadDStream(b3)
            | BITv05_reloadDStream(b4);
        while (endSignal == BITv05_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            HUFv05_DECODE_SYMBOLX2_2!(op1, b1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op2, b2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op3, b3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op4, b4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_1!(op1, b1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_1!(op2, b2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_1!(op3, b3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_1!(op4, b4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op1, b1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op2, b2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op3, b3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op4, b4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_0!(op1, b1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_0!(op2, b2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_0!(op3, b3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_0!(op4, b4, dt, dtLog);
            endSignal = BITv05_reloadDStream(b1)
                | BITv05_reloadDStream(b2)
                | BITv05_reloadDStream(b3)
                | BITv05_reloadDStream(b4);
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
        HUFv05_decodeStreamX2(op1, b1, opStart2, dt, dtLog);
        HUFv05_decodeStreamX2(op2, b2, opStart3, dt, dtLog);
        HUFv05_decodeStreamX2(op3, b3, opStart4, dt, dtLog);
        HUFv05_decodeStreamX2(op4, b4, oend, dt, dtLog);

        /* check */
        endSignal = BITv05_endOfDStream(b1)
            & BITv05_endOfDStream(b2)
            & BITv05_endOfDStream(b3)
            & BITv05_endOfDStream(b4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
        return dstSize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U16; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)] =
        [0; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut errorCode: usize;

    errorCode = HUFv05_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

    HUFv05_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/* *************************/
/* double-symbols decoding */
/* *************************/

pub unsafe fn HUFv05_fillDTableX4Level2(
    DTable: *mut HUFv05_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: c_int,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    let mut DElt = HUFv05_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1];
    let mut s: U32;

    /* get pre-calculated rankVal */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1]>(),
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
            *DTable.add(i as usize) = DElt;
            i = i.wrapping_add(1);
        }
    }

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        /* note : sortedSymbols already skipped */
        let symbol: U32 = (*sortedSymbols.add(s as usize)).symbol as U32;
        let weight: U32 = (*sortedSymbols.add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let length: U32 = 1u32.wrapping_shl(sizeLog.wrapping_sub(nbBits));
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
            *DTable.add(i as usize) = DElt;
            i = i.wrapping_add(1);
            if !(i < end) {
                break;
            }
        } /* since length >= 1 */

        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s = s.wrapping_add(1);
    }
}

pub type rankVal_t = [[U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1];
    HUFv05_ABSOLUTEMAX_TABLELOG as usize];

pub unsafe fn HUFv05_fillDTableX4(
    DTable: *mut HUFv05_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const [U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1];
    /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int;
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1]>(),
    );

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.add(s as usize)).weight as U32;
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
            sortedRank = *rankStart.add(minWeight as usize);
            HUFv05_fillDTableX4Level2(
                DTable.add(start as usize),
                targetLog.wrapping_sub(nbBits),
                nbBits,
                rankValOrigin.add(nbBits as usize) as *const U32,
                minWeight,
                sortedList.add(sortedRank as usize),
                sortedListSize.wrapping_sub(sortedRank),
                nbBitsBaseline,
                symbol,
            );
        } else {
            let mut i: U32;
            let end: U32 = start.wrapping_add(length);
            let mut DElt = HUFv05_DEltX4 {
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
                i = i.wrapping_add(1);
            }
        }
        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s = s.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_readDTableX4(
    DTable: *mut c_uint,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightList: [BYTE; HUFv05_MAX_SYMBOL_VALUE as usize + 1] =
        [0; HUFv05_MAX_SYMBOL_VALUE as usize + 1];
    let mut sortedSymbol: [sortedSymbol_t; HUFv05_MAX_SYMBOL_VALUE as usize + 1] =
        [sortedSymbol_t {
            symbol: 0,
            weight: 0,
        }; HUFv05_MAX_SYMBOL_VALUE as usize + 1];
    let mut rankStats: [U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1];
    let mut rankStart0: [U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 2] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 2];
    let rankStart: *mut U32 = rankStart0.as_mut_ptr().add(1);
    let mut rankVal: rankVal_t = [[0; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1];
        HUFv05_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let mut sizeOfSort: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.add(0);
    let iSize: usize;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUFv05_DEltX4).add(1);

    if memLog > HUFv05_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUFv05_readStats(
        weightList.as_mut_ptr(),
        HUFv05_MAX_SYMBOL_VALUE as usize + 1,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv05_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > memLog {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable can't fit code depth */
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW = maxW.wrapping_sub(1);
    } /* necessarily finds a solution before 0 */

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankStats[w as usize]);
            *rankStart.add(w as usize) = current;
            w = w.wrapping_add(1);
        }
        *rankStart.add(0) = nextRankStart; /* put all 0w symbols at the end of sorted list */
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = *rankStart.add(w as usize);
            *rankStart.add(w as usize) = r.wrapping_add(1);
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s = s.wrapping_add(1);
        }
        *rankStart.add(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: c_int = (memLog.wrapping_sub(tableLog)).wrapping_sub(1) as c_int; /* tableLog <= memLog */
        let rankValBase: *mut [U32; HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1] =
            rankVal.as_mut_ptr();
        let rankVal0: *mut U32 = rankValBase as *mut U32;
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankVal;
            nextRankVal = nextRankVal.wrapping_add(
                rankStats[w as usize].wrapping_shl(w.wrapping_add(rescale as U32)),
            );
            *rankVal0.add(w as usize) = current;
            w = w.wrapping_add(1);
        }
        consumed = minBits;
        while consumed <= memLog.wrapping_sub(minBits) {
            let rankValPtr: *mut U32 = rankValBase.add(consumed as usize) as *mut U32;
            w = 1;
            while w <= maxW {
                *rankValPtr.add(w as usize) = (*rankVal0.add(w as usize)).wrapping_shr(consumed);
                w = w.wrapping_add(1);
            }
            consumed = consumed.wrapping_add(1);
        }
    }

    HUFv05_fillDTableX4(
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

pub unsafe fn HUFv05_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BITv05_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.add(val) as *const c_void, 2);
    BITv05_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

pub unsafe fn HUFv05_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BITv05_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BITv05_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (core::mem::size_of::<usize>() * 8) as c_uint {
            BITv05_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (core::mem::size_of::<usize>() * 8) as c_uint {
                /* ugly hack; works only because it's the last symbol */
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as c_uint;
            }
        }
    }
    1
}

macro_rules! HUFv05_DECODE_SYMBOLX4_0 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.wrapping_add(HUFv05_decodeSymbolX4(
            $ptr as *mut c_void,
            $DStreamPtr,
            $dt,
            $dtLog,
        ) as usize);
    }};
}

macro_rules! HUFv05_DECODE_SYMBOLX4_1 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {
        if (MEM_64bits() != 0) || (HUFv05_MAX_TABLELOG <= 12) {
            HUFv05_DECODE_SYMBOLX4_0!($ptr, $DStreamPtr, $dt, $dtLog)
        }
    };
}

macro_rules! HUFv05_DECODE_SYMBOLX4_2 {
    ($ptr:ident, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {
        if MEM_64bits() != 0 {
            HUFv05_DECODE_SYMBOLX4_0!($ptr, $DStreamPtr, $dt, $dtLog)
        }
    };
}

pub unsafe fn HUFv05_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv05_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && (p < pEnd.wrapping_sub(7))
    {
        HUFv05_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX4_1!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(2))
    {
        HUFv05_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p <= pEnd.wrapping_sub(2) {
        HUFv05_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog); /* no need to reload : reached the end of DStream */
    }

    if p < pEnd {
        p = p.wrapping_add(
            HUFv05_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
        );
    }

    (p as usize).wrapping_sub(pStart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const c_uint,
) -> usize {
    let istart = cSrc as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.wrapping_add(dstSize);

    let dtLog: U32 = *DTable.add(0);
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv05_DEltX4).add(1);
    let errorCode: usize;

    /* Init */
    let mut bitD: BITv05_DStream_t = core::mem::zeroed();
    let bitDp: *mut BITv05_DStream_t = &mut bitD;
    errorCode = BITv05_initDStream(bitDp, istart as *const c_void, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }

    /* finish bitStreams one by one */
    HUFv05_decodeStreamX4(ostart, bitDp, oend, dt, dtLog);

    /* check */
    if BITv05_endOfDStream(bitDp) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* decoded size */
    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [c_uint; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)] =
        [0; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize: usize = HUFv05_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize = cSrcSize.wrapping_sub(hSize);

    HUFv05_decompress1X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const c_uint,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv05_DEltX4).add(1);
        let dtLog: U32 = *DTable.add(0);
        let mut errorCode: usize;

        /* Init */
        let mut bitD1: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv05_DStream_t = core::mem::zeroed();
        let b1: *mut BITv05_DStream_t = &mut bitD1;
        let b2: *mut BITv05_DStream_t = &mut bitD2;
        let b3: *mut BITv05_DStream_t = &mut bitD3;
        let b4: *mut BITv05_DStream_t = &mut bitD4;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.wrapping_add(6); /* jumpTable */
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize.wrapping_add(3)) / 4;
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
        errorCode = BITv05_initDStream(b1, istart1 as *const c_void, length1);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(b2, istart2 as *const c_void, length2);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(b3, istart3 as *const c_void, length3);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(b4, istart4 as *const c_void, length4);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv05_reloadDStream(b1)
            | BITv05_reloadDStream(b2)
            | BITv05_reloadDStream(b3)
            | BITv05_reloadDStream(b4);
        while (endSignal == BITv05_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            HUFv05_DECODE_SYMBOLX4_2!(op1, b1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op2, b2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op3, b3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op4, b4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_1!(op1, b1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_1!(op2, b2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_1!(op3, b3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_1!(op4, b4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op1, b1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op2, b2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op3, b3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op4, b4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_0!(op1, b1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_0!(op2, b2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_0!(op3, b3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_0!(op4, b4, dt, dtLog);

            endSignal = BITv05_reloadDStream(b1)
                | BITv05_reloadDStream(b2)
                | BITv05_reloadDStream(b3)
                | BITv05_reloadDStream(b4);
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
        HUFv05_decodeStreamX4(op1, b1, opStart2, dt, dtLog);
        HUFv05_decodeStreamX4(op2, b2, opStart3, dt, dtLog);
        HUFv05_decodeStreamX4(op3, b3, opStart4, dt, dtLog);
        HUFv05_decodeStreamX4(op4, b4, oend, dt, dtLog);

        /* check */
        endSignal = BITv05_endOfDStream(b1)
            & BITv05_endOfDStream(b2)
            & BITv05_endOfDStream(b3)
            & BITv05_endOfDStream(b4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
        return dstSize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [c_uint; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)] =
        [0; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize: usize = HUFv05_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize = cSrcSize.wrapping_sub(hSize);

    HUFv05_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/* ********************************/
/* Generic decompression selector */
/* ********************************/

#[repr(C)]
#[derive(Copy, Clone)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}

const fn at(tableTime: U32, decode256Time: U32) -> algo_time_t {
    algo_time_t {
        tableTime,
        decode256Time,
    }
}

pub static algoTime: [[algo_time_t; 3]; 16] = [
    /* single, double, quad */
    [at(0, 0), at(1, 1), at(2, 2)], /* Q==0 : impossible */
    [at(0, 0), at(1, 1), at(2, 2)], /* Q==1 : impossible */
    [at(38, 130), at(1313, 74), at(2151, 38)], /* Q == 2 : 12-18% */
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

pub type decompressionAlgo =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let decompress: [Option<decompressionAlgo>; 3] = [
        Some(HUFv05_decompress4X2 as decompressionAlgo),
        Some(HUFv05_decompress4X4 as decompressionAlgo),
        None,
    ];
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
    if cSrcSize >= dstSize {
        return ERROR(ZSTD_error_corruption_detected); /* invalid, or not compressed */
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
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

    /* advantage to algorithms using less memory, for cache eviction */
    Dtime[1] = Dtime[1].wrapping_add(Dtime[1] >> 4);
    Dtime[2] = Dtime[2].wrapping_add(Dtime[2] >> 3);

    if Dtime[1] < Dtime[0] {
        algoNb = 1;
    }

    (decompress[algoNb as usize].unwrap())(dst, dstSize, cSrc, cSrcSize)
}
