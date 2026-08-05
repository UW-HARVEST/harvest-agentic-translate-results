//! Translation of legacy/zstd_v07.c (zstd v0.7 decompressor).
//! Self-contained: defines its own FSE/HUF/BIT/ZBUFF/ZSTD-decoder internals.
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
use crate::common::error::{code as ec, err_is_error, error as err_code};
use crate::common::xxhash::{
    XXH64_state_t, ZSTD_XXH64_digest as XXH64_digest, ZSTD_XXH64_reset as XXH64_reset,
    ZSTD_XXH64_update as XXH64_update,
};

/* ****************************************************************
 * Error management (uses shared modern zstd error enum)
 * ************************************************************** */
#[inline]
fn ERROR(code: i32) -> usize {
    err_code(code)
}
#[inline]
fn ERR_isError(code: usize) -> c_uint {
    err_is_error(code)
}
#[inline]
fn ERR_getErrorName(code: usize) -> *const c_char {
    crate::common::error::err_get_error_name(code)
}

/* ****************************************************************
 * Constants
 * ************************************************************** */
const ZSTDv07_MAGICNUMBER: u32 = 0xFD2FB527;
const ZSTDv07_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;

const ZSTDv07_WINDOWLOG_MAX_64: u32 = 27;
const ZSTDv07_WINDOWLOG_MAX: u32 = ZSTDv07_WINDOWLOG_MAX_64;
const ZSTDv07_WINDOWLOG_ABSOLUTEMIN: u32 = 10;

const ZSTDv07_FRAMEHEADERSIZE_MAX: usize = 18;
const ZSTDv07_frameHeaderSize_min: usize = 5;
const ZSTDv07_frameHeaderSize_max: usize = ZSTDv07_FRAMEHEADERSIZE_MAX;
const ZSTDv07_skippableHeaderSize: usize = 8;

const ZSTDv07_BLOCKSIZE_ABSOLUTEMAX: usize = 128 * 1024;
const WILDCOPY_OVERLENGTH: usize = 8;

/* FSE */
const FSEv07_MAX_MEMORY_USAGE: u32 = 14;
const FSEv07_MAX_SYMBOL_VALUE: u32 = 255;
const FSEv07_MAX_TABLELOG: u32 = FSEv07_MAX_MEMORY_USAGE - 2;
const FSEv07_MIN_TABLELOG: u32 = 5;
const FSEv07_TABLELOG_ABSOLUTE_MAX: u32 = 15;
#[inline]
fn FSEv07_TABLESTEP(tableSize: u32) -> u32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}
#[inline]
const fn FSEv07_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

/* HUF */
const HUFv07_TABLELOG_ABSOLUTEMAX: u32 = 16;
const HUFv07_TABLELOG_MAX: u32 = 12;
const HUFv07_SYMBOLVALUE_MAX: u32 = 255;
#[inline]
const fn HUFv07_DTABLE_SIZE(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

/* ZSTD internal */
const ZSTDv07_REP_NUM: usize = 3;
const ZSTDv07_REP_INIT: usize = ZSTDv07_REP_NUM;
static repStartValue: [u32; ZSTDv07_REP_NUM] = [1, 4, 8];

const ZSTDv07_BLOCKHEADERSIZE: usize = 3;
const ZSTDv07_blockHeaderSize: usize = ZSTDv07_BLOCKHEADERSIZE;

const MIN_SEQUENCES_SIZE: usize = 1;
const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

const LONGNBSEQ: i32 = 0x7F00;
const MINMATCH: usize = 3;

const MaxLL: usize = 35;
const MaxML: usize = 52;
const MaxOff: usize = 28;
const MaxSeq: usize = if MaxLL > MaxML { MaxLL } else { MaxML };
const MLFSELog: u32 = 9;
const LLFSELog: u32 = 9;
const OffFSELog: u32 = 8;

const FSEv07_ENCODING_RAW: u32 = 0;
const FSEv07_ENCODING_RLE: u32 = 1;
const FSEv07_ENCODING_STATIC: u32 = 2;
const FSEv07_ENCODING_DYNAMIC: u32 = 3;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);
const ZSTDv07_DICT_MAGIC: u32 = 0xEC30A437;

static ZSTDv07_fcs_fieldSize: [usize; 4] = [0, 2, 4, 8];
static ZSTDv07_did_fieldSize: [usize; 4] = [0, 1, 2, 4];

static LL_bits: [u32; MaxLL + 1] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
static LL_defaultNorm: [i16; MaxLL + 1] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
const LL_defaultNormLog: u32 = 6;

static ML_bits: [u32; MaxML + 1] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
static ML_defaultNorm: [i16; MaxML + 1] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
const ML_defaultNormLog: u32 = 6;

static OF_defaultNorm: [i16; MaxOff + 1] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
const OF_defaultNormLog: u32 = 5;

/* ****************************************************************
 * mem.h : low-level memory access (little-endian 64-bit)
 * ************************************************************** */
#[inline]
fn MEM_32bits() -> c_uint {
    (core::mem::size_of::<usize>() == 4) as c_uint
}
#[inline]
fn MEM_64bits() -> c_uint {
    (core::mem::size_of::<usize>() == 8) as c_uint
}
#[inline]
unsafe fn MEM_read16(p: *const u8) -> u16 {
    core::ptr::read_unaligned(p as *const u16)
}
#[inline]
unsafe fn MEM_read32(p: *const u8) -> u32 {
    core::ptr::read_unaligned(p as *const u32)
}
#[inline]
unsafe fn MEM_read64(p: *const u8) -> u64 {
    core::ptr::read_unaligned(p as *const u64)
}
#[inline]
unsafe fn MEM_readLEST(p: *const u8) -> usize {
    MEM_read64(p) as usize
}
#[inline]
unsafe fn MEM_readLE16(p: *const u8) -> u16 {
    MEM_read16(p)
}
#[inline]
unsafe fn MEM_readLE32(p: *const u8) -> u32 {
    MEM_read32(p)
}
#[inline]
unsafe fn MEM_readLE64(p: *const u8) -> u64 {
    MEM_read64(p)
}
#[inline]
unsafe fn MEM_writeLE16(p: *mut u8, v: u16) {
    core::ptr::write_unaligned(p as *mut u16, v);
}

/* ****************************************************************
 * bitstream (backward reader)
 * ************************************************************** */
#[repr(C)]
struct BITv07_DStream_t {
    bitContainer: usize,
    bitsConsumed: c_uint,
    ptr: *const c_char,
    start: *const c_char,
}

const BITv07_DStream_unfinished: u32 = 0;
const BITv07_DStream_endOfBuffer: u32 = 1;
const BITv07_DStream_completed: u32 = 2;
const BITv07_DStream_overflow: u32 = 3;

#[inline]
fn BITv07_highbit32(val: u32) -> c_uint {
    val.leading_zeros() ^ 31
}

unsafe fn BITv07_initDStream(
    bitD: *mut BITv07_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BITv07_DStream_t>());
        return ERROR(ec::SRCSIZE_WRONG);
    }

    let src = srcBuffer as *const u8;
    if srcSize >= core::mem::size_of::<usize>() {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (src.add(srcSize - core::mem::size_of::<usize>())) as *const c_char;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const u8);
        let lastByte = *src.add(srcSize - 1);
        (*bitD).bitsConsumed = if lastByte != 0 {
            8 - BITv07_highbit32(lastByte as u32)
        } else {
            0
        };
        if lastByte == 0 {
            return ERROR(ec::GENERIC);
        }
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const u8) as usize;
        let bits = core::mem::size_of::<usize>() * 8;
        if srcSize >= 7 {
            (*bitD).bitContainer += (*src.add(6) as usize) << (bits - 16);
        }
        if srcSize >= 6 {
            (*bitD).bitContainer += (*src.add(5) as usize) << (bits - 24);
        }
        if srcSize >= 5 {
            (*bitD).bitContainer += (*src.add(4) as usize) << (bits - 32);
        }
        if srcSize >= 4 {
            (*bitD).bitContainer += (*src.add(3) as usize) << 24;
        }
        if srcSize >= 3 {
            (*bitD).bitContainer += (*src.add(2) as usize) << 16;
        }
        if srcSize >= 2 {
            (*bitD).bitContainer += (*src.add(1) as usize) << 8;
        }
        let lastByte = *src.add(srcSize - 1);
        (*bitD).bitsConsumed = if lastByte != 0 {
            8 - BITv07_highbit32(lastByte as u32)
        } else {
            0
        };
        if lastByte == 0 {
            return ERROR(ec::GENERIC);
        }
        (*bitD).bitsConsumed += ((core::mem::size_of::<usize>() - srcSize) * 8) as c_uint;
    }

    srcSize
}

#[inline]
unsafe fn BITv07_lookBits(bitD: *const BITv07_DStream_t, nbBits: u32) -> usize {
    let bitMask: u32 = (core::mem::size_of::<usize>() * 8 - 1) as u32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1) >> ((bitMask - nbBits) & bitMask)
}

#[inline]
unsafe fn BITv07_lookBitsFast(bitD: *const BITv07_DStream_t, nbBits: u32) -> usize {
    let bitMask: u32 = (core::mem::size_of::<usize>() * 8 - 1) as u32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> (((bitMask + 1) - nbBits) & bitMask)
}

#[inline]
unsafe fn BITv07_skipBits(bitD: *mut BITv07_DStream_t, nbBits: u32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
unsafe fn BITv07_readBits(bitD: *mut BITv07_DStream_t, nbBits: u32) -> usize {
    let value = BITv07_lookBits(bitD, nbBits);
    BITv07_skipBits(bitD, nbBits);
    value
}

#[inline]
unsafe fn BITv07_readBitsFast(bitD: *mut BITv07_DStream_t, nbBits: u32) -> usize {
    let value = BITv07_lookBitsFast(bitD, nbBits);
    BITv07_skipBits(bitD, nbBits);
    value
}

unsafe fn BITv07_reloadDStream(bitD: *mut BITv07_DStream_t) -> u32 {
    if (*bitD).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
        return BITv07_DStream_overflow;
    }

    if (*bitD).ptr as usize >= ((*bitD).start as usize + core::mem::size_of::<usize>()) {
        (*bitD).ptr = ((*bitD).ptr as *const u8).offset(-(((*bitD).bitsConsumed >> 3) as isize))
            as *const c_char;
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const u8);
        return BITv07_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize) < core::mem::size_of::<usize>() * 8 {
            return BITv07_DStream_endOfBuffer;
        }
        return BITv07_DStream_completed;
    }
    {
        let mut nbBytes: u32 = (*bitD).bitsConsumed >> 3;
        let mut result: u32 = BITv07_DStream_unfinished;
        if (((*bitD).ptr as *const u8).offset(-(nbBytes as isize))) < ((*bitD).start as *const u8) {
            nbBytes = ((*bitD).ptr as usize - (*bitD).start as usize) as u32;
            result = BITv07_DStream_endOfBuffer;
        }
        (*bitD).ptr = ((*bitD).ptr as *const u8).offset(-(nbBytes as isize)) as *const c_char;
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const u8);
        result
    }
}

#[inline]
unsafe fn BITv07_endOfDStream(dStream: *const BITv07_DStream_t) -> c_uint {
    (((*dStream).ptr == (*dStream).start)
        && ((*dStream).bitsConsumed as usize == core::mem::size_of::<usize>() * 8)) as c_uint
}

/* ****************************************************************
 * FSE / HUF error management (exported)
 * ************************************************************** */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* ****************************************************************
 * FSE decode types
 * ************************************************************** */
type FSEv07_DTable = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct FSEv07_DTableHeader {
    tableLog: u16,
    fastMode: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSEv07_decode_t {
    newState: u16,
    symbol: u8,
    nbBits: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSEv07_DState_t {
    state: usize,
    table: *const c_void,
}

#[inline]
unsafe fn FSEv07_initDState(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
    dt: *const FSEv07_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv07_DTableHeader;
    (*DStatePtr).state = BITv07_readBits(bitD, (*DTableH).tableLog as u32);
    BITv07_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline]
unsafe fn FSEv07_peekSymbol(DStatePtr: *const FSEv07_DState_t) -> u8 {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

#[inline]
unsafe fn FSEv07_updateState(DStatePtr: *mut FSEv07_DState_t, bitD: *mut BITv07_DStream_t) {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as u32;
    let lowBits = BITv07_readBits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
}

#[inline]
unsafe fn FSEv07_decodeSymbol(DStatePtr: *mut FSEv07_DState_t, bitD: *mut BITv07_DStream_t) -> u8 {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as u32;
    let symbol = DInfo.symbol;
    let lowBits = BITv07_readBits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
unsafe fn FSEv07_decodeSymbolFast(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
) -> u8 {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as u32;
    let symbol = DInfo.symbol;
    let lowBits = BITv07_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

/* ****************************************************************
 * FSE NCount decoding
 * ************************************************************** */
#[inline]
fn FSEv07_abs(a: i16) -> i16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_readNCount(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const u8;
    let iend = istart.add(hbSize);
    let mut ip = istart;
    let mut nbBits: i32;
    let mut remaining: i32;
    let mut threshold: i32;
    let mut bitStream: u32;
    let mut bitCount: i32;
    let mut charnum: c_uint = 0;
    let mut previous0: i32 = 0;

    if hbSize < 4 {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    bitStream = MEM_readLE32(ip);
    nbBits = (bitStream & 0xF) as i32 + FSEv07_MIN_TABLELOG as i32;
    if nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX as i32 {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1i32 << nbBits) + 1;
    threshold = 1i32 << nbBits;
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0 = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 += 24;
                if ip < iend.offset(-5) {
                    ip = ip.add(2);
                    bitStream = MEM_readLE32(ip) >> bitCount;
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
                return ERROR(ec::MAXSYMBOLVALUE_TOOSMALL);
            }
            while charnum < n0 {
                *normalizedCounter.add(charnum as usize) = 0;
                charnum += 1;
            }
            if (ip <= iend.offset(-7)) || (ip.offset((bitCount >> 3) as isize) <= iend.offset(-4)) {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip) >> bitCount;
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: i16 = ((2 * threshold - 1) - remaining) as i16;
            let mut count: i16;

            if ((bitStream & (threshold as u32 - 1)) as u32) < (max as u32) {
                count = (bitStream & (threshold as u32 - 1)) as i16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold as u32 - 1)) as i16;
                if count as i32 >= threshold {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1;
            remaining -= FSEv07_abs(count) as i32;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as i32;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            if (ip <= iend.offset(-7)) || (ip.offset((bitCount >> 3) as isize) <= iend.offset(-4)) {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend as isize - 4 - ip as isize)) as i32;
                ip = iend.offset(-4);
            }
            bitStream = MEM_readLE32(ip) >> (bitCount & 31);
        }
    }
    if remaining != 1 {
        return ERROR(ec::GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    if (ip as usize - istart as usize) > hbSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip as usize - istart as usize
}

/* ****************************************************************
 * HUFv07_readStats
 * ************************************************************** */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readStats(
    huffWeight: *mut u8,
    hwSize: usize,
    rankStats: *mut u32,
    nbSymbolsPtr: *mut u32,
    tableLogPtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightTotal: u32;
    let mut ip = src as *const u8;
    let mut iSize: usize;
    let mut oSize: usize;

    if srcSize == 0 {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        if iSize >= 242 {
            static l: [u32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = l[iSize - 242] as usize;
            memset(huffWeight as *mut c_void, 1, hwSize);
            iSize = 0;
        } else {
            oSize = iSize - 127;
            iSize = (oSize + 1) / 2;
            if iSize + 1 > srcSize {
                return ERROR(ec::SRCSIZE_WRONG);
            }
            if oSize >= hwSize {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            ip = ip.add(1);
            let mut n: u32 = 0;
            while (n as usize) < oSize {
                *huffWeight.add(n as usize) = *ip.add(n as usize / 2) >> 4;
                *huffWeight.add(n as usize + 1) = *ip.add(n as usize / 2) & 15;
                n += 2;
            }
        }
    } else {
        if iSize + 1 > srcSize {
            return ERROR(ec::SRCSIZE_WRONG);
        }
        oSize = FSEv07_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.add(1) as *const c_void,
            iSize,
        );
        if FSEv07_isError(oSize) != 0 {
            return oSize;
        }
    }

    memset(
        rankStats as *mut c_void,
        0,
        (HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1) * core::mem::size_of::<u32>(),
    );
    weightTotal = 0;
    {
        let mut n: u32 = 0;
        while (n as usize) < oSize {
            let w = *huffWeight.add(n as usize);
            if w as u32 >= HUFv07_TABLELOG_ABSOLUTEMAX {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            *rankStats.add(w as usize) += 1;
            weightTotal += (1u32 << w) >> 1;
            n += 1;
        }
    }
    if weightTotal == 0 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    {
        let tableLog = BITv07_highbit32(weightTotal) + 1;
        if tableLog > HUFv07_TABLELOG_ABSOLUTEMAX {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        *tableLogPtr = tableLog;
        {
            let total = 1u32 << tableLog;
            let rest = total - weightTotal;
            let verif = 1u32 << BITv07_highbit32(rest);
            let lastWeight = BITv07_highbit32(rest) + 1;
            if verif != rest {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            *huffWeight.add(oSize) = lastWeight as u8;
            *rankStats.add(lastWeight as usize) += 1;
        }
    }

    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    *nbSymbolsPtr = (oSize + 1) as u32;
    iSize + 1
}

/* ****************************************************************
 * FSE decompression
 * ************************************************************** */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_createDTable(mut tableLog: c_uint) -> *mut FSEv07_DTable {
    if tableLog > FSEv07_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv07_TABLELOG_ABSOLUTE_MAX;
    }
    malloc(FSEv07_DTABLE_SIZE_U32(tableLog) * core::mem::size_of::<u32>()) as *mut FSEv07_DTable
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
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSEv07_decode_t;
    let mut symbolNext = [0u16; FSEv07_MAX_SYMBOL_VALUE as usize + 1];

    let maxSV1 = maxSymbolValue + 1;
    let tableSize: u32 = 1 << tableLog;
    let mut highThreshold = tableSize - 1;

    if maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE {
        return ERROR(ec::MAXSYMBOLVALUE_TOOLARGE);
    }
    if tableLog > FSEv07_MAX_TABLELOG {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }

    {
        let mut DTableH = FSEv07_DTableHeader {
            tableLog: tableLog as u16,
            fastMode: 1,
        };
        {
            let largeLimit: i16 = (1i32 << (tableLog - 1)) as i16;
            let mut s: u32 = 0;
            while s < maxSV1 {
                if *normalizedCounter.add(s as usize) == -1 {
                    (*tableDecode.add(highThreshold as usize)).symbol = s as u8;
                    highThreshold -= 1;
                    symbolNext[s as usize] = 1;
                } else {
                    if *normalizedCounter.add(s as usize) >= largeLimit {
                        DTableH.fastMode = 0;
                    }
                    symbolNext[s as usize] = *normalizedCounter.add(s as usize) as u16;
                }
                s += 1;
            }
        }
        core::ptr::copy_nonoverlapping(
            &DTableH as *const FSEv07_DTableHeader as *const u8,
            dt as *mut u8,
            core::mem::size_of::<FSEv07_DTableHeader>(),
        );
    }

    {
        let tableMask = tableSize - 1;
        let step = FSEv07_TABLESTEP(tableSize);
        let mut position: u32 = 0;
        let mut s: u32 = 0;
        while s < maxSV1 {
            let mut i: i32 = 0;
            while i < *normalizedCounter.add(s as usize) as i32 {
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
    }

    {
        let mut u: u32 = 0;
        while u < tableSize {
            let symbol = (*tableDecode.add(u as usize)).symbol;
            let nextState = symbolNext[symbol as usize];
            symbolNext[symbol as usize] += 1;
            (*tableDecode.add(u as usize)).nbBits =
                (tableLog - BITv07_highbit32(nextState as u32)) as u8;
            (*tableDecode.add(u as usize)).newState =
                (((nextState as u32) << (*tableDecode.add(u as usize)).nbBits) - tableSize) as u16;
            u += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable_rle(dt: *mut FSEv07_DTable, symbolValue: u8) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv07_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let cell = dPtr as *mut FSEv07_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable_raw(dt: *mut FSEv07_DTable, nbBits: c_uint) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv07_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv07_decode_t;
    let tableSize: u32 = 1 << nbBits;
    let tableMask = tableSize - 1;
    let maxSV1 = tableMask + 1;

    if nbBits < 1 {
        return ERROR(ec::GENERIC);
    }

    (*DTableH).tableLog = nbBits as u16;
    (*DTableH).fastMode = 1;
    let mut s: u32 = 0;
    while s < maxSV1 {
        (*dinfo.add(s as usize)).newState = 0;
        (*dinfo.add(s as usize)).symbol = s as u8;
        (*dinfo.add(s as usize)).nbBits = nbBits as u8;
        s += 1;
    }

    0
}

unsafe fn FSEv07_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv07_DTable,
    fast: c_uint,
) -> usize {
    let ostart = dst as *mut u8;
    let mut op = ostart;
    let omax = op.add(maxDstSize);
    let olimit = omax.offset(-3);

    let mut bitD = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
    let bitD = bitD.as_mut_ptr();
    let mut state1 = core::mem::MaybeUninit::<FSEv07_DState_t>::uninit();
    let state1 = state1.as_mut_ptr();
    let mut state2 = core::mem::MaybeUninit::<FSEv07_DState_t>::uninit();
    let state2 = state2.as_mut_ptr();

    {
        let errorCode = BITv07_initDStream(bitD, cSrc, cSrcSize);
        if FSEv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv07_initDState(state1, bitD, dt);
    FSEv07_initDState(state2, bitD, dt);

    macro_rules! GETSYMBOL {
        ($sp:expr) => {
            if fast != 0 {
                FSEv07_decodeSymbolFast($sp, bitD)
            } else {
                FSEv07_decodeSymbol($sp, bitD)
            }
        };
    }

    /* 4 symbols per loop */
    while (BITv07_reloadDStream(bitD) == BITv07_DStream_unfinished) && (op < olimit) {
        *op.add(0) = GETSYMBOL!(state1);

        if FSEv07_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            BITv07_reloadDStream(bitD);
        }

        *op.add(1) = GETSYMBOL!(state2);

        if FSEv07_MAX_TABLELOG * 4 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            if BITv07_reloadDStream(bitD) > BITv07_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = GETSYMBOL!(state1);

        if FSEv07_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            BITv07_reloadDStream(bitD);
        }

        *op.add(3) = GETSYMBOL!(state2);

        op = op.add(4);
    }

    /* tail */
    loop {
        if op > omax.offset(-2) {
            return ERROR(ec::DSTSIZE_TOOSMALL);
        }
        *op = GETSYMBOL!(state1);
        op = op.add(1);

        if BITv07_reloadDStream(bitD) == BITv07_DStream_overflow {
            *op = GETSYMBOL!(state2);
            op = op.add(1);
            break;
        }

        if op > omax.offset(-2) {
            return ERROR(ec::DSTSIZE_TOOSMALL);
        }
        *op = GETSYMBOL!(state2);
        op = op.add(1);

        if BITv07_reloadDStream(bitD) == BITv07_DStream_overflow {
            *op = GETSYMBOL!(state1);
            op = op.add(1);
            break;
        }
    }

    op as usize - ostart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSEv07_DTable,
) -> usize {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv07_DTableHeader;
    let fastMode = (*DTableH).fastMode as u32;

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
    let istart = cSrc as *const u8;
    let mut ip = istart;
    let mut counting = [0i16; FSEv07_MAX_SYMBOL_VALUE as usize + 1];
    let mut dt = [0u32; FSEv07_DTABLE_SIZE_U32(FSEv07_MAX_TABLELOG)];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv07_MAX_SYMBOL_VALUE;

    if cSrcSize < 2 {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    {
        let NCountLength = FSEv07_readNCount(
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
            return ERROR(ec::SRCSIZE_WRONG);
        }
        ip = ip.add(NCountLength);
        cSrcSize -= NCountLength;
    }

    {
        let errorCode = FSEv07_buildDTable(
            dt.as_mut_ptr(),
            counting.as_ptr(),
            maxSymbolValue,
            tableLog,
        );
        if FSEv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv07_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

/* ****************************************************************
 * HUF decompression
 * ************************************************************** */
type HUFv07_DTable = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct DTableDesc {
    maxTableLog: u8,
    tableType: u8,
    tableLog: u8,
    reserved: u8,
}

#[inline]
unsafe fn HUFv07_getDTableDesc(table: *const HUFv07_DTable) -> DTableDesc {
    let mut dtd = DTableDesc {
        maxTableLog: 0,
        tableType: 0,
        tableLog: 0,
        reserved: 0,
    };
    core::ptr::copy_nonoverlapping(
        table as *const u8,
        &mut dtd as *mut DTableDesc as *mut u8,
        core::mem::size_of::<DTableDesc>(),
    );
    dtd
}

/* single-symbol decoding */
#[repr(C)]
#[derive(Clone, Copy)]
struct HUFv07_DEltX2 {
    byte: u8,
    nbBits: u8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX2(
    DTable: *mut HUFv07_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut huffWeight = [0u8; HUFv07_SYMBOLVALUE_MAX as usize + 1];
    let mut rankVal = [0u32; HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1];
    let mut tableLog: u32 = 0;
    let mut nbSymbols: u32 = 0;
    let iSize: usize;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv07_DEltX2;

    iSize = HUFv07_readStats(
        huffWeight.as_mut_ptr(),
        HUFv07_SYMBOLVALUE_MAX as usize + 1,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }

    {
        let mut dtd = HUFv07_getDTableDesc(DTable);
        if tableLog > (dtd.maxTableLog as u32 + 1) {
            return ERROR(ec::TABLELOG_TOOLARGE);
        }
        dtd.tableType = 0;
        dtd.tableLog = tableLog as u8;
        core::ptr::copy_nonoverlapping(
            &dtd as *const DTableDesc as *const u8,
            DTable as *mut u8,
            core::mem::size_of::<DTableDesc>(),
        );
    }

    {
        let mut nextRankStart: u32 = 0;
        let mut n: u32 = 1;
        while n < tableLog + 1 {
            let current = nextRankStart;
            nextRankStart += rankVal[n as usize] << (n - 1);
            rankVal[n as usize] = current;
            n += 1;
        }
    }

    {
        let mut n: u32 = 0;
        while n < nbSymbols {
            let w = huffWeight[n as usize] as u32;
            let length = (1u32 << w) >> 1;
            let mut i: u32;
            let D = HUFv07_DEltX2 {
                byte: n as u8,
                nbBits: (tableLog + 1 - w) as u8,
            };
            i = rankVal[w as usize];
            while i < rankVal[w as usize] + length {
                *dt.add(i as usize) = D;
                i += 1;
            }
            rankVal[w as usize] += length;
            n += 1;
        }
    }

    iSize
}

#[inline]
unsafe fn HUFv07_decodeSymbolX2(
    Dstream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX2,
    dtLog: u32,
) -> u8 {
    let val = BITv07_lookBitsFast(Dstream, dtLog);
    let c = (*dt.add(val)).byte;
    BITv07_skipBits(Dstream, (*dt.add(val)).nbBits as u32);
    c
}

unsafe fn HUFv07_decodeStreamX2(
    mut p: *mut u8,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut u8,
    dt: *const HUFv07_DEltX2,
    dtLog: u32,
) -> usize {
    let pStart = p;

    /* up to 4 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished) && (p <= pEnd.offset(-4)) {
        // X2_2 : MEM_64bits()
        *p = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        // X2_1 : MEM_64bits() || HUFv07_TABLELOG_MAX<=12
        *p = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        // X2_2
        *p = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        // X2_0
        *p = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    /* closer to the end */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished) && (p < pEnd) {
        *p = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        *p = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    pEnd as usize - pStart as usize
}

unsafe fn HUFv07_decompress1X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let op = dst as *mut u8;
    let oend = op.add(dstSize);
    let dtPtr = DTable.add(1) as *const c_void;
    let dt = dtPtr as *const HUFv07_DEltX2;
    let mut bitD = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
    let bitD = bitD.as_mut_ptr();
    let dtd = HUFv07_getDTableDesc(DTable);
    let dtLog = dtd.tableLog as u32;

    {
        let errorCode = BITv07_initDStream(bitD, cSrc, cSrcSize);
        if HUFv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv07_decodeStreamX2(op, bitD, oend, dt, dtLog);

    if BITv07_endOfDStream(bitD) == 0 {
        return ERROR(ec::CORRUPTION_DETECTED);
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
    let dtd = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        return ERROR(ec::GENERIC);
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
    let mut ip = cSrc as *const u8;

    let hSize = HUFv07_readDTableX2(DCtx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
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
    let mut DTable = [0u32; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)];
    DTable[0] = (HUFv07_TABLELOG_MAX - 1) * 0x1000001;
    HUFv07_decompress1X2_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

unsafe fn HUFv07_decompress4X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    {
        let istart = cSrc as *const u8;
        let ostart = dst as *mut u8;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX2;

        let mut bitD1 = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
        let bitD1 = bitD1.as_mut_ptr();
        let mut bitD2 = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
        let bitD2 = bitD2.as_mut_ptr();
        let mut bitD3 = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
        let bitD3 = bitD3.as_mut_ptr();
        let mut bitD4 = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
        let bitD4 = bitD4.as_mut_ptr();
        let length1 = MEM_readLE16(istart) as usize;
        let length2 = MEM_readLE16(istart.add(2)) as usize;
        let length3 = MEM_readLE16(istart.add(4)) as usize;
        let length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
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
        let dtd = HUFv07_getDTableDesc(DTable);
        let dtLog = dtd.tableLog as u32;

        if length4 > cSrcSize {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        {
            let errorCode = BITv07_initDStream(bitD1, istart1 as *const c_void, length1);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(bitD2, istart2 as *const c_void, length2);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(bitD3, istart3 as *const c_void, length3);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(bitD4, istart4 as *const c_void, length4);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }

        macro_rules! X2_2 {
            ($op:expr, $bd:expr) => {
                *$op = HUFv07_decodeSymbolX2($bd, dt, dtLog);
                $op = $op.add(1);
            };
        }
        macro_rules! X2_1 {
            ($op:expr, $bd:expr) => {
                *$op = HUFv07_decodeSymbolX2($bd, dt, dtLog);
                $op = $op.add(1);
            };
        }
        macro_rules! X2_0 {
            ($op:expr, $bd:expr) => {
                *$op = HUFv07_decodeSymbolX2($bd, dt, dtLog);
                $op = $op.add(1);
            };
        }

        endSignal = BITv07_reloadDStream(bitD1)
            | BITv07_reloadDStream(bitD2)
            | BITv07_reloadDStream(bitD3)
            | BITv07_reloadDStream(bitD4);
        while (endSignal == BITv07_DStream_unfinished) && (op4 < oend.offset(-7)) {
            X2_2!(op1, bitD1);
            X2_2!(op2, bitD2);
            X2_2!(op3, bitD3);
            X2_2!(op4, bitD4);
            X2_1!(op1, bitD1);
            X2_1!(op2, bitD2);
            X2_1!(op3, bitD3);
            X2_1!(op4, bitD4);
            X2_2!(op1, bitD1);
            X2_2!(op2, bitD2);
            X2_2!(op3, bitD3);
            X2_2!(op4, bitD4);
            X2_0!(op1, bitD1);
            X2_0!(op2, bitD2);
            X2_0!(op3, bitD3);
            X2_0!(op4, bitD4);
            endSignal = BITv07_reloadDStream(bitD1)
                | BITv07_reloadDStream(bitD2)
                | BITv07_reloadDStream(bitD3)
                | BITv07_reloadDStream(bitD4);
        }

        if op1 > opStart2 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        if op2 > opStart3 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        if op3 > opStart4 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }

        HUFv07_decodeStreamX2(op1, bitD1, opStart2, dt, dtLog);
        HUFv07_decodeStreamX2(op2, bitD2, opStart3, dt, dtLog);
        HUFv07_decodeStreamX2(op3, bitD3, opStart4, dt, dtLog);
        HUFv07_decodeStreamX2(op4, bitD4, oend, dt, dtLog);

        endSignal = BITv07_endOfDStream(bitD1)
            & BITv07_endOfDStream(bitD2)
            & BITv07_endOfDStream(bitD3)
            & BITv07_endOfDStream(bitD4);
        if endSignal == 0 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }

        dstSize
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
    let dtd = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        return ERROR(ec::GENERIC);
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
    let mut ip = cSrc as *const u8;

    let hSize = HUFv07_readDTableX2(dctx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
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
    let mut DTable = [0u32; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)];
    DTable[0] = (HUFv07_TABLELOG_MAX - 1) * 0x1000001;
    HUFv07_decompress4X2_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

/* double-symbols decoding */
#[repr(C)]
#[derive(Clone, Copy)]
struct HUFv07_DEltX4 {
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

unsafe fn HUFv07_fillDTableX4Level2(
    DTable: *mut HUFv07_DEltX4,
    sizeLog: u32,
    consumed: u32,
    rankValOrigin: *const u32,
    minWeight: i32,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: u32,
    nbBitsBaseline: u32,
    baseSeq: u16,
) {
    let mut DElt = HUFv07_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal = [0u32; HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1];

    core::ptr::copy_nonoverlapping(
        rankValOrigin,
        rankVal.as_mut_ptr(),
        HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1,
    );

    if minWeight > 1 {
        let skipSize = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut u16 as *mut u8, baseSeq);
        DElt.nbBits = consumed as u8;
        DElt.length = 1;
        let mut i: u32 = 0;
        while i < skipSize {
            *DTable.add(i as usize) = DElt;
            i += 1;
        }
    }

    {
        let mut s: u32 = 0;
        while s < sortedListSize {
            let symbol = (*sortedSymbols.add(s as usize)).symbol as u32;
            let weight = (*sortedSymbols.add(s as usize)).weight as u32;
            let nbBits = nbBitsBaseline - weight;
            let length = 1u32 << (sizeLog - nbBits);
            let start = rankVal[weight as usize];
            let mut i = start;
            let end = start + length;

            MEM_writeLE16(
                &mut DElt.sequence as *mut u16 as *mut u8,
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
}

type rankVal_t = [[u32; HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1]; HUFv07_TABLELOG_ABSOLUTEMAX as usize];

unsafe fn HUFv07_fillDTableX4(
    DTable: *mut HUFv07_DEltX4,
    targetLog: u32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: u32,
    rankStart: *const u32,
    rankValOrigin: *mut rankVal_t,
    maxWeight: u32,
    nbBitsBaseline: u32,
) {
    let mut rankVal = [0u32; HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1];
    let scaleLog: i32 = nbBitsBaseline as i32 - targetLog as i32;
    let minBits = nbBitsBaseline - maxWeight;
    let mut s: u32;

    core::ptr::copy_nonoverlapping(
        (*rankValOrigin)[0].as_ptr(),
        rankVal.as_mut_ptr(),
        HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1,
    );

    s = 0;
    while s < sortedListSize {
        let symbol = (*sortedList.add(s as usize)).symbol as u16;
        let weight = (*sortedList.add(s as usize)).weight as u32;
        let nbBits = nbBitsBaseline - weight;
        let start = rankVal[weight as usize];
        let length = 1u32 << (targetLog - nbBits);

        if targetLog - nbBits >= minBits {
            let sortedRank: u32;
            let mut minWeight = nbBits as i32 + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUFv07_fillDTableX4Level2(
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
            let mut DElt = HUFv07_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };
            MEM_writeLE16(&mut DElt.sequence as *mut u16 as *mut u8, symbol);
            DElt.nbBits = nbBits as u8;
            DElt.length = 1;
            {
                let end = start + length;
                let mut u = start;
                while u < end {
                    *DTable.add(u as usize) = DElt;
                    u += 1;
                }
            }
        }
        rankVal[weight as usize] += length;
        s += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX4(
    DTable: *mut HUFv07_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightList = [0u8; HUFv07_SYMBOLVALUE_MAX as usize + 1];
    let mut sortedSymbol = [sortedSymbol_t { symbol: 0, weight: 0 }; HUFv07_SYMBOLVALUE_MAX as usize + 1];
    let mut rankStats = [0u32; HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1];
    let mut rankStart0 = [0u32; HUFv07_TABLELOG_ABSOLUTEMAX as usize + 2];
    let mut rankVal: rankVal_t =
        [[0u32; HUFv07_TABLELOG_ABSOLUTEMAX as usize + 1]; HUFv07_TABLELOG_ABSOLUTEMAX as usize];
    let mut tableLog: u32 = 0;
    let mut maxW: u32;
    let sizeOfSort: u32;
    let mut nbSymbols: u32 = 0;
    let mut dtd = HUFv07_getDTableDesc(DTable);
    let maxTableLog = dtd.maxTableLog as u32;
    let iSize: usize;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv07_DEltX4;

    if maxTableLog > HUFv07_TABLELOG_ABSOLUTEMAX {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }

    // rankStart = rankStart0 + 1
    iSize = HUFv07_readStats(
        weightList.as_mut_ptr(),
        HUFv07_SYMBOLVALUE_MAX as usize + 1,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }

    if tableLog > maxTableLog {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }

    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW -= 1;
    }

    {
        let mut nextRankStart: u32 = 0;
        let mut w: u32 = 1;
        while w < maxW + 1 {
            let current = nextRankStart;
            nextRankStart += rankStats[w as usize];
            rankStart0[(w + 1) as usize] = current; // rankStart[w] = rankStart0[w+1]
            w += 1;
        }
        rankStart0[1] = nextRankStart; // rankStart[0]
        sizeOfSort = nextRankStart;
    }

    {
        let mut s: u32 = 0;
        while s < nbSymbols {
            let w = weightList[s as usize] as u32;
            let r = rankStart0[(w + 1) as usize]; // rankStart[w]++
            rankStart0[(w + 1) as usize] += 1;
            sortedSymbol[r as usize].symbol = s as u8;
            sortedSymbol[r as usize].weight = w as u8;
            s += 1;
        }
        rankStart0[1] = 0; // rankStart[0] = 0
    }

    {
        // rankVal0 = rankVal[0]
        {
            let rescale: i32 = (maxTableLog as i32 - tableLog as i32) - 1;
            let mut nextRankVal: u32 = 0;
            let mut w: u32 = 1;
            while w < maxW + 1 {
                let current = nextRankVal;
                nextRankVal += rankStats[w as usize] << (w as i32 + rescale);
                rankVal[0][w as usize] = current;
                w += 1;
            }
        }
        {
            let minBits = tableLog + 1 - maxW;
            let mut consumed = minBits;
            while consumed < maxTableLog - minBits + 1 {
                let mut w: u32 = 1;
                while w < maxW + 1 {
                    rankVal[consumed as usize][w as usize] = rankVal[0][w as usize] >> consumed;
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
        &mut rankVal as *mut rankVal_t,
        maxW,
        tableLog + 1,
    );

    dtd.tableLog = maxTableLog as u8;
    dtd.tableType = 1;
    core::ptr::copy_nonoverlapping(
        &dtd as *const DTableDesc as *const u8,
        DTable as *mut u8,
        core::mem::size_of::<DTableDesc>(),
    );
    iSize
}

#[inline]
unsafe fn HUFv07_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: u32,
) -> u32 {
    let val = BITv07_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 2);
    BITv07_skipBits(DStream, (*dt.add(val)).nbBits as u32);
    (*dt.add(val)).length as u32
}

#[inline]
unsafe fn HUFv07_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: u32,
) -> u32 {
    let val = BITv07_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BITv07_skipBits(DStream, (*dt.add(val)).nbBits as u32);
    } else {
        if ((*DStream).bitsConsumed as usize) < (core::mem::size_of::<usize>() * 8) {
            BITv07_skipBits(DStream, (*dt.add(val)).nbBits as u32);
            if (*DStream).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as c_uint;
            }
        }
    }
    1
}

unsafe fn HUFv07_decodeStreamX4(
    mut p: *mut u8,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut u8,
    dt: *const HUFv07_DEltX4,
    dtLog: u32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished) && (p < pEnd.offset(-7)) {
        // X4_2
        p = p.add(HUFv07_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        // X4_1
        p = p.add(HUFv07_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        // X4_2
        p = p.add(HUFv07_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        // X4_0
        p = p.add(HUFv07_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    /* closer to end : up to 2 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished) && (p <= pEnd.offset(-2)) {
        p = p.add(HUFv07_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    while p <= pEnd.offset(-2) {
        p = p.add(HUFv07_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    if p < pEnd {
        p = p.add(HUFv07_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    p as usize - pStart as usize
}

unsafe fn HUFv07_decompress1X4_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let mut bitD = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
    let bitD = bitD.as_mut_ptr();

    {
        let errorCode = BITv07_initDStream(bitD, cSrc, cSrcSize);
        if HUFv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    {
        let ostart = dst as *mut u8;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX4;
        let dtd = HUFv07_getDTableDesc(DTable);
        HUFv07_decodeStreamX4(ostart, bitD, oend, dt, dtd.tableLog as u32);
    }

    if BITv07_endOfDStream(bitD) == 0 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

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
    let dtd = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 1 {
        return ERROR(ec::GENERIC);
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
    let mut ip = cSrc as *const u8;

    let hSize = HUFv07_readDTableX4(DCtx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
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
    let mut DTable = [0u32; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)];
    DTable[0] = HUFv07_TABLELOG_MAX * 0x1000001;
    HUFv07_decompress1X4_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

unsafe fn HUFv07_decompress4X4_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    {
        let istart = cSrc as *const u8;
        let ostart = dst as *mut u8;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX4;

        let mut bitD1 = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
        let bitD1 = bitD1.as_mut_ptr();
        let mut bitD2 = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
        let bitD2 = bitD2.as_mut_ptr();
        let mut bitD3 = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
        let bitD3 = bitD3.as_mut_ptr();
        let mut bitD4 = core::mem::MaybeUninit::<BITv07_DStream_t>::uninit();
        let bitD4 = bitD4.as_mut_ptr();
        let length1 = MEM_readLE16(istart) as usize;
        let length2 = MEM_readLE16(istart.add(2)) as usize;
        let length3 = MEM_readLE16(istart.add(4)) as usize;
        let length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
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
        let dtd = HUFv07_getDTableDesc(DTable);
        let dtLog = dtd.tableLog as u32;

        if length4 > cSrcSize {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        {
            let errorCode = BITv07_initDStream(bitD1, istart1 as *const c_void, length1);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(bitD2, istart2 as *const c_void, length2);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(bitD3, istart3 as *const c_void, length3);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(bitD4, istart4 as *const c_void, length4);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }

        macro_rules! X4_2 {
            ($op:expr, $bd:expr) => {
                $op = $op.add(HUFv07_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize);
            };
        }
        macro_rules! X4_1 {
            ($op:expr, $bd:expr) => {
                $op = $op.add(HUFv07_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize);
            };
        }
        macro_rules! X4_0 {
            ($op:expr, $bd:expr) => {
                $op = $op.add(HUFv07_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize);
            };
        }

        endSignal = BITv07_reloadDStream(bitD1)
            | BITv07_reloadDStream(bitD2)
            | BITv07_reloadDStream(bitD3)
            | BITv07_reloadDStream(bitD4);
        while (endSignal == BITv07_DStream_unfinished) && (op4 < oend.offset(-7)) {
            X4_2!(op1, bitD1);
            X4_2!(op2, bitD2);
            X4_2!(op3, bitD3);
            X4_2!(op4, bitD4);
            X4_1!(op1, bitD1);
            X4_1!(op2, bitD2);
            X4_1!(op3, bitD3);
            X4_1!(op4, bitD4);
            X4_2!(op1, bitD1);
            X4_2!(op2, bitD2);
            X4_2!(op3, bitD3);
            X4_2!(op4, bitD4);
            X4_0!(op1, bitD1);
            X4_0!(op2, bitD2);
            X4_0!(op3, bitD3);
            X4_0!(op4, bitD4);

            endSignal = BITv07_reloadDStream(bitD1)
                | BITv07_reloadDStream(bitD2)
                | BITv07_reloadDStream(bitD3)
                | BITv07_reloadDStream(bitD4);
        }

        if op1 > opStart2 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        if op2 > opStart3 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        if op3 > opStart4 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }

        HUFv07_decodeStreamX4(op1, bitD1, opStart2, dt, dtLog);
        HUFv07_decodeStreamX4(op2, bitD2, opStart3, dt, dtLog);
        HUFv07_decodeStreamX4(op3, bitD3, opStart4, dt, dtLog);
        HUFv07_decodeStreamX4(op4, bitD4, oend, dt, dtLog);

        {
            let endCheck = BITv07_endOfDStream(bitD1)
                & BITv07_endOfDStream(bitD2)
                & BITv07_endOfDStream(bitD3)
                & BITv07_endOfDStream(bitD4);
            if endCheck == 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
        }

        dstSize
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
    let dtd = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 1 {
        return ERROR(ec::GENERIC);
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
    let mut ip = cSrc as *const u8;

    let hSize = HUFv07_readDTableX4(dctx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
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
    let mut DTable = [0u32; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)];
    DTable[0] = HUFv07_TABLELOG_MAX * 0x1000001;
    HUFv07_decompress4X4_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

/* Generic decompression selector */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    let dtd = HUFv07_getDTableDesc(DTable);
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
    let dtd = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        HUFv07_decompress4X4_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    } else {
        HUFv07_decompress4X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    }
}

#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: u32,
    decode256Time: u32,
}

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
pub unsafe extern "C" fn HUFv07_selectDecoder(dstSize: usize, cSrcSize: usize) -> u32 {
    let Q = (cSrcSize * 16 / dstSize) as u32;
    let D256 = (dstSize >> 8) as u32;
    let DTime0 = algoTime[Q as usize][0].tableTime + (algoTime[Q as usize][0].decode256Time * D256);
    let mut DTime1 =
        algoTime[Q as usize][1].tableTime + (algoTime[Q as usize][1].decode256Time * D256);
    DTime1 += DTime1 >> 3;

    (DTime1 < DTime0) as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    if dstSize == 0 {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if cSrcSize > dstSize {
        return ERROR(ec::CORRUPTION_DETECTED);
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const u8) as i32, dstSize);
        return dstSize;
    }

    {
        let algoNb = HUFv07_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUFv07_decompress4X4(dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv07_decompress4X2(dst, dstSize, cSrc, cSrcSize)
        }
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
    if dstSize == 0 {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if cSrcSize > dstSize {
        return ERROR(ec::CORRUPTION_DETECTED);
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const u8) as i32, dstSize);
        return dstSize;
    }

    {
        let algoNb = HUFv07_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUFv07_decompress4X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv07_decompress4X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        }
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
    if dstSize == 0 {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if (cSrcSize >= dstSize) || (cSrcSize <= 1) {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    {
        let algoNb = HUFv07_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUFv07_decompress4X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv07_decompress4X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        }
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
    if dstSize == 0 {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if cSrcSize > dstSize {
        return ERROR(ec::CORRUPTION_DETECTED);
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const u8) as i32, dstSize);
        return dstSize;
    }

    {
        let algoNb = HUFv07_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUFv07_decompress1X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv07_decompress1X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        }
    }
}

/* ****************************************************************
 * ZSTD / ZBUFF error management (exported)
 * ************************************************************** */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

/* ****************************************************************
 * Custom memory allocation
 * ************************************************************** */
type ZSTDv07_allocFunction = Option<unsafe extern "C" fn(opaque: *mut c_void, size: usize) -> *mut c_void>;
type ZSTDv07_freeFunction = Option<unsafe extern "C" fn(opaque: *mut c_void, address: *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTDv07_customMem {
    customAlloc: ZSTDv07_allocFunction,
    customFree: ZSTDv07_freeFunction,
    opaque: *mut c_void,
}

unsafe extern "C" fn ZSTDv07_defaultAllocFunction(_opaque: *mut c_void, size: usize) -> *mut c_void {
    malloc(size)
}

unsafe extern "C" fn ZSTDv07_defaultFreeFunction(_opaque: *mut c_void, address: *mut c_void) {
    free(address);
}

const defaultCustomMem: ZSTDv07_customMem = ZSTDv07_customMem {
    customAlloc: Some(ZSTDv07_defaultAllocFunction),
    customFree: Some(ZSTDv07_defaultFreeFunction),
    opaque: core::ptr::null_mut(),
};

/* ****************************************************************
 * ZSTD frame params (public ABI)
 * ************************************************************** */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTDv07_frameParams {
    pub frameContentSize: c_ulonglong,
    pub windowSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
}

/* shared helpers */
#[inline]
unsafe fn ZSTDv07_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}
#[inline]
unsafe fn ZSTDv07_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

#[inline]
unsafe fn ZSTDv07_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const u8;
    let mut op = dst as *mut u8;
    let oend = op.offset(length);
    loop {
        ZSTDv07_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if op >= oend {
            break;
        }
    }
}

/* ****************************************************************
 * Context management
 * ************************************************************** */
const ZSTDds_getFrameHeaderSize: u32 = 0;
const ZSTDds_decodeFrameHeader: u32 = 1;
const ZSTDds_decodeBlockHeader: u32 = 2;
const ZSTDds_decompressBlock: u32 = 3;
const ZSTDds_decodeSkippableHeader: u32 = 4;
const ZSTDds_skipFrame: u32 = 5;
type ZSTDv07_dStage = u32;

const bt_compressed: u32 = 0;
const bt_raw: u32 = 1;
const bt_rle: u32 = 2;
const bt_end: u32 = 3;
type blockType_t = u32;

const lbt_huffman: u32 = 0;
const lbt_repeat: u32 = 1;
const lbt_raw: u32 = 2;
const lbt_rle: u32 = 3;

#[repr(C)]
pub struct ZSTDv07_DCtx {
    LLTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(LLFSELog)],
    OffTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(OffFSELog)],
    MLTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(MLFSELog)],
    hufTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    previousDstEnd: *const c_void,
    base: *const c_void,
    vBase: *const c_void,
    dictEnd: *const c_void,
    expected: usize,
    rep: [u32; 3],
    fParams: ZSTDv07_frameParams,
    bType: blockType_t,
    stage: ZSTDv07_dStage,
    litEntropy: u32,
    fseEntropy: u32,
    xxhState: XXH64_state_t,
    headerSize: usize,
    dictID: u32,
    litPtr: *const u8,
    customMem: ZSTDv07_customMem,
    litSize: usize,
    litBuffer: [u8; ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + WILDCOPY_OVERLENGTH],
    headerBuffer: [u8; ZSTDv07_FRAMEHEADERSIZE_MAX],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_sizeofDCtx(_dctx: *const ZSTDv07_DCtx) -> usize {
    core::mem::size_of::<ZSTDv07_DCtx>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_estimateDCtxSize() -> usize {
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
    (*dctx).hufTable[0] = (ZSTD_HUFFDTABLE_CAPACITY_LOG) * 0x1000001;
    (*dctx).litEntropy = 0;
    (*dctx).fseEntropy = 0;
    (*dctx).dictID = 0;
    {
        let mut i = 0;
        while i < ZSTDv07_REP_NUM {
            (*dctx).rep[i] = repStartValue[i];
            i += 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDCtx_advanced(
    mut customMem: ZSTDv07_customMem,
) -> *mut ZSTDv07_DCtx {
    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem;
    }

    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    let dctx = (customMem.customAlloc.unwrap())(
        customMem.opaque,
        core::mem::size_of::<ZSTDv07_DCtx>(),
    ) as *mut ZSTDv07_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    memcpy(
        &mut (*dctx).customMem as *mut ZSTDv07_customMem as *mut c_void,
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
        return 0;
    }
    ((*dctx).customMem.customFree.unwrap())((*dctx).customMem.opaque, dctx as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_copyDCtx(dstDCtx: *mut ZSTDv07_DCtx, srcDCtx: *const ZSTDv07_DCtx) {
    memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv07_DCtx>()
            - (ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + WILDCOPY_OVERLENGTH + ZSTDv07_frameHeaderSize_max),
    );
}

/* ****************************************************************
 * Frame header / block header decoding
 * ************************************************************** */
unsafe fn ZSTDv07_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    {
        let fhd = *((src as *const u8).add(4));
        let dictID = (fhd & 3) as u32;
        let directMode = ((fhd >> 5) & 1) as u32;
        let fcsId = (fhd >> 6) as u32;
        ZSTDv07_frameHeaderSize_min
            + (directMode == 0) as usize
            + ZSTDv07_did_fieldSize[dictID as usize]
            + ZSTDv07_fcs_fieldSize[fcsId as usize]
            + ((directMode != 0) && (ZSTDv07_fcs_fieldSize[fcsId as usize] == 0)) as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getFrameParams(
    fparamsPtr: *mut ZSTDv07_frameParams,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip = src as *const u8;

    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ZSTDv07_frameHeaderSize_min;
    }
    memset(fparamsPtr as *mut c_void, 0, core::mem::size_of::<ZSTDv07_frameParams>());
    if MEM_readLE32(src as *const u8) != ZSTDv07_MAGICNUMBER {
        if (MEM_readLE32(src as *const u8) & 0xFFFFFFF0u32) == ZSTDv07_MAGIC_SKIPPABLE_START {
            if srcSize < ZSTDv07_skippableHeaderSize {
                return ZSTDv07_skippableHeaderSize;
            }
            (*fparamsPtr).frameContentSize =
                MEM_readLE32((src as *const u8).add(4)) as c_ulonglong;
            (*fparamsPtr).windowSize = 0;
            return 0;
        }
        return ERROR(ec::PREFIX_UNKNOWN);
    }

    {
        let fhsize = ZSTDv07_frameHeaderSize(src, srcSize);
        if srcSize < fhsize {
            return fhsize;
        }
    }

    {
        let fhdByte = *ip.add(4);
        let mut pos: usize = 5;
        let dictIDSizeCode = (fhdByte & 3) as u32;
        let checksumFlag = ((fhdByte >> 2) & 1) as u32;
        let directMode = ((fhdByte >> 5) & 1) as u32;
        let fcsID = (fhdByte >> 6) as u32;
        let windowSizeMax: u32 = 1u32 << ZSTDv07_WINDOWLOG_MAX;
        let mut windowSize: u32 = 0;
        let mut dictID: u32 = 0;
        let mut frameContentSize: u64 = 0;
        if (fhdByte & 0x08) != 0 {
            return ERROR(ec::FRAMEPARAMETER_UNSUPPORTED);
        }
        if directMode == 0 {
            let wlByte = *ip.add(pos);
            pos += 1;
            let windowLog = (wlByte >> 3) as u32 + ZSTDv07_WINDOWLOG_ABSOLUTEMIN;
            if windowLog > ZSTDv07_WINDOWLOG_MAX {
                return ERROR(ec::FRAMEPARAMETER_UNSUPPORTED);
            }
            windowSize = 1u32 << windowLog;
            windowSize += (windowSize >> 3) * (wlByte & 7) as u32;
        }

        match dictIDSizeCode {
            1 => {
                dictID = *ip.add(pos) as u32;
                pos += 1;
            }
            2 => {
                dictID = MEM_readLE16(ip.add(pos)) as u32;
                pos += 2;
            }
            3 => {
                dictID = MEM_readLE32(ip.add(pos));
                pos += 4;
            }
            _ => {}
        }
        match fcsID {
            1 => {
                frameContentSize = MEM_readLE16(ip.add(pos)) as u64 + 256;
            }
            2 => {
                frameContentSize = MEM_readLE32(ip.add(pos)) as u64;
            }
            3 => {
                frameContentSize = MEM_readLE64(ip.add(pos));
            }
            _ => {
                if directMode != 0 {
                    frameContentSize = *ip.add(pos) as u64;
                }
            }
        }
        if windowSize == 0 {
            windowSize = frameContentSize as u32;
        }
        if windowSize > windowSizeMax {
            return ERROR(ec::FRAMEPARAMETER_UNSUPPORTED);
        }
        (*fparamsPtr).frameContentSize = frameContentSize as c_ulonglong;
        (*fparamsPtr).windowSize = windowSize;
        (*fparamsPtr).dictID = dictID;
        (*fparamsPtr).checksumFlag = checksumFlag;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getDecompressedSize(
    src: *const c_void,
    srcSize: usize,
) -> c_ulonglong {
    let mut fparams = core::mem::MaybeUninit::<ZSTDv07_frameParams>::uninit();
    let frResult = ZSTDv07_getFrameParams(fparams.as_mut_ptr(), src, srcSize);
    if frResult != 0 {
        return 0;
    }
    fparams.assume_init().frameContentSize
}

unsafe fn ZSTDv07_decodeFrameHeader(
    dctx: *mut ZSTDv07_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result = ZSTDv07_getFrameParams(&mut (*dctx).fParams, src, srcSize);
    if (*dctx).fParams.dictID != 0 && ((*dctx).dictID != (*dctx).fParams.dictID) {
        return ERROR(ec::DICTIONARY_WRONG);
    }
    if (*dctx).fParams.checksumFlag != 0 {
        XXH64_reset(&mut (*dctx).xxhState, 0);
    }
    result
}

#[repr(C)]
#[derive(Clone, Copy)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: u32,
}

unsafe fn ZSTDv07_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_ = src as *const u8;
    let cSize: u32;

    if srcSize < ZSTDv07_blockHeaderSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    (*bpPtr).blockType = ((*in_) >> 6) as blockType_t;
    cSize = *in_.add(2) as u32 + ((*in_.add(1) as u32) << 8) + (((*in_.add(0) & 7) as u32) << 16);
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as usize
}

unsafe fn ZSTDv07_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize > dstCapacity {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

unsafe fn ZSTDv07_decodeLiteralsBlock(
    dctx: *mut ZSTDv07_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const u8;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    match (*istart.add(0) >> 6) as u32 {
        lbt_huffman => {
            let litSize: usize;
            let litCSize: usize;
            let mut singleStream: u32 = 0;
            let mut lhSize = ((*istart.add(0) >> 4) & 3) as u32;
            if srcSize < 5 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            match lhSize {
                2 => {
                    lhSize = 4;
                    litSize = (((*istart.add(0) & 15) as usize) << 10)
                        + ((*istart.add(1) as usize) << 2)
                        + ((*istart.add(2) as usize) >> 6);
                    litCSize = (((*istart.add(2) & 63) as usize) << 8) + *istart.add(3) as usize;
                }
                3 => {
                    lhSize = 5;
                    litSize = (((*istart.add(0) & 15) as usize) << 14)
                        + ((*istart.add(1) as usize) << 6)
                        + ((*istart.add(2) as usize) >> 2);
                    litCSize = (((*istart.add(2) & 3) as usize) << 16)
                        + ((*istart.add(3) as usize) << 8)
                        + *istart.add(4) as usize;
                }
                _ => {
                    lhSize = 3;
                    singleStream = (*istart.add(0) & 16) as u32;
                    litSize = (((*istart.add(0) & 15) as usize) << 6)
                        + ((*istart.add(1) as usize) >> 2);
                    litCSize = (((*istart.add(1) & 3) as usize) << 8) + *istart.add(2) as usize;
                }
            }
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ec::CORRUPTION_DETECTED);
            }

            let res = if singleStream != 0 {
                HUFv07_decompress1X2_DCtx(
                    (*dctx).hufTable.as_mut_ptr(),
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            } else {
                HUFv07_decompress4X_hufOnly(
                    (*dctx).hufTable.as_mut_ptr(),
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            };
            if HUFv07_isError(res) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            (*dctx).litEntropy = 1;
            memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            litCSize + lhSize as usize
        }
        lbt_repeat => {
            let litSize: usize;
            let litCSize: usize;
            let mut lhSize = ((*istart.add(0) >> 4) & 3) as u32;
            if lhSize != 1 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            if (*dctx).litEntropy == 0 {
                return ERROR(ec::DICTIONARY_CORRUPTED);
            }

            lhSize = 3;
            litSize = (((*istart.add(0) & 15) as usize) << 6) + ((*istart.add(1) as usize) >> 2);
            litCSize = (((*istart.add(1) & 3) as usize) << 8) + *istart.add(2) as usize;
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ec::CORRUPTION_DETECTED);
            }

            {
                let errorCode = HUFv07_decompress1X4_usingDTable(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.add(lhSize as usize) as *const c_void,
                    litCSize,
                    (*dctx).hufTable.as_ptr(),
                );
                if HUFv07_isError(errorCode) != 0 {
                    return ERROR(ec::CORRUPTION_DETECTED);
                }
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
        lbt_raw => {
            let litSize: usize;
            let mut lhSize = ((*istart.add(0) >> 4) & 3) as u32;
            match lhSize {
                2 => {
                    litSize = (((*istart.add(0) & 15) as usize) << 8) + *istart.add(1) as usize;
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
                    return ERROR(ec::CORRUPTION_DETECTED);
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
            (*dctx).litPtr = istart.add(lhSize as usize);
            (*dctx).litSize = litSize;
            lhSize as usize + litSize
        }
        lbt_rle => {
            let litSize: usize;
            let mut lhSize = ((*istart.add(0) >> 4) & 3) as u32;
            match lhSize {
                2 => {
                    litSize = (((*istart.add(0) & 15) as usize) << 8) + *istart.add(1) as usize;
                }
                3 => {
                    litSize = (((*istart.add(0) & 15) as usize) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + *istart.add(2) as usize;
                    if srcSize < 4 {
                        return ERROR(ec::CORRUPTION_DETECTED);
                    }
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) & 31) as usize;
                }
            }
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.add(lhSize as usize) as i32,
                litSize + WILDCOPY_OVERLENGTH,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            lhSize as usize + 1
        }
        _ => ERROR(ec::CORRUPTION_DETECTED),
    }
}

/* ****************************************************************
 * Sequence decoding
 * ************************************************************** */
unsafe fn ZSTDv07_buildSeqTable(
    DTable: *mut FSEv07_DTable,
    type_: u32,
    max: u32,
    maxLog: u32,
    src: *const c_void,
    srcSize: usize,
    defaultNorm: *const i16,
    defaultLog: u32,
    flagRepeatTable: u32,
) -> usize {
    match type_ {
        FSEv07_ENCODING_RLE => {
            if srcSize == 0 {
                return ERROR(ec::SRCSIZE_WRONG);
            }
            if (*(src as *const u8)) as u32 > max {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            FSEv07_buildDTable_rle(DTable, *(src as *const u8));
            1
        }
        FSEv07_ENCODING_RAW => {
            FSEv07_buildDTable(DTable, defaultNorm, max, defaultLog);
            0
        }
        FSEv07_ENCODING_STATIC => {
            if flagRepeatTable == 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            0
        }
        _ => {
            // FSEv07_ENCODING_DYNAMIC
            let mut tableLog: u32 = 0;
            let mut norm = [0i16; MaxSeq + 1];
            let mut max_mut = max;
            let headerSize =
                FSEv07_readNCount(norm.as_mut_ptr(), &mut max_mut, &mut tableLog, src, srcSize);
            if FSEv07_isError(headerSize) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            if tableLog > maxLog {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            FSEv07_buildDTable(DTable, norm.as_ptr(), max_mut, tableLog);
            headerSize
        }
    }
}

unsafe fn ZSTDv07_decodeSeqHeaders(
    nbSeqPtr: *mut c_int,
    DTableLL: *mut FSEv07_DTable,
    DTableML: *mut FSEv07_DTable,
    DTableOffb: *mut FSEv07_DTable,
    flagRepeatTable: u32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const u8;
    let iend = istart.add(srcSize);
    let mut ip = istart;

    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    {
        let mut nbSeq: c_int = *ip as c_int;
        ip = ip.add(1);
        if nbSeq == 0 {
            *nbSeqPtr = 0;
            return 1;
        }
        if nbSeq > 0x7F {
            if nbSeq == 0xFF {
                if ip.add(2) > iend {
                    return ERROR(ec::SRCSIZE_WRONG);
                }
                nbSeq = MEM_readLE16(ip) as c_int + LONGNBSEQ;
                ip = ip.add(2);
            } else {
                if ip >= iend {
                    return ERROR(ec::SRCSIZE_WRONG);
                }
                nbSeq = ((nbSeq - 0x80) << 8) + *ip as c_int;
                ip = ip.add(1);
            }
        }
        *nbSeqPtr = nbSeq;
    }

    if ip.add(4) > iend {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    {
        let LLtype = (*ip >> 6) as u32;
        let OFtype = ((*ip >> 4) & 3) as u32;
        let MLtype = ((*ip >> 2) & 3) as u32;
        ip = ip.add(1);

        {
            let llhSize = ZSTDv07_buildSeqTable(
                DTableLL,
                LLtype,
                MaxLL as u32,
                LLFSELog,
                ip as *const c_void,
                iend as usize - ip as usize,
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(llhSize) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            ip = ip.add(llhSize);
        }
        {
            let ofhSize = ZSTDv07_buildSeqTable(
                DTableOffb,
                OFtype,
                MaxOff as u32,
                OffFSELog,
                ip as *const c_void,
                iend as usize - ip as usize,
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(ofhSize) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            ip = ip.add(ofhSize);
        }
        {
            let mlhSize = ZSTDv07_buildSeqTable(
                DTableML,
                MLtype,
                MaxML as u32,
                MLFSELog,
                ip as *const c_void,
                iend as usize - ip as usize,
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(mlhSize) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            ip = ip.add(mlhSize);
        }
    }

    ip as usize - istart as usize
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
    DStream: BITv07_DStream_t,
    stateLL: FSEv07_DState_t,
    stateOffb: FSEv07_DState_t,
    stateML: FSEv07_DState_t,
    prevOffset: [usize; ZSTDv07_REP_INIT],
}

static LL_base: [u32; MaxLL + 1] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

static ML_base: [u32; MaxML + 1] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
    0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003,
];

static OF_base: [u32; MaxOff + 1] = [
    0, 1, 1, 5, 0xD, 0x1D, 0x3D, 0x7D, 0xFD, 0x1FD, 0x3FD, 0x7FD, 0xFFD, 0x1FFD, 0x3FFD, 0x7FFD,
    0xFFFD, 0x1FFFD, 0x3FFFD, 0x7FFFD, 0xFFFFD, 0x1FFFFD, 0x3FFFFD, 0x7FFFFD, 0xFFFFFD, 0x1FFFFFD,
    0x3FFFFFD, 0x7FFFFFD, 0xFFFFFFD,
];

unsafe fn ZSTDv07_decodeSequence(seqState: *mut seqState_t) -> seq_t {
    let mut seq = seq_t {
        litLength: 0,
        matchLength: 0,
        offset: 0,
    };

    let llCode = FSEv07_peekSymbol(&(*seqState).stateLL) as u32;
    let mlCode = FSEv07_peekSymbol(&(*seqState).stateML) as u32;
    let ofCode = FSEv07_peekSymbol(&(*seqState).stateOffb) as u32;

    let llBits = LL_bits[llCode as usize];
    let mlBits = ML_bits[mlCode as usize];
    let ofBits = ofCode;
    let totalBits = llBits + mlBits + ofBits;

    {
        let mut offset: usize;
        if ofCode == 0 {
            offset = 0;
        } else {
            offset = OF_base[ofCode as usize] as usize
                + BITv07_readBits(&mut (*seqState).DStream, ofBits);
            if MEM_32bits() != 0 {
                BITv07_reloadDStream(&mut (*seqState).DStream);
            }
        }

        if ofCode <= 1 {
            if (llCode == 0) && (offset <= 1) {
                offset = 1 - offset;
            }
            if offset != 0 {
                let temp = (*seqState).prevOffset[offset];
                if offset != 1 {
                    (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
                }
                (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
                (*seqState).prevOffset[0] = temp;
                offset = temp;
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

    seq.matchLength = ML_base[mlCode as usize] as usize
        + (if mlCode > 31 {
            BITv07_readBits(&mut (*seqState).DStream, mlBits)
        } else {
            0
        });
    if MEM_32bits() != 0 && (mlBits + llBits > 24) {
        BITv07_reloadDStream(&mut (*seqState).DStream);
    }

    seq.litLength = LL_base[llCode as usize] as usize
        + (if llCode > 15 {
            BITv07_readBits(&mut (*seqState).DStream, llBits)
        } else {
            0
        });
    if MEM_32bits() != 0
        || (totalBits > 64 - 7 - (LLFSELog + MLFSELog + OffFSELog))
    {
        BITv07_reloadDStream(&mut (*seqState).DStream);
    }

    FSEv07_updateState(&mut (*seqState).stateLL, &mut (*seqState).DStream);
    FSEv07_updateState(&mut (*seqState).stateML, &mut (*seqState).DStream);
    if MEM_32bits() != 0 {
        BITv07_reloadDStream(&mut (*seqState).DStream);
    }
    FSEv07_updateState(&mut (*seqState).stateOffb, &mut (*seqState).DStream);

    seq
}

unsafe fn ZSTDv07_execSequence(
    mut op: *mut u8,
    oend: *mut u8,
    mut sequence: seq_t,
    litPtr: *mut *const u8,
    litLimit: *const u8,
    base: *const u8,
    vBase: *const u8,
    dictEnd: *const u8,
) -> usize {
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.add(sequenceLength);
    let oend_w = oend.offset(-(WILDCOPY_OVERLENGTH as isize));
    let iLitEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const u8 = oLitEnd.offset(-(sequence.offset as isize)) as *const u8;

    if sequence.litLength + WILDCOPY_OVERLENGTH > (oend as usize - op as usize) {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if sequenceLength > (oend as usize - op as usize) {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if sequence.litLength > (litLimit as usize - (*litPtr) as usize) {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    ZSTDv07_wildcopy(op as *mut c_void, (*litPtr) as *const c_void, sequence.litLength as isize);
    op = oLitEnd;
    *litPtr = iLitEnd;

    if sequence.offset > (oLitEnd as usize - base as usize) {
        if sequence.offset > (oLitEnd as usize - vBase as usize) {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        match_ = dictEnd.offset(-((base as usize - match_ as usize) as isize));
        if match_.add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        {
            let length1 = dictEnd as usize - match_ as usize;
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.add(length1);
            sequence.matchLength -= length1;
            match_ = base;
            if op > oend_w || sequence.matchLength < MINMATCH {
                while op < oMatchEnd {
                    *op = *match_;
                    op = op.add(1);
                    match_ = match_.add(1);
                }
                return sequenceLength;
            }
        }
    }

    if sequence.offset < 8 {
        static dec32table: [u32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
        static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
        let sub2 = dec64table[sequence.offset];
        *op.add(0) = *match_.add(0);
        *op.add(1) = *match_.add(1);
        *op.add(2) = *match_.add(2);
        *op.add(3) = *match_.add(3);
        match_ = match_.add(dec32table[sequence.offset] as usize);
        ZSTDv07_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.offset(-(sub2 as isize));
    } else {
        ZSTDv07_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.add(8);
    match_ = match_.add(8);

    if oMatchEnd > oend.offset(-((16 - MINMATCH) as isize)) {
        if op < oend_w {
            ZSTDv07_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                oend_w as isize - op as isize,
            );
            match_ = match_.add(oend_w as usize - op as usize);
            op = oend_w;
        }
        while op < oMatchEnd {
            *op = *match_;
            op = op.add(1);
            match_ = match_.add(1);
        }
    } else {
        ZSTDv07_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize - 8,
        );
    }
    sequenceLength
}

unsafe fn ZSTDv07_decompressSequences(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
) -> usize {
    let mut ip = seqStart as *const u8;
    let iend = ip.add(seqSize);
    let ostart = dst as *mut u8;
    let oend = ostart.add(maxDstSize);
    let mut op = ostart;
    let mut litPtr = (*dctx).litPtr;
    let litEnd = litPtr.add((*dctx).litSize);
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const u8;
    let vBase = (*dctx).vBase as *const u8;
    let dictEnd = (*dctx).dictEnd as *const u8;
    let mut nbSeq: c_int = 0;

    {
        let seqHSize = ZSTDv07_decodeSeqHeaders(
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
        ip = ip.add(seqHSize);
    }

    if nbSeq != 0 {
        let mut seqState = core::mem::MaybeUninit::<seqState_t>::uninit();
        let seqState = seqState.as_mut_ptr();
        (*dctx).fseEntropy = 1;
        {
            let mut i = 0;
            while i < ZSTDv07_REP_INIT {
                (*seqState).prevOffset[i] = (*dctx).rep[i] as usize;
                i += 1;
            }
        }
        {
            let errorCode =
                BITv07_initDStream(&mut (*seqState).DStream, ip as *const c_void, iend as usize - ip as usize);
            if ERR_isError(errorCode) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
        }
        FSEv07_initDState(&mut (*seqState).stateLL, &mut (*seqState).DStream, DTableLL);
        FSEv07_initDState(&mut (*seqState).stateOffb, &mut (*seqState).DStream, DTableOffb);
        FSEv07_initDState(&mut (*seqState).stateML, &mut (*seqState).DStream, DTableML);

        while (BITv07_reloadDStream(&mut (*seqState).DStream) <= BITv07_DStream_completed)
            && nbSeq != 0
        {
            nbSeq -= 1;
            {
                let sequence = ZSTDv07_decodeSequence(seqState);
                let oneSeqSize = ZSTDv07_execSequence(
                    op, oend, sequence, &mut litPtr, litEnd, base, vBase, dictEnd,
                );
                if ZSTDv07_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.add(oneSeqSize);
            }
        }

        if nbSeq != 0 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        {
            let mut i = 0;
            while i < ZSTDv07_REP_INIT {
                (*dctx).rep[i] = (*seqState).prevOffset[i] as u32;
                i += 1;
            }
        }
    }

    {
        let lastLLSize = litEnd as usize - litPtr as usize;
        if lastLLSize > (oend as usize - op as usize) {
            return ERROR(ec::DSTSIZE_TOOSMALL);
        }
        if lastLLSize > 0 {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    op as usize - ostart as usize
}

unsafe fn ZSTDv07_checkContinuity(dctx: *mut ZSTDv07_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const u8).offset(
            -(((*dctx).previousDstEnd as isize) - ((*dctx).base as isize)),
        ) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

unsafe fn ZSTDv07_decompressBlock_internal(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    let mut ip = src as *const u8;

    if srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    {
        let litCSize = ZSTDv07_decodeLiteralsBlock(dctx, src, srcSize);
        if ZSTDv07_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.add(litCSize);
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
    ZSTDv07_checkContinuity(dctx, dst);
    dSize = ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
    (*dctx).previousDstEnd = (dst as *const u8).add(dSize) as *const c_void;
    dSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_insertBlock(
    dctx: *mut ZSTDv07_DCtx,
    blockStart: *const c_void,
    blockSize: usize,
) -> usize {
    ZSTDv07_checkContinuity(dctx, blockStart);
    (*dctx).previousDstEnd = (blockStart as *const u8).add(blockSize) as *const c_void;
    blockSize
}

unsafe fn ZSTDv07_generateNxBytes(
    dst: *mut c_void,
    dstCapacity: usize,
    byte: u8,
    length: usize,
) -> usize {
    if length > dstCapacity {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if length > 0 {
        memset(dst, byte as i32, length);
    }
    length
}

unsafe fn ZSTDv07_decompressFrame(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const u8;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut u8;
    let oend = ostart.add(dstCapacity);
    let mut op = ostart;
    let mut remainingSize = srcSize;

    if srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    {
        let frameHeaderSize = ZSTDv07_frameHeaderSize(src, ZSTDv07_frameHeaderSize_min);
        if ZSTDv07_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv07_blockHeaderSize {
            return ERROR(ec::SRCSIZE_WRONG);
        }
        if ZSTDv07_decodeFrameHeader(dctx, src, frameHeaderSize) != 0 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    loop {
        let decodedSize: usize;
        let mut blockProperties = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let cBlockSize = ZSTDv07_getcBlockSize(
            ip as *const c_void,
            iend as usize - ip as usize,
            &mut blockProperties,
        );
        if ZSTDv07_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTDv07_blockHeaderSize);
        remainingSize -= ZSTDv07_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(ec::SRCSIZE_WRONG);
        }

        match blockProperties.blockType {
            bt_compressed => {
                decodedSize = ZSTDv07_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    oend as usize - op as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_raw => {
                decodedSize = ZSTDv07_copyRawBlock(
                    op as *mut c_void,
                    oend as usize - op as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_rle => {
                decodedSize = ZSTDv07_generateNxBytes(
                    op as *mut c_void,
                    oend as usize - op as usize,
                    *ip,
                    blockProperties.origSize as usize,
                );
            }
            bt_end => {
                if remainingSize != 0 {
                    return ERROR(ec::SRCSIZE_WRONG);
                }
                decodedSize = 0;
            }
            _ => {
                return ERROR(ec::GENERIC);
            }
        }
        if blockProperties.blockType == bt_end {
            break;
        }

        if ZSTDv07_isError(decodedSize) != 0 {
            return decodedSize;
        }
        if (*dctx).fParams.checksumFlag != 0 {
            XXH64_update(&mut (*dctx).xxhState, op as *const c_void, decodedSize);
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op as usize - ostart as usize
}

unsafe fn ZSTDv07_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv07_DCtx,
    refDCtx: *const ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv07_copyDCtx(dctx, refDCtx);
    ZSTDv07_checkContinuity(dctx, dst);
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
    ZSTDv07_checkContinuity(dctx, dst);
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
    ZSTDv07_decompress_usingDict(dctx, dst, dstCapacity, src, srcSize, core::ptr::null(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let regenSize: usize;
    let dctx = ZSTDv07_createDCtx();
    if dctx.is_null() {
        return ERROR(ec::MEMORY_ALLOCATION);
    }
    regenSize = ZSTDv07_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv07_freeDCtx(dctx);
    regenSize
}

unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
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
    dBound: *mut c_ulonglong,
) {
    let mut ip = src as *const u8;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;

    if srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::SRCSIZE_WRONG));
        return;
    }

    {
        let frameHeaderSize = ZSTDv07_frameHeaderSize(src, srcSize);
        if ZSTDv07_isError(frameHeaderSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, frameHeaderSize);
            return;
        }
        if MEM_readLE32(src as *const u8) != ZSTDv07_MAGICNUMBER {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::PREFIX_UNKNOWN));
            return;
        }
        if srcSize < frameHeaderSize + ZSTDv07_blockHeaderSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::SRCSIZE_WRONG));
            return;
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    loop {
        let mut blockProperties = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let cBlockSize =
            ZSTDv07_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv07_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.add(ZSTDv07_blockHeaderSize);
        remainingSize -= ZSTDv07_blockHeaderSize;

        if blockProperties.blockType == bt_end {
            break;
        }

        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::SRCSIZE_WRONG));
            return;
        }

        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = ip as usize - src as usize;
    *dBound = (nbBlocks * ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) as c_ulonglong;
}

/* ****************************************************************
 * Streaming Decompression API
 * ************************************************************** */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_nextSrcSizeToDecompress(dctx: *mut ZSTDv07_DCtx) -> usize {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isSkipFrame(dctx: *mut ZSTDv07_DCtx) -> c_int {
    ((*dctx).stage == ZSTDds_skipFrame) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressContinue(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize != (*dctx).expected {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    if dstCapacity != 0 {
        ZSTDv07_checkContinuity(dctx, dst);
    }

    match (*dctx).stage {
        ZSTDds_getFrameHeaderSize => {
            if srcSize != ZSTDv07_frameHeaderSize_min {
                return ERROR(ec::SRCSIZE_WRONG);
            }
            if (MEM_readLE32(src as *const u8) & 0xFFFFFFF0u32) == ZSTDv07_MAGIC_SKIPPABLE_START {
                memcpy(
                    (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                    src,
                    ZSTDv07_frameHeaderSize_min,
                );
                (*dctx).expected = ZSTDv07_skippableHeaderSize - ZSTDv07_frameHeaderSize_min;
                (*dctx).stage = ZSTDds_decodeSkippableHeader;
                return 0;
            }
            (*dctx).headerSize = ZSTDv07_frameHeaderSize(src, ZSTDv07_frameHeaderSize_min);
            if ZSTDv07_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            memcpy(
                (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTDv07_frameHeaderSize_min,
            );
            if (*dctx).headerSize > ZSTDv07_frameHeaderSize_min {
                (*dctx).expected = (*dctx).headerSize - ZSTDv07_frameHeaderSize_min;
                (*dctx).stage = ZSTDds_decodeFrameHeader;
                return 0;
            }
            (*dctx).expected = 0;
            // fall-through
            {
                let result: usize;
                memcpy(
                    (*dctx).headerBuffer.as_mut_ptr().add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
                    src,
                    (*dctx).expected,
                );
                result = ZSTDv07_decodeFrameHeader(
                    dctx,
                    (*dctx).headerBuffer.as_ptr() as *const c_void,
                    (*dctx).headerSize,
                );
                if ZSTDv07_isError(result) != 0 {
                    return result;
                }
                (*dctx).expected = ZSTDv07_blockHeaderSize;
                (*dctx).stage = ZSTDds_decodeBlockHeader;
                return 0;
            }
        }
        ZSTDds_decodeFrameHeader => {
            let result: usize;
            memcpy(
                (*dctx).headerBuffer.as_mut_ptr().add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
                src,
                (*dctx).expected,
            );
            result = ZSTDv07_decodeFrameHeader(
                dctx,
                (*dctx).headerBuffer.as_ptr() as *const c_void,
                (*dctx).headerSize,
            );
            if ZSTDv07_isError(result) != 0 {
                return result;
            }
            (*dctx).expected = ZSTDv07_blockHeaderSize;
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            0
        }
        ZSTDds_decodeBlockHeader => {
            let mut bp = blockProperties_t {
                blockType: 0,
                origSize: 0,
            };
            let cBlockSize = ZSTDv07_getcBlockSize(src, ZSTDv07_blockHeaderSize, &mut bp);
            if ZSTDv07_isError(cBlockSize) != 0 {
                return cBlockSize;
            }
            if bp.blockType == bt_end {
                if (*dctx).fParams.checksumFlag != 0 {
                    let h64 = XXH64_digest(&(*dctx).xxhState);
                    let h32 = ((h64 >> 11) as u32) & ((1 << 22) - 1);
                    let ip = src as *const u8;
                    let check32 =
                        *ip.add(2) as u32 + ((*ip.add(1) as u32) << 8) + (((*ip.add(0) & 0x3F) as u32) << 16);
                    if check32 != h32 {
                        return ERROR(ec::CHECKSUM_WRONG);
                    }
                }
                (*dctx).expected = 0;
                (*dctx).stage = ZSTDds_getFrameHeaderSize;
            } else {
                (*dctx).expected = cBlockSize;
                (*dctx).bType = bp.blockType;
                (*dctx).stage = ZSTDds_decompressBlock;
            }
            0
        }
        ZSTDds_decompressBlock => {
            let rSize: usize;
            match (*dctx).bType {
                bt_compressed => {
                    rSize = ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
                }
                bt_raw => {
                    rSize = ZSTDv07_copyRawBlock(dst, dstCapacity, src, srcSize);
                }
                bt_rle => {
                    return ERROR(ec::GENERIC);
                }
                bt_end => {
                    rSize = 0;
                }
                _ => {
                    return ERROR(ec::GENERIC);
                }
            }
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            (*dctx).expected = ZSTDv07_blockHeaderSize;
            if ZSTDv07_isError(rSize) != 0 {
                return rSize;
            }
            (*dctx).previousDstEnd = (dst as *const u8).add(rSize) as *const c_void;
            if (*dctx).fParams.checksumFlag != 0 {
                XXH64_update(&mut (*dctx).xxhState, dst as *const c_void, rSize);
            }
            rSize
        }
        ZSTDds_decodeSkippableHeader => {
            memcpy(
                (*dctx).headerBuffer.as_mut_ptr().add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
                src,
                (*dctx).expected,
            );
            (*dctx).expected = MEM_readLE32((*dctx).headerBuffer.as_ptr().add(4)) as usize;
            (*dctx).stage = ZSTDds_skipFrame;
            0
        }
        ZSTDds_skipFrame => {
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            0
        }
        _ => ERROR(ec::GENERIC),
    }
}

unsafe fn ZSTDv07_refDictContent(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const u8).offset(
        -(((*dctx).previousDstEnd as isize) - ((*dctx).base as isize)),
    ) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const u8).add(dictSize) as *const c_void;
    0
}

unsafe fn ZSTDv07_loadEntropy(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut dictPtr = dict as *const u8;
    let dictEnd = dictPtr.add(dictSize);

    {
        let hSize = HUFv07_readDTableX4((*dctx).hufTable.as_mut_ptr(), dict, dictSize);
        if HUFv07_isError(hSize) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        dictPtr = dictPtr.add(hSize);
    }

    {
        let mut offcodeNCount = [0i16; MaxOff + 1];
        let mut offcodeMaxValue: u32 = MaxOff as u32;
        let mut offcodeLog: u32 = 0;
        let offcodeHeaderSize = FSEv07_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            dictEnd as usize - dictPtr as usize,
        );
        if FSEv07_isError(offcodeHeaderSize) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        {
            let errorCode = FSEv07_buildDTable(
                (*dctx).OffTable.as_mut_ptr(),
                offcodeNCount.as_ptr(),
                offcodeMaxValue,
                offcodeLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ec::DICTIONARY_CORRUPTED);
            }
        }
        dictPtr = dictPtr.add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount = [0i16; MaxML + 1];
        let mut matchlengthMaxValue: u32 = MaxML as u32;
        let mut matchlengthLog: u32 = 0;
        let matchlengthHeaderSize = FSEv07_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            dictEnd as usize - dictPtr as usize,
        );
        if FSEv07_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        {
            let errorCode = FSEv07_buildDTable(
                (*dctx).MLTable.as_mut_ptr(),
                matchlengthNCount.as_ptr(),
                matchlengthMaxValue,
                matchlengthLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ec::DICTIONARY_CORRUPTED);
            }
        }
        dictPtr = dictPtr.add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount = [0i16; MaxLL + 1];
        let mut litlengthMaxValue: u32 = MaxLL as u32;
        let mut litlengthLog: u32 = 0;
        let litlengthHeaderSize = FSEv07_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            dictEnd as usize - dictPtr as usize,
        );
        if FSEv07_isError(litlengthHeaderSize) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        {
            let errorCode = FSEv07_buildDTable(
                (*dctx).LLTable.as_mut_ptr(),
                litlengthNCount.as_ptr(),
                litlengthMaxValue,
                litlengthLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ec::DICTIONARY_CORRUPTED);
            }
        }
        dictPtr = dictPtr.add(litlengthHeaderSize);
    }

    if dictPtr.add(12) > dictEnd {
        return ERROR(ec::DICTIONARY_CORRUPTED);
    }
    (*dctx).rep[0] = MEM_readLE32(dictPtr.add(0));
    if (*dctx).rep[0] == 0 || (*dctx).rep[0] as usize >= dictSize {
        return ERROR(ec::DICTIONARY_CORRUPTED);
    }
    (*dctx).rep[1] = MEM_readLE32(dictPtr.add(4));
    if (*dctx).rep[1] == 0 || (*dctx).rep[1] as usize >= dictSize {
        return ERROR(ec::DICTIONARY_CORRUPTED);
    }
    (*dctx).rep[2] = MEM_readLE32(dictPtr.add(8));
    if (*dctx).rep[2] == 0 || (*dctx).rep[2] as usize >= dictSize {
        return ERROR(ec::DICTIONARY_CORRUPTED);
    }
    dictPtr = dictPtr.add(12);

    (*dctx).litEntropy = 1;
    (*dctx).fseEntropy = 1;
    dictPtr as usize - dict as usize
}

unsafe fn ZSTDv07_decompress_insertDictionary(
    dctx: *mut ZSTDv07_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    if dictSize < 8 {
        return ZSTDv07_refDictContent(dctx, dict, dictSize);
    }
    {
        let magic = MEM_readLE32(dict as *const u8);
        if magic != ZSTDv07_DICT_MAGIC {
            return ZSTDv07_refDictContent(dctx, dict, dictSize);
        }
    }
    (*dctx).dictID = MEM_readLE32((dict as *const u8).add(4));

    dict = (dict as *const u8).add(8) as *const c_void;
    dictSize -= 8;
    {
        let eSize = ZSTDv07_loadEntropy(dctx, dict, dictSize);
        if ZSTDv07_isError(eSize) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        dict = (dict as *const u8).add(eSize) as *const c_void;
        dictSize -= eSize;
    }

    ZSTDv07_refDictContent(dctx, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBegin_usingDict(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    {
        let errorCode = ZSTDv07_decompressBegin(dctx);
        if ZSTDv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    if !dict.is_null() && dictSize != 0 {
        let errorCode = ZSTDv07_decompress_insertDictionary(dctx, dict, dictSize);
        if ZSTDv07_isError(errorCode) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
    }

    0
}

#[repr(C)]
pub struct ZSTDv07_DDict {
    dict: *mut c_void,
    dictSize: usize,
    refContext: *mut ZSTDv07_DCtx,
}

unsafe fn ZSTDv07_createDDict_advanced(
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
        let ddict = (customMem.customAlloc.unwrap())(
            customMem.opaque,
            core::mem::size_of::<ZSTDv07_DDict>(),
        ) as *mut ZSTDv07_DDict;
        let dictContent = (customMem.customAlloc.unwrap())(customMem.opaque, dictSize);
        let dctx = ZSTDv07_createDCtx_advanced(customMem);

        if dictContent.is_null() || ddict.is_null() || dctx.is_null() {
            (customMem.customFree.unwrap())(customMem.opaque, dictContent);
            (customMem.customFree.unwrap())(customMem.opaque, ddict as *mut c_void);
            (customMem.customFree.unwrap())(customMem.opaque, dctx as *mut c_void);
            return core::ptr::null_mut();
        }

        memcpy(dictContent, dict, dictSize);
        {
            let errorCode = ZSTDv07_decompressBegin_usingDict(dctx, dictContent, dictSize);
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
        ddict
    }
}

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
    let cFree = (*(*ddict).refContext).customMem.customFree;
    let opaque = (*(*ddict).refContext).customMem.opaque;
    ZSTDv07_freeDCtx((*ddict).refContext);
    (cFree.unwrap())(opaque, (*ddict).dict);
    (cFree.unwrap())(opaque, ddict as *mut c_void);
    0
}

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

/* ****************************************************************
 * ZBUFF : Buffered streaming decompression
 * ************************************************************** */
#[inline]
fn ZBUFF_MIN(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}
#[inline]
fn ZBUFF_MAX(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

const ZBUFFds_init: u32 = 0;
const ZBUFFds_loadHeader: u32 = 1;
const ZBUFFds_read: u32 = 2;
const ZBUFFds_load: u32 = 3;
const ZBUFFds_flush: u32 = 4;
type ZBUFFv07_dStage = u32;

#[repr(C)]
pub struct ZBUFFv07_DCtx {
    zd: *mut ZSTDv07_DCtx,
    fParams: ZSTDv07_frameParams,
    stage: ZBUFFv07_dStage,
    inBuff: *mut c_char,
    inBuffSize: usize,
    inPos: usize,
    outBuff: *mut c_char,
    outBuffSize: usize,
    outStart: usize,
    outEnd: usize,
    blockSize: usize,
    headerBuffer: [u8; ZSTDv07_FRAMEHEADERSIZE_MAX],
    lhSize: usize,
    customMem: ZSTDv07_customMem,
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
    memset(zbd as *mut c_void, 0, core::mem::size_of::<ZBUFFv07_DCtx>());
    memcpy(
        &mut (*zbd).customMem as *mut ZSTDv07_customMem as *mut c_void,
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
        return 0;
    }
    ZSTDv07_freeDCtx((*zbd).zd);
    if !(*zbd).inBuff.is_null() {
        ((*zbd).customMem.customFree.unwrap())((*zbd).customMem.opaque, (*zbd).inBuff as *mut c_void);
    }
    if !(*zbd).outBuff.is_null() {
        ((*zbd).customMem.customFree.unwrap())((*zbd).customMem.opaque, (*zbd).outBuff as *mut c_void);
    }
    ((*zbd).customMem.customFree.unwrap())((*zbd).customMem.opaque, zbd as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInitDictionary(
    zbd: *mut ZBUFFv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*zbd).stage = ZBUFFds_loadHeader;
    (*zbd).lhSize = 0;
    (*zbd).inPos = 0;
    (*zbd).outStart = 0;
    (*zbd).outEnd = 0;
    ZSTDv07_decompressBegin_usingDict((*zbd).zd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInit(zbd: *mut ZBUFFv07_DCtx) -> usize {
    ZBUFFv07_decompressInitDictionary(zbd, core::ptr::null(), 0)
}

#[inline]
unsafe fn ZBUFFv07_limitCopy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length = ZBUFF_MIN(dstCapacity, srcSize);
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressContinue(
    zbd: *mut ZBUFFv07_DCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart = src as *const c_char;
    let iend = istart.add(*srcSizePtr);
    let mut ip = istart;
    let ostart = dst as *mut c_char;
    let oend = ostart.add(*dstCapacityPtr);
    let mut op = ostart;
    let mut notDone: u32 = 1;

    // The C source uses a `switch` inside `while(notDone)` with fall-through
    // between the loadHeader -> read -> load -> flush stages. This is
    // reproduced here with an if-chain: a C `break` (out of the switch) maps
    // to `continue 'outer`, and C fall-through maps to letting control flow
    // into the next `if` in the same iteration.
    'outer: while notDone != 0 {
        if (*zbd).stage == ZBUFFds_init {
            return ERROR(ec::INIT_MISSING);
        }

        if (*zbd).stage == ZBUFFds_loadHeader {
            let mut fellThrough = false;
            {
                let hSize = ZSTDv07_getFrameParams(
                    &mut (*zbd).fParams,
                    (*zbd).headerBuffer.as_ptr() as *const c_void,
                    (*zbd).lhSize,
                );
                if ZSTDv07_isError(hSize) != 0 {
                    return hSize;
                }
                if hSize != 0 {
                    let toLoad = hSize - (*zbd).lhSize;
                    if toLoad > (iend as usize - ip as usize) {
                        if !ip.is_null() {
                            memcpy(
                                (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize) as *mut c_void,
                                ip as *const c_void,
                                iend as usize - ip as usize,
                            );
                        }
                        (*zbd).lhSize += iend as usize - ip as usize;
                        *dstCapacityPtr = 0;
                        return (hSize - (*zbd).lhSize) + ZSTDv07_blockHeaderSize;
                    }
                    memcpy(
                        (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize) as *mut c_void,
                        ip as *const c_void,
                        toLoad,
                    );
                    (*zbd).lhSize = hSize;
                    ip = ip.add(toLoad);
                    continue 'outer;
                }
            }

            {
                let h1Size = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                let h1Result = ZSTDv07_decompressContinue(
                    (*zbd).zd,
                    core::ptr::null_mut(),
                    0,
                    (*zbd).headerBuffer.as_ptr() as *const c_void,
                    h1Size,
                );
                if ZSTDv07_isError(h1Result) != 0 {
                    return h1Result;
                }
                if h1Size < (*zbd).lhSize {
                    let h2Size = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                    let h2Result = ZSTDv07_decompressContinue(
                        (*zbd).zd,
                        core::ptr::null_mut(),
                        0,
                        (*zbd).headerBuffer.as_ptr().add(h1Size) as *const c_void,
                        h2Size,
                    );
                    if ZSTDv07_isError(h2Result) != 0 {
                        return h2Result;
                    }
                }
            }

            (*zbd).fParams.windowSize = ZBUFF_MAX(
                (*zbd).fParams.windowSize as usize,
                1usize << ZSTDv07_WINDOWLOG_ABSOLUTEMIN,
            ) as c_uint;

            {
                let blockSize =
                    ZBUFF_MIN((*zbd).fParams.windowSize as usize, ZSTDv07_BLOCKSIZE_ABSOLUTEMAX);
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
                        return ERROR(ec::MEMORY_ALLOCATION);
                    }
                }
                {
                    let neededOutSize =
                        (*zbd).fParams.windowSize as usize + blockSize + WILDCOPY_OVERLENGTH * 2;
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
                            return ERROR(ec::MEMORY_ALLOCATION);
                        }
                    }
                }
            }
            (*zbd).stage = ZBUFFds_read;
            fellThrough = true;
            let _ = fellThrough;
            // fall-through to read
        }

        if (*zbd).stage == ZBUFFds_read {
            {
                let neededInSize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                if neededInSize == 0 {
                    (*zbd).stage = ZBUFFds_init;
                    notDone = 0;
                    continue 'outer;
                }
                if (iend as usize - ip as usize) >= neededInSize {
                    let isSkipFrame = ZSTDv07_isSkipFrame((*zbd).zd);
                    let decodedSize = ZSTDv07_decompressContinue(
                        (*zbd).zd,
                        (*zbd).outBuff.add((*zbd).outStart) as *mut c_void,
                        if isSkipFrame != 0 {
                            0
                        } else {
                            (*zbd).outBuffSize - (*zbd).outStart
                        },
                        ip as *const c_void,
                        neededInSize,
                    );
                    if ZSTDv07_isError(decodedSize) != 0 {
                        return decodedSize;
                    }
                    ip = ip.add(neededInSize);
                    if decodedSize == 0 && isSkipFrame == 0 {
                        continue 'outer;
                    }
                    (*zbd).outEnd = (*zbd).outStart + decodedSize;
                    (*zbd).stage = ZBUFFds_flush;
                    continue 'outer;
                }
                if ip == iend {
                    notDone = 0;
                    continue 'outer;
                }
                (*zbd).stage = ZBUFFds_load;
            }
            // fall-through to load
        }

        if (*zbd).stage == ZBUFFds_load {
            {
                let neededInSize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                let toLoad = neededInSize - (*zbd).inPos;
                let loadedSize: usize;
                if toLoad > (*zbd).inBuffSize - (*zbd).inPos {
                    return ERROR(ec::CORRUPTION_DETECTED);
                }
                loadedSize = ZBUFFv07_limitCopy(
                    (*zbd).inBuff.add((*zbd).inPos) as *mut c_void,
                    toLoad,
                    ip as *const c_void,
                    iend as usize - ip as usize,
                );
                ip = ip.add(loadedSize);
                (*zbd).inPos += loadedSize;
                if loadedSize < toLoad {
                    notDone = 0;
                    continue 'outer;
                }

                {
                    let isSkipFrame = ZSTDv07_isSkipFrame((*zbd).zd);
                    let decodedSize = ZSTDv07_decompressContinue(
                        (*zbd).zd,
                        (*zbd).outBuff.add((*zbd).outStart) as *mut c_void,
                        (*zbd).outBuffSize - (*zbd).outStart,
                        (*zbd).inBuff as *const c_void,
                        neededInSize,
                    );
                    if ZSTDv07_isError(decodedSize) != 0 {
                        return decodedSize;
                    }
                    (*zbd).inPos = 0;
                    if decodedSize == 0 && isSkipFrame == 0 {
                        (*zbd).stage = ZBUFFds_read;
                        continue 'outer;
                    }
                    (*zbd).outEnd = (*zbd).outStart + decodedSize;
                    (*zbd).stage = ZBUFFds_flush;
                    // fall-through to flush
                }
            }
        }

        if (*zbd).stage == ZBUFFds_flush {
            {
                let toFlushSize = (*zbd).outEnd - (*zbd).outStart;
                let flushedSize = ZBUFFv07_limitCopy(
                    op as *mut c_void,
                    oend as usize - op as usize,
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
                    continue 'outer;
                }
                notDone = 0;
                continue 'outer;
            }
        }

        return ERROR(ec::GENERIC);
    }

    *srcSizePtr = ip as usize - istart as usize;
    *dstCapacityPtr = op as usize - ostart as usize;
    {
        let mut nextSrcSizeHint = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
        nextSrcSizeHint -= (*zbd).inPos;
        nextSrcSizeHint
    }
}

/* Tool functions */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_recommendedDInSize() -> usize {
    ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + ZSTDv07_blockHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_recommendedDOutSize() -> usize {
    ZSTDv07_BLOCKSIZE_ABSOLUTEMAX
}
