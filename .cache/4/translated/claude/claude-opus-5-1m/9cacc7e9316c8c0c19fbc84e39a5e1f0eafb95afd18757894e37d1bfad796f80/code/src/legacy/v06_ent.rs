//! Transliteration of the first half of `legacy/zstd_v06.c` : C lines 1..2618.
//!
//! This covers the bundled private copies of `mem.h`, the `zstd_v06` static
//! header constants, `zstd_internal` (ZSTDv06_CCOMMON_H_MODULE), `bitstream.h`
//! (`BITv06_*`), `fse.h`/`fse_static.h`/`fse_decompress.c` (`FSEv06_*`),
//! `entropy_common.c` and `huf.h`/`huf_static.h`/`huf_decompress.c`
//! (`HUFv06_*`).
//!
//! Everything from `/* Common functions of Zstd compression library */`
//! (C line 2619, `ZSTDv06_isError`) onwards lives in the sibling module.
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
    unused_comparisons
)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::error_private::*;
use crate::mem::{calloc, free, malloc, memcpy, memmove, memset, qsort};

/* ******************************************************************
   mem.h : Basic Types
****************************************************************** */
pub type BYTE = u8;
pub type U16 = u16;
pub type U32 = u32;
pub type U64 = u64;
pub type S16 = i16;

/* ******************************************************************
   mem.h : Memory I/O
****************************************************************** */
pub unsafe fn MEM_32bits() -> c_uint {
    (core::mem::size_of::<usize>() == 4) as c_uint
}

pub unsafe fn MEM_64bits() -> c_uint {
    (core::mem::size_of::<usize>() == 8) as c_uint
}

pub unsafe fn MEM_isLittleEndian() -> c_uint {
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
    core::ptr::write_unaligned(memPtr as *mut U16, value);
}

pub unsafe fn MEM_swap32(input: U32) -> U32 {
    input.swap_bytes()
}

pub unsafe fn MEM_swap64(input: U64) -> U64 {
    input.swap_bytes()
}

/*=== Little endian r/w ===*/

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

pub unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        MEM_read32(memPtr)
    } else {
        MEM_swap32(MEM_read32(memPtr))
    }
}

pub unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        MEM_read64(memPtr)
    } else {
        MEM_swap64(MEM_read64(memPtr))
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
   zstd_v06.h "static linking only" section
****************************************************************** */
pub const ZSTDv06_FRAMEHEADERSIZE_MAX: usize = 13; /* for static allocation */
pub static ZSTDv06_frameHeaderSize_min: usize = 5;
pub static ZSTDv06_frameHeaderSize_max: usize = ZSTDv06_FRAMEHEADERSIZE_MAX;

pub const ZSTDv06_BLOCKSIZE_MAX: usize = 128 * 1024; /* define, for static allocation */

/* ******************************************************************
   zstd_internal : Common constants
****************************************************************** */
pub const ZSTDv06_DICT_MAGIC: U32 = 0xEC30A436;

pub const ZSTDv06_REP_NUM: u32 = 3;
pub const ZSTDv06_REP_INIT: u32 = ZSTDv06_REP_NUM;
pub const ZSTDv06_REP_MOVE: u32 = ZSTDv06_REP_NUM - 1;

pub const BIT7: u32 = 128;
pub const BIT6: u32 = 64;
pub const BIT5: u32 = 32;
pub const BIT4: u32 = 16;
pub const BIT1: u32 = 2;
pub const BIT0: u32 = 1;

pub const ZSTDv06_WINDOWLOG_ABSOLUTEMIN: u32 = 12;
pub static ZSTDv06_fcs_fieldSize: [usize; 4] = [0, 1, 2, 8];

pub const ZSTDv06_BLOCKHEADERSIZE: usize = 3;
pub static ZSTDv06_blockHeaderSize: usize = ZSTDv06_BLOCKHEADERSIZE;

pub type blockType_t = c_int;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

pub const MIN_SEQUENCES_SIZE: usize = 1; /* nbSeq==0 */
pub const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

pub const IS_HUF: u32 = 0;
pub const IS_PCH: u32 = 1;
pub const IS_RAW: u32 = 2;
pub const IS_RLE: u32 = 3;

pub const LONGNBSEQ: u32 = 0x7F00;

pub const MINMATCH: u32 = 3;
pub const EQUAL_READ32: u32 = 4;
pub const REPCODE_STARTVALUE: u32 = 1;

pub const Litbits: u32 = 8;
pub const MaxLit: u32 = (1u32 << Litbits) - 1;
pub const MaxML: u32 = 52;
pub const MaxLL: u32 = 35;
pub const MaxOff: u32 = 28;
pub const MaxSeq: u32 = if MaxLL > MaxML { MaxLL } else { MaxML };
pub const MLFSELog: u32 = 9;
pub const LLFSELog: u32 = 9;
pub const OffFSELog: u32 = 8;

pub const FSEv06_ENCODING_RAW: u32 = 0;
pub const FSEv06_ENCODING_RLE: u32 = 1;
pub const FSEv06_ENCODING_STATIC: u32 = 2;
pub const FSEv06_ENCODING_DYNAMIC: u32 = 3;

pub const ZSTD_CONTENTSIZE_ERROR: U64 = 0u64.wrapping_sub(2);

pub static LL_bits: [U32; (MaxLL + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
pub static LL_defaultNorm: [S16; (MaxLL + 1) as usize] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1,
    1, -1, -1, -1, -1,
];
pub static LL_defaultNormLog: U32 = 6;

pub static ML_bits: [U32; (MaxML + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
pub static ML_defaultNorm: [S16; (MaxML + 1) as usize] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
pub static ML_defaultNormLog: U32 = 6;

pub static OF_defaultNorm: [S16; (MaxOff + 1) as usize] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
pub static OF_defaultNormLog: U32 = 5;

/*-*******************************************
*  Shared functions to include for inlining
*********************************************/
pub unsafe fn ZSTDv06_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

/* ! ZSTDv06_wildcopy() :
*   custom version of memcpy(), can copy up to 7 bytes too many (8 bytes if length==0) */
pub const WILDCOPY_OVERLENGTH: usize = 8;

pub unsafe fn ZSTDv06_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_offset(length);
    loop {
        /* COPY8(op, ip) */
        ZSTDv06_copy8(op as *mut c_void, ip as *const c_void);
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
#[derive(Copy, Clone)]
pub struct ZSTDv06_match_t {
    pub off: U32,
    pub len: U32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTDv06_optimal_t {
    pub price: U32,
    pub off: U32,
    pub mlen: U32,
    pub litlen: U32,
    pub rep: [U32; 3], /* ZSTDv06_REP_INIT */
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTDv06_stats_t {
    pub unused: U32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SeqStore_t {
    pub buffer: *mut c_void,
    pub offsetStart: *mut U32,
    pub offset: *mut U32,
    pub offCodeStart: *mut BYTE,
    pub litStart: *mut BYTE,
    pub lit: *mut BYTE,
    pub litLengthStart: *mut U16,
    pub litLength: *mut U16,
    pub llCodeStart: *mut BYTE,
    pub matchLengthStart: *mut U16,
    pub matchLength: *mut U16,
    pub mlCodeStart: *mut BYTE,
    pub longLengthID: U32, /* 0 == no longLength; 1 == Lit.longLength; 2 == Match.longLength; */
    pub longLengthPos: U32,
    /* opt */
    pub priceTable: *mut ZSTDv06_optimal_t,
    pub matchTable: *mut ZSTDv06_match_t,
    pub matchLengthFreq: *mut U32,
    pub litLengthFreq: *mut U32,
    pub litFreq: *mut U32,
    pub offCodeFreq: *mut U32,
    pub matchLengthSum: U32,
    pub matchSum: U32,
    pub litLengthSum: U32,
    pub litSum: U32,
    pub offCodeSum: U32,
    pub log2matchLengthSum: U32,
    pub log2matchSum: U32,
    pub log2litLengthSum: U32,
    pub log2litSum: U32,
    pub log2offCodeSum: U32,
    pub factor: U32,
    pub cachedPrice: U32,
    pub cachedLitLength: U32,
    pub cachedLiterals: *const BYTE,
    pub stats: ZSTDv06_stats_t,
}

/* ******************************************************************
   bitstream.h : bitStream decoding API (read backward)
****************************************************************** */
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

/*-**************************************************************
*  Internal functions
****************************************************************/
pub unsafe fn BITv06_highbit32(val: U32) -> c_uint {
    /* __builtin_clz (val) ^ 31 */
    (val.leading_zeros() ^ 31) as c_uint
}

/*-********************************************************
* bitStream decoding
**********************************************************/
/* ! BITv06_initDStream() :
*   Initialize a BITv06_DStream_t.
*   `bitD` : a pointer to an already allocated BITv06_DStream_t structure.
*   `srcSize` must be the *exact* size of the bitStream, in bytes.
*   @return : size of stream (== srcSize) or an errorCode if a problem is detected
*/
pub unsafe fn BITv06_initDStream(
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
            (*bitD).bitsConsumed = 8u32.wrapping_sub(BITv06_highbit32(lastByte as U32));
        }
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        /* switch(srcSize) with fall-through ; srcSize is within [1..7] here */
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*(srcBuffer as *const BYTE).wrapping_add(6) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 16),
            );
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*(srcBuffer as *const BYTE).wrapping_add(5) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 24),
            );
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*(srcBuffer as *const BYTE).wrapping_add(4) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 32),
            );
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).wrapping_add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).wrapping_add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).wrapping_add(1) as usize) << 8);
        }
        {
            let lastByte: BYTE = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1);
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC); /* endMark not present */
            }
            (*bitD).bitsConsumed = 8u32.wrapping_sub(BITv06_highbit32(lastByte as U32));
        }
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(
            ((core::mem::size_of::<usize>() - srcSize) as U32).wrapping_mul(8),
        );
    }

    srcSize
}

pub unsafe fn BITv06_lookBits(bitD: *const BITv06_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask.wrapping_sub(nbBits)) & bitMask)
}

/* ! BITv06_lookBitsFast() :
*   unsafe version; only works if nbBits >= 1 */
pub unsafe fn BITv06_lookBitsFast(bitD: *const BITv06_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask.wrapping_add(1)).wrapping_sub(nbBits)) & bitMask)
}

pub unsafe fn BITv06_skipBits(bitD: *mut BITv06_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

pub unsafe fn BITv06_readBits(bitD: *mut BITv06_DStream_t, nbBits: U32) -> usize {
    let value: usize = BITv06_lookBits(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

/* ! BITv06_readBitsFast() :
*   unsafe version; only works if nbBits >= 1 */
pub unsafe fn BITv06_readBitsFast(bitD: *mut BITv06_DStream_t, nbBits: U32) -> usize {
    let value: usize = BITv06_lookBitsFast(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

pub unsafe fn BITv06_reloadDStream(bitD: *mut BITv06_DStream_t) -> BITv06_DStream_status {
    if (*bitD).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
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
        if ((*bitD).bitsConsumed as usize) < core::mem::size_of::<usize>() * 8 {
            return BITv06_DStream_endOfBuffer;
        }
        return BITv06_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv06_DStream_status = BITv06_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize).wrapping_sub((*bitD).start as usize) as U32; /* ptr > start */
            result = BITv06_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void); /* reminder : srcSize > sizeof(bitD) */
        return result;
    }
}

/* ! BITv06_endOfDStream() :
*   @return Tells if DStream has exactly reached its end (all bits consumed).
*/
pub unsafe fn BITv06_endOfDStream(DStream: *const BITv06_DStream_t) -> c_uint {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed as usize == core::mem::size_of::<usize>() * 8)) as c_uint
}

/* ******************************************************************
   FSEv06 : public types
****************************************************************** */
/* ! Constructor and Destructor of FSEv06_DTable.
    Note that its size depends on 'tableLog' */
pub type FSEv06_DTable = c_uint; /* don't allocate that. It's just a way to be more restrictive than void* */

/* FSE buffer bounds */
pub const FSEv06_NCOUNTBOUND: usize = 512;

pub const fn FSEv06_BLOCKBOUND(size: usize) -> usize {
    size + (size >> 7)
}

pub const fn FSEv06_COMPRESSBOUND(size: usize) -> usize {
    FSEv06_NCOUNTBOUND + FSEv06_BLOCKBOUND(size)
}

pub const fn FSEv06_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

/* *****************************************
*  FSE symbol decompression API
*******************************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv06_DState_t {
    pub state: usize,
    pub table: *const c_void, /* precise table may vary, depending on U16 */
}

/* ======    Decompression    ====== */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv06_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
} /* sizeof U32 */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv06_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
} /* size == U32 */

pub unsafe fn FSEv06_initDState(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
    dt: *const FSEv06_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH: *const FSEv06_DTableHeader = ptr as *const FSEv06_DTableHeader;
    (*DStatePtr).state = BITv06_readBits(bitD, (*DTableH).tableLog as U32);
    BITv06_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

pub unsafe fn FSEv06_peekSymbol(DStatePtr: *const FSEv06_DState_t) -> BYTE {
    let DInfo: FSEv06_decode_t = *((*DStatePtr).table as *const FSEv06_decode_t)
        .wrapping_add((*DStatePtr).state);
    DInfo.symbol
}

pub unsafe fn FSEv06_updateState(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) {
    let DInfo: FSEv06_decode_t = *((*DStatePtr).table as *const FSEv06_decode_t)
        .wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let lowBits: usize = BITv06_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
}

pub unsafe fn FSEv06_decodeSymbol(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) -> BYTE {
    let DInfo: FSEv06_decode_t = *((*DStatePtr).table as *const FSEv06_decode_t)
        .wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = BITv06_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* ! FSEv06_decodeSymbolFast() :
    unsafe, only works if no symbol has a probability > 50% */
pub unsafe fn FSEv06_decodeSymbolFast(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) -> BYTE {
    let DInfo: FSEv06_decode_t = *((*DStatePtr).table as *const FSEv06_decode_t)
        .wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = BITv06_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* **************************************************************
*  FSE Tuning parameters / Constants
****************************************************************/
pub const FSEv06_MAX_MEMORY_USAGE: u32 = 14;
pub const FSEv06_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSEv06_MAX_SYMBOL_VALUE: u32 = 255;

pub const FSEv06_MAX_TABLELOG: u32 = FSEv06_MAX_MEMORY_USAGE - 2;
pub const FSEv06_MAX_TABLESIZE: u32 = 1u32 << FSEv06_MAX_TABLELOG;
pub const FSEv06_MAXTABLESIZE_MASK: u32 = FSEv06_MAX_TABLESIZE - 1;
pub const FSEv06_DEFAULT_TABLELOG: u32 = FSEv06_DEFAULT_MEMORY_USAGE - 2;
pub const FSEv06_MIN_TABLELOG: u32 = 5;

pub const FSEv06_TABLELOG_ABSOLUTE_MAX: u32 = 15;

pub const fn FSEv06_TABLESTEP(tableSize: u32) -> u32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

/*-****************************************
*  FSE Error Management
******************************************/
#[unsafe(no_mangle)]
pub extern "C" fn FSEv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn FSEv06_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
*  HUF Error Management
****************************************************************/
pub unsafe fn HUFv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/*-**************************************************************
*  FSE NCount encoding-decoding
****************************************************************/
pub fn FSEv06_abs(a: S16) -> S16 {
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
    nbBits = ((bitStream & 0xF).wrapping_add(FSEv06_MIN_TABLELOG)) as c_int; /* extract tableLog */
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
                if ip < iend.wrapping_sub(5) {
                    ip = ip.wrapping_add(2);
                    bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr(bitCount as U32);
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
                bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr(bitCount as U32);
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: S16 = ((2 * threshold - 1) - remaining) as S16;
            let mut count: S16;

            if (bitStream & (threshold - 1) as U32) < ((max as c_int) as U32) {
                count = (bitStream & (threshold - 1) as U32) as S16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold - 1) as U32) as S16;
                if count as c_int >= threshold {
                    count = count.wrapping_sub(max);
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining -= FSEv06_abs(count) as c_int;
            *normalizedCounter.wrapping_add(charnum as usize) = count;
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
                bitCount -= (8usize
                    .wrapping_mul((iend as usize).wrapping_sub(4).wrapping_sub(ip as usize)))
                    as c_int;
                ip = iend.wrapping_sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void) >> ((bitCount & 31) as U32);
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

/* **************************************************************
*  FSE decoder : Complex types
****************************************************************/
pub type DTable_max_t = [U32; FSEv06_DTABLE_SIZE_U32(FSEv06_MAX_TABLELOG)];

/* Function templates */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_createDTable(mut tableLog: c_uint) -> *mut FSEv06_DTable {
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
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let tdPtr = dt.wrapping_add(1) as *mut c_void; /* because *dt is unsigned, 32-bits aligned on 32-bits */
    let tableDecode: *mut FSEv06_decode_t = tdPtr as *mut FSEv06_decode_t;
    let mut symbolNext: [U16; (FSEv06_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSEv06_MAX_SYMBOL_VALUE + 1) as usize];

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
        let mut DTableH: FSEv06_DTableHeader = FSEv06_DTableHeader {
            tableLog: 0,
            fastMode: 0,
        };
        DTableH.tableLog = tableLog as U16;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = (1i32.wrapping_shl(tableLog.wrapping_sub(1))) as S16;
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
        let mut s: U32;
        let mut position: U32 = 0;
        s = 0;
        while s < maxSV1 {
            let mut i: c_int = 0;
            while i < *normalizedCounter.wrapping_add(s as usize) as c_int {
                (*tableDecode.wrapping_add(position as usize)).symbol = s as BYTE;
                position = (position.wrapping_add(step)) & tableMask;
                while position > highThreshold {
                    position = (position.wrapping_add(step)) & tableMask;
                    /* lowprob area */
                }
                i += 1;
            }
            s = s.wrapping_add(1);
        }

        if position != 0 {
            return ERROR(ZSTD_error_GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */
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
                .wrapping_shl((*tableDecode.wrapping_add(u as usize)).nbBits as U32))
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
    let DTableH: *mut FSEv06_DTableHeader = ptr as *mut FSEv06_DTableHeader;
    let dPtr = dt.wrapping_add(1) as *mut c_void;
    let cell: *mut FSEv06_decode_t = dPtr as *mut FSEv06_decode_t;

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
    let DTableH: *mut FSEv06_DTableHeader = ptr as *mut FSEv06_DTableHeader;
    let dPtr = dt.wrapping_add(1) as *mut c_void;
    let dinfo: *mut FSEv06_decode_t = dPtr as *mut FSEv06_decode_t;
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
        (*dinfo.wrapping_add(s as usize)).newState = 0;
        (*dinfo.wrapping_add(s as usize)).symbol = s as BYTE;
        (*dinfo.wrapping_add(s as usize)).nbBits = nbBits as BYTE;
        s = s.wrapping_add(1);
    }

    0
}

pub unsafe fn FSEv06_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv06_DTable,
    fast: c_uint,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.wrapping_add(maxDstSize);
    let olimit: *mut BYTE = omax.wrapping_sub(3);

    let mut bitD: BITv06_DStream_t = BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1: FSEv06_DState_t = FSEv06_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2: FSEv06_DState_t = FSEv06_DState_t {
        state: 0,
        table: core::ptr::null(),
    };

    /* Init */
    {
        let errorCode: usize = BITv06_initDStream(&mut bitD, cSrc, cSrcSize); /* replaced last arg by maxCompressed Size */
        if ERR_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv06_initDState(&mut state1, &mut bitD, dt);
    FSEv06_initDState(&mut state2, &mut bitD, dt);

    /* 4 symbols per loop */
    loop {
        if !((BITv06_reloadDStream(&mut bitD) == BITv06_DStream_unfinished) && (op < olimit)) {
            break;
        }
        *op.wrapping_add(0) = if fast != 0 {
            FSEv06_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv06_decodeSymbol(&mut state1, &mut bitD)
        };

        if (FSEv06_MAX_TABLELOG as usize) * 2 + 7 > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            BITv06_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(1) = if fast != 0 {
            FSEv06_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv06_decodeSymbol(&mut state2, &mut bitD)
        };

        if (FSEv06_MAX_TABLELOG as usize) * 4 + 7 > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            if BITv06_reloadDStream(&mut bitD) > BITv06_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.wrapping_add(2) = if fast != 0 {
            FSEv06_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv06_decodeSymbol(&mut state1, &mut bitD)
        };

        if (FSEv06_MAX_TABLELOG as usize) * 2 + 7 > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            BITv06_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(3) = if fast != 0 {
            FSEv06_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv06_decodeSymbol(&mut state2, &mut bitD)
        };

        op = op.wrapping_add(4);
    }

    /* tail */
    /* note : BITv06_reloadDStream(&bitD) >= FSEv06_DStream_partiallyFilled; Ends at exactly BITv06_DStream_completed */
    loop {
        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = if fast != 0 {
            FSEv06_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv06_decodeSymbol(&mut state1, &mut bitD)
        };
        op = op.wrapping_add(1);

        if BITv06_reloadDStream(&mut bitD) == BITv06_DStream_overflow {
            *op = if fast != 0 {
                FSEv06_decodeSymbolFast(&mut state2, &mut bitD)
            } else {
                FSEv06_decodeSymbol(&mut state2, &mut bitD)
            };
            op = op.wrapping_add(1);
            break;
        }

        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = if fast != 0 {
            FSEv06_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv06_decodeSymbol(&mut state2, &mut bitD)
        };
        op = op.wrapping_add(1);

        if BITv06_reloadDStream(&mut bitD) == BITv06_DStream_overflow {
            *op = if fast != 0 {
                FSEv06_decodeSymbolFast(&mut state1, &mut bitD)
            } else {
                FSEv06_decodeSymbol(&mut state1, &mut bitD)
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
    let DTableH: *const FSEv06_DTableHeader = ptr as *const FSEv06_DTableHeader;
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
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut counting: [i16; (FSEv06_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSEv06_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: DTable_max_t = [0; FSEv06_DTABLE_SIZE_U32(FSEv06_MAX_TABLELOG)]; /* Static analyzer seems unable to understand this table will be properly initialized later */
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv06_MAX_SYMBOL_VALUE;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
    }

    /* normal FSE decoding mode */
    {
        let NCountLength: usize = FSEv06_readNCount(
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
        cSrcSize = cSrcSize.wrapping_sub(NCountLength);
    }

    {
        let errorCode: usize = FSEv06_buildDTable(
            dt.as_mut_ptr() as *mut FSEv06_DTable,
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
        dt.as_ptr() as *const FSEv06_DTable,
    ) /* always return, even if it is an error code */
}

/* ******************************************************************
   HUFv06 : static allocation / constants
****************************************************************** */
/* HUF buffer bounds */
pub const HUFv06_CTABLEBOUND: usize = 129;

pub const fn HUFv06_BLOCKBOUND(size: usize) -> usize {
    size + (size >> 8) + 8
}

pub const fn HUFv06_COMPRESSBOUND(size: usize) -> usize {
    HUFv06_CTABLEBOUND + HUFv06_BLOCKBOUND(size)
}

/* static allocation of HUF's DTable */
pub const fn HUFv06_DTABLE_SIZE(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

pub const HUFv06_ABSOLUTEMAX_TABLELOG: u32 = 16; /* absolute limit of HUFv06_MAX_TABLELOG. Beyond that value, code does not work */
pub const HUFv06_MAX_TABLELOG: u32 = 12; /* max configured tableLog (for static allocation) */
pub const HUFv06_DEFAULT_TABLELOG: u32 = HUFv06_MAX_TABLELOG;
pub const HUFv06_MAX_SYMBOL_VALUE: u32 = 255;

/* local `static U32 l[14]` of HUFv06_readStats() */
pub static HUFv06_readStats_l: [U32; 14] =
    [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];

/* ! HUFv06_readStats() :
    Read compact Huffman tree, saved by HUFv06_writeCTable().
    `huffWeight` is destination buffer.
    @return : size read from `src`
*/
pub unsafe fn HUFv06_readStats(
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
    /* memset(huffWeight, 0, hwSize); */ /* is not necessary, even though some analyzer complain ... */

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            oSize = HUFv06_readStats_l[iSize - 242] as usize;
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
        oSize = FSEv06_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.wrapping_add(1) as *const c_void,
            iSize,
        ); /* max (hwSize-1) values decoded, as last one is implied */
        if FSEv06_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankStats as *mut c_void,
        0,
        (HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    {
        let mut n: U32 = 0;
        while (n as usize) < oSize {
            if (*huffWeight.wrapping_add(n as usize) as U32) >= HUFv06_ABSOLUTEMAX_TABLELOG {
                return ERROR(ZSTD_error_corruption_detected);
            }
            let w = *huffWeight.wrapping_add(n as usize) as usize;
            *rankStats.wrapping_add(w) = (*rankStats.wrapping_add(w)).wrapping_add(1);
            weightTotal = weightTotal
                .wrapping_add((1u32.wrapping_shl(*huffWeight.wrapping_add(n as usize) as U32)) >> 1);
            n = n.wrapping_add(1);
        }
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    {
        let tableLog: U32 = BITv06_highbit32(weightTotal).wrapping_add(1);
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
                return ERROR(ZSTD_error_corruption_detected); /* last value must be a clean power of 2 */
            }
            *huffWeight.wrapping_add(oSize) = lastWeight as BYTE;
            *rankStats.wrapping_add(lastWeight as usize) =
                (*rankStats.wrapping_add(lastWeight as usize)).wrapping_add(1);
        }
    }

    /* check tree construction validity */
    if (*rankStats.wrapping_add(1) < 2) || (*rankStats.wrapping_add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */
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
} /* single-symbol decoding */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUFv06_DEltX4 {
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

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_readDTableX2(
    DTable: *mut U16,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut huffWeight: [BYTE; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal: [U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize]; /* large enough for values from 0 to 16 */
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.wrapping_add(1) as *mut c_void;
    let dt: *mut HUFv06_DEltX2 = dtPtr as *mut HUFv06_DEltX2;

    iSize = HUFv06_readStats(
        huffWeight.as_mut_ptr(),
        (HUFv06_MAX_SYMBOL_VALUE + 1) as usize,
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
    if tableLog > *DTable.wrapping_add(0) as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable is too small */
    }
    *DTable.wrapping_add(0) = tableLog as U16; /* maybe should separate sizeof allocated DTable, from used size of DTable, in case of re-use */

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n < tableLog.wrapping_add(1) {
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
        let mut D: HUFv06_DEltX2 = HUFv06_DEltX2 { byte: 0, nbBits: 0 };
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

pub unsafe fn HUFv06_decodeSymbolX2(
    Dstream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val: usize = BITv06_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.wrapping_add(val)).byte;
    BITv06_skipBits(Dstream, (*dt.wrapping_add(val)).nbBits as U32);
    c
}

pub unsafe fn HUFv06_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart: *mut BYTE = p;

    /* up to 4 symbols at a time */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(4))
    {
        /* HUFv06_DECODE_SYMBOLX2_2 */
        if MEM_64bits() != 0 {
            *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }
        /* HUFv06_DECODE_SYMBOLX2_1 */
        if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
            *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }
        /* HUFv06_DECODE_SYMBOLX2_2 */
        if MEM_64bits() != 0 {
            *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }
        /* HUFv06_DECODE_SYMBOLX2_0 */
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.wrapping_add(1);
    }

    /* closer to the end */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished) && (p < pEnd) {
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.wrapping_add(1);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.wrapping_add(1);
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
    let op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_add(dstSize);
    let dtLog: U32 = *DTable.wrapping_add(0) as U32;
    let dtPtr = DTable as *const c_void;
    let dt: *const HUFv06_DEltX2 = (dtPtr as *const HUFv06_DEltX2).wrapping_add(1);
    let mut bitD: BITv06_DStream_t = BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };

    {
        let errorCode: usize = BITv06_initDStream(&mut bitD, cSrc, cSrcSize);
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
    /* HUFv06_CREATE_STATIC_DTABLEX2(DTable, HUFv06_MAX_TABLELOG) */
    let mut DTable: [U16; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG as U16;
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let errorCode: usize = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

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
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt: *const HUFv06_DEltX2 = (dtPtr as *const HUFv06_DEltX2).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0) as U32;
        let mut errorCode: usize;

        /* Init */
        let mut bitD1: BITv06_DStream_t = BITv06_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2: BITv06_DStream_t = bitD1;
        let mut bitD3: BITv06_DStream_t = bitD1;
        let mut bitD4: BITv06_DStream_t = bitD1;
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
        while (endSignal == BITv06_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            if MEM_64bits() != 0 {
                *op1 = HUFv06_decodeSymbolX2(&mut bitD1, dt, dtLog);
                op1 = op1.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op2 = HUFv06_decodeSymbolX2(&mut bitD2, dt, dtLog);
                op2 = op2.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op3 = HUFv06_decodeSymbolX2(&mut bitD3, dt, dtLog);
                op3 = op3.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op4 = HUFv06_decodeSymbolX2(&mut bitD4, dt, dtLog);
                op4 = op4.wrapping_add(1);
            }
            if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
                *op1 = HUFv06_decodeSymbolX2(&mut bitD1, dt, dtLog);
                op1 = op1.wrapping_add(1);
            }
            if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
                *op2 = HUFv06_decodeSymbolX2(&mut bitD2, dt, dtLog);
                op2 = op2.wrapping_add(1);
            }
            if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
                *op3 = HUFv06_decodeSymbolX2(&mut bitD3, dt, dtLog);
                op3 = op3.wrapping_add(1);
            }
            if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
                *op4 = HUFv06_decodeSymbolX2(&mut bitD4, dt, dtLog);
                op4 = op4.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op1 = HUFv06_decodeSymbolX2(&mut bitD1, dt, dtLog);
                op1 = op1.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op2 = HUFv06_decodeSymbolX2(&mut bitD2, dt, dtLog);
                op2 = op2.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op3 = HUFv06_decodeSymbolX2(&mut bitD3, dt, dtLog);
                op3 = op3.wrapping_add(1);
            }
            if MEM_64bits() != 0 {
                *op4 = HUFv06_decodeSymbolX2(&mut bitD4, dt, dtLog);
                op4 = op4.wrapping_add(1);
            }
            *op1 = HUFv06_decodeSymbolX2(&mut bitD1, dt, dtLog);
            op1 = op1.wrapping_add(1);
            *op2 = HUFv06_decodeSymbolX2(&mut bitD2, dt, dtLog);
            op2 = op2.wrapping_add(1);
            *op3 = HUFv06_decodeSymbolX2(&mut bitD3, dt, dtLog);
            op3 = op3.wrapping_add(1);
            *op4 = HUFv06_decodeSymbolX2(&mut bitD4, dt, dtLog);
            op4 = op4.wrapping_add(1);
            endSignal = BITv06_reloadDStream(&mut bitD1)
                | BITv06_reloadDStream(&mut bitD2)
                | BITv06_reloadDStream(&mut bitD3)
                | BITv06_reloadDStream(&mut bitD4);
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
        return dstSize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X2(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    /* HUFv06_CREATE_STATIC_DTABLEX2(DTable, HUFv06_MAX_TABLELOG) */
    let mut DTable: [U16; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG as U16;
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let errorCode: usize = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

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

pub unsafe fn HUFv06_fillDTableX4Level2(
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
    let mut DElt: HUFv06_DEltX4 = HUFv06_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];

    /* get pre-calculated rankVal */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
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
            core::ptr::write_unaligned(DTable.wrapping_add(i as usize), DElt);
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
                core::ptr::write_unaligned(DTable.wrapping_add(i as usize), DElt);
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

/* typedef U32 rankVal_t[HUFv06_ABSOLUTEMAX_TABLELOG][HUFv06_ABSOLUTEMAX_TABLELOG + 1]; */
pub const rankVal_t_ROWS: usize = HUFv06_ABSOLUTEMAX_TABLELOG as usize;
pub const rankVal_t_COLS: usize = HUFv06_ABSOLUTEMAX_TABLELOG as usize + 1;
pub type rankVal_t = [[U32; rankVal_t_COLS]; rankVal_t_ROWS];

pub unsafe fn HUFv06_fillDTableX4(
    DTable: *mut HUFv06_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *mut [U32; rankVal_t_COLS],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int; /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
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
            let mut minWeight: c_int = nbBits.wrapping_add(scaleLog as U32) as c_int;
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
            let mut DElt: HUFv06_DEltX4 = HUFv06_DEltX4 {
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
                    core::ptr::write_unaligned(DTable.wrapping_add(u as usize), DElt);
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
    let mut weightList: [BYTE; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t {
            symbol: 0,
            weight: 0,
        }; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 2) as usize] =
        [0; (HUFv06_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let rankStart: *mut U32 = rankStart0.as_mut_ptr().wrapping_add(1);
    let mut rankVal: rankVal_t = [[0; rankVal_t_COLS]; rankVal_t_ROWS];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let mut sizeOfSort: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.wrapping_add(0);
    let iSize: usize;
    let dtPtr = DTable as *mut c_void;
    let dt: *mut HUFv06_DEltX4 = (dtPtr as *mut HUFv06_DEltX4).wrapping_add(1);

    if memLog > HUFv06_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    /* memset(weightList, 0, sizeof(weightList)); */ /* is not necessary, even though some analyzer complain ... */

    iSize = HUFv06_readStats(
        weightList.as_mut_ptr(),
        (HUFv06_MAX_SYMBOL_VALUE + 1) as usize,
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
    while *rankStats.as_ptr().wrapping_add(maxW as usize) == 0 {
        maxW = maxW.wrapping_sub(1);
    } /* necessarily finds a solution before 0 */

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w < maxW.wrapping_add(1) {
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
            *rankStart.wrapping_add(w as usize) =
                (*rankStart.wrapping_add(w as usize)).wrapping_add(1);
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s = s.wrapping_add(1);
        }
        *rankStart.wrapping_add(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let rankVal0: *mut U32 = rankVal[0].as_mut_ptr();
        {
            let rescale: c_int = (memLog.wrapping_sub(tableLog)).wrapping_sub(1) as c_int; /* tableLog <= memLog */
            let mut nextRankVal: U32 = 0;
            let mut w: U32;
            w = 1;
            while w < maxW.wrapping_add(1) {
                let current: U32 = nextRankVal;
                nextRankVal = nextRankVal.wrapping_add(
                    rankStats[w as usize].wrapping_shl(w.wrapping_add(rescale as U32)),
                );
                *rankVal0.wrapping_add(w as usize) = current;
                w = w.wrapping_add(1);
            }
        }
        {
            let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
            let mut consumed: U32;
            consumed = minBits;
            while consumed < memLog.wrapping_sub(minBits).wrapping_add(1) {
                let rankValPtr: *mut U32 = rankVal[consumed as usize].as_mut_ptr();
                let mut w: U32;
                w = 1;
                while w < maxW.wrapping_add(1) {
                    *rankValPtr.wrapping_add(w as usize) =
                        (*rankVal0.wrapping_add(w as usize)).wrapping_shr(consumed);
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
        rankVal.as_mut_ptr(),
        maxW,
        tableLog.wrapping_add(1),
    );

    iSize
}

pub unsafe fn HUFv06_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BITv06_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 2);
    let DElt: HUFv06_DEltX4 = core::ptr::read_unaligned(dt.wrapping_add(val));
    BITv06_skipBits(DStream, DElt.nbBits as U32);
    DElt.length as U32
}

pub unsafe fn HUFv06_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: usize = BITv06_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 1);
    let DElt: HUFv06_DEltX4 = core::ptr::read_unaligned(dt.wrapping_add(val));
    if DElt.length == 1 {
        BITv06_skipBits(DStream, DElt.nbBits as U32);
    } else {
        if ((*DStream).bitsConsumed as usize) < (core::mem::size_of::<usize>() * 8) {
            BITv06_skipBits(DStream, DElt.nbBits as U32);
            if ((*DStream).bitsConsumed as usize) > (core::mem::size_of::<usize>() * 8) {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as c_uint;
                /* ugly hack; works only because it's the last symbol. Note : can't easily extract nbBits from just this symbol */
            }
        }
    }
    1
}

pub unsafe fn HUFv06_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart: *mut BYTE = p;

    /* up to 8 symbols at a time */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p < pEnd.wrapping_sub(7))
    {
        /* HUFv06_DECODE_SYMBOLX4_2 */
        if MEM_64bits() != 0 {
            p = p.wrapping_add(
                HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
            );
        }
        /* HUFv06_DECODE_SYMBOLX4_1 */
        if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
            p = p.wrapping_add(
                HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
            );
        }
        /* HUFv06_DECODE_SYMBOLX4_2 */
        if MEM_64bits() != 0 {
            p = p.wrapping_add(
                HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
            );
        }
        /* HUFv06_DECODE_SYMBOLX4_0 */
        p = p.wrapping_add(
            HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
        );
    }

    /* closer to the end */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(2))
    {
        p = p.wrapping_add(
            HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
        );
    }

    while p <= pEnd.wrapping_sub(2) {
        p = p.wrapping_add(
            HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
        ); /* no need to reload : reached the end of DStream */
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
    let istart: *const BYTE = cSrc as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);

    let dtLog: U32 = *DTable.wrapping_add(0);
    let dtPtr = DTable as *const c_void;
    let dt: *const HUFv06_DEltX4 = (dtPtr as *const HUFv06_DEltX4).wrapping_add(1);

    /* Init */
    let mut bitD: BITv06_DStream_t = BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    {
        let errorCode: usize = BITv06_initDStream(&mut bitD, istart as *const c_void, cSrcSize);
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
    /* HUFv06_CREATE_STATIC_DTABLEX4(DTable, HUFv06_MAX_TABLELOG) */
    let mut DTable: [U32; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize = cSrcSize.wrapping_sub(hSize);

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
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt: *const HUFv06_DEltX4 = (dtPtr as *const HUFv06_DEltX4).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0);
        let mut errorCode: usize;

        /* Init */
        let mut bitD1: BITv06_DStream_t = BITv06_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2: BITv06_DStream_t = bitD1;
        let mut bitD3: BITv06_DStream_t = bitD1;
        let mut bitD4: BITv06_DStream_t = bitD1;
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
        while (endSignal == BITv06_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            if MEM_64bits() != 0 {
                op1 = op1.wrapping_add(HUFv06_decodeSymbolX4(
                    op1 as *mut c_void,
                    &mut bitD1,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 {
                op2 = op2.wrapping_add(HUFv06_decodeSymbolX4(
                    op2 as *mut c_void,
                    &mut bitD2,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 {
                op3 = op3.wrapping_add(HUFv06_decodeSymbolX4(
                    op3 as *mut c_void,
                    &mut bitD3,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 {
                op4 = op4.wrapping_add(HUFv06_decodeSymbolX4(
                    op4 as *mut c_void,
                    &mut bitD4,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
                op1 = op1.wrapping_add(HUFv06_decodeSymbolX4(
                    op1 as *mut c_void,
                    &mut bitD1,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
                op2 = op2.wrapping_add(HUFv06_decodeSymbolX4(
                    op2 as *mut c_void,
                    &mut bitD2,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
                op3 = op3.wrapping_add(HUFv06_decodeSymbolX4(
                    op3 as *mut c_void,
                    &mut bitD3,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 || HUFv06_MAX_TABLELOG <= 12 {
                op4 = op4.wrapping_add(HUFv06_decodeSymbolX4(
                    op4 as *mut c_void,
                    &mut bitD4,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 {
                op1 = op1.wrapping_add(HUFv06_decodeSymbolX4(
                    op1 as *mut c_void,
                    &mut bitD1,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 {
                op2 = op2.wrapping_add(HUFv06_decodeSymbolX4(
                    op2 as *mut c_void,
                    &mut bitD2,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 {
                op3 = op3.wrapping_add(HUFv06_decodeSymbolX4(
                    op3 as *mut c_void,
                    &mut bitD3,
                    dt,
                    dtLog,
                ) as usize);
            }
            if MEM_64bits() != 0 {
                op4 = op4.wrapping_add(HUFv06_decodeSymbolX4(
                    op4 as *mut c_void,
                    &mut bitD4,
                    dt,
                    dtLog,
                ) as usize);
            }
            op1 = op1.wrapping_add(HUFv06_decodeSymbolX4(
                op1 as *mut c_void,
                &mut bitD1,
                dt,
                dtLog,
            ) as usize);
            op2 = op2.wrapping_add(HUFv06_decodeSymbolX4(
                op2 as *mut c_void,
                &mut bitD2,
                dt,
                dtLog,
            ) as usize);
            op3 = op3.wrapping_add(HUFv06_decodeSymbolX4(
                op3 as *mut c_void,
                &mut bitD3,
                dt,
                dtLog,
            ) as usize);
            op4 = op4.wrapping_add(HUFv06_decodeSymbolX4(
                op4 as *mut c_void,
                &mut bitD4,
                dt,
                dtLog,
            ) as usize);

            endSignal = BITv06_reloadDStream(&mut bitD1)
                | BITv06_reloadDStream(&mut bitD2)
                | BITv06_reloadDStream(&mut bitD3)
                | BITv06_reloadDStream(&mut bitD4);
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
        return dstSize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    /* HUFv06_CREATE_STATIC_DTABLEX4(DTable, HUFv06_MAX_TABLELOG) */
    let mut DTable: [U32; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize = cSrcSize.wrapping_sub(hSize);

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
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}

pub static algoTime: [[algo_time_t; 3]; 16] = [
    /* single, double, quad */
    [
        algo_time_t { tableTime: 0, decode256Time: 0 },
        algo_time_t { tableTime: 1, decode256Time: 1 },
        algo_time_t { tableTime: 2, decode256Time: 2 },
    ], /* Q==0 : impossible */
    [
        algo_time_t { tableTime: 0, decode256Time: 0 },
        algo_time_t { tableTime: 1, decode256Time: 1 },
        algo_time_t { tableTime: 2, decode256Time: 2 },
    ], /* Q==1 : impossible */
    [
        algo_time_t { tableTime: 38, decode256Time: 130 },
        algo_time_t { tableTime: 1313, decode256Time: 74 },
        algo_time_t { tableTime: 2151, decode256Time: 38 },
    ], /* Q == 2 : 12-18% */
    [
        algo_time_t { tableTime: 448, decode256Time: 128 },
        algo_time_t { tableTime: 1353, decode256Time: 74 },
        algo_time_t { tableTime: 2238, decode256Time: 41 },
    ], /* Q == 3 : 18-25% */
    [
        algo_time_t { tableTime: 556, decode256Time: 128 },
        algo_time_t { tableTime: 1353, decode256Time: 74 },
        algo_time_t { tableTime: 2238, decode256Time: 47 },
    ], /* Q == 4 : 25-32% */
    [
        algo_time_t { tableTime: 714, decode256Time: 128 },
        algo_time_t { tableTime: 1418, decode256Time: 74 },
        algo_time_t { tableTime: 2436, decode256Time: 53 },
    ], /* Q == 5 : 32-38% */
    [
        algo_time_t { tableTime: 883, decode256Time: 128 },
        algo_time_t { tableTime: 1437, decode256Time: 74 },
        algo_time_t { tableTime: 2464, decode256Time: 61 },
    ], /* Q == 6 : 38-44% */
    [
        algo_time_t { tableTime: 897, decode256Time: 128 },
        algo_time_t { tableTime: 1515, decode256Time: 75 },
        algo_time_t { tableTime: 2622, decode256Time: 68 },
    ], /* Q == 7 : 44-50% */
    [
        algo_time_t { tableTime: 926, decode256Time: 128 },
        algo_time_t { tableTime: 1613, decode256Time: 75 },
        algo_time_t { tableTime: 2730, decode256Time: 75 },
    ], /* Q == 8 : 50-56% */
    [
        algo_time_t { tableTime: 947, decode256Time: 128 },
        algo_time_t { tableTime: 1729, decode256Time: 77 },
        algo_time_t { tableTime: 3359, decode256Time: 77 },
    ], /* Q == 9 : 56-62% */
    [
        algo_time_t { tableTime: 1107, decode256Time: 128 },
        algo_time_t { tableTime: 2083, decode256Time: 81 },
        algo_time_t { tableTime: 4006, decode256Time: 84 },
    ], /* Q ==10 : 62-69% */
    [
        algo_time_t { tableTime: 1177, decode256Time: 128 },
        algo_time_t { tableTime: 2379, decode256Time: 87 },
        algo_time_t { tableTime: 4785, decode256Time: 88 },
    ], /* Q ==11 : 69-75% */
    [
        algo_time_t { tableTime: 1242, decode256Time: 128 },
        algo_time_t { tableTime: 2415, decode256Time: 93 },
        algo_time_t { tableTime: 5155, decode256Time: 84 },
    ], /* Q ==12 : 75-81% */
    [
        algo_time_t { tableTime: 1349, decode256Time: 128 },
        algo_time_t { tableTime: 2644, decode256Time: 106 },
        algo_time_t { tableTime: 5260, decode256Time: 106 },
    ], /* Q ==13 : 81-87% */
    [
        algo_time_t { tableTime: 1455, decode256Time: 128 },
        algo_time_t { tableTime: 2422, decode256Time: 124 },
        algo_time_t { tableTime: 4174, decode256Time: 124 },
    ], /* Q ==14 : 87-93% */
    [
        algo_time_t { tableTime: 722, decode256Time: 128 },
        algo_time_t { tableTime: 1891, decode256Time: 145 },
        algo_time_t { tableTime: 1936, decode256Time: 146 },
    ], /* Q ==15 : 93-99% */
];

pub type decompressionAlgo =
    Option<unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize>;

/* local `static const decompressionAlgo decompress[3]` of HUFv06_decompress() */
pub static HUFv06_decompress_decompress: [decompressionAlgo; 3] = [
    Some(HUFv06_decompress4X2),
    Some(HUFv06_decompress4X4),
    None,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
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
        memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    } /* RLE */

    /* decoder timing evaluation */
    {
        let Q: U32 = (cSrcSize.wrapping_mul(16) / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
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
    Dtime[2] = Dtime[2].wrapping_add(Dtime[2] >> 3); /* advantage to algorithms using less memory, for cache eviction */

    {
        let mut algoNb: U32 = 0;
        if Dtime[1] < Dtime[0] {
            algoNb = 1;
        }
        /* if (Dtime[2] < Dtime[algoNb]) algoNb = 2; */ /* current speed of HUFv06_decompress4X6 is not good */
        return match HUFv06_decompress_decompress[algoNb as usize] {
            Some(f) => f(dst, dstSize, cSrc, cSrcSize),
            None => 0,
        };
    }

    /* return HUFv06_decompress4X2(dst, dstSize, cSrc, cSrcSize); */ /* multi-streams single-symbol decoding */
    /* return HUFv06_decompress4X4(dst, dstSize, cSrc, cSrcSize); */ /* multi-streams double-symbols decoding */
    /* return HUFv06_decompress4X6(dst, dstSize, cSrc, cSrcSize); */ /* multi-streams quad-symbols decoding */
}
