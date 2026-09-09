//! Literal Rust transliteration of `c_src/src/legacy/zstd_v04.c`.
//!
//! This file is a self-contained translation unit: it bundles its own
//! mem/endian helpers, error handling, bitstream reader, FSE decoder, Huff0
//! decoder, block/frame decoders, DCtx and both streaming state machines
//! (bufferless ZSTD + buffered ZBUFF).
//!
//! Only the 17 exported wrappers (`ZSTDv04_*` / `ZBUFFv04_*`) are
//! `#[unsafe(no_mangle)]`. Note there is intentionally NO `ZSTDv04_isError`
//! export: in this file `ZSTD_isError` is `static`. Every un-suffixed helper
//! (`FSE_*`, `HUF_*`, `BIT_*`, `MEM_*`, `ZSTD_*`, `ZBUFF_*`) is a plain
//! `pub unsafe fn` -> module namespacing keeps them from clashing with the
//! modern library symbols.

use core::ffi::{c_char, c_void};

// error strings come from the shared common implementation so that
// ZBUFFv04_getErrorName produces byte-identical output to the C library.
use crate::common::error_private::ERR_getErrorName as COMMON_ERR_getErrorName;

// ============================================================================
// Basic types (mem.h)
// ============================================================================
pub type BYTE = u8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;
pub type size_t = usize;

extern "C" {
    fn malloc(size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    fn memset(dst: *mut c_void, c: i32, n: size_t) -> *mut c_void;
}

// ----------------------------------------------------------------------------
// Memory I/O (all reads/writes are little-endian on the target)
// ----------------------------------------------------------------------------
#[inline]
pub unsafe fn MEM_32bits() -> u32 {
    (core::mem::size_of::<*const c_void>() == 4) as u32
}
#[inline]
pub unsafe fn MEM_64bits() -> u32 {
    (core::mem::size_of::<*const c_void>() == 8) as u32
}

#[inline]
pub unsafe fn MEM_read16(memPtr: *const c_void) -> U16 {
    (memPtr as *const U16).read_unaligned()
}
#[inline]
pub unsafe fn MEM_read32(memPtr: *const c_void) -> U32 {
    (memPtr as *const U32).read_unaligned()
}
#[inline]
pub unsafe fn MEM_read64(memPtr: *const c_void) -> U64 {
    (memPtr as *const U64).read_unaligned()
}
#[inline]
pub unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    (memPtr as *mut U16).write_unaligned(value)
}
#[inline]
pub unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    U16::from_le(MEM_read16(memPtr))
}
#[inline]
pub unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    MEM_write16(memPtr, val.to_le())
}
#[inline]
pub unsafe fn MEM_readLE24(memPtr: *const c_void) -> U32 {
    (MEM_readLE16(memPtr) as U32)
        .wrapping_add(((*((memPtr as *const BYTE).add(2))) as U32) << 16)
}
#[inline]
pub unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    U32::from_le(MEM_read32(memPtr))
}
#[inline]
pub unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    U64::from_le(MEM_read64(memPtr))
}
#[inline]
pub unsafe fn MEM_readLEST(memPtr: *const c_void) -> size_t {
    if MEM_32bits() != 0 {
        MEM_readLE32(memPtr) as size_t
    } else {
        MEM_readLE64(memPtr) as size_t
    }
}

// ============================================================================
// Error codes.
//
// IMPORTANT: zstd_v04.c #includes "../common/error_private.h" *before* its own
// (absent) inline error enum. ERROR()/ERR_isError() therefore resolve to the
// MODERN common definitions using the ZSTD_error_* values from zstd_errors.h.
// There is no local FSE_ERROR_* enum in this file. Hence we use those numeric
// values (verified by comparing return codes against the reference libzstd.so).
// ============================================================================
pub const ZSTD_error_no_error: U32 = 0;
pub const ZSTD_error_GENERIC: U32 = 1;
pub const ZSTD_error_prefix_unknown: U32 = 10;
pub const ZSTD_error_version_unsupported: U32 = 12;
pub const ZSTD_error_frameParameter_unsupported: U32 = 14;
pub const ZSTD_error_corruption_detected: U32 = 20;
pub const ZSTD_error_tableLog_tooLarge: U32 = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: U32 = 46;
pub const ZSTD_error_maxSymbolValue_tooSmall: U32 = 48;
pub const ZSTD_error_init_missing: U32 = 62;
pub const ZSTD_error_memory_allocation: U32 = 64;
pub const ZSTD_error_dstSize_tooSmall: U32 = 70;
pub const ZSTD_error_srcSize_wrong: U32 = 72;
pub const ZSTD_error_maxCode: U32 = 120;

// #define ERROR(name) (size_t)-PREFIX(name)
#[inline]
pub fn ERROR(code: U32) -> size_t {
    (0isize.wrapping_sub(code as isize)) as size_t
}

// ERR_STATIC unsigned ERR_isError(size_t code) { return (code > ERROR(maxCode)); }
#[inline]
pub unsafe fn ERR_isError(code: size_t) -> u32 {
    (code > ERROR(ZSTD_error_maxCode)) as u32
}

#[inline]
pub unsafe fn ERR_getErrorName(code: size_t) -> *const c_char {
    COMMON_ERR_getErrorName(code)
}

// ============================================================================
// zstd_internal common constants
// ============================================================================
pub const ZSTD_WINDOWLOG_ABSOLUTEMIN: U32 = 11;
pub const ZSTD_MAGICNUMBER: U32 = 0xFD2FB524; // v0.4

pub const BLOCKSIZE: size_t = 128 * 1024;

pub const ZSTD_blockHeaderSize: size_t = 3;
pub const ZSTD_frameHeaderSize_min: size_t = 5;
pub const ZSTD_frameHeaderSize_max: size_t = 5;

pub const BIT0: U32 = 1;
pub const BIT1: U32 = 2;

pub const IS_RAW: U32 = BIT0;
pub const IS_RLE: U32 = BIT1;

pub const MINMATCH: size_t = 4;

pub const MLbits: U32 = 7;
pub const LLbits: U32 = 6;
pub const Offbits: U32 = 5;
pub const MaxML: U32 = (1 << MLbits) - 1;
pub const MaxLL: U32 = (1 << LLbits) - 1;
pub const MaxOff: U32 = (1 << Offbits) - 1;
pub const MLFSELog: U32 = 10;
pub const LLFSELog: U32 = 10;
pub const OffFSELog: U32 = 9;

pub const MIN_SEQUENCES_SIZE: size_t = 2 + 2 + 3 + 1;
pub const MIN_CBLOCK_SIZE: size_t = 3 + MIN_SEQUENCES_SIZE;

pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

// typedef enum { bt_compressed, bt_raw, bt_rle, bt_end } blockType_t;
pub type blockType_t = u32;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

#[inline]
pub unsafe fn MIN(a: size_t, b: size_t) -> size_t {
    if a < b {
        a
    } else {
        b
    }
}

// static void ZSTD_copy8(void* dst, const void* src) { memcpy(dst, src, 8); }
#[inline]
pub unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

// ZSTD_wildcopy : custom version of memcpy(), can copy up to 7-8 bytes too many
#[inline]
pub unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.offset(length);
    // do { COPY8(op, ip) } while (op < oend);
    loop {
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if !(op < oend) {
            break;
        }
    }
}

// ============================================================================
// Bitstream decompression API (read backward)
// ============================================================================
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BIT_DStream_t {
    pub bitContainer: size_t,
    pub bitsConsumed: u32,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

pub type BIT_DStream_status = u32;
pub const BIT_DStream_unfinished: BIT_DStream_status = 0;
pub const BIT_DStream_endOfBuffer: BIT_DStream_status = 1;
pub const BIT_DStream_completed: BIT_DStream_status = 2;
pub const BIT_DStream_overflow: BIT_DStream_status = 3;

#[inline]
pub unsafe fn BIT_highbit32(val: U32) -> u32 {
    // __builtin_clz(val) ^ 31   (val != 0 in all callers of interest)
    31 ^ val.leading_zeros()
}

pub unsafe fn BIT_initDStream(
    bitD: *mut BIT_DStream_t,
    srcBuffer: *const c_void,
    srcSize: size_t,
) -> size_t {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BIT_DStream_t>());
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    if srcSize >= core::mem::size_of::<size_t>() {
        // normal case
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr =
            (srcBuffer as *const c_char).add(srcSize - core::mem::size_of::<size_t>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        let contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); // endMark not present
        }
        (*bitD).bitsConsumed = 8 - BIT_highbit32(contain32);
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as size_t;
        let stbits = core::mem::size_of::<size_t>() * 8;
        let sp = (*bitD).start as *const BYTE;
        let b = |i: usize| -> size_t { *(sp.add(i)) as size_t };
        // switch with fall-through
        match srcSize {
            7 => {
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add(b(6) << (stbits - 16));
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add(b(5) << (stbits - 24));
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add(b(4) << (stbits - 32));
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(3) << 24);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(2) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(1) << 8);
            }
            6 => {
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add(b(5) << (stbits - 24));
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add(b(4) << (stbits - 32));
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(3) << 24);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(2) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(1) << 8);
            }
            5 => {
                (*bitD).bitContainer =
                    (*bitD).bitContainer.wrapping_add(b(4) << (stbits - 32));
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(3) << 24);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(2) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(1) << 8);
            }
            4 => {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(3) << 24);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(2) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(1) << 8);
            }
            3 => {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(2) << 16);
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(1) << 8);
            }
            2 => {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(b(1) << 8);
            }
            _ => {}
        }
        let contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC); // endMark not present
        }
        (*bitD).bitsConsumed = 8 - BIT_highbit32(contain32);
        (*bitD).bitsConsumed += ((core::mem::size_of::<size_t>() - srcSize) * 8) as u32;
    }

    srcSize
}

#[inline]
pub unsafe fn BIT_lookBits(bitD: *const BIT_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() * 8) as U32 - 1;
    ((((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask - nbBits) & bitMask)) as size_t
}

#[inline]
pub unsafe fn BIT_lookBitsFast(bitD: *const BIT_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() * 8) as U32 - 1;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1) - nbBits) & bitMask)
}

#[inline]
pub unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
pub unsafe fn BIT_readBits(bitD: *mut BIT_DStream_t, nbBits: U32) -> size_t {
    let value = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

#[inline]
pub unsafe fn BIT_readBitsFast(bitD: *mut BIT_DStream_t, nbBits: U32) -> size_t {
    let value = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

pub unsafe fn BIT_reloadDStream(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    let stbytes = core::mem::size_of::<size_t>();
    if (*bitD).bitsConsumed > (stbytes * 8) as u32 {
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.add(stbytes) {
        (*bitD).ptr = (*bitD).ptr.offset(-(((*bitD).bitsConsumed >> 3) as isize));
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize) < stbytes * 8 {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BIT_DStream_status = BIT_DStream_unfinished;
        if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32; // ptr > start
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline]
pub unsafe fn BIT_endOfDStream(dstream: *const BIT_DStream_t) -> u32 {
    (((*dstream).ptr == (*dstream).start)
        && ((*dstream).bitsConsumed as usize == core::mem::size_of::<size_t>() * 8)) as u32
}

// ============================================================================
// FSE : Finite State Entropy coder
// ============================================================================
pub type FSE_DTable = u32;

pub const FSE_MAX_MEMORY_USAGE: u32 = 14;
pub const FSE_MAX_SYMBOL_VALUE: u32 = 255;
pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2; // 12
pub const FSE_MIN_TABLELOG: u32 = 5;
pub const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

// #define FSE_DTABLE_SIZE_U32(maxTableLog) (1 + (1<<maxTableLog))
pub const fn FSE_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

// typedef U32 DTable_max_t[FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)];
pub const DTABLE_MAX_SIZE: usize = FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_DState_t {
    pub state: size_t,
    pub table: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
}

#[inline]
pub unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
    dt: *const FSE_DTable,
) {
    let mut DTableH: FSE_DTableHeader = core::mem::zeroed();
    memcpy(
        &mut DTableH as *mut FSE_DTableHeader as *mut c_void,
        dt as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    (*DStatePtr).state = BIT_readBits(bitD, DTableH.tableLog as U32);
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline]
pub unsafe fn FSE_decodeSymbol(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BIT_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

#[inline]
pub unsafe fn FSE_decodeSymbolFast(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BIT_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

#[inline]
pub unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

#[inline]
pub unsafe fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

pub unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> size_t {
    let mut DTableH: FSE_DTableHeader = core::mem::zeroed();
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSE_decode_t;
    let tableSize: U32 = 1 << tableLog;
    let tableMask: U32 = tableSize - 1;
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize - 1;
    let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    // Sanity Checks
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    // Init, lay down lowprob symbols
    memset(
        tableDecode as *mut c_void,
        0,
        core::mem::size_of::<FSE_decode_t>() * (maxSymbolValue as usize + 1),
    );
    DTableH.tableLog = tableLog as U16;
    s = 0;
    while s <= maxSymbolValue {
        let nc = *normalizedCounter.add(s as usize);
        if nc == -1 {
            (*tableDecode.add(highThreshold as usize)).symbol = s as BYTE;
            highThreshold = highThreshold.wrapping_sub(1);
            symbolNext[s as usize] = 1;
        } else {
            if nc >= largeLimit {
                noLarge = 0;
            }
            symbolNext[s as usize] = nc as U16;
        }
        s += 1;
    }

    // Spread symbols
    s = 0;
    while s <= maxSymbolValue {
        let mut i: i32 = 0;
        let nc = *normalizedCounter.add(s as usize) as i32;
        while i < nc {
            (*tableDecode.add(position as usize)).symbol = s as BYTE;
            position = (position + step) & tableMask;
            while position > highThreshold {
                position = (position + step) & tableMask; // lowprob area
            }
            i += 1;
        }
        s += 1;
    }

    if position != 0 {
        return ERROR(ZSTD_error_GENERIC);
    }

    // Build Decoding table
    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol = (*tableDecode.add(i as usize)).symbol;
            let nextState = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = nextState.wrapping_add(1);
            (*tableDecode.add(i as usize)).nbBits =
                (tableLog - BIT_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(i as usize)).newState =
                (((nextState as U32) << (*tableDecode.add(i as usize)).nbBits)
                    .wrapping_sub(tableSize)) as U16;
            i += 1;
        }
    }

    DTableH.fastMode = noLarge as U16;
    memcpy(
        dt as *mut c_void,
        &DTableH as *const FSE_DTableHeader as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    0
}

#[inline]
pub unsafe fn FSE_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

#[inline]
pub unsafe fn FSE_abs(a: S16) -> S16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

pub unsafe fn FSE_readNCount(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: size_t,
) -> size_t {
    let istart = headerBuffer as *const BYTE;
    let iend = istart.add(hbSize);
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
    nbBits = ((bitStream & 0xF) + FSE_MIN_TABLELOG) as i32; // extract tableLog
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as i32 {
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
            let mut n0 = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 += 24;
                if ip < iend.offset(-5) {
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
                return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
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
                bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: S16 = ((2 * threshold - 1) - remaining) as S16;
            let mut count: S16;

            if (bitStream & (threshold as U32 - 1)) < (max as U32) {
                count = (bitStream & (threshold as U32 - 1)) as S16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold as U32 - 1)) as S16;
                if count >= threshold as S16 {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1; // extra accuracy
            remaining -= FSE_abs(count) as i32;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as i32;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            {
                if (ip <= iend.offset(-7))
                    || (ip.offset((bitCount >> 3) as isize) <= iend.offset(-4))
                {
                    ip = ip.offset((bitCount >> 3) as isize);
                    bitCount &= 7;
                } else {
                    bitCount -= (8 * (iend.offset(-4).offset_from(ip))) as i32;
                    ip = iend.offset(-4);
                }
                bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    if (ip.offset_from(istart) as size_t) > hbSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip.offset_from(istart) as size_t
}

pub unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> size_t {
    let DTableH = dt as *mut FSE_DTableHeader;
    let cell = dt.add(1) as *mut FSE_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

pub unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: u32) -> size_t {
    let DTableH = dt as *mut FSE_DTableHeader;
    let dinfo = dt.add(1) as *mut FSE_decode_t;
    let tableSize: u32 = 1 << nbBits;
    let tableMask: u32 = tableSize - 1;
    let maxSymbolValue: u32 = tableMask;
    let mut s: u32;

    // Sanity checks
    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC); // min size
    }

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

pub unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSE_DTable,
    fast: u32,
) -> size_t {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.add(maxDstSize);
    let olimit = omax.offset(-3);

    let mut bitD: BIT_DStream_t = core::mem::zeroed();
    let mut state1: FSE_DState_t = core::mem::zeroed();
    let mut state2: FSE_DState_t = core::mem::zeroed();

    // Init
    let errorCode = BIT_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    macro_rules! FSE_GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSE_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSE_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    let stbits = (core::mem::size_of::<size_t>() * 8) as u32;

    // 4 symbols per loop
    while (BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit) {
        *op.add(0) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > stbits {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(1) = FSE_GETSYMBOL!(&mut state2);

        if FSE_MAX_TABLELOG * 4 + 7 > stbits {
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > stbits {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(3) = FSE_GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    // tail
    loop {
        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0
                && (fast != 0 || FSE_endOfDState(&state1) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL!(&mut state1);
        op = op.add(1);

        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0
                && (fast != 0 || FSE_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL!(&mut state2);
        op = op.add(1);
    }

    // end ?
    if BIT_endOfDStream(&bitD) != 0
        && FSE_endOfDState(&state1) != 0
        && FSE_endOfDState(&state2) != 0
    {
        return op.offset_from(ostart) as size_t;
    }

    if op == omax {
        return ERROR(ZSTD_error_dstSize_tooSmall); // dst buffer is full, but cSrc unfinished
    }

    ERROR(ZSTD_error_corruption_detected)
}

pub unsafe fn FSE_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSE_DTable,
) -> size_t {
    let mut DTableH: FSE_DTableHeader = core::mem::zeroed();
    let fastMode: U32;

    memcpy(
        &mut DTableH as *mut FSE_DTableHeader as *mut c_void,
        dt as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    fastMode = DTableH.fastMode as U32;

    if fastMode != 0 {
        return FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

pub unsafe fn FSE_decompress(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [S16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [FSE_DTable; DTABLE_MAX_SIZE] = [0; DTABLE_MAX_SIZE];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE;
    let mut errorCode: size_t;
    let mut cSrcSize = cSrcSize;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong); // too small input size
    }

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
        return ERROR(ZSTD_error_srcSize_wrong); // too small input size
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSE_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    // always return, even if it is an error code
    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

// ============================================================================
// Huff0 : Huffman coder
// ============================================================================
pub const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16;
pub const HUF_MAX_TABLELOG: u32 = 12;
pub const HUF_MAX_SYMBOL_VALUE: u32 = 255;

// #define HUF_DTABLE_SIZE(maxTableLog) (1 + (1<<maxTableLog))
pub const fn HUF_DTABLE_SIZE(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

#[inline]
pub unsafe fn HUF_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
} // single-symbol decoding

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
} // double-symbols decoding

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
    pub weight: BYTE,
}

pub unsafe fn HUF_readStats(
    huffWeight: *mut BYTE,
    hwSize: size_t,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut weightTotal: U32;
    let tableLog: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: size_t;
    let oSize: size_t;
    let mut n: U32;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.add(0) as size_t;

    if iSize >= 128 {
        // special header
        if iSize >= 242 {
            // RLE
            static L: [i32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = L[(iSize - 242) as usize] as size_t;
            memset(huffWeight as *mut c_void, 1, hwSize);
            iSize = 0;
        } else {
            // Incompressible
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
            while (n as size_t) < oSize {
                *huffWeight.add(n as usize) = *ip.add((n / 2) as usize) >> 4;
                *huffWeight.add((n + 1) as usize) = *ip.add((n / 2) as usize) & 15;
                n += 2;
            }
        }
    } else {
        // header compressed with FSE (normal case)
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        oSize = FSE_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.add(1) as *const c_void,
            iSize,
        );
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }

    // collect weight stats
    memset(
        rankStats as *mut c_void,
        0,
        (HUF_ABSOLUTEMAX_TABLELOG as usize + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as size_t) < oSize {
        let w = *huffWeight.add(n as usize) as U32;
        if w >= HUF_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *rankStats.add(w as usize) += 1;
        weightTotal += (1u32 << w) >> 1;
        n += 1;
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    // get last non-null symbol weight (implied, total must be 2^n)
    tableLog = BIT_highbit32(weightTotal) + 1;
    if tableLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_corruption_detected);
    }
    {
        let total: U32 = 1 << tableLog;
        let rest: U32 = total - weightTotal;
        let verif: U32 = 1 << BIT_highbit32(rest);
        let lastWeight: U32 = BIT_highbit32(rest) + 1;
        if verif != rest {
            return ERROR(ZSTD_error_corruption_detected); // last value must be a clean power of 2
        }
        *huffWeight.add(oSize) = lastWeight as BYTE;
        *rankStats.add(lastWeight as usize) += 1;
    }

    // check tree construction validity
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    // results
    *nbSymbolsPtr = (oSize + 1) as U32;
    *tableLogPtr = tableLog;
    iSize + 1
}

// -------------------- single-symbol decoding --------------------

pub unsafe fn HUF_readDTableX2(DTable: *mut U16, src: *const c_void, srcSize: size_t) -> size_t {
    let mut huffWeight: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut tableLog: U32 = 0;
    let iSize: size_t;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUF_DEltX2;

    iSize = HUF_readStats(
        huffWeight.as_mut_ptr(),
        (HUF_MAX_SYMBOL_VALUE + 1) as size_t,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    // check result
    if tableLog > *DTable.add(0) as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge); // DTable is too small
    }
    *DTable.add(0) = tableLog as U16;

    // Prepare ranks
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current = nextRankStart;
        nextRankStart += rankVal[n as usize] << (n - 1);
        rankVal[n as usize] = current;
        n += 1;
    }

    // fill DTable
    n = 0;
    while n < nbSymbols {
        let w = huffWeight[n as usize] as U32;
        let length = (1u32 << w) >> 1;
        let mut i: U32;
        let mut D: HUF_DEltX2 = core::mem::zeroed();
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
pub unsafe fn HUF_decodeSymbolX2(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BIT_lookBitsFast(Dstream, dtLog); // note : dtLog >= 1
    let c = (*dt.add(val)).byte;
    BIT_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

// HUF_DECODE_SYMBOLX2_0(ptr, DStreamPtr): *ptr++ = HUF_decodeSymbolX2(...)
macro_rules! HUF_DECODE_SYMBOLX2_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUF_decodeSymbolX2($DStreamPtr, $dt, $dtLog);
        $ptr = $ptr.add(1);
    }};
}
// HUF_DECODE_SYMBOLX2_1: if (MEM_64bits() || (HUF_MAX_TABLELOG<=12)) _0
macro_rules! HUF_DECODE_SYMBOLX2_1 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUF_MAX_TABLELOG <= 12) {
            HUF_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    }};
}
// HUF_DECODE_SYMBOLX2_2: if (MEM_64bits()) _0
macro_rules! HUF_DECODE_SYMBOLX2_2 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            HUF_DECODE_SYMBOLX2_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    }};
}

pub unsafe fn HUF_decodeStreamX2(
    p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart = p;
    let mut p = p;

    // up to 4 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.offset(-4)) {
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    // closer to the end
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd) {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    // no more data to retrieve from bitstream, hence no need to reload
    while p < pEnd {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    pEnd.offset_from(pStart) as size_t
}

pub unsafe fn HUF_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const U16,
) -> size_t {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected); // strict minimum : jump table + 1 byte per stream
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUF_DEltX2).add(1);
        let dtLog = *DTable.add(0) as U32;
        let mut errorCode: size_t;

        // Init
        let mut bitD1: BIT_DStream_t = core::mem::zeroed();
        let mut bitD2: BIT_DStream_t = core::mem::zeroed();
        let mut bitD3: BIT_DStream_t = core::mem::zeroed();
        let mut bitD4: BIT_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart as *const c_void) as size_t;
        let length2 = MEM_readLE16(istart.add(2) as *const c_void) as size_t;
        let length3 = MEM_readLE16(istart.add(4) as *const c_void) as size_t;
        let length4: size_t;
        let istart1 = istart.add(6); // jumpTable
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
            return ERROR(ZSTD_error_corruption_detected); // overflow
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

        // 16-32 symbols per loop
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.offset(-7)) {
            HUF_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX2_1!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX2_1!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX2_1!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX2_1!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX2_0!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX2_0!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX2_0!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX2_0!(op4, &mut bitD4, dt, dtLog);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        // check corruption
        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        // note : op4 supposed already verified within main loop

        // finish bitStreams one by one
        HUF_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        // check
        endSignal = BIT_endOfDStream(&bitD1)
            & BIT_endOfDStream(&bitD2)
            & BIT_endOfDStream(&bitD3)
            & BIT_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        // decoded size
        dstSize
    }
}

pub unsafe fn HUF_decompress4X2(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    // HUF_CREATE_STATIC_DTABLEX2(DTable, HUF_MAX_TABLELOG): unsigned short[..] = { maxTableLog }
    let mut DTable: [U16; HUF_DTABLE_SIZE(HUF_MAX_TABLELOG)] =
        [0; HUF_DTABLE_SIZE(HUF_MAX_TABLELOG)];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;
    let errorCode: size_t;

    errorCode = HUF_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUF_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// -------------------- double-symbols decoding --------------------

pub unsafe fn HUF_fillDTableX4Level2(
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
    let mut DElt: HUF_DEltX4 = core::mem::zeroed();
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut s: U32;

    // get pre-calculated rankVal
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of_val(&rankVal),
    );

    // fill skipped values
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

    // fill DTable
    s = 0;
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

// typedef U32 rankVal_t[HUF_ABSOLUTEMAX_TABLELOG][HUF_ABSOLUTEMAX_TABLELOG + 1];
pub type rankVal_t = [[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    HUF_ABSOLUTEMAX_TABLELOG as usize];

pub unsafe fn HUF_fillDTableX4(
    DTable: *mut HUF_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *mut rankVal_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: i32 = nbBitsBaseline as i32 - targetLog as i32;
    let minBits: U32 = nbBitsBaseline - maxWeight;
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        (*rankValOrigin).as_ptr() as *const c_void,
        core::mem::size_of_val(&rankVal),
    );

    // fill DTable
    s = 0;
    while s < sortedListSize {
        let symbol = (*sortedList.add(s as usize)).symbol as U16;
        let weight = (*sortedList.add(s as usize)).weight as U32;
        let nbBits = nbBitsBaseline - weight;
        let start = rankVal[weight as usize];
        let length = 1u32 << (targetLog - nbBits);

        if targetLog - nbBits >= minBits {
            // enough room for a second symbol
            let sortedRank: U32;
            let mut minWeight = nbBits as i32 + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUF_fillDTableX4Level2(
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
            let mut i: U32;
            let end = start + length;
            let mut DElt: HUF_DEltX4 = core::mem::zeroed();

            MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, symbol);
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

pub unsafe fn HUF_readDTableX4(DTable: *mut U32, src: *const c_void, srcSize: size_t) -> size_t {
    let mut weightList: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let mut rankVal: rankVal_t =
        [[0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]; HUF_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog = *DTable.add(0);
    let iSize: size_t;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUF_DEltX4).add(1);

    // rankStart = rankStart0 + 1
    let rankStart = rankStart0.as_mut_ptr().add(1);

    if memLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUF_readStats(
        weightList.as_mut_ptr(),
        (HUF_MAX_SYMBOL_VALUE + 1) as size_t,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    // check result
    if tableLog > memLog {
        return ERROR(ZSTD_error_tableLog_tooLarge); // DTable can't fit code depth
    }

    // find maxWeight
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        if maxW == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
        maxW -= 1;
    }

    // Get start index of each weight
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w <= maxW {
            let current = nextRankStart;
            nextRankStart += rankStats[w as usize];
            *rankStart.add(w as usize) = current;
            w += 1;
        }
        *rankStart.add(0) = nextRankStart; // put all 0w symbols at the end of sorted list
        sizeOfSort = nextRankStart;
    }

    // sort symbols by weight
    {
        let mut s: U32;
        s = 0;
        while s < nbSymbols {
            let w = weightList[s as usize] as U32;
            let r = *rankStart.add(w as usize);
            *rankStart.add(w as usize) = r + 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        *rankStart.add(0) = 0; // forget 0w symbols; this is beginning of weight(1)
    }

    // Build rankVal
    {
        let minBits: U32 = tableLog + 1 - maxW;
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: i32 = (memLog as i32 - tableLog as i32) - 1;
        // rankVal0 = rankVal[0]
        w = 1;
        while w <= maxW {
            let current = nextRankVal;
            nextRankVal += rankStats[w as usize] << (w as i32 + rescale);
            rankVal[0][w as usize] = current;
            w += 1;
        }
        consumed = minBits;
        while consumed <= memLog - minBits {
            w = 1;
            while w <= maxW {
                let v = rankVal[0][w as usize] >> consumed;
                rankVal[consumed as usize][w as usize] = v;
                w += 1;
            }
            consumed += 1;
        }
    }

    HUF_fillDTableX4(
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
pub unsafe fn HUF_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog); // note : dtLog >= 1
    memcpy(op, dt.add(val) as *const c_void, 2);
    BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

#[inline]
pub unsafe fn HUF_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog); // note : dtLog >= 1
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        let stbits = (core::mem::size_of::<size_t>() * 8) as u32;
        if (*DStream).bitsConsumed < stbits {
            BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > stbits {
                (*DStream).bitsConsumed = stbits; // ugly hack; works only because it's the last symbol
            }
        }
    }
    1
}

macro_rules! HUF_DECODE_SYMBOLX4_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.offset(HUF_decodeSymbolX4($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog)
            as isize);
    }};
}
macro_rules! HUF_DECODE_SYMBOLX4_1 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUF_MAX_TABLELOG <= 12) {
            HUF_DECODE_SYMBOLX4_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    }};
}
macro_rules! HUF_DECODE_SYMBOLX4_2 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            HUF_DECODE_SYMBOLX4_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    }};
}

pub unsafe fn HUF_decodeStreamX4(
    p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> size_t {
    let pStart = p;
    let mut p = p;

    // up to 8 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd.offset(-7)) {
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    // closer to the end
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.offset(-2)) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p <= pEnd.offset(-2) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog); // no need to reload
    }

    if p < pEnd {
        p = p.offset(HUF_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as isize);
    }

    p.offset_from(pStart) as size_t
}

pub unsafe fn HUF_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const U32,
) -> size_t {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUF_DEltX4).add(1);
        let dtLog = *DTable.add(0);
        let mut errorCode: size_t;

        let mut bitD1: BIT_DStream_t = core::mem::zeroed();
        let mut bitD2: BIT_DStream_t = core::mem::zeroed();
        let mut bitD3: BIT_DStream_t = core::mem::zeroed();
        let mut bitD4: BIT_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart as *const c_void) as size_t;
        let length2 = MEM_readLE16(istart.add(2) as *const c_void) as size_t;
        let length3 = MEM_readLE16(istart.add(4) as *const c_void) as size_t;
        let length4: size_t;
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
            return ERROR(ZSTD_error_corruption_detected);
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

        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.offset(-7)) {
            HUF_DECODE_SYMBOLX4_2!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX4_1!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX4_1!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX4_1!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX4_1!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX4_2!(op4, &mut bitD4, dt, dtLog);
            HUF_DECODE_SYMBOLX4_0!(op1, &mut bitD1, dt, dtLog);
            HUF_DECODE_SYMBOLX4_0!(op2, &mut bitD2, dt, dtLog);
            HUF_DECODE_SYMBOLX4_0!(op3, &mut bitD3, dt, dtLog);
            HUF_DECODE_SYMBOLX4_0!(op4, &mut bitD4, dt, dtLog);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        // check corruption
        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        // finish bitStreams one by one
        HUF_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        // check
        endSignal = BIT_endOfDStream(&bitD1)
            & BIT_endOfDStream(&bitD2)
            & BIT_endOfDStream(&bitD3)
            & BIT_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        dstSize
    }
}

pub unsafe fn HUF_decompress4X4(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    // HUF_CREATE_STATIC_DTABLEX4(DTable, HUF_MAX_TABLELOG): unsigned int[..] = { maxTableLog }
    let mut DTable: [U32; HUF_DTABLE_SIZE(HUF_MAX_TABLELOG)] =
        [0; HUF_DTABLE_SIZE(HUF_MAX_TABLELOG)];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let hSize = HUF_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// -------------------- Generic decompression selector --------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
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

pub unsafe fn HUF_decompress(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    // static const decompressionAlgo decompress[3] = { HUF_decompress4X2, HUF_decompress4X4, NULL };
    let mut Q: U32;
    let D256: U32 = (dstSize >> 8) as U32;
    let mut Dtime: [U32; 3] = [0; 3];
    let mut algoNb: U32 = 0;
    let mut n: i32;

    // validation checks
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected); // invalid
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize; // not compressed
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as i32, dstSize);
        return dstSize; // RLE
    }

    // decoder timing evaluation
    Q = (cSrcSize * 16 / dstSize) as U32; // Q < 16 since dstSize > cSrcSize
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

    if algoNb == 0 {
        HUF_decompress4X2(dst, dstSize, cSrc, cSrcSize)
    } else {
        HUF_decompress4X4(dst, dstSize, cSrc, cSrcSize)
    }
}

// ============================================================================
// zstd - decompression module for v0.4 legacy format
// ============================================================================

pub type ZSTD_strategy = u32;
pub const ZSTD_fast: ZSTD_strategy = 0;
pub const ZSTD_greedy: ZSTD_strategy = 1;
pub const ZSTD_lazy: ZSTD_strategy = 2;
pub const ZSTD_lazy2: ZSTD_strategy = 3;
pub const ZSTD_btlazy2: ZSTD_strategy = 4;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_parameters {
    pub srcSize: U64,
    pub windowLog: U32,
    pub contentLog: U32,
    pub hashLog: U32,
    pub searchLog: U32,
    pub searchLength: U32,
    pub strategy: ZSTD_strategy,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

// static void ZSTD_copy4(void* dst, const void* src) { memcpy(dst, src, 4); }
#[inline]
pub unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

#[inline]
pub unsafe fn ZSTD_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

pub type ZSTD_dStage = u32;
pub const ZSTDds_getFrameHeaderSize: ZSTD_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTD_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTD_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTD_dStage = 3;

#[repr(C)]
pub struct ZSTDv04_Dctx_s {
    pub LLTable: [U32; FSE_DTABLE_SIZE_U32(LLFSELog)],
    pub OffTable: [U32; FSE_DTABLE_SIZE_U32(OffFSELog)],
    pub MLTable: [U32; FSE_DTABLE_SIZE_U32(MLFSELog)],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: size_t,
    pub headerSize: size_t,
    pub params: ZSTD_parameters,
    pub bType: blockType_t,
    pub stage: ZSTD_dStage,
    pub litPtr: *const BYTE,
    pub litSize: size_t,
    pub litBuffer: [BYTE; BLOCKSIZE + 8],
    pub headerBuffer: [BYTE; ZSTD_frameHeaderSize_max],
}

pub type ZSTD_DCtx = ZSTDv04_Dctx_s;

pub unsafe fn ZSTD_resetDCtx(dctx: *mut ZSTD_DCtx) -> size_t {
    (*dctx).expected = ZSTD_frameHeaderSize_min;
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    0
}

pub unsafe fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    let dctx = malloc(core::mem::size_of::<ZSTD_DCtx>()) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_resetDCtx(dctx);
    dctx
}

pub unsafe fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> size_t {
    free(dctx as *mut c_void);
    0
}

// ZSTD_decodeFrameHeader_Part1
pub unsafe fn ZSTD_decodeFrameHeader_Part1(
    zc: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
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

pub unsafe fn ZSTD_getFrameParams(
    params: *mut ZSTD_parameters,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let magicNumber: U32;
    if srcSize < ZSTD_frameHeaderSize_min {
        return ZSTD_frameHeaderSize_max;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    memset(params as *mut c_void, 0, core::mem::size_of::<ZSTD_parameters>());
    (*params).windowLog =
        ((*((src as *const BYTE).add(4)) & 15) as U32) + ZSTD_WINDOWLOG_ABSOLUTEMIN;
    if (*((src as *const BYTE).add(4)) >> 4) != 0 {
        return ERROR(ZSTD_error_frameParameter_unsupported); // reserved bits
    }
    0
}

// ZSTD_decodeFrameHeader_Part2
pub unsafe fn ZSTD_decodeFrameHeader_Part2(
    zc: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let result: size_t;
    if srcSize != (*zc).headerSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    result = ZSTD_getFrameParams(&mut (*zc).params, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).params.windowLog > 25) {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    result
}

pub unsafe fn ZSTD_getcBlockSize(
    src: *const c_void,
    srcSize: size_t,
    bpPtr: *mut blockProperties_t,
) -> size_t {
    let inp = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    headerFlags = *inp;
    cSize = (*inp.add(2) as U32)
        + ((*inp.add(1) as U32) << 8)
        + (((*inp.add(0) & 7) as U32) << 16);

    (*bpPtr).blockType = (headerFlags >> 6) as blockType_t;
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as size_t
}

pub unsafe fn ZSTD_copyRawBlock(
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    if srcSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

// ZSTD_decompressLiterals
pub unsafe fn ZSTD_decompressLiterals(
    dst: *mut c_void,
    maxDstSizePtr: *mut size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let ip = src as *const BYTE;

    let litSize = ((MEM_readLE32(src) & 0x1FFFFF) >> 2) as size_t;
    let litCSize = ((MEM_readLE32(ip.add(2) as *const c_void) & 0xFFFFFF) >> 5) as size_t;

    if litSize > *maxDstSizePtr {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if litCSize + 5 > srcSize {
        return ERROR(ZSTD_error_corruption_detected);
    }

    if HUF_isError(HUF_decompress(dst, litSize, ip.add(5) as *const c_void, litCSize)) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    *maxDstSizePtr = litSize;
    litCSize + 5
}

// ZSTD_decodeLiteralsBlock
pub unsafe fn ZSTD_decodeLiteralsBlock(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart & 3) as U32 {
        // compressed
        0 => {
            let mut litSize: size_t = BLOCKSIZE;
            let readSize =
                ZSTD_decompressLiterals((*dctx).litBuffer.as_mut_ptr() as *mut c_void, &mut litSize, src, srcSize);
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                8,
            );
            readSize // works if it's an error too
        }
        // IS_RAW
        1 => {
            let litSize = ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as size_t;
            if litSize > srcSize - 11 {
                if litSize > BLOCKSIZE {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                if litSize > srcSize - 3 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                memcpy(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    istart as *const c_void,
                    litSize,
                );
                (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
                (*dctx).litSize = litSize;
                memset(
                    (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                    0,
                    8,
                );
                return litSize + 3;
            }
            // direct reference into compressed stream
            (*dctx).litPtr = istart.add(3);
            (*dctx).litSize = litSize;
            litSize + 3
        }
        // IS_RLE
        2 => {
            let litSize = ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as size_t;
            if litSize > BLOCKSIZE {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.add(3) as i32,
                litSize + 8,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            4
        }
        _ => ERROR(ZSTD_error_corruption_detected), // forbidden nominal case
    }
}

pub unsafe fn ZSTD_decodeSeqHeaders(
    nbSeq: *mut i32,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut size_t,
    DTableLL: *mut FSE_DTable,
    DTableML: *mut FSE_DTable,
    DTableOffb: *mut FSE_DTable,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let istart = src as *const BYTE;
    let mut ip = istart;
    let iend = istart.add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: U32 = 0;
    let mut Offlog: U32 = 0;
    let mut MLlog: U32 = 0;
    let mut dumpsLength: size_t;

    // check
    if srcSize < 5 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    // SeqHead
    *nbSeq = MEM_readLE16(ip as *const c_void) as i32;
    ip = ip.add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = *ip.add(2) as size_t;
        dumpsLength += (*ip.add(1) as size_t) << 8;
        ip = ip.add(3);
    } else {
        dumpsLength = *ip.add(1) as size_t;
        dumpsLength += ((*ip.add(0) & 1) as size_t) << 8;
        ip = ip.add(2);
    }
    *dumpsPtr = ip;
    ip = ip.add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    // check
    if ip > iend.offset(-3) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    // sequences
    {
        let mut norm: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut headerSize: size_t;

        // Build DTables (LL)
        match LLtype {
            x if x == bt_rle => {
                LLlog = 0;
                FSE_buildDTable_rle(DTableLL, *ip);
                ip = ip.add(1);
            }
            x if x == bt_raw => {
                LLlog = LLbits;
                FSE_buildDTable_raw(DTableLL, LLbits);
            }
            _ => {
                let mut max: U32 = MaxLL;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut LLlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as size_t,
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if LLlog > LLFSELog {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        // Offb
        match Offtype {
            x if x == bt_rle => {
                Offlog = 0;
                if ip > iend.offset(-2) {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                FSE_buildDTable_rle(DTableOffb, *ip & MaxOff as BYTE);
                ip = ip.add(1);
            }
            x if x == bt_raw => {
                Offlog = Offbits;
                FSE_buildDTable_raw(DTableOffb, Offbits);
            }
            _ => {
                let mut max: U32 = MaxOff;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut Offlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as size_t,
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if Offlog > OffFSELog {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
            }
        }

        // ML
        match MLtype {
            x if x == bt_rle => {
                MLlog = 0;
                if ip > iend.offset(-2) {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                FSE_buildDTable_rle(DTableML, *ip);
                ip = ip.add(1);
            }
            x if x == bt_raw => {
                MLlog = MLbits;
                FSE_buildDTable_raw(DTableML, MLbits);
            }
            _ => {
                let mut max: U32 = MaxML;
                headerSize = FSE_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut MLlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as size_t,
                );
                if FSE_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if MLlog > MLFSELog {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
            }
        }
        let _ = (LLlog, Offlog, MLlog);
    }

    ip.offset_from(istart) as size_t
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct seq_t {
    pub litLength: size_t,
    pub offset: size_t,
    pub matchLength: size_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct seqState_t {
    pub DStream: BIT_DStream_t,
    pub stateLL: FSE_DState_t,
    pub stateOffb: FSE_DState_t,
    pub stateML: FSE_DState_t,
    pub prevOffset: size_t,
    pub dumps: *const BYTE,
    pub dumpsEnd: *const BYTE,
}

pub unsafe fn ZSTD_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let mut litLength: size_t;
    let prevOffset: size_t;
    let mut offset: size_t;
    let mut matchLength: size_t;
    let mut dumps = (*seqState).dumps;
    let de = (*seqState).dumpsEnd;

    // Literal length
    litLength = FSE_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream) as size_t;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    if litLength == MaxLL as size_t {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.add(1);
            v
        } else {
            0
        };
        if add < 255 {
            litLength += add as size_t;
        } else if dumps.add(3) <= de {
            litLength = MEM_readLE24(dumps as *const c_void) as size_t;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.offset(-1);
        }
    }

    // Offset
    {
        static offsetPrefix: [U32; (MaxOff + 1) as usize] = [
            1, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
            65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216,
            33554432, 1, 1, 1, 1, 1,
        ];
        let offsetCode: U32;
        let mut nbBits: U32;
        offsetCode = FSE_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream) as U32;
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        nbBits = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0; // cmove
        }
        offset = (offsetPrefix[offsetCode as usize] as size_t)
            .wrapping_add(BIT_readBits(&mut (*seqState).DStream, nbBits));
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset; // cmove
        }
        if (offsetCode != 0) || (litLength == 0) {
            (*seqState).prevOffset = (*seq).offset; // cmove
        }
    }

    // MatchLength
    matchLength = FSE_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as size_t;
    if matchLength == MaxML as size_t {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.add(1);
            v
        } else {
            0
        };
        if add < 255 {
            matchLength += add as size_t;
        } else if dumps.add(3) <= de {
            matchLength = MEM_readLE24(dumps as *const c_void) as size_t;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.offset(-1);
        }
    }
    matchLength += MINMATCH;

    // save result
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

pub unsafe fn ZSTD_execSequence(
    op: *mut BYTE,
    oend: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    static dec32table: [i32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; // added
    static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; // subtracted
    let mut sequence = sequence;
    let mut op = op;
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.add(sequenceLength);
    let oend_8 = oend.offset(-8);
    let litEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.offset(-(sequence.offset as isize));

    // checks
    let seqLength = sequence.litLength + sequence.matchLength;

    if seqLength > (oend.offset_from(op) as size_t) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit.offset_from(*litPtr) as size_t) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if oLitEnd > oend_8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    if oMatchEnd > oend {
        return ERROR(ZSTD_error_dstSize_tooSmall); // overwrite beyond dst buffer
    }
    if litEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected); // overRead beyond lit buffer
    }

    // copy Literals
    ZSTD_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = litEnd; // update for next sequence

    // copy Match
    if sequence.offset > (oLitEnd.offset_from(base) as size_t) {
        // offset beyond prefix
        if sequence.offset > (oLitEnd.offset_from(vBase) as size_t) {
            return ERROR(ZSTD_error_corruption_detected);
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
        // span extDict & currentPrefixSegment
        {
            let length1 = dictEnd.offset_from(match_) as size_t;
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
    // Requirement: op <= oend_8

    // match within prefix
    if sequence.offset < 8 {
        // close range match, overlap
        let sub2 = dec64table[sequence.offset];
        *op.add(0) = *match_.add(0);
        *op.add(1) = *match_.add(1);
        *op.add(2) = *match_.add(2);
        *op.add(3) = *match_.add(3);
        match_ = match_.add(dec32table[sequence.offset] as usize);
        ZSTD_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.offset(-(sub2 as isize));
    } else {
        ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.add(8);
    match_ = match_.add(8);

    if oMatchEnd > oend.offset(-(16 - MINMATCH as isize)) {
        if op < oend_8 {
            ZSTD_wildcopy(
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
        ZSTD_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize - 8,
        );
    }
    sequenceLength
}

pub unsafe fn ZSTD_decompressSequences(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
) -> size_t {
    let mut ip = seqStart as *const BYTE;
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut errorCode: size_t;
    let mut dumpsLength: size_t = 0;
    let mut litPtr = (*dctx).litPtr;
    let litEnd = litPtr.add((*dctx).litSize);
    let mut nbSeq: i32 = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;

    // Build Decoding Tables
    errorCode = ZSTD_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        iend.offset_from(ip) as size_t,
    );
    if ZSTD_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.add(errorCode);

    // Regen sequences
    {
        let mut sequence: seq_t = core::mem::zeroed();
        let mut seqState: seqState_t = core::mem::zeroed();

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        sequence.offset = 4;
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.add(dumpsLength);
        seqState.prevOffset = 4;
        errorCode = BIT_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            iend.offset_from(ip) as size_t,
        );
        if ERR_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSE_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSE_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSE_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BIT_reloadDStream(&mut seqState.DStream) <= BIT_DStream_completed) && nbSeq != 0 {
            let oneSeqSize: size_t;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(
                op, oend, sequence, &mut litPtr, litEnd, base, vBase, dictEnd,
            );
            if ZSTD_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
        }

        // check if reached exact end
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        // last literal segment
        {
            let lastLLSize = litEnd.offset_from(litPtr) as size_t;
            if litPtr > litEnd {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if op.add(lastLLSize) > oend {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            if lastLLSize > 0 {
                if op != litPtr as *mut BYTE {
                    memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.add(lastLLSize);
            }
        }
    }

    op.offset_from(ostart) as size_t
}

pub unsafe fn ZSTD_checkContinuity(dctx: *mut ZSTD_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        // not contiguous
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char).offset(
            -((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char),
        ) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

pub unsafe fn ZSTD_decompressBlock_internal(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    // blockType == blockCompressed
    let mut ip = src as *const BYTE;
    let litCSize: size_t;
    let mut srcSize = srcSize;

    if srcSize > BLOCKSIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    // Decode literals sub-block
    litCSize = ZSTD_decodeLiteralsBlock(dctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.add(litCSize);
    srcSize -= litCSize;

    ZSTD_decompressSequences(dctx, dst, maxDstSize, ip as *const c_void, srcSize)
}

pub unsafe fn ZSTD_decompress_usingDict(
    ctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    let mut ip = src as *const BYTE;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut remainingSize = srcSize;
    let mut blockProperties: blockProperties_t = core::mem::zeroed();

    // init
    ZSTD_resetDCtx(ctx);
    if !dict.is_null() {
        ZSTD_decompress_insertDictionary(ctx, dict, dictSize);
        (*ctx).dictEnd = (*ctx).previousDstEnd;
        (*ctx).vBase = (dst as *const c_char).offset(
            -((*ctx).previousDstEnd as *const c_char).offset_from((*ctx).base as *const c_char),
        ) as *const c_void;
        (*ctx).base = dst;
    } else {
        (*ctx).base = dst;
        (*ctx).vBase = dst;
        (*ctx).dictEnd = dst;
    }

    // Frame Header
    {
        let mut frameHeaderSize: size_t;
        if srcSize < ZSTD_frameHeaderSize_min + ZSTD_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        frameHeaderSize = ZSTD_decodeFrameHeader_Part1(ctx, src, ZSTD_frameHeaderSize_min);
        if ZSTD_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTD_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
        frameHeaderSize = ZSTD_decodeFrameHeader_Part2(ctx, src, frameHeaderSize);
        if ZSTD_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
    }

    // Loop on each block
    loop {
        let mut decodedSize: size_t = 0;
        let cBlockSize = ZSTD_getcBlockSize(
            ip as *const c_void,
            iend.offset_from(ip) as size_t,
            &mut blockProperties,
        );
        if ZSTD_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                decodedSize = ZSTD_decompressBlock_internal(
                    ctx,
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_raw => {
                decodedSize = ZSTD_copyRawBlock(
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_rle => {
                return ERROR(ZSTD_error_GENERIC); // not yet supported
            }
            x if x == bt_end => {
                // end of frame
                if remainingSize != 0 {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC); // impossible
            }
        }
        if cBlockSize == 0 {
            break; // bt_end
        }

        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op.offset_from(ostart) as size_t
}

// ZSTD_errorFrameSizeInfoLegacy
pub unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut size_t,
    dBound: *mut core::ffi::c_ulonglong,
    ret: size_t,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

// ******************************
// Streaming Decompression API
// ******************************
pub unsafe fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> size_t {
    (*dctx).expected
}

pub unsafe fn ZSTD_decompressContinue(
    ctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    // Sanity check
    if srcSize != (*ctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ZSTD_checkContinuity(ctx, dst);

    // Decompress : frame header; part 1
    match (*ctx).stage {
        ZSTDds_getFrameHeaderSize => {
            // get frame header size
            if srcSize != ZSTD_frameHeaderSize_min {
                return ERROR(ZSTD_error_srcSize_wrong); // impossible
            }
            (*ctx).headerSize = ZSTD_decodeFrameHeader_Part1(ctx, src, ZSTD_frameHeaderSize_min);
            if ZSTD_isError((*ctx).headerSize) != 0 {
                return (*ctx).headerSize;
            }
            memcpy(
                (*ctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTD_frameHeaderSize_min,
            );
            if (*ctx).headerSize > ZSTD_frameHeaderSize_min {
                return ERROR(ZSTD_error_GENERIC); // impossible
            }
            (*ctx).expected = 0; // not necessary to copy more
            // fallthrough
            {
                let result = ZSTD_decodeFrameHeader_Part2(
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
        }
        ZSTDds_decodeFrameHeader => {
            // get frame header
            let result = ZSTD_decodeFrameHeader_Part2(
                ctx,
                (*ctx).headerBuffer.as_ptr() as *const c_void,
                (*ctx).headerSize,
            );
            if ZSTD_isError(result) != 0 {
                return result;
            }
            (*ctx).expected = ZSTD_blockHeaderSize;
            (*ctx).stage = ZSTDds_decodeBlockHeader;
            0
        }
        ZSTDds_decodeBlockHeader => {
            // Decode block header
            let mut bp: blockProperties_t = core::mem::zeroed();
            let blockSize = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
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
            0
        }
        ZSTDds_decompressBlock => {
            // Decompress : block content
            let rSize: size_t;
            match (*ctx).bType {
                x if x == bt_compressed => {
                    rSize = ZSTD_decompressBlock_internal(ctx, dst, maxDstSize, src, srcSize);
                }
                x if x == bt_raw => {
                    rSize = ZSTD_copyRawBlock(dst, maxDstSize, src, srcSize);
                }
                x if x == bt_rle => {
                    return ERROR(ZSTD_error_GENERIC); // not yet handled
                }
                x if x == bt_end => {
                    // should never happen (filtered at phase 1)
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
            (*ctx).previousDstEnd = (dst as *const c_char).add(rSize) as *const c_void;
            rSize
        }
        _ => ERROR(ZSTD_error_GENERIC), // impossible
    }
}

pub unsafe fn ZSTD_decompress_insertDictionary(
    ctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) {
    (*ctx).dictEnd = (*ctx).previousDstEnd;
    (*ctx).vBase = (dict as *const c_char).offset(
        -((*ctx).previousDstEnd as *const c_char).offset_from((*ctx).base as *const c_char),
    ) as *const c_void;
    (*ctx).base = dict;
    (*ctx).previousDstEnd = (dict as *const c_char).add(dictSize) as *const c_void;
}

// ============================================================================
// Buffered version of Zstd decompression (ZBUFF)
// ============================================================================
pub type ZBUFF_dStage = u32;
pub const ZBUFFds_init: ZBUFF_dStage = 0;
pub const ZBUFFds_readHeader: ZBUFF_dStage = 1;
pub const ZBUFFds_loadHeader: ZBUFF_dStage = 2;
pub const ZBUFFds_decodeHeader: ZBUFF_dStage = 3;
pub const ZBUFFds_read: ZBUFF_dStage = 4;
pub const ZBUFFds_load: ZBUFF_dStage = 5;
pub const ZBUFFds_flush: ZBUFF_dStage = 6;

#[repr(C)]
pub struct ZBUFFv04_DCtx_s {
    pub zc: *mut ZSTD_DCtx,
    pub params: ZSTD_parameters,
    pub inBuff: *mut c_char,
    pub inBuffSize: size_t,
    pub inPos: size_t,
    pub outBuff: *mut c_char,
    pub outBuffSize: size_t,
    pub outStart: size_t,
    pub outEnd: size_t,
    pub hPos: size_t,
    pub dict: *const c_char,
    pub dictSize: size_t,
    pub stage: ZBUFF_dStage,
    pub headerBuffer: [u8; ZSTD_frameHeaderSize_max],
}

pub type ZBUFF_DCtx = ZBUFFv04_DCtx_s;

pub unsafe fn ZBUFF_createDCtx() -> *mut ZBUFF_DCtx {
    let zbc = malloc(core::mem::size_of::<ZBUFF_DCtx>()) as *mut ZBUFF_DCtx;
    if zbc.is_null() {
        return core::ptr::null_mut();
    }
    memset(zbc as *mut c_void, 0, core::mem::size_of::<ZBUFF_DCtx>());
    (*zbc).zc = ZSTD_createDCtx();
    (*zbc).stage = ZBUFFds_init;
    zbc
}

pub unsafe fn ZBUFF_freeDCtx(zbc: *mut ZBUFF_DCtx) -> size_t {
    if zbc.is_null() {
        return 0; // support free on null
    }
    ZSTD_freeDCtx((*zbc).zc);
    free((*zbc).inBuff as *mut c_void);
    free((*zbc).outBuff as *mut c_void);
    free(zbc as *mut c_void);
    0
}

pub unsafe fn ZBUFF_decompressInit(zbc: *mut ZBUFF_DCtx) -> size_t {
    (*zbc).stage = ZBUFFds_readHeader;
    (*zbc).hPos = 0;
    (*zbc).inPos = 0;
    (*zbc).outStart = 0;
    (*zbc).outEnd = 0;
    (*zbc).dictSize = 0;
    ZSTD_resetDCtx((*zbc).zc)
}

pub unsafe fn ZBUFF_decompressWithDictionary(
    zbc: *mut ZBUFF_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    (*zbc).dict = src as *const c_char;
    (*zbc).dictSize = srcSize;
    0
}

pub unsafe fn ZBUFF_limitCopy(
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let length = MIN(maxDstSize, srcSize);
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

pub unsafe fn ZBUFF_decompressContinue(
    zbc: *mut ZBUFF_DCtx,
    dst: *mut c_void,
    maxDstSizePtr: *mut size_t,
    src: *const c_void,
    srcSizePtr: *mut size_t,
) -> size_t {
    let istart = src as *const c_char;
    let mut ip = istart;
    let iend = istart.add(*srcSizePtr);
    let ostart = dst as *mut c_char;
    let mut op = ostart;
    let oend = ostart.add(*maxDstSizePtr);
    let mut notDone: U32 = 1;

    // The C code is a `while (notDone) { switch (zbc->stage) { ... } }` with
    // several `/* fall-through */` edges between cases. We model the switch as
    // a labeled inner loop: each stage runs its body; a C `break` becomes
    // `break 'sw` (re-enters the outer while), and a fall-through becomes
    // advancing `stage` and NOT breaking (the inner loop re-dispatches).
    while notDone != 0 {
        'sw: loop {
            match (*zbc).stage {
                ZBUFFds_init => {
                    return ERROR(ZSTD_error_init_missing);
                }

                ZBUFFds_readHeader => {
                    // read header from src
                    let headerSize = ZSTD_getFrameParams(&mut (*zbc).params, src, *srcSizePtr);
                    if ZSTD_isError(headerSize) != 0 {
                        return headerSize;
                    }
                    if headerSize != 0 {
                        // not enough input to decode header
                        memcpy(
                            (*zbc).headerBuffer.as_mut_ptr().add((*zbc).hPos) as *mut c_void,
                            src,
                            *srcSizePtr,
                        );
                        (*zbc).hPos += *srcSizePtr;
                        *maxDstSizePtr = 0;
                        (*zbc).stage = ZBUFFds_loadHeader;
                        return headerSize - (*zbc).hPos;
                    }
                    (*zbc).stage = ZBUFFds_decodeHeader;
                    break 'sw; // C: `break;`
                }

                ZBUFFds_loadHeader => {
                    // complete header from src
                    let mut headerSize = ZBUFF_limitCopy(
                        (*zbc).headerBuffer.as_mut_ptr().add((*zbc).hPos) as *mut c_void,
                        ZSTD_frameHeaderSize_max - (*zbc).hPos,
                        src,
                        *srcSizePtr,
                    );
                    (*zbc).hPos += headerSize;
                    ip = ip.add(headerSize);
                    headerSize = ZSTD_getFrameParams(
                        &mut (*zbc).params,
                        (*zbc).headerBuffer.as_ptr() as *const c_void,
                        (*zbc).hPos,
                    );
                    if ZSTD_isError(headerSize) != 0 {
                        return headerSize;
                    }
                    if headerSize != 0 {
                        // not enough input to decode header
                        *maxDstSizePtr = 0;
                        return headerSize - (*zbc).hPos;
                    }
                    // intentional fallthrough to ZBUFFds_decodeHeader
                    (*zbc).stage = ZBUFFds_decodeHeader;
                    continue 'sw;
                }

                ZBUFFds_decodeHeader => {
                    // apply header to create / resize buffers
                    {
                        let neededOutSize: size_t = (1usize) << (*zbc).params.windowLog;
                        let neededInSize: size_t = BLOCKSIZE; // a block is never > BLOCKSIZE
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
                    if (*zbc).dictSize != 0 {
                        ZSTD_decompress_insertDictionary(
                            (*zbc).zc,
                            (*zbc).dict as *const c_void,
                            (*zbc).dictSize,
                        );
                    }
                    if (*zbc).hPos != 0 {
                        // some data already loaded into headerBuffer : transfer into inBuff
                        memcpy(
                            (*zbc).inBuff as *mut c_void,
                            (*zbc).headerBuffer.as_ptr() as *const c_void,
                            (*zbc).hPos,
                        );
                        (*zbc).inPos = (*zbc).hPos;
                        (*zbc).hPos = 0;
                        (*zbc).stage = ZBUFFds_load;
                        break 'sw; // C: `break;`
                    }
                    (*zbc).stage = ZBUFFds_read;
                    // fall-through to ZBUFFds_read
                    continue 'sw;
                }

                ZBUFFds_read => {
                    let neededInSize = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
                    if neededInSize == 0 {
                        // end of frame
                        (*zbc).stage = ZBUFFds_init;
                        notDone = 0;
                        break 'sw;
                    }
                    if (iend.offset_from(ip) as size_t) >= neededInSize {
                        // directly decode from src
                        let decodedSize = ZSTD_decompressContinue(
                            (*zbc).zc,
                            (*zbc).outBuff.add((*zbc).outStart) as *mut c_void,
                            (*zbc).outBuffSize - (*zbc).outStart,
                            ip as *const c_void,
                            neededInSize,
                        );
                        if ZSTD_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        ip = ip.add(neededInSize);
                        if decodedSize == 0 {
                            break 'sw; // this was just a header
                        }
                        (*zbc).outEnd = (*zbc).outStart + decodedSize;
                        (*zbc).stage = ZBUFFds_flush;
                        break 'sw;
                    }
                    if ip == iend {
                        notDone = 0;
                        break 'sw;
                    } // no more input
                    (*zbc).stage = ZBUFFds_load;
                    // fall-through to ZBUFFds_load
                    continue 'sw;
                }

                ZBUFFds_load => {
                    let neededInSize = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
                    let toLoad = neededInSize - (*zbc).inPos;
                    let loadedSize: size_t;
                    if toLoad > (*zbc).inBuffSize - (*zbc).inPos {
                        return ERROR(ZSTD_error_corruption_detected); // should never happen
                    }
                    loadedSize = ZBUFF_limitCopy(
                        (*zbc).inBuff.add((*zbc).inPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        iend.offset_from(ip) as size_t,
                    );
                    ip = ip.add(loadedSize);
                    (*zbc).inPos += loadedSize;
                    if loadedSize < toLoad {
                        notDone = 0;
                        break 'sw;
                    } // not enough input, wait for more
                    {
                        let decodedSize = ZSTD_decompressContinue(
                            (*zbc).zc,
                            (*zbc).outBuff.add((*zbc).outStart) as *mut c_void,
                            (*zbc).outBuffSize - (*zbc).outStart,
                            (*zbc).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ZSTD_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        (*zbc).inPos = 0; // input is consumed
                        if decodedSize == 0 {
                            (*zbc).stage = ZBUFFds_read;
                            break 'sw;
                        } // this was just a header
                        (*zbc).outEnd = (*zbc).outStart + decodedSize;
                        (*zbc).stage = ZBUFFds_flush;
                        // ZBUFFds_flush follows
                    }
                    // fall-through to ZBUFFds_flush
                    continue 'sw;
                }

                ZBUFFds_flush => {
                    let toFlushSize = (*zbc).outEnd - (*zbc).outStart;
                    let flushedSize = ZBUFF_limitCopy(
                        op as *mut c_void,
                        oend.offset_from(op) as size_t,
                        (*zbc).outBuff.add((*zbc).outStart) as *const c_void,
                        toFlushSize,
                    );
                    op = op.add(flushedSize);
                    (*zbc).outStart += flushedSize;
                    if flushedSize == toFlushSize {
                        (*zbc).stage = ZBUFFds_read;
                        if (*zbc).outStart + BLOCKSIZE > (*zbc).outBuffSize {
                            (*zbc).outStart = 0;
                            (*zbc).outEnd = 0;
                        }
                        break 'sw;
                    }
                    // cannot flush everything
                    notDone = 0;
                    break 'sw;
                }

                _ => return ERROR(ZSTD_error_GENERIC), // impossible
            }
        }
    }

    *srcSizePtr = ip.offset_from(istart) as size_t;
    *maxDstSizePtr = op.offset_from(ostart) as size_t;

    {
        let mut nextSrcSizeHint = ZSTD_nextSrcSizeToDecompress((*zbc).zc);
        if nextSrcSizeHint > 3 {
            nextSrcSizeHint += 3; // get the next block header while at it
        }
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbc).inPos); // already loaded
        nextSrcSizeHint
    }
}

// ============================================================================
// Tool functions + exported (no_mangle) wrappers
// ============================================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_isError(errorCode: size_t) -> u32 {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_getErrorName(errorCode: size_t) -> *const c_char {
    ERR_getErrorName(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_recommendedDInSize() -> size_t {
    BLOCKSIZE + 3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_recommendedDOutSize() -> size_t {
    BLOCKSIZE
}

// -*- final wrapping stage -*-

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_decompressDCtx(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_decompress_usingDict(dctx, dst, maxDstSize, src, srcSize, core::ptr::null(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_decompress(
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    // ZSTD_HEAPMODE == 1
    let regenSize: size_t;
    let dctx = ZSTD_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv04_decompressDCtx(dctx, dst, maxDstSize, src, srcSize);
    ZSTD_freeDCtx(dctx);
    regenSize
}

// ZSTDv04_isError is `static unsigned ZSTD_isError` in C -> NOT exported.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: size_t,
    cSize: *mut size_t,
    dBound: *mut core::ffi::c_ulonglong,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: size_t = 0;
    let mut blockProperties: blockProperties_t = core::mem::zeroed();

    // Frame Header
    if srcSize < ZSTD_frameHeaderSize_min {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    if MEM_readLE32(src) != ZSTD_MAGICNUMBER {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
        return;
    }
    ip = ip.add(ZSTD_frameHeaderSize_min);
    remainingSize -= ZSTD_frameHeaderSize_min;

    // Loop on each block
    loop {
        let cBlockSize =
            ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break; // bt_end
        }

        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = ip.offset_from(src as *const BYTE) as size_t;
    *dBound = (nbBlocks * BLOCKSIZE) as core::ffi::c_ulonglong;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_resetDCtx(dctx: *mut ZSTDv04_Dctx_s) -> size_t {
    ZSTD_resetDCtx(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_nextSrcSizeToDecompress(dctx: *mut ZSTDv04_Dctx_s) -> size_t {
    ZSTD_nextSrcSizeToDecompress(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_decompressContinue(
    dctx: *mut ZSTDv04_Dctx_s,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_decompressContinue(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_createDCtx() -> *mut ZBUFFv04_DCtx_s {
    ZBUFF_createDCtx()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_freeDCtx(dctx: *mut ZBUFFv04_DCtx_s) -> size_t {
    ZBUFF_freeDCtx(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressInit(dctx: *mut ZBUFFv04_DCtx_s) -> size_t {
    ZBUFF_decompressInit(dctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressWithDictionary(
    dctx: *mut ZBUFFv04_DCtx_s,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZBUFF_decompressWithDictionary(dctx, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv04_decompressContinue(
    dctx: *mut ZBUFFv04_DCtx_s,
    dst: *mut c_void,
    maxDstSizePtr: *mut size_t,
    src: *const c_void,
    srcSizePtr: *mut size_t,
) -> size_t {
    ZBUFF_decompressContinue(dctx, dst, maxDstSizePtr, src, srcSizePtr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_createDCtx() -> *mut ZSTD_DCtx {
    ZSTD_createDCtx()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv04_freeDCtx(dctx: *mut ZSTD_DCtx) -> size_t {
    ZSTD_freeDCtx(dctx)
}
