//! Translation of legacy/zstd_v06.c — self-contained zstd v0.6 decoder + ZBUFF,
//! with its own internal FSEv06/HUFv06 decoders.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code, unused_mut, unused_assignments, unused_parens)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use crate::common::allocations::{free, malloc, memcpy, memmove, memset};
use crate::common::error::{code as ec, err_get_error_name, err_is_error, error as zerror};

type BYTE = u8;
type U16 = u16;
type S16 = i16;
type U32 = u32;
type S32 = i32;
type U64 = u64;
type S64 = i64;

#[inline]
fn ERROR(c: i32) -> usize {
    zerror(c)
}

#[inline]
fn ERR_isError(code: usize) -> c_uint {
    err_is_error(code)
}

/* ======  mem.h  ====== */

#[inline]
fn MEM_32bits() -> u32 {
    0
}
#[inline]
fn MEM_64bits() -> u32 {
    1
}

#[inline]
unsafe fn MEM_read16(memPtr: *const c_void) -> U16 {
    (memPtr as *const U16).read_unaligned()
}
#[inline]
unsafe fn MEM_read32(memPtr: *const c_void) -> U32 {
    (memPtr as *const U32).read_unaligned()
}
#[inline]
unsafe fn MEM_read64(memPtr: *const c_void) -> U64 {
    (memPtr as *const U64).read_unaligned()
}
#[inline]
unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    (memPtr as *mut U16).write_unaligned(value);
}
#[inline]
unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    MEM_read16(memPtr)
}
#[inline]
unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    MEM_write16(memPtr, val);
}
#[inline]
unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    MEM_read32(memPtr)
}
#[inline]
unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    MEM_read64(memPtr)
}
#[inline]
unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    MEM_readLE64(memPtr) as usize
}

/* ======  zstd_internal common constants  ====== */

const ZSTDv06_DICT_MAGIC: u32 = 0xEC30A436;
const ZSTDv06_REP_NUM: usize = 3;
const ZSTDv06_REP_INIT: usize = ZSTDv06_REP_NUM;
const ZSTDv06_REP_MOVE: usize = ZSTDv06_REP_NUM - 1;

const ZSTDv06_WINDOWLOG_ABSOLUTEMIN: u32 = 12;
static ZSTDv06_fcs_fieldSize: [usize; 4] = [0, 1, 2, 8];

const ZSTDv06_BLOCKHEADERSIZE: usize = 3;
const ZSTDv06_blockHeaderSize: usize = ZSTDv06_BLOCKHEADERSIZE;

const bt_compressed: u32 = 0;
const bt_raw: u32 = 1;
const bt_rle: u32 = 2;
const bt_end: u32 = 3;

const MIN_SEQUENCES_SIZE: usize = 1;
const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

const IS_HUF: u32 = 0;
const IS_PCH: u32 = 1;
const IS_RAW: u32 = 2;
const IS_RLE: u32 = 3;

const LONGNBSEQ: i32 = 0x7F00;

const MINMATCH: usize = 3;
const REPCODE_STARTVALUE: usize = 1;

const MaxLL: usize = 35;
const MaxML: usize = 52;
const MaxOff: usize = 28;
const MaxSeq: usize = if MaxLL > MaxML { MaxLL } else { MaxML };
const MLFSELog: u32 = 9;
const LLFSELog: u32 = 9;
const OffFSELog: u32 = 8;

const FSEv06_ENCODING_RAW: u32 = 0;
const FSEv06_ENCODING_RLE: u32 = 1;
const FSEv06_ENCODING_STATIC: u32 = 2;
const FSEv06_ENCODING_DYNAMIC: u32 = 3;

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

const WILDCOPY_OVERLENGTH: usize = 8;

#[inline]
unsafe fn ZSTDv06_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

#[inline]
unsafe fn ZSTDv06_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.offset(length);
    loop {
        ZSTDv06_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if !(op < oend) {
            break;
        }
    }
}

/* ======  static ZSTD constants  ====== */
const ZSTDv06_FRAMEHEADERSIZE_MAX: usize = 13;
const ZSTDv06_frameHeaderSize_min: usize = 5;
const ZSTDv06_frameHeaderSize_max: usize = ZSTDv06_FRAMEHEADERSIZE_MAX;
const ZSTDv06_BLOCKSIZE_MAX: usize = 128 * 1024;
const ZSTDv06_MAGICNUMBER: u32 = 0xFD2FB526;

/* ======  bitstream.h  ====== */

#[repr(C)]
struct BITv06_DStream_t {
    bitContainer: usize,
    bitsConsumed: c_uint,
    ptr: *const c_char,
    start: *const c_char,
}

const BITv06_DStream_unfinished: u32 = 0;
const BITv06_DStream_endOfBuffer: u32 = 1;
const BITv06_DStream_completed: u32 = 2;
const BITv06_DStream_overflow: u32 = 3;
type BITv06_DStream_status = u32;

#[inline]
fn BITv06_highbit32(val: U32) -> c_uint {
    31 ^ val.leading_zeros()
}

unsafe fn BITv06_initDStream(
    bitD: *mut BITv06_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BITv06_DStream_t>());
        return ERROR(ec::SRCSIZE_WRONG);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char)
            .add(srcSize)
            .sub(core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        let lastByte = *(srcBuffer as *const BYTE).add(srcSize - 1);
        if lastByte == 0 {
            return ERROR(ec::GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BITv06_highbit32(lastByte as U32);
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let sb = srcBuffer as *const BYTE;
        match srcSize {
            7 => {
                (*bitD).bitContainer +=
                    (*sb.add(6) as usize) << (core::mem::size_of::<usize>() * 8 - 16);
                (*bitD).bitContainer +=
                    (*sb.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24);
                (*bitD).bitContainer +=
                    (*sb.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
                (*bitD).bitContainer += (*sb.add(3) as usize) << 24;
                (*bitD).bitContainer += (*sb.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sb.add(1) as usize) << 8;
            }
            6 => {
                (*bitD).bitContainer +=
                    (*sb.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24);
                (*bitD).bitContainer +=
                    (*sb.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
                (*bitD).bitContainer += (*sb.add(3) as usize) << 24;
                (*bitD).bitContainer += (*sb.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sb.add(1) as usize) << 8;
            }
            5 => {
                (*bitD).bitContainer +=
                    (*sb.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
                (*bitD).bitContainer += (*sb.add(3) as usize) << 24;
                (*bitD).bitContainer += (*sb.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sb.add(1) as usize) << 8;
            }
            4 => {
                (*bitD).bitContainer += (*sb.add(3) as usize) << 24;
                (*bitD).bitContainer += (*sb.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sb.add(1) as usize) << 8;
            }
            3 => {
                (*bitD).bitContainer += (*sb.add(2) as usize) << 16;
                (*bitD).bitContainer += (*sb.add(1) as usize) << 8;
            }
            2 => {
                (*bitD).bitContainer += (*sb.add(1) as usize) << 8;
            }
            _ => {}
        }
        let lastByte = *(srcBuffer as *const BYTE).add(srcSize - 1);
        if lastByte == 0 {
            return ERROR(ec::GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BITv06_highbit32(lastByte as U32);
        (*bitD).bitsConsumed +=
            ((core::mem::size_of::<usize>() - srcSize) * 8) as U32;
    }

    srcSize
}

#[inline]
unsafe fn BITv06_lookBits(bitD: *const BITv06_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask - nbBits) & bitMask)
}

#[inline]
unsafe fn BITv06_lookBitsFast(bitD: *const BITv06_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1) - nbBits) & bitMask)
}

#[inline]
unsafe fn BITv06_skipBits(bitD: *mut BITv06_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
unsafe fn BITv06_readBits(bitD: *mut BITv06_DStream_t, nbBits: U32) -> usize {
    let value = BITv06_lookBits(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

#[inline]
unsafe fn BITv06_readBitsFast(bitD: *mut BITv06_DStream_t, nbBits: U32) -> usize {
    let value = BITv06_lookBitsFast(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

unsafe fn BITv06_reloadDStream(bitD: *mut BITv06_DStream_t) -> BITv06_DStream_status {
    if (*bitD).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
        return BITv06_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD).ptr.offset(-(((*bitD).bitsConsumed >> 3) as isize));
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
        if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32;
            result = BITv06_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline]
unsafe fn BITv06_endOfDStream(DStream: *const BITv06_DStream_t) -> c_uint {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed as usize == core::mem::size_of::<usize>() * 8))
        as c_uint
}

/* ======  FSE types  ====== */

type FSEv06_DTable = c_uint;

#[repr(C)]
#[derive(Clone, Copy)]
struct FSEv06_DTableHeader {
    tableLog: U16,
    fastMode: U16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSEv06_decode_t {
    newState: u16,
    symbol: u8,
    nbBits: u8,
}

#[repr(C)]
struct FSEv06_DState_t {
    state: usize,
    table: *const c_void,
}

const FSEv06_MAX_SYMBOL_VALUE: usize = 255;
const FSEv06_MAX_MEMORY_USAGE: u32 = 14;
const FSEv06_DEFAULT_MEMORY_USAGE: u32 = 13;
const FSEv06_MAX_TABLELOG: u32 = FSEv06_MAX_MEMORY_USAGE - 2;
const FSEv06_MAX_TABLESIZE: u32 = 1u32 << FSEv06_MAX_TABLELOG;
const FSEv06_MAXTABLESIZE_MASK: u32 = FSEv06_MAX_TABLESIZE - 1;
const FSEv06_DEFAULT_TABLELOG: u32 = FSEv06_DEFAULT_MEMORY_USAGE - 2;
const FSEv06_MIN_TABLELOG: i32 = 5;
const FSEv06_TABLELOG_ABSOLUTE_MAX: i32 = 15;

#[inline]
fn FSEv06_TABLESTEP(tableSize: u32) -> u32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[inline]
fn FSEv06_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

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

#[inline]
unsafe fn FSEv06_peekSymbol(DStatePtr: *const FSEv06_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

#[inline]
unsafe fn FSEv06_updateState(DStatePtr: *mut FSEv06_DState_t, bitD: *mut BITv06_DStream_t) {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let lowBits = BITv06_readBits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
}

#[inline]
unsafe fn FSEv06_decodeSymbol(DStatePtr: *mut FSEv06_DState_t, bitD: *mut BITv06_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BITv06_readBits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
unsafe fn FSEv06_decodeSymbolFast(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BITv06_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

/* ======  FSE + HUF Error Management  ====== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_getErrorName(code: usize) -> *const c_char {
    err_get_error_name(code)
}

#[inline]
fn HUFv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/* ======  FSE NCount decoding  ====== */

#[inline]
fn FSEv06_abs(a: S16) -> S16 {
    if a < 0 {
        -a
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
        return ERROR(ec::SRCSIZE_WRONG);
    }
    bitStream = MEM_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) as c_int) + FSEv06_MIN_TABLELOG;
    if nbBits > FSEv06_TABLELOG_ABSOLUTE_MAX {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1 << nbBits) + 1;
    threshold = 1 << nbBits;
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0 = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 += 24;
                if ip < iend.sub(5) {
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
                return ERROR(ec::MAXSYMBOLVALUE_TOOSMALL);
            }
            while charnum < n0 {
                *normalizedCounter.add(charnum as usize) = 0;
                charnum += 1;
            }
            if (ip <= iend.sub(7))
                || (ip.offset((bitCount >> 3) as isize) <= iend.sub(4))
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
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
                if count as c_int >= threshold {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1;
            remaining -= FSEv06_abs(count) as c_int;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as c_int;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            if (ip <= iend.sub(7))
                || (ip.offset((bitCount >> 3) as isize) <= iend.sub(4))
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend.offset_from(ip) - 4)) as c_int;
                ip = iend.sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
        }
    }
    if remaining != 1 {
        return ERROR(ec::GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    if (ip.offset_from(istart) as usize) > hbSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip.offset_from(istart) as usize
}

/* ======  FSE DTable build / decompress  ====== */

const DTABLE_MAX_SIZE: usize = FSEv06_DTABLE_SIZE_U32_CONST;
const FSEv06_DTABLE_SIZE_U32_CONST: usize = (1 + (1u32 << FSEv06_MAX_TABLELOG)) as usize;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_createDTable(mut tableLog: c_uint) -> *mut FSEv06_DTable {
    if tableLog > FSEv06_TABLELOG_ABSOLUTE_MAX as c_uint {
        tableLog = FSEv06_TABLELOG_ABSOLUTE_MAX as c_uint;
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
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSEv06_decode_t;
    let mut symbolNext: [U16; FSEv06_MAX_SYMBOL_VALUE + 1] = [0; FSEv06_MAX_SYMBOL_VALUE + 1];

    let maxSV1: U32 = maxSymbolValue + 1;
    let tableSize: U32 = 1 << tableLog;
    let mut highThreshold: U32 = tableSize - 1;

    if maxSymbolValue > FSEv06_MAX_SYMBOL_VALUE as c_uint {
        return ERROR(ec::MAXSYMBOLVALUE_TOOLARGE);
    }
    if tableLog > FSEv06_MAX_TABLELOG {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }

    {
        let mut DTableH = FSEv06_DTableHeader {
            tableLog: tableLog as U16,
            fastMode: 1,
        };
        {
            let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let nc = *normalizedCounter.add(s as usize);
                if nc == -1 {
                    (*tableDecode.add(highThreshold as usize)).symbol = s as BYTE;
                    highThreshold -= 1;
                    symbolNext[s as usize] = 1;
                } else {
                    if nc >= largeLimit {
                        DTableH.fastMode = 0;
                    }
                    symbolNext[s as usize] = nc as U16;
                }
                s += 1;
            }
        }
        memcpy(
            dt as *mut c_void,
            &DTableH as *const FSEv06_DTableHeader as *const c_void,
            core::mem::size_of::<FSEv06_DTableHeader>(),
        );
    }

    {
        let tableMask: U32 = tableSize - 1;
        let step: U32 = FSEv06_TABLESTEP(tableSize);
        let mut s: U32 = 0;
        let mut position: U32 = 0;
        while s < maxSV1 {
            let mut i: c_int = 0;
            let nc = *normalizedCounter.add(s as usize) as c_int;
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
            return ERROR(ec::GENERIC);
        }
    }

    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol = (*tableDecode.add(u as usize)).symbol;
            let nextState = symbolNext[symbol as usize];
            symbolNext[symbol as usize] += 1;
            (*tableDecode.add(u as usize)).nbBits =
                (tableLog - BITv06_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(u as usize)).newState = (((nextState as U32)
                << (*tableDecode.add(u as usize)).nbBits as U32)
                .wrapping_sub(tableSize)) as U16;
            u += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_buildDTable_rle(dt: *mut FSEv06_DTable, symbolValue: u8) -> usize {
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
pub unsafe extern "C" fn FSEv06_buildDTable_raw(dt: *mut FSEv06_DTable, nbBits: c_uint) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv06_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv06_decode_t;
    let tableSize: c_uint = 1 << nbBits;
    let tableMask: c_uint = tableSize - 1;
    let maxSV1: c_uint = tableMask + 1;

    if nbBits < 1 {
        return ERROR(ec::GENERIC);
    }

    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    let mut s: c_uint = 0;
    while s < maxSV1 {
        (*dinfo.add(s as usize)).newState = 0;
        (*dinfo.add(s as usize)).symbol = s as BYTE;
        (*dinfo.add(s as usize)).nbBits = nbBits as BYTE;
        s += 1;
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
    let omax = op.add(maxDstSize);
    let olimit = omax.sub(3);

    let mut bitD: BITv06_DStream_t = core::mem::zeroed();
    let mut state1: FSEv06_DState_t = core::mem::zeroed();
    let mut state2: FSEv06_DState_t = core::mem::zeroed();

    {
        let errorCode = BITv06_initDStream(&mut bitD, cSrc, cSrcSize);
        if FSEv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv06_initDState(&mut state1, &mut bitD, dt);
    FSEv06_initDState(&mut state2, &mut bitD, dt);

    macro_rules! GETSYMBOL {
        ($sp:expr) => {
            if fast != 0 {
                FSEv06_decodeSymbolFast($sp, &mut bitD)
            } else {
                FSEv06_decodeSymbol($sp, &mut bitD)
            }
        };
    }

    while (BITv06_reloadDStream(&mut bitD) == BITv06_DStream_unfinished) && (op < olimit) {
        *op.add(0) = GETSYMBOL!(&mut state1);
        if FSEv06_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            BITv06_reloadDStream(&mut bitD);
        }
        *op.add(1) = GETSYMBOL!(&mut state2);
        if FSEv06_MAX_TABLELOG * 4 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            if BITv06_reloadDStream(&mut bitD) > BITv06_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }
        *op.add(2) = GETSYMBOL!(&mut state1);
        if FSEv06_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            BITv06_reloadDStream(&mut bitD);
        }
        *op.add(3) = GETSYMBOL!(&mut state2);
        op = op.add(4);
    }

    loop {
        if op > omax.sub(2) {
            return ERROR(ec::DSTSIZE_TOOSMALL);
        }
        *op = GETSYMBOL!(&mut state1);
        op = op.add(1);
        if BITv06_reloadDStream(&mut bitD) == BITv06_DStream_overflow {
            *op = GETSYMBOL!(&mut state2);
            op = op.add(1);
            break;
        }
        if op > omax.sub(2) {
            return ERROR(ec::DSTSIZE_TOOSMALL);
        }
        *op = GETSYMBOL!(&mut state2);
        op = op.add(1);
        if BITv06_reloadDStream(&mut bitD) == BITv06_DStream_overflow {
            *op = GETSYMBOL!(&mut state1);
            op = op.add(1);
            break;
        }
    }

    op.offset_from(ostart) as usize
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
    let fastMode = (*DTableH).fastMode as U32;

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
    let mut counting: [i16; FSEv06_MAX_SYMBOL_VALUE + 1] = [0; FSEv06_MAX_SYMBOL_VALUE + 1];
    let mut dt: [U32; DTABLE_MAX_SIZE] = [0; DTABLE_MAX_SIZE];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv06_MAX_SYMBOL_VALUE as c_uint;

    if cSrcSize < 2 {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    {
        let NCountLength = FSEv06_readNCount(
            counting.as_mut_ptr(),
            &mut maxSymbolValue,
            &mut tableLog,
            istart as *const c_void,
            cSrcSize,
        );
        if FSEv06_isError(NCountLength) != 0 {
            return NCountLength;
        }
        if NCountLength >= cSrcSize {
            return ERROR(ec::SRCSIZE_WRONG);
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
        if FSEv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv06_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

/* ======  HUF constants and readStats  ====== */

const HUFv06_ABSOLUTEMAX_TABLELOG: usize = 16;
const HUFv06_MAX_TABLELOG: u32 = 12;
const HUFv06_DEFAULT_TABLELOG: u32 = HUFv06_MAX_TABLELOG;
const HUFv06_MAX_SYMBOL_VALUE: usize = 255;

#[inline]
const fn HUFv06_DTABLE_SIZE(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

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
    let mut oSize: usize;

    if srcSize == 0 {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        if iSize >= 242 {
            static l: [U32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
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
        if iSize + 1 > srcSize {
            return ERROR(ec::SRCSIZE_WRONG);
        }
        oSize = FSEv06_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.add(1) as *const c_void,
            iSize,
        );
        if FSEv06_isError(oSize) != 0 {
            return oSize;
        }
    }

    memset(
        rankStats as *mut c_void,
        0,
        (HUFv06_ABSOLUTEMAX_TABLELOG + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    {
        let mut n: U32 = 0;
        while (n as usize) < oSize {
            let w = *huffWeight.add(n as usize);
            if w as usize >= HUFv06_ABSOLUTEMAX_TABLELOG {
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
        let tableLog: U32 = BITv06_highbit32(weightTotal) + 1;
        if tableLog as usize > HUFv06_ABSOLUTEMAX_TABLELOG {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        *tableLogPtr = tableLog;
        {
            let total: U32 = 1 << tableLog;
            let rest: U32 = total - weightTotal;
            let verif: U32 = 1 << BITv06_highbit32(rest);
            let lastWeight: U32 = BITv06_highbit32(rest) + 1;
            if verif != rest {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            *huffWeight.add(oSize) = lastWeight as BYTE;
            *rankStats.add(lastWeight as usize) += 1;
        }
    }

    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

/* ======  HUF X2 : single-symbol decoding  ====== */

#[repr(C)]
#[derive(Clone, Copy)]
struct HUFv06_DEltX2 {
    byte: BYTE,
    nbBits: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HUFv06_DEltX4 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_readDTableX2(
    DTable: *mut u16,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut huffWeight: [BYTE; HUFv06_MAX_SYMBOL_VALUE + 1] = [0; HUFv06_MAX_SYMBOL_VALUE + 1];
    let mut rankVal: [U32; HUFv06_ABSOLUTEMAX_TABLELOG + 1] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG + 1];
    let mut tableLog: U32 = 0;
    let iSize: usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.add(1) as *mut c_void;
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

    if tableLog > *DTable.add(0) as U32 {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }
    *DTable.add(0) = tableLog as u16;

    nextRankStart = 0;
    n = 1;
    while n < tableLog + 1 {
        let current = nextRankStart;
        nextRankStart += rankVal[n as usize] << (n - 1);
        rankVal[n as usize] = current;
        n += 1;
    }

    n = 0;
    while n < nbSymbols {
        let w = huffWeight[n as usize] as U32;
        let length = (1u32 << w) >> 1;
        let mut i: U32;
        let mut D = HUFv06_DEltX2 { byte: 0, nbBits: 0 };
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
unsafe fn HUFv06_decodeSymbolX2(
    Dstream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BITv06_lookBitsFast(Dstream, dtLog);
    let c = (*dt.add(val)).byte;
    BITv06_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

unsafe fn HUFv06_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;

    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished) && (p <= pEnd.sub(4)) {
        // SYMBOLX2_2 (64bits)
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        // SYMBOLX2_1 (64bits || tablelog<=12)
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        // SYMBOLX2_2
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        // SYMBOLX2_0
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished) && (p < pEnd) {
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    while p < pEnd {
        *p = HUFv06_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    pEnd.offset_from(pStart) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const u16,
) -> usize {
    let op = dst as *mut BYTE;
    let oend = op.add(dstSize);
    let dtLog = *DTable.add(0) as U32;
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv06_DEltX2).add(1);
    let mut bitD: BITv06_DStream_t = core::mem::zeroed();

    {
        let errorCode = BITv06_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv06_decodeStreamX2(op, &mut bitD, oend, dt, dtLog);

    if BITv06_endOfDStream(&bitD) == 0 {
        return ERROR(ec::CORRUPTION_DETECTED);
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
    let mut DTable: [u16; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG as u16;
    let mut ip = cSrc as *const BYTE;

    let errorCode = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUFv06_decompress1X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const u16,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv06_DEltX2).add(1);
        let dtLog = *DTable.add(0) as U32;
        let mut errorCode: usize;

        let mut bitD1: BITv06_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv06_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv06_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv06_DStream_t = core::mem::zeroed();
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

        length4 = cSrcSize - (length1 + length2 + length3 + 6);
        if length4 > cSrcSize {
            return ERROR(ec::CORRUPTION_DETECTED);
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

        endSignal = BITv06_reloadDStream(&mut bitD1)
            | BITv06_reloadDStream(&mut bitD2)
            | BITv06_reloadDStream(&mut bitD3)
            | BITv06_reloadDStream(&mut bitD4);
        while (endSignal == BITv06_DStream_unfinished) && (op4 < oend.sub(7)) {
            macro_rules! X2_2 {
                ($op:expr, $bd:expr) => {{
                    *$op = HUFv06_decodeSymbolX2($bd, dt, dtLog);
                    $op = $op.add(1);
                }};
            }
            X2_2!(op1, &mut bitD1);
            X2_2!(op2, &mut bitD2);
            X2_2!(op3, &mut bitD3);
            X2_2!(op4, &mut bitD4);
            X2_2!(op1, &mut bitD1);
            X2_2!(op2, &mut bitD2);
            X2_2!(op3, &mut bitD3);
            X2_2!(op4, &mut bitD4);
            X2_2!(op1, &mut bitD1);
            X2_2!(op2, &mut bitD2);
            X2_2!(op3, &mut bitD3);
            X2_2!(op4, &mut bitD4);
            X2_2!(op1, &mut bitD1);
            X2_2!(op2, &mut bitD2);
            X2_2!(op3, &mut bitD3);
            X2_2!(op4, &mut bitD4);
            endSignal = BITv06_reloadDStream(&mut bitD1)
                | BITv06_reloadDStream(&mut bitD2)
                | BITv06_reloadDStream(&mut bitD3)
                | BITv06_reloadDStream(&mut bitD4);
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

        HUFv06_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv06_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv06_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv06_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        endSignal = BITv06_endOfDStream(&bitD1)
            & BITv06_endOfDStream(&bitD2)
            & BITv06_endOfDStream(&bitD3)
            & BITv06_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }

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
    let mut DTable: [u16; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG as u16;
    let mut ip = cSrc as *const BYTE;

    let errorCode = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUFv06_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/* ======  HUF X4 : double-symbol decoding  ====== */

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
    let mut rankVal: [U32; HUFv06_ABSOLUTEMAX_TABLELOG + 1] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG + 1];

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv06_ABSOLUTEMAX_TABLELOG + 1]>(),
    );

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

    {
        let mut s: U32 = 0;
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
}

type rankVal_t = [[U32; HUFv06_ABSOLUTEMAX_TABLELOG + 1]; HUFv06_ABSOLUTEMAX_TABLELOG];

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
    let mut rankVal: [U32; HUFv06_ABSOLUTEMAX_TABLELOG + 1] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG + 1];
    let scaleLog: c_int = (nbBitsBaseline - targetLog) as c_int;
    let minBits: U32 = nbBitsBaseline - maxWeight;
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        (*rankValOrigin).as_ptr() as *const c_void,
        core::mem::size_of::<[U32; HUFv06_ABSOLUTEMAX_TABLELOG + 1]>(),
    );

    s = 0;
    while s < sortedListSize {
        let symbol = (*sortedList.add(s as usize)).symbol as U16;
        let weight = (*sortedList.add(s as usize)).weight as U32;
        let nbBits = nbBitsBaseline - weight;
        let start = rankVal[weight as usize];
        let length = 1u32 << (targetLog - nbBits);

        if (targetLog - nbBits) >= minBits {
            let sortedRank: U32;
            let mut minWeight: c_int = nbBits as c_int + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUFv06_fillDTableX4Level2(
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
                let end = start + length;
                u = start;
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
pub unsafe extern "C" fn HUFv06_readDTableX4(
    DTable: *mut c_uint,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightList: [BYTE; HUFv06_MAX_SYMBOL_VALUE + 1] = [0; HUFv06_MAX_SYMBOL_VALUE + 1];
    let mut sortedSymbol: [sortedSymbol_t; HUFv06_MAX_SYMBOL_VALUE + 1] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; HUFv06_MAX_SYMBOL_VALUE + 1];
    let mut rankStats: [U32; HUFv06_ABSOLUTEMAX_TABLELOG + 1] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG + 1];
    let mut rankStart0: [U32; HUFv06_ABSOLUTEMAX_TABLELOG + 2] =
        [0; HUFv06_ABSOLUTEMAX_TABLELOG + 2];
    let mut rankVal: rankVal_t =
        [[0; HUFv06_ABSOLUTEMAX_TABLELOG + 1]; HUFv06_ABSOLUTEMAX_TABLELOG];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog = *DTable.add(0);
    let iSize: usize;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUFv06_DEltX4).add(1);

    if memLog > HUFv06_ABSOLUTEMAX_TABLELOG as c_uint {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }

    // rankStart = rankStart0 + 1 ; emulate via index offset by using a helper closure over rankStart0.
    let rankStart = rankStart0.as_mut_ptr().add(1);

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

    if tableLog > memLog {
        return ERROR(ec::TABLELOG_TOOLARGE);
    }

    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW -= 1;
    }

    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w < maxW + 1 {
            let current = nextRankStart;
            nextRankStart += rankStats[w as usize];
            *rankStart.add(w as usize) = current;
            w += 1;
        }
        *rankStart.add(0) = nextRankStart;
        sizeOfSort = nextRankStart;
    }

    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w = weightList[s as usize] as U32;
            let r = *rankStart.add(w as usize);
            *rankStart.add(w as usize) += 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        *rankStart.add(0) = 0;
    }

    {
        // rankVal0 = rankVal[0]
        {
            let rescale: c_int = ((memLog - tableLog) as c_int) - 1;
            let mut nextRankVal: U32 = 0;
            let mut w: U32 = 1;
            while w < maxW + 1 {
                let current = nextRankVal;
                nextRankVal += rankStats[w as usize] << (w as c_int + rescale);
                rankVal[0][w as usize] = current;
                w += 1;
            }
        }
        {
            let minBits: U32 = tableLog + 1 - maxW;
            let mut consumed: U32 = minBits;
            while consumed < memLog - minBits + 1 {
                let mut w: U32 = 1;
                while w < maxW + 1 {
                    let v = rankVal[0][w as usize] >> consumed;
                    rankVal[consumed as usize][w as usize] = v;
                    w += 1;
                }
                consumed += 1;
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
        tableLog + 1,
    );

    iSize
}

#[inline]
unsafe fn HUFv06_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv06_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 2);
    BITv06_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

#[inline]
unsafe fn HUFv06_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv06_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BITv06_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if ((*DStream).bitsConsumed as usize) < (core::mem::size_of::<usize>() * 8) {
            BITv06_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed as usize > (core::mem::size_of::<usize>() * 8) {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as c_uint;
            }
        }
    }
    1
}

unsafe fn HUFv06_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;

    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished) && (p < pEnd.sub(7)) {
        // X4_2 (64bits)
        p = p.add(HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        // X4_1
        p = p.add(HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        // X4_2
        p = p.add(HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        // X4_0
        p = p.add(HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished) && (p <= pEnd.sub(2)) {
        p = p.add(HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    while p <= pEnd.sub(2) {
        p = p.add(HUFv06_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    if p < pEnd {
        p = p.add(HUFv06_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    p.offset_from(pStart) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const c_uint,
) -> usize {
    let istart = cSrc as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dstSize);

    let dtLog = *DTable.add(0);
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv06_DEltX4).add(1);

    let mut bitD: BITv06_DStream_t = core::mem::zeroed();
    {
        let errorCode = BITv06_initDStream(&mut bitD, istart as *const c_void, cSrcSize);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv06_decodeStreamX4(ostart, &mut bitD, oend, dt, dtLog);

    if BITv06_endOfDStream(&bitD) == 0 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X4(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let mut DTable: [c_uint; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUFv06_decompress1X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const c_uint,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv06_DEltX4).add(1);
        let dtLog = *DTable.add(0);
        let mut errorCode: usize;

        let mut bitD1: BITv06_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv06_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv06_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv06_DStream_t = core::mem::zeroed();
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

        length4 = cSrcSize - (length1 + length2 + length3 + 6);
        if length4 > cSrcSize {
            return ERROR(ec::CORRUPTION_DETECTED);
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

        endSignal = BITv06_reloadDStream(&mut bitD1)
            | BITv06_reloadDStream(&mut bitD2)
            | BITv06_reloadDStream(&mut bitD3)
            | BITv06_reloadDStream(&mut bitD4);
        while (endSignal == BITv06_DStream_unfinished) && (op4 < oend.sub(7)) {
            macro_rules! X4 {
                ($op:expr, $bd:expr) => {{
                    $op = $op.add(
                        HUFv06_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize,
                    );
                }};
            }
            X4!(op1, &mut bitD1);
            X4!(op2, &mut bitD2);
            X4!(op3, &mut bitD3);
            X4!(op4, &mut bitD4);
            X4!(op1, &mut bitD1);
            X4!(op2, &mut bitD2);
            X4!(op3, &mut bitD3);
            X4!(op4, &mut bitD4);
            X4!(op1, &mut bitD1);
            X4!(op2, &mut bitD2);
            X4!(op3, &mut bitD3);
            X4!(op4, &mut bitD4);
            X4!(op1, &mut bitD1);
            X4!(op2, &mut bitD2);
            X4!(op3, &mut bitD3);
            X4!(op4, &mut bitD4);
            endSignal = BITv06_reloadDStream(&mut bitD1)
                | BITv06_reloadDStream(&mut bitD2)
                | BITv06_reloadDStream(&mut bitD3)
                | BITv06_reloadDStream(&mut bitD4);
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

        HUFv06_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv06_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv06_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv06_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        endSignal = BITv06_endOfDStream(&bitD1)
            & BITv06_endOfDStream(&bitD2)
            & BITv06_endOfDStream(&bitD3)
            & BITv06_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }

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
    let mut DTable: [c_uint; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)] =
        [0; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUFv06_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

/* ======  HUF generic decompress selector  ====== */

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let decompress: [Option<
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize,
    >; 3] = [Some(HUFv06_decompress4X2), Some(HUFv06_decompress4X4), None];
    let mut Dtime: [U32; 3] = [0; 3];

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
        memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    }

    {
        let Q: U32 = (cSrcSize * 16 / dstSize) as U32;
        let D256: U32 = (dstSize >> 8) as U32;
        let mut n: U32 = 0;
        while n < 3 {
            Dtime[n as usize] = algoTime[Q as usize][n as usize].tableTime
                + (algoTime[Q as usize][n as usize].decode256Time * D256);
            n += 1;
        }
    }

    Dtime[1] += Dtime[1] >> 4;
    Dtime[2] += Dtime[2] >> 3;

    {
        let mut algoNb: U32 = 0;
        if Dtime[1] < Dtime[0] {
            algoNb = 1;
        }
        return (decompress[algoNb as usize].unwrap())(dst, dstSize, cSrc, cSrcSize);
    }
}

/* ======  ZSTD / ZBUFF Error Management  ====== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_getErrorName(code: usize) -> *const c_char {
    err_get_error_name(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_getErrorName(errorCode: usize) -> *const c_char {
    err_get_error_name(errorCode)
}

/* ======  ZSTD Decompression Context  ====== */

#[inline]
unsafe fn ZSTDv06_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

const ZSTDds_getFrameHeaderSize: u32 = 0;
const ZSTDds_decodeFrameHeader: u32 = 1;
const ZSTDds_decodeBlockHeader: u32 = 2;
const ZSTDds_decompressBlock: u32 = 3;
type ZSTDv06_dStage = u32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTDv06_frameParams {
    frameContentSize: u64,
    windowLog: c_uint,
}

#[repr(C)]
pub struct ZSTDv06_DCtx {
    LLTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32_LL],
    OffTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32_OFF],
    MLTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32_ML],
    hufTableX4: [c_uint; HUFv06_DTABLE_SIZE_CAP],
    previousDstEnd: *const c_void,
    base: *const c_void,
    vBase: *const c_void,
    dictEnd: *const c_void,
    expected: usize,
    headerSize: usize,
    fParams: ZSTDv06_frameParams,
    bType: u32, // blockType_t
    stage: ZSTDv06_dStage,
    flagRepeatTable: U32,
    litPtr: *const BYTE,
    litSize: usize,
    litBuffer: [BYTE; ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH],
    headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
}

const FSEv06_DTABLE_SIZE_U32_LL: usize = (1 + (1u32 << LLFSELog)) as usize;
const FSEv06_DTABLE_SIZE_U32_OFF: usize = (1 + (1u32 << OffFSELog)) as usize;
const FSEv06_DTABLE_SIZE_U32_ML: usize = (1 + (1u32 << MLFSELog)) as usize;
const HUFv06_DTABLE_SIZE_CAP: usize = (1 + (1u32 << ZSTD_HUFFDTABLE_CAPACITY_LOG)) as usize;

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
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_copyDCtx(dstDCtx: *mut ZSTDv06_DCtx, srcDCtx: *const ZSTDv06_DCtx) {
    memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv06_DCtx>()
            - (ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH + ZSTDv06_frameHeaderSize_max),
    );
}

/* ======  frame header  ====== */

unsafe fn ZSTDv06_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    if srcSize < ZSTDv06_frameHeaderSize_min {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    let fcsId: U32 = (*(src as *const BYTE).add(4) as U32) >> 6;
    ZSTDv06_frameHeaderSize_min + ZSTDv06_fcs_fieldSize[fcsId as usize]
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
        return ERROR(ec::PREFIX_UNKNOWN);
    }

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
        let frameDesc: BYTE = *ip.add(4);
        (*fparamsPtr).windowLog = (frameDesc as U32 & 0xF) + ZSTDv06_WINDOWLOG_ABSOLUTEMIN;
        if (frameDesc & 0x20) != 0 {
            return ERROR(ec::FRAMEPARAMETER_UNSUPPORTED);
        }
        match frameDesc >> 6 {
            1 => (*fparamsPtr).frameContentSize = *ip.add(5) as u64,
            2 => {
                (*fparamsPtr).frameContentSize =
                    MEM_readLE16(ip.add(5) as *const c_void) as u64 + 256
            }
            3 => (*fparamsPtr).frameContentSize = MEM_readLE64(ip.add(5) as *const c_void),
            _ => (*fparamsPtr).frameContentSize = 0,
        }
    }
    0
}

unsafe fn ZSTDv06_decodeFrameHeader(
    zc: *mut ZSTDv06_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result = ZSTDv06_getFrameParams(&mut (*zc).fParams, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).fParams.windowLog > 25) {
        return ERROR(ec::FRAMEPARAMETER_UNSUPPORTED);
    }
    result
}

#[repr(C)]
struct blockProperties_t {
    blockType: u32,
    origSize: U32,
}

unsafe fn ZSTDv06_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let inp = src as *const BYTE;
    let cSize: U32;

    if srcSize < ZSTDv06_blockHeaderSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    (*bpPtr).blockType = (*inp.add(0) as U32) >> 6;
    cSize = *inp.add(2) as U32
        + ((*inp.add(1) as U32) << 8)
        + (((*inp.add(0) as U32) & 7) << 16);
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

unsafe fn ZSTDv06_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if dst.is_null() {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if srcSize > dstCapacity {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    memcpy(dst, src, srcSize);
    srcSize
}

/* ======  literals block  ====== */

unsafe fn ZSTDv06_decodeLiteralsBlock(
    dctx: *mut ZSTDv06_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    match (*istart.add(0) as U32) >> 6 {
        x if x == IS_HUF => {
            let litSize: usize;
            let litCSize: usize;
            let mut singleStream: usize = 0;
            let mut lhSize: U32 = ((*istart.add(0) as U32) >> 4) & 3;
            if srcSize < 5 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            match lhSize {
                2 => {
                    lhSize = 4;
                    litSize = (((*istart.add(0) as usize) & 15) << 10)
                        + ((*istart.add(1) as usize) << 2)
                        + ((*istart.add(2) as usize) >> 6);
                    litCSize = (((*istart.add(2) as usize) & 63) << 8) + *istart.add(3) as usize;
                }
                3 => {
                    lhSize = 5;
                    litSize = (((*istart.add(0) as usize) & 15) << 14)
                        + ((*istart.add(1) as usize) << 6)
                        + ((*istart.add(2) as usize) >> 2);
                    litCSize = (((*istart.add(2) as usize) & 3) << 16)
                        + ((*istart.add(3) as usize) << 8)
                        + *istart.add(4) as usize;
                }
                _ => {
                    // case 0, 1, default
                    lhSize = 3;
                    singleStream = (*istart.add(0) as usize) & 16;
                    litSize = (((*istart.add(0) as usize) & 15) << 6)
                        + ((*istart.add(1) as usize) >> 2);
                    litCSize = (((*istart.add(1) as usize) & 3) << 8) + *istart.add(2) as usize;
                }
            }
            if litSize > ZSTDv06_BLOCKSIZE_MAX {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ec::CORRUPTION_DETECTED);
            }

            let res = if singleStream != 0 {
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
            if HUFv06_isError(res) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
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
        x if x == IS_PCH => {
            let litSize: usize;
            let litCSize: usize;
            let mut lhSize: U32 = ((*istart.add(0) as U32) >> 4) & 3;
            if lhSize != 1 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            if (*dctx).flagRepeatTable == 0 {
                return ERROR(ec::DICTIONARY_CORRUPTED);
            }

            lhSize = 3;
            litSize = (((*istart.add(0) as usize) & 15) << 6) + ((*istart.add(1) as usize) >> 2);
            litCSize = (((*istart.add(1) as usize) & 3) << 8) + *istart.add(2) as usize;
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ec::CORRUPTION_DETECTED);
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
        x if x == IS_RAW => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.add(0) as U32) >> 4) & 3;
            match lhSize {
                2 => {
                    litSize = (((*istart.add(0) as usize) & 15) << 8) + *istart.add(1) as usize;
                }
                3 => {
                    litSize = (((*istart.add(0) as usize) & 15) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + *istart.add(2) as usize;
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) as usize) & 31;
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
        x if x == IS_RLE => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.add(0) as U32) >> 4) & 3;
            match lhSize {
                2 => {
                    litSize = (((*istart.add(0) as usize) & 15) << 8) + *istart.add(1) as usize;
                }
                3 => {
                    litSize = (((*istart.add(0) as usize) & 15) << 16)
                        + ((*istart.add(1) as usize) << 8)
                        + *istart.add(2) as usize;
                    if srcSize < 4 {
                        return ERROR(ec::CORRUPTION_DETECTED);
                    }
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) as usize) & 31;
                }
            }
            if litSize > ZSTDv06_BLOCKSIZE_MAX {
                return ERROR(ec::CORRUPTION_DETECTED);
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
        _ => ERROR(ec::CORRUPTION_DETECTED),
    }
}

/* ======  sequences  ====== */

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
    match type_ {
        x if x == FSEv06_ENCODING_RLE => {
            if srcSize == 0 {
                return ERROR(ec::SRCSIZE_WRONG);
            }
            if (*(src as *const BYTE) as U32) > max {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            FSEv06_buildDTable_rle(DTable, *(src as *const BYTE));
            1
        }
        x if x == FSEv06_ENCODING_RAW => {
            FSEv06_buildDTable(DTable, defaultNorm, max, defaultLog);
            0
        }
        x if x == FSEv06_ENCODING_STATIC => {
            if flagRepeatTable == 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            0
        }
        _ => {
            // default / FSEv06_ENCODING_DYNAMIC
            let mut tableLog: U32 = 0;
            let mut norm: [S16; MaxSeq + 1] = [0; MaxSeq + 1];
            let mut maxv = max;
            let headerSize =
                FSEv06_readNCount(norm.as_mut_ptr(), &mut maxv, &mut tableLog, src, srcSize);
            if FSEv06_isError(headerSize) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            if tableLog > maxLog {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            FSEv06_buildDTable(DTable, norm.as_ptr(), maxv, tableLog);
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
                nbSeq = MEM_readLE16(ip as *const c_void) as c_int + LONGNBSEQ;
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
        let LLtype: U32 = (*ip as U32) >> 6;
        let Offtype: U32 = ((*ip as U32) >> 4) & 3;
        let MLtype: U32 = ((*ip as U32) >> 2) & 3;
        ip = ip.add(1);

        {
            let bhSize = ZSTDv06_buildSeqTable(
                DTableLL,
                LLtype,
                MaxLL as U32,
                LLFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv06_isError(bhSize) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            ip = ip.add(bhSize);
        }
        {
            let bhSize = ZSTDv06_buildSeqTable(
                DTableOffb,
                Offtype,
                MaxOff as U32,
                OffFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv06_isError(bhSize) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            ip = ip.add(bhSize);
        }
        {
            let bhSize = ZSTDv06_buildSeqTable(
                DTableML,
                MLtype,
                MaxML as U32,
                MLFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv06_isError(bhSize) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
            }
            ip = ip.add(bhSize);
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
    0x3FFFFFF, 1, 1,
];

unsafe fn ZSTDv06_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let llCode: U32 = FSEv06_peekSymbol(&(*seqState).stateLL) as U32;
    let mlCode: U32 = FSEv06_peekSymbol(&(*seqState).stateML) as U32;
    let ofCode: U32 = FSEv06_peekSymbol(&(*seqState).stateOffb) as U32;

    let llBits: U32 = LL_bits[llCode as usize];
    let mlBits: U32 = ML_bits[mlCode as usize];
    let ofBits: U32 = ofCode;
    let totalBits: U32 = llBits + mlBits + ofBits;

    {
        let mut offset: usize;
        if ofCode == 0 {
            offset = 0;
        } else {
            offset = OF_base[ofCode as usize] as usize
                + BITv06_readBits(&mut (*seqState).DStream, ofBits);
            if MEM_32bits() != 0 {
                BITv06_reloadDStream(&mut (*seqState).DStream);
            }
        }

        if offset < ZSTDv06_REP_NUM {
            if llCode == 0 && offset <= 1 {
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
            offset -= ZSTDv06_REP_MOVE;
            (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
            (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
            (*seqState).prevOffset[0] = offset;
        }
        (*seq).offset = offset;
    }

    (*seq).matchLength = ML_base[mlCode as usize] as usize
        + MINMATCH
        + (if mlCode > 31 {
            BITv06_readBits(&mut (*seqState).DStream, mlBits)
        } else {
            0
        });
    if MEM_32bits() != 0 && (mlBits + llBits > 24) {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }

    (*seq).litLength = LL_base[llCode as usize] as usize
        + (if llCode > 15 {
            BITv06_readBits(&mut (*seqState).DStream, llBits)
        } else {
            0
        });
    if MEM_32bits() != 0
        || (totalBits > 64 - 7 - (LLFSELog + MLFSELog + OffFSELog))
    {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }

    FSEv06_updateState(&mut (*seqState).stateLL, &mut (*seqState).DStream);
    FSEv06_updateState(&mut (*seqState).stateML, &mut (*seqState).DStream);
    if MEM_32bits() != 0 {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }
    FSEv06_updateState(&mut (*seqState).stateOffb, &mut (*seqState).DStream);
}

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
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.add(sequenceLength);
    let oend_8 = oend.sub(8);
    let iLitEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.offset(-(sequence.offset as isize)) as *const BYTE;

    let seqLength = sequence.litLength + sequence.matchLength;

    if seqLength > (oend.offset_from(op) as usize) {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if sequence.litLength > (litLimit.offset_from(*litPtr) as usize) {
        return ERROR(ec::CORRUPTION_DETECTED);
    }
    if oLitEnd > oend_8 {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }

    if oMatchEnd > oend {
        return ERROR(ec::DSTSIZE_TOOSMALL);
    }
    if iLitEnd > litLimit {
        return ERROR(ec::CORRUPTION_DETECTED);
    }

    ZSTDv06_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = iLitEnd;

    if sequence.offset > (oLitEnd.offset_from(base) as usize) {
        if sequence.offset > (oLitEnd.offset_from(vBase) as usize) {
            return ERROR(ec::CORRUPTION_DETECTED);
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

    if sequence.offset < 8 {
        static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
        static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
        let sub2: c_int = dec64table[sequence.offset];
        *op.add(0) = *match_.add(0);
        *op.add(1) = *match_.add(1);
        *op.add(2) = *match_.add(2);
        *op.add(3) = *match_.add(3);
        match_ = match_.add(dec32table[sequence.offset] as usize);
        ZSTDv06_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.offset(-(sub2 as isize));
    } else {
        ZSTDv06_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.add(8);
    match_ = match_.add(8);

    if oMatchEnd > oend.sub(16 - MINMATCH) {
        if op < oend_8 {
            ZSTDv06_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                oend_8.offset_from(op) as isize,
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
        ZSTDv06_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize - 8,
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
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(maxDstSize);
    let mut op = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd = litPtr.add((*dctx).litSize);
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: c_int = 0;

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
        if ZSTDv06_isError(seqHSize) != 0 {
            return seqHSize;
        }
        ip = ip.add(seqHSize);
        (*dctx).flagRepeatTable = 0;
    }

    if nbSeq != 0 {
        let mut sequence: seq_t = core::mem::zeroed();
        let mut seqState: seqState_t = core::mem::zeroed();

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
                i += 1;
            }
        }
        {
            let errorCode = BITv06_initDStream(
                &mut seqState.DStream,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ec::CORRUPTION_DETECTED);
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
                    op, oend, sequence, &mut litPtr, litEnd, base, vBase, dictEnd,
                );
                if ZSTDv06_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.add(oneSeqSize);
            }
        }

        if nbSeq != 0 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
    }

    {
        let lastLLSize = litEnd.offset_from(litPtr) as usize;
        if litPtr > litEnd {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        if op.add(lastLLSize) > oend {
            return ERROR(ec::DSTSIZE_TOOSMALL);
        }
        if lastLLSize > 0 {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    op.offset_from(ostart) as usize
}

unsafe fn ZSTDv06_checkContinuity(dctx: *mut ZSTDv06_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char)
            .offset(-((*dctx).previousDstEnd as *const c_char)
                .offset_from((*dctx).base as *const c_char))
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
    let mut ip = src as *const BYTE;

    if srcSize >= ZSTDv06_BLOCKSIZE_MAX {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    {
        let litCSize = ZSTDv06_decodeLiteralsBlock(dctx, src, srcSize);
        if ZSTDv06_isError(litCSize) != 0 {
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

unsafe fn ZSTDv06_decompressFrame(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(dstCapacity);
    let mut remainingSize = srcSize;
    let mut blockProperties = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };

    if srcSize < ZSTDv06_frameHeaderSize_min + ZSTDv06_blockHeaderSize {
        return ERROR(ec::SRCSIZE_WRONG);
    }

    {
        let frameHeaderSize = ZSTDv06_frameHeaderSize(src, ZSTDv06_frameHeaderSize_min);
        if ZSTDv06_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize {
            return ERROR(ec::SRCSIZE_WRONG);
        }
        if ZSTDv06_decodeFrameHeader(dctx, src, frameHeaderSize) != 0 {
            return ERROR(ec::CORRUPTION_DETECTED);
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    loop {
        let mut decodedSize: usize = 0;
        let cBlockSize = ZSTDv06_getcBlockSize(
            ip as *const c_void,
            iend.offset_from(ip) as usize,
            &mut blockProperties,
        );
        if ZSTDv06_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTDv06_blockHeaderSize);
        remainingSize -= ZSTDv06_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(ec::SRCSIZE_WRONG);
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                decodedSize = ZSTDv06_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_raw => {
                decodedSize = ZSTDv06_copyRawBlock(
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_rle => {
                return ERROR(ec::GENERIC);
            }
            x if x == bt_end => {
                if remainingSize != 0 {
                    return ERROR(ec::SRCSIZE_WRONG);
                }
            }
            _ => {
                return ERROR(ec::GENERIC);
            }
        }
        if cBlockSize == 0 {
            break;
        }

        if ZSTDv06_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op.offset_from(ostart) as usize
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
    ZSTDv06_decompress_usingDict(dctx, dst, dstCapacity, src, srcSize, core::ptr::null(), 0)
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
        return ERROR(ec::MEMORY_ALLOCATION);
    }
    regenSize = ZSTDv06_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv06_freeDCtx(dctx);
    regenSize
}

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

    {
        let frameHeaderSize = ZSTDv06_frameHeaderSize(src, srcSize);
        if ZSTDv06_isError(frameHeaderSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, frameHeaderSize);
            return;
        }
        if MEM_readLE32(src) != ZSTDv06_MAGICNUMBER {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::PREFIX_UNKNOWN));
            return;
        }
        if srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::SRCSIZE_WRONG));
            return;
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    loop {
        let cBlockSize =
            ZSTDv06_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv06_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.add(ZSTDv06_blockHeaderSize);
        remainingSize -= ZSTDv06_blockHeaderSize;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ec::SRCSIZE_WRONG));
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
    *dBound = (nbBlocks * ZSTDv06_BLOCKSIZE_MAX) as u64;
}

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
    if srcSize != (*dctx).expected {
        return ERROR(ec::SRCSIZE_WRONG);
    }
    if dstCapacity != 0 {
        ZSTDv06_checkContinuity(dctx, dst);
    }

    match (*dctx).stage {
        x if x == ZSTDds_getFrameHeaderSize => {
            if srcSize != ZSTDv06_frameHeaderSize_min {
                return ERROR(ec::SRCSIZE_WRONG);
            }
            (*dctx).headerSize = ZSTDv06_frameHeaderSize(src, ZSTDv06_frameHeaderSize_min);
            if ZSTDv06_isError((*dctx).headerSize) != 0 {
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
            (*dctx).expected = 0;
            // fall-through
            ZSTDv06_decompressContinue_decodeFrameHeader(dctx, src)
        }
        x if x == ZSTDds_decodeFrameHeader => {
            ZSTDv06_decompressContinue_decodeFrameHeader(dctx, src)
        }
        x if x == ZSTDds_decodeBlockHeader => {
            let mut bp: blockProperties_t = blockProperties_t {
                blockType: 0,
                origSize: 0,
            };
            let cBlockSize = ZSTDv06_getcBlockSize(src, ZSTDv06_blockHeaderSize, &mut bp);
            if ZSTDv06_isError(cBlockSize) != 0 {
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
            0
        }
        x if x == ZSTDds_decompressBlock => {
            let mut rSize: usize;
            match (*dctx).bType {
                y if y == bt_compressed => {
                    rSize =
                        ZSTDv06_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
                }
                y if y == bt_raw => {
                    rSize = ZSTDv06_copyRawBlock(dst, dstCapacity, src, srcSize);
                }
                y if y == bt_rle => {
                    return ERROR(ec::GENERIC);
                }
                y if y == bt_end => {
                    rSize = 0;
                }
                _ => {
                    return ERROR(ec::GENERIC);
                }
            }
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            (*dctx).expected = ZSTDv06_blockHeaderSize;
            if ZSTDv06_isError(rSize) != 0 {
                return rSize;
            }
            (*dctx).previousDstEnd = (dst as *mut c_char).add(rSize) as *const c_void;
            rSize
        }
        _ => ERROR(ec::GENERIC),
    }
}

// Helper for the fall-through from getFrameHeaderSize into decodeFrameHeader.
unsafe fn ZSTDv06_decompressContinue_decodeFrameHeader(
    dctx: *mut ZSTDv06_DCtx,
    src: *const c_void,
) -> usize {
    let result: usize;
    memcpy(
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
    if ZSTDv06_isError(result) != 0 {
        return result;
    }
    (*dctx).expected = ZSTDv06_blockHeaderSize;
    (*dctx).stage = ZSTDds_decodeBlockHeader;
    0
}

unsafe fn ZSTDv06_refDictContent(dctx: *mut ZSTDv06_DCtx, dict: *const c_void, dictSize: usize) {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const c_char)
        .offset(-((*dctx).previousDstEnd as *const c_char)
            .offset_from((*dctx).base as *const c_char)) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const c_char).add(dictSize) as *const c_void;
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
        return ERROR(ec::DICTIONARY_CORRUPTED);
    }
    dict = (dict as *const c_char).add(hSize) as *const c_void;
    dictSize -= hSize;

    {
        let mut offcodeNCount: [i16; MaxOff + 1] = [0; MaxOff + 1];
        let mut offcodeMaxValue: c_uint = MaxOff as c_uint;
        let mut offcodeLog: c_uint = 0;
        offcodeHeaderSize = FSEv06_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dict,
            dictSize,
        );
        if FSEv06_isError(offcodeHeaderSize) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).OffTable.as_mut_ptr(),
                offcodeNCount.as_ptr(),
                offcodeMaxValue,
                offcodeLog,
            );
            if FSEv06_isError(errorCode) != 0 {
                return ERROR(ec::DICTIONARY_CORRUPTED);
            }
        }
        dict = (dict as *const c_char).add(offcodeHeaderSize) as *const c_void;
        dictSize -= offcodeHeaderSize;
    }

    {
        let mut matchlengthNCount: [i16; MaxML + 1] = [0; MaxML + 1];
        let mut matchlengthMaxValue: c_uint = MaxML as c_uint;
        let mut matchlengthLog: c_uint = 0;
        matchlengthHeaderSize = FSEv06_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dict,
            dictSize,
        );
        if FSEv06_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).MLTable.as_mut_ptr(),
                matchlengthNCount.as_ptr(),
                matchlengthMaxValue,
                matchlengthLog,
            );
            if FSEv06_isError(errorCode) != 0 {
                return ERROR(ec::DICTIONARY_CORRUPTED);
            }
        }
        dict = (dict as *const c_char).add(matchlengthHeaderSize) as *const c_void;
        dictSize -= matchlengthHeaderSize;
    }

    {
        let mut litlengthNCount: [i16; MaxLL + 1] = [0; MaxLL + 1];
        let mut litlengthMaxValue: c_uint = MaxLL as c_uint;
        let mut litlengthLog: c_uint = 0;
        litlengthHeaderSize = FSEv06_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dict,
            dictSize,
        );
        if FSEv06_isError(litlengthHeaderSize) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).LLTable.as_mut_ptr(),
                litlengthNCount.as_ptr(),
                litlengthMaxValue,
                litlengthLog,
            );
            if FSEv06_isError(errorCode) != 0 {
                return ERROR(ec::DICTIONARY_CORRUPTED);
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
        ZSTDv06_refDictContent(dctx, dict, dictSize);
        return 0;
    }
    dict = (dict as *const c_char).add(4) as *const c_void;
    dictSize -= 4;
    eSize = ZSTDv06_loadEntropy(dctx, dict, dictSize);
    if ZSTDv06_isError(eSize) != 0 {
        return ERROR(ec::DICTIONARY_CORRUPTED);
    }

    dict = (dict as *const c_char).add(eSize) as *const c_void;
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
        if ZSTDv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    if !dict.is_null() && dictSize != 0 {
        let errorCode = ZSTDv06_decompress_insertDictionary(dctx, dict, dictSize);
        if ZSTDv06_isError(errorCode) != 0 {
            return ERROR(ec::DICTIONARY_CORRUPTED);
        }
    }

    0
}

/* ======  ZBUFF : buffered streaming decompression  ====== */

const ZBUFFds_init: u32 = 0;
const ZBUFFds_loadHeader: u32 = 1;
const ZBUFFds_read: u32 = 2;
const ZBUFFds_load: u32 = 3;
const ZBUFFds_flush: u32 = 4;
type ZBUFFv06_dStage = u32;

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
    memset(zbd as *mut c_void, 0, core::mem::size_of::<ZBUFFv06_DCtx>());
    (*zbd).zd = ZSTDv06_createDCtx();
    if (*zbd).zd.is_null() {
        ZBUFFv06_freeDCtx(zbd);
        return core::ptr::null_mut();
    }
    (*zbd).stage = ZBUFFds_init;
    zbd
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_freeDCtx(zbd: *mut ZBUFFv06_DCtx) -> usize {
    if zbd.is_null() {
        return 0;
    }
    ZSTDv06_freeDCtx((*zbd).zd);
    free((*zbd).inBuff as *mut c_void);
    free((*zbd).outBuff as *mut c_void);
    free(zbd as *mut c_void);
    0
}

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

#[inline]
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressContinue(
    zbd: *mut ZBUFFv06_DCtx,
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
    let mut notDone: U32 = 1;

    while notDone != 0 {
        // `cur` tracks fall-through between stages within one switch pass.
        let mut cur = (*zbd).stage;

        'stages: loop {
            if cur == ZBUFFds_init {
                return ERROR(ec::INIT_MISSING);
            }

            if cur == ZBUFFds_loadHeader {
                {
                    let hSize = ZSTDv06_getFrameParams(
                        &mut (*zbd).fParams,
                        (*zbd).headerBuffer.as_ptr() as *const c_void,
                        (*zbd).lhSize,
                    );
                    if hSize != 0 {
                        let toLoad = hSize - (*zbd).lhSize;
                        if ZSTDv06_isError(hSize) != 0 {
                            return hSize;
                        }
                        if toLoad > (iend.offset_from(ip) as usize) {
                            if !ip.is_null() {
                                memcpy(
                                    (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize)
                                        as *mut c_void,
                                    ip as *const c_void,
                                    iend.offset_from(ip) as usize,
                                );
                            }
                            (*zbd).lhSize += iend.offset_from(ip) as usize;
                            *dstCapacityPtr = 0;
                            return (hSize - (*zbd).lhSize) + ZSTDv06_blockHeaderSize;
                        }
                        memcpy(
                            (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize) as *mut c_void,
                            ip as *const c_void,
                            toLoad,
                        );
                        (*zbd).lhSize = hSize;
                        ip = ip.add(toLoad);
                        break 'stages;
                    }
                }

                {
                    let h1Size = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    let h1Result = ZSTDv06_decompressContinue(
                        (*zbd).zd,
                        core::ptr::null_mut(),
                        0,
                        (*zbd).headerBuffer.as_ptr() as *const c_void,
                        h1Size,
                    );
                    if ZSTDv06_isError(h1Result) != 0 {
                        return h1Result;
                    }
                    if h1Size < (*zbd).lhSize {
                        let h2Size = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                        let h2Result = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            core::ptr::null_mut(),
                            0,
                            (*zbd).headerBuffer.as_ptr().add(h1Size) as *const c_void,
                            h2Size,
                        );
                        if ZSTDv06_isError(h2Result) != 0 {
                            return h2Result;
                        }
                    }
                }

                {
                    let blockSize = if (1usize << (*zbd).fParams.windowLog) < ZSTDv06_BLOCKSIZE_MAX
                    {
                        1usize << (*zbd).fParams.windowLog
                    } else {
                        ZSTDv06_BLOCKSIZE_MAX
                    };
                    (*zbd).blockSize = blockSize;
                    if (*zbd).inBuffSize < blockSize {
                        free((*zbd).inBuff as *mut c_void);
                        (*zbd).inBuffSize = blockSize;
                        (*zbd).inBuff = malloc(blockSize) as *mut c_char;
                        if (*zbd).inBuff.is_null() {
                            return ERROR(ec::MEMORY_ALLOCATION);
                        }
                    }
                    {
                        let neededOutSize = (1usize << (*zbd).fParams.windowLog)
                            + blockSize
                            + WILDCOPY_OVERLENGTH * 2;
                        if (*zbd).outBuffSize < neededOutSize {
                            free((*zbd).outBuff as *mut c_void);
                            (*zbd).outBuffSize = neededOutSize;
                            (*zbd).outBuff = malloc(neededOutSize) as *mut c_char;
                            if (*zbd).outBuff.is_null() {
                                return ERROR(ec::MEMORY_ALLOCATION);
                            }
                        }
                    }
                }
                (*zbd).stage = ZBUFFds_read;
                cur = ZBUFFds_read;
                // fall-through
            }

            if cur == ZBUFFds_read {
                {
                    let neededInSize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    if neededInSize == 0 {
                        (*zbd).stage = ZBUFFds_init;
                        notDone = 0;
                        break 'stages;
                    }
                    if (iend.offset_from(ip) as usize) >= neededInSize {
                        let decodedSize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize - (*zbd).outStart,
                            ip as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv06_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        ip = ip.add(neededInSize);
                        if decodedSize == 0 {
                            break 'stages;
                        }
                        (*zbd).outEnd = (*zbd).outStart + decodedSize;
                        (*zbd).stage = ZBUFFds_flush;
                        break 'stages;
                    }
                    if ip == iend {
                        notDone = 0;
                        break 'stages;
                    }
                    (*zbd).stage = ZBUFFds_load;
                }
                cur = ZBUFFds_load;
                // fall-through
            }

            if cur == ZBUFFds_load {
                {
                    let neededInSize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    let toLoad = neededInSize - (*zbd).inPos;
                    let loadedSize: usize;
                    if toLoad > (*zbd).inBuffSize - (*zbd).inPos {
                        return ERROR(ec::CORRUPTION_DETECTED);
                    }
                    loadedSize = ZBUFFv06_limitCopy(
                        (*zbd).inBuff.add((*zbd).inPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        iend.offset_from(ip) as usize,
                    );
                    ip = ip.add(loadedSize);
                    (*zbd).inPos += loadedSize;
                    if loadedSize < toLoad {
                        notDone = 0;
                        break 'stages;
                    }

                    {
                        let decodedSize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize - (*zbd).outStart,
                            (*zbd).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv06_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        (*zbd).inPos = 0;
                        if decodedSize == 0 {
                            (*zbd).stage = ZBUFFds_read;
                            break 'stages;
                        }
                        (*zbd).outEnd = (*zbd).outStart + decodedSize;
                        (*zbd).stage = ZBUFFds_flush;
                    }
                }
                cur = ZBUFFds_flush;
                // fall-through
            }

            if cur == ZBUFFds_flush {
                let toFlushSize = (*zbd).outEnd - (*zbd).outStart;
                let flushedSize = ZBUFFv06_limitCopy(
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
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
                    break 'stages;
                }
                notDone = 0;
                break 'stages;
            }

            // default: impossible
            return ERROR(ec::GENERIC);
        }
    }

    *srcSizePtr = ip.offset_from(istart) as usize;
    *dstCapacityPtr = op.offset_from(ostart) as usize;
    {
        let mut nextSrcSizeHint = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
        if nextSrcSizeHint > ZSTDv06_blockHeaderSize {
            nextSrcSizeHint += ZSTDv06_blockHeaderSize;
        }
        nextSrcSizeHint -= (*zbd).inPos;
        nextSrcSizeHint
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_recommendedDInSize() -> usize {
    ZSTDv06_BLOCKSIZE_MAX + ZSTDv06_blockHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_recommendedDOutSize() -> usize {
    ZSTDv06_BLOCKSIZE_MAX
}

