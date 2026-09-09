//! Translation of `legacy/zstd_v05.c`
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::cmem::{
    BYTE, MEM_readLE16, MEM_readLE32, MEM_readLEST, MEM_writeLE16, S16, U16, U32, U64,
    ZSTD_memcpy, ZSTD_memmove, ZSTD_memset, free, malloc,
};
use crate::error_private::{
    ERR_getErrorName, ERR_isError, ERROR, ZSTD_error_GENERIC, ZSTD_error_corruption_detected,
    ZSTD_error_dictionary_corrupted, ZSTD_error_dstSize_tooSmall,
    ZSTD_error_frameParameter_unsupported, ZSTD_error_init_missing, ZSTD_error_maxSymbolValue_tooLarge,
    ZSTD_error_maxSymbolValue_tooSmall, ZSTD_error_memory_allocation, ZSTD_error_prefix_unknown,
    ZSTD_error_srcSize_wrong, ZSTD_error_tableLog_tooLarge,
};

/* *************************************
*  zstd_v05.h  -  public types/constants
***************************************/

pub type ZSTDv05_strategy = c_uint;
pub const ZSTDv05_fast: ZSTDv05_strategy = 0;
pub const ZSTDv05_greedy: ZSTDv05_strategy = 1;
pub const ZSTDv05_lazy: ZSTDv05_strategy = 2;
pub const ZSTDv05_lazy2: ZSTDv05_strategy = 3;
pub const ZSTDv05_btlazy2: ZSTDv05_strategy = 4;
pub const ZSTDv05_opt: ZSTDv05_strategy = 5;
pub const ZSTDv05_btopt: ZSTDv05_strategy = 6;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTDv05_parameters {
    pub srcSize: U64,
    pub windowLog: U32,
    pub contentLog: U32,
    pub hashLog: U32,
    pub searchLog: U32,
    pub searchLength: U32,
    pub targetLength: U32,
    pub strategy: ZSTDv05_strategy,
}

pub const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525;
pub const ZSTDv05_WINDOWLOG_ABSOLUTEMIN: U32 = 11;

/* *************************************
*  zstd_internal common constants
***************************************/

pub const ZSTDv05_DICT_MAGIC: U32 = 0xEC30A435;

pub const BLOCKSIZE: usize = 128 * (1 << 10);

static ZSTDv05_blockHeaderSize: usize = 3;
static ZSTDv05_frameHeaderSize_min: usize = 5;
pub const ZSTDv05_frameHeaderSize_max: usize = 5;

pub const BITv057: c_uint = 128;
pub const BITv056: c_uint = 64;
pub const BITv055: c_uint = 32;
pub const BITv054: c_uint = 16;
pub const BITv051: c_uint = 2;
pub const BITv050: c_uint = 1;

pub const IS_HUFv05: U32 = 0;
pub const IS_PCH: U32 = 1;
pub const IS_RAW: U32 = 2;
pub const IS_RLE: U32 = 3;

pub const MINMATCH: usize = 4;
pub const REPCODE_STARTVALUE: usize = 1;

pub const Litbits: c_uint = 8;
pub const MLbits: c_uint = 7;
pub const LLbits: c_uint = 6;
pub const Offbits: c_uint = 5;
pub const MaxLit: usize = (1 << Litbits) - 1;
pub const MaxML: usize = (1 << MLbits) - 1;
pub const MaxLL: usize = (1 << LLbits) - 1;
pub const MaxOff: usize = (1 << Offbits) - 1;
pub const MLFSEv05Log: c_uint = 10;
pub const LLFSEv05Log: c_uint = 10;
pub const OffFSEv05Log: c_uint = 9;

pub const FSEv05_ENCODING_RAW: U32 = 0;
pub const FSEv05_ENCODING_RLE: U32 = 1;
pub const FSEv05_ENCODING_STATIC: U32 = 2;
pub const FSEv05_ENCODING_DYNAMIC: U32 = 3;

pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: c_uint = 12;

pub const MIN_SEQUENCES_SIZE: usize = 1;
pub const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

pub const WILDCOPY_OVERLENGTH: usize = 8;

pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

pub type blockType_t = c_uint;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

/*-*******************************************
*  Shared functions to include for inlining
*********************************************/
unsafe fn ZSTDv05_copy8(dst: *mut c_void, src: *const c_void) {
    ZSTD_memcpy(dst, src, 8);
}

/*  ZSTDv05_wildcopy() :
*   custom version of memcpy(), can copy up to 7 bytes too many (8 bytes if length==0) */
unsafe fn ZSTDv05_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = ((op as isize).wrapping_add(length)) as *mut BYTE;
    loop {
        ZSTDv05_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if !(op < oend) {
            break;
        }
    }
}

/*-*******************************************
*  bitstream (BITv05)
*********************************************/

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

fn BITv05_highbit32(val: U32) -> c_uint {
    (val.leading_zeros() ^ 31) as c_uint
}

unsafe fn BITv05_initDStream(
    bitD: *mut BITv05_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        ZSTD_memset(
            bitD as *mut c_void,
            0,
            core::mem::size_of::<BITv05_DStream_t>(),
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        /* normal case */
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char)
            .add(srcSize)
            .sub(core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); /* endMark not present */
        }
        (*bitD).bitsConsumed = 8 - BITv05_highbit32(contain32);
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let s = (*bitD).start as *const BYTE;
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*s.add(6) as usize) << (core::mem::size_of::<usize>() * 8 - 16));
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*s.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24));
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*s.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32));
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*s.add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*s.add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*s.add(1) as usize) << 8);
        }
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); /* endMark not present */
        }
        (*bitD).bitsConsumed = 8 - BITv05_highbit32(contain32);
        (*bitD).bitsConsumed = (*bitD)
            .bitsConsumed
            .wrapping_add(((core::mem::size_of::<usize>() - srcSize) * 8) as U32);
    }

    srcSize
}

unsafe fn BITv05_lookBits(bitD: *mut BITv05_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> (bitMask.wrapping_sub(nbBits) & bitMask)
}

/*  BITv05_lookBitsFast :
*   unsafe version; only works if nbBits >= 1 */
unsafe fn BITv05_lookBitsFast(bitD: *mut BITv05_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((bitMask.wrapping_add(1).wrapping_sub(nbBits)) & bitMask)
}

unsafe fn BITv05_skipBits(bitD: *mut BITv05_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

unsafe fn BITv05_readBits(bitD: *mut BITv05_DStream_t, nbBits: c_uint) -> usize {
    let value = BITv05_lookBits(bitD, nbBits);
    BITv05_skipBits(bitD, nbBits);
    value
}

/* BITv05_readBitsFast :
*  unsafe version; only works if nbBits >= 1 */
unsafe fn BITv05_readBitsFast(bitD: *mut BITv05_DStream_t, nbBits: c_uint) -> usize {
    let value = BITv05_lookBitsFast(bitD, nbBits);
    BITv05_skipBits(bitD, nbBits);
    value
}

unsafe fn BITv05_reloadDStream(bitD: *mut BITv05_DStream_t) -> BITv05_DStream_status {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as U32 {
        /* should never happen */
        return BITv05_DStream_overflow;
    }

    if (*bitD).ptr as usize >= ((*bitD).start as usize) + core::mem::size_of::<usize>() {
        (*bitD).ptr = ((*bitD).ptr as usize).wrapping_sub(((*bitD).bitsConsumed >> 3) as usize)
            as *const c_char;
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
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
        let mut result: BITv05_DStream_status = BITv05_DStream_unfinished;
        if (((*bitD).ptr as usize).wrapping_sub(nbBytes as usize)) < ((*bitD).start as usize) {
            nbBytes = (((*bitD).ptr as usize) - ((*bitD).start as usize)) as U32; /* ptr > start */
            result = BITv05_DStream_endOfBuffer;
        }
        (*bitD).ptr = ((*bitD).ptr as usize).wrapping_sub(nbBytes as usize) as *const c_char;
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

/*  BITv05_endOfDStream */
unsafe fn BITv05_endOfDStream(DStream: *const BITv05_DStream_t) -> c_uint {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as U32)) as c_uint
}

/*-*******************************************
*  FSEv05 types
*********************************************/

pub type FSEv05_DTable = c_uint;

pub const fn FSEv05_DTABLE_SIZE_U32(maxTableLog: usize) -> usize {
    1 + (1usize << maxTableLog)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv05_DState_t {
    pub state: usize,
    pub table: *const c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv05_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv05_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
}

unsafe fn FSEv05_initDState(
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

unsafe fn FSEv05_peakSymbol(DStatePtr: *mut FSEv05_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

unsafe fn FSEv05_decodeSymbol(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv05_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

unsafe fn FSEv05_decodeSymbolFast(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv05_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

unsafe fn FSEv05_endOfDState(DStatePtr: *const FSEv05_DState_t) -> c_uint {
    ((*DStatePtr).state == 0) as c_uint
}

/* ***************************************************************
*  FSEv05 constants
*****************************************************************/
pub const FSEv05_MAX_MEMORY_USAGE: usize = 14;
pub const FSEv05_DEFAULT_MEMORY_USAGE: usize = 13;
pub const FSEv05_MAX_SYMBOL_VALUE: usize = 255;

pub const FSEv05_MAX_TABLELOG: usize = FSEv05_MAX_MEMORY_USAGE - 2;
pub const FSEv05_MAX_TABLESIZE: usize = 1usize << FSEv05_MAX_TABLELOG;
pub const FSEv05_MAXTABLESIZE_MASK: usize = FSEv05_MAX_TABLESIZE - 1;
pub const FSEv05_DEFAULT_TABLELOG: usize = FSEv05_DEFAULT_MEMORY_USAGE - 2;
pub const FSEv05_MIN_TABLELOG: usize = 5;
pub const FSEv05_TABLELOG_ABSOLUTE_MAX: usize = 15;

/* **************************************************************
*  Templates
****************************************************************/
fn FSEv05_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1)
        .wrapping_add(tableSize >> 3)
        .wrapping_add(3)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_createDTable(mut tableLog: c_uint) -> *mut FSEv05_DTable {
    if tableLog as usize > FSEv05_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv05_TABLELOG_ABSOLUTE_MAX as c_uint;
    }
    malloc(FSEv05_DTABLE_SIZE_U32(tableLog as usize) * core::mem::size_of::<U32>())
        as *mut FSEv05_DTable
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_freeDTable(dt: *mut FSEv05_DTable) {
    free(dt as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable(
    dt: *mut FSEv05_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let mut DTableH = FSEv05_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSEv05_decode_t;
    let tableSize: U32 = 1u32 << tableLog;
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let step: U32 = FSEv05_tableStep(tableSize);
    let mut symbolNext: [U16; FSEv05_MAX_SYMBOL_VALUE + 1] = [0; FSEv05_MAX_SYMBOL_VALUE + 1];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);
    let largeLimit: S16 = (1i32).wrapping_shl(tableLog.wrapping_sub(1)) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    /* Sanity Checks */
    if maxSymbolValue as usize > FSEv05_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog as usize > FSEv05_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    ZSTD_memset(
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
            let symbol: BYTE = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.add(i as usize)).nbBits =
                (tableLog.wrapping_sub(BITv05_highbit32(nextState as U32))) as BYTE;
            (*tableDecode.add(i as usize)).newState = (((nextState as U32)
                << (*tableDecode.add(i as usize)).nbBits)
                .wrapping_sub(tableSize)) as U16;
            i = i.wrapping_add(1);
        }
    }

    DTableH.fastMode = noLarge as U16;
    ZSTD_memcpy(
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
pub unsafe extern "C" fn FSEv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/*-**************************************************************
*  FSEv05 NCount encoding-decoding
****************************************************************/
fn FSEv05_abs(a: i16) -> i16 {
    if a < 0 { a.wrapping_neg() } else { a }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_readNCount(
    normalizedCounter: *mut i16,
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
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    bitStream = MEM_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) as c_int) + FSEv05_MIN_TABLELOG as c_int; /* extract tableLog */
    if nbBits > FSEv05_TABLELOG_ABSOLUTE_MAX as c_int {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1i32 << nbBits) + 1;
    threshold = 1i32 << nbBits;
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0: c_uint = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 = n0.wrapping_add(24);
                if (ip as usize) < (iend as usize) - 5 {
                    ip = ip.add(2);
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
                *normalizedCounter.add(charnum as usize) = 0;
                charnum = charnum.wrapping_add(1);
            }
            if ((ip as usize) <= (iend as usize).wrapping_sub(7))
                || ((ip as usize).wrapping_add((bitCount >> 3) as usize)
                    <= (iend as usize).wrapping_sub(4))
            {
                ip = ip.add((bitCount >> 3) as usize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: i16 = ((2 * threshold - 1) - remaining) as i16;
            let mut count: i16;

            if (bitStream & (threshold - 1) as U32) < (max as U32) {
                count = (bitStream & (threshold - 1) as U32) as i16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold - 1) as U32) as i16;
                if (count as c_int) >= threshold {
                    /* C promotes the `short` count to `int` here; truncating
                     * `threshold` to i16 instead would flip the comparison. */
                    count = count.wrapping_sub(max);
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining -= FSEv05_abs(count) as c_int;
            *normalizedCounter.add(charnum as usize) = count;
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
                ip = ip.add((bitCount >> 3) as usize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * ((iend as isize) - 4 - (ip as isize))) as c_int;
                ip = iend.sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
        }
    }
    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    *maxSVPtr = charnum.wrapping_sub(1);

    ip = ip.add(((bitCount + 7) >> 3) as usize);
    if ((ip as usize) - (istart as usize)) > hbSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    (ip as usize) - (istart as usize)
}

/*-*******************************************************
*  Decompression (Byte symbols)
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
pub unsafe extern "C" fn FSEv05_buildDTable_raw(
    dt: *mut FSEv05_DTable,
    nbBits: c_uint,
) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv05_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv05_decode_t;
    let tableSize: c_uint = 1u32 << nbBits;
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

unsafe fn FSEv05_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv05_DTable,
    fast: c_uint,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.add(maxDstSize);
    let olimit = ((omax as isize) - 3) as *mut BYTE;

    let mut bitD = BITv05_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1 = FSEv05_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2 = FSEv05_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut errorCode: usize;

    /* Init */
    errorCode = BITv05_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }

    FSEv05_initDState(&mut state1, &mut bitD, dt);
    FSEv05_initDState(&mut state2, &mut bitD, dt);

    /* 4 symbols per loop */
    while (BITv05_reloadDStream(&mut bitD) == BITv05_DStream_unfinished) && (op < olimit) {
        *op.add(0) = if fast != 0 {
            FSEv05_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv05_decodeSymbol(&mut state1, &mut bitD)
        };

        /* FSEv05_MAX_TABLELOG*2+7 > 64 is false on 64-bit : no reload */

        *op.add(1) = if fast != 0 {
            FSEv05_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv05_decodeSymbol(&mut state2, &mut bitD)
        };

        /* FSEv05_MAX_TABLELOG*4+7 > 64 is false on 64-bit : no reload */

        *op.add(2) = if fast != 0 {
            FSEv05_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv05_decodeSymbol(&mut state1, &mut bitD)
        };

        *op.add(3) = if fast != 0 {
            FSEv05_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv05_decodeSymbol(&mut state2, &mut bitD)
        };

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

        *op = if fast != 0 {
            FSEv05_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv05_decodeSymbol(&mut state1, &mut bitD)
        };
        op = op.add(1);

        if (BITv05_reloadDStream(&mut bitD) > BITv05_DStream_completed)
            || (op == omax)
            || (BITv05_endOfDStream(&bitD) != 0
                && (fast != 0 || FSEv05_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = if fast != 0 {
            FSEv05_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv05_decodeSymbol(&mut state2, &mut bitD)
        };
        op = op.add(1);
    }

    /* end ? */
    if BITv05_endOfDStream(&bitD) != 0
        && FSEv05_endOfDState(&state1) != 0
        && FSEv05_endOfDState(&state2) != 0
    {
        return (op as usize) - (ostart as usize);
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
    let mut counting: [i16; FSEv05_MAX_SYMBOL_VALUE + 1] = [0; FSEv05_MAX_SYMBOL_VALUE + 1];
    let mut dt: [c_uint; FSEv05_DTABLE_SIZE_U32(FSEv05_MAX_TABLELOG)] =
        [0; FSEv05_DTABLE_SIZE_U32(FSEv05_MAX_TABLELOG)];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv05_MAX_SYMBOL_VALUE as c_uint;
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

    /* always return, even if it is an error code */
    FSEv05_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const c_void,
        cSrcSize,
        dt.as_ptr(),
    )
}

/* **************************************************************
*  HUFv05 constants
****************************************************************/
pub const HUFv05_ABSOLUTEMAX_TABLELOG: usize = 16;
pub const HUFv05_MAX_TABLELOG: usize = 12;
pub const HUFv05_DEFAULT_TABLELOG: usize = HUFv05_MAX_TABLELOG;
pub const HUFv05_MAX_SYMBOL_VALUE: usize = 255;

pub const fn HUFv05_DTABLE_SIZE(maxTableLog: usize) -> usize {
    1 + (1usize << maxTableLog)
}

/* **************************************************************
*  Error Management
****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* *******************************************************
*  Huff0 : Huffman block decompression
*********************************************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUFv05_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUFv05_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
    pub weight: BYTE,
}

static HUFv05_readStats_l: [c_int; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];

/*  HUFv05_readStats */
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
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            oSize = HUFv05_readStats_l[iSize - 242] as usize;
            ZSTD_memset(huffWeight as *mut c_void, 1, hwSize);
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
            ip = ip.add(1);
            n = 0;
            while (n as usize) < oSize {
                *huffWeight.add(n as usize) = *ip.add((n / 2) as usize) >> 4;
                *huffWeight.add(n as usize + 1) = *ip.add((n / 2) as usize) & 15;
                n = n.wrapping_add(2);
            }
        }
    } else {
        /* header compressed with FSEv05 (normal case) */
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
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
    ZSTD_memset(
        rankStats as *mut c_void,
        0,
        (HUFv05_ABSOLUTEMAX_TABLELOG + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if (*huffWeight.add(n as usize) as usize) >= HUFv05_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *rankStats.add(*huffWeight.add(n as usize) as usize) =
            (*rankStats.add(*huffWeight.add(n as usize) as usize)).wrapping_add(1);
        weightTotal = weightTotal
            .wrapping_add((1u32.wrapping_shl(*huffWeight.add(n as usize) as u32)) >> 1);
        n = n.wrapping_add(1);
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    tableLog = BITv05_highbit32(weightTotal).wrapping_add(1);
    if (tableLog as usize) > HUFv05_ABSOLUTEMAX_TABLELOG {
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
        *rankStats.add(lastWeight as usize) =
            (*rankStats.add(lastWeight as usize)).wrapping_add(1);
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
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
    let mut huffWeight: [BYTE; HUFv05_MAX_SYMBOL_VALUE + 1] = [0; HUFv05_MAX_SYMBOL_VALUE + 1];
    let mut rankVal: [U32; HUFv05_ABSOLUTEMAX_TABLELOG + 1] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG + 1];
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv05_DEltX2;

    iSize = HUFv05_readStats(
        huffWeight.as_mut_ptr(),
        HUFv05_MAX_SYMBOL_VALUE + 1,
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
        let mut D = HUFv05_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = (tableLog.wrapping_add(1).wrapping_sub(w)) as BYTE;
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

unsafe fn HUFv05_decodeSymbolX2(
    Dstream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BITv05_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.add(val)).byte;
    BITv05_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
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
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && ((p as usize) <= (pEnd as usize).wrapping_sub(4))
    {
        *p = HUFv05_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        *p = HUFv05_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        *p = HUFv05_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        *p = HUFv05_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    /* closer to the end */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished) && (p < pEnd) {
        *p = HUFv05_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        *p = HUFv05_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    (pEnd as usize) - (pStart as usize)
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
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv05_DEltX2).add(1);
    let mut bitD = BITv05_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };

    if dstSize <= cSrcSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
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
    let errorCode: usize;

    errorCode = HUFv05_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUFv05_decompress1X2_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
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
        let oend = ostart.add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv05_DEltX2).add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BITv05_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = bitD1;
        let mut bitD3 = bitD1;
        let mut bitD4 = bitD1;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.add(6); /* jumpTable */
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = ostart.add(segmentSize);
        let opStart3 = opStart2.add(segmentSize);
        let opStart4 = opStart3.add(segmentSize);
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

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv05_reloadDStream(&mut bitD1)
            | BITv05_reloadDStream(&mut bitD2)
            | BITv05_reloadDStream(&mut bitD3)
            | BITv05_reloadDStream(&mut bitD4);
        while (endSignal == BITv05_DStream_unfinished)
            && ((op4 as usize) < ((oend as usize).wrapping_sub(7)))
        {
            for _ in 0..4 {
                *op1 = HUFv05_decodeSymbolX2(&mut bitD1, dt, dtLog);
                op1 = op1.add(1);
                *op2 = HUFv05_decodeSymbolX2(&mut bitD2, dt, dtLog);
                op2 = op2.add(1);
                *op3 = HUFv05_decodeSymbolX2(&mut bitD3, dt, dtLog);
                op3 = op3.add(1);
                *op4 = HUFv05_decodeSymbolX2(&mut bitD4, dt, dtLog);
                op4 = op4.add(1);
            }
            endSignal = BITv05_reloadDStream(&mut bitD1)
                | BITv05_reloadDStream(&mut bitD2)
                | BITv05_reloadDStream(&mut bitD3)
                | BITv05_reloadDStream(&mut bitD4);
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
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
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
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUFv05_decompress4X2_usingDTable(
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
    let mut DElt = HUFv05_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; HUFv05_ABSOLUTEMAX_TABLELOG + 1] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG + 1];
    let mut s: U32;

    /* get pre-calculated rankVal */
    ZSTD_memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv05_ABSOLUTEMAX_TABLELOG + 1]>(),
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
        }

        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s = s.wrapping_add(1);
    }
}

pub type rankVal_t = [[U32; HUFv05_ABSOLUTEMAX_TABLELOG + 1]; HUFv05_ABSOLUTEMAX_TABLELOG];

unsafe fn HUFv05_fillDTableX4(
    DTable: *mut HUFv05_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const [U32; HUFv05_ABSOLUTEMAX_TABLELOG + 1],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; HUFv05_ABSOLUTEMAX_TABLELOG + 1] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG + 1];
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int;
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    ZSTD_memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv05_ABSOLUTEMAX_TABLELOG + 1]>(),
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
            let mut minWeight: c_int = (nbBits as c_int).wrapping_add(scaleLog);
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUFv05_fillDTableX4Level2(
                DTable.add(start as usize),
                targetLog.wrapping_sub(nbBits),
                nbBits,
                (*rankValOrigin.add(nbBits as usize)).as_ptr(),
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
    let mut weightList: [BYTE; HUFv05_MAX_SYMBOL_VALUE + 1] = [0; HUFv05_MAX_SYMBOL_VALUE + 1];
    let mut sortedSymbol: [sortedSymbol_t; HUFv05_MAX_SYMBOL_VALUE + 1] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; HUFv05_MAX_SYMBOL_VALUE + 1];
    let mut rankStats: [U32; HUFv05_ABSOLUTEMAX_TABLELOG + 1] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG + 1];
    let mut rankStart0: [U32; HUFv05_ABSOLUTEMAX_TABLELOG + 2] =
        [0; HUFv05_ABSOLUTEMAX_TABLELOG + 2];
    let rankStart = rankStart0.as_mut_ptr().add(1);
    let mut rankVal: rankVal_t =
        [[0u32; HUFv05_ABSOLUTEMAX_TABLELOG + 1]; HUFv05_ABSOLUTEMAX_TABLELOG];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.add(0);
    let iSize: usize;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUFv05_DEltX4).add(1);

    if (memLog as usize) > HUFv05_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUFv05_readStats(
        weightList.as_mut_ptr(),
        HUFv05_MAX_SYMBOL_VALUE + 1,
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
    }

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
            *rankStart.add(w as usize) = (*rankStart.add(w as usize)).wrapping_add(1);
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
        let rescale: c_int = (memLog.wrapping_sub(tableLog) as c_int) - 1; /* tableLog <= memLog */
        w = 1;
        while w <= maxW {
            let current: U32 = nextRankVal;
            nextRankVal = nextRankVal.wrapping_add(
                rankStats[w as usize].wrapping_shl((w as c_int).wrapping_add(rescale) as u32),
            );
            rankVal[0][w as usize] = current;
            w = w.wrapping_add(1);
        }
        consumed = minBits;
        while consumed <= memLog.wrapping_sub(minBits) {
            w = 1;
            while w <= maxW {
                let v = rankVal[0][w as usize] >> consumed;
                rankVal[consumed as usize][w as usize] = v;
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

unsafe fn HUFv05_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv05_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    ZSTD_memcpy(op, dt.add(val) as *const c_void, 2);
    BITv05_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

unsafe fn HUFv05_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv05_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    ZSTD_memcpy(op, dt.add(val) as *const c_void, 1);
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

unsafe fn HUFv05_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv05_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && ((p as usize) < (pEnd as usize).wrapping_sub(7))
    {
        for _ in 0..4 {
            let n = HUFv05_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog);
            p = p.add(n as usize);
        }
    }

    /* closer to the end */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && ((p as usize) <= (pEnd as usize).wrapping_sub(2))
    {
        let n = HUFv05_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog);
        p = p.add(n as usize);
    }

    while (p as usize) <= (pEnd as usize).wrapping_sub(2) {
        let n = HUFv05_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog);
        p = p.add(n as usize);
    }

    if p < pEnd {
        let n = HUFv05_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog);
        p = p.add(n as usize);
    }

    (p as usize) - (pStart as usize)
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
    let oend = ostart.add(dstSize);

    let dtLog: U32 = *DTable.add(0);
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv05_DEltX4).add(1);
    let errorCode: usize;

    /* Init */
    let mut bitD = BITv05_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    errorCode = BITv05_initDStream(&mut bitD, istart as *const c_void, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }

    /* finish bitStreams one by one */
    HUFv05_decodeStreamX4(ostart, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv05_endOfDStream(&bitD) == 0 {
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
    DTable[0] = HUFv05_MAX_TABLELOG as c_uint;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv05_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUFv05_decompress1X4_usingDTable(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
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
        let oend = ostart.add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv05_DEltX4).add(1);
        let dtLog: U32 = *DTable.add(0);
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BITv05_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = bitD1;
        let mut bitD3 = bitD1;
        let mut bitD4 = bitD1;
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize;
        let istart1 = istart.add(6); /* jumpTable */
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = ostart.add(segmentSize);
        let opStart3 = opStart2.add(segmentSize);
        let opStart4 = opStart3.add(segmentSize);
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

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv05_reloadDStream(&mut bitD1)
            | BITv05_reloadDStream(&mut bitD2)
            | BITv05_reloadDStream(&mut bitD3)
            | BITv05_reloadDStream(&mut bitD4);
        while (endSignal == BITv05_DStream_unfinished)
            && ((op4 as usize) < ((oend as usize).wrapping_sub(7)))
        {
            for _ in 0..4 {
                let n1 = HUFv05_decodeSymbolX4(op1 as *mut c_void, &mut bitD1, dt, dtLog);
                op1 = op1.add(n1 as usize);
                let n2 = HUFv05_decodeSymbolX4(op2 as *mut c_void, &mut bitD2, dt, dtLog);
                op2 = op2.add(n2 as usize);
                let n3 = HUFv05_decodeSymbolX4(op3 as *mut c_void, &mut bitD3, dt, dtLog);
                op3 = op3.add(n3 as usize);
                let n4 = HUFv05_decodeSymbolX4(op4 as *mut c_void, &mut bitD4, dt, dtLog);
                op4 = op4.add(n4 as usize);
            }

            endSignal = BITv05_reloadDStream(&mut bitD1)
                | BITv05_reloadDStream(&mut bitD2)
                | BITv05_reloadDStream(&mut bitD3)
                | BITv05_reloadDStream(&mut bitD4);
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
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
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
    let mut DTable: [c_uint; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)] =
        [0; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG as c_uint;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv05_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUFv05_decompress4X4_usingDTable(
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
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}

static algoTime: [[algo_time_t; 3]; 16] = {
    const fn a(t: U32, d: U32) -> algo_time_t {
        algo_time_t {
            tableTime: t,
            decode256Time: d,
        }
    }
    [
        [a(0, 0), a(1, 1), a(2, 2)],           /* Q==0 : impossible */
        [a(0, 0), a(1, 1), a(2, 2)],           /* Q==1 : impossible */
        [a(38, 130), a(1313, 74), a(2151, 38)], /* Q == 2 : 12-18% */
        [a(448, 128), a(1353, 74), a(2238, 41)], /* Q == 3 : 18-25% */
        [a(556, 128), a(1353, 74), a(2238, 47)], /* Q == 4 : 25-32% */
        [a(714, 128), a(1418, 74), a(2436, 53)], /* Q == 5 : 32-38% */
        [a(883, 128), a(1437, 74), a(2464, 61)], /* Q == 6 : 38-44% */
        [a(897, 128), a(1515, 75), a(2622, 68)], /* Q == 7 : 44-50% */
        [a(926, 128), a(1613, 75), a(2730, 75)], /* Q == 8 : 50-56% */
        [a(947, 128), a(1729, 77), a(3359, 77)], /* Q == 9 : 56-62% */
        [a(1107, 128), a(2083, 81), a(4006, 84)], /* Q ==10 : 62-69% */
        [a(1177, 128), a(2379, 87), a(4785, 88)], /* Q ==11 : 69-75% */
        [a(1242, 128), a(2415, 93), a(5155, 84)], /* Q ==12 : 75-81% */
        [a(1349, 128), a(2644, 106), a(5260, 106)], /* Q ==13 : 81-87% */
        [a(1455, 128), a(2422, 124), a(4174, 124)], /* Q ==14 : 87-93% */
        [a(722, 128), a(1891, 145), a(1936, 146)], /* Q ==15 : 93-99% */
    ]
};

pub type decompressionAlgo =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;

static HUFv05_decompress_algos: [Option<decompressionAlgo>; 3] = [
    Some(HUFv05_decompress4X2),
    Some(HUFv05_decompress4X4),
    None,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress(
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
    if cSrcSize >= dstSize {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if cSrcSize == 1 {
        ZSTD_memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    } /* RLE */

    /* decoder timing evaluation */
    Q = (cSrcSize * 16 / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
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
    Dtime[2] = Dtime[2].wrapping_add(Dtime[2] >> 3);

    if Dtime[1] < Dtime[0] {
        algoNb = 1;
    }

    let f = HUFv05_decompress_algos[algoNb as usize];
    (f.unwrap_unchecked())(dst, dstSize, cSrc, cSrcSize)
}

/*-*************************************
*  Local types
***************************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

/* *******************************************************
*  Memory operations
**********************************************************/
unsafe fn ZSTDv05_copy4(dst: *mut c_void, src: *const c_void) {
    ZSTD_memcpy(dst, src, 4);
}

/* *************************************
*  Error Management
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* *************************************************************
*   Context management
***************************************************************/
pub type ZSTDv05_dStage = c_uint;
pub const ZSTDv05ds_getFrameHeaderSize: ZSTDv05_dStage = 0;
pub const ZSTDv05ds_decodeFrameHeader: ZSTDv05_dStage = 1;
pub const ZSTDv05ds_decodeBlockHeader: ZSTDv05_dStage = 2;
pub const ZSTDv05ds_decompressBlock: ZSTDv05_dStage = 3;

#[repr(C)]
pub struct ZSTDv05_DCtx {
    pub LLTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(LLFSEv05Log as usize)],
    pub OffTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(OffFSEv05Log as usize)],
    pub MLTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(MLFSEv05Log as usize)],
    pub hufTableX4: [c_uint; HUFv05_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG as usize)],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: usize,
    pub headerSize: usize,
    pub params: ZSTDv05_parameters,
    pub bType: blockType_t,
    pub stage: ZSTDv05_dStage,
    pub flagStaticTables: U32,
    pub litPtr: *const BYTE,
    pub litSize: usize,
    pub litBuffer: [BYTE; BLOCKSIZE + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [BYTE; ZSTDv05_frameHeaderSize_max],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_sizeofDCtx() -> usize {
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
    0 /* reserved as a potential error code in the future */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_copyDCtx(
    dstDCtx: *mut ZSTDv05_DCtx,
    srcDCtx: *const ZSTDv05_DCtx,
) {
    ZSTD_memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv05_DCtx>()
            - (BLOCKSIZE + WILDCOPY_OVERLENGTH + ZSTDv05_frameHeaderSize_max),
    ); /* no need to copy workspace */
}

/* *************************************************************
*   Decompression section
***************************************************************/

/** ZSTDv05_decodeFrameHeader_Part1() */
unsafe fn ZSTDv05_decodeFrameHeader_Part1(
    zc: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let magicNumber: U32;
    if srcSize != ZSTDv05_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTDv05_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
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
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTDv05_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    ZSTD_memset(
        params as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv05_parameters>(),
    );
    (*params).windowLog =
        ((*(src as *const BYTE).add(4) & 15) as U32).wrapping_add(ZSTDv05_WINDOWLOG_ABSOLUTEMIN);
    if (*(src as *const BYTE).add(4) >> 4) != 0 {
        return ERROR(ZSTD_error_frameParameter_unsupported); /* reserved bits */
    }
    0
}

/** ZSTDv05_decodeFrameHeader_Part2() */
unsafe fn ZSTDv05_decodeFrameHeader_Part2(
    zc: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result: usize;
    if srcSize != (*zc).headerSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    result = ZSTDv05_getFrameParams(&mut (*zc).params, src, srcSize);
    /* MEM_32bits() is false on this target : the windowLog check is skipped */
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
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    headerFlags = *in_;
    cSize = (*in_.add(2) as U32)
        .wrapping_add((*in_.add(1) as U32) << 8)
        .wrapping_add(((*in_.add(0) & 7) as U32) << 16);

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

unsafe fn ZSTDv05_copyRawBlock(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if dst.is_null() {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    ZSTD_memcpy(dst, src, srcSize);
    srcSize
}

/*  ZSTDv05_decodeLiteralsBlock() */
unsafe fn ZSTDv05_decodeLiteralsBlock(
    dctx: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;

    /* any compressed block with literals segment must be at least this size */
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart.add(0) >> 6) as U32 {
        IS_HUFv05 => {
            let litSize: usize;
            let litCSize: usize;
            let mut singleStream: usize = 0;
            let mut lhSize: U32 = ((*istart.add(0)) >> 4) as U32 & 3;
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            match lhSize {
                2 => {
                    /* 2 - 2 - 14 - 14 */
                    lhSize = 4;
                    litSize = (((*istart.add(0) & 15) as usize) << 10)
                        + ((*istart.add(1) as usize) << 2)
                        + ((*istart.add(2) as usize) >> 6);
                    litCSize =
                        (((*istart.add(2) & 63) as usize) << 8) + (*istart.add(3) as usize);
                }
                3 => {
                    /* 2 - 2 - 18 - 18 */
                    lhSize = 5;
                    litSize = (((*istart.add(0) & 15) as usize) << 14)
                        + ((*istart.add(1) as usize) << 6)
                        + ((*istart.add(2) as usize) >> 2);
                    litCSize = (((*istart.add(2) & 3) as usize) << 16)
                        + ((*istart.add(3) as usize) << 8)
                        + (*istart.add(4) as usize);
                }
                _ => {
                    /* 2 - 2 - 10 - 10 */
                    lhSize = 3;
                    singleStream = (*istart.add(0) & 16) as usize;
                    litSize = (((*istart.add(0) & 15) as usize) << 6)
                        + ((*istart.add(1) as usize) >> 2);
                    litCSize = (((*istart.add(1) & 3) as usize) << 8) + (*istart.add(2) as usize);
                }
            }
            if litSize > BLOCKSIZE {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
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
                return ERROR(ZSTD_error_corruption_detected);
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            ZSTD_memset(
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
                /* only case supported for now : small litSize, single stream */
                return ERROR(ZSTD_error_corruption_detected);
            }
            if (*dctx).flagStaticTables == 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }

            /* 2 - 2 - 10 - 10 */
            lhSize = 3;
            litSize =
                (((*istart.add(0) & 15) as usize) << 6) + ((*istart.add(1) as usize) >> 2);
            litCSize = (((*istart.add(1) & 3) as usize) << 8) + (*istart.add(2) as usize);
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            errorCode = HUFv05_decompress1X4_usingDTable(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                litSize,
                istart.add(lhSize as usize) as *const c_void,
                litCSize,
                (*dctx).hufTableX4.as_ptr(),
            );
            if HUFv05_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            ZSTD_memset(
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
                    litSize = (((*istart.add(0) & 15) as usize) << 8) + (*istart.add(1) as usize);
                }
                3 => {
                    litSize = (((*istart.add(0) & 15) as usize) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + (*istart.add(2) as usize);
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) & 31) as usize;
                }
            }

            if lhSize as usize + litSize + WILDCOPY_OVERLENGTH > srcSize {
                /* risk reading beyond src buffer with wildcopy */
                if litSize + lhSize as usize > srcSize {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ZSTD_memcpy(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    istart.add(lhSize as usize) as *const c_void,
                    litSize,
                );
                (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
                (*dctx).litSize = litSize;
                ZSTD_memset(
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
                    litSize = (((*istart.add(0) & 15) as usize) << 8) + (*istart.add(1) as usize);
                }
                3 => {
                    litSize = (((*istart.add(0) & 15) as usize) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + (*istart.add(2) as usize);
                    if srcSize < 4 {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) & 31) as usize;
                }
            }
            if litSize > BLOCKSIZE {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ZSTD_memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.add(lhSize as usize) as c_int,
                litSize + WILDCOPY_OVERLENGTH,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            lhSize as usize + 1
        }
        _ => ERROR(ZSTD_error_corruption_detected), /* impossible */
    }
}

unsafe fn ZSTDv05_decodeSeqHeaders(
    nbSeq: *mut c_int,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut FSEv05_DTable,
    DTableML: *mut FSEv05_DTable,
    DTableOffb: *mut FSEv05_DTable,
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
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    *nbSeq = *ip as c_int;
    ip = ip.add(1);
    if *nbSeq == 0 {
        return 1;
    }
    if *nbSeq >= 128 {
        if ip >= iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        *nbSeq = ((*nbSeq - 128) << 8) + (*ip as c_int);
        ip = ip.add(1);
    }

    if ip >= iend {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        if ip.add(3) > iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        let mut dl = *ip.add(2) as usize;
        dl += (*ip.add(1) as usize) << 8;
        dumpsLength = dl;
        ip = ip.add(3);
    } else {
        if ip.add(2) > iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        let mut dl = *ip.add(1) as usize;
        dl += ((*ip.add(0) & 1) as usize) << 8;
        dumpsLength = dl;
        ip = ip.add(2);
    }
    *dumpsPtr = ip;
    ip = ip.add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    /* check */
    if ip > iend.sub(3) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* sequences */
    {
        let mut norm: [S16; MaxML + 1] = [0; MaxML + 1];
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
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            _ => {
                let mut max: c_uint = MaxLL as c_uint;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut LLlog,
                    ip as *const c_void,
                    (iend as usize) - (ip as usize),
                );
                if FSEv05_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if LLlog > LLFSEv05Log {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.add(headerSize);
                FSEv05_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        match Offtype {
            FSEv05_ENCODING_RLE => {
                Offlog = 0;
                if ip > iend.sub(2) {
                    return ERROR(ZSTD_error_srcSize_wrong);
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
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            _ => {
                let mut max: c_uint = MaxOff as c_uint;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut Offlog,
                    ip as *const c_void,
                    (iend as usize) - (ip as usize),
                );
                if FSEv05_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if Offlog > OffFSEv05Log {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.add(headerSize);
                FSEv05_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
            }
        }

        match MLtype {
            FSEv05_ENCODING_RLE => {
                MLlog = 0;
                if ip > iend.sub(2) {
                    return ERROR(ZSTD_error_srcSize_wrong);
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
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            _ => {
                let mut max: c_uint = MaxML as c_uint;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut MLlog,
                    ip as *const c_void,
                    (iend as usize) - (ip as usize),
                );
                if FSEv05_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if MLlog > MLFSEv05Log {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.add(headerSize);
                FSEv05_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
            }
        }
    }

    (ip as usize) - (istart as usize)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct seq_t {
    pub litLength: usize,
    pub matchLength: usize,
    pub offset: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct seqState_t {
    pub DStream: BITv05_DStream_t,
    pub stateLL: FSEv05_DState_t,
    pub stateOffb: FSEv05_DState_t,
    pub stateML: FSEv05_DState_t,
    pub prevOffset: usize,
    pub dumps: *const BYTE,
    pub dumpsEnd: *const BYTE,
}

static ZSTDv05_offsetPrefix: [U32; MaxOff + 1] = [
    1, /*fake*/ 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
    65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432,
    /*fake*/ 1, 1, 1, 1, 1,
];

unsafe fn ZSTDv05_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let mut litLength: usize;
    let prevOffset: usize;
    let mut offset: usize;
    let mut matchLength: usize;
    let mut dumps: *const BYTE = (*seqState).dumps;
    let de: *const BYTE = (*seqState).dumpsEnd;

    /* Literal length */
    litLength = FSEv05_peakSymbol(&mut (*seqState).stateLL) as usize;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    if litLength == MaxLL {
        let add: U32 = *dumps as U32;
        dumps = dumps.add(1);
        if add < 255 {
            litLength = litLength.wrapping_add(add as usize);
        } else if (dumps as usize) + 2 <= (de as usize) {
            litLength = MEM_readLE16(dumps as *const c_void) as usize;
            dumps = dumps.add(2);
            if (litLength & 1) != 0 && dumps < de {
                litLength = litLength.wrapping_add(((*dumps as c_int) << 16) as usize);
                dumps = dumps.add(1);
            }
            litLength >>= 1;
        }
        if dumps >= de {
            dumps = ((de as isize) - 1) as *const BYTE;
        } /* late correction */
    }

    /* Offset */
    {
        let offsetCode: U32 = FSEv05_peakSymbol(&mut (*seqState).stateOffb) as U32;
        let mut nbBits: U32 = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0;
        } /* cmove */
        offset = (ZSTDv05_offsetPrefix[offsetCode as usize] as usize)
            .wrapping_add(BITv05_readBits(&mut (*seqState).DStream, nbBits));
        /* MEM_32bits() is false : no reload */
        if offsetCode == 0 {
            offset = prevOffset;
        } /* repcode, cmove */
        if (offsetCode | ((litLength == 0) as U32)) != 0 {
            (*seqState).prevOffset = (*seq).offset;
        } /* cmove */
        FSEv05_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream); /* update */
    }

    /* Literal length update */
    FSEv05_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream); /* update */
    /* MEM_32bits() is false : no reload */

    /* MatchLength */
    matchLength =
        FSEv05_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as usize;
    if matchLength == MaxML {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.add(1);
            v
        } else {
            0
        };
        if add < 255 {
            matchLength = matchLength.wrapping_add(add as usize);
        } else if (dumps as usize) + 2 <= (de as usize) {
            matchLength = MEM_readLE16(dumps as *const c_void) as usize;
            dumps = dumps.add(2);
            if (matchLength & 1) != 0 && dumps < de {
                matchLength = matchLength.wrapping_add(((*dumps as c_int) << 16) as usize);
                dumps = dumps.add(1);
            }
            matchLength >>= 1;
        }
        if dumps >= de {
            dumps = ((de as isize) - 1) as *const BYTE;
        } /* late correction */
    }
    matchLength = matchLength.wrapping_add(MINMATCH);

    /* save result */
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

static ZSTDv05_dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
static ZSTDv05_dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */

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
    let sequenceLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd = op.add(sequenceLength); /* risk : address space overflow (32-bits) */
    let oend_8 = ((oend as isize) - 8) as *mut BYTE;
    let litEnd = (*litPtr).add(sequence.litLength);
    let mut r#match: *const BYTE =
        ((oLitEnd as usize).wrapping_sub(sequence.offset)) as *const BYTE;

    /* checks */
    let seqLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > ((oend as usize) - (op as usize)) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > ((litLimit as usize).wrapping_sub(*litPtr as usize)) {
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
    ZSTDv05_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = litEnd; /* update for next sequence */

    /* copy Match */
    if sequence.offset > ((oLitEnd as usize).wrapping_sub(base as usize)) {
        /* offset beyond prefix */
        if sequence.offset > ((oLitEnd as usize).wrapping_sub(vBase as usize)) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        r#match = ((dictEnd as isize) - ((base as isize) - (r#match as isize))) as *const BYTE;
        if (r#match as usize).wrapping_add(sequence.matchLength) <= (dictEnd as usize) {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                r#match as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = (dictEnd as usize) - (r#match as usize);
            ZSTD_memmove(oLitEnd as *mut c_void, r#match as *const c_void, length1);
            op = oLitEnd.add(length1);
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            r#match = base;
            if op > oend_8 || sequence.matchLength < MINMATCH {
                while op < oMatchEnd {
                    *op = *r#match;
                    op = op.add(1);
                    r#match = r#match.add(1);
                }
                return sequenceLength;
            }
        }
    }
    /* Requirement: op <= oend_8 */

    /* match within prefix */
    if sequence.offset < 8 {
        /* close range match, overlap */
        let sub2: c_int = ZSTDv05_dec64table[sequence.offset];
        *op.add(0) = *r#match.add(0);
        *op.add(1) = *r#match.add(1);
        *op.add(2) = *r#match.add(2);
        *op.add(3) = *r#match.add(3);
        r#match = r#match.add(ZSTDv05_dec32table[sequence.offset] as usize);
        ZSTDv05_copy4(op.add(4) as *mut c_void, r#match as *const c_void);
        r#match = ((r#match as isize) - (sub2 as isize)) as *const BYTE;
    } else {
        ZSTDv05_copy8(op as *mut c_void, r#match as *const c_void);
    }
    op = op.add(8);
    r#match = r#match.add(8);

    if oMatchEnd > ((oend as isize) - (16 - MINMATCH as isize)) as *mut BYTE {
        if op < oend_8 {
            ZSTDv05_wildcopy(
                op as *mut c_void,
                r#match as *const c_void,
                (oend_8 as isize) - (op as isize),
            );
            r#match = ((r#match as isize) + ((oend_8 as isize) - (op as isize))) as *const BYTE;
            op = oend_8;
        }
        while op < oMatchEnd {
            *op = *r#match;
            op = op.add(1);
            r#match = r#match.add(1);
        }
    } else {
        ZSTDv05_wildcopy(
            op as *mut c_void,
            r#match as *const c_void,
            (sequence.matchLength as isize) - 8,
        ); /* works even if matchLength < 8 */
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
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.add((*dctx).litSize);
    let mut nbSeq: c_int = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL: *mut c_uint = (*dctx).LLTable.as_mut_ptr();
    let DTableML: *mut c_uint = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb: *mut c_uint = (*dctx).OffTable.as_mut_ptr();
    let base: *const BYTE = (*dctx).base as *const BYTE;
    let vBase: *const BYTE = (*dctx).vBase as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;

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
        let mut sequence = seq_t {
            litLength: 0,
            matchLength: 0,
            offset: 0,
        };
        let mut seqState = seqState_t {
            DStream: BITv05_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            },
            stateLL: FSEv05_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateOffb: FSEv05_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateML: FSEv05_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            prevOffset: 0,
            dumps: core::ptr::null(),
            dumpsEnd: core::ptr::null(),
        };

        ZSTD_memset(
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
            (iend as usize) - (ip as usize),
        );
        if ERR_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
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
                op,
                oend,
                sequence,
                &mut litPtr,
                litEnd,
                base,
                vBase,
                dictEnd,
            );
            if ZSTDv05_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
    }

    /* last literal segment */
    {
        let lastLLSize: usize = (litEnd as usize).wrapping_sub(litPtr as usize);
        if litPtr > litEnd {
            return ERROR(ZSTD_error_corruption_detected); /* too many literals already used */
        }
        if (op as usize).wrapping_add(lastLLSize) > (oend as usize) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if lastLLSize > 0 {
            ZSTD_memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    (op as usize) - (ostart as usize)
}

unsafe fn ZSTDv05_checkContinuity(dctx: *mut ZSTDv05_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = ((dst as isize)
            - (((*dctx).previousDstEnd as isize) - ((*dctx).base as isize)))
            as *const c_void;
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
    /* blockType == blockCompressed */
    let mut ip = src as *const BYTE;
    let litCSize: usize;

    if srcSize >= BLOCKSIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
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

/*  ZSTDv05_decompress_continueDCtx */
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
    let mut remainingSize: usize = srcSize;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };
    ZSTD_memset(
        &mut blockProperties as *mut blockProperties_t as *mut c_void,
        0,
        core::mem::size_of::<blockProperties_t>(),
    );

    /* Frame Header */
    {
        let mut frameHeaderSize: usize;
        if srcSize < ZSTDv05_frameHeaderSize_min + ZSTDv05_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        frameHeaderSize =
            ZSTDv05_decodeFrameHeader_Part1(dctx, src, ZSTDv05_frameHeaderSize_min);
        if ZSTDv05_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv05_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
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
            (iend as usize) - (ip as usize),
            &mut blockProperties,
        );
        if ZSTDv05_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTDv05_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv05_blockHeaderSize);
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        match blockProperties.blockType {
            bt_compressed => {
                decodedSize = ZSTDv05_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    (oend as usize) - (op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_raw => {
                decodedSize = ZSTDv05_copyRawBlock(
                    op as *mut c_void,
                    (oend as usize) - (op as usize),
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

        if ZSTDv05_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    (op as usize) - (ostart as usize)
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
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv05_decompressDCtx(dctx, dst, maxDstSize, src, srcSize);
    ZSTDv05_freeDCtx(dctx);
    regenSize
}

/* ZSTD_errorFrameSizeInfoLegacy() :
   assumes `cSize` and `dBound` are _not_ NULL */
unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut u64, ret: usize) {
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
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTDv05_frameHeaderSize_min {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    if MEM_readLE32(src) != ZSTDv05_MAGICNUMBER {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
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
        remainingSize = remainingSize.wrapping_sub(ZSTDv05_blockHeaderSize);
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break; /* bt_end */
        }

        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = (ip as usize) - (src as usize);
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
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ZSTDv05_checkContinuity(dctx, dst);

    /* Decompress : frame header; part 1 */
    match (*dctx).stage {
        ZSTDv05ds_getFrameHeaderSize => {
            /* get frame header size */
            if srcSize != ZSTDv05_frameHeaderSize_min {
                return ERROR(ZSTD_error_srcSize_wrong); /* impossible */
            }
            (*dctx).headerSize =
                ZSTDv05_decodeFrameHeader_Part1(dctx, src, ZSTDv05_frameHeaderSize_min);
            if ZSTDv05_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            ZSTD_memcpy(
                (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTDv05_frameHeaderSize_min,
            );
            if (*dctx).headerSize > ZSTDv05_frameHeaderSize_min {
                return ERROR(ZSTD_error_GENERIC); /* should never happen */
            }
            (*dctx).expected = 0; /* not necessary to copy more */
            /* fallthrough : ZSTDv05ds_decodeFrameHeader */
            {
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
        }
        ZSTDv05ds_decodeFrameHeader => {
            /* get frame header */
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
        ZSTDv05ds_decodeBlockHeader => {
            /* Decode block header */
            let mut bp = blockProperties_t {
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
        ZSTDv05ds_decompressBlock => {
            /* Decompress : block content */
            let rSize: usize;
            match (*dctx).bType {
                bt_compressed => {
                    rSize =
                        ZSTDv05_decompressBlock_internal(dctx, dst, maxDstSize, src, srcSize);
                }
                bt_raw => {
                    rSize = ZSTDv05_copyRawBlock(dst, maxDstSize, src, srcSize);
                }
                bt_rle => {
                    return ERROR(ZSTD_error_GENERIC); /* not yet handled */
                }
                bt_end => {
                    /* should never happen (filtered at phase 1) */
                    rSize = 0;
                }
                _ => {
                    return ERROR(ZSTD_error_GENERIC); /* impossible */
                }
            }
            (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
            (*dctx).expected = ZSTDv05_blockHeaderSize;
            if ZSTDv05_isError(rSize) != 0 {
                return rSize;
            }
            (*dctx).previousDstEnd = (dst as *mut c_char).add(rSize) as *const c_void;
            rSize
        }
        _ => ERROR(ZSTD_error_GENERIC), /* impossible */
    }
}

unsafe fn ZSTDv05_refDictContent(dctx: *mut ZSTDv05_DCtx, dict: *const c_void, dictSize: usize) {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = ((dict as isize)
        - (((*dctx).previousDstEnd as isize) - ((*dctx).base as isize))) as *const c_void;
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
    let mut offcodeNCount: [i16; MaxOff + 1] = [0; MaxOff + 1];
    let mut offcodeMaxValue: c_uint = MaxOff as c_uint;
    let mut offcodeLog: c_uint = 0;
    let mut matchlengthNCount: [i16; MaxML + 1] = [0; MaxML + 1];
    let mut matchlengthMaxValue: c_uint = MaxML as c_uint;
    let mut matchlengthLog: c_uint = 0;
    let mut litlengthNCount: [i16; MaxLL + 1] = [0; MaxLL + 1];
    let mut litlengthMaxValue: c_uint = MaxLL as c_uint;
    let mut litlengthLog: c_uint = 0;

    hSize = HUFv05_readDTableX4((*dctx).hufTableX4.as_mut_ptr(), dict, dictSize);
    if HUFv05_isError(hSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
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
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if offcodeLog > OffFSEv05Log {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).OffTable.as_mut_ptr(),
        offcodeNCount.as_ptr(),
        offcodeMaxValue,
        offcodeLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
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
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if matchlengthLog > MLFSEv05Log {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).MLTable.as_mut_ptr(),
        matchlengthNCount.as_ptr(),
        matchlengthMaxValue,
        matchlengthLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
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
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if FSEv05_isError(litlengthHeaderSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).LLTable.as_mut_ptr(),
        litlengthNCount.as_ptr(),
        litlengthMaxValue,
        litlengthLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    (*dctx).flagStaticTables = 1;
    hSize
        .wrapping_add(offcodeHeaderSize)
        .wrapping_add(matchlengthHeaderSize)
        .wrapping_add(litlengthHeaderSize)
}

unsafe fn ZSTDv05_decompress_insertDictionary(
    dctx: *mut ZSTDv05_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let eSize: usize;
    let magic: U32 = MEM_readLE32(dict);
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
        return ERROR(ZSTD_error_dictionary_corrupted);
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
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }

    0
}

/* *************************************
*  ZBUFF - Constants
***************************************/
static ZBUFFv05_blockHeaderSize: usize = 3;

unsafe fn ZBUFFv05_limitCopy(
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
        ZSTD_memcpy(dst, src, length);
    }
    length
}

pub type ZBUFFv05_dStage = c_uint;
pub const ZBUFFv05ds_init: ZBUFFv05_dStage = 0;
pub const ZBUFFv05ds_readHeader: ZBUFFv05_dStage = 1;
pub const ZBUFFv05ds_loadHeader: ZBUFFv05_dStage = 2;
pub const ZBUFFv05ds_decodeHeader: ZBUFFv05_dStage = 3;
pub const ZBUFFv05ds_read: ZBUFFv05_dStage = 4;
pub const ZBUFFv05ds_load: ZBUFFv05_dStage = 5;
pub const ZBUFFv05ds_flush: ZBUFFv05_dStage = 6;

/* *** Resource management *** */
#[repr(C)]
pub struct ZBUFFv05_DCtx {
    pub zc: *mut ZSTDv05_DCtx,
    pub params: ZSTDv05_parameters,
    pub inBuff: *mut c_char,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub outBuff: *mut c_char,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub hPos: usize,
    pub stage: ZBUFFv05_dStage,
    pub headerBuffer: [BYTE; ZSTDv05_frameHeaderSize_max],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_createDCtx() -> *mut ZBUFFv05_DCtx {
    let zbc = malloc(core::mem::size_of::<ZBUFFv05_DCtx>()) as *mut ZBUFFv05_DCtx;
    if zbc.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_memset(
        zbc as *mut c_void,
        0,
        core::mem::size_of::<ZBUFFv05_DCtx>(),
    );
    (*zbc).zc = ZSTDv05_createDCtx();
    (*zbc).stage = ZBUFFv05ds_init;
    zbc
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_freeDCtx(zbc: *mut ZBUFFv05_DCtx) -> usize {
    if zbc.is_null() {
        return 0; /* support free on null */
    }
    ZSTDv05_freeDCtx((*zbc).zc);
    free((*zbc).inBuff as *mut c_void);
    free((*zbc).outBuff as *mut c_void);
    free(zbc as *mut c_void);
    0
}

/* *** Initialization *** */
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

/* *** Decompression *** */
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
        let mut cur: ZBUFFv05_dStage = (*zbc).stage;
        'sw: loop {
            match cur {
                ZBUFFv05ds_init => {
                    return ERROR(ZSTD_error_init_missing);
                }

                ZBUFFv05ds_readHeader => {
                    /* read header from src */
                    let headerSize = ZSTDv05_getFrameParams(&mut (*zbc).params, src, *srcSizePtr);
                    if ZSTDv05_isError(headerSize) != 0 {
                        return headerSize;
                    }
                    if headerSize != 0 {
                        /* not enough input to decode header : tell how many bytes would be necessary */
                        ZSTD_memcpy(
                            (*zbc).headerBuffer.as_mut_ptr().add((*zbc).hPos) as *mut c_void,
                            src,
                            *srcSizePtr,
                        );
                        (*zbc).hPos += *srcSizePtr;
                        *maxDstSizePtr = 0;
                        (*zbc).stage = ZBUFFv05ds_loadHeader;
                        return headerSize.wrapping_sub((*zbc).hPos);
                    }
                    (*zbc).stage = ZBUFFv05ds_decodeHeader;
                    break 'sw;
                }

                ZBUFFv05ds_loadHeader => {
                    /* complete header from src */
                    let mut headerSize = ZBUFFv05_limitCopy(
                        (*zbc).headerBuffer.as_mut_ptr().add((*zbc).hPos) as *mut c_void,
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
                        /* not enough input to decode header */
                        *maxDstSizePtr = 0;
                        return headerSize.wrapping_sub((*zbc).hPos);
                    }
                    /* fall-through */
                    cur = ZBUFFv05ds_decodeHeader;
                    continue 'sw;
                }

                ZBUFFv05ds_decodeHeader => {
                    /* apply header to create / resize buffers */
                    {
                        let neededOutSize: usize = 1usize << (*zbc).params.windowLog;
                        let neededInSize: usize = BLOCKSIZE; /* a block is never > BLOCKSIZE */
                        if (*zbc).inBuffSize < neededInSize {
                            free((*zbc).inBuff as *mut c_void);
                            (*zbc).inBuffSize = neededInSize;
                            (*zbc).inBuff = malloc(neededInSize) as *mut c_char;
                            if (*zbc).inBuff.is_null() {
                                return ERROR(ZSTD_error_memory_allocation);
                            }
                        }
                        if (*zbc).outBuffSize < neededOutSize {
                            free((*zbc).outBuff as *mut c_void);
                            (*zbc).outBuffSize = neededOutSize;
                            (*zbc).outBuff = malloc(neededOutSize) as *mut c_char;
                            if (*zbc).outBuff.is_null() {
                                return ERROR(ZSTD_error_memory_allocation);
                            }
                        }
                    }
                    if (*zbc).hPos != 0 {
                        /* some data already loaded into headerBuffer : transfer into inBuff */
                        ZSTD_memcpy(
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
                    cur = ZBUFFv05ds_read;
                    continue 'sw;
                }

                ZBUFFv05ds_read => {
                    let neededInSize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
                    if neededInSize == 0 {
                        /* end of frame */
                        (*zbc).stage = ZBUFFv05ds_init;
                        notDone = 0;
                        break 'sw;
                    }
                    if ((iend as usize) - (ip as usize)) >= neededInSize {
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
                            break 'sw; /* this was just a header */
                        }
                        (*zbc).outEnd = (*zbc).outStart + decodedSize;
                        (*zbc).stage = ZBUFFv05ds_flush;
                        break 'sw;
                    }
                    if ip == iend {
                        notDone = 0;
                        break 'sw;
                    } /* no more input */
                    (*zbc).stage = ZBUFFv05ds_load;
                    /* fall-through */
                    cur = ZBUFFv05ds_load;
                    continue 'sw;
                }

                ZBUFFv05ds_load => {
                    let neededInSize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
                    let toLoad: usize = neededInSize.wrapping_sub((*zbc).inPos);
                    let loadedSize: usize;
                    if toLoad > (*zbc).inBuffSize.wrapping_sub((*zbc).inPos) {
                        return ERROR(ZSTD_error_corruption_detected); /* should never happen */
                    }
                    loadedSize = ZBUFFv05_limitCopy(
                        (*zbc).inBuff.add((*zbc).inPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        (iend as usize) - (ip as usize),
                    );
                    ip = ip.add(loadedSize);
                    (*zbc).inPos += loadedSize;
                    if loadedSize < toLoad {
                        notDone = 0;
                        break 'sw;
                    } /* not enough input, wait for more */
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
                        (*zbc).inPos = 0; /* input is consumed */
                        if decodedSize == 0 {
                            (*zbc).stage = ZBUFFv05ds_read;
                            break 'sw;
                        } /* this was just a header */
                        (*zbc).outEnd = (*zbc).outStart + decodedSize;
                        (*zbc).stage = ZBUFFv05ds_flush;
                        /* ZBUFFv05ds_flush follows */
                    }
                    cur = ZBUFFv05ds_flush;
                    continue 'sw;
                }

                ZBUFFv05ds_flush => {
                    let toFlushSize: usize = (*zbc).outEnd - (*zbc).outStart;
                    let flushedSize = ZBUFFv05_limitCopy(
                        op as *mut c_void,
                        (oend as usize) - (op as usize),
                        (*zbc).outBuff.add((*zbc).outStart) as *const c_void,
                        toFlushSize,
                    );
                    op = op.add(flushedSize);
                    (*zbc).outStart += flushedSize;
                    if flushedSize == toFlushSize {
                        (*zbc).stage = ZBUFFv05ds_read;
                        if (*zbc).outStart + BLOCKSIZE > (*zbc).outBuffSize {
                            (*zbc).outStart = 0;
                            (*zbc).outEnd = 0;
                        }
                        break 'sw;
                    }
                    /* cannot flush everything */
                    notDone = 0;
                    break 'sw;
                }

                _ => {
                    return ERROR(ZSTD_error_GENERIC); /* impossible */
                }
            }
        }
    }

    *srcSizePtr = (ip as usize) - (istart as usize);
    *maxDstSizePtr = (op as usize) - (ostart as usize);

    {
        let mut nextSrcSizeHint = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
        if nextSrcSizeHint > ZBUFFv05_blockHeaderSize {
            nextSrcSizeHint += ZBUFFv05_blockHeaderSize; /* get next block header too */
        }
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbc).inPos); /* already loaded */
        nextSrcSizeHint
    }
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_recommendedDInSize() -> usize {
    BLOCKSIZE + ZBUFFv05_blockHeaderSize /* block header size */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_recommendedDOutSize() -> usize {
    BLOCKSIZE
}
