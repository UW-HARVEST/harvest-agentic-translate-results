//! Translation of `legacy/zstd_v06.c`
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::cmem::{
    free, malloc, MEM_32bits, MEM_64bits, MEM_readLE16, MEM_readLE32, MEM_readLE64, MEM_readLEST,
    MEM_writeLE16, ZSTD_memcpy, ZSTD_memmove, ZSTD_memset, BYTE, S16, U16, U32, U64,
};
use crate::error_private::{
    ERR_getErrorName, ERR_isError, ERROR, ZSTD_error_GENERIC, ZSTD_error_corruption_detected,
    ZSTD_error_dictionary_corrupted, ZSTD_error_dstSize_tooSmall,
    ZSTD_error_frameParameter_unsupported, ZSTD_error_init_missing, ZSTD_error_maxSymbolValue_tooLarge,
    ZSTD_error_maxSymbolValue_tooSmall, ZSTD_error_memory_allocation, ZSTD_error_prefix_unknown,
    ZSTD_error_srcSize_wrong, ZSTD_error_tableLog_tooLarge,
};

/* ******************************************************************
 *  zstd_internal (v06)
 ********************************************************************/

pub const ZSTDv06_DICT_MAGIC: U32 = 0xEC30A436;

pub const ZSTDv06_REP_NUM: usize = 3;
pub const ZSTDv06_REP_INIT: usize = ZSTDv06_REP_NUM;
pub const ZSTDv06_REP_MOVE: usize = ZSTDv06_REP_NUM - 1;

pub const ZSTDv06_WINDOWLOG_ABSOLUTEMIN: U32 = 12;

static ZSTDv06_fcs_fieldSize: [usize; 4] = [0, 1, 2, 8];

pub const ZSTDv06_BLOCKHEADERSIZE: usize = 3;
pub const ZSTDv06_blockHeaderSize: usize = ZSTDv06_BLOCKHEADERSIZE;

pub type blockType_t = c_uint;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

pub const MIN_SEQUENCES_SIZE: usize = 1;
pub const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: U32 = 12;

pub const IS_HUF: U32 = 0;
pub const IS_PCH: U32 = 1;
pub const IS_RAW: U32 = 2;
pub const IS_RLE: U32 = 3;

pub const LONGNBSEQ: U32 = 0x7F00;

pub const MINMATCH: usize = 3;
pub const EQUAL_READ32: usize = 4;
pub const REPCODE_STARTVALUE: usize = 1;

pub const Litbits: U32 = 8;
pub const MaxLit: U32 = (1 << Litbits) - 1;
pub const MaxML: U32 = 52;
pub const MaxLL: U32 = 35;
pub const MaxOff: U32 = 28;
pub const MaxSeq: U32 = MaxML;
pub const MLFSELog: U32 = 9;
pub const LLFSELog: U32 = 9;
pub const OffFSELog: U32 = 8;

pub const FSEv06_ENCODING_RAW: U32 = 0;
pub const FSEv06_ENCODING_RLE: U32 = 1;
pub const FSEv06_ENCODING_STATIC: U32 = 2;
pub const FSEv06_ENCODING_DYNAMIC: U32 = 3;

pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

static LL_bits: [U32; 36] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
static LL_defaultNorm: [S16; 36] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
static LL_defaultNormLog: U32 = 6;

static ML_bits: [U32; 53] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
static ML_defaultNorm: [S16; 53] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
static ML_defaultNormLog: U32 = 6;

static OF_defaultNorm: [S16; 29] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
static OF_defaultNormLog: U32 = 5;

/*-*******************************************
 *  Shared functions to include for inlining
 *********************************************/
#[inline(always)]
unsafe fn ZSTDv06_copy8(dst: *mut c_void, src: *const c_void) {
    let v = (src as *const u64).read_unaligned();
    (dst as *mut u64).write_unaligned(v);
}

pub const WILDCOPY_OVERLENGTH: usize = 8;

#[inline(always)]
unsafe fn ZSTDv06_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = (op as usize).wrapping_add(length as usize) as *mut BYTE;
    loop {
        ZSTDv06_copy8(op as *mut c_void, ip as *const c_void);
        op = (op as usize).wrapping_add(8) as *mut BYTE;
        ip = (ip as usize).wrapping_add(8) as *const BYTE;
        if !((op as usize) < (oend as usize)) {
            break;
        }
    }
}

/* ******************************************************************
 *  bitstream (v06)
 ********************************************************************/

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BITv06_DStream_t {
    pub bitContainer: usize,
    pub bitsConsumed: c_uint,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

pub type BITv06_DStream_status = c_uint;
pub const BITv06_DStream_unfinished: BITv06_DStream_status = 0;
pub const BITv06_DStream_endOfBuffer: BITv06_DStream_status = 1;
pub const BITv06_DStream_completed: BITv06_DStream_status = 2;
pub const BITv06_DStream_overflow: BITv06_DStream_status = 3;

#[inline(always)]
fn BITv06_highbit32(val: U32) -> c_uint {
    31u32 ^ val.leading_zeros()
}

unsafe fn BITv06_initDStream(
    bitD: *mut BITv06_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        ZSTD_memset(
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
            .add(srcSize)
            .sub(core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        {
            let lastByte: BYTE = *(srcBuffer as *const BYTE).add(srcSize - 1);
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC); /* endMark not present */
            }
            (*bitD).bitsConsumed = 8 - BITv06_highbit32(lastByte as U32);
        }
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let sb = srcBuffer as *const BYTE;
        let nbits = core::mem::size_of::<usize>() * 8;
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(6) as usize) << (nbits - 16));
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(5) as usize) << (nbits - 24));
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(4) as usize) << (nbits - 32));
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(1) as usize) << 8);
        }
        {
            let lastByte: BYTE = *sb.add(srcSize - 1);
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC); /* endMark not present */
            }
            (*bitD).bitsConsumed = 8 - BITv06_highbit32(lastByte as U32);
        }
        (*bitD).bitsConsumed = (*bitD)
            .bitsConsumed
            .wrapping_add(((core::mem::size_of::<usize>() - srcSize) * 8) as U32);
    }

    srcSize
}

#[inline(always)]
unsafe fn BITv06_lookBits(bitD: *const BITv06_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> (bitMask.wrapping_sub(nbBits) & bitMask)
}

#[inline(always)]
unsafe fn BITv06_lookBitsFast(bitD: *const BITv06_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((bitMask.wrapping_add(1).wrapping_sub(nbBits)) & bitMask)
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

#[inline(always)]
unsafe fn BITv06_readBitsFast(bitD: *mut BITv06_DStream_t, nbBits: U32) -> usize {
    let value = BITv06_lookBitsFast(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

unsafe fn BITv06_reloadDStream(bitD: *mut BITv06_DStream_t) -> BITv06_DStream_status {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as U32 {
        return BITv06_DStream_overflow;
    }

    if (*bitD).ptr as usize >= ((*bitD).start as usize) + core::mem::size_of::<usize>() {
        (*bitD).ptr =
            ((*bitD).ptr as usize).wrapping_sub(((*bitD).bitsConsumed >> 3) as usize) as *const c_char;
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BITv06_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as U32 {
            return BITv06_DStream_endOfBuffer;
        }
        return BITv06_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv06_DStream_status = BITv06_DStream_unfinished;
        if ((*bitD).ptr as usize).wrapping_sub(nbBytes as usize) < (*bitD).start as usize {
            nbBytes = (((*bitD).ptr as usize) - ((*bitD).start as usize)) as U32;
            result = BITv06_DStream_endOfBuffer;
        }
        (*bitD).ptr = ((*bitD).ptr as usize).wrapping_sub(nbBytes as usize) as *const c_char;
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes * 8);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline(always)]
unsafe fn BITv06_endOfDStream(DStream: *const BITv06_DStream_t) -> c_uint {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as U32)) as c_uint
}

/* ******************************************************************
 *  FSE (v06) static definitions
 ********************************************************************/

pub type FSEv06_DTable = c_uint;

pub const FSEv06_NCOUNTBOUND: usize = 512;

pub const FSEv06_MAX_MEMORY_USAGE: U32 = 14;
pub const FSEv06_DEFAULT_MEMORY_USAGE: U32 = 13;
pub const FSEv06_MAX_SYMBOL_VALUE: U32 = 255;

pub const FSEv06_MAX_TABLELOG: U32 = FSEv06_MAX_MEMORY_USAGE - 2;
pub const FSEv06_MAX_TABLESIZE: U32 = 1u32 << FSEv06_MAX_TABLELOG;
pub const FSEv06_MAXTABLESIZE_MASK: U32 = FSEv06_MAX_TABLESIZE - 1;
pub const FSEv06_DEFAULT_TABLELOG: U32 = FSEv06_DEFAULT_MEMORY_USAGE - 2;
pub const FSEv06_MIN_TABLELOG: U32 = 5;
pub const FSEv06_TABLELOG_ABSOLUTE_MAX: U32 = 15;

#[inline(always)]
const fn FSEv06_TABLESTEP(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv06_DState_t {
    pub state: usize,
    pub table: *const c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv06_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv06_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
}

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
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline(always)]
unsafe fn FSEv06_peekSymbol(DStatePtr: *const FSEv06_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

#[inline(always)]
unsafe fn FSEv06_updateState(DStatePtr: *mut FSEv06_DState_t, bitD: *mut BITv06_DStream_t) {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let lowBits = BITv06_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
}

#[inline(always)]
unsafe fn FSEv06_decodeSymbol(DStatePtr: *mut FSEv06_DState_t, bitD: *mut BITv06_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv06_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

#[inline(always)]
unsafe fn FSEv06_decodeSymbolFast(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv06_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* ******************************************************************
 *  Common functions of New Generation Entropy library
 ********************************************************************/

/*-****************************************
 *  FSE Error Management
 ******************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
 *  HUF Error Management
 ****************************************************************/
#[inline(always)]
unsafe fn HUFv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/*-**************************************************************
 *  FSE NCount encoding-decoding
 ****************************************************************/
#[inline(always)]
fn FSEv06_abs(a: i16) -> i16 {
    if a < 0 {
        a.wrapping_neg()
    } else {
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_readNCount(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const BYTE;
    let iend = (istart as usize).wrapping_add(hbSize) as *const BYTE;
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
    nbBits = ((bitStream & 0xF) + FSEv06_MIN_TABLELOG) as c_int; /* extract tableLog */
    if nbBits > FSEv06_TABLELOG_ABSOLUTE_MAX as c_int {
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
                if (ip as usize) < (iend as usize).wrapping_sub(5) {
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
                    count = count.wrapping_sub(max);
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining -= FSEv06_abs(count) as c_int;
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
                bitCount -= (8isize
                    * ((iend as isize) - 4 - (ip as isize))) as c_int;
                ip = (iend as usize).wrapping_sub(4) as *const BYTE;
            }
            bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
        }
    } /* while ((remaining>1) && (charnum<=*maxSVPtr)) */
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

/* ******************************************************************
 *  FSE : Finite State Entropy decoder
 ********************************************************************/

/* DTable_max_t == U32[FSEv06_DTABLE_SIZE_U32(FSEv06_MAX_TABLELOG)] */
const DTABLE_MAX_SIZE: usize = 1 + (1 << 12);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_createDTable(mut tableLog: c_uint) -> *mut FSEv06_DTable {
    if tableLog > FSEv06_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv06_TABLELOG_ABSOLUTE_MAX;
    }
    malloc((1usize + (1usize << tableLog)) * core::mem::size_of::<U32>()) as *mut FSEv06_DTable
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_freeDTable(dt: *mut FSEv06_DTable) {
    free(dt as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_buildDTable(
    dt: *mut FSEv06_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSEv06_decode_t;
    let mut symbolNext: [U16; 256] = [0; 256];

    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);
    let tableSize: U32 = 1u32 << tableLog;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);

    /* Sanity Checks */
    if maxSymbolValue > FSEv06_MAX_SYMBOL_VALUE {
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
            let largeLimit: S16 = (1i32.wrapping_shl(tableLog.wrapping_sub(1))) as S16;
            let mut s: U32 = 0;
            while s < maxSV1 {
                if *normalizedCounter.add(s as usize) == -1 {
                    (*tableDecode.add(highThreshold as usize)).symbol = s as BYTE;
                    highThreshold = highThreshold.wrapping_sub(1);
                    symbolNext[s as usize] = 1;
                } else {
                    if *normalizedCounter.add(s as usize) >= largeLimit {
                        DTableH.fastMode = 0;
                    }
                    symbolNext[s as usize] = *normalizedCounter.add(s as usize) as U16;
                }
                s = s.wrapping_add(1);
            }
        }
        ZSTD_memcpy(
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
    }

    /* Build Decoding table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol: BYTE = (*tableDecode.add(u as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = nextState.wrapping_add(1);
            (*tableDecode.add(u as usize)).nbBits =
                tableLog.wrapping_sub(BITv06_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(u as usize)).newState = ((nextState as U32)
                .wrapping_shl((*tableDecode.add(u as usize)).nbBits as U32))
            .wrapping_sub(tableSize) as U16;
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
    let dPtr = dt.add(1) as *mut c_void;
    let cell = dPtr as *mut FSEv06_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_buildDTable_raw(
    dt: *mut FSEv06_DTable,
    nbBits: c_uint,
) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv06_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv06_decode_t;
    let tableSize: c_uint = 1u32 << nbBits;
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
        (*dinfo.add(s as usize)).newState = 0;
        (*dinfo.add(s as usize)).symbol = s as BYTE;
        (*dinfo.add(s as usize)).nbBits = nbBits as BYTE;
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
    fast: c_uint,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = (ostart as usize).wrapping_add(maxDstSize) as *mut BYTE;
    let olimit = (omax as usize).wrapping_sub(3) as *mut BYTE;

    let mut bitD = BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1 = FSEv06_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2 = FSEv06_DState_t {
        state: 0,
        table: core::ptr::null(),
    };

    /* Init */
    {
        let errorCode = BITv06_initDStream(&mut bitD, cSrc, cSrcSize);
        if ERR_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv06_initDState(&mut state1, &mut bitD, dt);
    FSEv06_initDState(&mut state2, &mut bitD, dt);

    macro_rules! GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSEv06_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSEv06_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    /* 4 symbols per loop */
    while (BITv06_reloadDStream(&mut bitD) == BITv06_DStream_unfinished)
        && ((op as usize) < (olimit as usize))
    {
        *op.add(0) = GETSYMBOL!(&mut state1);

        if (FSEv06_MAX_TABLELOG * 2 + 7) as usize > core::mem::size_of::<usize>() * 8 {
            BITv06_reloadDStream(&mut bitD);
        }

        *op.add(1) = GETSYMBOL!(&mut state2);

        if (FSEv06_MAX_TABLELOG * 4 + 7) as usize > core::mem::size_of::<usize>() * 8 {
            if BITv06_reloadDStream(&mut bitD) > BITv06_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = GETSYMBOL!(&mut state1);

        if (FSEv06_MAX_TABLELOG * 2 + 7) as usize > core::mem::size_of::<usize>() * 8 {
            BITv06_reloadDStream(&mut bitD);
        }

        *op.add(3) = GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    /* tail */
    loop {
        if (op as usize) > (omax as usize).wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = GETSYMBOL!(&mut state1);
        op = op.add(1);

        if BITv06_reloadDStream(&mut bitD) == BITv06_DStream_overflow {
            *op = GETSYMBOL!(&mut state2);
            op = op.add(1);
            break;
        }

        if (op as usize) > (omax as usize).wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = GETSYMBOL!(&mut state2);
        op = op.add(1);

        if BITv06_reloadDStream(&mut bitD) == BITv06_DStream_overflow {
            *op = GETSYMBOL!(&mut state1);
            op = op.add(1);
            break;
        }
    }

    (op as usize) - (ostart as usize)
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
    mut cSrcSize: usize,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [i16; 256] = [0; 256];
    let mut dt: [U32; DTABLE_MAX_SIZE] = [0; DTABLE_MAX_SIZE];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv06_MAX_SYMBOL_VALUE;

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
        ip = ip.add(NCountLength);
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
 *  HUF (v06) static definitions
 ********************************************************************/

pub const HUFv06_CTABLEBOUND: usize = 129;

pub const HUFv06_ABSOLUTEMAX_TABLELOG: U32 = 16;
pub const HUFv06_MAX_TABLELOG: U32 = 12;
pub const HUFv06_DEFAULT_TABLELOG: U32 = HUFv06_MAX_TABLELOG;
pub const HUFv06_MAX_SYMBOL_VALUE: U32 = 255;

/* HUFv06_DTABLE_SIZE(maxTableLog) == 1 + (1<<maxTableLog) */
const HUFv06_DTABLE_SIZE_MAX_TABLELOG: usize = 1 + (1 << 12);

static HUFv06_readStats_l: [U32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];

unsafe fn HUFv06_readStats(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightTotal: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: usize;
    let oSize: usize;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            oSize = HUFv06_readStats_l[iSize - 242] as usize;
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
            {
                let mut n: U32 = 0;
                while (n as usize) < oSize {
                    *huffWeight.add(n as usize) = *ip.add((n / 2) as usize) >> 4;
                    *huffWeight.add(n as usize + 1) = *ip.add((n / 2) as usize) & 15;
                    n += 2;
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
            ip.add(1) as *const c_void,
            iSize,
        );
        if ERR_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    ZSTD_memset(
        rankStats as *mut c_void,
        0,
        (HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    {
        let mut n: U32 = 0;
        while (n as usize) < oSize {
            if *huffWeight.add(n as usize) as U32 >= HUFv06_ABSOLUTEMAX_TABLELOG {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *rankStats.add(*huffWeight.add(n as usize) as usize) =
                (*rankStats.add(*huffWeight.add(n as usize) as usize)).wrapping_add(1);
            weightTotal =
                weightTotal.wrapping_add((1u32.wrapping_shl(*huffWeight.add(n as usize) as U32)) >> 1);
            n += 1;
        }
    }
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
                return ERROR(ZSTD_error_corruption_detected);
            }
            *huffWeight.add(oSize) = lastWeight as BYTE;
            *rankStats.add(lastWeight as usize) =
                (*rankStats.add(lastWeight as usize)).wrapping_add(1);
        }
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || ((*rankStats.add(1) & 1) != 0) {
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
pub struct HUFv06_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUFv06_DEltX4 {
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

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_readDTableX2(
    DTable: *mut U16,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut huffWeight: [BYTE; 256] = [0; 256];
    let mut rankVal: [U32; 17] = [0; 17]; /* large enough for values from 0 to 16 */
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv06_DEltX2;

    iSize = HUFv06_readStats(
        huffWeight.as_mut_ptr(),
        HUFv06_MAX_SYMBOL_VALUE as usize + 1,
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
    if tableLog > *DTable.add(0) as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable is too small */
    }
    *DTable.add(0) = tableLog as U16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n < tableLog + 1 {
        let current: U32 = nextRankStart;
        nextRankStart = nextRankStart.wrapping_add(rankVal[n as usize].wrapping_shl(n - 1));
        rankVal[n as usize] = current;
        n += 1;
    }

    /* fill DTable */
    n = 0;
    while n < nbSymbols {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32.wrapping_shl(w)) >> 1;
        let mut i: U32;
        let mut D = HUFv06_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = (tableLog.wrapping_add(1).wrapping_sub(w)) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize].wrapping_add(length) {
            *dt.add(i as usize) = D;
            i = i.wrapping_add(1);
        }
        rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
        n += 1;
    }

    iSize
}

unsafe fn HUFv06_decodeSymbolX2(
    Dstream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BITv06_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.add(val)).byte;
    BITv06_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

#[inline(always)]
unsafe fn HUFv06_DECODE_SYMBOLX2_0(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) {
    **p = HUFv06_decodeSymbolX2(DStreamPtr, dt, dtLog);
    *p = (*p).add(1);
}

#[inline(always)]
unsafe fn HUFv06_DECODE_SYMBOLX2_1(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) {
    if (MEM_64bits() != 0) || (HUFv06_MAX_TABLELOG <= 12) {
        HUFv06_DECODE_SYMBOLX2_0(p, DStreamPtr, dt, dtLog);
    }
}

#[inline(always)]
unsafe fn HUFv06_DECODE_SYMBOLX2_2(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) {
    if MEM_64bits() != 0 {
        HUFv06_DECODE_SYMBOLX2_0(p, DStreamPtr, dt, dtLog);
    }
}

unsafe fn HUFv06_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 4 symbols at a time */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && ((p as usize) <= (pEnd as usize).wrapping_sub(4))
    {
        HUFv06_DECODE_SYMBOLX2_2(&mut p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX2_1(&mut p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX2_2(&mut p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && ((p as usize) < (pEnd as usize))
    {
        HUFv06_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while (p as usize) < (pEnd as usize) {
        HUFv06_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
    }

    (pEnd as usize) - (pStart as usize)
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
    let oend = (op as usize).wrapping_add(dstSize) as *mut BYTE;
    let dtLog: U32 = *DTable.add(0) as U32;
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv06_DEltX2).add(1);
    let mut bitD = BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };

    {
        let errorCode = BITv06_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv06_decodeStreamX2(op, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv06_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U16; HUFv06_DTABLE_SIZE_MAX_TABLELOG] = [0; HUFv06_DTABLE_SIZE_MAX_TABLELOG];
    DTable[0] = HUFv06_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;

    let errorCode = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(errorCode);
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
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = (ostart as usize).wrapping_add(dstSize) as *mut BYTE;
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv06_DEltX2).add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BITv06_DStream_t {
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
        let istart2 = (istart1 as usize).wrapping_add(length1) as *const BYTE;
        let istart3 = (istart2 as usize).wrapping_add(length2) as *const BYTE;
        let istart4 = (istart3 as usize).wrapping_add(length3) as *const BYTE;
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = (ostart as usize).wrapping_add(segmentSize) as *mut BYTE;
        let opStart3 = (opStart2 as usize).wrapping_add(segmentSize) as *mut BYTE;
        let opStart4 = (opStart3 as usize).wrapping_add(segmentSize) as *mut BYTE;
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
        errorCode = BITv06_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv06_reloadDStream(&mut bitD1)
            | BITv06_reloadDStream(&mut bitD2)
            | BITv06_reloadDStream(&mut bitD3)
            | BITv06_reloadDStream(&mut bitD4);
        while (endSignal == BITv06_DStream_unfinished)
            && ((op4 as usize) < (oend as usize).wrapping_sub(7))
        {
            HUFv06_DECODE_SYMBOLX2_2(&mut op1, &mut bitD1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_2(&mut op2, &mut bitD2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_2(&mut op3, &mut bitD3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_2(&mut op4, &mut bitD4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_1(&mut op1, &mut bitD1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_1(&mut op2, &mut bitD2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_1(&mut op3, &mut bitD3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_1(&mut op4, &mut bitD4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_2(&mut op1, &mut bitD1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_2(&mut op2, &mut bitD2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_2(&mut op3, &mut bitD3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_2(&mut op4, &mut bitD4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0(&mut op1, &mut bitD1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0(&mut op2, &mut bitD2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0(&mut op3, &mut bitD3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX2_0(&mut op4, &mut bitD4, dt, dtLog);
            endSignal = BITv06_reloadDStream(&mut bitD1)
                | BITv06_reloadDStream(&mut bitD2)
                | BITv06_reloadDStream(&mut bitD3)
                | BITv06_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if (op1 as usize) > (opStart2 as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if (op2 as usize) > (opStart3 as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if (op3 as usize) > (opStart4 as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUFv06_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv06_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv06_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv06_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BITv06_endOfDStream(&bitD1)
            & BITv06_endOfDStream(&bitD2)
            & BITv06_endOfDStream(&bitD3)
            & BITv06_endOfDStream(&bitD4);
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
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U16; HUFv06_DTABLE_SIZE_MAX_TABLELOG] = [0; HUFv06_DTABLE_SIZE_MAX_TABLELOG];
    DTable[0] = HUFv06_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;

    let errorCode = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(errorCode);
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
    minWeight: c_int,
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
    let mut rankVal: [U32; 17] = [0; 17];

    /* get pre-calculated rankVal */
    ZSTD_memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; 17]>(),
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
            *DTable.add(i as usize) = DElt;
            i = i.wrapping_add(1);
        }
    }

    /* fill DTable */
    {
        let mut s: U32 = 0;
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
            }

            rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
            s = s.wrapping_add(1);
        }
    }
}

/* rankVal_t == U32[HUFv06_ABSOLUTEMAX_TABLELOG][HUFv06_ABSOLUTEMAX_TABLELOG + 1] */
pub type rankVal_t = [[U32; 17]; 16];

unsafe fn HUFv06_fillDTableX4(
    DTable: *mut HUFv06_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *mut rankVal_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; 17] = [0; 17];
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int; /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    ZSTD_memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; 17]>(),
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
            let mut minWeight: c_int = nbBits as c_int + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUFv06_fillDTableX4Level2(
                DTable.add(start as usize),
                targetLog.wrapping_sub(nbBits),
                nbBits,
                (*rankValOrigin)[nbBits as usize].as_ptr(),
                minWeight,
                sortedList.add(sortedRank as usize),
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
                    *DTable.add(u as usize) = DElt;
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
    let mut weightList: [BYTE; 256] = [0; 256];
    let mut sortedSymbol: [sortedSymbol_t; 256] = [sortedSymbol_t {
        symbol: 0,
        weight: 0,
    }; 256];
    let mut rankStats: [U32; 17] = [0; 17];
    let mut rankStart0: [U32; 18] = [0; 18];
    let rankStart = rankStart0.as_mut_ptr().add(1);
    let mut rankVal: rankVal_t = [[0u32; 17]; 16];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.add(0);
    let iSize: usize;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUFv06_DEltX4).add(1);

    if memLog > HUFv06_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUFv06_readStats(
        weightList.as_mut_ptr(),
        HUFv06_MAX_SYMBOL_VALUE as usize + 1,
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
    }

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w < maxW + 1 {
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankStats[w as usize]);
            *rankStart.add(w as usize) = current;
            w += 1;
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
        {
            let rescale: c_int = (memLog.wrapping_sub(tableLog).wrapping_sub(1)) as c_int; /* tableLog <= memLog */
            let mut nextRankVal: U32 = 0;
            let mut w: U32;
            w = 1;
            while w < maxW + 1 {
                let current: U32 = nextRankVal;
                nextRankVal = nextRankVal
                    .wrapping_add(rankStats[w as usize].wrapping_shl((w as c_int + rescale) as U32));
                rankVal[0][w as usize] = current;
                w += 1;
            }
        }
        {
            let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
            let mut consumed: U32;
            consumed = minBits;
            while consumed < memLog.wrapping_sub(minBits).wrapping_add(1) {
                let mut w: U32;
                w = 1;
                while w < maxW + 1 {
                    let v = rankVal[0][w as usize] >> consumed;
                    rankVal[consumed as usize][w as usize] = v;
                    w += 1;
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
        &mut rankVal,
        maxW,
        tableLog.wrapping_add(1),
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
    ZSTD_memcpy(op, dt.add(val) as *const c_void, 2);
    BITv06_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

unsafe fn HUFv06_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv06_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    ZSTD_memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BITv06_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (core::mem::size_of::<usize>() * 8) as U32 {
            BITv06_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (core::mem::size_of::<usize>() * 8) as U32 {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as U32;
            }
        }
    }
    1
}

#[inline(always)]
unsafe fn HUFv06_DECODE_SYMBOLX4_0(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) {
    let n = HUFv06_decodeSymbolX4(*p as *mut c_void, DStreamPtr, dt, dtLog);
    *p = (*p).add(n as usize);
}

#[inline(always)]
unsafe fn HUFv06_DECODE_SYMBOLX4_1(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) {
    if (MEM_64bits() != 0) || (HUFv06_MAX_TABLELOG <= 12) {
        HUFv06_DECODE_SYMBOLX4_0(p, DStreamPtr, dt, dtLog);
    }
}

#[inline(always)]
unsafe fn HUFv06_DECODE_SYMBOLX4_2(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) {
    if MEM_64bits() != 0 {
        HUFv06_DECODE_SYMBOLX4_0(p, DStreamPtr, dt, dtLog);
    }
}

unsafe fn HUFv06_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && ((p as usize) < (pEnd as usize).wrapping_sub(7))
    {
        HUFv06_DECODE_SYMBOLX4_2(&mut p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX4_1(&mut p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX4_2(&mut p, bitDPtr, dt, dtLog);
        HUFv06_DECODE_SYMBOLX4_0(&mut p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && ((p as usize) <= (pEnd as usize).wrapping_sub(2))
    {
        HUFv06_DECODE_SYMBOLX4_0(&mut p, bitDPtr, dt, dtLog);
    }

    while (p as usize) <= (pEnd as usize).wrapping_sub(2) {
        HUFv06_DECODE_SYMBOLX4_0(&mut p, bitDPtr, dt, dtLog); /* no need to reload */
    }

    if (p as usize) < (pEnd as usize) {
        p = p.add(HUFv06_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    (p as usize) - (pStart as usize)
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
    let oend = (ostart as usize).wrapping_add(dstSize) as *mut BYTE;

    let dtLog: U32 = *DTable.add(0);
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv06_DEltX4).add(1);

    /* Init */
    let mut bitD = BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    {
        let errorCode = BITv06_initDStream(&mut bitD, istart as *const c_void, cSrcSize);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    /* decode */
    HUFv06_decodeStreamX4(ostart, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv06_endOfDStream(&bitD) == 0 {
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
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U32; HUFv06_DTABLE_SIZE_MAX_TABLELOG] = [0; HUFv06_DTABLE_SIZE_MAX_TABLELOG];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
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
        return ERROR(ZSTD_error_corruption_detected); /* strict minimum : jump table + 1 byte per stream */
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = (ostart as usize).wrapping_add(dstSize) as *mut BYTE;
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv06_DEltX4).add(1);
        let dtLog: U32 = *DTable.add(0);
        let mut errorCode: usize;

        /* Init */
        let mut bitD1 = BITv06_DStream_t {
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
        let istart2 = (istart1 as usize).wrapping_add(length1) as *const BYTE;
        let istart3 = (istart2 as usize).wrapping_add(length2) as *const BYTE;
        let istart4 = (istart3 as usize).wrapping_add(length3) as *const BYTE;
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = (ostart as usize).wrapping_add(segmentSize) as *mut BYTE;
        let opStart3 = (opStart2 as usize).wrapping_add(segmentSize) as *mut BYTE;
        let opStart4 = (opStart3 as usize).wrapping_add(segmentSize) as *mut BYTE;
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
        errorCode = BITv06_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv06_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv06_reloadDStream(&mut bitD1)
            | BITv06_reloadDStream(&mut bitD2)
            | BITv06_reloadDStream(&mut bitD3)
            | BITv06_reloadDStream(&mut bitD4);
        while (endSignal == BITv06_DStream_unfinished)
            && ((op4 as usize) < (oend as usize).wrapping_sub(7))
        {
            HUFv06_DECODE_SYMBOLX4_2(&mut op1, &mut bitD1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_2(&mut op2, &mut bitD2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_2(&mut op3, &mut bitD3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_2(&mut op4, &mut bitD4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_1(&mut op1, &mut bitD1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_1(&mut op2, &mut bitD2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_1(&mut op3, &mut bitD3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_1(&mut op4, &mut bitD4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_2(&mut op1, &mut bitD1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_2(&mut op2, &mut bitD2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_2(&mut op3, &mut bitD3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_2(&mut op4, &mut bitD4, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0(&mut op1, &mut bitD1, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0(&mut op2, &mut bitD2, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0(&mut op3, &mut bitD3, dt, dtLog);
            HUFv06_DECODE_SYMBOLX4_0(&mut op4, &mut bitD4, dt, dtLog);

            endSignal = BITv06_reloadDStream(&mut bitD1)
                | BITv06_reloadDStream(&mut bitD2)
                | BITv06_reloadDStream(&mut bitD3)
                | BITv06_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if (op1 as usize) > (opStart2 as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if (op2 as usize) > (opStart3 as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if (op3 as usize) > (opStart4 as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUFv06_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv06_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv06_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv06_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BITv06_endOfDStream(&bitD1)
            & BITv06_endOfDStream(&bitD2)
            & BITv06_endOfDStream(&bitD3)
            & BITv06_endOfDStream(&bitD4);
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
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [U32; HUFv06_DTABLE_SIZE_MAX_TABLELOG] = [0; HUFv06_DTABLE_SIZE_MAX_TABLELOG];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
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

macro_rules! at {
    ($a:expr, $b:expr) => {
        algo_time_t {
            tableTime: $a,
            decode256Time: $b,
        }
    };
}

static algoTime: [[algo_time_t; 3]; 16] = [
    /* single, double, quad */
    [at!(0, 0), at!(1, 1), at!(2, 2)],            /* Q==0 : impossible */
    [at!(0, 0), at!(1, 1), at!(2, 2)],            /* Q==1 : impossible */
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
];

type decompressionAlgo =
    unsafe extern "C" fn(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    static decompress: [Option<decompressionAlgo>; 3] = [
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
        ZSTD_memcpy(dst, cSrc, dstSize);
        return dstSize;
    } /* not compressed */
    if cSrcSize == 1 {
        ZSTD_memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    } /* RLE */

    /* decoder timing evaluation */
    {
        let Q: U32 = (cSrcSize * 16 / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
        let D256: U32 = (dstSize >> 8) as U32;
        let mut n: U32 = 0;
        while n < 3 {
            Dtime[n as usize] = algoTime[Q as usize][n as usize]
                .tableTime
                .wrapping_add(algoTime[Q as usize][n as usize].decode256Time.wrapping_mul(D256));
            n += 1;
        }
    }

    Dtime[1] = Dtime[1].wrapping_add(Dtime[1] >> 4);
    Dtime[2] = Dtime[2].wrapping_add(Dtime[2] >> 3);

    {
        let mut algoNb: U32 = 0;
        if Dtime[1] < Dtime[0] {
            algoNb = 1;
        }
        return match decompress[algoNb as usize] {
            Some(f) => f(dst, dstSize, cSrc, cSrcSize),
            None => 0,
        };
    }
}

/*-****************************************
 *  ZSTD Error Management
 ******************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
 *  ZBUFF Error Management
 ****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

/* ***************************************************************
 *  zstd decompress (v06)
 *****************************************************************/

pub const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526;
pub const ZSTDv06_FRAMEHEADERSIZE_MAX: usize = 13;
pub const ZSTDv06_frameHeaderSize_min: usize = 5;
pub const ZSTDv06_frameHeaderSize_max: usize = ZSTDv06_FRAMEHEADERSIZE_MAX;
pub const ZSTDv06_BLOCKSIZE_MAX: usize = 128 * 1024;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTDv06_frameParams {
    pub frameContentSize: u64,
    pub windowLog: c_uint,
}

/*_*******************************************************
 *  Memory operations
 **********************************************************/
#[inline(always)]
unsafe fn ZSTDv06_copy4(dst: *mut c_void, src: *const c_void) {
    let v = (src as *const u32).read_unaligned();
    (dst as *mut u32).write_unaligned(v);
}

/*-*************************************************************
 *   Context management
 ***************************************************************/
pub type ZSTDv06_dStage = c_uint;
pub const ZSTDds_getFrameHeaderSize: ZSTDv06_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTDv06_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTDv06_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTDv06_dStage = 3;

#[repr(C)]
pub struct ZSTDv06_DCtx {
    pub LLTable: [FSEv06_DTable; 1 + (1 << 9)],
    pub OffTable: [FSEv06_DTable; 1 + (1 << 8)],
    pub MLTable: [FSEv06_DTable; 1 + (1 << 9)],
    pub hufTableX4: [c_uint; 1 + (1 << 12)],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: usize,
    pub headerSize: usize,
    pub fParams: ZSTDv06_frameParams,
    pub bType: blockType_t,
    pub stage: ZSTDv06_dStage,
    pub flagRepeatTable: U32,
    pub litPtr: *const BYTE,
    pub litSize: usize,
    pub litBuffer: [BYTE; ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
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
    ZSTD_memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv06_DCtx>()
            - (ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH + ZSTDv06_frameHeaderSize_max),
    );
}

/*-*************************************************************
 *   Decompression section
 ***************************************************************/

/** ZSTDv06_frameHeaderSize() */
unsafe fn ZSTDv06_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    if srcSize < ZSTDv06_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let fcsId: U32 = (*(src as *const BYTE).add(4)) as U32 >> 6;
        ZSTDv06_frameHeaderSize_min + ZSTDv06_fcs_fieldSize[fcsId as usize]
    }
}

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

    ZSTD_memset(
        fparamsPtr as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv06_frameParams>(),
    );
    {
        let frameDesc: BYTE = *ip.add(4);
        (*fparamsPtr).windowLog = (frameDesc as U32 & 0xF) + ZSTDv06_WINDOWLOG_ABSOLUTEMIN;
        if (frameDesc & 0x20) != 0 {
            return ERROR(ZSTD_error_frameParameter_unsupported); /* reserved 1 bit */
        }
        match frameDesc >> 6 {
            /* fcsId */
            1 => (*fparamsPtr).frameContentSize = *ip.add(5) as u64,
            2 => {
                (*fparamsPtr).frameContentSize =
                    MEM_readLE16(ip.add(5) as *const c_void) as u64 + 256
            }
            3 => {
                (*fparamsPtr).frameContentSize = MEM_readLE64(ip.add(5) as *const c_void)
            }
            _ => (*fparamsPtr).frameContentSize = 0,
        }
    }
    0
}

/** ZSTDv06_decodeFrameHeader() */
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
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

/* ZSTDv06_getcBlockSize() */
unsafe fn ZSTDv06_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_: *const BYTE = src as *const BYTE;
    let cSize: U32;

    if srcSize < ZSTDv06_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (*bpPtr).blockType = ((*in_) as U32 >> 6) as blockType_t;
    cSize = (*in_.add(2) as U32)
        .wrapping_add((*in_.add(1) as U32) << 8)
        .wrapping_add(((*in_.add(0) as U32) & 7) << 16);
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
    ZSTD_memcpy(dst, src, srcSize);
    srcSize
}

/* ZSTDv06_decodeLiteralsBlock() */
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

    match (*istart.add(0)) as U32 >> 6 {
        IS_HUF => {
            let litSize: usize;
            let litCSize: usize;
            let mut singleStream: usize = 0;
            let mut lhSize: U32 = ((*istart.add(0)) as U32 >> 4) & 3;
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            match lhSize {
                2 => {
                    /* 2 - 2 - 14 - 14 */
                    lhSize = 4;
                    litSize = (((*istart.add(0)) as usize & 15) << 10)
                        + ((*istart.add(1) as usize) << 2)
                        + ((*istart.add(2) as usize) >> 6);
                    litCSize = (((*istart.add(2)) as usize & 63) << 8) + *istart.add(3) as usize;
                }
                3 => {
                    /* 2 - 2 - 18 - 18 */
                    lhSize = 5;
                    litSize = (((*istart.add(0)) as usize & 15) << 14)
                        + ((*istart.add(1) as usize) << 6)
                        + ((*istart.add(2) as usize) >> 2);
                    litCSize = (((*istart.add(2)) as usize & 3) << 16)
                        + ((*istart.add(3) as usize) << 8)
                        + *istart.add(4) as usize;
                }
                _ => {
                    /* 2 - 2 - 10 - 10 */
                    lhSize = 3;
                    singleStream = (*istart.add(0) & 16) as usize;
                    litSize = (((*istart.add(0)) as usize & 15) << 6)
                        + ((*istart.add(1) as usize) >> 2);
                    litCSize = (((*istart.add(1)) as usize & 3) << 8) + *istart.add(2) as usize;
                }
            }
            if litSize > ZSTDv06_BLOCKSIZE_MAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            let r = if singleStream != 0 {
                HUFv06_decompress1X2(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            } else {
                HUFv06_decompress(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            };
            if HUFv06_isError(r) != 0 {
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
            let litSize: usize;
            let litCSize: usize;
            let mut lhSize: U32 = ((*istart.add(0)) as U32 >> 4) & 3;
            if lhSize != 1 {
                /* only case supported for now : small litSize, single stream */
                return ERROR(ZSTD_error_corruption_detected);
            }
            if (*dctx).flagRepeatTable == 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }

            /* 2 - 2 - 10 - 10 */
            lhSize = 3;
            litSize = (((*istart.add(0)) as usize & 15) << 6) + ((*istart.add(1) as usize) >> 2);
            litCSize = (((*istart.add(1)) as usize & 3) << 8) + *istart.add(2) as usize;
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            {
                let errorCode = HUFv06_decompress1X4_usingDTable(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.add(lhSize as usize) as *const c_void,
                    litCSize,
                    (*dctx).hufTableX4.as_ptr(),
                );
                if HUFv06_isError(errorCode) != 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
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
            let mut lhSize: U32 = ((*istart.add(0)) as U32 >> 4) & 3;
            match lhSize {
                2 => {
                    litSize = (((*istart.add(0)) as usize & 15) << 8) + *istart.add(1) as usize;
                }
                3 => {
                    litSize = (((*istart.add(0)) as usize & 15) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + *istart.add(2) as usize;
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
            let mut lhSize: U32 = ((*istart.add(0)) as U32 >> 4) & 3;
            match lhSize {
                2 => {
                    litSize = (((*istart.add(0)) as usize & 15) << 8) + *istart.add(1) as usize;
                }
                3 => {
                    litSize = (((*istart.add(0)) as usize & 15) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + *istart.add(2) as usize;
                    if srcSize < 4 {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) & 31) as usize;
                }
            }
            if litSize > ZSTDv06_BLOCKSIZE_MAX {
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

/* ZSTDv06_buildSeqTable() */
unsafe fn ZSTDv06_buildSeqTable(
    DTable: *mut FSEv06_DTable,
    type_: U32,
    mut max: U32,
    maxLog: U32,
    src: *const c_void,
    srcSize: usize,
    defaultNorm: *const S16,
    defaultLog: U32,
    flagRepeatTable: U32,
) -> usize {
    match type_ {
        FSEv06_ENCODING_RLE => {
            if srcSize == 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if (*(src as *const BYTE)) as U32 > max {
                return ERROR(ZSTD_error_corruption_detected);
            }
            FSEv06_buildDTable_rle(DTable, *(src as *const BYTE));
            1
        }
        FSEv06_ENCODING_RAW => {
            FSEv06_buildDTable(DTable, defaultNorm, max, defaultLog);
            0
        }
        FSEv06_ENCODING_STATIC => {
            if flagRepeatTable == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            0
        }
        _ => {
            let mut tableLog: U32 = 0;
            let mut norm: [S16; 53] = [0; 53];
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
}

unsafe fn ZSTDv06_decodeSeqHeaders(
    nbSeqPtr: *mut c_int,
    DTableLL: *mut FSEv06_DTable,
    DTableML: *mut FSEv06_DTable,
    DTableOffb: *mut FSEv06_DTable,
    flagRepeatTable: U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;
    let iend = (istart as usize).wrapping_add(srcSize) as *const BYTE;
    let mut ip = istart;

    /* check */
    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    {
        let mut nbSeq: c_int = *ip as c_int;
        ip = ip.add(1);
        if nbSeq == 0 {
            *nbSeqPtr = 0;
            return 1;
        }
        if nbSeq > 0x7F {
            if nbSeq == 0xFF {
                if (ip as usize).wrapping_add(2) > (iend as usize) {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = (MEM_readLE16(ip as *const c_void) as U32).wrapping_add(LONGNBSEQ) as c_int;
                ip = ip.add(2);
            } else {
                if (ip as usize) >= (iend as usize) {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = ((nbSeq - 0x80) << 8) + *ip as c_int;
                ip = ip.add(1);
            }
        }
        *nbSeqPtr = nbSeq;
    }

    /* FSE table descriptors */
    if (ip as usize).wrapping_add(4) > (iend as usize) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let LLtype: U32 = (*ip) as U32 >> 6;
        let Offtype: U32 = ((*ip) as U32 >> 4) & 3;
        let MLtype: U32 = ((*ip) as U32 >> 2) & 3;
        ip = ip.add(1);

        /* Build DTables */
        {
            let bhSize = ZSTDv06_buildSeqTable(
                DTableLL,
                LLtype,
                MaxLL,
                LLFSELog,
                ip as *const c_void,
                (iend as usize) - (ip as usize),
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                flagRepeatTable,
            );
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(bhSize);
        }
        {
            let bhSize = ZSTDv06_buildSeqTable(
                DTableOffb,
                Offtype,
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                (iend as usize) - (ip as usize),
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                flagRepeatTable,
            );
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(bhSize);
        }
        {
            let bhSize = ZSTDv06_buildSeqTable(
                DTableML,
                MLtype,
                MaxML,
                MLFSELog,
                ip as *const c_void,
                (iend as usize) - (ip as usize),
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                flagRepeatTable,
            );
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(bhSize);
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
    pub DStream: BITv06_DStream_t,
    pub stateLL: FSEv06_DState_t,
    pub stateOffb: FSEv06_DState_t,
    pub stateML: FSEv06_DState_t,
    pub prevOffset: [usize; ZSTDv06_REP_INIT],
}

static LL_base: [U32; 36] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

static ML_base: [U32; 53] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 34, 36, 38, 40, 44, 48, 56, 64, 80, 96, 0x80, 0x100, 0x200, 0x400,
    0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

static OF_base: [U32; 29] = [
    0, 1, 3, 7, 0xF, 0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF, 0x3FFF, 0x7FFF,
    0xFFFF, 0x1FFFF, 0x3FFFF, 0x7FFFF, 0xFFFFF, 0x1FFFFF, 0x3FFFFF, 0x7FFFFF, 0xFFFFFF, 0x1FFFFFF,
    0x3FFFFFF, /*fake*/ 1, 1,
];

unsafe fn ZSTDv06_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    /* Literal length */
    let llCode: U32 = FSEv06_peekSymbol(&(*seqState).stateLL) as U32;
    let mlCode: U32 = FSEv06_peekSymbol(&(*seqState).stateML) as U32;
    let ofCode: U32 = FSEv06_peekSymbol(&(*seqState).stateOffb) as U32;

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
            offset = (OF_base[ofCode as usize] as usize)
                .wrapping_add(BITv06_readBits(&mut (*seqState).DStream, ofBits));
            if MEM_32bits() != 0 {
                BITv06_reloadDStream(&mut (*seqState).DStream);
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
            BITv06_readBits(&mut (*seqState).DStream, mlBits)
        } else {
            0
        });
    if (MEM_32bits() != 0) && (mlBits.wrapping_add(llBits) > 24) {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }

    (*seq).litLength = (LL_base[llCode as usize] as usize).wrapping_add(if llCode > 15 {
        BITv06_readBits(&mut (*seqState).DStream, llBits)
    } else {
        0
    });
    if (MEM_32bits() != 0)
        || (totalBits > 64 - 7 - (LLFSELog + MLFSELog + OffFSELog))
    {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }

    /* ANS state update */
    FSEv06_updateState(&mut (*seqState).stateLL, &mut (*seqState).DStream);
    FSEv06_updateState(&mut (*seqState).stateML, &mut (*seqState).DStream);
    if MEM_32bits() != 0 {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }
    FSEv06_updateState(&mut (*seqState).stateOffb, &mut (*seqState).DStream);
}

static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */

unsafe fn ZSTDv06_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let oLitEnd = (op as usize).wrapping_add(sequence.litLength) as *mut BYTE;
    let sequenceLength = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd = (op as usize).wrapping_add(sequenceLength) as *mut BYTE;
    let oend_8 = (oend as usize).wrapping_sub(8) as *mut BYTE;
    let iLitEnd = (*litPtr as usize).wrapping_add(sequence.litLength) as *const BYTE;
    let mut match_ = (oLitEnd as usize).wrapping_sub(sequence.offset) as *const BYTE;

    /* checks */
    let seqLength = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > ((oend as usize) - (op as usize)) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > ((litLimit as usize).wrapping_sub(*litPtr as usize)) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Now we know there are no overflow in literal nor match lengths, can use pointer checks */
    if (oLitEnd as usize) > (oend_8 as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    if (oMatchEnd as usize) > (oend as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* overwrite beyond dst buffer */
    }
    if (iLitEnd as usize) > (litLimit as usize) {
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
    if sequence.offset > ((oLitEnd as usize).wrapping_sub(base as usize)) {
        /* offset beyond prefix */
        if sequence.offset > ((oLitEnd as usize).wrapping_sub(vBase as usize)) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = (dictEnd as usize)
            .wrapping_sub((base as usize).wrapping_sub(match_ as usize)) as *const BYTE;
        if (match_ as usize).wrapping_add(sequence.matchLength) <= (dictEnd as usize) {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = (dictEnd as usize).wrapping_sub(match_ as usize);
            ZSTD_memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = (oLitEnd as usize).wrapping_add(length1) as *mut BYTE;
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            match_ = base;
            if (op as usize) > (oend_8 as usize) || sequence.matchLength < MINMATCH {
                while (op as usize) < (oMatchEnd as usize) {
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
        let sub2: c_int = dec64table[sequence.offset];
        *op.add(0) = *match_.add(0);
        *op.add(1) = *match_.add(1);
        *op.add(2) = *match_.add(2);
        *op.add(3) = *match_.add(3);
        match_ = match_.add(dec32table[sequence.offset] as usize);
        ZSTDv06_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
        match_ = (match_ as usize).wrapping_sub(sub2 as usize) as *const BYTE;
    } else {
        ZSTDv06_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.add(8);
    match_ = match_.add(8);

    if (oMatchEnd as usize) > (oend as usize).wrapping_sub(16 - MINMATCH) {
        if (op as usize) < (oend_8 as usize) {
            ZSTDv06_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                ((oend_8 as usize) - (op as usize)) as isize,
            );
            match_ = (match_ as usize).wrapping_add((oend_8 as usize) - (op as usize)) as *const BYTE;
            op = oend_8;
        }
        while (op as usize) < (oMatchEnd as usize) {
            *op = *match_;
            op = op.add(1);
            match_ = match_.add(1);
        }
    } else {
        ZSTDv06_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            (sequence.matchLength as isize) - 8,
        );
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
    let iend = (ip as usize).wrapping_add(seqSize) as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = (ostart as usize).wrapping_add(maxDstSize) as *mut BYTE;
    let mut op = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd = (litPtr as usize).wrapping_add((*dctx).litSize) as *const BYTE;
    let DTableLL: *mut FSEv06_DTable = (*dctx).LLTable.as_mut_ptr();
    let DTableML: *mut FSEv06_DTable = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb: *mut FSEv06_DTable = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: c_int = 0;

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
        ip = ip.add(seqHSize);
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
            DStream: BITv06_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            },
            stateLL: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateOffb: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateML: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            prevOffset: [0; ZSTDv06_REP_INIT],
        };

        ZSTD_memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        sequence.offset = REPCODE_STARTVALUE;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTDv06_REP_INIT {
                seqState.prevOffset[i as usize] = REPCODE_STARTVALUE;
                i += 1;
            }
        }
        {
            let errorCode = BITv06_initDStream(
                &mut seqState.DStream,
                ip as *const c_void,
                (iend as usize) - (ip as usize),
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
                op = op.add(oneSeqSize);
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
        if (litPtr as usize) > (litEnd as usize) {
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

unsafe fn ZSTDv06_checkContinuity(dctx: *mut ZSTDv06_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as usize)
            .wrapping_sub(((*dctx).previousDstEnd as usize).wrapping_sub((*dctx).base as usize))
            as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

unsafe fn ZSTDv06_decompressBlock_internal(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip = src as *const BYTE;

    if srcSize >= ZSTDv06_BLOCKSIZE_MAX {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals sub-block */
    {
        let litCSize = ZSTDv06_decodeLiteralsBlock(dctx, src, srcSize);
        if ERR_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.add(litCSize);
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

/* ZSTDv06_decompressFrame() */
unsafe fn ZSTDv06_decompressFrame(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend = (ip as usize).wrapping_add(srcSize) as *const BYTE;
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = (ostart as usize).wrapping_add(dstCapacity) as *mut BYTE;
    let mut remainingSize: usize = srcSize;
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
        ip = ip.add(frameHeaderSize);
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

        ip = ip.add(ZSTDv06_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv06_blockHeaderSize);
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        match blockProperties.blockType {
            bt_compressed => {
                decodedSize = ZSTDv06_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_raw => {
                decodedSize = ZSTDv06_copyRawBlock(
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

        if ERR_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    (op as usize) - (ostart as usize)
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
    let regenSize: usize;
    let dctx = ZSTDv06_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv06_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv06_freeDCtx(dctx);
    regenSize
}

/* ZSTD_errorFrameSizeInfoLegacy() */
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
    let mut remainingSize: usize = srcSize;
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
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
            return;
        }
        if srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }
        ip = ip.add(frameHeaderSize);
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

        ip = ip.add(ZSTDv06_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv06_blockHeaderSize);
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
    loop {
        match stage {
            ZSTDds_getFrameHeaderSize => {
                if srcSize != ZSTDv06_frameHeaderSize_min {
                    return ERROR(ZSTD_error_srcSize_wrong); /* impossible */
                }
                (*dctx).headerSize = ZSTDv06_frameHeaderSize(src, ZSTDv06_frameHeaderSize_min);
                if ERR_isError((*dctx).headerSize) != 0 {
                    return (*dctx).headerSize;
                }
                ZSTD_memcpy(
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
                continue;
            }
            ZSTDds_decodeFrameHeader => {
                let result: usize;
                ZSTD_memcpy(
                    (*dctx)
                        .headerBuffer
                        .as_mut_ptr()
                        .add(ZSTDv06_frameHeaderSize_min) as *mut c_void,
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
            ZSTDds_decodeBlockHeader => {
                let mut bp = blockProperties_t {
                    blockType: bt_compressed,
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
            ZSTDds_decompressBlock => {
                let rSize: usize;
                match (*dctx).bType {
                    bt_compressed => {
                        rSize =
                            ZSTDv06_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
                    }
                    bt_raw => {
                        rSize = ZSTDv06_copyRawBlock(dst, dstCapacity, src, srcSize);
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
                (*dctx).stage = ZSTDds_decodeBlockHeader;
                (*dctx).expected = ZSTDv06_blockHeaderSize;
                if ERR_isError(rSize) != 0 {
                    return rSize;
                }
                (*dctx).previousDstEnd = (dst as *mut c_char).add(rSize) as *const c_void;
                return rSize;
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC); /* impossible */
            }
        }
    }
}

unsafe fn ZSTDv06_refDictContent(
    dctx: *mut ZSTDv06_DCtx,
    dict: *const c_void,
    dictSize: usize,
) {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as usize)
        .wrapping_sub(((*dctx).previousDstEnd as usize).wrapping_sub((*dctx).base as usize))
        as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as usize).wrapping_add(dictSize) as *const c_void;
}

unsafe fn ZSTDv06_loadEntropy(
    dctx: *mut ZSTDv06_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let hSize: usize;
    let offcodeHeaderSize: usize;
    let matchlengthHeaderSize: usize;
    let litlengthHeaderSize: usize;

    hSize = HUFv06_readDTableX4((*dctx).hufTableX4.as_mut_ptr(), dict, dictSize);
    if HUFv06_isError(hSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as usize).wrapping_add(hSize) as *const c_void;
    dictSize -= hSize;

    {
        let mut offcodeNCount: [i16; 29] = [0; 29];
        let mut offcodeMaxValue: c_uint = MaxOff;
        let mut offcodeLog: c_uint = 0;
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
        dict = (dict as usize).wrapping_add(offcodeHeaderSize) as *const c_void;
        dictSize -= offcodeHeaderSize;
    }

    {
        let mut matchlengthNCount: [i16; 53] = [0; 53];
        let mut matchlengthMaxValue: c_uint = MaxML;
        let mut matchlengthLog: c_uint = 0;
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
        dict = (dict as usize).wrapping_add(matchlengthHeaderSize) as *const c_void;
        dictSize -= matchlengthHeaderSize;
    }

    {
        let mut litlengthNCount: [i16; 36] = [0; 36];
        let mut litlengthMaxValue: c_uint = MaxLL;
        let mut litlengthLog: c_uint = 0;
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
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let eSize: usize;
    let magic: U32 = MEM_readLE32(dict);
    if magic != ZSTDv06_DICT_MAGIC {
        /* pure content mode */
        ZSTDv06_refDictContent(dctx, dict, dictSize);
        return 0;
    }
    /* load entropy tables */
    dict = (dict as usize).wrapping_add(4) as *const c_void;
    dictSize -= 4;
    eSize = ZSTDv06_loadEntropy(dctx, dict, dictSize);
    if ERR_isError(eSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    /* reference dictionary content */
    dict = (dict as usize).wrapping_add(eSize) as *const c_void;
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

/* ***************************************************************************
 *  Buffered version of Zstd decompression library
 *****************************************************************************/

pub type ZBUFFv06_dStage = c_uint;
pub const ZBUFFds_init: ZBUFFv06_dStage = 0;
pub const ZBUFFds_loadHeader: ZBUFFv06_dStage = 1;
pub const ZBUFFds_read: ZBUFFv06_dStage = 2;
pub const ZBUFFds_load: ZBUFFv06_dStage = 3;
pub const ZBUFFds_flush: ZBUFFv06_dStage = 4;

/* *** Resource management *** */
#[repr(C)]
pub struct ZBUFFv06_DCtx {
    pub zd: *mut ZSTDv06_DCtx,
    pub fParams: ZSTDv06_frameParams,
    pub stage: ZBUFFv06_dStage,
    pub inBuff: *mut c_char,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub outBuff: *mut c_char,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub blockSize: usize,
    pub headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
    pub lhSize: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_createDCtx() -> *mut ZBUFFv06_DCtx {
    let zbd = malloc(core::mem::size_of::<ZBUFFv06_DCtx>()) as *mut ZBUFFv06_DCtx;
    if zbd.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_memset(
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
        ZSTD_memcpy(dst, src, length);
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
    let iend = (istart as usize).wrapping_add(*srcSizePtr) as *const c_char;
    let mut ip = istart;
    let ostart = dst as *mut c_char;
    let oend = (ostart as usize).wrapping_add(*dstCapacityPtr) as *mut c_char;
    let mut op = ostart;
    let mut notDone: U32 = 1;

    while notDone != 0 {
        let mut st: ZBUFFv06_dStage = (*zbd).stage;
        'sw: loop {
            match st {
                ZBUFFds_init => {
                    return ERROR(ZSTD_error_init_missing);
                }

                ZBUFFds_loadHeader => {
                    {
                        let hSize = ZSTDv06_getFrameParams(
                            &mut (*zbd).fParams,
                            (*zbd).headerBuffer.as_ptr() as *const c_void,
                            (*zbd).lhSize,
                        );
                        if hSize != 0 {
                            let toLoad = hSize.wrapping_sub((*zbd).lhSize); /* if hSize!=0, hSize > zbd->lhSize */
                            if ERR_isError(hSize) != 0 {
                                return hSize;
                            }
                            if toLoad > ((iend as usize).wrapping_sub(ip as usize)) {
                                /* not enough input to load full header */
                                if !ip.is_null() {
                                    ZSTD_memcpy(
                                        (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize)
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
                            ZSTD_memcpy(
                                (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize) as *mut c_void,
                                ip as *const c_void,
                                toLoad,
                            );
                            (*zbd).lhSize = hSize;
                            ip = ip.add(toLoad);
                            break 'sw;
                        }
                    }

                    /* Consume header */
                    {
                        let h1Size = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd); /* == ZSTDv06_frameHeaderSize_min */
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
                                (*zbd).headerBuffer.as_ptr().add(h1Size) as *const c_void,
                                h2Size,
                            );
                            if ERR_isError(h2Result) != 0 {
                                return h2Result;
                            }
                        }
                    }

                    /* Frame header instruct buffer sizes */
                    {
                        let blockSize: usize = {
                            let a = (1i32 << (*zbd).fParams.windowLog) as usize;
                            let b = ZSTDv06_BLOCKSIZE_MAX;
                            if a < b {
                                a
                            } else {
                                b
                            }
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
                            let neededOutSize: usize = (1usize << (*zbd).fParams.windowLog)
                                + blockSize
                                + WILDCOPY_OVERLENGTH * 2;
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
                    st = ZBUFFds_read;
                    continue 'sw;
                }

                ZBUFFds_read => {
                    {
                        let neededInSize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                        if neededInSize == 0 {
                            /* end of frame */
                            (*zbd).stage = ZBUFFds_init;
                            notDone = 0;
                            break 'sw;
                        }
                        if ((iend as usize).wrapping_sub(ip as usize)) >= neededInSize {
                            /* decode directly from src */
                            let decodedSize = ZSTDv06_decompressContinue(
                                (*zbd).zd,
                                (*zbd).outBuff.add((*zbd).outStart) as *mut c_void,
                                (*zbd).outBuffSize - (*zbd).outStart,
                                ip as *const c_void,
                                neededInSize,
                            );
                            if ERR_isError(decodedSize) != 0 {
                                return decodedSize;
                            }
                            ip = ip.add(neededInSize);
                            if decodedSize == 0 {
                                break 'sw; /* this was just a header */
                            }
                            (*zbd).outEnd = (*zbd).outStart + decodedSize;
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
                    continue 'sw;
                }

                ZBUFFds_load => {
                    {
                        let neededInSize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                        let toLoad = neededInSize.wrapping_sub((*zbd).inPos);
                        let loadedSize: usize;
                        if toLoad > (*zbd).inBuffSize.wrapping_sub((*zbd).inPos) {
                            return ERROR(ZSTD_error_corruption_detected); /* should never happen */
                        }
                        loadedSize = ZBUFFv06_limitCopy(
                            (*zbd).inBuff.add((*zbd).inPos) as *mut c_void,
                            toLoad,
                            ip as *const c_void,
                            (iend as usize).wrapping_sub(ip as usize),
                        );
                        ip = ip.add(loadedSize);
                        (*zbd).inPos += loadedSize;
                        if loadedSize < toLoad {
                            notDone = 0;
                            break 'sw;
                        } /* not enough input, wait for more */

                        /* decode loaded input */
                        {
                            let decodedSize = ZSTDv06_decompressContinue(
                                (*zbd).zd,
                                (*zbd).outBuff.add((*zbd).outStart) as *mut c_void,
                                (*zbd).outBuffSize - (*zbd).outStart,
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
                            (*zbd).outEnd = (*zbd).outStart + decodedSize;
                            (*zbd).stage = ZBUFFds_flush;
                            /* ZBUFFds_flush follows */
                        }
                    }
                    /* fall-through */
                    st = ZBUFFds_flush;
                    continue 'sw;
                }

                ZBUFFds_flush => {
                    {
                        let toFlushSize = (*zbd).outEnd.wrapping_sub((*zbd).outStart);
                        let flushedSize = ZBUFFv06_limitCopy(
                            op as *mut c_void,
                            (oend as usize).wrapping_sub(op as usize),
                            (*zbd).outBuff.add((*zbd).outStart) as *const c_void,
                            toFlushSize,
                        );
                        op = op.add(flushedSize);
                        (*zbd).outStart += flushedSize;
                        if flushedSize == toFlushSize {
                            (*zbd).stage = ZBUFFds_read;
                            if (*zbd).outStart + (*zbd).blockSize > (*zbd).outBuffSize {
                                (*zbd).outStart = 0;
                                (*zbd).outEnd = 0;
                            }
                            break 'sw;
                        }
                        /* cannot flush everything */
                        notDone = 0;
                        break 'sw;
                    }
                }
                _ => {
                    return ERROR(ZSTD_error_GENERIC); /* impossible */
                }
            }
        }
    }

    /* result */
    *srcSizePtr = (ip as usize) - (istart as usize);
    *dstCapacityPtr = (op as usize) - (ostart as usize);
    {
        let mut nextSrcSizeHint = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
        if nextSrcSizeHint > ZSTDv06_blockHeaderSize {
            nextSrcSizeHint += ZSTDv06_blockHeaderSize; /* get following block header too */
        }
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
