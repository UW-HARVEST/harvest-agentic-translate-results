//! Translation of `legacy/zstd_v04.c` (+ `legacy/zstd_v04.h`).
//!
//! This C file is entirely self-contained : it embeds its own private copies of
//! `mem.h`, `bitstream.h`, FSE, Huff0, the v0.4 decompressor and the v0.4
//! `ZBUFF` streaming layer.  All of those helpers were `static` in C, so they
//! are private functions here; only the 17 `ZSTDv04_*` / `ZBUFFv04_*` wrappers
//! at the bottom of the file are exported.
#![allow(dead_code)]

use core::ffi::{c_char, c_void};
use core::mem::size_of;

use crate::libc;

/* ******************************************************************
 *   mem.h
 *******************************************************************/

type BYTE = u8;
type U16 = u16;
type S16 = i16;
type U32 = u32;
type S32 = i32;
type U64 = u64;
type S64 = i64;

#[inline(always)]
fn MEM_32bits() -> u32 {
    (size_of::<*const c_void>() == 4) as u32
}

#[inline(always)]
fn MEM_64bits() -> u32 {
    (size_of::<*const c_void>() == 8) as u32
}

#[inline(always)]
fn MEM_isLittleEndian() -> u32 {
    cfg!(target_endian = "little") as u32
}

#[inline(always)]
unsafe fn MEM_read16(memPtr: *const c_void) -> U16 {
    (memPtr as *const U16).read_unaligned()
}

#[inline(always)]
unsafe fn MEM_read32(memPtr: *const c_void) -> U32 {
    (memPtr as *const U32).read_unaligned()
}

#[inline(always)]
unsafe fn MEM_read64(memPtr: *const c_void) -> U64 {
    (memPtr as *const U64).read_unaligned()
}

#[inline(always)]
unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    (memPtr as *mut U16).write_unaligned(value)
}

#[inline(always)]
unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    /* MEM_isLittleEndian() */
    MEM_read16(memPtr)
}

#[inline(always)]
unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    /* MEM_isLittleEndian() */
    MEM_write16(memPtr, val)
}

#[inline(always)]
unsafe fn MEM_readLE24(memPtr: *const c_void) -> U32 {
    (MEM_readLE16(memPtr) as U32).wrapping_add((*(memPtr as *const BYTE).add(2) as U32) << 16)
}

#[inline(always)]
unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    /* MEM_isLittleEndian() */
    MEM_read32(memPtr)
}

#[inline(always)]
unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    /* MEM_isLittleEndian() */
    MEM_read64(memPtr)
}

#[inline(always)]
unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    /* MEM_64bits() */
    MEM_readLE64(memPtr) as usize
}

/* ******************************************************************
 *   error_private.h  (ERROR / ERR_isError / ERR_getErrorName)
 *******************************************************************/

const ZSTD_error_no_error: i32 = 0;
const ZSTD_error_GENERIC: i32 = 1;
const ZSTD_error_prefix_unknown: i32 = 10;
const ZSTD_error_frameParameter_unsupported: i32 = 14;
const ZSTD_error_corruption_detected: i32 = 20;
const ZSTD_error_tableLog_tooLarge: i32 = 44;
const ZSTD_error_maxSymbolValue_tooLarge: i32 = 46;
const ZSTD_error_maxSymbolValue_tooSmall: i32 = 48;
const ZSTD_error_init_missing: i32 = 62;
const ZSTD_error_memory_allocation: i32 = 64;
const ZSTD_error_dstSize_tooSmall: i32 = 70;
const ZSTD_error_srcSize_wrong: i32 = 72;
const ZSTD_error_maxCode: i32 = 120;

extern "C" {
    /* defined in `common/error_private.c` */
    fn ERR_getErrorString(code: i32) -> *const c_char;
}

#[inline(always)]
const fn ERROR(code: i32) -> usize {
    (-(code as isize)) as usize
}

#[inline(always)]
fn ERR_isError(code: usize) -> u32 {
    (code > ERROR(ZSTD_error_maxCode)) as u32
}

#[inline(always)]
fn ERR_getErrorCode(code: usize) -> i32 {
    if ERR_isError(code) == 0 {
        return 0;
    }
    (0usize.wrapping_sub(code)) as i32
}

#[inline(always)]
unsafe fn ERR_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorString(ERR_getErrorCode(code))
}

/* *************************************
*  Common constants  (zstd_internal)
***************************************/

const ZSTD_MAGICNUMBER: U32 = 0xFD2FB524;
const ZSTD_WINDOWLOG_ABSOLUTEMIN: U32 = 11;

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

const MLbits: U32 = 7;
const LLbits: U32 = 6;
const Offbits: U32 = 5;
const MaxML: U32 = (1 << MLbits) - 1;
const MaxLL: U32 = (1 << LLbits) - 1;
const MaxOff: U32 = (1 << Offbits) - 1;
const MLFSELog: U32 = 10;
const LLFSELog: U32 = 10;
const OffFSELog: U32 = 9;

const MIN_SEQUENCES_SIZE: usize = 2 + 2 + 3 + 1;
const MIN_CBLOCK_SIZE: usize = 3 + MIN_SEQUENCES_SIZE;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

/* blockType_t : { bt_compressed, bt_raw, bt_rle, bt_end } */
type blockType_t = u32;
const bt_compressed: blockType_t = 0;
const bt_raw: blockType_t = 1;
const bt_rle: blockType_t = 2;
const bt_end: blockType_t = 3;

/* ******************************************
*  Shared functions to include for inlining
********************************************/

#[inline(always)]
unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    libc::memcpy(dst, src, 8);
}

/* ZSTD_wildcopy : custom version of memcpy(), can copy up to 7-8 bytes too many */
unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_offset(length);
    loop {
        /* COPY8(op, ip) */
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
        if !(op < oend) {
            break;
        }
    }
}

/* ******************************************************************
   bitstream
****************************************************************** */

#[repr(C)]
#[derive(Copy, Clone)]
struct BIT_DStream_t {
    bitContainer: usize,
    bitsConsumed: u32,
    ptr: *const c_char,
    start: *const c_char,
}

impl BIT_DStream_t {
    const fn new() -> BIT_DStream_t {
        BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        }
    }
}

type BIT_DStream_status = u32;
const BIT_DStream_unfinished: BIT_DStream_status = 0;
const BIT_DStream_endOfBuffer: BIT_DStream_status = 1;
const BIT_DStream_completed: BIT_DStream_status = 2;
const BIT_DStream_overflow: BIT_DStream_status = 3;

#[inline(always)]
fn BIT_highbit32(val: U32) -> u32 {
    /* __builtin_clz (val) ^ 31 */
    31u32 ^ val.leading_zeros()
}

unsafe fn BIT_initDStream(
    bitD: *mut BIT_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        libc::memset(bitD as *mut c_void, 0, size_of::<BIT_DStream_t>());
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    if srcSize >= size_of::<usize>() {
        /* normal case */
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char)
            .wrapping_add(srcSize)
            .wrapping_sub(size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        contain32 = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); /* endMark not present */
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        {
            let start = (*bitD).start as *const BYTE;
            if srcSize >= 7 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*start.wrapping_add(6) as usize) << (size_of::<usize>() * 8 - 16));
            }
            if srcSize >= 6 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*start.wrapping_add(5) as usize) << (size_of::<usize>() * 8 - 24));
            }
            if srcSize >= 5 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*start.wrapping_add(4) as usize) << (size_of::<usize>() * 8 - 32));
            }
            if srcSize >= 4 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*start.wrapping_add(3) as usize) << 24);
            }
            if srcSize >= 3 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*start.wrapping_add(2) as usize) << 16);
            }
            if srcSize >= 2 {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*start.wrapping_add(1) as usize) << 8);
            }
        }
        contain32 = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); /* endMark not present */
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
        (*bitD).bitsConsumed = (*bitD)
            .bitsConsumed
            .wrapping_add(((size_of::<usize>() - srcSize) as U32).wrapping_mul(8));
    }

    srcSize
}

#[inline(always)]
unsafe fn BIT_lookBits(bitD: *const BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer.wrapping_shl((*bitD).bitsConsumed & bitMask)) >> 1)
        .wrapping_shr(bitMask.wrapping_sub(nbBits) & bitMask)
}

/* BIT_lookBitsFast : unsafe version; only works if nbBits >= 1 */
#[inline(always)]
unsafe fn BIT_lookBitsFast(bitD: *const BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer.wrapping_shl((*bitD).bitsConsumed & bitMask))
        .wrapping_shr((bitMask.wrapping_add(1).wrapping_sub(nbBits)) & bitMask)
}

#[inline(always)]
unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

#[inline(always)]
unsafe fn BIT_readBits(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

/* BIT_readBitsFast : unsafe version; only works if nbBits >= 1 */
#[inline(always)]
unsafe fn BIT_readBitsFast(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

unsafe fn BIT_reloadDStream(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).bitsConsumed > (size_of::<usize>() * 8) as U32 {
        /* should never happen */
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(size_of::<usize>()) {
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (size_of::<usize>() * 8) as U32 {
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

/* BIT_endOfDStream : @return Tells if DStream has reached its exact end */
#[inline(always)]
unsafe fn BIT_endOfDStream(DStream: *const BIT_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (size_of::<usize>() * 8) as U32)) as u32
}

/* ******************************************************************
   FSE : Finite State Entropy coder
****************************************************************** */

const FSE_MAX_MEMORY_USAGE: u32 = 14;
const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
const FSE_MAX_SYMBOL_VALUE: usize = 255;

const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

const FSE_NCOUNTBOUND: usize = 512;

const fn FSE_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

type FSE_DTable = U32;

/* DTable_max_t */
const DTABLE_MAX_SIZE: usize = FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG);

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_DTableHeader {
    tableLog: U16,
    fastMode: U16,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_decode_t {
    newState: U16,
    symbol: BYTE,
    nbBits: BYTE,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_DState_t {
    state: usize,
    table: *const c_void, /* precise table may vary, depending on U16 */
}

impl FSE_DState_t {
    const fn new() -> FSE_DState_t {
        FSE_DState_t {
            state: 0,
            table: core::ptr::null(),
        }
    }
}

unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
    dt: *const FSE_DTable,
) {
    let mut DTableH = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    libc::memcpy(
        &mut DTableH as *mut FSE_DTableHeader as *mut c_void,
        dt as *const c_void,
        size_of::<FSE_DTableHeader>(),
    );
    (*DStatePtr).state = BIT_readBits(bitD, DTableH.tableLog as U32);
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

#[inline(always)]
unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

#[inline(always)]
unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

#[inline(always)]
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
    normalizedCounter: *const S16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    let mut DTableH = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tdPtr: *mut c_void = dt.wrapping_add(1) as *mut c_void;
    let tableDecode: *mut FSE_decode_t = tdPtr as *mut FSE_decode_t;
    let tableSize: U32 = 1u32.wrapping_shl(tableLog);
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; FSE_MAX_SYMBOL_VALUE + 1] = [0; FSE_MAX_SYMBOL_VALUE + 1];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);
    let largeLimit: S16 = (1i32.wrapping_shl(tableLog.wrapping_sub(1))) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    /* Sanity Checks */
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE as u32 {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    libc::memset(
        tableDecode as *mut c_void,
        0,
        size_of::<FSE_decode_t>() * (maxSymbolValue as usize + 1),
    );
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
            symbolNext[symbol as usize] = nextState.wrapping_add(1);
            (*tableDecode.wrapping_add(i as usize)).nbBits =
                tableLog.wrapping_sub(BIT_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.wrapping_add(i as usize)).newState = ((nextState as U32)
                .wrapping_shl((*tableDecode.wrapping_add(i as usize)).nbBits as U32))
                .wrapping_sub(tableSize) as U16;
            i = i.wrapping_add(1);
        }
    }

    DTableH.fastMode = noLarge as U16;
    libc::memcpy(
        dt as *mut c_void,
        &DTableH as *const FSE_DTableHeader as *const c_void,
        size_of::<FSE_DTableHeader>(),
    );
    0
}

/******************************************
*  FSE helper functions
******************************************/
fn FSE_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/****************************************************************
*  FSE NCount encoding-decoding
****************************************************************/
fn FSE_abs(a: S16) -> S16 {
    if a < 0 {
        a.wrapping_neg()
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
    let istart: *const BYTE = headerBuffer as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(hbSize);
    let mut ip: *const BYTE = istart;
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
    nbBits = ((bitStream & 0xF).wrapping_add(FSE_MIN_TABLELOG)) as i32; /* extract tableLog */
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as i32 {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as u32;
    remaining = (1i32.wrapping_shl(nbBits as u32)).wrapping_add(1);
    threshold = 1i32.wrapping_shl(nbBits as u32);
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
            let max: S16 = ((2i32.wrapping_mul(threshold).wrapping_sub(1)).wrapping_sub(remaining)) as S16;
            let mut count: S16;

            if (bitStream & (threshold.wrapping_sub(1)) as U32) < (max as U32) {
                count = (bitStream & (threshold.wrapping_sub(1)) as U32) as S16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2i32.wrapping_mul(threshold).wrapping_sub(1)) as U32) as S16;
                if count as i32 >= threshold {
                    count = (count as i32).wrapping_sub(max as i32) as S16;
                }
                bitCount += nbBits;
            }

            count = (count as i32).wrapping_sub(1) as S16; /* extra accuracy */
            remaining = remaining.wrapping_sub(FSE_abs(count) as i32);
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
                    bitCount = bitCount.wrapping_sub(
                        (8isize.wrapping_mul(
                            (iend as isize).wrapping_sub(4).wrapping_sub(ip as isize),
                        )) as i32,
                    );
                    ip = iend.wrapping_sub(4);
                }
                bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr((bitCount & 31) as u32);
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
unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> usize {
    let ptr: *mut c_void = dt as *mut c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let dPtr: *mut c_void = dt.wrapping_add(1) as *mut c_void;
    let cell: *mut FSE_decode_t = dPtr as *mut FSE_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: u32) -> usize {
    let ptr: *mut c_void = dt as *mut c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let dPtr: *mut c_void = dt.wrapping_add(1) as *mut c_void;
    let dinfo: *mut FSE_decode_t = dPtr as *mut FSE_decode_t;
    let tableSize: u32 = 1u32.wrapping_shl(nbBits);
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
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.wrapping_add(maxDstSize);
    let olimit: *mut BYTE = omax.wrapping_sub(3);

    let mut bitD = BIT_DStream_t::new();
    let mut state1 = FSE_DState_t::new();
    let mut state2 = FSE_DState_t::new();
    let mut errorCode: usize;

    /* Init */
    errorCode = BIT_initDStream(&mut bitD, cSrc, cSrcSize); /* replaced last arg by maxCompressed Size */
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    /* 4 symbols per loop */
    while (BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit) {
        *op.wrapping_add(0) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };

        if (FSE_MAX_TABLELOG * 2 + 7) as usize > size_of::<usize>() * 8 {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(1) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state2, &mut bitD)
        };

        if (FSE_MAX_TABLELOG * 4 + 7) as usize > size_of::<usize>() * 8 {
            /* This test must be static */
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.wrapping_add(2) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };

        if (FSE_MAX_TABLELOG * 2 + 7) as usize > size_of::<usize>() * 8 {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(3) = if fast != 0 {
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
    let fastMode: U32;

    libc::memcpy(
        &mut DTableH as *mut FSE_DTableHeader as *mut c_void,
        dt as *const c_void,
        size_of::<FSE_DTableHeader>(),
    );
    fastMode = DTableH.fastMode as U32;

    /* select fast mode (static) */
    if fastMode != 0 {
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
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut counting: [S16; FSE_MAX_SYMBOL_VALUE + 1] = [0; FSE_MAX_SYMBOL_VALUE + 1];
    let mut dt: [U32; DTABLE_MAX_SIZE] = [0; DTABLE_MAX_SIZE];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE as u32;
    let mut errorCode: usize;
    let mut cSrcSize = cSrcSize;

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
   Huff0 : Huffman coder, part of New Generation Entropy library
****************************************************************** */

const HUF_ABSOLUTEMAX_TABLELOG: usize = 16;
const HUF_MAX_TABLELOG: u32 = 12;
const HUF_DEFAULT_TABLELOG: u32 = HUF_MAX_TABLELOG;
const HUF_MAX_SYMBOL_VALUE: usize = 255;

const fn HUF_DTABLE_SIZE(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

fn HUF_isError(code: usize) -> u32 {
    ERR_isError(code)
}

#[repr(C)]
#[derive(Copy, Clone)]
struct HUF_DEltX2 {
    byte: BYTE,
    nbBits: BYTE,
} /* single-symbol decoding */

#[repr(C)]
#[derive(Copy, Clone)]
struct HUF_DEltX4 {
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

/* HUF_readStats
    Read compact Huffman tree, saved by HUF_writeCTable
    @huffWeight : destination buffer
    @return : size read from `src`
*/
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
    let mut ip: *const BYTE = src as *const BYTE;
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
            libc::memset(huffWeight as *mut c_void, 1, hwSize);
            iSize = 0;
        } else {
            /* Incompressible */
            oSize = iSize.wrapping_sub(127);
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
                *huffWeight.wrapping_add(n as usize + 1) =
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
            hwSize.wrapping_sub(1),
            ip.wrapping_add(1) as *const c_void,
            iSize,
        ); /* max (hwSize-1) values decoded, as last one is implied */
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    libc::memset(
        rankStats as *mut c_void,
        0,
        (HUF_ABSOLUTEMAX_TABLELOG + 1) * size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if *huffWeight.wrapping_add(n as usize) >= HUF_ABSOLUTEMAX_TABLELOG as BYTE {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *rankStats.wrapping_add(*huffWeight.wrapping_add(n as usize) as usize) =
            (*rankStats.wrapping_add(*huffWeight.wrapping_add(n as usize) as usize))
                .wrapping_add(1);
        weightTotal = weightTotal.wrapping_add(
            (1u32.wrapping_shl(*huffWeight.wrapping_add(n as usize) as U32)) >> 1,
        );
        n = n.wrapping_add(1);
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    tableLog = BIT_highbit32(weightTotal).wrapping_add(1);
    if tableLog > HUF_ABSOLUTEMAX_TABLELOG as U32 {
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
        *huffWeight.wrapping_add(oSize) = lastWeight as BYTE;
        *rankStats.wrapping_add(lastWeight as usize) =
            (*rankStats.wrapping_add(lastWeight as usize)).wrapping_add(1);
    }

    /* check tree construction validity */
    if (*rankStats.wrapping_add(1) < 2) || (*rankStats.wrapping_add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */
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
    let mut huffWeight: [BYTE; HUF_MAX_SYMBOL_VALUE + 1] = [0; HUF_MAX_SYMBOL_VALUE + 1];
    let mut rankVal: [U32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1]; /* large enough for values from 0 to 16 */
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr: *mut c_void = DTable.wrapping_add(1) as *mut c_void;
    let dt: *mut HUF_DEltX2 = dtPtr as *mut HUF_DEltX2;

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
    if tableLog > *DTable.wrapping_add(0) as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable is too small */
    }
    *DTable.wrapping_add(0) = tableLog as U16;

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
        let mut D = HUF_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = tableLog.wrapping_add(1).wrapping_sub(w) as BYTE;
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

#[inline(always)]
unsafe fn HUF_decodeSymbolX2(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val: usize = BIT_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.wrapping_add(val)).byte;
    BIT_skipBits(Dstream, (*dt.wrapping_add(val)).nbBits as U32);
    c
}

unsafe fn HUF_decodeStreamX2(
    p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart: *mut BYTE = p;
    let mut p: *mut BYTE = p;

    /* up to 4 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.wrapping_sub(4)) {
        /* HUF_DECODE_SYMBOLX2_2 */
        if MEM_64bits() != 0 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }
        /* HUF_DECODE_SYMBOLX2_1 */
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }
        /* HUF_DECODE_SYMBOLX2_2 */
        if MEM_64bits() != 0 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }
        /* HUF_DECODE_SYMBOLX2_0 */
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.wrapping_add(1);
    }

    /* closer to the end */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd) {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.wrapping_add(1);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.wrapping_add(1);
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
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let dtPtr: *const c_void = DTable as *const c_void;
        let dt: *const HUF_DEltX2 = (dtPtr as *const HUF_DEltX2).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0) as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BIT_DStream_t::new();
        let mut bitD2 = BIT_DStream_t::new();
        let mut bitD3 = BIT_DStream_t::new();
        let mut bitD4 = BIT_DStream_t::new();
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
            if MEM_64bits() != 0 {
                *op1 = HUF_decodeSymbolX2(&mut bitD1, dt, dtLog);
                op1 = op1.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op2 = HUF_decodeSymbolX2(&mut bitD2, dt, dtLog);
                op2 = op2.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op3 = HUF_decodeSymbolX2(&mut bitD3, dt, dtLog);
                op3 = op3.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op4 = HUF_decodeSymbolX2(&mut bitD4, dt, dtLog);
                op4 = op4.wrapping_add(1);
            }
            if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                *op1 = HUF_decodeSymbolX2(&mut bitD1, dt, dtLog);
                op1 = op1.wrapping_add(1);
            }
            if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                *op2 = HUF_decodeSymbolX2(&mut bitD2, dt, dtLog);
                op2 = op2.wrapping_add(1);
            }
            if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                *op3 = HUF_decodeSymbolX2(&mut bitD3, dt, dtLog);
                op3 = op3.wrapping_add(1);
            }
            if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                *op4 = HUF_decodeSymbolX2(&mut bitD4, dt, dtLog);
                op4 = op4.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op1 = HUF_decodeSymbolX2(&mut bitD1, dt, dtLog);
                op1 = op1.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op2 = HUF_decodeSymbolX2(&mut bitD2, dt, dtLog);
                op2 = op2.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op3 = HUF_decodeSymbolX2(&mut bitD3, dt, dtLog);
                op3 = op3.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op4 = HUF_decodeSymbolX2(&mut bitD4, dt, dtLog);
                op4 = op4.wrapping_add(1);
            }
            *op1 = HUF_decodeSymbolX2(&mut bitD1, dt, dtLog);
            op1 = op1.wrapping_add(1);
            *op2 = HUF_decodeSymbolX2(&mut bitD2, dt, dtLog);
            op2 = op2.wrapping_add(1);
            *op3 = HUF_decodeSymbolX2(&mut bitD3, dt, dtLog);
            op3 = op3.wrapping_add(1);
            *op4 = HUF_decodeSymbolX2(&mut bitD4, dt, dtLog);
            op4 = op4.wrapping_add(1);

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
    cSrcSize: usize,
) -> usize {
    let mut DTable: [U16; HUF_DTABLE_SIZE(HUF_MAX_TABLELOG)] =
        [0; HUF_DTABLE_SIZE(HUF_MAX_TABLELOG)];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let errorCode: usize;
    let mut cSrcSize = cSrcSize;

    errorCode = HUF_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

    HUF_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
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
    let mut rankVal: [U32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1];
    let mut s: U32;

    /* get pre-calculated rankVal */
    libc::memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        size_of::<[U32; HUF_ABSOLUTEMAX_TABLELOG + 1]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: U32;
        let skipSize: U32 = rankVal[minWeight as usize];
        MEM_writeLE16(
            &mut DElt.sequence as *mut U16 as *mut c_void,
            baseSeq,
        );
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

/* rankVal_t : U32[HUF_ABSOLUTEMAX_TABLELOG][HUF_ABSOLUTEMAX_TABLELOG + 1] */
const RANKVAL_ROW: usize = HUF_ABSOLUTEMAX_TABLELOG + 1;

unsafe fn HUF_fillDTableX4(
    DTable: *mut HUF_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const U32,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1];
    let scaleLog: i32 = nbBitsBaseline.wrapping_sub(targetLog) as i32; /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    libc::memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        size_of::<[U32; HUF_ABSOLUTEMAX_TABLELOG + 1]>(),
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
            HUF_fillDTableX4Level2(
                DTable.wrapping_add(start as usize),
                targetLog.wrapping_sub(nbBits),
                nbBits,
                rankValOrigin.wrapping_add(nbBits as usize * RANKVAL_ROW),
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
    let mut weightList: [BYTE; HUF_MAX_SYMBOL_VALUE + 1] = [0; HUF_MAX_SYMBOL_VALUE + 1];
    let mut sortedSymbol: [sortedSymbol_t; HUF_MAX_SYMBOL_VALUE + 1] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; HUF_MAX_SYMBOL_VALUE + 1];
    let mut rankStats: [U32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1];
    let mut rankStart0: [U32; HUF_ABSOLUTEMAX_TABLELOG + 2] = [0; HUF_ABSOLUTEMAX_TABLELOG + 2];
    let rankStart: *mut U32 = rankStart0.as_mut_ptr().wrapping_add(1);
    let mut rankVal: [[U32; RANKVAL_ROW]; HUF_ABSOLUTEMAX_TABLELOG] =
        [[0; RANKVAL_ROW]; HUF_ABSOLUTEMAX_TABLELOG];
    let rankValBase: *mut U32 = rankVal.as_mut_ptr() as *mut U32;
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.wrapping_add(0);
    let iSize: usize;
    let dtPtr: *mut c_void = DTable as *mut c_void;
    let dt: *mut HUF_DEltX4 = (dtPtr as *mut HUF_DEltX4).wrapping_add(1);

    if memLog > HUF_ABSOLUTEMAX_TABLELOG as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge);
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
            *rankStart.wrapping_add(w as usize) = current;
            w = w.wrapping_add(1);
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
            *rankStart.wrapping_add(w as usize) = r.wrapping_add(1);
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s = s.wrapping_add(1);
        }
        *rankStart.wrapping_add(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: i32 = (memLog.wrapping_sub(tableLog)).wrapping_sub(1) as i32; /* tableLog <= memLog */
        let rankVal0: *mut U32 = rankValBase;
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankVal;
            nextRankVal = nextRankVal.wrapping_add(
                rankStats[w as usize].wrapping_shl(w.wrapping_add(rescale as U32)),
            );
            *rankVal0.wrapping_add(w as usize) = current;
            w = w.wrapping_add(1);
        }
        consumed = minBits;
        while consumed <= memLog.wrapping_sub(minBits) {
            let rankValPtr: *mut U32 = rankValBase.wrapping_add(consumed as usize * RANKVAL_ROW);
            w = 1;
            while w <= maxW {
                *rankValPtr.wrapping_add(w as usize) =
                    (*rankVal0.wrapping_add(w as usize)).wrapping_shr(consumed);
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
        rankValBase as *const U32,
        maxW,
        tableLog.wrapping_add(1),
    );

    iSize
}

#[inline(always)]
unsafe fn HUF_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    libc::memcpy(op, dt.wrapping_add(val) as *const c_void, 2);
    BIT_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    (*dt.wrapping_add(val)).length as U32
}

#[inline(always)]
unsafe fn HUF_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    libc::memcpy(op, dt.wrapping_add(val) as *const c_void, 1);
    if (*dt.wrapping_add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (size_of::<usize>() * 8) as U32 {
            BIT_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (size_of::<usize>() * 8) as U32 {
                (*DStream).bitsConsumed = (size_of::<usize>() * 8) as U32; /* ugly hack; works only because it's the last symbol */
            }
        }
    }
    1
}

unsafe fn HUF_decodeStreamX4(
    p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart: *mut BYTE = p;
    let mut p: *mut BYTE = p;

    /* up to 8 symbols at a time */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd.wrapping_sub(7)) {
        if MEM_64bits() != 0 {
            p = p.wrapping_add(
                HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
            );
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
            p = p.wrapping_add(
                HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
            );
        }
        if MEM_64bits() != 0 {
            p = p.wrapping_add(
                HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
            );
        }
        p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    /* closer to the end */
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(2))
    {
        p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    while p <= pEnd.wrapping_sub(2) {
        p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        /* no need to reload : reached the end of DStream */
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
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let dtPtr: *const c_void = DTable as *const c_void;
        let dt: *const HUF_DEltX4 = (dtPtr as *const HUF_DEltX4).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0);
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BIT_DStream_t::new();
        let mut bitD2 = BIT_DStream_t::new();
        let mut bitD3 = BIT_DStream_t::new();
        let mut bitD4 = BIT_DStream_t::new();
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
            if MEM_64bits() != 0 {
                op1 = op1.wrapping_add(
                    HUF_decodeSymbolX4(op1 as *mut c_void, &mut bitD1, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 {
                op2 = op2.wrapping_add(
                    HUF_decodeSymbolX4(op2 as *mut c_void, &mut bitD2, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 {
                op3 = op3.wrapping_add(
                    HUF_decodeSymbolX4(op3 as *mut c_void, &mut bitD3, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 {
                op4 = op4.wrapping_add(
                    HUF_decodeSymbolX4(op4 as *mut c_void, &mut bitD4, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                op1 = op1.wrapping_add(
                    HUF_decodeSymbolX4(op1 as *mut c_void, &mut bitD1, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                op2 = op2.wrapping_add(
                    HUF_decodeSymbolX4(op2 as *mut c_void, &mut bitD2, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                op3 = op3.wrapping_add(
                    HUF_decodeSymbolX4(op3 as *mut c_void, &mut bitD3, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                op4 = op4.wrapping_add(
                    HUF_decodeSymbolX4(op4 as *mut c_void, &mut bitD4, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 {
                op1 = op1.wrapping_add(
                    HUF_decodeSymbolX4(op1 as *mut c_void, &mut bitD1, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 {
                op2 = op2.wrapping_add(
                    HUF_decodeSymbolX4(op2 as *mut c_void, &mut bitD2, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 {
                op3 = op3.wrapping_add(
                    HUF_decodeSymbolX4(op3 as *mut c_void, &mut bitD3, dt, dtLog) as usize,
                );
            }
            if MEM_64bits() != 0 {
                op4 = op4.wrapping_add(
                    HUF_decodeSymbolX4(op4 as *mut c_void, &mut bitD4, dt, dtLog) as usize,
                );
            }
            op1 = op1.wrapping_add(
                HUF_decodeSymbolX4(op1 as *mut c_void, &mut bitD1, dt, dtLog) as usize,
            );
            op2 = op2.wrapping_add(
                HUF_decodeSymbolX4(op2 as *mut c_void, &mut bitD2, dt, dtLog) as usize,
            );
            op3 = op3.wrapping_add(
                HUF_decodeSymbolX4(op3 as *mut c_void, &mut bitD3, dt, dtLog) as usize,
            );
            op4 = op4.wrapping_add(
                HUF_decodeSymbolX4(op4 as *mut c_void, &mut bitD4, dt, dtLog) as usize,
            );

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
    cSrcSize: usize,
) -> usize {
    let mut DTable: [U32; HUF_DTABLE_SIZE(HUF_MAX_TABLELOG)] =
        [0; HUF_DTABLE_SIZE(HUF_MAX_TABLELOG)];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let hSize: usize = HUF_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize = cSrcSize.wrapping_sub(hSize);

    HUF_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/**********************************/
/* Generic decompression selector */
/**********************************/

#[repr(C)]
#[derive(Copy, Clone)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
}

const fn AT(tableTime: U32, decode256Time: U32) -> algo_time_t {
    algo_time_t {
        tableTime,
        decode256Time,
    }
}

static algoTime: [[algo_time_t; 3]; 16] = [
    /* single, double, quad */
    [AT(0, 0), AT(1, 1), AT(2, 2)],             /* Q==0 : impossible */
    [AT(0, 0), AT(1, 1), AT(2, 2)],             /* Q==1 : impossible */
    [AT(38, 130), AT(1313, 74), AT(2151, 38)],  /* Q == 2 : 12-18% */
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

unsafe fn HUF_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* static const decompressionAlgo decompress[3] = { HUF_decompress4X2, HUF_decompress4X4, NULL }; */
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
        libc::memcpy(dst, cSrc, dstSize);
        return dstSize; /* not compressed */
    }
    if cSrcSize == 1 {
        libc::memset(dst, *(cSrc as *const BYTE) as libc::c_int, dstSize);
        return dstSize; /* RLE */
    }

    /* decoder timing evaluation */
    Q = (cSrcSize.wrapping_mul(16) / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
    n = 0;
    while n < 3 {
        Dtime[n as usize] = algoTime[Q as usize][n as usize]
            .tableTime
            .wrapping_add(
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

    if algoNb == 1 {
        return HUF_decompress4X4(dst, dstSize, cSrc, cSrcSize);
    }
    HUF_decompress4X2(dst, dstSize, cSrc, cSrcSize)
}

/*
    zstd - decompression module for v0.4 legacy format
*/

/* *************************************
*  Types  (zstd_static.h)
***************************************/

/* ZSTD_strategy : { ZSTD_fast, ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2, ZSTD_btlazy2 } */
type ZSTD_strategy = u32;
const ZSTD_fast: ZSTD_strategy = 0;
const ZSTD_greedy: ZSTD_strategy = 1;
const ZSTD_lazy: ZSTD_strategy = 2;
const ZSTD_lazy2: ZSTD_strategy = 3;
const ZSTD_btlazy2: ZSTD_strategy = 4;

#[repr(C)]
#[derive(Copy, Clone)]
struct ZSTD_parameters {
    srcSize: U64,
    windowLog: U32,
    contentLog: U32,
    hashLog: U32,
    searchLog: U32,
    searchLength: U32,
    strategy: ZSTD_strategy,
}

impl ZSTD_parameters {
    const fn new() -> ZSTD_parameters {
        ZSTD_parameters {
            srcSize: 0,
            windowLog: 0,
            contentLog: 0,
            hashLog: 0,
            searchLog: 0,
            searchLength: 0,
            strategy: 0,
        }
    }
}

/* *************************************
*  Local types
***************************************/
#[repr(C)]
#[derive(Copy, Clone)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

/* *******************************************************
*  Memory operations
**********************************************************/
#[inline(always)]
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    libc::memcpy(dst, src, 4);
}

/* *************************************
*  Error Management
***************************************/
/* ZSTD_isError : tells if a return value is an error code */
fn ZSTD_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/* *************************************************************
*   Context management
***************************************************************/
/* ZSTD_dStage */
type ZSTD_dStage = u32;
const ZSTDds_getFrameHeaderSize: ZSTD_dStage = 0;
const ZSTDds_decodeFrameHeader: ZSTD_dStage = 1;
const ZSTDds_decodeBlockHeader: ZSTD_dStage = 2;
const ZSTDds_decompressBlock: ZSTD_dStage = 3;

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
    litBuffer: [BYTE; BLOCKSIZE + 8 /* margin for wildcopy */],
    headerBuffer: [BYTE; ZSTD_frameHeaderSize_max],
}

pub type ZSTDv04_Dctx = ZSTDv04_Dctx_s;
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
    let dctx: *mut ZSTD_DCtx = libc::malloc(size_of::<ZSTD_DCtx>()) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_resetDCtx(dctx);
    dctx
}

unsafe fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    libc::free(dctx as *mut c_void);
    0
}

/* *************************************************************
*   Decompression section
***************************************************************/
/** ZSTD_decodeFrameHeader_Part1
*   decode the 1st part of the Frame Header, which tells Frame Header size.
*   srcSize must be == ZSTD_frameHeaderSize_min
*   @return : the full size of the Frame Header */
unsafe fn ZSTD_decodeFrameHeader_Part1(
    zc: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let magicNumber: U32;
    if srcSize != ZSTD_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    (*zc).headerSize = ZSTD_frameHeaderSize_min;
    (*zc).headerSize
}

unsafe fn ZSTD_getFrameParams(
    params: *mut ZSTD_parameters,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let magicNumber: U32;
    if srcSize < ZSTD_frameHeaderSize_min {
        return ZSTD_frameHeaderSize_max;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    libc::memset(params as *mut c_void, 0, size_of::<ZSTD_parameters>());
    (*params).windowLog =
        ((*(src as *const BYTE).wrapping_add(4) & 15) as U32).wrapping_add(ZSTD_WINDOWLOG_ABSOLUTEMIN);
    if (*(src as *const BYTE).wrapping_add(4) >> 4) != 0 {
        return ERROR(ZSTD_error_frameParameter_unsupported); /* reserved bits */
    }
    0
}

/** ZSTD_decodeFrameHeader_Part2
*   decode the full Frame Header
*   srcSize must be the size provided by ZSTD_decodeFrameHeader_Part1
*   @return : 0, or an error code, which can be tested using ZSTD_isError() */
unsafe fn ZSTD_decodeFrameHeader_Part2(
    zc: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result: usize;
    if srcSize != (*zc).headerSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    result = ZSTD_getFrameParams(&mut (*zc).params, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).params.windowLog > 25) {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    result
}

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

unsafe fn ZSTD_copyRawBlock(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0 {
        libc::memcpy(dst, src, srcSize);
    }
    srcSize
}

/** ZSTD_decompressLiterals
    @return : nb of bytes read from src, or an error code*/
unsafe fn ZSTD_decompressLiterals(
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip: *const BYTE = src as *const BYTE;

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
unsafe fn ZSTD_decodeLiteralsBlock(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;

    /* any compressed block with literals segment must be at least this size */
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart as u32) & 3 {
        /* compressed */
        0 => {
            let mut litSize: usize = BLOCKSIZE;
            let readSize: usize = ZSTD_decompressLiterals(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                &mut litSize,
                src,
                srcSize,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            libc::memset(
                (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                8,
            );
            readSize /* works if it's an error too */
        }
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
                libc::memcpy(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    istart as *const c_void,
                    litSize,
                );
                (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
                (*dctx).litSize = litSize;
                libc::memset(
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
            libc::memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.wrapping_add(3) as libc::c_int,
                litSize.wrapping_add(8),
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            4
        }
        _ => ERROR(ZSTD_error_corruption_detected), /* forbidden nominal case */
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
    let mut ip: *const BYTE = istart;
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
        return ERROR(ZSTD_error_srcSize_wrong); /* min : all 3 are "raw", hence no header */
    }

    /* sequences */
    {
        let mut norm: [S16; MaxML as usize + 1] = [0; MaxML as usize + 1]; /* assumption : MaxML >= MaxLL >= MaxOff */
        let mut headerSize: usize;

        /* Build DTables */
        if LLtype == bt_rle {
            LLlog = 0;
            let v: BYTE = *ip;
            ip = ip.wrapping_add(1);
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
            let v: BYTE = *ip;
            ip = ip.wrapping_add(1);
            FSE_buildDTable_rle(DTableOffb, v & MaxOff as BYTE); /* if *ip > MaxOff, data is corrupted */
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
            let v: BYTE = *ip;
            ip = ip.wrapping_add(1);
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
#[derive(Copy, Clone)]
struct seq_t {
    litLength: usize,
    offset: usize,
    matchLength: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct seqState_t {
    DStream: BIT_DStream_t,
    stateLL: FSE_DState_t,
    stateOffb: FSE_DState_t,
    stateML: FSE_DState_t,
    prevOffset: usize,
    dumps: *const BYTE,
    dumpsEnd: *const BYTE,
}

impl seqState_t {
    const fn new() -> seqState_t {
        seqState_t {
            DStream: BIT_DStream_t::new(),
            stateLL: FSE_DState_t::new(),
            stateOffb: FSE_DState_t::new(),
            stateML: FSE_DState_t::new(),
            prevOffset: 0,
            dumps: core::ptr::null(),
            dumpsEnd: core::ptr::null(),
        }
    }
}

unsafe fn ZSTD_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let mut litLength: usize;
    let prevOffset: usize;
    let mut offset: usize;
    let mut matchLength: usize;
    let mut dumps: *const BYTE = (*seqState).dumps;
    let de: *const BYTE = (*seqState).dumpsEnd;

    /* Literal length */
    litLength = FSE_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream) as usize;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
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
            dumps = de.wrapping_sub(1);
        } /* late correction, to avoid read overflow (data is now corrupted anyway) */
    }

    /* Offset */
    {
        static offsetPrefix: [U32; MaxOff as usize + 1] = [
            1 /*fake*/, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384,
            32768, 65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216,
            33554432, /*fake*/ 1, 1, 1, 1, 1,
        ];
        let offsetCode: U32;
        let mut nbBits: U32;
        offsetCode =
            FSE_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream) as U32; /* <= maxOff, by table construction */
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        nbBits = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0; /* cmove */
        }
        offset = (*offsetPrefix.as_ptr().wrapping_add(offsetCode as usize) as usize)
            .wrapping_add(BIT_readBits(&mut (*seqState).DStream, nbBits));
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset; /* cmove */
        }
        if (offsetCode | ((litLength == 0) as U32)) != 0 {
            (*seqState).prevOffset = (*seq).offset; /* cmove */
        }
    }

    /* MatchLength */
    matchLength = FSE_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as usize;
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
            dumps = de.wrapping_sub(1);
        } /* late correction, to avoid read overflow (data is now corrupted anyway) */
    }
    matchLength = matchLength.wrapping_add(MINMATCH);

    /* save result */
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

unsafe fn ZSTD_execSequence(
    op: *mut BYTE,
    oend: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    static dec32table: [i32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
    static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */
    let mut sequence = sequence;
    let mut op: *mut BYTE = op;
    let oLitEnd: *mut BYTE = op.wrapping_add(sequence.litLength);
    let sequenceLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd: *mut BYTE = op.wrapping_add(sequenceLength); /* risk : address space overflow (32-bits) */
    let oend_8: *mut BYTE = oend.wrapping_sub(8);
    let litEnd: *const BYTE = (*litPtr).wrapping_add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

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
    if sequence.offset > (oLitEnd as usize).wrapping_sub(base as usize) {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as usize).wrapping_sub(vBase as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_sub((base as usize).wrapping_sub(match_ as usize));
        if match_.wrapping_add(sequence.matchLength) <= dictEnd {
            libc::memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = (dictEnd as usize).wrapping_sub(match_ as usize);
            libc::memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
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
        let sub2: i32 = *dec64table.as_ptr().wrapping_add(sequence.offset);
        *op.wrapping_add(0) = *match_.wrapping_add(0);
        *op.wrapping_add(1) = *match_.wrapping_add(1);
        *op.wrapping_add(2) = *match_.wrapping_add(2);
        *op.wrapping_add(3) = *match_.wrapping_add(3);
        match_ = match_
            .wrapping_offset(*dec32table.as_ptr().wrapping_add(sequence.offset) as isize);
        ZSTD_copy4(op.wrapping_add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.wrapping_offset(-(sub2 as isize));
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
        ); /* works even if matchLength < 8, but must be signed */
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
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(seqSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
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
    let base: *const BYTE = (*dctx).base as *const BYTE;
    let vBase: *const BYTE = (*dctx).vBase as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;

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
        let mut seqState = seqState_t::new();

        libc::memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            size_of::<seq_t>(),
        );
        sequence.offset = 4;
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.wrapping_add(dumpsLength);
        seqState.prevOffset = 4;
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

        while (BIT_reloadDStream(&mut seqState.DStream) <= BIT_DStream_completed) && (nbSeq != 0)
        {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(
                op,
                oend,
                sequence,
                &mut litPtr,
                litEnd,
                base,
                vBase,
                dictEnd,
            );
            if ZSTD_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.wrapping_add(oneSeqSize);
        }

        /* check if reached exact end */
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected); /* DStream should be entirely and exactly consumed */
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
                    libc::memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.wrapping_add(lastLLSize);
            }
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

unsafe fn ZSTD_checkContinuity(dctx: *mut ZSTD_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char).wrapping_offset(
            -(((*dctx).previousDstEnd as *const c_char as isize)
                .wrapping_sub((*dctx).base as *const c_char as isize)),
        ) as *const c_void;
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
    /* blockType == blockCompressed */
    let mut ip: *const BYTE = src as *const BYTE;
    let litCSize: usize;
    let mut srcSize = srcSize;

    if srcSize > BLOCKSIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* Decode literals sub-block */
    litCSize = ZSTD_decodeLiteralsBlock(dctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.wrapping_add(litCSize);
    srcSize = srcSize.wrapping_sub(litCSize);

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
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut remainingSize: usize = srcSize;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* init */
    ZSTD_resetDCtx(ctx);
    if !dict.is_null() {
        ZSTD_decompress_insertDictionary(ctx, dict, dictSize);
        (*ctx).dictEnd = (*ctx).previousDstEnd;
        (*ctx).vBase = (dst as *const c_char).wrapping_offset(
            -(((*ctx).previousDstEnd as *const c_char as isize)
                .wrapping_sub((*ctx).base as *const c_char as isize)),
        ) as *const c_void;
        (*ctx).base = dst;
    } else {
        (*ctx).dictEnd = dst;
        (*ctx).base = dst;
        (*ctx).vBase = dst;
    }

    /* Frame Header */
    {
        let mut frameHeaderSize: usize;
        if srcSize < ZSTD_frameHeaderSize_min + ZSTD_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        frameHeaderSize = ZSTD_decodeFrameHeader_Part1(ctx, src, ZSTD_frameHeaderSize_min);
        if ZSTD_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize.wrapping_add(ZSTD_blockHeaderSize) {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize = remainingSize.wrapping_sub(frameHeaderSize);
        frameHeaderSize = ZSTD_decodeFrameHeader_Part2(ctx, src, frameHeaderSize);
        if ZSTD_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
    }

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
            bt_compressed => {
                decodedSize = ZSTD_decompressBlock_internal(
                    ctx,
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_raw => {
                decodedSize = ZSTD_copyRawBlock(
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_rle => {
                return ERROR(ZSTD_error_GENERIC); /* not yet supported */
            }
            bt_end => {
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

/* ZSTD_errorFrameSizeInfoLegacy() :
   assumes `cSize` and `dBound` are _not_ NULL */
unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut u64, ret: usize) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

/* ******************************
*  Streaming Decompression API
********************************/
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
    ZSTD_checkContinuity(ctx, dst as *const c_void);

    let mut stage: ZSTD_dStage = (*ctx).stage;

    /* Decompress : frame header; part 1 */
    if stage == ZSTDds_getFrameHeaderSize {
        /* get frame header size */
        if srcSize != ZSTD_frameHeaderSize_min {
            return ERROR(ZSTD_error_srcSize_wrong); /* impossible */
        }
        (*ctx).headerSize = ZSTD_decodeFrameHeader_Part1(ctx, src, ZSTD_frameHeaderSize_min);
        if ZSTD_isError((*ctx).headerSize) != 0 {
            return (*ctx).headerSize;
        }
        libc::memcpy(
            (*ctx).headerBuffer.as_mut_ptr() as *mut c_void,
            src,
            ZSTD_frameHeaderSize_min,
        );
        if (*ctx).headerSize > ZSTD_frameHeaderSize_min {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
        (*ctx).expected = 0; /* not necessary to copy more */
        stage = ZSTDds_decodeFrameHeader; /* fallthrough */
    }
    if stage == ZSTDds_decodeFrameHeader {
        /* get frame header */
        let result: usize = ZSTD_decodeFrameHeader_Part2(
            ctx,
            (*ctx).headerBuffer.as_ptr() as *const c_void,
            (*ctx).headerSize,
        );
        if ZSTD_isError(result) != 0 {
            return result;
        }
        (*ctx).expected = ZSTD_blockHeaderSize;
        (*ctx).stage = ZSTDds_decodeBlockHeader;
        return 0;
    }
    if stage == ZSTDds_decodeBlockHeader {
        /* Decode block header */
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
            (*ctx).stage = ZSTDds_getFrameHeaderSize;
        } else {
            (*ctx).expected = blockSize;
            (*ctx).bType = bp.blockType;
            (*ctx).stage = ZSTDds_decompressBlock;
        }
        return 0;
    }
    if stage == ZSTDds_decompressBlock {
        /* Decompress : block content */
        let rSize: usize;
        match (*ctx).bType {
            bt_compressed => {
                rSize = ZSTD_decompressBlock_internal(ctx, dst, maxDstSize, src, srcSize);
            }
            bt_raw => {
                rSize = ZSTD_copyRawBlock(dst, maxDstSize, src, srcSize);
            }
            bt_rle => {
                return ERROR(ZSTD_error_GENERIC); /* not yet handled */
            }
            bt_end => {
                /* should never happen (filtered at phase 1) */
                rSize = 0;
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
        (*ctx).stage = ZSTDds_decodeBlockHeader;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTD_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = (dst as *const c_char).wrapping_add(rSize) as *const c_void;
        return rSize;
    }

    ERROR(ZSTD_error_GENERIC) /* impossible */
}

unsafe fn ZSTD_decompress_insertDictionary(
    ctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) {
    (*ctx).dictEnd = (*ctx).previousDstEnd;
    (*ctx).vBase = (dict as *const c_char).wrapping_offset(
        -(((*ctx).previousDstEnd as *const c_char as isize)
            .wrapping_sub((*ctx).base as *const c_char as isize)),
    ) as *const c_void;
    (*ctx).base = dict;
    (*ctx).previousDstEnd = (dict as *const c_char).wrapping_add(dictSize) as *const c_void;
}

/*
    Buffered version of Zstd compression library
*/

/* ZBUFF_dStage */
type ZBUFF_dStage = u32;
const ZBUFFds_init: ZBUFF_dStage = 0;
const ZBUFFds_readHeader: ZBUFF_dStage = 1;
const ZBUFFds_loadHeader: ZBUFF_dStage = 2;
const ZBUFFds_decodeHeader: ZBUFF_dStage = 3;
const ZBUFFds_read: ZBUFF_dStage = 4;
const ZBUFFds_load: ZBUFF_dStage = 5;
const ZBUFFds_flush: ZBUFF_dStage = 6;

/* *** Resource management *** */

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
    headerBuffer: [BYTE; ZSTD_frameHeaderSize_max],
}

pub type ZBUFFv04_DCtx = ZBUFFv04_DCtx_s;
type ZBUFF_DCtx = ZBUFFv04_DCtx_s;

unsafe fn ZBUFF_createDCtx() -> *mut ZBUFF_DCtx {
    let zbc: *mut ZBUFF_DCtx = libc::malloc(size_of::<ZBUFF_DCtx>()) as *mut ZBUFF_DCtx;
    if zbc.is_null() {
        return core::ptr::null_mut();
    }
    libc::memset(zbc as *mut c_void, 0, size_of::<ZBUFF_DCtx>());
    (*zbc).zc = ZSTD_createDCtx();
    (*zbc).stage = ZBUFFds_init;
    zbc
}

unsafe fn ZBUFF_freeDCtx(zbc: *mut ZBUFF_DCtx) -> usize {
    if zbc.is_null() {
        return 0; /* support free on null */
    }
    ZSTD_freeDCtx((*zbc).zc);
    libc::free((*zbc).inBuff as *mut c_void);
    libc::free((*zbc).outBuff as *mut c_void);
    libc::free(zbc as *mut c_void);
    0
}

/* *** Initialization *** */

unsafe fn ZBUFF_decompressInit(zbc: *mut ZBUFF_DCtx) -> usize {
    (*zbc).stage = ZBUFFds_readHeader;
    (*zbc).dictSize = 0;
    (*zbc).outEnd = 0;
    (*zbc).outStart = 0;
    (*zbc).inPos = 0;
    (*zbc).hPos = 0;
    ZSTD_resetDCtx((*zbc).zc)
}

unsafe fn ZBUFF_decompressWithDictionary(
    zbc: *mut ZBUFF_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    (*zbc).dict = src as *const c_char;
    (*zbc).dictSize = srcSize;
    0
}

unsafe fn ZBUFF_limitCopy(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length: usize = if maxDstSize < srcSize {
        maxDstSize
    } else {
        srcSize
    };
    if length > 0 {
        libc::memcpy(dst, src, length);
    }
    length
}

/* *** Decompression *** */

unsafe fn ZBUFF_decompressContinue(
    zbc: *mut ZBUFF_DCtx,
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart: *const c_char = src as *const c_char;
    let mut ip: *const c_char = istart;
    let iend: *const c_char = istart.wrapping_add(*srcSizePtr);
    let ostart: *mut c_char = dst as *mut c_char;
    let mut op: *mut c_char = ostart;
    let oend: *mut c_char = ostart.wrapping_add(*maxDstSizePtr);
    let mut notDone: U32 = 1;

    while notDone != 0 {
        let mut stage: ZBUFF_dStage = (*zbc).stage;

        'sw: {
            if stage == ZBUFFds_init {
                return ERROR(ZSTD_error_init_missing);
            }

            if stage == ZBUFFds_readHeader {
                /* read header from src */
                let headerSize: usize =
                    ZSTD_getFrameParams(&mut (*zbc).params, src, *srcSizePtr);
                if ZSTD_isError(headerSize) != 0 {
                    return headerSize;
                }
                if headerSize != 0 {
                    /* not enough input to decode header : tell how many bytes would be necessary */
                    libc::memcpy(
                        (*zbc).headerBuffer.as_mut_ptr().wrapping_add((*zbc).hPos)
                            as *mut c_void,
                        src,
                        *srcSizePtr,
                    );
                    (*zbc).hPos = (*zbc).hPos.wrapping_add(*srcSizePtr);
                    *maxDstSizePtr = 0;
                    (*zbc).stage = ZBUFFds_loadHeader;
                    return headerSize.wrapping_sub((*zbc).hPos);
                }
                (*zbc).stage = ZBUFFds_decodeHeader;
                break 'sw;
            }

            if stage == ZBUFFds_loadHeader {
                /* complete header from src */
                let mut headerSize: usize = ZBUFF_limitCopy(
                    (*zbc).headerBuffer.as_mut_ptr().wrapping_add((*zbc).hPos) as *mut c_void,
                    ZSTD_frameHeaderSize_max.wrapping_sub((*zbc).hPos),
                    src,
                    *srcSizePtr,
                );
                (*zbc).hPos = (*zbc).hPos.wrapping_add(headerSize);
                ip = ip.wrapping_add(headerSize);
                headerSize = ZSTD_getFrameParams(
                    &mut (*zbc).params,
                    (*zbc).headerBuffer.as_ptr() as *const c_void,
                    (*zbc).hPos,
                );
                if ZSTD_isError(headerSize) != 0 {
                    return headerSize;
                }
                if headerSize != 0 {
                    /* not enough input to decode header : tell how many bytes would be necessary */
                    *maxDstSizePtr = 0;
                    return headerSize.wrapping_sub((*zbc).hPos);
                }
                /* intentional fallthrough */
                stage = ZBUFFds_decodeHeader;
            }

            if stage == ZBUFFds_decodeHeader {
                /* apply header to create / resize buffers */
                {
                    let neededOutSize: usize = 1usize.wrapping_shl((*zbc).params.windowLog);
                    let neededInSize: usize = BLOCKSIZE; /* a block is never > BLOCKSIZE */
                    if (*zbc).inBuffSize < neededInSize {
                        libc::free((*zbc).inBuff as *mut c_void);
                        (*zbc).inBuffSize = neededInSize;
                        (*zbc).inBuff = libc::malloc(neededInSize) as *mut c_char;
                        if (*zbc).inBuff.is_null() {
                            return ERROR(ZSTD_error_memory_allocation);
                        }
                    }
                    if (*zbc).outBuffSize < neededOutSize {
                        libc::free((*zbc).outBuff as *mut c_void);
                        (*zbc).outBuffSize = neededOutSize;
                        (*zbc).outBuff = libc::malloc(neededOutSize) as *mut c_char;
                        if (*zbc).outBuff.is_null() {
                            return ERROR(ZSTD_error_memory_allocation);
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
                    /* some data already loaded into headerBuffer : transfer into inBuff */
                    libc::memcpy(
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
                /* fall-through */
                stage = ZBUFFds_read;
            }

            if stage == ZBUFFds_read {
                let neededInSize: usize = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
                if neededInSize == 0 {
                    /* end of frame */
                    (*zbc).stage = ZBUFFds_init;
                    notDone = 0;
                    break 'sw;
                }
                if ((iend as usize).wrapping_sub(ip as usize)) >= neededInSize {
                    /* directly decode from src */
                    let decodedSize: usize = ZSTD_decompressContinue(
                        (*zbc).zc,
                        (*zbc).outBuff.wrapping_add((*zbc).outStart) as *mut c_void,
                        (*zbc).outBuffSize.wrapping_sub((*zbc).outStart),
                        ip as *const c_void,
                        neededInSize,
                    );
                    if ZSTD_isError(decodedSize) != 0 {
                        return decodedSize;
                    }
                    ip = ip.wrapping_add(neededInSize);
                    if decodedSize == 0 {
                        break 'sw; /* this was just a header */
                    }
                    (*zbc).outEnd = (*zbc).outStart.wrapping_add(decodedSize);
                    (*zbc).stage = ZBUFFds_flush;
                    break 'sw;
                }
                if ip == iend {
                    notDone = 0;
                    break 'sw;
                } /* no more input */
                (*zbc).stage = ZBUFFds_load;
                /* fall-through */
                stage = ZBUFFds_load;
            }

            if stage == ZBUFFds_load {
                let neededInSize: usize = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
                let toLoad: usize = neededInSize.wrapping_sub((*zbc).inPos); /* should always be <= remaining space within inBuff */
                let loadedSize: usize;
                if toLoad > (*zbc).inBuffSize.wrapping_sub((*zbc).inPos) {
                    return ERROR(ZSTD_error_corruption_detected); /* should never happen */
                }
                loadedSize = ZBUFF_limitCopy(
                    (*zbc).inBuff.wrapping_add((*zbc).inPos) as *mut c_void,
                    toLoad,
                    ip as *const c_void,
                    (iend as usize).wrapping_sub(ip as usize),
                );
                ip = ip.wrapping_add(loadedSize);
                (*zbc).inPos = (*zbc).inPos.wrapping_add(loadedSize);
                if loadedSize < toLoad {
                    notDone = 0;
                    break 'sw;
                } /* not enough input, wait for more */
                {
                    let decodedSize: usize = ZSTD_decompressContinue(
                        (*zbc).zc,
                        (*zbc).outBuff.wrapping_add((*zbc).outStart) as *mut c_void,
                        (*zbc).outBuffSize.wrapping_sub((*zbc).outStart),
                        (*zbc).inBuff as *const c_void,
                        neededInSize,
                    );
                    if ZSTD_isError(decodedSize) != 0 {
                        return decodedSize;
                    }
                    (*zbc).inPos = 0; /* input is consumed */
                    if decodedSize == 0 {
                        (*zbc).stage = ZBUFFds_read;
                        break 'sw;
                    } /* this was just a header */
                    (*zbc).outEnd = (*zbc).outStart.wrapping_add(decodedSize);
                    (*zbc).stage = ZBUFFds_flush;
                    /* ZBUFFds_flush follows */
                }
                /* fall-through */
                stage = ZBUFFds_flush;
            }

            if stage == ZBUFFds_flush {
                let toFlushSize: usize = (*zbc).outEnd.wrapping_sub((*zbc).outStart);
                let flushedSize: usize = ZBUFF_limitCopy(
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    (*zbc).outBuff.wrapping_add((*zbc).outStart) as *const c_void,
                    toFlushSize,
                );
                op = op.wrapping_add(flushedSize);
                (*zbc).outStart = (*zbc).outStart.wrapping_add(flushedSize);
                if flushedSize == toFlushSize {
                    (*zbc).stage = ZBUFFds_read;
                    if (*zbc).outStart.wrapping_add(BLOCKSIZE) > (*zbc).outBuffSize {
                        (*zbc).outStart = 0;
                        (*zbc).outEnd = 0;
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

    *srcSizePtr = (ip as usize).wrapping_sub(istart as usize);
    *maxDstSizePtr = (op as usize).wrapping_sub(ostart as usize);

    {
        let mut nextSrcSizeHint: usize = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
        if nextSrcSizeHint > 3 {
            nextSrcSizeHint = nextSrcSizeHint.wrapping_add(3); /* get the next block header while at it */
        }
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbc).inPos); /* already loaded*/
        nextSrcSizeHint
    }
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_isError(errorCode: usize) -> u32 {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_recommendedDInSize() -> usize {
    BLOCKSIZE + 3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_recommendedDOutSize() -> usize {
    BLOCKSIZE
}

/*- ========================================================================= -*/

/* final wrapping stage */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_decompressDCtx(
    dctx: *mut ZSTD_DCtx,
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
    /* ZSTD_HEAPMODE == 1 */
    let regenSize: usize;
    let dctx: *mut ZSTD_DCtx = ZSTD_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv04_decompressDCtx(dctx, dst, maxDstSize, src, srcSize);
    ZSTD_freeDCtx(dctx);
    regenSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize_min {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    if MEM_readLE32(src) != ZSTD_MAGICNUMBER {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
        return;
    }
    ip = ip.wrapping_add(ZSTD_frameHeaderSize_min);
    remainingSize = remainingSize.wrapping_sub(ZSTD_frameHeaderSize_min);

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

    *cSize = (ip as usize).wrapping_sub(src as *const BYTE as usize);
    *dBound = (nbBlocks as u64).wrapping_mul(BLOCKSIZE as u64);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_resetDCtx(dctx: *mut ZSTDv04_Dctx) -> usize {
    ZSTD_resetDCtx(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_nextSrcSizeToDecompress(dctx: *mut ZSTDv04_Dctx) -> usize {
    ZSTD_nextSrcSizeToDecompress(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_decompressContinue(
    dctx: *mut ZSTDv04_Dctx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompressContinue(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_createDCtx() -> *mut ZBUFFv04_DCtx {
    ZBUFF_createDCtx()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_freeDCtx(dctx: *mut ZBUFFv04_DCtx) -> usize {
    ZBUFF_freeDCtx(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressInit(dctx: *mut ZBUFFv04_DCtx) -> usize {
    ZBUFF_decompressInit(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressWithDictionary(
    dctx: *mut ZBUFFv04_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZBUFF_decompressWithDictionary(dctx, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressContinue(
    dctx: *mut ZBUFFv04_DCtx,
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    ZBUFF_decompressContinue(dctx, dst, maxDstSizePtr, src, srcSizePtr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_createDCtx() -> *mut ZSTD_DCtx {
    ZSTD_createDCtx()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    ZSTD_freeDCtx(dctx)
}
