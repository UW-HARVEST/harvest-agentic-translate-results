//! Translation of `legacy/zstd_v06.c` (+ `legacy/zstd_v06.h`).
//!
//! This file is entirely self-contained, exactly like the C original: it
//! carries its own `mem.h` helpers, its own `BITv06_*` bitstream, its own
//! `FSEv06_*` / `HUFv06_*` entropy decoders, the `ZSTDv06_*` decoder and the
//! `ZBUFFv06_*` streaming layer.

use core::ffi::{c_char, c_void};

use crate::libc::{free, malloc, memcpy, memmove, memset};

/* ******************************************************************
 *  error_private.h  (the C file does `#include "../common/error_private.h"`)
 ********************************************************************/

pub const ZSTD_error_no_error: i32 = 0;
pub const ZSTD_error_GENERIC: i32 = 1;
pub const ZSTD_error_prefix_unknown: i32 = 10;
pub const ZSTD_error_version_unsupported: i32 = 12;
pub const ZSTD_error_frameParameter_unsupported: i32 = 14;
pub const ZSTD_error_frameParameter_windowTooLarge: i32 = 16;
pub const ZSTD_error_corruption_detected: i32 = 20;
pub const ZSTD_error_checksum_wrong: i32 = 22;
pub const ZSTD_error_literals_headerWrong: i32 = 24;
pub const ZSTD_error_dictionary_corrupted: i32 = 30;
pub const ZSTD_error_dictionary_wrong: i32 = 32;
pub const ZSTD_error_dictionaryCreation_failed: i32 = 34;
pub const ZSTD_error_parameter_unsupported: i32 = 40;
pub const ZSTD_error_parameter_combination_unsupported: i32 = 41;
pub const ZSTD_error_parameter_outOfBound: i32 = 42;
pub const ZSTD_error_tableLog_tooLarge: i32 = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: i32 = 46;
pub const ZSTD_error_maxSymbolValue_tooSmall: i32 = 48;
pub const ZSTD_error_cannotProduce_uncompressedBlock: i32 = 49;
pub const ZSTD_error_stabilityCondition_notRespected: i32 = 50;
pub const ZSTD_error_stage_wrong: i32 = 60;
pub const ZSTD_error_init_missing: i32 = 62;
pub const ZSTD_error_memory_allocation: i32 = 64;
pub const ZSTD_error_workSpace_tooSmall: i32 = 66;
pub const ZSTD_error_dstSize_tooSmall: i32 = 70;
pub const ZSTD_error_srcSize_wrong: i32 = 72;
pub const ZSTD_error_dstBuffer_null: i32 = 74;
pub const ZSTD_error_maxCode: i32 = 120;

/// `ERROR(name)` : `((size_t)-PREFIX(name))`
#[inline(always)]
const fn ERROR(code: i32) -> usize {
    (-(code as isize)) as usize
}

#[inline(always)]
const fn ERR_isError(code: usize) -> u32 {
    (code > ERROR(ZSTD_error_maxCode)) as u32
}

#[inline(always)]
const fn ERR_getErrorCode(code: usize) -> i32 {
    if ERR_isError(code) == 0 {
        return 0;
    }
    (0usize.wrapping_sub(code)) as i32
}

extern "C" {
    /* defined by common/error_private.c (translated in crate::common::error_private) */
    fn ERR_getErrorString(code: i32) -> *const c_char;
}

#[inline(always)]
unsafe fn ERR_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorString(ERR_getErrorCode(code))
}

/* ******************************************************************
 *  mem.h  (v06 private copy)
 ********************************************************************/

pub type BYTE = u8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

#[inline(always)]
fn MEM_32bits() -> u32 {
    (core::mem::size_of::<usize>() == 4) as u32
}

#[inline(always)]
fn MEM_64bits() -> u32 {
    (core::mem::size_of::<usize>() == 8) as u32
}

#[inline(always)]
fn MEM_isLittleEndian() -> u32 {
    1
}

#[inline(always)]
unsafe fn MEM_read16(memPtr: *const c_void) -> U16 {
    core::ptr::read_unaligned(memPtr as *const U16)
}

#[inline(always)]
unsafe fn MEM_read32(memPtr: *const c_void) -> U32 {
    core::ptr::read_unaligned(memPtr as *const U32)
}

#[inline(always)]
unsafe fn MEM_read64(memPtr: *const c_void) -> U64 {
    core::ptr::read_unaligned(memPtr as *const U64)
}

#[inline(always)]
unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    core::ptr::write_unaligned(memPtr as *mut U16, value);
}

#[inline(always)]
fn MEM_swap32(input: U32) -> U32 {
    input.swap_bytes()
}

#[inline(always)]
fn MEM_swap64(input: U64) -> U64 {
    input.swap_bytes()
}

/*=== Little endian r/w ===*/

#[inline(always)]
unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        MEM_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}

#[inline(always)]
unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let p = memPtr as *mut BYTE;
        *p = val as BYTE;
        *p.add(1) = (val >> 8) as BYTE;
    }
}

#[inline(always)]
unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        MEM_read32(memPtr)
    } else {
        MEM_swap32(MEM_read32(memPtr))
    }
}

#[inline(always)]
unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        MEM_read64(memPtr)
    } else {
        MEM_swap64(MEM_read64(memPtr))
    }
}

#[inline(always)]
unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    if MEM_32bits() != 0 {
        MEM_readLE32(memPtr) as usize
    } else {
        MEM_readLE64(memPtr) as usize
    }
}

/* ******************************************************************
 *  zstd_v06.h / zstd_v06 static header constants
 ********************************************************************/

pub const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526;

pub const ZSTDv06_FRAMEHEADERSIZE_MAX: usize = 13;
const ZSTDv06_frameHeaderSize_min: usize = 5;
const ZSTDv06_frameHeaderSize_max: usize = ZSTDv06_FRAMEHEADERSIZE_MAX;

pub const ZSTDv06_BLOCKSIZE_MAX: usize = 128 * 1024;

/* ******************************************************************
 *  zstd_internal (v06)
 ********************************************************************/

const ZSTDv06_DICT_MAGIC: U32 = 0xEC30A436;

const ZSTDv06_REP_NUM: usize = 3;
const ZSTDv06_REP_INIT: usize = ZSTDv06_REP_NUM;
const ZSTDv06_REP_MOVE: usize = ZSTDv06_REP_NUM - 1;

const ZSTDv06_WINDOWLOG_ABSOLUTEMIN: u32 = 12;
static ZSTDv06_fcs_fieldSize: [usize; 4] = [0, 1, 2, 8];

const ZSTDv06_BLOCKHEADERSIZE: usize = 3;
const ZSTDv06_blockHeaderSize: usize = ZSTDv06_BLOCKHEADERSIZE;

/* blockType_t */
pub type blockType_t = u32;
const bt_compressed: blockType_t = 0;
const bt_raw: blockType_t = 1;
const bt_rle: blockType_t = 2;
const bt_end: blockType_t = 3;

const MIN_SEQUENCES_SIZE: usize = 1;
const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

const IS_HUF: u32 = 0;
const IS_PCH: u32 = 1;
const IS_RAW: u32 = 2;
const IS_RLE: u32 = 3;

const LONGNBSEQ: i32 = 0x7F00;

const MINMATCH: usize = 3;
const EQUAL_READ32: usize = 4;
const REPCODE_STARTVALUE: usize = 1;

const Litbits: u32 = 8;
const MaxLit: usize = (1usize << Litbits) - 1;
const MaxML: usize = 52;
const MaxLL: usize = 35;
const MaxOff: usize = 28;
const MaxSeq: usize = if MaxLL > MaxML { MaxLL } else { MaxML };
const MLFSELog: u32 = 9;
const LLFSELog: u32 = 9;
const OffFSELog: u32 = 8;

const FSEv06_ENCODING_RAW: U32 = 0;
const FSEv06_ENCODING_RLE: U32 = 1;
const FSEv06_ENCODING_STATIC: U32 = 2;
const FSEv06_ENCODING_DYNAMIC: U32 = 3;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

static LL_bits: [U32; MaxLL + 1] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
static LL_defaultNorm: [S16; MaxLL + 1] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
const LL_defaultNormLog: U32 = 6;

static ML_bits: [U32; MaxML + 1] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
static ML_defaultNorm: [S16; MaxML + 1] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
const ML_defaultNormLog: U32 = 6;

static OF_defaultNorm: [S16; MaxOff + 1] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
const OF_defaultNormLog: U32 = 5;

/*-*******************************************
 *  Shared functions to include for inlining
 *********************************************/

#[inline(always)]
unsafe fn ZSTDv06_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

const WILDCOPY_OVERLENGTH: usize = 8;

/// `ZSTDv06_wildcopy()` : can copy up to 7 bytes too many (8 if length==0)
#[inline(always)]
unsafe fn ZSTDv06_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.wrapping_offset(length);
    loop {
        ZSTDv06_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
        if !(op < oend) {
            break;
        }
    }
}

/* ******************************************************************
 *  bitstream (v06)
 ********************************************************************/

#[repr(C)]
#[derive(Copy, Clone)]
struct BITv06_DStream_t {
    bitContainer: usize,
    bitsConsumed: u32,
    ptr: *const c_char,
    start: *const c_char,
}

impl BITv06_DStream_t {
    const fn new() -> Self {
        BITv06_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        }
    }
}

type BITv06_DStream_status = u32;
const BITv06_DStream_unfinished: BITv06_DStream_status = 0;
const BITv06_DStream_endOfBuffer: BITv06_DStream_status = 1;
const BITv06_DStream_completed: BITv06_DStream_status = 2;
const BITv06_DStream_overflow: BITv06_DStream_status = 3;

#[inline(always)]
fn BITv06_highbit32(val: U32) -> u32 {
    /* __builtin_clz(val) ^ 31 */
    val.leading_zeros() ^ 31
}

/// `BITv06_initDStream()`
unsafe fn BITv06_initDStream(
    bitD: *mut BITv06_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        memset(
            bitD as *mut c_void,
            0,
            core::mem::size_of::<BITv06_DStream_t>(),
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
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC); /* endMark not present */
            }
            (*bitD).bitsConsumed = 8 - BITv06_highbit32(lastByte as U32);
        }
    } else {
        let p = srcBuffer as *const BYTE;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        /* switch(srcSize) with fall-through */
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*p.add(6) as usize) << (core::mem::size_of::<usize>() * 8 - 16));
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*p.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24));
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*p.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32));
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*p.add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*p.add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add((*p.add(1) as usize) << 8);
        }
        {
            let lastByte: BYTE = *p.wrapping_add(srcSize - 1);
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC); /* endMark not present */
            }
            (*bitD).bitsConsumed = 8 - BITv06_highbit32(lastByte as U32);
        }
        (*bitD).bitsConsumed = (*bitD)
            .bitsConsumed
            .wrapping_add(((core::mem::size_of::<usize>() - srcSize) as U32).wrapping_mul(8));
    }

    srcSize
}

#[inline(always)]
unsafe fn BITv06_lookBits(bitD: *const BITv06_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD)
        .bitContainer
        .wrapping_shl((*bitD).bitsConsumed & bitMask))
        >> 1)
        .wrapping_shr(bitMask.wrapping_sub(nbBits) & bitMask)
}

/// unsafe version; only works if nbBits >= 1
#[inline(always)]
unsafe fn BITv06_lookBitsFast(bitD: *const BITv06_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (*bitD)
        .bitContainer
        .wrapping_shl((*bitD).bitsConsumed & bitMask)
        .wrapping_shr(bitMask.wrapping_add(1).wrapping_sub(nbBits) & bitMask)
}

#[inline(always)]
unsafe fn BITv06_skipBits(bitD: *mut BITv06_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

#[inline(always)]
unsafe fn BITv06_readBits(bitD: *mut BITv06_DStream_t, nbBits: U32) -> usize {
    let value = BITv06_lookBits(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

/// unsafe version; only works if nbBits >= 1
#[inline(always)]
unsafe fn BITv06_readBitsFast(bitD: *mut BITv06_DStream_t, nbBits: U32) -> usize {
    let value = BITv06_lookBitsFast(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

unsafe fn BITv06_reloadDStream(bitD: *mut BITv06_DStream_t) -> BITv06_DStream_status {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as u32 {
        /* should never happen */
        return BITv06_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD)
            .ptr
            .wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BITv06_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as u32 {
            return BITv06_DStream_endOfBuffer;
        }
        return BITv06_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv06_DStream_status = BITv06_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize).wrapping_sub((*bitD).start as usize) as U32;
            result = BITv06_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

/// `@return` Tells if DStream has exactly reached its end (all bits consumed).
#[inline(always)]
unsafe fn BITv06_endOfDStream(DStream: *const BITv06_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as u32)) as u32
}

/* ******************************************************************
 *  FSE (v06) static header
 ********************************************************************/

pub type FSEv06_DTable = u32;

const fn FSEv06_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

const FSEv06_MAX_MEMORY_USAGE: u32 = 14;
const FSEv06_DEFAULT_MEMORY_USAGE: u32 = 13;
const FSEv06_MAX_SYMBOL_VALUE: usize = 255;

const FSEv06_MAX_TABLELOG: u32 = FSEv06_MAX_MEMORY_USAGE - 2;
const FSEv06_MAX_TABLESIZE: u32 = 1u32 << FSEv06_MAX_TABLELOG;
const FSEv06_MAXTABLESIZE_MASK: u32 = FSEv06_MAX_TABLESIZE - 1;
const FSEv06_DEFAULT_TABLELOG: u32 = FSEv06_DEFAULT_MEMORY_USAGE - 2;
const FSEv06_MIN_TABLELOG: u32 = 5;
const FSEv06_TABLELOG_ABSOLUTE_MAX: u32 = 15;

#[inline(always)]
const fn FSEv06_TABLESTEP(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FSEv06_DState_t {
    state: usize,
    table: *const c_void, /* precise table may vary, depending on U16 */
}

impl FSEv06_DState_t {
    const fn new() -> Self {
        FSEv06_DState_t {
            state: 0,
            table: core::ptr::null(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FSEv06_DTableHeader {
    tableLog: U16,
    fastMode: U16,
} /* sizeof U32 */

#[repr(C)]
#[derive(Copy, Clone)]
struct FSEv06_decode_t {
    newState: u16,
    symbol: u8,
    nbBits: u8,
} /* size == U32 */

#[inline(always)]
unsafe fn FSEv06_initDState(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
    dt: *const FSEv06_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv06_DTableHeader;
    (*DStatePtr).state = BITv06_readBits(bitD, (*DTableH).tableLog as U32);
    BITv06_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

#[inline(always)]
unsafe fn FSEv06_peekSymbol(DStatePtr: *const FSEv06_DState_t) -> BYTE {
    let DInfo =
        *((*DStatePtr).table as *const FSEv06_decode_t).wrapping_add((*DStatePtr).state);
    DInfo.symbol
}

#[inline(always)]
unsafe fn FSEv06_updateState(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) {
    let DInfo =
        *((*DStatePtr).table as *const FSEv06_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let lowBits = BITv06_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
}

#[inline(always)]
unsafe fn FSEv06_decodeSymbol(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) -> BYTE {
    let DInfo =
        *((*DStatePtr).table as *const FSEv06_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv06_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/// unsafe, only works if no symbol has a probability > 50%
#[inline(always)]
unsafe fn FSEv06_decodeSymbolFast(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) -> BYTE {
    let DInfo =
        *((*DStatePtr).table as *const FSEv06_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv06_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* ******************************************************************
 *  entropy_common (v06)
 ********************************************************************/

/*-****************************************
 *  FSE Error Management
 ******************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_isError(code: usize) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
 *  HUF Error Management
 ****************************************************************/
unsafe fn HUFv06_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/*-**************************************************************
 *  FSE NCount encoding-decoding
 ****************************************************************/
fn FSEv06_abs(a: S16) -> S16 {
    if a < 0 {
        a.wrapping_neg()
    } else {
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_readNCount(
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
    nbBits = ((bitStream & 0xF) as i32) + FSEv06_MIN_TABLELOG as i32; /* extract tableLog */
    if nbBits > FSEv06_TABLELOG_ABSOLUTE_MAX as i32 {
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
            let max: S16 = ((2 * threshold - 1) - remaining) as S16;
            let mut count: S16;

            if (bitStream & ((threshold - 1) as U32)) < ((max as i32) as U32) {
                count = (bitStream & ((threshold - 1) as U32)) as S16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & ((2 * threshold - 1) as U32)) as S16;
                if (count as i32) >= threshold {
                    count = ((count as i32) - (max as i32)) as S16;
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining -= FSEv06_abs(count) as i32;
            *normalizedCounter.wrapping_add(charnum as usize) = count;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as i32;
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
                bitCount -= (8usize
                    .wrapping_mul((iend as usize).wrapping_sub(4).wrapping_sub(ip as usize)))
                    as i32;
                ip = iend.wrapping_sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr((bitCount & 31) as u32);
        }
    } /* while ((remaining>1) && (charnum<=*maxSVPtr)) */
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

/* ******************************************************************
 *  FSE decompression (v06)
 ********************************************************************/

type DTable_max_t = [U32; FSEv06_DTABLE_SIZE_U32(FSEv06_MAX_TABLELOG)];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_createDTable(tableLog: u32) -> *mut FSEv06_DTable {
    let mut tableLog = tableLog;
    if tableLog > FSEv06_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv06_TABLELOG_ABSOLUTE_MAX;
    }
    malloc(FSEv06_DTABLE_SIZE_U32(tableLog) * core::mem::size_of::<U32>()) as *mut FSEv06_DTable
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_freeDTable(dt: *mut FSEv06_DTable) {
    free(dt as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_buildDTable(
    dt: *mut FSEv06_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    let tdPtr = dt.wrapping_add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSEv06_decode_t;
    let mut symbolNext: [U16; FSEv06_MAX_SYMBOL_VALUE + 1] = [0; FSEv06_MAX_SYMBOL_VALUE + 1];

    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);
    let tableSize: U32 = 1u32.wrapping_shl(tableLog);
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);

    /* Sanity Checks */
    if maxSymbolValue > FSEv06_MAX_SYMBOL_VALUE as u32 {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSEv06_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    {
        let mut DTableH = FSEv06_DTableHeader {
            tableLog: 0,
            fastMode: 0,
        };
        DTableH.tableLog = tableLog as U16;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = 1i32.wrapping_shl(tableLog.wrapping_sub(1)) as S16;
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
                s = s.wrapping_add(1);
            }
        }
        memcpy(
            dt as *mut c_void,
            &DTableH as *const FSEv06_DTableHeader as *const c_void,
            core::mem::size_of::<FSEv06_DTableHeader>(),
        );
    }

    /* Spread symbols */
    {
        let tableMask: U32 = tableSize.wrapping_sub(1);
        let step: U32 = FSEv06_TABLESTEP(tableSize);
        let mut s: U32 = 0;
        let mut position: U32 = 0;
        while s < maxSV1 {
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
    }

    /* Build Decoding table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol: BYTE = (*tableDecode.wrapping_add(u as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.wrapping_add(u as usize)).nbBits =
                tableLog.wrapping_sub(BITv06_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.wrapping_add(u as usize)).newState = ((nextState as U32)
                .wrapping_shl((*tableDecode.wrapping_add(u as usize)).nbBits as u32)
                .wrapping_sub(tableSize)) as U16;
            u = u.wrapping_add(1);
        }
    }

    0
}

/*-*******************************************************
 *  Decompression (Byte symbols)
 *********************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_buildDTable_rle(
    dt: *mut FSEv06_DTable,
    symbolValue: BYTE,
) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv06_DTableHeader;
    let dPtr = dt.wrapping_add(1) as *mut c_void;
    let cell = dPtr as *mut FSEv06_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_buildDTable_raw(dt: *mut FSEv06_DTable, nbBits: u32) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv06_DTableHeader;
    let dPtr = dt.wrapping_add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv06_decode_t;
    let tableSize: u32 = 1u32.wrapping_shl(nbBits);
    let tableMask: u32 = tableSize.wrapping_sub(1);
    let maxSV1: u32 = tableMask.wrapping_add(1);

    /* Sanity checks */
    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC); /* min size */
    }

    /* Build Decoding Table */
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    let mut s: u32 = 0;
    while s < maxSV1 {
        (*dinfo.wrapping_add(s as usize)).newState = 0;
        (*dinfo.wrapping_add(s as usize)).symbol = s as BYTE;
        (*dinfo.wrapping_add(s as usize)).nbBits = nbBits as BYTE;
        s = s.wrapping_add(1);
    }

    0
}

unsafe fn FSEv06_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv06_DTable,
    fast: u32,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.wrapping_add(maxDstSize);
    let olimit = omax.wrapping_sub(3);

    let mut bitD = BITv06_DStream_t::new();
    let mut state1 = FSEv06_DState_t::new();
    let mut state2 = FSEv06_DState_t::new();
    let bitDp: *mut BITv06_DStream_t = &mut bitD;
    let s1: *mut FSEv06_DState_t = &mut state1;
    let s2: *mut FSEv06_DState_t = &mut state2;

    /* Init */
    {
        let errorCode = BITv06_initDStream(bitDp, cSrc, cSrcSize);
        if ERR_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv06_initDState(s1, bitDp, dt);
    FSEv06_initDState(s2, bitDp, dt);

    /* FSEv06_MAX_TABLELOG*2+7 == 31 and *4+7 == 55, both <= 64,
     * so the static reload tests inside the loop are compiled out. */

    /* 4 symbols per loop */
    while (BITv06_reloadDStream(bitDp) == BITv06_DStream_unfinished) && (op < olimit) {
        *op.wrapping_add(0) = if fast != 0 {
            FSEv06_decodeSymbolFast(s1, bitDp)
        } else {
            FSEv06_decodeSymbol(s1, bitDp)
        };
        *op.wrapping_add(1) = if fast != 0 {
            FSEv06_decodeSymbolFast(s2, bitDp)
        } else {
            FSEv06_decodeSymbol(s2, bitDp)
        };
        *op.wrapping_add(2) = if fast != 0 {
            FSEv06_decodeSymbolFast(s1, bitDp)
        } else {
            FSEv06_decodeSymbol(s1, bitDp)
        };
        *op.wrapping_add(3) = if fast != 0 {
            FSEv06_decodeSymbolFast(s2, bitDp)
        } else {
            FSEv06_decodeSymbol(s2, bitDp)
        };
        op = op.wrapping_add(4);
    }

    /* tail */
    loop {
        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = if fast != 0 {
            FSEv06_decodeSymbolFast(s1, bitDp)
        } else {
            FSEv06_decodeSymbol(s1, bitDp)
        };
        op = op.wrapping_add(1);

        if BITv06_reloadDStream(bitDp) == BITv06_DStream_overflow {
            *op = if fast != 0 {
                FSEv06_decodeSymbolFast(s2, bitDp)
            } else {
                FSEv06_decodeSymbol(s2, bitDp)
            };
            op = op.wrapping_add(1);
            break;
        }

        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = if fast != 0 {
            FSEv06_decodeSymbolFast(s2, bitDp)
        } else {
            FSEv06_decodeSymbol(s2, bitDp)
        };
        op = op.wrapping_add(1);

        if BITv06_reloadDStream(bitDp) == BITv06_DStream_overflow {
            *op = if fast != 0 {
                FSEv06_decodeSymbolFast(s1, bitDp)
            } else {
                FSEv06_decodeSymbol(s1, bitDp)
            };
            op = op.wrapping_add(1);
            break;
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv06_DTable,
) -> usize {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv06_DTableHeader;
    let fastMode: U32 = (*DTableH).fastMode as U32;

    /* select fast mode (static) */
    if fastMode != 0 {
        return FSEv06_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSEv06_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [i16; FSEv06_MAX_SYMBOL_VALUE + 1] = [0; FSEv06_MAX_SYMBOL_VALUE + 1];
    let mut dt: DTable_max_t = [0; FSEv06_DTABLE_SIZE_U32(FSEv06_MAX_TABLELOG)];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSEv06_MAX_SYMBOL_VALUE as u32;
    let mut cSrcSize = cSrcSize;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
    }

    /* normal FSE decoding mode */
    {
        let NCountLength = FSEv06_readNCount(
            counting.as_mut_ptr(),
            &mut maxSymbolValue,
            &mut tableLog,
            istart as *const c_void,
            cSrcSize,
        );
        if ERR_isError(NCountLength) != 0 {
            return NCountLength;
        }
        if NCountLength >= cSrcSize {
            return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
        }
        ip = ip.wrapping_add(NCountLength);
        cSrcSize -= NCountLength;
    }

    {
        let errorCode = FSEv06_buildDTable(
            dt.as_mut_ptr(),
            counting.as_ptr(),
            maxSymbolValue,
            tableLog,
        );
        if ERR_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv06_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const c_void,
        cSrcSize,
        dt.as_ptr(),
    )
}

/* ******************************************************************
 *  HUF (v06) static header
 ********************************************************************/

const HUFv06_CTABLEBOUND: usize = 129;

const fn HUFv06_DTABLE_SIZE(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

const HUFv06_ABSOLUTEMAX_TABLELOG: u32 = 16;
const HUFv06_MAX_TABLELOG: u32 = 12;
const HUFv06_DEFAULT_TABLELOG: u32 = HUFv06_MAX_TABLELOG;
const HUFv06_MAX_SYMBOL_VALUE: usize = 255;

/// `HUFv06_readStats()` : Read compact Huffman tree, saved by `HUFv06_writeCTable()`.
unsafe fn HUFv06_readStats(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let weightTotal: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: usize;
    let oSize: usize;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip as usize;
    /* memset(huffWeight, 0, hwSize); is not necessary */

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            static l: [U32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
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
            {
                let mut n: U32 = 0;
                while (n as usize) < oSize {
                    *huffWeight.wrapping_add(n as usize) =
                        *ip.wrapping_add((n / 2) as usize) >> 4;
                    *huffWeight.wrapping_add(n as usize + 1) =
                        *ip.wrapping_add((n / 2) as usize) & 15;
                    n = n.wrapping_add(2);
                }
            }
        }
    } else {
        /* header compressed with FSE (normal case) */
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        oSize = FSEv06_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.wrapping_add(1) as *const c_void,
            iSize,
        ); /* max (hwSize-1) values decoded, as last one is implied */
        if ERR_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankStats as *mut c_void,
        0,
        (HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1) * core::mem::size_of::<U32>(),
    );
    let mut weightTotal_acc: U32 = 0;
    {
        let mut n: U32 = 0;
        while (n as usize) < oSize {
            if *huffWeight.wrapping_add(n as usize) >= HUFv06_ABSOLUTEMAX_TABLELOG as BYTE {
                return ERROR(ZSTD_error_corruption_detected);
            }
            let idx = *huffWeight.wrapping_add(n as usize) as usize;
            *rankStats.wrapping_add(idx) = (*rankStats.wrapping_add(idx)).wrapping_add(1);
            weightTotal_acc =
                weightTotal_acc.wrapping_add((1u32.wrapping_shl(idx as u32)) >> 1);
            n = n.wrapping_add(1);
        }
    }
    weightTotal = weightTotal_acc;
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    {
        let tableLog: U32 = BITv06_highbit32(weightTotal) + 1;
        if tableLog > HUFv06_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *tableLogPtr = tableLog;
        /* determine last weight */
        {
            let total: U32 = 1u32.wrapping_shl(tableLog);
            let rest: U32 = total.wrapping_sub(weightTotal);
            let verif: U32 = 1u32.wrapping_shl(BITv06_highbit32(rest));
            let lastWeight: U32 = BITv06_highbit32(rest).wrapping_add(1);
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected); /* must be a clean power of 2 */
            }
            *huffWeight.wrapping_add(oSize) = lastWeight as BYTE;
            *rankStats.wrapping_add(lastWeight as usize) =
                (*rankStats.wrapping_add(lastWeight as usize)).wrapping_add(1);
        }
    }

    /* check tree construction validity */
    if (*rankStats.wrapping_add(1) < 2) || ((*rankStats.wrapping_add(1) & 1) != 0) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

/* *******************************************************
 *  HUF : Huffman block decompression
 *********************************************************/
#[repr(C)]
#[derive(Copy, Clone)]
struct HUFv06_DEltX2 {
    byte: BYTE,
    nbBits: BYTE,
} /* single-symbol decoding */

#[repr(C)]
#[derive(Copy, Clone)]
struct HUFv06_DEltX4 {
    sequence: U16,
    nbBits: BYTE,
    length: BYTE,
} /* double-symbols decoding */

#[repr(C)]
#[derive(Copy, Clone)]
struct sortedSymbol_t {
    symbol: BYTE,
    weight: BYTE,
}

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_readDTableX2(
    DTable: *mut U16,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut huffWeight: [BYTE; HUFv06_MAX_SYMBOL_VALUE + 1] = [0; HUFv06_MAX_SYMBOL_VALUE + 1];
    let mut rankVal: [U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1];
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.wrapping_add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv06_DEltX2;

    iSize = HUFv06_readStats(
        huffWeight.as_mut_ptr(),
        HUFv06_MAX_SYMBOL_VALUE + 1,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv06_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > *DTable as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable is too small */
    }
    *DTable = tableLog as U16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n < tableLog + 1 {
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
        let mut D = HUFv06_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = (tableLog + 1).wrapping_sub(w) as BYTE;
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

unsafe fn HUFv06_decodeSymbolX2(
    Dstream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BITv06_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.wrapping_add(val)).byte;
    BITv06_skipBits(Dstream, (*dt.wrapping_add(val)).nbBits as U32);
    c
}

/* MEM_64bits() == 1 and HUFv06_MAX_TABLELOG <= 12, so
 * HUFv06_DECODE_SYMBOLX2_0/_1/_2 all expand to one decode. */
macro_rules! HUFv06_DECODE_SYMBOLX2_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUFv06_decodeSymbolX2($DStreamPtr, $dt, $dtLog);
        $ptr = $ptr.wrapping_add(1);
    }};
}

unsafe fn HUFv06_decodeStreamX2(
    p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;
    let mut p = p;

    /* up to 4 symbols at a time */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(4))
    {
        HUFv06_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished) && (p < pEnd) {
        HUFv06_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        HUFv06_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    (pEnd as usize).wrapping_sub(pStart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    let op = dst as *mut BYTE;
    let oend = op.wrapping_add(dstSize);
    let dtLog: U32 = *DTable as U32;
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv06_DEltX2).wrapping_add(1);
    let mut bitD = BITv06_DStream_t::new();
    let bitDp: *mut BITv06_DStream_t = &mut bitD;

    {
        let errorCode = BITv06_initDStream(bitDp, cSrc, cSrcSize);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv06_decodeStreamX2(op, bitDp, oend, dt, dtLog);

    /* check */
    if BITv06_endOfDStream(bitDp) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let mut DTable: [U16; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let errorCode = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    HUFv06_decompress1X2_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    /* Check */
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum */
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv06_DEltX2).wrapping_add(1);
        let dtLog: U32 = *DTable as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BITv06_DStream_t::new();
        let mut bitD2 = BITv06_DStream_t::new();
        let mut bitD3 = BITv06_DStream_t::new();
        let mut bitD4 = BITv06_DStream_t::new();
        let b1: *mut BITv06_DStream_t = &mut bitD1;
        let b2: *mut BITv06_DStream_t = &mut bitD2;
        let b3: *mut BITv06_DStream_t = &mut bitD3;
        let b4: *mut BITv06_DStream_t = &mut bitD4;
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

        length4 = cSrcSize
            .wrapping_sub(length1.wrapping_add(length2).wrapping_add(length3).wrapping_add(6));
        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        errorCode = BITv06_initDStream(b1, istart1 as *const c_void, length1);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(b2, istart2 as *const c_void, length2);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(b3, istart3 as *const c_void, length3);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(b4, istart4 as *const c_void, length4);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv06_reloadDStream(b1)
            | BITv06_reloadDStream(b2)
            | BITv06_reloadDStream(b3)
            | BITv06_reloadDStream(b4);
        while (endSignal == BITv06_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            HUFv06_DECODE_SYMBOLX2_0!(op1, b1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op2, b2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op3, b3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op4, b4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op1, b1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op2, b2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op3, b3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op4, b4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op1, b1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op2, b2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op3, b3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op4, b4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op1, b1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op2, b2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op3, b3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0!(op4, b4, dt, dtLog);
            endSignal = BITv06_reloadDStream(b1)
                | BITv06_reloadDStream(b2)
                | BITv06_reloadDStream(b3)
                | BITv06_reloadDStream(b4);
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
        HUFv06_decodeStreamX2(op1, b1, opStart2, dt, dtLog);
        HUFv06_decodeStreamX2(op2, b2, opStart3, dt, dtLog);
        HUFv06_decodeStreamX2(op3, b3, opStart4, dt, dtLog);
        HUFv06_decodeStreamX2(op4, b4, oend, dt, dtLog);

        /* check */
        endSignal = BITv06_endOfDStream(b1)
            & BITv06_endOfDStream(b2)
            & BITv06_endOfDStream(b3)
            & BITv06_endOfDStream(b4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
        dstSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let mut DTable: [U16; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let errorCode = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    HUFv06_decompress4X2_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
}

/* *************************/
/* double-symbols decoding */
/* *************************/

unsafe fn HUFv06_fillDTableX4Level2(
    DTable: *mut HUFv06_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: i32,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    let mut DElt = HUFv06_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1];

    /* get pre-calculated rankVal */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: U32 = 0;
        let skipSize: U32 = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1;
        while i < skipSize {
            *DTable.wrapping_add(i as usize) = DElt;
            i = i.wrapping_add(1);
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
}

type rankVal_t = [[U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1];
    HUFv06_ABSOLUTEMAX_TABLELOG as usize];

unsafe fn HUFv06_fillDTableX4(
    DTable: *mut HUFv06_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const [U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1];
    let scaleLog: i32 = nbBitsBaseline.wrapping_sub(targetLog) as i32; /* note : targetLog >= srcLog */
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1]>(),
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
            let mut minWeight: i32 = nbBits.wrapping_add(scaleLog as U32) as i32;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.wrapping_add(minWeight as usize);
            HUFv06_fillDTableX4Level2(
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
            let mut DElt = HUFv06_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };
            MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1;
            {
                let mut u: U32;
                let end: U32 = start.wrapping_add(length);
                u = start;
                while u < end {
                    *DTable.wrapping_add(u as usize) = DElt;
                    u = u.wrapping_add(1);
                }
            }
        }
        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s = s.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_readDTableX4(
    DTable: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightList: [BYTE; HUFv06_MAX_SYMBOL_VALUE + 1] = [0; HUFv06_MAX_SYMBOL_VALUE + 1];
    let mut sortedSymbol: [sortedSymbol_t; HUFv06_MAX_SYMBOL_VALUE + 1] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; HUFv06_MAX_SYMBOL_VALUE + 1];
    let mut rankStats: [U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1];
    let mut rankStart0: [U32; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 2] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 2];
    let rankStart: *mut U32 = rankStart0.as_mut_ptr().wrapping_add(1);
    let mut rankVal: rankVal_t =
        [[0; HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1]; HUFv06_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable;
    let iSize: usize;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUFv06_DEltX4).wrapping_add(1);

    if memLog > HUFv06_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUFv06_readStats(
        weightList.as_mut_ptr(),
        HUFv06_MAX_SYMBOL_VALUE + 1,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv06_isError(iSize) != 0 {
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
        let mut w: U32 = 1;
        let mut nextRankStart: U32 = 0;
        while w < maxW + 1 {
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankStats[w as usize]);
            *rankStart.wrapping_add(w as usize) = current;
            w = w.wrapping_add(1);
        }
        *rankStart.wrapping_add(0) = nextRankStart; /* put all 0w symbols at the end */
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
        *rankStart.wrapping_add(0) = 0; /* forget 0w symbols */
    }

    /* Build rankVal */
    {
        {
            let rescale: i32 = (memLog.wrapping_sub(tableLog).wrapping_sub(1)) as i32;
            let mut nextRankVal: U32 = 0;
            let mut w: U32 = 1;
            while w < maxW + 1 {
                let current: U32 = nextRankVal;
                nextRankVal = nextRankVal.wrapping_add(
                    rankStats[w as usize].wrapping_shl(w.wrapping_add(rescale as U32)),
                );
                rankVal[0][w as usize] = current;
                w = w.wrapping_add(1);
            }
        }
        {
            let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
            let mut consumed: U32 = minBits;
            while consumed < memLog.wrapping_sub(minBits).wrapping_add(1) {
                let mut w: U32 = 1;
                while w < maxW + 1 {
                    rankVal[consumed as usize][w as usize] =
                        rankVal[0][w as usize].wrapping_shr(consumed);
                    w = w.wrapping_add(1);
                }
                consumed = consumed.wrapping_add(1);
            }
        }
    }

    HUFv06_fillDTableX4(
        dt,
        memLog,
        sortedSymbol.as_ptr(),
        sizeOfSort,
        rankStart0.as_ptr(),
        rankVal.as_ptr(),
        maxW,
        tableLog + 1,
    );

    iSize
}

unsafe fn HUFv06_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv06_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 2);
    BITv06_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    (*dt.wrapping_add(val)).length as U32
}

unsafe fn HUFv06_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv06_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 1);
    if (*dt.wrapping_add(val)).length == 1 {
        BITv06_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (core::mem::size_of::<usize>() * 8) as u32 {
            BITv06_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (core::mem::size_of::<usize>() * 8) as u32 {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as u32;
            }
        }
    }
    1
}

/* MEM_64bits() == 1 and HUFv06_MAX_TABLELOG <= 12 : _0/_1/_2 identical */
macro_rules! HUFv06_DECODE_SYMBOLX4_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.wrapping_add(HUFv06_decodeSymbolX4(
            $ptr as *mut c_void,
            $DStreamPtr,
            $dt,
            $dtLog,
        ) as usize);
    }};
}

unsafe fn HUFv06_decodeStreamX4(
    p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;
    let mut p = p;

    /* up to 8 symbols at a time */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p < pEnd.wrapping_sub(7))
    {
        HUFv06_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(2))
    {
        HUFv06_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p <= pEnd.wrapping_sub(2) {
        HUFv06_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog); /* no need to reload */
    }

    if p < pEnd {
        p = p.wrapping_add(
            HUFv06_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
        );
    }

    (p as usize).wrapping_sub(pStart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U32,
) -> usize {
    let istart = cSrc as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.wrapping_add(dstSize);

    let dtLog: U32 = *DTable;
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv06_DEltX4).wrapping_add(1);

    /* Init */
    let mut bitD = BITv06_DStream_t::new();
    let bitDp: *mut BITv06_DStream_t = &mut bitD;
    {
        let errorCode = BITv06_initDStream(bitDp, istart as *const c_void, cSrcSize);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    /* decode */
    HUFv06_decodeStreamX4(ostart, bitDp, oend, dt, dtLog);

    /* check */
    if BITv06_endOfDStream(bitDp) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* decoded size */
    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let mut DTable: [U32; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let hSize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv06_decompress1X4_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U32,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum */
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv06_DEltX4).wrapping_add(1);
        let dtLog: U32 = *DTable;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BITv06_DStream_t::new();
        let mut bitD2 = BITv06_DStream_t::new();
        let mut bitD3 = BITv06_DStream_t::new();
        let mut bitD4 = BITv06_DStream_t::new();
        let b1: *mut BITv06_DStream_t = &mut bitD1;
        let b2: *mut BITv06_DStream_t = &mut bitD2;
        let b3: *mut BITv06_DStream_t = &mut bitD3;
        let b4: *mut BITv06_DStream_t = &mut bitD4;
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

        length4 = cSrcSize
            .wrapping_sub(length1.wrapping_add(length2).wrapping_add(length3).wrapping_add(6));
        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        errorCode = BITv06_initDStream(b1, istart1 as *const c_void, length1);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(b2, istart2 as *const c_void, length2);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(b3, istart3 as *const c_void, length3);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(b4, istart4 as *const c_void, length4);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv06_reloadDStream(b1)
            | BITv06_reloadDStream(b2)
            | BITv06_reloadDStream(b3)
            | BITv06_reloadDStream(b4);
        while (endSignal == BITv06_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            HUFv06_DECODE_SYMBOLX4_0!(op1, b1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op2, b2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op3, b3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op4, b4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op1, b1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op2, b2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op3, b3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op4, b4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op1, b1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op2, b2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op3, b3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op4, b4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op1, b1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op2, b2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op3, b3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0!(op4, b4, dt, dtLog);

            endSignal = BITv06_reloadDStream(b1)
                | BITv06_reloadDStream(b2)
                | BITv06_reloadDStream(b3)
                | BITv06_reloadDStream(b4);
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
        HUFv06_decodeStreamX4(op1, b1, opStart2, dt, dtLog);
        HUFv06_decodeStreamX4(op2, b2, opStart3, dt, dtLog);
        HUFv06_decodeStreamX4(op3, b3, opStart4, dt, dtLog);
        HUFv06_decodeStreamX4(op4, b4, oend, dt, dtLog);

        /* check */
        endSignal = BITv06_endOfDStream(b1)
            & BITv06_endOfDStream(b2)
            & BITv06_endOfDStream(b3)
            & BITv06_endOfDStream(b4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
        dstSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let mut DTable: [U32; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let hSize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv06_decompress4X4_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
}

/* ********************************/
/* Generic decompression selector */
/* ********************************/

#[repr(C)]
#[derive(Copy, Clone)]
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
    [at(0, 0), at(1, 1), at(2, 2)],                 /* Q==0 : impossible */
    [at(0, 0), at(1, 1), at(2, 2)],                 /* Q==1 : impossible */
    [at(38, 130), at(1313, 74), at(2151, 38)],      /* Q == 2 : 12-18% */
    [at(448, 128), at(1353, 74), at(2238, 41)],     /* Q == 3 : 18-25% */
    [at(556, 128), at(1353, 74), at(2238, 47)],     /* Q == 4 : 25-32% */
    [at(714, 128), at(1418, 74), at(2436, 53)],     /* Q == 5 : 32-38% */
    [at(883, 128), at(1437, 74), at(2464, 61)],     /* Q == 6 : 38-44% */
    [at(897, 128), at(1515, 75), at(2622, 68)],     /* Q == 7 : 44-50% */
    [at(926, 128), at(1613, 75), at(2730, 75)],     /* Q == 8 : 50-56% */
    [at(947, 128), at(1729, 77), at(3359, 77)],     /* Q == 9 : 56-62% */
    [at(1107, 128), at(2083, 81), at(4006, 84)],    /* Q ==10 : 62-69% */
    [at(1177, 128), at(2379, 87), at(4785, 88)],    /* Q ==11 : 69-75% */
    [at(1242, 128), at(2415, 93), at(5155, 84)],    /* Q ==12 : 75-81% */
    [at(1349, 128), at(2644, 106), at(5260, 106)],  /* Q ==13 : 81-87% */
    [at(1455, 128), at(2422, 124), at(4174, 124)],  /* Q ==14 : 87-93% */
    [at(722, 128), at(1891, 145), at(1936, 146)],   /* Q ==15 : 93-99% */
];

type decompressionAlgo =
    Option<unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize>;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    static decompress: [decompressionAlgo; 3] = [
        Some(HUFv06_decompress4X2),
        Some(HUFv06_decompress4X4),
        None,
    ];
    let mut Dtime: [U32; 3] = [0; 3]; /* decompression time estimation */

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
        memset(dst, *(cSrc as *const BYTE) as i32, dstSize);
        return dstSize;
    } /* RLE */

    /* decoder timing evaluation */
    {
        let Q: U32 = (cSrcSize * 16 / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
        let D256: U32 = (dstSize >> 8) as U32;
        let mut n: U32 = 0;
        while n < 3 {
            Dtime[n as usize] = algoTime[Q as usize][n as usize].tableTime.wrapping_add(
                algoTime[Q as usize][n as usize]
                    .decode256Time
                    .wrapping_mul(D256),
            );
            n = n.wrapping_add(1);
        }
    }

    Dtime[1] = Dtime[1].wrapping_add(Dtime[1] >> 4);
    Dtime[2] = Dtime[2].wrapping_add(Dtime[2] >> 3);

    {
        let mut algoNb: U32 = 0;
        if Dtime[1] < Dtime[0] {
            algoNb = 1;
        }
        /* if (Dtime[2] < Dtime[algoNb]) algoNb = 2; */
        return (decompress[algoNb as usize].unwrap())(dst, dstSize, cSrc, cSrcSize);
    }
}

/* ******************************************************************
 *  zstd_common (v06)
 ********************************************************************/

/* `ZSTDv06_isError()` : tells if a return value is an error code */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/* `ZSTDv06_getErrorName()` */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
 *  ZBUFF Error Management
 ****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_isError(errorCode: usize) -> u32 {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

/* ******************************************************************
 *  zstd decompression (v06)
 ********************************************************************/

/*_*******************************************************
 *  Memory operations
 **********************************************************/
#[inline(always)]
unsafe fn ZSTDv06_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

/*-*************************************************************
 *   Context management
 ***************************************************************/
pub type ZSTDv06_dStage = u32;
const ZSTDds_getFrameHeaderSize: ZSTDv06_dStage = 0;
const ZSTDds_decodeFrameHeader: ZSTDv06_dStage = 1;
const ZSTDds_decodeBlockHeader: ZSTDv06_dStage = 2;
const ZSTDds_decompressBlock: ZSTDv06_dStage = 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTDv06_frameParams {
    pub frameContentSize: u64,
    pub windowLog: u32,
}

#[repr(C)]
pub struct ZSTDv06_DCtx {
    LLTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(LLFSELog)],
    OffTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(OffFSELog)],
    MLTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(MLFSELog)],
    hufTableX4: [u32; HUFv06_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    previousDstEnd: *const c_void,
    base: *const c_void,
    vBase: *const c_void,
    dictEnd: *const c_void,
    expected: usize,
    headerSize: usize,
    fParams: ZSTDv06_frameParams,
    bType: blockType_t, /* used in ZSTDv06_decompressContinue() */
    stage: ZSTDv06_dStage,
    flagRepeatTable: U32,
    litPtr: *const BYTE,
    litSize: usize,
    litBuffer: [BYTE; ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH],
    headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_sizeofDCtx() -> usize {
    core::mem::size_of::<ZSTDv06_DCtx>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBegin(dctx: *mut ZSTDv06_DCtx) -> usize {
    (*dctx).expected = ZSTDv06_frameHeaderSize_min;
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).hufTableX4[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG;
    (*dctx).flagRepeatTable = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_createDCtx() -> *mut ZSTDv06_DCtx {
    let dctx = malloc(core::mem::size_of::<ZSTDv06_DCtx>()) as *mut ZSTDv06_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDv06_decompressBegin(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_freeDCtx(dctx: *mut ZSTDv06_DCtx) -> usize {
    free(dctx as *mut c_void);
    0 /* reserved as a potential error code in the future */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_copyDCtx(
    dstDCtx: *mut ZSTDv06_DCtx,
    srcDCtx: *const ZSTDv06_DCtx,
) {
    memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv06_DCtx>()
            - (ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH + ZSTDv06_frameHeaderSize_max),
    ); /* no need to copy workspace */
}

/*-*************************************************************
 *   Decompression section
 ***************************************************************/

/// `ZSTDv06_frameHeaderSize()` : srcSize must be >= ZSTDv06_frameHeaderSize_min.
unsafe fn ZSTDv06_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    if srcSize < ZSTDv06_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let fcsId: U32 = (*(src as *const BYTE).wrapping_add(4) as U32) >> 6;
        ZSTDv06_frameHeaderSize_min + ZSTDv06_fcs_fieldSize[fcsId as usize]
    }
}

/// `ZSTDv06_getFrameParams()` : decode Frame Header, or provide expected `srcSize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_getFrameParams(
    fparamsPtr: *mut ZSTDv06_frameParams,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip = src as *const BYTE;

    if srcSize < ZSTDv06_frameHeaderSize_min {
        return ZSTDv06_frameHeaderSize_min;
    }
    if MEM_readLE32(src) != ZSTDv06_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }

    /* ensure there is enough `srcSize` to fully read/decode frame header */
    {
        let fhsize = ZSTDv06_frameHeaderSize(src, srcSize);
        if srcSize < fhsize {
            return fhsize;
        }
    }

    memset(
        fparamsPtr as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv06_frameParams>(),
    );
    {
        let frameDesc: BYTE = *ip.wrapping_add(4);
        (*fparamsPtr).windowLog = ((frameDesc & 0xF) as u32) + ZSTDv06_WINDOWLOG_ABSOLUTEMIN;
        if (frameDesc & 0x20) != 0 {
            return ERROR(ZSTD_error_frameParameter_unsupported); /* reserved 1 bit */
        }
        match frameDesc >> 6 {
            /* fcsId */
            1 => {
                (*fparamsPtr).frameContentSize = *ip.wrapping_add(5) as u64;
            }
            2 => {
                (*fparamsPtr).frameContentSize =
                    MEM_readLE16(ip.wrapping_add(5) as *const c_void) as u64 + 256;
            }
            3 => {
                (*fparamsPtr).frameContentSize =
                    MEM_readLE64(ip.wrapping_add(5) as *const c_void);
            }
            _ => {
                /* default / case 0 */
                (*fparamsPtr).frameContentSize = 0;
            }
        }
    }
    0
}

/// `ZSTDv06_decodeFrameHeader()`
unsafe fn ZSTDv06_decodeFrameHeader(
    zc: *mut ZSTDv06_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result = ZSTDv06_getFrameParams(&mut (*zc).fParams, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).fParams.windowLog > 25) {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    result
}

#[repr(C)]
#[derive(Copy, Clone)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

/* `ZSTDv06_getcBlockSize()` : size of compressed block from block header `src` */
unsafe fn ZSTDv06_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_ = src as *const BYTE;
    let cSize: U32;

    if srcSize < ZSTDv06_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (*bpPtr).blockType = ((*in_) >> 6) as blockType_t;
    cSize = (*in_.wrapping_add(2) as U32)
        .wrapping_add((*in_.wrapping_add(1) as U32) << 8)
        .wrapping_add(((*in_ & 7) as U32) << 16);
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as usize
}

unsafe fn ZSTDv06_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if dst.is_null() {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    memcpy(dst, src, srcSize);
    srcSize
}

/* `ZSTDv06_decodeLiteralsBlock()` : `@return` nb of bytes read from src (< srcSize) */
unsafe fn ZSTDv06_decodeLiteralsBlock(
    dctx: *mut ZSTDv06_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;

    /* any compressed block with literals segment must be at least this size */
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    let sel: u32 = (*istart >> 6) as u32;
    if sel == IS_HUF {
        let litSize: usize;
        let litCSize: usize;
        let mut singleStream: usize = 0;
        let mut lhSize: U32 = (((*istart) >> 4) & 3) as U32;
        if srcSize < 5 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if lhSize == 2 {
            /* 2 - 2 - 14 - 14 */
            lhSize = 4;
            litSize = (((*istart & 15) as usize) << 10)
                + ((*istart.wrapping_add(1) as usize) << 2)
                + ((*istart.wrapping_add(2) as usize) >> 6);
            litCSize = (((*istart.wrapping_add(2) & 63) as usize) << 8)
                + (*istart.wrapping_add(3) as usize);
        } else if lhSize == 3 {
            /* 2 - 2 - 18 - 18 */
            lhSize = 5;
            litSize = (((*istart & 15) as usize) << 14)
                + ((*istart.wrapping_add(1) as usize) << 6)
                + ((*istart.wrapping_add(2) as usize) >> 2);
            litCSize = (((*istart.wrapping_add(2) & 3) as usize) << 16)
                + ((*istart.wrapping_add(3) as usize) << 8)
                + (*istart.wrapping_add(4) as usize);
        } else {
            /* case 0, 1, default : 2 - 2 - 10 - 10 */
            lhSize = 3;
            singleStream = (*istart & 16) as usize;
            litSize = (((*istart & 15) as usize) << 6)
                + ((*istart.wrapping_add(1) as usize) >> 2);
            litCSize = (((*istart.wrapping_add(1) & 3) as usize) << 8)
                + (*istart.wrapping_add(2) as usize);
        }
        if litSize > ZSTDv06_BLOCKSIZE_MAX {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if litCSize + lhSize as usize > srcSize {
            return ERROR(ZSTD_error_corruption_detected);
        }

        let hufResult = if singleStream != 0 {
            HUFv06_decompress1X2(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                litSize,
                istart.wrapping_add(lhSize as usize) as *const c_void,
                litCSize,
            )
        } else {
            HUFv06_decompress(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                litSize,
                istart.wrapping_add(lhSize as usize) as *const c_void,
                litCSize,
            )
        };
        if HUFv06_isError(hufResult) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
        (*dctx).litSize = litSize;
        memset(
            (*dctx)
                .litBuffer
                .as_mut_ptr()
                .wrapping_add((*dctx).litSize) as *mut c_void,
            0,
            WILDCOPY_OVERLENGTH,
        );
        return litCSize + lhSize as usize;
    } else if sel == IS_PCH {
        let litSize: usize;
        let litCSize: usize;
        let mut lhSize: U32 = (((*istart) >> 4) & 3) as U32;
        if lhSize != 1 {
            /* only case supported for now : small litSize, single stream */
            return ERROR(ZSTD_error_corruption_detected);
        }
        if (*dctx).flagRepeatTable == 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }

        /* 2 - 2 - 10 - 10 */
        lhSize = 3;
        litSize =
            (((*istart & 15) as usize) << 6) + ((*istart.wrapping_add(1) as usize) >> 2);
        litCSize = (((*istart.wrapping_add(1) & 3) as usize) << 8)
            + (*istart.wrapping_add(2) as usize);
        if litCSize + lhSize as usize > srcSize {
            return ERROR(ZSTD_error_corruption_detected);
        }

        {
            let errorCode = HUFv06_decompress1X4_usingDTable(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                litSize,
                istart.wrapping_add(lhSize as usize) as *const c_void,
                litCSize,
                (*dctx).hufTableX4.as_ptr(),
            );
            if HUFv06_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }
        (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
        (*dctx).litSize = litSize;
        memset(
            (*dctx)
                .litBuffer
                .as_mut_ptr()
                .wrapping_add((*dctx).litSize) as *mut c_void,
            0,
            WILDCOPY_OVERLENGTH,
        );
        return litCSize + lhSize as usize;
    } else if sel == IS_RAW {
        let litSize: usize;
        let mut lhSize: U32 = (((*istart) >> 4) & 3) as U32;
        if lhSize == 2 {
            litSize =
                (((*istart & 15) as usize) << 8) + (*istart.wrapping_add(1) as usize);
        } else if lhSize == 3 {
            litSize = (((*istart & 15) as usize) << 16)
                + ((*istart.wrapping_add(1) as usize) << 8)
                + (*istart.wrapping_add(2) as usize);
        } else {
            lhSize = 1;
            litSize = (*istart & 31) as usize;
        }

        if lhSize as usize + litSize + WILDCOPY_OVERLENGTH > srcSize {
            /* risk reading beyond src buffer with wildcopy */
            if litSize + lhSize as usize > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memcpy(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                istart.wrapping_add(lhSize as usize) as *const c_void,
                litSize,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx)
                    .litBuffer
                    .as_mut_ptr()
                    .wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            return lhSize as usize + litSize;
        }
        /* direct reference into compressed stream */
        (*dctx).litPtr = istart.wrapping_add(lhSize as usize);
        (*dctx).litSize = litSize;
        return lhSize as usize + litSize;
    } else if sel == IS_RLE {
        let litSize: usize;
        let mut lhSize: U32 = (((*istart) >> 4) & 3) as U32;
        if lhSize == 2 {
            litSize =
                (((*istart & 15) as usize) << 8) + (*istart.wrapping_add(1) as usize);
        } else if lhSize == 3 {
            litSize = (((*istart & 15) as usize) << 16)
                + ((*istart.wrapping_add(1) as usize) << 8)
                + (*istart.wrapping_add(2) as usize);
            if srcSize < 4 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        } else {
            lhSize = 1;
            litSize = (*istart & 31) as usize;
        }
        if litSize > ZSTDv06_BLOCKSIZE_MAX {
            return ERROR(ZSTD_error_corruption_detected);
        }
        memset(
            (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
            *istart.wrapping_add(lhSize as usize) as i32,
            litSize + WILDCOPY_OVERLENGTH,
        );
        (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
        (*dctx).litSize = litSize;
        return lhSize as usize + 1;
    }
    ERROR(ZSTD_error_corruption_detected) /* impossible */
}

/* `ZSTDv06_buildSeqTable()` */
unsafe fn ZSTDv06_buildSeqTable(
    DTable: *mut FSEv06_DTable,
    type_: U32,
    max: U32,
    maxLog: U32,
    src: *const c_void,
    srcSize: usize,
    defaultNorm: *const S16,
    defaultLog: U32,
    flagRepeatTable: U32,
) -> usize {
    let mut max = max;
    if type_ == FSEv06_ENCODING_RLE {
        if srcSize == 0 {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if (*(src as *const BYTE) as U32) > max {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSEv06_buildDTable_rle(DTable, *(src as *const BYTE)); /* if *src > max, data is corrupted */
        1
    } else if type_ == FSEv06_ENCODING_RAW {
        FSEv06_buildDTable(DTable, defaultNorm, max, defaultLog);
        0
    } else if type_ == FSEv06_ENCODING_STATIC {
        if flagRepeatTable == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        0
    } else {
        /* default / FSEv06_ENCODING_DYNAMIC */
        let mut tableLog: U32 = 0;
        let mut norm: [S16; MaxSeq + 1] = [0; MaxSeq + 1];
        let headerSize =
            FSEv06_readNCount(norm.as_mut_ptr(), &mut max, &mut tableLog, src, srcSize);
        if ERR_isError(headerSize) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if tableLog > maxLog {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSEv06_buildDTable(DTable, norm.as_ptr(), max, tableLog);
        headerSize
    }
}

unsafe fn ZSTDv06_decodeSeqHeaders(
    nbSeqPtr: *mut i32,
    DTableLL: *mut FSEv06_DTable,
    DTableML: *mut FSEv06_DTable,
    DTableOffb: *mut FSEv06_DTable,
    flagRepeatTable: U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;
    let iend = istart.wrapping_add(srcSize);
    let mut ip = istart;

    /* check */
    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    {
        let mut nbSeq: i32 = *ip as i32;
        ip = ip.wrapping_add(1);
        if nbSeq == 0 {
            *nbSeqPtr = 0;
            return 1;
        }
        if nbSeq > 0x7F {
            if nbSeq == 0xFF {
                if ip.wrapping_add(2) > iend {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = (MEM_readLE16(ip as *const c_void) as i32).wrapping_add(LONGNBSEQ);
                ip = ip.wrapping_add(2);
            } else {
                if ip >= iend {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = ((nbSeq - 0x80) << 8) + (*ip as i32);
                ip = ip.wrapping_add(1);
            }
        }
        *nbSeqPtr = nbSeq;
    }

    /* FSE table descriptors */
    if ip.wrapping_add(4) > iend {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let LLtype: U32 = (*ip >> 6) as U32;
        let Offtype: U32 = ((*ip >> 4) & 3) as U32;
        let MLtype: U32 = ((*ip >> 2) & 3) as U32;
        ip = ip.wrapping_add(1);

        /* Build DTables */
        {
            let bhSize = ZSTDv06_buildSeqTable(
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
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
        {
            let bhSize = ZSTDv06_buildSeqTable(
                DTableOffb,
                Offtype,
                MaxOff as U32,
                OffFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                flagRepeatTable,
            );
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
        {
            let bhSize = ZSTDv06_buildSeqTable(
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
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
    }

    (ip as usize).wrapping_sub(istart as usize)
}

#[repr(C)]
#[derive(Copy, Clone)]
struct seq_t {
    litLength: usize,
    matchLength: usize,
    offset: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct seqState_t {
    DStream: BITv06_DStream_t,
    stateLL: FSEv06_DState_t,
    stateOffb: FSEv06_DState_t,
    stateML: FSEv06_DState_t,
    prevOffset: [usize; ZSTDv06_REP_INIT],
}

static LL_base: [U32; MaxLL + 1] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

static ML_base: [U32; MaxML + 1] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 34, 36, 38, 40, 44, 48, 56, 64, 80, 96, 0x80, 0x100, 0x200, 0x400,
    0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

static OF_base: [U32; MaxOff + 1] = [
    0, 1, 3, 7, 0xF, 0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF, 0x3FFF, 0x7FFF,
    0xFFFF, 0x1FFFF, 0x3FFFF, 0x7FFFF, 0xFFFFF, 0x1FFFFF, 0x3FFFFF, 0x7FFFFF, 0xFFFFFF, 0x1FFFFFF,
    0x3FFFFFF, /*fake*/ 1, 1,
];

unsafe fn ZSTDv06_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    /* Literal length */
    let llCode: U32 = FSEv06_peekSymbol(&(*seqState).stateLL) as U32;
    let mlCode: U32 = FSEv06_peekSymbol(&(*seqState).stateML) as U32;
    let ofCode: U32 = FSEv06_peekSymbol(&(*seqState).stateOffb) as U32; /* <= maxOff */

    let llBits: U32 = LL_bits[llCode as usize];
    let mlBits: U32 = ML_bits[mlCode as usize];
    let ofBits: U32 = ofCode;
    let totalBits: U32 = llBits.wrapping_add(mlBits).wrapping_add(ofBits);

    let dsp: *mut BITv06_DStream_t = &mut (*seqState).DStream;

    /* sequence */
    {
        let mut offset: usize;
        if ofCode == 0 {
            offset = 0;
        } else {
            offset = (OF_base[ofCode as usize] as usize)
                .wrapping_add(BITv06_readBits(dsp, ofBits)); /* <= 26 bits */
            if MEM_32bits() != 0 {
                BITv06_reloadDStream(dsp);
            }
        }

        if offset < ZSTDv06_REP_NUM {
            if llCode == 0 && offset <= 1 {
                offset = 1usize.wrapping_sub(offset);
            }

            if offset != 0 {
                let temp = (*seqState).prevOffset[offset];
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
            offset = offset.wrapping_sub(ZSTDv06_REP_MOVE);
            (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
            (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
            (*seqState).prevOffset[0] = offset;
        }
        (*seq).offset = offset;
    }

    (*seq).matchLength = (ML_base[mlCode as usize] as usize)
        .wrapping_add(MINMATCH)
        .wrapping_add(if mlCode > 31 {
            BITv06_readBits(dsp, mlBits)
        } else {
            0
        }); /* <= 16 bits */
    if (MEM_32bits() != 0) && (mlBits.wrapping_add(llBits) > 24) {
        BITv06_reloadDStream(dsp);
    }

    (*seq).litLength = (LL_base[llCode as usize] as usize).wrapping_add(if llCode > 15 {
        BITv06_readBits(dsp, llBits)
    } else {
        0
    }); /* <= 16 bits */
    if (MEM_32bits() != 0)
        || (totalBits > 64 - 7 - (LLFSELog + MLFSELog + OffFSELog))
    {
        BITv06_reloadDStream(dsp);
    }

    /* ANS state update */
    FSEv06_updateState(&mut (*seqState).stateLL, dsp); /* <= 9 bits */
    FSEv06_updateState(&mut (*seqState).stateML, dsp); /* <= 9 bits */
    if MEM_32bits() != 0 {
        BITv06_reloadDStream(dsp); /* <= 18 bits */
    }
    FSEv06_updateState(&mut (*seqState).stateOffb, dsp); /* <= 8 bits */
}

static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */

unsafe fn ZSTDv06_execSequence(
    op_in: *mut BYTE,
    oend: *mut BYTE,
    sequence_in: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let mut op = op_in;
    let mut sequence = sequence_in;
    let oLitEnd = op.wrapping_add(sequence.litLength);
    let sequenceLength = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd = op.wrapping_add(sequenceLength); /* risk : address space overflow (32-bits) */
    let oend_8 = oend.wrapping_sub(8);
    let iLitEnd = (*litPtr).wrapping_add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* checks */
    let seqLength = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > (oend as usize).wrapping_sub(op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize).wrapping_sub(*litPtr as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Now we know there are no overflow in literal nor match lengths */
    if oLitEnd > oend_8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    if oMatchEnd > oend {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* overwrite beyond dst buffer */
    }
    if iLitEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected); /* overRead beyond lit buffer */
    }

    /* copy Literals */
    ZSTDv06_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* copy Match */
    if sequence.offset > (oLitEnd as usize).wrapping_sub(base as usize) {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as usize).wrapping_sub(vBase as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_sub((base as usize).wrapping_sub(match_ as usize));
        if match_.wrapping_add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = (dictEnd as usize).wrapping_sub(match_ as usize);
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.wrapping_add(length1);
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            match_ = base;
            if op > oend_8 || sequence.matchLength < MINMATCH {
                while op < oMatchEnd {
                    *op = *match_;
                    op = op.wrapping_add(1);
                    match_ = match_.wrapping_add(1);
                }
                return sequenceLength;
            }
        }
    }
    /* Requirement: op <= oend_8 */

    /* match within prefix */
    if sequence.offset < 8 {
        /* close range match, overlap */
        let sub2: i32 = dec64table[sequence.offset];
        *op.wrapping_add(0) = *match_.wrapping_add(0);
        *op.wrapping_add(1) = *match_.wrapping_add(1);
        *op.wrapping_add(2) = *match_.wrapping_add(2);
        *op.wrapping_add(3) = *match_.wrapping_add(3);
        match_ = match_.wrapping_add(dec32table[sequence.offset] as usize);
        ZSTDv06_copy4(op.wrapping_add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.wrapping_offset(-(sub2 as isize));
    } else {
        ZSTDv06_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.wrapping_add(8);
    match_ = match_.wrapping_add(8);

    if oMatchEnd > oend.wrapping_sub(16 - MINMATCH) {
        if op < oend_8 {
            let diff = (oend_8 as usize).wrapping_sub(op as usize);
            ZSTDv06_wildcopy(op as *mut c_void, match_ as *const c_void, diff as isize);
            match_ = match_.wrapping_add(diff);
            op = oend_8;
        }
        while op < oMatchEnd {
            *op = *match_;
            op = op.wrapping_add(1);
            match_ = match_.wrapping_add(1);
        }
    } else {
        ZSTDv06_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            (sequence.matchLength as isize).wrapping_sub(8),
        ); /* works even if matchLength < 8 */
    }
    sequenceLength
}

unsafe fn ZSTDv06_decompressSequences(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
) -> usize {
    let mut ip = seqStart as *const BYTE;
    let iend = ip.wrapping_add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = ostart.wrapping_add(maxDstSize);
    let mut op = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd = litPtr.wrapping_add((*dctx).litSize);
    let DTableLL: *mut FSEv06_DTable = (*dctx).LLTable.as_mut_ptr();
    let DTableML: *mut FSEv06_DTable = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb: *mut FSEv06_DTable = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: i32 = 0;

    /* Build Decoding Tables */
    {
        let seqHSize = ZSTDv06_decodeSeqHeaders(
            &mut nbSeq,
            DTableLL,
            DTableML,
            DTableOffb,
            (*dctx).flagRepeatTable,
            ip as *const c_void,
            seqSize,
        );
        if ERR_isError(seqHSize) != 0 {
            return seqHSize;
        }
        ip = ip.wrapping_add(seqHSize);
        (*dctx).flagRepeatTable = 0;
    }

    /* Regen sequences */
    if nbSeq != 0 {
        let mut sequence = seq_t {
            litLength: 0,
            matchLength: 0,
            offset: 0,
        };
        let mut seqState = seqState_t {
            DStream: BITv06_DStream_t::new(),
            stateLL: FSEv06_DState_t::new(),
            stateOffb: FSEv06_DState_t::new(),
            stateML: FSEv06_DState_t::new(),
            prevOffset: [0; ZSTDv06_REP_INIT],
        };

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        sequence.offset = REPCODE_STARTVALUE;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTDv06_REP_INIT {
                seqState.prevOffset[i as usize] = REPCODE_STARTVALUE;
                i = i.wrapping_add(1);
            }
        }
        {
            let errorCode = BITv06_initDStream(
                &mut seqState.DStream,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }
        FSEv06_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSEv06_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSEv06_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BITv06_reloadDStream(&mut seqState.DStream) <= BITv06_DStream_completed)
            && nbSeq != 0
        {
            nbSeq -= 1;
            ZSTDv06_decodeSequence(&mut sequence, &mut seqState);

            {
                let oneSeqSize = ZSTDv06_execSequence(
                    op,
                    oend,
                    sequence,
                    &mut litPtr,
                    litEnd,
                    base,
                    vBase,
                    dictEnd,
                );
                if ERR_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.wrapping_add(oneSeqSize);
            }
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
    }

    /* last literal segment */
    {
        let lastLLSize = (litEnd as usize).wrapping_sub(litPtr as usize);
        if litPtr > litEnd {
            return ERROR(ZSTD_error_corruption_detected); /* too many literals already used */
        }
        if op.wrapping_add(lastLLSize) > oend {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if lastLLSize > 0 {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

unsafe fn ZSTDv06_checkContinuity(dctx: *mut ZSTDv06_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char).wrapping_sub(
            ((*dctx).previousDstEnd as usize).wrapping_sub((*dctx).base as usize),
        ) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

unsafe fn ZSTDv06_decompressBlock_internal(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip = src as *const BYTE;
    let mut srcSize = srcSize;

    if srcSize >= ZSTDv06_BLOCKSIZE_MAX {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals sub-block */
    {
        let litCSize = ZSTDv06_decodeLiteralsBlock(dctx, src, srcSize);
        if ERR_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.wrapping_add(litCSize);
        srcSize -= litCSize;
    }
    ZSTDv06_decompressSequences(dctx, dst, dstCapacity, ip as *const c_void, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBlock(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv06_checkContinuity(dctx, dst);
    ZSTDv06_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize)
}

/* `ZSTDv06_decompressFrame()` : `dctx` must be properly initialized */
unsafe fn ZSTDv06_decompressFrame(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend = ip.wrapping_add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.wrapping_add(dstCapacity);
    let mut remainingSize = srcSize;
    let mut blockProperties = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };

    /* check */
    if srcSize < ZSTDv06_frameHeaderSize_min + ZSTDv06_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Frame Header */
    {
        let frameHeaderSize = ZSTDv06_frameHeaderSize(src, ZSTDv06_frameHeaderSize_min);
        if ERR_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if ZSTDv06_decodeFrameHeader(dctx, src, frameHeaderSize) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    /* Loop on each block */
    loop {
        let mut decodedSize: usize = 0;
        let cBlockSize = ZSTDv06_getcBlockSize(
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
            &mut blockProperties,
        );
        if ERR_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.wrapping_add(ZSTDv06_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv06_blockHeaderSize);
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        if blockProperties.blockType == bt_compressed {
            decodedSize = ZSTDv06_decompressBlock_internal(
                dctx,
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                ip as *const c_void,
                cBlockSize,
            );
        } else if blockProperties.blockType == bt_raw {
            decodedSize = ZSTDv06_copyRawBlock(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                ip as *const c_void,
                cBlockSize,
            );
        } else if blockProperties.blockType == bt_rle {
            return ERROR(ZSTD_error_GENERIC); /* not yet supported */
        } else if blockProperties.blockType == bt_end {
            /* end of frame */
            if remainingSize != 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
        } else {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
        if cBlockSize == 0 {
            break; /* bt_end */
        }

        if ERR_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.wrapping_add(decodedSize);
        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    (op as usize).wrapping_sub(ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv06_DCtx,
    refDCtx: *const ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv06_copyDCtx(dctx, refDCtx);
    ZSTDv06_checkContinuity(dctx, dst);
    ZSTDv06_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompress_usingDict(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTDv06_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv06_checkContinuity(dctx, dst);
    ZSTDv06_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressDCtx(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv06_decompress_usingDict(
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
pub unsafe extern "C" fn ZSTDv06_decompress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* ZSTDv06_HEAPMODE == 1 */
    let regenSize: usize;
    let dctx = ZSTDv06_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv06_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv06_freeDCtx(dctx);
    regenSize
}

/* `ZSTD_errorFrameSizeInfoLegacy()` : assumes `cSize` and `dBound` are not NULL */
unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut u64, ret: usize) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;
    let mut blockProperties = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };

    /* Frame Header */
    {
        let frameHeaderSize = ZSTDv06_frameHeaderSize(src, srcSize);
        if ERR_isError(frameHeaderSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, frameHeaderSize);
            return;
        }
        if MEM_readLE32(src) != ZSTDv06_MAGICNUMBER {
            ZSTD_errorFrameSizeInfoLegacy(
                cSize,
                dBound,
                ERROR(ZSTD_error_prefix_unknown),
            );
            return;
        }
        if srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    /* Loop on each block */
    loop {
        let cBlockSize =
            ZSTDv06_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ERR_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTDv06_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv06_blockHeaderSize);
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break; /* bt_end */
        }

        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = (ip as usize).wrapping_sub(src as usize);
    *dBound = (nbBlocks * ZSTDv06_BLOCKSIZE_MAX) as u64;
}

/*_******************************
 *  Streaming Decompression API
 ********************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_nextSrcSizeToDecompress(dctx: *mut ZSTDv06_DCtx) -> usize {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressContinue(
    dctx: *mut ZSTDv06_DCtx,
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
        ZSTDv06_checkContinuity(dctx, dst);
    }

    /* Decompress : frame header; part 1 */
    let mut stage: ZSTDv06_dStage = (*dctx).stage;

    if stage == ZSTDds_getFrameHeaderSize {
        if srcSize != ZSTDv06_frameHeaderSize_min {
            return ERROR(ZSTD_error_srcSize_wrong); /* impossible */
        }
        (*dctx).headerSize = ZSTDv06_frameHeaderSize(src, ZSTDv06_frameHeaderSize_min);
        if ERR_isError((*dctx).headerSize) != 0 {
            return (*dctx).headerSize;
        }
        memcpy(
            (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
            src,
            ZSTDv06_frameHeaderSize_min,
        );
        if (*dctx).headerSize > ZSTDv06_frameHeaderSize_min {
            (*dctx).expected = (*dctx).headerSize - ZSTDv06_frameHeaderSize_min;
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
            (*dctx)
                .headerBuffer
                .as_mut_ptr()
                .wrapping_add(ZSTDv06_frameHeaderSize_min) as *mut c_void,
            src,
            (*dctx).expected,
        );
        result = ZSTDv06_decodeFrameHeader(
            dctx,
            (*dctx).headerBuffer.as_ptr() as *const c_void,
            (*dctx).headerSize,
        );
        if ERR_isError(result) != 0 {
            return result;
        }
        (*dctx).expected = ZSTDv06_blockHeaderSize;
        (*dctx).stage = ZSTDds_decodeBlockHeader;
        return 0;
    }

    if stage == ZSTDds_decodeBlockHeader {
        let mut bp = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let cBlockSize = ZSTDv06_getcBlockSize(src, ZSTDv06_blockHeaderSize, &mut bp);
        if ERR_isError(cBlockSize) != 0 {
            return cBlockSize;
        }
        if bp.blockType == bt_end {
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
        let bType = (*dctx).bType;
        if bType == bt_compressed {
            rSize = ZSTDv06_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
        } else if bType == bt_raw {
            rSize = ZSTDv06_copyRawBlock(dst, dstCapacity, src, srcSize);
        } else if bType == bt_rle {
            return ERROR(ZSTD_error_GENERIC); /* not yet handled */
        } else if bType == bt_end {
            /* should never happen (filtered at phase 1) */
            rSize = 0;
        } else {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
        (*dctx).stage = ZSTDds_decodeBlockHeader;
        (*dctx).expected = ZSTDv06_blockHeaderSize;
        if ERR_isError(rSize) != 0 {
            return rSize;
        }
        (*dctx).previousDstEnd = (dst as *const c_char).wrapping_add(rSize) as *const c_void;
        return rSize;
    }

    ERROR(ZSTD_error_GENERIC) /* impossible */
}

unsafe fn ZSTDv06_refDictContent(
    dctx: *mut ZSTDv06_DCtx,
    dict: *const c_void,
    dictSize: usize,
) {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const c_char).wrapping_sub(
        ((*dctx).previousDstEnd as usize).wrapping_sub((*dctx).base as usize),
    ) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd =
        (dict as *const c_char).wrapping_add(dictSize) as *const c_void;
}

unsafe fn ZSTDv06_loadEntropy(
    dctx: *mut ZSTDv06_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let hSize: usize;
    let offcodeHeaderSize: usize;
    let matchlengthHeaderSize: usize;
    let litlengthHeaderSize: usize;
    let mut dict = dict;
    let mut dictSize = dictSize;

    hSize = HUFv06_readDTableX4((*dctx).hufTableX4.as_mut_ptr(), dict, dictSize);
    if HUFv06_isError(hSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(hSize) as *const c_void;
    dictSize -= hSize;

    {
        let mut offcodeNCount: [i16; MaxOff + 1] = [0; MaxOff + 1];
        let mut offcodeMaxValue: u32 = MaxOff as u32;
        let mut offcodeLog: u32 = 0;
        offcodeHeaderSize = FSEv06_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dict,
            dictSize,
        );
        if ERR_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).OffTable.as_mut_ptr(),
                offcodeNCount.as_ptr(),
                offcodeMaxValue,
                offcodeLog,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dict = (dict as *const c_char).wrapping_add(offcodeHeaderSize) as *const c_void;
        dictSize -= offcodeHeaderSize;
    }

    {
        let mut matchlengthNCount: [i16; MaxML + 1] = [0; MaxML + 1];
        let mut matchlengthMaxValue: u32 = MaxML as u32;
        let mut matchlengthLog: u32 = 0;
        matchlengthHeaderSize = FSEv06_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dict,
            dictSize,
        );
        if ERR_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).MLTable.as_mut_ptr(),
                matchlengthNCount.as_ptr(),
                matchlengthMaxValue,
                matchlengthLog,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dict = (dict as *const c_char).wrapping_add(matchlengthHeaderSize) as *const c_void;
        dictSize -= matchlengthHeaderSize;
    }

    {
        let mut litlengthNCount: [i16; MaxLL + 1] = [0; MaxLL + 1];
        let mut litlengthMaxValue: u32 = MaxLL as u32;
        let mut litlengthLog: u32 = 0;
        litlengthHeaderSize = FSEv06_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dict,
            dictSize,
        );
        if ERR_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).LLTable.as_mut_ptr(),
                litlengthNCount.as_ptr(),
                litlengthMaxValue,
                litlengthLog,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
    }

    (*dctx).flagRepeatTable = 1;
    hSize + offcodeHeaderSize + matchlengthHeaderSize + litlengthHeaderSize
}

unsafe fn ZSTDv06_decompress_insertDictionary(
    dctx: *mut ZSTDv06_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let eSize: usize;
    let mut dict = dict;
    let mut dictSize = dictSize;
    let magic: U32 = MEM_readLE32(dict);
    if magic != ZSTDv06_DICT_MAGIC {
        /* pure content mode */
        ZSTDv06_refDictContent(dctx, dict, dictSize);
        return 0;
    }
    /* load entropy tables */
    dict = (dict as *const c_char).wrapping_add(4) as *const c_void;
    dictSize -= 4;
    eSize = ZSTDv06_loadEntropy(dctx, dict, dictSize);
    if ERR_isError(eSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    /* reference dictionary content */
    dict = (dict as *const c_char).wrapping_add(eSize) as *const c_void;
    dictSize -= eSize;
    ZSTDv06_refDictContent(dctx, dict, dictSize);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBegin_usingDict(
    dctx: *mut ZSTDv06_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    {
        let errorCode = ZSTDv06_decompressBegin(dctx);
        if ERR_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    if !dict.is_null() && dictSize != 0 {
        let errorCode = ZSTDv06_decompress_insertDictionary(dctx, dict, dictSize);
        if ERR_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }

    0
}

/* ******************************************************************
 *  Buffered version (ZBUFFv06)
 ********************************************************************/

pub type ZBUFFv06_dStage = u32;
const ZBUFFds_init: ZBUFFv06_dStage = 0;
const ZBUFFds_loadHeader: ZBUFFv06_dStage = 1;
const ZBUFFds_read: ZBUFFv06_dStage = 2;
const ZBUFFds_load: ZBUFFv06_dStage = 3;
const ZBUFFds_flush: ZBUFFv06_dStage = 4;

/* *** Resource management *** */
#[repr(C)]
pub struct ZBUFFv06_DCtx {
    zd: *mut ZSTDv06_DCtx,
    fParams: ZSTDv06_frameParams,
    stage: ZBUFFv06_dStage,
    inBuff: *mut c_char,
    inBuffSize: usize,
    inPos: usize,
    outBuff: *mut c_char,
    outBuffSize: usize,
    outStart: usize,
    outEnd: usize,
    blockSize: usize,
    headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
    lhSize: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_createDCtx() -> *mut ZBUFFv06_DCtx {
    let zbd = malloc(core::mem::size_of::<ZBUFFv06_DCtx>()) as *mut ZBUFFv06_DCtx;
    if zbd.is_null() {
        return core::ptr::null_mut();
    }
    memset(
        zbd as *mut c_void,
        0,
        core::mem::size_of::<ZBUFFv06_DCtx>(),
    );
    (*zbd).zd = ZSTDv06_createDCtx();
    if (*zbd).zd.is_null() {
        ZBUFFv06_freeDCtx(zbd); /* avoid leaking the context */
        return core::ptr::null_mut();
    }
    (*zbd).stage = ZBUFFds_init;
    zbd
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_freeDCtx(zbd: *mut ZBUFFv06_DCtx) -> usize {
    if zbd.is_null() {
        return 0; /* support free on null */
    }
    ZSTDv06_freeDCtx((*zbd).zd);
    free((*zbd).inBuff as *mut c_void);
    free((*zbd).outBuff as *mut c_void);
    free(zbd as *mut c_void);
    0
}

/* *** Initialization *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressInitDictionary(
    zbd: *mut ZBUFFv06_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*zbd).stage = ZBUFFds_loadHeader;
    (*zbd).outEnd = 0;
    (*zbd).outStart = 0;
    (*zbd).inPos = 0;
    (*zbd).lhSize = 0;
    ZSTDv06_decompressBegin_usingDict((*zbd).zd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressInit(zbd: *mut ZBUFFv06_DCtx) -> usize {
    ZBUFFv06_decompressInitDictionary(zbd, core::ptr::null(), 0)
}

#[inline(always)]
unsafe fn ZBUFFv06_limitCopy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length = if dstCapacity < srcSize {
        dstCapacity
    } else {
        srcSize
    };
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

/* *** Decompression *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressContinue(
    zbd: *mut ZBUFFv06_DCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart = src as *const c_char;
    let iend = istart.wrapping_add(*srcSizePtr);
    let mut ip = istart;
    let ostart = dst as *mut c_char;
    let oend = ostart.wrapping_add(*dstCapacityPtr);
    let mut op = ostart;
    let mut notDone: U32 = 1;

    while notDone != 0 {
        let mut cur: ZBUFFv06_dStage = (*zbd).stage;
        'sw: {
            if cur == ZBUFFds_init {
                return ERROR(ZSTD_error_init_missing);
            }

            if cur == ZBUFFds_loadHeader {
                {
                    let hSize = ZSTDv06_getFrameParams(
                        &mut (*zbd).fParams,
                        (*zbd).headerBuffer.as_ptr() as *const c_void,
                        (*zbd).lhSize,
                    );
                    if hSize != 0 {
                        let toLoad = hSize.wrapping_sub((*zbd).lhSize); /* hSize > lhSize */
                        if ERR_isError(hSize) != 0 {
                            return hSize;
                        }
                        if toLoad > (iend as usize).wrapping_sub(ip as usize) {
                            /* not enough input to load full header */
                            if !ip.is_null() {
                                memcpy(
                                    (*zbd)
                                        .headerBuffer
                                        .as_mut_ptr()
                                        .wrapping_add((*zbd).lhSize)
                                        as *mut c_void,
                                    ip as *const c_void,
                                    (iend as usize).wrapping_sub(ip as usize),
                                );
                            }
                            (*zbd).lhSize = (*zbd)
                                .lhSize
                                .wrapping_add((iend as usize).wrapping_sub(ip as usize));
                            *dstCapacityPtr = 0;
                            return hSize.wrapping_sub((*zbd).lhSize) + ZSTDv06_blockHeaderSize;
                        }
                        memcpy(
                            (*zbd).headerBuffer.as_mut_ptr().wrapping_add((*zbd).lhSize)
                                as *mut c_void,
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
                    let h1Size = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd); /* == frameHeaderSize_min */
                    let h1Result = ZSTDv06_decompressContinue(
                        (*zbd).zd,
                        core::ptr::null_mut(),
                        0,
                        (*zbd).headerBuffer.as_ptr() as *const c_void,
                        h1Size,
                    );
                    if ERR_isError(h1Result) != 0 {
                        return h1Result;
                    }
                    if h1Size < (*zbd).lhSize {
                        /* long header */
                        let h2Size = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                        let h2Result = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            core::ptr::null_mut(),
                            0,
                            (*zbd).headerBuffer.as_ptr().wrapping_add(h1Size)
                                as *const c_void,
                            h2Size,
                        );
                        if ERR_isError(h2Result) != 0 {
                            return h2Result;
                        }
                    }
                }

                /* Frame header instruct buffer sizes */
                {
                    let one_shl_wlog: usize = 1usize.wrapping_shl((*zbd).fParams.windowLog);
                    let blockSize: usize = if one_shl_wlog < ZSTDv06_BLOCKSIZE_MAX {
                        one_shl_wlog
                    } else {
                        ZSTDv06_BLOCKSIZE_MAX
                    };
                    (*zbd).blockSize = blockSize;
                    if (*zbd).inBuffSize < blockSize {
                        free((*zbd).inBuff as *mut c_void);
                        (*zbd).inBuffSize = blockSize;
                        (*zbd).inBuff = malloc(blockSize) as *mut c_char;
                        if (*zbd).inBuff.is_null() {
                            return ERROR(ZSTD_error_memory_allocation);
                        }
                    }
                    {
                        let neededOutSize: usize = (1usize
                            .wrapping_shl((*zbd).fParams.windowLog))
                            .wrapping_add(blockSize)
                            .wrapping_add(WILDCOPY_OVERLENGTH * 2);
                        if (*zbd).outBuffSize < neededOutSize {
                            free((*zbd).outBuff as *mut c_void);
                            (*zbd).outBuffSize = neededOutSize;
                            (*zbd).outBuff = malloc(neededOutSize) as *mut c_char;
                            if (*zbd).outBuff.is_null() {
                                return ERROR(ZSTD_error_memory_allocation);
                            }
                        }
                    }
                }
                (*zbd).stage = ZBUFFds_read;
                /* fall-through */
                cur = ZBUFFds_read;
            }

            if cur == ZBUFFds_read {
                {
                    let neededInSize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    if neededInSize == 0 {
                        /* end of frame */
                        (*zbd).stage = ZBUFFds_init;
                        notDone = 0;
                        break 'sw;
                    }
                    if (iend as usize).wrapping_sub(ip as usize) >= neededInSize {
                        /* decode directly from src */
                        let decodedSize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.wrapping_add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize.wrapping_sub((*zbd).outStart),
                            ip as *const c_void,
                            neededInSize,
                        );
                        if ERR_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        ip = ip.wrapping_add(neededInSize);
                        if decodedSize == 0 {
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
                cur = ZBUFFds_load;
            }

            if cur == ZBUFFds_load {
                {
                    let neededInSize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    let toLoad = neededInSize.wrapping_sub((*zbd).inPos);
                    let loadedSize: usize;
                    if toLoad > (*zbd).inBuffSize.wrapping_sub((*zbd).inPos) {
                        return ERROR(ZSTD_error_corruption_detected); /* should never happen */
                    }
                    loadedSize = ZBUFFv06_limitCopy(
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
                        let decodedSize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.wrapping_add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize.wrapping_sub((*zbd).outStart),
                            (*zbd).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ERR_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        (*zbd).inPos = 0; /* input is consumed */
                        if decodedSize == 0 {
                            (*zbd).stage = ZBUFFds_read;
                            break 'sw;
                        } /* this was just a header */
                        (*zbd).outEnd = (*zbd).outStart.wrapping_add(decodedSize);
                        (*zbd).stage = ZBUFFds_flush;
                        /* ZBUFFds_flush follows */
                    }
                }
                /* fall-through */
                cur = ZBUFFds_flush;
            }

            if cur == ZBUFFds_flush {
                let toFlushSize = (*zbd).outEnd.wrapping_sub((*zbd).outStart);
                let flushedSize = ZBUFFv06_limitCopy(
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
                        (*zbd).outStart = 0;
                        (*zbd).outEnd = 0;
                    }
                    break 'sw;
                }
                /* cannot flush everything */
                notDone = 0;
                break 'sw;
            }

            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
    }

    /* result */
    *srcSizePtr = (ip as usize).wrapping_sub(istart as usize);
    *dstCapacityPtr = (op as usize).wrapping_sub(ostart as usize);
    {
        let mut nextSrcSizeHint = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
        if nextSrcSizeHint > ZSTDv06_blockHeaderSize {
            nextSrcSizeHint = nextSrcSizeHint.wrapping_add(ZSTDv06_blockHeaderSize);
        } /* get following block header too */
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbd).inPos); /* already loaded */
        nextSrcSizeHint
    }
}

/* *************************************
 *  Tool functions
 ***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_recommendedDInSize() -> usize {
    ZSTDv06_BLOCKSIZE_MAX + ZSTDv06_blockHeaderSize /* block header size */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_recommendedDOutSize() -> usize {
    ZSTDv06_BLOCKSIZE_MAX
}
