//! Faithful translation of `legacy/zstd_v05.c` (zstd v0.5 decoder).
//! Self-contained: defines its own internal FSE / HUF / ZBUFF code.
//! Target platform: little-endian 64-bit. `size_t` == `usize` == 8 bytes.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_variables)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::common::allocations::{free, malloc, memcpy, memmove, memset};
use crate::common::error::{code, err_get_error_name, err_is_error, error};
use crate::common::mem::{
    mem_32bits, mem_64bits, mem_read_le16, mem_read_le32, mem_read_le_st, mem_write_le16,
};

/*-*************************************
*  Basic types
***************************************/
type BYTE = u8;
type U16 = u16;
type S16 = i16;
type U32 = u32;
type S32 = i32;
type U64 = u64;

/*-*************************************
*  Error helpers
***************************************/
#[inline]
fn ERR_isError(c: usize) -> c_uint {
    err_is_error(c)
}

/*-*************************************
*  Common constants (zstd_internal.h)
***************************************/
const ZSTDv05_DICT_MAGIC: U32 = 0xEC30A435;
const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525;
const ZSTDv05_WINDOWLOG_ABSOLUTEMIN: u32 = 11;

const BLOCKSIZE: usize = 128 * 1024;

const ZSTDv05_blockHeaderSize: usize = 3;
const ZSTDv05_frameHeaderSize_min: usize = 5;
const ZSTDv05_frameHeaderSize_max: usize = 5;

const IS_HUFv05: u32 = 0;
const IS_PCH: u32 = 1;
const IS_RAW: u32 = 2;
const IS_RLE: u32 = 3;

const MINMATCH: usize = 4;
const REPCODE_STARTVALUE: usize = 1;

const MLbits: u32 = 7;
const LLbits: u32 = 6;
const Offbits: u32 = 5;
const MaxML: u32 = (1 << MLbits) - 1;
const MaxLL: u32 = (1 << LLbits) - 1;
const MaxOff: u32 = (1 << Offbits) - 1;
const MLFSEv05Log: u32 = 10;
const LLFSEv05Log: u32 = 10;
const OffFSEv05Log: u32 = 9;

const FSEv05_ENCODING_RAW: u32 = 0;
const FSEv05_ENCODING_RLE: u32 = 1;
const FSEv05_ENCODING_STATIC: u32 = 2;
const FSEv05_ENCODING_DYNAMIC: u32 = 3;

const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

const MIN_SEQUENCES_SIZE: usize = 1;
const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

const WILDCOPY_OVERLENGTH: usize = 8;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

/* blockType_t */
const bt_compressed: u32 = 0;
const bt_raw: u32 = 1;
const bt_rle: u32 = 2;
const bt_end: u32 = 3;

/*-*************************************
*  FSEv05 constants
***************************************/
const FSEv05_MAX_MEMORY_USAGE: u32 = 14;
const FSEv05_MAX_SYMBOL_VALUE: u32 = 255;
const FSEv05_MAX_TABLELOG: u32 = FSEv05_MAX_MEMORY_USAGE - 2;
const FSEv05_MIN_TABLELOG: u32 = 5;
const FSEv05_TABLELOG_ABSOLUTE_MAX: u32 = 15;

#[inline]
const fn FSEv05_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}
const DTABLE_MAX_SIZE: usize = 1 + (1 << FSEv05_MAX_TABLELOG); // DTable_max_t

/*-*************************************
*  HUFv05 constants
***************************************/
const HUFv05_ABSOLUTEMAX_TABLELOG: u32 = 16;
const HUFv05_MAX_TABLELOG: u32 = 12;
const HUFv05_MAX_SYMBOL_VALUE: u32 = 255;
#[inline]
const fn HUFv05_DTABLE_SIZE(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

/*-*************************************
*  bitStream types
***************************************/
#[repr(C)]
#[derive(Clone, Copy)]
struct BITv05_DStream_t {
    bitContainer: usize,
    bitsConsumed: c_uint,
    ptr: *const u8,
    start: *const u8,
}

/* BITv05_DStream_status */
const BITv05_DStream_unfinished: U32 = 0;
const BITv05_DStream_endOfBuffer: U32 = 1;
const BITv05_DStream_completed: U32 = 2;
const BITv05_DStream_overflow: U32 = 3;

/*-*************************************
*  FSEv05 decompression types
***************************************/
#[repr(C)]
#[derive(Clone, Copy)]
struct FSEv05_DTableHeader {
    tableLog: U16,
    fastMode: U16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSEv05_decode_t {
    newState: U16,
    symbol: BYTE,
    nbBits: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSEv05_DState_t {
    state: usize,
    table: *const c_void,
}

/*-*************************************
*  HUFv05 decompression types
***************************************/
#[repr(C)]
#[derive(Clone, Copy)]
struct HUFv05_DEltX2 {
    byte: BYTE,
    nbBits: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HUFv05_DEltX4 {
    sequence: U16,
    nbBits: BYTE,
    length: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct sortedSymbol_t {
    symbol: BYTE,
    weight: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
}

/*-*************************************
*  ZSTDv05 types
***************************************/
/* ZSTDv05_strategy is an enum (C int) */
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTDv05_parameters {
    srcSize: U64,
    windowLog: U32,
    contentLog: U32,
    hashLog: U32,
    searchLog: U32,
    searchLength: U32,
    targetLength: U32,
    strategy: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct blockProperties_t {
    blockType: U32, /* blockType_t */
    origSize: U32,
}

/* ZSTDv05_dStage */
const ZSTDv05ds_getFrameHeaderSize: U32 = 0;
const ZSTDv05ds_decodeFrameHeader: U32 = 1;
const ZSTDv05ds_decodeBlockHeader: U32 = 2;
const ZSTDv05ds_decompressBlock: U32 = 3;

#[repr(C)]
struct ZSTDv05_DCtx {
    LLTable: [U32; 1 + (1 << LLFSEv05Log)],       /* 1025 */
    OffTable: [U32; 1 + (1 << OffFSEv05Log)],     /* 513 */
    MLTable: [U32; 1 + (1 << MLFSEv05Log)],       /* 1025 */
    hufTableX4: [U32; 1 + (1 << ZSTD_HUFFDTABLE_CAPACITY_LOG)], /* 4097 */
    previousDstEnd: *const c_void,
    base: *const c_void,
    vBase: *const c_void,
    dictEnd: *const c_void,
    expected: usize,
    headerSize: usize,
    params: ZSTDv05_parameters,
    bType: U32, /* blockType_t */
    stage: U32, /* ZSTDv05_dStage */
    flagStaticTables: U32,
    litPtr: *const BYTE,
    litSize: usize,
    litBuffer: [BYTE; BLOCKSIZE + WILDCOPY_OVERLENGTH],
    headerBuffer: [BYTE; ZSTDv05_frameHeaderSize_max],
}

/* ZBUFFv05_dStage */
const ZBUFFv05ds_init: U32 = 0;
const ZBUFFv05ds_readHeader: U32 = 1;
const ZBUFFv05ds_loadHeader: U32 = 2;
const ZBUFFv05ds_decodeHeader: U32 = 3;
const ZBUFFv05ds_read: U32 = 4;
const ZBUFFv05ds_load: U32 = 5;
const ZBUFFv05ds_flush: U32 = 6;

const ZBUFFv05_blockHeaderSize: usize = 3;

#[repr(C)]
struct ZBUFFv05_DCtx {
    zc: *mut ZSTDv05_DCtx,
    params: ZSTDv05_parameters,
    inBuff: *mut c_char,
    inBuffSize: usize,
    inPos: usize,
    outBuff: *mut c_char,
    outBuffSize: usize,
    outStart: usize,
    outEnd: usize,
    hPos: usize,
    stage: U32, /* ZBUFFv05_dStage */
    headerBuffer: [u8; ZSTDv05_frameHeaderSize_max],
}

/*-*************************************
*  Low-level helpers
***************************************/
#[inline]
fn BITv05_highbit32(val: U32) -> c_uint {
    val.leading_zeros() ^ 31
}

#[inline]
unsafe fn ZSTDv05_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

#[inline]
unsafe fn ZSTDv05_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

/// custom version of memcpy(), can copy up to 7 bytes too many (8 if length==0)
#[inline]
unsafe fn ZSTDv05_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.offset(length);
    loop {
        ZSTDv05_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if !(op < oend) {
            break;
        }
    }
}

/*-********************************************************
* bitStream decoding
**********************************************************/
unsafe fn BITv05_initDStream(
    bitD: *mut BITv05_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BITv05_DStream_t>());
        return error(code::SRCSIZE_WRONG);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        /* normal case */
        let contain32: U32;
        (*bitD).start = srcBuffer as *const u8;
        (*bitD).ptr = (srcBuffer as *const u8).add(srcSize - core::mem::size_of::<usize>());
        (*bitD).bitContainer = mem_read_le_st((*bitD).ptr as *const c_void);
        contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return error(code::GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BITv05_highbit32(contain32);
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const u8;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let start = (*bitD).start as *const BYTE;
        match srcSize {
            7 => {
                (*bitD).bitContainer +=
                    (*(start.add(6)) as usize) << (core::mem::size_of::<usize>() * 8 - 16);
                (*bitD).bitContainer +=
                    (*(start.add(5)) as usize) << (core::mem::size_of::<usize>() * 8 - 24);
                (*bitD).bitContainer +=
                    (*(start.add(4)) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
                (*bitD).bitContainer += (*(start.add(3)) as usize) << 24;
                (*bitD).bitContainer += (*(start.add(2)) as usize) << 16;
                (*bitD).bitContainer += (*(start.add(1)) as usize) << 8;
            }
            6 => {
                (*bitD).bitContainer +=
                    (*(start.add(5)) as usize) << (core::mem::size_of::<usize>() * 8 - 24);
                (*bitD).bitContainer +=
                    (*(start.add(4)) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
                (*bitD).bitContainer += (*(start.add(3)) as usize) << 24;
                (*bitD).bitContainer += (*(start.add(2)) as usize) << 16;
                (*bitD).bitContainer += (*(start.add(1)) as usize) << 8;
            }
            5 => {
                (*bitD).bitContainer +=
                    (*(start.add(4)) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
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
        contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return error(code::GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BITv05_highbit32(contain32);
        (*bitD).bitsConsumed += ((core::mem::size_of::<usize>() - srcSize) * 8) as U32;
    }

    srcSize
}

#[inline]
unsafe fn BITv05_lookBits(bitD: *const BITv05_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> (((bitMask - nbBits) & bitMask) as usize)
}

#[inline]
unsafe fn BITv05_lookBitsFast(bitD: *const BITv05_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((((bitMask + 1) - nbBits) & bitMask) as usize)
}

#[inline]
unsafe fn BITv05_skipBits(bitD: *mut BITv05_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
unsafe fn BITv05_readBits(bitD: *mut BITv05_DStream_t, nbBits: c_uint) -> usize {
    let value = BITv05_lookBits(bitD, nbBits);
    BITv05_skipBits(bitD, nbBits);
    value
}

#[inline]
unsafe fn BITv05_readBitsFast(bitD: *mut BITv05_DStream_t, nbBits: c_uint) -> usize {
    let value = BITv05_lookBitsFast(bitD, nbBits);
    BITv05_skipBits(bitD, nbBits);
    value
}

unsafe fn BITv05_reloadDStream(bitD: *mut BITv05_DStream_t) -> U32 {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as U32 {
        return BITv05_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD).ptr.offset(-(((*bitD).bitsConsumed >> 3) as isize));
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = mem_read_le_st((*bitD).ptr as *const c_void);
        return BITv05_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as U32 {
            return BITv05_DStream_endOfBuffer;
        }
        return BITv05_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: U32 = BITv05_DStream_unfinished;
        if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32;
            result = BITv05_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = mem_read_le_st((*bitD).ptr as *const c_void);
        return result;
    }
}

#[inline]
unsafe fn BITv05_endOfDStream(dstream: *const BITv05_DStream_t) -> c_uint {
    (((*dstream).ptr == (*dstream).start)
        && ((*dstream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as U32)) as c_uint
}

/*-*************************************
*  FSEv05 symbol decompression
***************************************/
#[inline]
unsafe fn FSEv05_initDState(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
    dt: *const U32,
) {
    let DTableH = dt as *const FSEv05_DTableHeader;
    (*DStatePtr).state = BITv05_readBits(bitD, (*DTableH).tableLog as c_uint);
    BITv05_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline]
unsafe fn FSEv05_peakSymbol(DStatePtr: *const FSEv05_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

#[inline]
unsafe fn FSEv05_decodeSymbol(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BITv05_readBits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
unsafe fn FSEv05_decodeSymbolFast(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BITv05_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
unsafe fn FSEv05_endOfDState(DStatePtr: *const FSEv05_DState_t) -> c_uint {
    ((*DStatePtr).state == 0) as c_uint
}

/*-*************************************
*  FSEv05 table management
***************************************/
#[inline]
fn FSEv05_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_createDTable(mut tableLog: c_uint) -> *mut U32 {
    if tableLog > FSEv05_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv05_TABLELOG_ABSOLUTE_MAX;
    }
    malloc(FSEv05_DTABLE_SIZE_U32(tableLog) * core::mem::size_of::<U32>()) as *mut U32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_freeDTable(dt: *mut U32) {
    free(dt as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable(
    dt: *mut U32,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let mut DTableH: FSEv05_DTableHeader = FSEv05_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tableDecode = dt.add(1) as *mut FSEv05_decode_t;
    let tableSize: U32 = 1 << tableLog;
    let tableMask: U32 = tableSize - 1;
    let step: U32 = FSEv05_tableStep(tableSize);
    let mut symbolNext: [U16; (FSEv05_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSEv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize - 1;
    let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    /* Sanity Checks */
    if maxSymbolValue > FSEv05_MAX_SYMBOL_VALUE {
        return error(code::MAXSYMBOLVALUE_TOOLARGE);
    }
    if tableLog > FSEv05_MAX_TABLELOG {
        return error(code::TABLELOG_TOOLARGE);
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
            highThreshold -= 1;
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

    /* Build Decoding table */
    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] += 1;
            (*tableDecode.add(i as usize)).nbBits =
                (tableLog - BITv05_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(i as usize)).newState = (((nextState as U32)
                << (*tableDecode.add(i as usize)).nbBits as U32)
                .wrapping_sub(tableSize)) as U16;
            i += 1;
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

#[unsafe(no_mangle)]
pub extern "C" fn FSEv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn FSEv05_getErrorName(code: usize) -> *const c_char {
    err_get_error_name(code)
}

/*-*************************************
*  FSEv05 NCount encoding-decoding
***************************************/
#[inline]
fn FSEv05_abs(a: S16) -> S16 {
    if a < 0 {
        -a
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
        return error(code::SRCSIZE_WRONG);
    }
    bitStream = mem_read_le32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) + FSEv05_MIN_TABLELOG) as c_int;
    if nbBits > FSEv05_TABLELOG_ABSOLUTE_MAX as c_int {
        return error(code::TABLELOG_TOOLARGE);
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
                    bitStream = mem_read_le32(ip as *const c_void) >> bitCount;
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
            if (ip <= iend.offset(-7))
                || (ip.offset((bitCount >> 3) as isize) <= iend.offset(-4))
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
                bitStream = mem_read_le32(ip as *const c_void) >> bitCount;
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
            remaining -= FSEv05_abs(count) as c_int;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as c_int;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            if (ip <= iend.offset(-7))
                || (ip.offset((bitCount >> 3) as isize) <= iend.offset(-4))
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend.offset(-4).offset_from(ip))) as c_int;
                ip = iend.offset(-4);
            }
            bitStream = mem_read_le32(ip as *const c_void) >> (bitCount & 31);
        }
    }
    if remaining != 1 {
        return error(code::GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    if (ip.offset_from(istart) as usize) > hbSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip.offset_from(istart) as usize
}

/*-*************************************
*  Decompression (Byte symbols)
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable_rle(dt: *mut U32, symbolValue: BYTE) -> usize {
    let DTableH = dt as *mut FSEv05_DTableHeader;
    let cell = dt.add(1) as *mut FSEv05_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable_raw(dt: *mut U32, nbBits: c_uint) -> usize {
    let DTableH = dt as *mut FSEv05_DTableHeader;
    let dinfo = dt.add(1) as *mut FSEv05_decode_t;
    let tableSize: c_uint = 1 << nbBits;
    let tableMask: c_uint = tableSize - 1;
    let maxSymbolValue: c_uint = tableMask;
    let mut s: c_uint;

    /* Sanity checks */
    if nbBits < 1 {
        return error(code::GENERIC);
    }

    /* Build Decoding Table */
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

unsafe fn FSEv05_decompress_usingDTable_generic(
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

    let mut bitD: BITv05_DStream_t = core::mem::zeroed();
    let mut state1: FSEv05_DState_t = core::mem::zeroed();
    let mut state2: FSEv05_DState_t = core::mem::zeroed();
    let errorCode: usize;

    /* Init */
    errorCode = BITv05_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }

    FSEv05_initDState(&mut state1, &mut bitD, dt);
    FSEv05_initDState(&mut state2, &mut bitD, dt);

    macro_rules! GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSEv05_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSEv05_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    /* 4 symbols per loop */
    while (BITv05_reloadDStream(&mut bitD) == BITv05_DStream_unfinished) && (op < olimit) {
        *op.add(0) = GETSYMBOL!(&mut state1);

        if FSEv05_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as U32 {
            BITv05_reloadDStream(&mut bitD);
        }

        *op.add(1) = GETSYMBOL!(&mut state2);

        if FSEv05_MAX_TABLELOG * 4 + 7 > (core::mem::size_of::<usize>() * 8) as U32 {
            if BITv05_reloadDStream(&mut bitD) > BITv05_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = GETSYMBOL!(&mut state1);

        if FSEv05_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as U32 {
            BITv05_reloadDStream(&mut bitD);
        }

        *op.add(3) = GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    /* tail */
    loop {
        if (BITv05_reloadDStream(&mut bitD) > BITv05_DStream_completed)
            || (op == omax)
            || (BITv05_endOfDStream(&bitD) != 0
                && (fast != 0 || FSEv05_endOfDState(&state1) != 0))
        {
            break;
        }

        *op = GETSYMBOL!(&mut state1);
        op = op.add(1);

        if (BITv05_reloadDStream(&mut bitD) > BITv05_DStream_completed)
            || (op == omax)
            || (BITv05_endOfDStream(&bitD) != 0
                && (fast != 0 || FSEv05_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = GETSYMBOL!(&mut state2);
        op = op.add(1);
    }

    /* end ? */
    if BITv05_endOfDStream(&bitD) != 0
        && FSEv05_endOfDState(&state1) != 0
        && FSEv05_endOfDState(&state2) != 0
    {
        return op.offset_from(ostart) as usize;
    }

    if op == omax {
        return error(code::DSTSIZE_TOOSMALL);
    }

    error(code::CORRUPTION_DETECTED)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const U32,
) -> usize {
    let DTableH = dt as *const FSEv05_DTableHeader;
    let fastMode: U32 = (*DTableH).fastMode as U32;

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
    let mut counting: [S16; (FSEv05_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSEv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [U32; DTABLE_MAX_SIZE] = [0; DTABLE_MAX_SIZE];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv05_MAX_SYMBOL_VALUE;
    let mut errorCode: usize;

    if cSrcSize < 2 {
        return error(code::SRCSIZE_WRONG);
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
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSEv05_buildDTable(
        dt.as_mut_ptr(),
        counting.as_ptr(),
        maxSymbolValue,
        tableLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }

    FSEv05_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

/* *******************************************************
*  Huff0 : Huffman block decompression
*********************************************************/
#[unsafe(no_mangle)]
pub extern "C" fn HUFv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUFv05_getErrorName(code: usize) -> *const c_char {
    err_get_error_name(code)
}

/* HUFv05_readStats : read compact Huffman tree */
unsafe fn HUFv05_readStats(
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
                return error(code::SRCSIZE_WRONG);
            }
            if oSize >= hwSize {
                return error(code::CORRUPTION_DETECTED);
            }
            ip = ip.add(1);
            n = 0;
            while (n as usize) < oSize {
                *huffWeight.add(n as usize) = *ip.add((n / 2) as usize) >> 4;
                *huffWeight.add(n as usize + 1) = *ip.add((n / 2) as usize) & 15;
                n += 2;
            }
        }
    } else {
        /* header compressed with FSEv05 (normal case) */
        if iSize + 1 > srcSize {
            return error(code::SRCSIZE_WRONG);
        }
        oSize = FSEv05_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
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
        if *huffWeight.add(n as usize) as U32 >= HUFv05_ABSOLUTEMAX_TABLELOG {
            return error(code::CORRUPTION_DETECTED);
        }
        *rankStats.add(*huffWeight.add(n as usize) as usize) += 1;
        weightTotal += (1u32 << *huffWeight.add(n as usize)) >> 1;
        n += 1;
    }
    if weightTotal == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    tableLog = BITv05_highbit32(weightTotal) + 1;
    if tableLog > HUFv05_ABSOLUTEMAX_TABLELOG {
        return error(code::CORRUPTION_DETECTED);
    }
    {
        let total: U32 = 1 << tableLog;
        let rest: U32 = total - weightTotal;
        let verif: U32 = 1 << BITv05_highbit32(rest);
        let lastWeight: U32 = BITv05_highbit32(rest) + 1;
        if verif != rest {
            return error(code::CORRUPTION_DETECTED);
        }
        *huffWeight.add(oSize) = lastWeight as BYTE;
        *rankStats.add(lastWeight as usize) += 1;
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    *tableLogPtr = tableLog;
    iSize + 1
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
    let mut huffWeight: [BYTE; (HUFv05_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUFv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal: [U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dt = (DTable.add(1)) as *mut HUFv05_DEltX2;

    iSize = HUFv05_readStats(
        huffWeight.as_mut_ptr(),
        (HUFv05_MAX_SYMBOL_VALUE + 1) as usize,
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
    if tableLog > *DTable.add(0) as U32 {
        return error(code::TABLELOG_TOOLARGE);
    }
    *DTable.add(0) = tableLog as U16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current: U32 = nextRankStart;
        nextRankStart += rankVal[n as usize] << (n - 1);
        rankVal[n as usize] = current;
        n += 1;
    }

    /* fill DTable */
    n = 0;
    while n < nbSymbols {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32 << w) >> 1;
        let mut i: U32;
        let mut D: HUFv05_DEltX2 = HUFv05_DEltX2 { byte: 0, nbBits: 0 };
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
unsafe fn HUFv05_decodeSymbolX2(
    Dstream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BITv05_lookBitsFast(Dstream, dtLog);
    let c = (*dt.add(val)).byte;
    BITv05_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

// Macros emulating HUFv05_DECODE_SYMBOLX2_{0,1,2}: advance *ptr by one symbol.
macro_rules! HUFv05_DECODE_SYMBOLX2_0 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUFv05_decodeSymbolX2($ds, $dt, $dtLog);
        $ptr = $ptr.add(1);
    }};
}
macro_rules! HUFv05_DECODE_SYMBOLX2_1 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        if mem_64bits() != 0 || (HUFv05_MAX_TABLELOG <= 12) {
            HUFv05_DECODE_SYMBOLX2_0!($ptr, $ds, $dt, $dtLog);
        }
    }};
}
macro_rules! HUFv05_DECODE_SYMBOLX2_2 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        if mem_64bits() != 0 {
            HUFv05_DECODE_SYMBOLX2_0!($ptr, $ds, $dt, $dtLog);
        }
    }};
}

unsafe fn HUFv05_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv05_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv05_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 4 symbols at a time */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished) && (p <= pEnd.offset(-4)) {
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

    pEnd.offset_from(pStart) as usize
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
    let oend = op.add(dstSize);
    let dtLog: U32 = *DTable.add(0) as U32;
    let dt = (DTable as *const HUFv05_DEltX2).add(1);
    let mut bitD: BITv05_DStream_t = core::mem::zeroed();

    if dstSize <= cSrcSize {
        return error(code::DSTSIZE_TOOSMALL);
    }
    {
        let errorCode = BITv05_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv05_decodeStreamX2(op, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv05_endOfDStream(&bitD) == 0 {
        return error(code::CORRUPTION_DETECTED);
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
    let errorCode: usize;

    errorCode = HUFv05_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

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
        return error(code::CORRUPTION_DETECTED);
    }
    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dt = (DTable as *const HUFv05_DEltX2).add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv05_DStream_t = core::mem::zeroed();
        let length1 = mem_read_le16(istart as *const c_void) as usize;
        let length2 = mem_read_le16(istart.add(2) as *const c_void) as usize;
        let length3 = mem_read_le16(istart.add(4) as *const c_void) as usize;
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
        errorCode = BITv05_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }

        /* 16-32 symbols per loop */
        endSignal = BITv05_reloadDStream(&mut bitD1)
            | BITv05_reloadDStream(&mut bitD2)
            | BITv05_reloadDStream(&mut bitD3)
            | BITv05_reloadDStream(&mut bitD4);
        while (endSignal == BITv05_DStream_unfinished) && (op4 < oend.offset(-7)) {
            HUFv05_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_1!(op1, &mut bitD1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_1!(op2, &mut bitD2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_1!(op3, &mut bitD3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_1!(op4, &mut bitD4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_0!(op1, &mut bitD1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_0!(op2, &mut bitD2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_0!(op3, &mut bitD3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX2_0!(op4, &mut bitD4, dt, dtLog);
            endSignal = BITv05_reloadDStream(&mut bitD1)
                | BITv05_reloadDStream(&mut bitD2)
                | BITv05_reloadDStream(&mut bitD3)
                | BITv05_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 > opStart2 {
            return error(code::CORRUPTION_DETECTED);
        }
        if op2 > opStart3 {
            return error(code::CORRUPTION_DETECTED);
        }
        if op3 > opStart4 {
            return error(code::CORRUPTION_DETECTED);
        }

        /* finish bitStreams one by one */
        HUFv05_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv05_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv05_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv05_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BITv05_endOfDStream(&bitD1)
            & BITv05_endOfDStream(&bitD2)
            & BITv05_endOfDStream(&bitD3)
            & BITv05_endOfDStream(&bitD4);
        if endSignal == 0 {
            return error(code::CORRUPTION_DETECTED);
        }

        dstSize
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
    let errorCode: usize;

    errorCode = HUFv05_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUFv05_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/* *************************/
/* double-symbols decoding */
/* *************************/
unsafe fn HUFv05_fillDTableX4Level2(
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
    let mut DElt: HUFv05_DEltX4 = HUFv05_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut s: U32;

    /* get pre-calculated rankVal */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: U32;
        let skipSize: U32 = rankVal[minWeight as usize];
        mem_write_le16(&mut DElt.sequence as *mut U16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
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
        let symbol: U32 = (*sortedSymbols.add(s as usize)).symbol as U32;
        let weight: U32 = (*sortedSymbols.add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline - weight;
        let length: U32 = 1 << (sizeLog - nbBits);
        let start: U32 = rankVal[weight as usize];
        let mut i: U32 = start;
        let end: U32 = start + length;

        mem_write_le16(
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

unsafe fn HUFv05_fillDTableX4(
    DTable: *mut HUFv05_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *mut [U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: c_int = (nbBitsBaseline - targetLog) as c_int;
    let minBits: U32 = nbBitsBaseline - maxWeight;
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline - weight;
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1 << (targetLog - nbBits);

        if targetLog - nbBits >= minBits {
            /* enough room for a second symbol */
            let sortedRank: U32;
            let mut minWeight: c_int = nbBits as c_int + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUFv05_fillDTableX4Level2(
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
            let end: U32 = start + length;
            let mut DElt: HUFv05_DEltX4 = HUFv05_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };

            mem_write_le16(&mut DElt.sequence as *mut U16 as *mut c_void, symbol);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_readDTableX4(
    DTable: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightList: [BYTE; (HUFv05_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUFv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUFv05_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; (HUFv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 2) as usize] =
        [0; (HUFv05_ABSOLUTEMAX_TABLELOG + 2) as usize];
    /* rankStart = rankStart0 + 1 : accessed via helper closures using indexes offset by 1 */
    let mut rankVal: [[U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
        HUFv05_ABSOLUTEMAX_TABLELOG as usize] =
        [[0; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize]; HUFv05_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.add(0);
    let iSize: usize;
    let dt = (DTable as *mut HUFv05_DEltX4).add(1);

    /* rankStart points at rankStart0[1]; emulate via raw pointer */
    let rankStart = rankStart0.as_mut_ptr().add(1);

    if memLog > HUFv05_ABSOLUTEMAX_TABLELOG {
        return error(code::TABLELOG_TOOLARGE);
    }

    iSize = HUFv05_readStats(
        weightList.as_mut_ptr(),
        (HUFv05_MAX_SYMBOL_VALUE + 1) as usize,
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
        return error(code::TABLELOG_TOOLARGE);
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW -= 1;
    }

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankStart;
            nextRankStart += rankStats[w as usize];
            *rankStart.add(w as usize) = current;
            w += 1;
        }
        *rankStart.add(0) = nextRankStart;
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32;
        s = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = *rankStart.add(w as usize);
            *rankStart.add(w as usize) += 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        *rankStart.add(0) = 0;
    }

    /* Build rankVal */
    {
        let minBits: U32 = tableLog + 1 - maxW;
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: c_int = (memLog - tableLog) as c_int - 1;
        /* rankVal0 = rankVal[0] */
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankVal;
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

    HUFv05_fillDTableX4(
        dt,
        memLog,
        sortedSymbol.as_ptr(),
        sizeOfSort,
        rankStart0.as_ptr(),
        rankVal.as_mut_ptr(),
        maxW,
        tableLog + 1,
    );

    iSize
}

#[inline]
unsafe fn HUFv05_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv05_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 2);
    BITv05_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

#[inline]
unsafe fn HUFv05_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv05_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BITv05_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (core::mem::size_of::<usize>() * 8) as U32 {
            BITv05_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (core::mem::size_of::<usize>() * 8) as U32 {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as U32;
            }
        }
    }
    1
}

macro_rules! HUFv05_DECODE_SYMBOLX4_0 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.add(HUFv05_decodeSymbolX4($ptr as *mut c_void, $ds, $dt, $dtLog) as usize);
    }};
}
macro_rules! HUFv05_DECODE_SYMBOLX4_1 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        if mem_64bits() != 0 || (HUFv05_MAX_TABLELOG <= 12) {
            $ptr = $ptr.add(HUFv05_decodeSymbolX4($ptr as *mut c_void, $ds, $dt, $dtLog) as usize);
        }
    }};
}
macro_rules! HUFv05_DECODE_SYMBOLX4_2 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        if mem_64bits() != 0 {
            $ptr = $ptr.add(HUFv05_decodeSymbolX4($ptr as *mut c_void, $ds, $dt, $dtLog) as usize);
        }
    }};
}

unsafe fn HUFv05_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv05_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished) && (p < pEnd.offset(-7)) {
        HUFv05_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX4_1!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUFv05_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished) && (p <= pEnd.offset(-2)) {
        HUFv05_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p <= pEnd.offset(-2) {
        HUFv05_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    if p < pEnd {
        p = p.add(HUFv05_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    p.offset_from(pStart) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U32,
) -> usize {
    let istart = cSrc as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dstSize);

    let dtLog: U32 = *DTable.add(0);
    let dt = (DTable as *const HUFv05_DEltX4).add(1);
    let errorCode: usize;

    /* Init */
    let mut bitD: BITv05_DStream_t = core::mem::zeroed();
    errorCode = BITv05_initDStream(&mut bitD, istart as *const c_void, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }

    /* finish bitStreams one by one */
    HUFv05_decodeStreamX4(ostart, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv05_endOfDStream(&bitD) == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U32; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)] =
        [0; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv05_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUFv05_decompress1X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U32,
) -> usize {
    if cSrcSize < 10 {
        return error(code::CORRUPTION_DETECTED);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dt = (DTable as *const HUFv05_DEltX4).add(1);
        let dtLog: U32 = *DTable.add(0);
        let mut errorCode: usize;

        /* Init */
        let mut bitD1: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv05_DStream_t = core::mem::zeroed();
        let length1 = mem_read_le16(istart as *const c_void) as usize;
        let length2 = mem_read_le16(istart.add(2) as *const c_void) as usize;
        let length3 = mem_read_le16(istart.add(4) as *const c_void) as usize;
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
        errorCode = BITv05_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }

        /* 16-32 symbols per loop */
        endSignal = BITv05_reloadDStream(&mut bitD1)
            | BITv05_reloadDStream(&mut bitD2)
            | BITv05_reloadDStream(&mut bitD3)
            | BITv05_reloadDStream(&mut bitD4);
        while (endSignal == BITv05_DStream_unfinished) && (op4 < oend.offset(-7)) {
            HUFv05_DECODE_SYMBOLX4_2!(op1, &mut bitD1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op2, &mut bitD2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op3, &mut bitD3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op4, &mut bitD4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_1!(op1, &mut bitD1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_1!(op2, &mut bitD2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_1!(op3, &mut bitD3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_1!(op4, &mut bitD4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op1, &mut bitD1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op2, &mut bitD2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op3, &mut bitD3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_2!(op4, &mut bitD4, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_0!(op1, &mut bitD1, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_0!(op2, &mut bitD2, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_0!(op3, &mut bitD3, dt, dtLog);
            HUFv05_DECODE_SYMBOLX4_0!(op4, &mut bitD4, dt, dtLog);

            endSignal = BITv05_reloadDStream(&mut bitD1)
                | BITv05_reloadDStream(&mut bitD2)
                | BITv05_reloadDStream(&mut bitD3)
                | BITv05_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 > opStart2 {
            return error(code::CORRUPTION_DETECTED);
        }
        if op2 > opStart3 {
            return error(code::CORRUPTION_DETECTED);
        }
        if op3 > opStart4 {
            return error(code::CORRUPTION_DETECTED);
        }

        /* finish bitStreams one by one */
        HUFv05_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv05_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv05_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv05_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BITv05_endOfDStream(&bitD1)
            & BITv05_endOfDStream(&bitD2)
            & BITv05_endOfDStream(&bitD3)
            & BITv05_endOfDStream(&bitD4);
        if endSignal == 0 {
            return error(code::CORRUPTION_DETECTED);
        }

        dstSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U32; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)] =
        [0; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv05_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUFv05_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/* ********************************/
/* Generic decompression selector */
/* ********************************/
static algoTime: [[algo_time_t; 3]; 16] = [
    [
        algo_time_t { tableTime: 0, decode256Time: 0 },
        algo_time_t { tableTime: 1, decode256Time: 1 },
        algo_time_t { tableTime: 2, decode256Time: 2 },
    ],
    [
        algo_time_t { tableTime: 0, decode256Time: 0 },
        algo_time_t { tableTime: 1, decode256Time: 1 },
        algo_time_t { tableTime: 2, decode256Time: 2 },
    ],
    [
        algo_time_t { tableTime: 38, decode256Time: 130 },
        algo_time_t { tableTime: 1313, decode256Time: 74 },
        algo_time_t { tableTime: 2151, decode256Time: 38 },
    ],
    [
        algo_time_t { tableTime: 448, decode256Time: 128 },
        algo_time_t { tableTime: 1353, decode256Time: 74 },
        algo_time_t { tableTime: 2238, decode256Time: 41 },
    ],
    [
        algo_time_t { tableTime: 556, decode256Time: 128 },
        algo_time_t { tableTime: 1353, decode256Time: 74 },
        algo_time_t { tableTime: 2238, decode256Time: 47 },
    ],
    [
        algo_time_t { tableTime: 714, decode256Time: 128 },
        algo_time_t { tableTime: 1418, decode256Time: 74 },
        algo_time_t { tableTime: 2436, decode256Time: 53 },
    ],
    [
        algo_time_t { tableTime: 883, decode256Time: 128 },
        algo_time_t { tableTime: 1437, decode256Time: 74 },
        algo_time_t { tableTime: 2464, decode256Time: 61 },
    ],
    [
        algo_time_t { tableTime: 897, decode256Time: 128 },
        algo_time_t { tableTime: 1515, decode256Time: 75 },
        algo_time_t { tableTime: 2622, decode256Time: 68 },
    ],
    [
        algo_time_t { tableTime: 926, decode256Time: 128 },
        algo_time_t { tableTime: 1613, decode256Time: 75 },
        algo_time_t { tableTime: 2730, decode256Time: 75 },
    ],
    [
        algo_time_t { tableTime: 947, decode256Time: 128 },
        algo_time_t { tableTime: 1729, decode256Time: 77 },
        algo_time_t { tableTime: 3359, decode256Time: 77 },
    ],
    [
        algo_time_t { tableTime: 1107, decode256Time: 128 },
        algo_time_t { tableTime: 2083, decode256Time: 81 },
        algo_time_t { tableTime: 4006, decode256Time: 84 },
    ],
    [
        algo_time_t { tableTime: 1177, decode256Time: 128 },
        algo_time_t { tableTime: 2379, decode256Time: 87 },
        algo_time_t { tableTime: 4785, decode256Time: 88 },
    ],
    [
        algo_time_t { tableTime: 1242, decode256Time: 128 },
        algo_time_t { tableTime: 2415, decode256Time: 93 },
        algo_time_t { tableTime: 5155, decode256Time: 84 },
    ],
    [
        algo_time_t { tableTime: 1349, decode256Time: 128 },
        algo_time_t { tableTime: 2644, decode256Time: 106 },
        algo_time_t { tableTime: 5260, decode256Time: 106 },
    ],
    [
        algo_time_t { tableTime: 1455, decode256Time: 128 },
        algo_time_t { tableTime: 2422, decode256Time: 124 },
        algo_time_t { tableTime: 4174, decode256Time: 124 },
    ],
    [
        algo_time_t { tableTime: 722, decode256Time: 128 },
        algo_time_t { tableTime: 1891, decode256Time: 145 },
        algo_time_t { tableTime: 1936, decode256Time: 146 },
    ],
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    type DecompressionAlgo =
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
    let decompress: [Option<DecompressionAlgo>; 3] =
        [Some(HUFv05_decompress4X2), Some(HUFv05_decompress4X4), None];
    /* estimate decompression time */
    let Q: U32;
    let D256: U32 = (dstSize >> 8) as U32;
    let mut Dtime: [U32; 3] = [0; 3];
    let mut algoNb: U32 = 0;
    let mut n: c_int;

    /* validation checks */
    if dstSize == 0 {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if cSrcSize >= dstSize {
        return error(code::CORRUPTION_DETECTED);
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    }

    /* decoder timing evaluation */
    Q = (cSrcSize * 16 / dstSize) as U32;
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

    (decompress[algoNb as usize].unwrap())(dst, dstSize, cSrc, cSrcSize)
}

/* *************************************
*  ZSTDv05 Error Management
***************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv05_getErrorName(code: usize) -> *const c_char {
    err_get_error_name(code)
}

/* *************************************************************
*   Context management
***************************************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv05_sizeofDCtx() -> usize {
    core::mem::size_of::<ZSTDv05_DCtx>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBegin(dctx: *mut ZSTDv05_DCtx) -> usize {
    (*dctx).expected = ZSTDv05_frameHeaderSize_min;
    (*dctx).stage = ZSTDv05ds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).hufTableX4[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG;
    (*dctx).flagStaticTables = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_createDCtx() -> *mut ZSTDv05_DCtx {
    let dctx = malloc(core::mem::size_of::<ZSTDv05_DCtx>()) as *mut ZSTDv05_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDv05_decompressBegin(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_freeDCtx(dctx: *mut ZSTDv05_DCtx) -> usize {
    free(dctx as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_copyDCtx(
    dstDCtx: *mut ZSTDv05_DCtx,
    srcDCtx: *const ZSTDv05_DCtx,
) {
    memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv05_DCtx>()
            - (BLOCKSIZE + WILDCOPY_OVERLENGTH + ZSTDv05_frameHeaderSize_max),
    );
}

/* *************************************************************
*   Decompression section
***************************************************************/
unsafe fn ZSTDv05_decodeFrameHeader_Part1(
    zc: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let magicNumber: U32;
    if srcSize != ZSTDv05_frameHeaderSize_min {
        return error(code::SRCSIZE_WRONG);
    }
    magicNumber = mem_read_le32(src);
    if magicNumber != ZSTDv05_MAGICNUMBER {
        return error(code::PREFIX_UNKNOWN);
    }
    (*zc).headerSize = ZSTDv05_frameHeaderSize_min;
    (*zc).headerSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_getFrameParams(
    params: *mut ZSTDv05_parameters,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let magicNumber: U32;
    if srcSize < ZSTDv05_frameHeaderSize_min {
        return ZSTDv05_frameHeaderSize_max;
    }
    magicNumber = mem_read_le32(src);
    if magicNumber != ZSTDv05_MAGICNUMBER {
        return error(code::PREFIX_UNKNOWN);
    }
    memset(
        params as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv05_parameters>(),
    );
    (*params).windowLog =
        (*(src as *const BYTE).add(4) & 15) as U32 + ZSTDv05_WINDOWLOG_ABSOLUTEMIN;
    if (*(src as *const BYTE).add(4) >> 4) != 0 {
        return error(code::FRAMEPARAMETER_UNSUPPORTED);
    }
    0
}

unsafe fn ZSTDv05_decodeFrameHeader_Part2(
    zc: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result: usize;
    if srcSize != (*zc).headerSize {
        return error(code::SRCSIZE_WRONG);
    }
    result = ZSTDv05_getFrameParams(&mut (*zc).params, src, srcSize);
    if (mem_32bits() != 0) && ((*zc).params.windowLog > 25) {
        return error(code::FRAMEPARAMETER_UNSUPPORTED);
    }
    result
}

unsafe fn ZSTDv05_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_: *const BYTE = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return error(code::SRCSIZE_WRONG);
    }

    headerFlags = *in_;
    cSize = *in_.add(2) as U32 + ((*in_.add(1) as U32) << 8) + (((*in_.add(0) as U32) & 7) << 16);

    (*bpPtr).blockType = (headerFlags >> 6) as U32;
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as usize
}

unsafe fn ZSTDv05_copyRawBlock(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if dst.is_null() {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if srcSize > maxDstSize {
        return error(code::DSTSIZE_TOOSMALL);
    }
    memcpy(dst, src, srcSize);
    srcSize
}

/* ZSTDv05_decodeLiteralsBlock() : @return nb of bytes read from src */
unsafe fn ZSTDv05_decodeLiteralsBlock(
    dctx: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return error(code::CORRUPTION_DETECTED);
    }

    match (*istart.add(0) >> 6) as u32 {
        IS_HUFv05 => {
            let litSize: usize;
            let litCSize: usize;
            let mut singleStream: usize = 0;
            let mut lhSize: U32 = ((*istart.add(0)) >> 4) as U32 & 3;
            if srcSize < 5 {
                return error(code::CORRUPTION_DETECTED);
            }
            match lhSize {
                2 => {
                    lhSize = 4;
                    litSize = (((*istart.add(0) & 15) as usize) << 10)
                        + ((*istart.add(1) as usize) << 2)
                        + ((*istart.add(2) >> 6) as usize);
                    litCSize =
                        (((*istart.add(2) & 63) as usize) << 8) + *istart.add(3) as usize;
                }
                3 => {
                    lhSize = 5;
                    litSize = (((*istart.add(0) & 15) as usize) << 14)
                        + ((*istart.add(1) as usize) << 6)
                        + ((*istart.add(2) >> 2) as usize);
                    litCSize = (((*istart.add(2) & 3) as usize) << 16)
                        + ((*istart.add(3) as usize) << 8)
                        + *istart.add(4) as usize;
                }
                _ => {
                    /* 0, 1, default */
                    lhSize = 3;
                    singleStream = (*istart.add(0) & 16) as usize;
                    litSize = (((*istart.add(0) & 15) as usize) << 6)
                        + ((*istart.add(1) >> 2) as usize);
                    litCSize =
                        (((*istart.add(1) & 3) as usize) << 8) + *istart.add(2) as usize;
                }
            }
            if litSize > BLOCKSIZE {
                return error(code::CORRUPTION_DETECTED);
            }
            if litCSize + lhSize as usize > srcSize {
                return error(code::CORRUPTION_DETECTED);
            }

            let r = if singleStream != 0 {
                HUFv05_decompress1X2(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            } else {
                HUFv05_decompress(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            };
            if HUFv05_isError(r) != 0 {
                return error(code::CORRUPTION_DETECTED);
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            litCSize + lhSize as usize
        }
        IS_PCH => {
            let errorCode: usize;
            let litSize: usize;
            let litCSize: usize;
            let mut lhSize: U32 = ((*istart.add(0)) >> 4) as U32 & 3;
            if lhSize != 1 {
                return error(code::CORRUPTION_DETECTED);
            }
            if (*dctx).flagStaticTables == 0 {
                return error(code::DICTIONARY_CORRUPTED);
            }

            lhSize = 3;
            litSize =
                (((*istart.add(0) & 15) as usize) << 6) + ((*istart.add(1) >> 2) as usize);
            litCSize = (((*istart.add(1) & 3) as usize) << 8) + *istart.add(2) as usize;
            if litCSize + lhSize as usize > srcSize {
                return error(code::CORRUPTION_DETECTED);
            }

            errorCode = HUFv05_decompress1X4_usingDTable(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                litSize,
                istart.add(lhSize as usize) as *const c_void,
                litCSize,
                (*dctx).hufTableX4.as_ptr(),
            );
            if HUFv05_isError(errorCode) != 0 {
                return error(code::CORRUPTION_DETECTED);
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            litCSize + lhSize as usize
        }
        IS_RAW => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.add(0)) >> 4) as U32 & 3;
            match lhSize {
                2 => {
                    litSize =
                        (((*istart.add(0) & 15) as usize) << 8) + *istart.add(1) as usize;
                }
                3 => {
                    litSize = (((*istart.add(0) & 15) as usize) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + *istart.add(2) as usize;
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) & 31) as usize;
                }
            }

            if lhSize as usize + litSize + WILDCOPY_OVERLENGTH > srcSize {
                if litSize + lhSize as usize > srcSize {
                    return error(code::CORRUPTION_DETECTED);
                }
                memcpy(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    istart.add(lhSize as usize) as *const c_void,
                    litSize,
                );
                (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
                (*dctx).litSize = litSize;
                memset(
                    (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                    0,
                    WILDCOPY_OVERLENGTH,
                );
                return lhSize as usize + litSize;
            }
            /* direct reference into compressed stream */
            (*dctx).litPtr = istart.add(lhSize as usize);
            (*dctx).litSize = litSize;
            lhSize as usize + litSize
        }
        IS_RLE => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.add(0)) >> 4) as U32 & 3;
            match lhSize {
                2 => {
                    litSize =
                        (((*istart.add(0) & 15) as usize) << 8) + *istart.add(1) as usize;
                }
                3 => {
                    litSize = (((*istart.add(0) & 15) as usize) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + *istart.add(2) as usize;
                    if srcSize < 4 {
                        return error(code::CORRUPTION_DETECTED);
                    }
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) & 31) as usize;
                }
            }
            if litSize > BLOCKSIZE {
                return error(code::CORRUPTION_DETECTED);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.add(lhSize as usize) as c_int,
                litSize + WILDCOPY_OVERLENGTH,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            lhSize as usize + 1
        }
        _ => error(code::CORRUPTION_DETECTED),
    }
}

unsafe fn ZSTDv05_decodeSeqHeaders(
    nbSeq: *mut c_int,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut U32,
    DTableML: *mut U32,
    DTableOffb: *mut U32,
    src: *const c_void,
    srcSize: usize,
    flagStaticTable: U32,
) -> usize {
    let istart = src as *const BYTE;
    let mut ip = istart;
    let iend = istart.add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: c_uint = 0;
    let mut Offlog: c_uint = 0;
    let mut MLlog: c_uint = 0;
    let dumpsLength: usize;

    /* check */
    if srcSize < MIN_SEQUENCES_SIZE {
        return error(code::SRCSIZE_WRONG);
    }

    /* SeqHead */
    *nbSeq = *ip as c_int;
    ip = ip.add(1);
    if *nbSeq == 0 {
        return 1;
    }
    if *nbSeq >= 128 {
        if ip >= iend {
            return error(code::SRCSIZE_WRONG);
        }
        *nbSeq = ((*nbSeq - 128) << 8) + *ip as c_int;
        ip = ip.add(1);
    }

    if ip >= iend {
        return error(code::SRCSIZE_WRONG);
    }
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if *ip & 2 != 0 {
        if ip.add(3) > iend {
            return error(code::SRCSIZE_WRONG);
        }
        dumpsLength = *ip.add(2) as usize;
        let dl = dumpsLength + ((*ip.add(1) as usize) << 8);
        ip = ip.add(3);
        *dumpsPtr = ip;
        ip = ip.add(dl);
        *dumpsLengthPtr = dl;
    } else {
        if ip.add(2) > iend {
            return error(code::SRCSIZE_WRONG);
        }
        dumpsLength = *ip.add(1) as usize;
        let dl = dumpsLength + (((*ip.add(0) as usize) & 1) << 8);
        ip = ip.add(2);
        *dumpsPtr = ip;
        ip = ip.add(dl);
        *dumpsLengthPtr = dl;
    }

    /* check */
    if ip > iend.offset(-3) {
        return error(code::SRCSIZE_WRONG);
    }

    /* sequences */
    {
        let mut norm: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut headerSize: usize;

        /* Build DTables */
        match LLtype {
            FSEv05_ENCODING_RLE => {
                LLlog = 0;
                FSEv05_buildDTable_rle(DTableLL, *ip);
                ip = ip.add(1);
            }
            FSEv05_ENCODING_RAW => {
                LLlog = LLbits;
                FSEv05_buildDTable_raw(DTableLL, LLbits);
            }
            FSEv05_ENCODING_STATIC => {
                if flagStaticTable == 0 {
                    return error(code::CORRUPTION_DETECTED);
                }
            }
            _ => {
                let mut max: c_uint = MaxLL;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut LLlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as usize,
                );
                if FSEv05_isError(headerSize) != 0 {
                    return error(code::GENERIC);
                }
                if LLlog > LLFSEv05Log {
                    return error(code::CORRUPTION_DETECTED);
                }
                ip = ip.add(headerSize);
                FSEv05_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        match Offtype {
            FSEv05_ENCODING_RLE => {
                Offlog = 0;
                if ip > iend.offset(-2) {
                    return error(code::SRCSIZE_WRONG);
                }
                FSEv05_buildDTable_rle(DTableOffb, *ip & MaxOff as BYTE);
                ip = ip.add(1);
            }
            FSEv05_ENCODING_RAW => {
                Offlog = Offbits;
                FSEv05_buildDTable_raw(DTableOffb, Offbits);
            }
            FSEv05_ENCODING_STATIC => {
                if flagStaticTable == 0 {
                    return error(code::CORRUPTION_DETECTED);
                }
            }
            _ => {
                let mut max: c_uint = MaxOff;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut Offlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as usize,
                );
                if FSEv05_isError(headerSize) != 0 {
                    return error(code::GENERIC);
                }
                if Offlog > OffFSEv05Log {
                    return error(code::CORRUPTION_DETECTED);
                }
                ip = ip.add(headerSize);
                FSEv05_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
            }
        }

        match MLtype {
            FSEv05_ENCODING_RLE => {
                MLlog = 0;
                if ip > iend.offset(-2) {
                    return error(code::SRCSIZE_WRONG);
                }
                FSEv05_buildDTable_rle(DTableML, *ip);
                ip = ip.add(1);
            }
            FSEv05_ENCODING_RAW => {
                MLlog = MLbits;
                FSEv05_buildDTable_raw(DTableML, MLbits);
            }
            FSEv05_ENCODING_STATIC => {
                if flagStaticTable == 0 {
                    return error(code::CORRUPTION_DETECTED);
                }
            }
            _ => {
                let mut max: c_uint = MaxML;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut MLlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as usize,
                );
                if FSEv05_isError(headerSize) != 0 {
                    return error(code::GENERIC);
                }
                if MLlog > MLFSEv05Log {
                    return error(code::CORRUPTION_DETECTED);
                }
                ip = ip.add(headerSize);
                FSEv05_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
            }
        }
    }

    ip.offset_from(istart) as usize
}

#[repr(C)]
#[derive(Clone, Copy)]
struct seq_t {
    litLength: usize,
    matchLength: usize,
    offset: usize,
}

#[repr(C)]
struct seqState_t {
    DStream: BITv05_DStream_t,
    stateLL: FSEv05_DState_t,
    stateOffb: FSEv05_DState_t,
    stateML: FSEv05_DState_t,
    prevOffset: usize,
    dumps: *const BYTE,
    dumpsEnd: *const BYTE,
}

static offsetPrefix: [U32; (MaxOff + 1) as usize] = [
    1, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
    131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, 1, 1, 1, 1,
    1,
];

unsafe fn ZSTDv05_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let mut litLength: usize;
    let prevOffset: usize;
    let mut offset: usize;
    let mut matchLength: usize;
    let mut dumps = (*seqState).dumps;
    let de = (*seqState).dumpsEnd;

    /* Literal length */
    litLength = FSEv05_peakSymbol(&(*seqState).stateLL) as usize;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    if litLength == MaxLL as usize {
        let add: U32 = *dumps as U32;
        dumps = dumps.add(1);
        if add < 255 {
            litLength += add as usize;
        } else if dumps.add(2) <= de {
            litLength = mem_read_le16(dumps as *const c_void) as usize;
            dumps = dumps.add(2);
            if (litLength & 1) != 0 && dumps < de {
                litLength += (*dumps as usize) << 16;
                dumps = dumps.add(1);
            }
            litLength >>= 1;
        }
        if dumps >= de {
            dumps = de.offset(-1);
        }
    }

    /* Offset */
    {
        let offsetCode: U32 = FSEv05_peakSymbol(&(*seqState).stateOffb) as U32;
        let mut nbBits: U32 = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0;
        }
        offset = offsetPrefix[offsetCode as usize] as usize
            + BITv05_readBits(&mut (*seqState).DStream, nbBits);
        if mem_32bits() != 0 {
            BITv05_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset;
        }
        if (offsetCode != 0) || (litLength == 0) {
            (*seqState).prevOffset = (*seq).offset;
        }
        FSEv05_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream);
    }

    /* Literal length update */
    FSEv05_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream);
    if mem_32bits() != 0 {
        BITv05_reloadDStream(&mut (*seqState).DStream);
    }

    /* MatchLength */
    matchLength = FSEv05_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as usize;
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
        } else if dumps.add(2) <= de {
            matchLength = mem_read_le16(dumps as *const c_void) as usize;
            dumps = dumps.add(2);
            if (matchLength & 1) != 0 && dumps < de {
                matchLength += (*dumps as usize) << 16;
                dumps = dumps.add(1);
            }
            matchLength >>= 1;
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

static dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];

unsafe fn ZSTDv05_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.add(sequenceLength);
    let oend_8 = oend.offset(-8);
    let litEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.offset(-(sequence.offset as isize)) as *const BYTE;

    /* checks */
    let seqLength: usize = sequence.litLength + sequence.matchLength;

    if seqLength > oend.offset_from(op) as usize {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as usize {
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

    /* copy Literals */
    ZSTDv05_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = litEnd;

    /* copy Match */
    if sequence.offset > oLitEnd.offset_from(base) as usize {
        /* offset beyond prefix */
        if sequence.offset > oLitEnd.offset_from(vBase) as usize {
            return error(code::CORRUPTION_DETECTED);
        }
        match_ = dictEnd.offset(-(base.offset_from(match_)));
        if match_.add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = dictEnd.offset_from(match_) as usize;
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
    /* Requirement: op <= oend_8 */

    /* match within prefix */
    if sequence.offset < 8 {
        /* close range match, overlap */
        let sub2 = dec64table[sequence.offset];
        *op.add(0) = *match_.add(0);
        *op.add(1) = *match_.add(1);
        *op.add(2) = *match_.add(2);
        *op.add(3) = *match_.add(3);
        match_ = match_.add(dec32table[sequence.offset] as usize);
        ZSTDv05_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.offset(-(sub2 as isize));
    } else {
        ZSTDv05_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.add(8);
    match_ = match_.add(8);

    if oMatchEnd > oend.offset(-(16 - MINMATCH as isize)) {
        if op < oend_8 {
            ZSTDv05_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                oend_8.offset_from(op),
            );
            match_ = match_.offset(oend_8.offset_from(op));
            op = oend_8;
        }
        while op < oMatchEnd {
            *op = *match_;
            op = op.add(1);
            match_ = match_.add(1);
        }
    } else {
        ZSTDv05_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize - 8,
        );
    }
    sequenceLength
}

unsafe fn ZSTDv05_decompressSequences(
    dctx: *mut ZSTDv05_DCtx,
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

    /* Build Decoding Tables */
    errorCode = ZSTDv05_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        seqSize,
        (*dctx).flagStaticTables,
    );
    if ZSTDv05_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.add(errorCode);

    /* Regen sequences */
    if nbSeq != 0 {
        let mut sequence: seq_t = core::mem::zeroed();
        let mut seqState: seqState_t = core::mem::zeroed();

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        sequence.offset = REPCODE_STARTVALUE;
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.add(dumpsLength);
        seqState.prevOffset = REPCODE_STARTVALUE;
        errorCode = BITv05_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            iend.offset_from(ip) as usize,
        );
        if ERR_isError(errorCode) != 0 {
            return error(code::CORRUPTION_DETECTED);
        }
        FSEv05_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSEv05_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSEv05_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BITv05_reloadDStream(&mut seqState.DStream) <= BITv05_DStream_completed)
            && nbSeq != 0
        {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTDv05_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTDv05_execSequence(
                op, oend, sequence, &mut litPtr, litEnd, base, vBase, dictEnd,
            );
            if ZSTDv05_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return error(code::CORRUPTION_DETECTED);
        }
    }

    /* last literal segment */
    {
        let lastLLSize = litEnd.offset_from(litPtr) as usize;
        if litPtr > litEnd {
            return error(code::CORRUPTION_DETECTED);
        }
        if op.add(lastLLSize) > oend {
            return error(code::DSTSIZE_TOOSMALL);
        }
        if lastLLSize > 0 {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    op.offset_from(ostart) as usize
}

unsafe fn ZSTDv05_checkContinuity(dctx: *mut ZSTDv05_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char).offset(
            -((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char),
        ) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

unsafe fn ZSTDv05_decompressBlock_internal(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let litCSize: usize;

    if srcSize >= BLOCKSIZE {
        return error(code::SRCSIZE_WRONG);
    }

    /* Decode literals sub-block */
    litCSize = ZSTDv05_decodeLiteralsBlock(dctx, src, srcSize);
    if ZSTDv05_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.add(litCSize);
    srcSize -= litCSize;

    ZSTDv05_decompressSequences(dctx, dst, dstCapacity, ip as *const c_void, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBlock(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv05_checkContinuity(dctx, dst);
    ZSTDv05_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize)
}

/* ZSTDv05_decompress_continueDCtx */
unsafe fn ZSTDv05_decompress_continueDCtx(
    dctx: *mut ZSTDv05_DCtx,
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
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };
    memset(
        &mut blockProperties as *mut blockProperties_t as *mut c_void,
        0,
        core::mem::size_of::<blockProperties_t>(),
    );

    /* Frame Header */
    {
        let mut frameHeaderSize: usize;
        if srcSize < ZSTDv05_frameHeaderSize_min + ZSTDv05_blockHeaderSize {
            return error(code::SRCSIZE_WRONG);
        }
        frameHeaderSize =
            ZSTDv05_decodeFrameHeader_Part1(dctx, src, ZSTDv05_frameHeaderSize_min);
        if ZSTDv05_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv05_blockHeaderSize {
            return error(code::SRCSIZE_WRONG);
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
        frameHeaderSize = ZSTDv05_decodeFrameHeader_Part2(dctx, src, frameHeaderSize);
        if ZSTDv05_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
    }

    /* Loop on each block */
    loop {
        let mut decodedSize: usize = 0;
        let cBlockSize = ZSTDv05_getcBlockSize(
            ip as *const c_void,
            iend.offset_from(ip) as usize,
            &mut blockProperties,
        );
        if ZSTDv05_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTDv05_blockHeaderSize);
        remainingSize -= ZSTDv05_blockHeaderSize;
        if cBlockSize > remainingSize {
            return error(code::SRCSIZE_WRONG);
        }

        match blockProperties.blockType {
            b if b == bt_compressed => {
                decodedSize = ZSTDv05_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            b if b == bt_raw => {
                decodedSize = ZSTDv05_copyRawBlock(
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            b if b == bt_rle => {
                return error(code::GENERIC);
            }
            b if b == bt_end => {
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

        if ZSTDv05_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op.offset_from(ostart) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv05_DCtx,
    refDCtx: *const ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv05_copyDCtx(dctx, refDCtx);
    ZSTDv05_checkContinuity(dctx, dst);
    ZSTDv05_decompress_continueDCtx(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompress_usingDict(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTDv05_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv05_checkContinuity(dctx, dst);
    ZSTDv05_decompress_continueDCtx(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressDCtx(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv05_decompress_usingDict(
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
pub unsafe extern "C" fn ZSTDv05_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* ZSTDv05_HEAPMODE == 1 */
    let regenSize: usize;
    let dctx = ZSTDv05_createDCtx();
    if dctx.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    regenSize = ZSTDv05_decompressDCtx(dctx, dst, maxDstSize, src, srcSize);
    ZSTDv05_freeDCtx(dctx);
    regenSize
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
pub unsafe extern "C" fn ZSTDv05_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTDv05_frameHeaderSize_min {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, error(code::SRCSIZE_WRONG));
        return;
    }
    if mem_read_le32(src) != ZSTDv05_MAGICNUMBER {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, error(code::PREFIX_UNKNOWN));
        return;
    }
    ip = ip.add(ZSTDv05_frameHeaderSize_min);
    remainingSize -= ZSTDv05_frameHeaderSize_min;

    /* Loop on each block */
    loop {
        let cBlockSize =
            ZSTDv05_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv05_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.add(ZSTDv05_blockHeaderSize);
        remainingSize -= ZSTDv05_blockHeaderSize;
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

    *cSize = ip.offset_from(src as *const BYTE) as usize;
    *dBound = (nbBlocks * BLOCKSIZE) as u64;
}

/* ******************************
*  Streaming Decompression API
********************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_nextSrcSizeToDecompress(dctx: *mut ZSTDv05_DCtx) -> usize {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressContinue(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* Sanity check */
    if srcSize != (*dctx).expected {
        return error(code::SRCSIZE_WRONG);
    }
    ZSTDv05_checkContinuity(dctx, dst);

    match (*dctx).stage {
        s if s == ZSTDv05ds_getFrameHeaderSize => {
            if srcSize != ZSTDv05_frameHeaderSize_min {
                return error(code::SRCSIZE_WRONG);
            }
            (*dctx).headerSize =
                ZSTDv05_decodeFrameHeader_Part1(dctx, src, ZSTDv05_frameHeaderSize_min);
            if ZSTDv05_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            memcpy(
                (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTDv05_frameHeaderSize_min,
            );
            if (*dctx).headerSize > ZSTDv05_frameHeaderSize_min {
                return error(code::GENERIC);
            }
            (*dctx).expected = 0;
            /* fallthrough */
            let result =
                ZSTDv05_decodeFrameHeader_Part2(dctx, (*dctx).headerBuffer.as_ptr() as *const c_void, (*dctx).headerSize);
            if ZSTDv05_isError(result) != 0 {
                return result;
            }
            (*dctx).expected = ZSTDv05_blockHeaderSize;
            (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
            0
        }
        s if s == ZSTDv05ds_decodeFrameHeader => {
            let result = ZSTDv05_decodeFrameHeader_Part2(
                dctx,
                (*dctx).headerBuffer.as_ptr() as *const c_void,
                (*dctx).headerSize,
            );
            if ZSTDv05_isError(result) != 0 {
                return result;
            }
            (*dctx).expected = ZSTDv05_blockHeaderSize;
            (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
            0
        }
        s if s == ZSTDv05ds_decodeBlockHeader => {
            let mut bp: blockProperties_t = blockProperties_t {
                blockType: 0,
                origSize: 0,
            };
            let blockSize = ZSTDv05_getcBlockSize(src, ZSTDv05_blockHeaderSize, &mut bp);
            if ZSTDv05_isError(blockSize) != 0 {
                return blockSize;
            }
            if bp.blockType == bt_end {
                (*dctx).expected = 0;
                (*dctx).stage = ZSTDv05ds_getFrameHeaderSize;
            } else {
                (*dctx).expected = blockSize;
                (*dctx).bType = bp.blockType;
                (*dctx).stage = ZSTDv05ds_decompressBlock;
            }
            0
        }
        s if s == ZSTDv05ds_decompressBlock => {
            let rSize: usize;
            match (*dctx).bType {
                b if b == bt_compressed => {
                    rSize = ZSTDv05_decompressBlock_internal(dctx, dst, maxDstSize, src, srcSize);
                }
                b if b == bt_raw => {
                    rSize = ZSTDv05_copyRawBlock(dst, maxDstSize, src, srcSize);
                }
                b if b == bt_rle => {
                    return error(code::GENERIC);
                }
                b if b == bt_end => {
                    rSize = 0;
                }
                _ => {
                    return error(code::GENERIC);
                }
            }
            (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
            (*dctx).expected = ZSTDv05_blockHeaderSize;
            if ZSTDv05_isError(rSize) != 0 {
                return rSize;
            }
            (*dctx).previousDstEnd = (dst as *const c_char).add(rSize) as *const c_void;
            rSize
        }
        _ => error(code::GENERIC),
    }
}

unsafe fn ZSTDv05_refDictContent(dctx: *mut ZSTDv05_DCtx, dict: *const c_void, dictSize: usize) {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const c_char).offset(
        -((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char),
    ) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const c_char).add(dictSize) as *const c_void;
}

unsafe fn ZSTDv05_loadEntropy(
    dctx: *mut ZSTDv05_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let hSize: usize;
    let offcodeHeaderSize: usize;
    let matchlengthHeaderSize: usize;
    let mut errorCode: usize;
    let litlengthHeaderSize: usize;
    let mut offcodeNCount: [S16; (MaxOff + 1) as usize] = [0; (MaxOff + 1) as usize];
    let mut offcodeMaxValue: c_uint = MaxOff;
    let mut offcodeLog: c_uint = 0;
    let mut matchlengthNCount: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
    let mut matchlengthMaxValue: c_uint = MaxML;
    let mut matchlengthLog: c_uint = 0;
    let mut litlengthNCount: [S16; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
    let mut litlengthMaxValue: c_uint = MaxLL;
    let mut litlengthLog: c_uint = 0;

    hSize = HUFv05_readDTableX4((*dctx).hufTableX4.as_mut_ptr(), dict, dictSize);
    if HUFv05_isError(hSize) != 0 {
        return error(code::DICTIONARY_CORRUPTED);
    }
    dict = (dict as *const c_char).add(hSize) as *const c_void;
    dictSize -= hSize;

    offcodeHeaderSize = FSEv05_readNCount(
        offcodeNCount.as_mut_ptr(),
        &mut offcodeMaxValue,
        &mut offcodeLog,
        dict,
        dictSize,
    );
    if FSEv05_isError(offcodeHeaderSize) != 0 {
        return error(code::DICTIONARY_CORRUPTED);
    }
    if offcodeLog > OffFSEv05Log {
        return error(code::DICTIONARY_CORRUPTED);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).OffTable.as_mut_ptr(),
        offcodeNCount.as_ptr(),
        offcodeMaxValue,
        offcodeLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return error(code::DICTIONARY_CORRUPTED);
    }
    dict = (dict as *const c_char).add(offcodeHeaderSize) as *const c_void;
    dictSize -= offcodeHeaderSize;

    matchlengthHeaderSize = FSEv05_readNCount(
        matchlengthNCount.as_mut_ptr(),
        &mut matchlengthMaxValue,
        &mut matchlengthLog,
        dict,
        dictSize,
    );
    if FSEv05_isError(matchlengthHeaderSize) != 0 {
        return error(code::DICTIONARY_CORRUPTED);
    }
    if matchlengthLog > MLFSEv05Log {
        return error(code::DICTIONARY_CORRUPTED);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).MLTable.as_mut_ptr(),
        matchlengthNCount.as_ptr(),
        matchlengthMaxValue,
        matchlengthLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return error(code::DICTIONARY_CORRUPTED);
    }
    dict = (dict as *const c_char).add(matchlengthHeaderSize) as *const c_void;
    dictSize -= matchlengthHeaderSize;

    litlengthHeaderSize = FSEv05_readNCount(
        litlengthNCount.as_mut_ptr(),
        &mut litlengthMaxValue,
        &mut litlengthLog,
        dict,
        dictSize,
    );
    if litlengthLog > LLFSEv05Log {
        return error(code::DICTIONARY_CORRUPTED);
    }
    if FSEv05_isError(litlengthHeaderSize) != 0 {
        return error(code::DICTIONARY_CORRUPTED);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).LLTable.as_mut_ptr(),
        litlengthNCount.as_ptr(),
        litlengthMaxValue,
        litlengthLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return error(code::DICTIONARY_CORRUPTED);
    }

    (*dctx).flagStaticTables = 1;
    hSize + offcodeHeaderSize + matchlengthHeaderSize + litlengthHeaderSize
}

unsafe fn ZSTDv05_decompress_insertDictionary(
    dctx: *mut ZSTDv05_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let eSize: usize;
    let magic: U32 = mem_read_le32(dict);
    if magic != ZSTDv05_DICT_MAGIC {
        /* pure content mode */
        ZSTDv05_refDictContent(dctx, dict, dictSize);
        return 0;
    }
    /* load entropy tables */
    dict = (dict as *const c_char).add(4) as *const c_void;
    dictSize -= 4;
    eSize = ZSTDv05_loadEntropy(dctx, dict, dictSize);
    if ZSTDv05_isError(eSize) != 0 {
        return error(code::DICTIONARY_CORRUPTED);
    }

    /* reference dictionary content */
    dict = (dict as *const c_char).add(eSize) as *const c_void;
    dictSize -= eSize;
    ZSTDv05_refDictContent(dctx, dict, dictSize);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBegin_usingDict(
    dctx: *mut ZSTDv05_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut errorCode: usize;
    errorCode = ZSTDv05_decompressBegin(dctx);
    if ZSTDv05_isError(errorCode) != 0 {
        return errorCode;
    }

    if !dict.is_null() && dictSize != 0 {
        errorCode = ZSTDv05_decompress_insertDictionary(dctx, dict, dictSize);
        if ZSTDv05_isError(errorCode) != 0 {
            return error(code::DICTIONARY_CORRUPTED);
        }
    }

    0
}

/* *************************************
*  ZBUFFv05 : Buffered version
***************************************/
#[inline]
unsafe fn ZBUFFv05_limitCopy(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length = if maxDstSize < srcSize {
        maxDstSize
    } else {
        srcSize
    };
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_createDCtx() -> *mut ZBUFFv05_DCtx {
    let zbc = malloc(core::mem::size_of::<ZBUFFv05_DCtx>()) as *mut ZBUFFv05_DCtx;
    if zbc.is_null() {
        return core::ptr::null_mut();
    }
    memset(zbc as *mut c_void, 0, core::mem::size_of::<ZBUFFv05_DCtx>());
    (*zbc).zc = ZSTDv05_createDCtx();
    (*zbc).stage = ZBUFFv05ds_init;
    zbc
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_freeDCtx(zbc: *mut ZBUFFv05_DCtx) -> usize {
    if zbc.is_null() {
        return 0;
    }
    ZSTDv05_freeDCtx((*zbc).zc);
    free((*zbc).inBuff as *mut c_void);
    free((*zbc).outBuff as *mut c_void);
    free(zbc as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressInitDictionary(
    zbc: *mut ZBUFFv05_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*zbc).stage = ZBUFFv05ds_readHeader;
    (*zbc).outEnd = 0;
    (*zbc).outStart = 0;
    (*zbc).inPos = 0;
    (*zbc).hPos = 0;
    ZSTDv05_decompressBegin_usingDict((*zbc).zc, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressInit(zbc: *mut ZBUFFv05_DCtx) -> usize {
    ZBUFFv05_decompressInitDictionary(zbc, core::ptr::null(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressContinue(
    zbc: *mut ZBUFFv05_DCtx,
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
        'sw: {
            'case_flush: {
                'case_load: {
                    'case_read: {
                        'case_decodeHeader: {
                            'case_loadHeader: {
                                'case_readHeader: {
                                    'case_init: {
                                        match (*zbc).stage {
                                            s if s == ZBUFFv05ds_init => break 'case_init,
                                            s if s == ZBUFFv05ds_readHeader => {
                                                break 'case_readHeader
                                            }
                                            s if s == ZBUFFv05ds_loadHeader => {
                                                break 'case_loadHeader
                                            }
                                            s if s == ZBUFFv05ds_decodeHeader => {
                                                break 'case_decodeHeader
                                            }
                                            s if s == ZBUFFv05ds_read => break 'case_read,
                                            s if s == ZBUFFv05ds_load => break 'case_load,
                                            s if s == ZBUFFv05ds_flush => break 'case_flush,
                                            _ => return error(code::GENERIC),
                                        }
                                    }
                                    /* ZBUFFv05ds_init */
                                    return error(code::INIT_MISSING);
                                }
                                /* ZBUFFv05ds_readHeader : read header from src */
                                {
                                    let headerSize =
                                        ZSTDv05_getFrameParams(&mut (*zbc).params, src, *srcSizePtr);
                                    if ZSTDv05_isError(headerSize) != 0 {
                                        return headerSize;
                                    }
                                    if headerSize != 0 {
                                        memcpy(
                                            (*zbc).headerBuffer.as_mut_ptr().add((*zbc).hPos)
                                                as *mut c_void,
                                            src,
                                            *srcSizePtr,
                                        );
                                        (*zbc).hPos += *srcSizePtr;
                                        *maxDstSizePtr = 0;
                                        (*zbc).stage = ZBUFFv05ds_loadHeader;
                                        return headerSize - (*zbc).hPos;
                                    }
                                    (*zbc).stage = ZBUFFv05ds_decodeHeader;
                                    break 'sw;
                                }
                            }
                            /* ZBUFFv05ds_loadHeader : complete header from src */
                            {
                                let mut headerSize = ZBUFFv05_limitCopy(
                                    (*zbc).headerBuffer.as_mut_ptr().add((*zbc).hPos)
                                        as *mut c_void,
                                    ZSTDv05_frameHeaderSize_max - (*zbc).hPos,
                                    src,
                                    *srcSizePtr,
                                );
                                (*zbc).hPos += headerSize;
                                ip = ip.add(headerSize);
                                headerSize = ZSTDv05_getFrameParams(
                                    &mut (*zbc).params,
                                    (*zbc).headerBuffer.as_ptr() as *const c_void,
                                    (*zbc).hPos,
                                );
                                if ZSTDv05_isError(headerSize) != 0 {
                                    return headerSize;
                                }
                                if headerSize != 0 {
                                    *maxDstSizePtr = 0;
                                    return headerSize - (*zbc).hPos;
                                }
                            }
                            /* fall-through */
                        }
                        /* ZBUFFv05ds_decodeHeader : apply header to create / resize buffers */
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
                        if (*zbc).hPos != 0 {
                            /* some data already loaded into headerBuffer : transfer into inBuff */
                            memcpy(
                                (*zbc).inBuff as *mut c_void,
                                (*zbc).headerBuffer.as_ptr() as *const c_void,
                                (*zbc).hPos,
                            );
                            (*zbc).inPos = (*zbc).hPos;
                            (*zbc).hPos = 0;
                            (*zbc).stage = ZBUFFv05ds_load;
                            break 'sw;
                        }
                        (*zbc).stage = ZBUFFv05ds_read;
                        /* fall-through */
                    }
                    /* ZBUFFv05ds_read */
                    {
                        let neededInSize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
                        if neededInSize == 0 {
                            /* end of frame */
                            (*zbc).stage = ZBUFFv05ds_init;
                            notDone = 0;
                            break 'sw;
                        }
                        if (iend.offset_from(ip) as usize) >= neededInSize {
                            /* directly decode from src */
                            let decodedSize = ZSTDv05_decompressContinue(
                                (*zbc).zc,
                                (*zbc).outBuff.add((*zbc).outStart) as *mut c_void,
                                (*zbc).outBuffSize - (*zbc).outStart,
                                ip as *const c_void,
                                neededInSize,
                            );
                            if ZSTDv05_isError(decodedSize) != 0 {
                                return decodedSize;
                            }
                            ip = ip.add(neededInSize);
                            if decodedSize == 0 {
                                break 'sw;
                            }
                            (*zbc).outEnd = (*zbc).outStart + decodedSize;
                            (*zbc).stage = ZBUFFv05ds_flush;
                            break 'sw;
                        }
                        if ip == iend {
                            notDone = 0;
                            break 'sw;
                        }
                        (*zbc).stage = ZBUFFv05ds_load;
                    }
                    /* fall-through */
                }
                /* ZBUFFv05ds_load */
                {
                    let neededInSize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
                    let toLoad = neededInSize - (*zbc).inPos;
                    let loadedSize: usize;
                    if toLoad > (*zbc).inBuffSize - (*zbc).inPos {
                        return error(code::CORRUPTION_DETECTED);
                    }
                    loadedSize = ZBUFFv05_limitCopy(
                        (*zbc).inBuff.add((*zbc).inPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        iend.offset_from(ip) as usize,
                    );
                    ip = ip.add(loadedSize);
                    (*zbc).inPos += loadedSize;
                    if loadedSize < toLoad {
                        notDone = 0;
                        break 'sw;
                    }
                    {
                        let decodedSize = ZSTDv05_decompressContinue(
                            (*zbc).zc,
                            (*zbc).outBuff.add((*zbc).outStart) as *mut c_void,
                            (*zbc).outBuffSize - (*zbc).outStart,
                            (*zbc).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv05_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        (*zbc).inPos = 0;
                        if decodedSize == 0 {
                            (*zbc).stage = ZBUFFv05ds_read;
                            break 'sw;
                        }
                        (*zbc).outEnd = (*zbc).outStart + decodedSize;
                        (*zbc).stage = ZBUFFv05ds_flush;
                    }
                }
                /* fall-through */
            }
            /* ZBUFFv05ds_flush */
            {
                let toFlushSize = (*zbc).outEnd - (*zbc).outStart;
                let flushedSize = ZBUFFv05_limitCopy(
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    (*zbc).outBuff.add((*zbc).outStart) as *const c_void,
                    toFlushSize,
                );
                op = op.add(flushedSize);
                (*zbc).outStart += flushedSize;
                if flushedSize == toFlushSize {
                    (*zbc).stage = ZBUFFv05ds_read;
                    if (*zbc).outStart + BLOCKSIZE > (*zbc).outBuffSize {
                        (*zbc).outEnd = 0;
                        (*zbc).outStart = 0;
                    }
                    break 'sw;
                }
                /* cannot flush everything */
                notDone = 0;
                break 'sw;
            }
        }
    }

    *srcSizePtr = ip.offset_from(istart) as usize;
    *maxDstSizePtr = op.offset_from(ostart) as usize;

    {
        let mut nextSrcSizeHint = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
        if nextSrcSizeHint > ZBUFFv05_blockHeaderSize {
            nextSrcSizeHint += ZBUFFv05_blockHeaderSize;
        }
        nextSrcSizeHint -= (*zbc).inPos;
        nextSrcSizeHint
    }
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv05_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv05_getErrorName(errorCode: usize) -> *const c_char {
    err_get_error_name(errorCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv05_recommendedDInSize() -> usize {
    BLOCKSIZE + ZBUFFv05_blockHeaderSize
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv05_recommendedDOutSize() -> usize {
    BLOCKSIZE
}












