//! Translation of `legacy/zstd_v01.c` (+ `legacy/zstd_v01.h`).
//!
//! This C file is entirely self contained: it carries its own private copies of
//! the memory accessors, the `FSE_*` bitstream reader / entropy decoder, the
//! `HUF_*` (Huff0, v0.1 flavour) decoder and the `ZSTD_*` v0.1 frame/block
//! decoder.  Everything except the nine `ZSTDv01_*` entry points was `static`
//! in C, so everything else is private to this module.
//!
//! The only shared header it `#include`s is `common/error_private.h`, used here
//! through `crate::common::error_private`.

use crate::common::error_private::{
    ERROR, ERR_isError, ZSTD_error_GENERIC, ZSTD_error_corruption_detected,
    ZSTD_error_dstSize_tooSmall, ZSTD_error_prefix_unknown, ZSTD_error_srcSize_wrong,
};
use crate::libc::{free, malloc, memcpy, memmove, memset, ZSTD_memmove};
use core::ffi::{c_char, c_int, c_ulonglong, c_void};

/* ******************************************************************
 *  Basic types
 ********************************************************************/

pub type BYTE = u8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

/* ******************************************************************
 *  Static allocation
 ********************************************************************/

/* #define FSE_DTABLE_SIZE_U32(maxTableLog)  (1 + (1<<maxTableLog)) */
const fn FSE_DTABLE_SIZE_U32(maxTableLog: usize) -> usize {
    1 + (1usize << maxTableLog)
}

/* #define HUF_DTABLE_SIZE_U16(maxTableLog)  (1 + (1<<maxTableLog)) */
const fn HUF_DTABLE_SIZE_U16(maxTableLog: usize) -> usize {
    1 + (1usize << maxTableLog)
}

/* ******************************************************************
 *  Error Management
 ********************************************************************/

/* typedef enum { FSE_LIST_ERRORS(FSE_GENERATE_ENUM) } FSE_errorCodes; */
const FSE_OK_NoError: usize = 0;
const FSE_ERROR_GENERIC: usize = 1;
const FSE_ERROR_tableLog_tooLarge: usize = 2;
const FSE_ERROR_maxSymbolValue_tooLarge: usize = 3;
const FSE_ERROR_maxSymbolValue_tooSmall: usize = 4;
const FSE_ERROR_dstSize_tooSmall: usize = 5;
const FSE_ERROR_srcSize_wrong: usize = 6;
const FSE_ERROR_corruptionDetected: usize = 7;
const FSE_ERROR_maxCode: usize = 8;

/* `(size_t)-FSE_ERROR_xxx` */
const fn FSE_ERR(code: usize) -> usize {
    (0usize).wrapping_sub(code)
}

/* ******************************************************************
 *  FSE symbol compression API
 ********************************************************************/

pub type FSE_CTable = u32;
pub type FSE_DTable = u32;

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_CStream_t {
    bitContainer: usize,
    bitPos: c_int,
    startPtr: *mut c_char,
    ptr: *mut c_char,
    endPtr: *mut c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_CState_t {
    value: isize,
    stateTable: *const c_void,
    symbolTT: *const c_void,
    stateLog: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_DStream_t {
    bitContainer: usize,
    bitsConsumed: u32,
    ptr: *const c_char,
    start: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_DState_t {
    state: usize,
    table: *const c_void, /* precise table may vary, depending on U16 */
}

/* FSE_DStream_status : result of FSE_reloadDStream() */
const FSE_DStream_unfinished: U32 = 0;
const FSE_DStream_endOfBuffer: U32 = 1;
const FSE_DStream_completed: U32 = 2;
const FSE_DStream_tooFar: U32 = 3;

/* ******************************************************************
 *  Tuning parameters
 ********************************************************************/

const FSE_MAX_MEMORY_USAGE: usize = 14;
const FSE_DEFAULT_MEMORY_USAGE: usize = 13;

const FSE_MAX_SYMBOL_VALUE: usize = 255;

/* ******************************************************************
 *  Byte symbol type
 ********************************************************************/

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_decode_t {
    /* size == U32 */
    newState: U16,
    symbol: u8,
    nbBits: u8,
}

/* ******************************************************************
 *  Memory I/O
 ********************************************************************/

unsafe fn FSE_32bits() -> u32 {
    (core::mem::size_of::<*const c_void>() == 4) as u32
}

unsafe fn FSE_isLittleEndian() -> u32 {
    /* const union { U32 i; BYTE c[4]; } one = { 1 }; return one.c[0]; */
    1
}

unsafe fn FSE_read16(memPtr: *const c_void) -> U16 {
    (memPtr as *const U16).read_unaligned()
}

unsafe fn FSE_read32(memPtr: *const c_void) -> U32 {
    (memPtr as *const U32).read_unaligned()
}

unsafe fn FSE_read64(memPtr: *const c_void) -> U64 {
    (memPtr as *const U64).read_unaligned()
}

unsafe fn FSE_readLE16(memPtr: *const c_void) -> U16 {
    if FSE_isLittleEndian() != 0 {
        FSE_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}

unsafe fn FSE_readLE32(memPtr: *const c_void) -> U32 {
    if FSE_isLittleEndian() != 0 {
        FSE_read32(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U32)
            .wrapping_add((*p.add(1) as U32) << 8)
            .wrapping_add((*p.add(2) as U32) << 16)
            .wrapping_add((*p.add(3) as U32) << 24)
    }
}

unsafe fn FSE_readLE64(memPtr: *const c_void) -> U64 {
    if FSE_isLittleEndian() != 0 {
        FSE_read64(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U64)
            .wrapping_add((*p.add(1) as U64) << 8)
            .wrapping_add((*p.add(2) as U64) << 16)
            .wrapping_add((*p.add(3) as U64) << 24)
            .wrapping_add((*p.add(4) as U64) << 32)
            .wrapping_add((*p.add(5) as U64) << 40)
            .wrapping_add((*p.add(6) as U64) << 48)
            .wrapping_add((*p.add(7) as U64) << 56)
    }
}

unsafe fn FSE_readLEST(memPtr: *const c_void) -> usize {
    if FSE_32bits() != 0 {
        FSE_readLE32(memPtr) as usize
    } else {
        FSE_readLE64(memPtr) as usize
    }
}

/* ******************************************************************
 *  Constants
 ********************************************************************/

const FSE_MAX_TABLELOG: usize = FSE_MAX_MEMORY_USAGE - 2;
const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
const FSE_DEFAULT_TABLELOG: usize = FSE_DEFAULT_MEMORY_USAGE - 2;
const FSE_MIN_TABLELOG: u32 = 5;

const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

/* ******************************************************************
 *  Complex types
 ********************************************************************/

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_symbolCompressionTransform {
    /* total 8 bytes */
    deltaFindState: c_int,
    deltaNbBits: U32,
}

type DTable_max_t = [U32; FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)];

/* ******************************************************************
 *  Internal functions
 ********************************************************************/

unsafe fn FSE_highbit32(val: U32) -> u32 {
    /* __builtin_clz (val) ^ 31 */
    val.leading_zeros() ^ 31
}

/* ******************************************************************
 *  Templates
 ********************************************************************/

unsafe fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1)
        .wrapping_add(tableSize >> 3)
        .wrapping_add(3)
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FSE_DTableHeader {
    /* sizeof U32 */
    tableLog: U16,
    fastMode: U16,
}

unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    /* because dt is unsigned, 32-bits aligned on 32-bits */
    let tableDecode = (ptr as *mut FSE_decode_t).wrapping_add(1);
    let tableSize: U32 = 1u32 << tableLog;
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; FSE_MAX_SYMBOL_VALUE + 1] = [0; FSE_MAX_SYMBOL_VALUE + 1];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);
    let largeLimit: S16 = (1i32 << (tableLog.wrapping_sub(1) & 31)) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    /* Sanity Checks */
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE as u32 {
        return FSE_ERR(FSE_ERROR_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG as u32 {
        return FSE_ERR(FSE_ERROR_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    (*DTableH.add(0)).tableLog = tableLog as U16;
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
                /* lowprob area */
                position = position.wrapping_add(step) & tableMask;
            }
            i += 1;
        }
        s = s.wrapping_add(1);
    }

    /* position must reach all cells once, otherwise normalizedCounter is incorrect */
    if position != 0 {
        return FSE_ERR(FSE_ERROR_GENERIC);
    }

    /* Build Decoding table */
    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = nextState.wrapping_add(1);
            (*tableDecode.add(i as usize)).nbBits =
                tableLog.wrapping_sub(FSE_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(i as usize)).newState = (((nextState as U32)
                << ((*tableDecode.add(i as usize)).nbBits as U32))
                .wrapping_sub(tableSize)) as U16;
            i = i.wrapping_add(1);
        }
    }

    (*DTableH).fastMode = noLarge as U16;
    0
}

/* ******************************************************************
 *  FSE byte symbol
 ********************************************************************/

unsafe fn FSE_isError(code: usize) -> u32 {
    (code > FSE_ERR(FSE_ERROR_maxCode)) as u32
}

unsafe fn FSE_abs(a: S16) -> S16 {
    if a < 0 {
        a.wrapping_neg()
    } else {
        a
    }
}

/* ******************************************************************
 *  Header bitstream management
 ********************************************************************/

unsafe fn FSE_readNCount(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
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
    let mut charnum: u32 = 0;
    let mut previous0: c_int = 0;

    if hbSize < 4 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    bitStream = FSE_readLE32(ip as *const c_void);
    /* extract tableLog */
    nbBits = ((bitStream & 0xF).wrapping_add(FSE_MIN_TABLELOG)) as c_int;
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as c_int {
        return FSE_ERR(FSE_ERROR_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as u32;
    remaining = (1i32 << nbBits).wrapping_add(1);
    threshold = 1i32 << nbBits;
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0: u32 = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 = n0.wrapping_add(24);
                if ip < iend.wrapping_sub(5) {
                    ip = ip.wrapping_add(2);
                    bitStream = FSE_readLE32(ip as *const c_void) >> bitCount;
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
                return FSE_ERR(FSE_ERROR_maxSymbolValue_tooSmall);
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
                bitStream = FSE_readLE32(ip as *const c_void) >> bitCount;
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: S16 = ((2 * threshold - 1) - remaining) as S16;
            let mut count: S16;

            if (bitStream & (threshold - 1) as U32) < max as U32 {
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
            remaining -= FSE_abs(count) as c_int;
            *normalizedCounter.add(charnum as usize) = count;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as c_int;
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
                    bitCount -= (8usize
                        .wrapping_mul((iend.wrapping_sub(4) as usize).wrapping_sub(ip as usize)))
                        as c_int;
                    ip = iend.wrapping_sub(4);
                }
                bitStream = FSE_readLE32(ip as *const c_void) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return FSE_ERR(FSE_ERROR_GENERIC);
    }
    *maxSVPtr = charnum.wrapping_sub(1);

    ip = ip.wrapping_offset(((bitCount + 7) >> 3) as isize);
    if ((ip as usize).wrapping_sub(istart as usize)) > hbSize {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    (ip as usize).wrapping_sub(istart as usize)
}

/* ******************************************************************
 *  Decompression (Byte symbols)
 ********************************************************************/

unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    /* because dt is unsigned */
    let cell = (ptr as *mut FSE_decode_t).wrapping_add(1);

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: u32) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    /* because dt is unsigned */
    let dinfo = (ptr as *mut FSE_decode_t).wrapping_add(1);
    let tableSize: u32 = 1u32 << nbBits;
    let tableMask: u32 = tableSize.wrapping_sub(1);
    let maxSymbolValue: u32 = tableMask;
    let mut s: u32;

    /* Sanity checks */
    if nbBits < 1 {
        /* min size */
        return FSE_ERR(FSE_ERROR_GENERIC);
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

/* FSE_initDStream
 * Initialize a FSE_DStream_t.
 * srcBuffer must point at the beginning of an FSE block.
 * The function result is the size of the FSE_block (== srcSize).
 * If srcSize is too small, the function will return an errorCode;
 */
unsafe fn FSE_initDStream(
    bitD: *mut FSE_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char)
            .wrapping_add(srcSize)
            .wrapping_sub(core::mem::size_of::<usize>());
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        contain32 = *(srcBuffer as *const BYTE).wrapping_add(srcSize.wrapping_sub(1)) as U32;
        if contain32 == 0 {
            /* stop bit not present */
            return FSE_ERR(FSE_ERROR_GENERIC);
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(FSE_highbit32(contain32));
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        /* switch(srcSize) with fallthrough, srcSize is in [1..7] here */
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*((*bitD).start as *const BYTE).add(6) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 16),
            );
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*((*bitD).start as *const BYTE).add(5) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 24),
            );
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*((*bitD).start as *const BYTE).add(4) as usize)
                    << (core::mem::size_of::<usize>() * 8 - 32),
            );
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*((*bitD).start as *const BYTE).add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*((*bitD).start as *const BYTE).add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*((*bitD).start as *const BYTE).add(1) as usize) << 8);
        }
        contain32 = *(srcBuffer as *const BYTE).wrapping_add(srcSize.wrapping_sub(1)) as U32;
        if contain32 == 0 {
            /* stop bit not present */
            return FSE_ERR(FSE_ERROR_GENERIC);
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(FSE_highbit32(contain32));
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(
            ((core::mem::size_of::<usize>().wrapping_sub(srcSize)) as U32).wrapping_mul(8),
        );
    }

    srcSize
}

/* FSE_lookBits
 * Provides next n bits from the bitContainer.
 * bitContainer is not modified (bits are still present for next read/look)
 * On 32-bits, maxNbBits==25
 * On 64-bits, maxNbBits==57
 * return : value extracted.
 */
unsafe fn FSE_lookBits(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> (bitMask.wrapping_sub(nbBits) & bitMask)
}

/* only if nbBits >= 1 !! */
unsafe fn FSE_lookBitsFast(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((bitMask.wrapping_add(1).wrapping_sub(nbBits)) & bitMask)
}

unsafe fn FSE_skipBits(bitD: *mut FSE_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

/* FSE_readBits
 * Read next n bits from the bitContainer.
 * On 32-bits, don't read more than maxNbBits==25
 * On 64-bits, don't read more than maxNbBits==57
 * Use the fast variant *only* if n >= 1.
 * return : value extracted.
 */
unsafe fn FSE_readBits(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let value = FSE_lookBits(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    value
}

/* only if nbBits >= 1 !! */
unsafe fn FSE_readBitsFast(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let value = FSE_lookBitsFast(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    value
}

unsafe fn FSE_reloadDStream(bitD: *mut FSE_DStream_t) -> u32 {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as u32 {
        /* should never happen */
        return FSE_DStream_tooFar;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD)
            .ptr
            .wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        return FSE_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as u32 {
            return FSE_DStream_endOfBuffer;
        }
        return FSE_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: U32 = FSE_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            /* ptr > start */
            nbBytes = ((*bitD).ptr as usize).wrapping_sub((*bitD).start as usize) as U32;
            result = FSE_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        /* reminder : srcSize > sizeof(bitD) */
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut FSE_DStream_t,
    dt: *const FSE_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSE_DTableHeader;
    (*DStatePtr).state = FSE_readBits(bitD, (*DTableH).tableLog as U32);
    FSE_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut FSE_DStream_t) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = FSE_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut FSE_DStream_t) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = FSE_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* FSE_endOfDStream
Tells if bitD has reached end of bitStream or not */
unsafe fn FSE_endOfDStream(bitD: *const FSE_DStream_t) -> u32 {
    (((*bitD).ptr == (*bitD).start)
        && ((*bitD).bitsConsumed == (core::mem::size_of::<usize>() * 8) as u32)) as u32
}

unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

/* #define FSE_GETSYMBOL(statePtr) fast ? FSE_decodeSymbolFast(statePtr, &bitD) : FSE_decodeSymbol(statePtr, &bitD) */
unsafe fn FSE_GETSYMBOL(
    statePtr: *mut FSE_DState_t,
    bitD: *mut FSE_DStream_t,
    fast: u32,
) -> BYTE {
    if fast != 0 {
        FSE_decodeSymbolFast(statePtr, bitD)
    } else {
        FSE_decodeSymbol(statePtr, bitD)
    }
}

unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: u32,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.wrapping_add(maxDstSize);
    let olimit = omax.wrapping_sub(3);

    let mut bitD = FSE_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1 = FSE_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2 = FSE_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut errorCode: usize;

    /* Init */
    /* replaced last arg by maxCompressed Size */
    errorCode = FSE_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    /* 4 symbols per loop */
    while (FSE_reloadDStream(&mut bitD) == FSE_DStream_unfinished) && (op < olimit) {
        *op.add(0) = FSE_GETSYMBOL(&mut state1, &mut bitD, fast);

        /* This test must be static */
        if FSE_MAX_TABLELOG * 2 + 7 > core::mem::size_of::<usize>() * 8 {
            FSE_reloadDStream(&mut bitD);
        }

        *op.add(1) = FSE_GETSYMBOL(&mut state2, &mut bitD, fast);

        /* This test must be static */
        if FSE_MAX_TABLELOG * 4 + 7 > core::mem::size_of::<usize>() * 8 {
            if FSE_reloadDStream(&mut bitD) > FSE_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.add(2) = FSE_GETSYMBOL(&mut state1, &mut bitD, fast);

        /* This test must be static */
        if FSE_MAX_TABLELOG * 2 + 7 > core::mem::size_of::<usize>() * 8 {
            FSE_reloadDStream(&mut bitD);
        }

        *op.add(3) = FSE_GETSYMBOL(&mut state2, &mut bitD, fast);

        op = op.wrapping_add(4);
    }

    /* tail */
    /* note : FSE_reloadDStream(&bitD) >= FSE_DStream_partiallyFilled; Ends at exactly FSE_DStream_completed */
    loop {
        if (FSE_reloadDStream(&mut bitD) > FSE_DStream_completed)
            || (op == omax)
            || (FSE_endOfDStream(&bitD) != 0 && (fast != 0 || FSE_endOfDState(&state1) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL(&mut state1, &mut bitD, fast);
        op = op.wrapping_add(1);

        if (FSE_reloadDStream(&mut bitD) > FSE_DStream_completed)
            || (op == omax)
            || (FSE_endOfDStream(&bitD) != 0 && (fast != 0 || FSE_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL(&mut state2, &mut bitD, fast);
        op = op.wrapping_add(1);
    }

    /* end ? */
    if FSE_endOfDStream(&bitD) != 0
        && FSE_endOfDState(&state1) != 0
        && FSE_endOfDState(&state2) != 0
    {
        return (op as usize).wrapping_sub(ostart as usize);
    }

    if op == omax {
        /* dst buffer is full, but cSrc unfinished */
        return FSE_ERR(FSE_ERROR_dstSize_tooSmall);
    }

    FSE_ERR(FSE_ERROR_corruptionDetected)
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
    /* memcpy() into local variable, to avoid strict aliasing warning */
    memcpy(
        &mut DTableH as *mut FSE_DTableHeader as *mut c_void,
        dt as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );

    /* select fast mode (static) */
    if DTableH.fastMode != 0 {
        return FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

unsafe fn FSE_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [S16; FSE_MAX_SYMBOL_VALUE + 1] = [0; FSE_MAX_SYMBOL_VALUE + 1];
    /* Static analyzer seems unable to understand this table will be properly initialized later */
    let mut dt: DTable_max_t = [0; FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE as u32;
    let mut errorCode: usize;

    if cSrcSize < 2 {
        /* too small input size */
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
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
        /* too small input size */
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
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

/* *******************************************************
 *  Huff0 : Huffman block compression
 *********************************************************/

const HUF_MAX_SYMBOL_VALUE: usize = 255;
const HUF_DEFAULT_TABLELOG: usize = 12;
const HUF_MAX_TABLELOG: usize = 12;
const HUF_ABSOLUTEMAX_TABLELOG: usize = 16;

#[repr(C)]
#[derive(Copy, Clone)]
struct HUF_CElt {
    val: U16,
    nbBits: BYTE,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct nodeElt {
    count: U32,
    parent: U16,
    byte: BYTE,
    nbBits: BYTE,
}

/* *******************************************************
 *  Huff0 : Huffman block decompression
 *********************************************************/

#[repr(C)]
#[derive(Copy, Clone)]
struct HUF_DElt {
    byte: BYTE,
    nbBits: BYTE,
}

unsafe fn HUF_readDTable(DTable: *mut U16, src: *const c_void, srcSize: usize) -> usize {
    let mut huffWeight: [BYTE; HUF_MAX_SYMBOL_VALUE + 1] = [0; HUF_MAX_SYMBOL_VALUE + 1];
    /* large enough for values from 0 to 16 */
    let mut rankVal: [U32; HUF_ABSOLUTEMAX_TABLELOG + 1] = [0; HUF_ABSOLUTEMAX_TABLELOG + 1];
    let mut weightTotal: U32;
    let maxBits: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: usize;
    let mut oSize: usize;
    let mut n: U32;
    let mut nextRankStart: U32;
    let ptr = DTable.wrapping_add(1) as *mut c_void;
    let dt = ptr as *mut HUF_DElt;

    if srcSize == 0 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            static l: [c_int; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = l[iSize - 242] as usize;
            memset(
                huffWeight.as_mut_ptr() as *mut c_void,
                1,
                core::mem::size_of::<[BYTE; HUF_MAX_SYMBOL_VALUE + 1]>(),
            );
            iSize = 0;
        } else {
            /* Incompressible */
            oSize = iSize - 127;
            iSize = (oSize + 1) / 2;
            if iSize + 1 > srcSize {
                return FSE_ERR(FSE_ERROR_srcSize_wrong);
            }
            ip = ip.wrapping_add(1);
            n = 0;
            while (n as usize) < oSize {
                huffWeight[n as usize] = *ip.add((n / 2) as usize) >> 4;
                huffWeight[n as usize + 1] = *ip.add((n / 2) as usize) & 15;
                n = n.wrapping_add(2);
            }
        }
    } else {
        /* header compressed with FSE (normal case) */
        if iSize + 1 > srcSize {
            return FSE_ERR(FSE_ERROR_srcSize_wrong);
        }
        /* max 255 values decoded, last one is implied */
        oSize = FSE_decompress(
            huffWeight.as_mut_ptr() as *mut c_void,
            HUF_MAX_SYMBOL_VALUE,
            ip.wrapping_add(1) as *const c_void,
            iSize,
        );
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankVal.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of::<[U32; HUF_ABSOLUTEMAX_TABLELOG + 1]>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if huffWeight[n as usize] as usize >= HUF_ABSOLUTEMAX_TABLELOG {
            return FSE_ERR(FSE_ERROR_corruptionDetected);
        }
        rankVal[huffWeight[n as usize] as usize] =
            rankVal[huffWeight[n as usize] as usize].wrapping_add(1);
        weightTotal =
            weightTotal.wrapping_add((1u32 << huffWeight[n as usize] as U32) >> 1);
        n = n.wrapping_add(1);
    }
    if weightTotal == 0 {
        return FSE_ERR(FSE_ERROR_corruptionDetected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    maxBits = FSE_highbit32(weightTotal).wrapping_add(1);
    if maxBits > *DTable.add(0) as U32 {
        /* DTable is too small */
        return FSE_ERR(FSE_ERROR_tableLog_tooLarge);
    }
    *DTable.add(0) = maxBits as U16;
    {
        let total: U32 = 1u32 << maxBits;
        let rest: U32 = total.wrapping_sub(weightTotal);
        let verif: U32 = 1u32 << FSE_highbit32(rest);
        let lastWeight: U32 = FSE_highbit32(rest).wrapping_add(1);
        if verif != rest {
            /* last value must be a clean power of 2 */
            return FSE_ERR(FSE_ERROR_corruptionDetected);
        }
        huffWeight[oSize] = lastWeight as BYTE;
        rankVal[lastWeight as usize] = rankVal[lastWeight as usize].wrapping_add(1);
    }

    /* check tree construction validity */
    if (rankVal[1] < 2) || (rankVal[1] & 1) != 0 {
        /* by construction : at least 2 elts of rank 1, must be even */
        return FSE_ERR(FSE_ERROR_corruptionDetected);
    }

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= maxBits {
        let current: U32 = nextRankStart;
        nextRankStart = nextRankStart.wrapping_add(rankVal[n as usize] << (n.wrapping_sub(1)));
        rankVal[n as usize] = current;
        n = n.wrapping_add(1);
    }

    /* fill DTable */
    n = 0;
    while (n as usize) <= oSize {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32 << w) >> 1;
        let mut i: U32;
        let mut D = HUF_DElt { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = maxBits.wrapping_add(1).wrapping_sub(w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize].wrapping_add(length) {
            *dt.add(i as usize) = D;
            i = i.wrapping_add(1);
        }
        rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
        n = n.wrapping_add(1);
    }

    iSize + 1
}

unsafe fn HUF_decodeSymbol(
    Dstream: *mut FSE_DStream_t,
    dt: *const HUF_DElt,
    dtLog: U32,
) -> BYTE {
    /* note : dtLog >= 1 */
    let val: usize = FSE_lookBitsFast(Dstream, dtLog);
    let c: BYTE = (*dt.add(val)).byte;
    FSE_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

/* -3% slower when non static */
unsafe fn HUF_decompress_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    if cSrcSize < 6 {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    {
        let ostart = dst as *mut BYTE;
        let mut op = ostart;
        let omax = op.wrapping_add(maxDstSize);
        let olimit = if maxDstSize < 15 {
            op
        } else {
            omax.wrapping_sub(15)
        };

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DElt).wrapping_add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;
        let mut reloadStatus: U32;

        /* Init */

        let jumpTable = cSrc as *const U16;
        let length1: usize = FSE_readLE16(jumpTable as *const c_void) as usize;
        let length2: usize = FSE_readLE16(jumpTable.wrapping_add(1) as *const c_void) as usize;
        let length3: usize = FSE_readLE16(jumpTable.wrapping_add(2) as *const c_void) as usize;
        /* check coherency !! */
        let length4: usize = cSrcSize
            .wrapping_sub(6)
            .wrapping_sub(length1)
            .wrapping_sub(length2)
            .wrapping_sub(length3);
        let start1 = (cSrc as *const c_char).wrapping_add(6);
        let start2 = start1.wrapping_add(length1);
        let start3 = start2.wrapping_add(length2);
        let start4 = start3.wrapping_add(length3);
        let mut bitD1 = FSE_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = bitD1;
        let mut bitD3 = bitD1;
        let mut bitD4 = bitD1;

        if length1.wrapping_add(length2).wrapping_add(length3).wrapping_add(6) >= cSrcSize {
            return FSE_ERR(FSE_ERROR_srcSize_wrong);
        }

        errorCode = FSE_initDStream(&mut bitD1, start1 as *const c_void, length1);
        if FSE_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = FSE_initDStream(&mut bitD2, start2 as *const c_void, length2);
        if FSE_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = FSE_initDStream(&mut bitD3, start3 as *const c_void, length3);
        if FSE_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = FSE_initDStream(&mut bitD4, start4 as *const c_void, length4);
        if FSE_isError(errorCode) != 0 {
            return errorCode;
        }

        reloadStatus = FSE_reloadDStream(&mut bitD2);

        /* 16 symbols per loop */
        /* D2-3-4 are supposed to be synchronized and finish together */
        while (reloadStatus < FSE_DStream_completed) && (op < olimit) {
            /* HUF_DECODE_SYMBOL_1 : the `FSE_32bits() && (HUF_MAX_TABLELOG>12)` reload is
             * statically disabled here; likewise HUF_DECODE_SYMBOL_2's `FSE_32bits()` one. */
            *op.add(0) = HUF_decodeSymbol(&mut bitD1, dt, dtLog);
            *op.add(1) = HUF_decodeSymbol(&mut bitD2, dt, dtLog);
            *op.add(2) = HUF_decodeSymbol(&mut bitD3, dt, dtLog);
            *op.add(3) = HUF_decodeSymbol(&mut bitD4, dt, dtLog);
            *op.add(4) = HUF_decodeSymbol(&mut bitD1, dt, dtLog);
            *op.add(5) = HUF_decodeSymbol(&mut bitD2, dt, dtLog);
            *op.add(6) = HUF_decodeSymbol(&mut bitD3, dt, dtLog);
            *op.add(7) = HUF_decodeSymbol(&mut bitD4, dt, dtLog);
            *op.add(8) = HUF_decodeSymbol(&mut bitD1, dt, dtLog);
            *op.add(9) = HUF_decodeSymbol(&mut bitD2, dt, dtLog);
            *op.add(10) = HUF_decodeSymbol(&mut bitD3, dt, dtLog);
            *op.add(11) = HUF_decodeSymbol(&mut bitD4, dt, dtLog);
            *op.add(12) = HUF_decodeSymbol(&mut bitD1, dt, dtLog);
            *op.add(13) = HUF_decodeSymbol(&mut bitD2, dt, dtLog);
            *op.add(14) = HUF_decodeSymbol(&mut bitD3, dt, dtLog);
            *op.add(15) = HUF_decodeSymbol(&mut bitD4, dt, dtLog);

            op = op.wrapping_add(16);
            reloadStatus = FSE_reloadDStream(&mut bitD2)
                | FSE_reloadDStream(&mut bitD3)
                | FSE_reloadDStream(&mut bitD4);
            FSE_reloadDStream(&mut bitD1);
        }

        if reloadStatus != FSE_DStream_completed {
            /* not complete : some bitStream might be FSE_DStream_unfinished */
            return FSE_ERR(FSE_ERROR_corruptionDetected);
        }

        /* tail */
        {
            /* bitTail = bitD1; */
            /* *much* slower : -20% !??! */
            let mut bitTail = FSE_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            };
            bitTail.ptr = bitD1.ptr;
            bitTail.bitsConsumed = bitD1.bitsConsumed;
            /* required in case of FSE_DStream_endOfBuffer */
            bitTail.bitContainer = bitD1.bitContainer;
            bitTail.start = start1;
            while (FSE_reloadDStream(&mut bitTail) < FSE_DStream_completed) && (op < omax) {
                *op.add(0) = HUF_decodeSymbol(&mut bitTail, dt, dtLog);
                op = op.wrapping_add(1);
            }

            if FSE_endOfDStream(&bitTail) != 0 {
                return (op as usize).wrapping_sub(ostart as usize);
            }
        }

        if op == omax {
            /* dst buffer is full, but cSrc unfinished */
            return FSE_ERR(FSE_ERROR_dstSize_tooSmall);
        }

        FSE_ERR(FSE_ERROR_corruptionDetected)
    }
}

unsafe fn HUF_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
) -> usize {
    /* HUF_CREATE_STATIC_DTABLE(DTable, HUF_MAX_TABLELOG) :
     * unsigned short DTable[HUF_DTABLE_SIZE_U16(HUF_MAX_TABLELOG)] = { HUF_MAX_TABLELOG } */
    let mut DTable: [U16; HUF_DTABLE_SIZE_U16(HUF_MAX_TABLELOG)] =
        [0; HUF_DTABLE_SIZE_U16(HUF_MAX_TABLELOG)];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let errorCode: usize;

    errorCode = HUF_readDTable(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return FSE_ERR(FSE_ERROR_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize = cSrcSize.wrapping_sub(errorCode);

    HUF_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const c_void,
        cSrcSize,
        DTable.as_ptr(),
    )
}

/* ******************************************************************
 *  zstd - standard compression library (v0.1 decoder)
 ********************************************************************/

/* MEMORY_USAGE */
const ZSTD_MEMORY_USAGE: usize = 17;

/* ******************************************************************
 *  Constants
 ********************************************************************/

/* 3rd version : seqNb header */
static ZSTD_magicNumber: U32 = 0xFD2FB51E;

const HASH_LOG: usize = ZSTD_MEMORY_USAGE - 2;
const HASH_TABLESIZE: usize = 1usize << HASH_LOG;
const HASH_MASK: usize = HASH_TABLESIZE - 1;

const KNUTH: u32 = 2654435761;

const BIT7: c_int = 128;
const BIT6: c_int = 64;
const BIT5: c_int = 32;
const BIT4: c_int = 16;

/* define, for static allocation */
const BLOCKSIZE: usize = 128 * (1 << 10);

const WORKPLACESIZE: usize = BLOCKSIZE * 3;
const MINMATCH: usize = 4;
const MLbits: u32 = 7;
const LLbits: u32 = 6;
const Offbits: u32 = 5;
const MaxML: usize = (1usize << MLbits) - 1;
const MaxLL: usize = (1usize << LLbits) - 1;
const MaxOff: usize = (1usize << Offbits) - 1;
const LitFSELog: u32 = 11;
const MLFSELog: usize = 10;
const LLFSELog: usize = 10;
const OffFSELog: usize = 9;
const MaxSeq: usize = if MaxLL < MaxML { MaxML } else { MaxLL };

const LITERAL_NOENTROPY: usize = 63;
/* to remove */
const COMMAND_NOENTROPY: usize = 7;

const ZSTD_CONTENTSIZE_ERROR: u64 = (0u64).wrapping_sub(2);

static ZSTD_blockHeaderSize: usize = 3;
static ZSTD_frameHeaderSize: usize = 4;

/* ******************************************************************
 *  Memory operations
 ********************************************************************/

unsafe fn ZSTD_32bits() -> u32 {
    (core::mem::size_of::<*const c_void>() == 4) as u32
}

unsafe fn ZSTD_isLittleEndian() -> u32 {
    /* const union { U32 i; BYTE c[4]; } one = { 1 }; return one.c[0]; */
    1
}

unsafe fn ZSTD_read16(p: *const c_void) -> U16 {
    (p as *const U16).read_unaligned()
}

unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.wrapping_offset(length);
    while op < oend {
        /* COPY8(op, ip) */
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
    }
}

unsafe fn ZSTD_readLE16(memPtr: *const c_void) -> U16 {
    if ZSTD_isLittleEndian() != 0 {
        ZSTD_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}

unsafe fn ZSTD_readLE24(memPtr: *const c_void) -> U32 {
    (ZSTD_readLE16(memPtr) as U32)
        .wrapping_add(((*(memPtr as *const BYTE).add(2) as c_int) << 16) as U32)
}

unsafe fn ZSTD_readBE32(memPtr: *const c_void) -> U32 {
    let p = memPtr as *const BYTE;
    ((*p.add(0) as U32) << 24)
        .wrapping_add((*p.add(1) as U32) << 16)
        .wrapping_add((*p.add(2) as U32) << 8)
        .wrapping_add((*p.add(3) as U32) << 0)
}

/* ******************************************************************
 *  Local structures
 ********************************************************************/

/* typedef enum { bt_compressed, bt_raw, bt_rle, bt_end } blockType_t; */
pub type blockType_t = c_int;
const bt_compressed: blockType_t = 0;
const bt_raw: blockType_t = 1;
const bt_rle: blockType_t = 2;
const bt_end: blockType_t = 3;

#[repr(C)]
#[derive(Copy, Clone)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct SeqStore_t {
    buffer: *mut c_void,
    offsetStart: *mut U32,
    offset: *mut U32,
    offCodeStart: *mut BYTE,
    offCode: *mut BYTE,
    litStart: *mut BYTE,
    lit: *mut BYTE,
    litLengthStart: *mut BYTE,
    litLength: *mut BYTE,
    matchLengthStart: *mut BYTE,
    matchLength: *mut BYTE,
    dumpsStart: *mut BYTE,
    dumps: *mut BYTE,
}

/* typedef struct ZSTD_Cctx_s { ... } cctxi_t; */
#[repr(C)]
struct cctxi_t {
    base: *const BYTE,
    current: U32,
    nextUpdate: U32,
    seqStore: SeqStore_t,
    hashTable: [U32; HASH_TABLESIZE],
    buffer: [BYTE; WORKPLACESIZE],
}

/* ******************************************************************
 *  Error Management
 ********************************************************************/

/* published entry point */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/* ******************************************************************
 *  Tool functions
 ********************************************************************/

const ZSTD_VERSION_MAJOR: u32 = 0;
const ZSTD_VERSION_MINOR: u32 = 1;
const ZSTD_VERSION_RELEASE: u32 = 3;
const ZSTD_VERSION_NUMBER: u32 =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;

/* ******************************************************************
 *  Decompression code
 ********************************************************************/

unsafe fn ZSTDv01_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_ = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    headerFlags = *in_;
    cSize = (*in_.add(2) as U32)
        .wrapping_add(((*in_.add(1) as c_int) << 8) as U32)
        .wrapping_add((((*in_.add(0) as c_int) & 7) << 16) as U32);

    (*bpPtr).blockType = (headerFlags >> 6) as blockType_t;
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

unsafe fn ZSTD_copyUncompressedBlock(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

unsafe fn ZSTD_decompressLiterals(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut op = dst as *mut BYTE;
    let oend = op.wrapping_add(maxDstSize);
    let ip = src as *const BYTE;
    let errorCode: usize;
    let mut litSize: usize;

    /* check : minimum 2, for litSize, +1, for content */
    if srcSize <= 3 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    litSize = (*ip.add(1) as usize).wrapping_add(((*ip.add(0) as c_int) << 8) as usize);
    /* mmmmh.... */
    litSize = litSize
        .wrapping_add(((((*ip.wrapping_offset(-3) as c_int) >> 3) & 7) << 16) as usize);
    op = oend.wrapping_sub(litSize);

    let _ = ctx;
    if litSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    errorCode = HUF_decompress(
        op as *mut c_void,
        litSize,
        ip.wrapping_add(2) as *const c_void,
        srcSize.wrapping_sub(2),
    );
    if FSE_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
    litSize
}

unsafe fn ZSTDv01_decodeLiteralsBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    litStart: *mut *const BYTE,
    litSize: *mut usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;
    let mut ip = istart;
    let ostart = dst as *mut BYTE;
    let oend = ostart.wrapping_add(maxDstSize);
    let mut litbp = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    let litcSize = ZSTDv01_getcBlockSize(src, srcSize, &mut litbp);
    if ZSTDv01_isError(litcSize) != 0 {
        return litcSize;
    }
    if litcSize > srcSize.wrapping_sub(ZSTD_blockHeaderSize) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(ZSTD_blockHeaderSize);

    match litbp.blockType {
        bt_raw => {
            *litStart = ip;
            ip = ip.wrapping_add(litcSize);
            *litSize = litcSize;
        }
        bt_rle => {
            let rleSize: usize = litbp.origSize as usize;
            if rleSize > maxDstSize {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            if srcSize == 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if rleSize > 0 {
                memset(
                    oend.wrapping_sub(rleSize) as *mut c_void,
                    *ip as c_int,
                    rleSize,
                );
            }
            *litStart = oend.wrapping_sub(rleSize);
            *litSize = rleSize;
            ip = ip.wrapping_add(1);
        }
        bt_compressed => {
            let decodedLitSize = ZSTD_decompressLiterals(
                ctx,
                dst,
                maxDstSize,
                ip as *const c_void,
                litcSize,
            );
            if ZSTDv01_isError(decodedLitSize) != 0 {
                return decodedLitSize;
            }
            *litStart = oend.wrapping_sub(decodedLitSize);
            *litSize = decodedLitSize;
            ip = ip.wrapping_add(litcSize);
        }
        _ => {
            /* bt_end, default */
            return ERROR(ZSTD_error_GENERIC);
        }
    }

    (ip as usize).wrapping_sub(istart as usize)
}

unsafe fn ZSTDv01_decodeSeqHeaders(
    nbSeq: *mut c_int,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut FSE_DTable,
    DTableML: *mut FSE_DTable,
    DTableOffb: *mut FSE_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;
    let mut ip = istart;
    let iend = istart.wrapping_add(srcSize);
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
    *nbSeq = ZSTD_readLE16(ip as *const c_void) as c_int;
    ip = ip.wrapping_add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = (*ip.add(2) as usize).wrapping_add(((*ip.add(1) as c_int) << 8) as usize);
        ip = ip.wrapping_add(3);
    } else {
        dumpsLength =
            (*ip.add(1) as usize).wrapping_add((((*ip.add(0) as c_int) & 1) << 8) as usize);
        ip = ip.wrapping_add(2);
    }
    *dumpsPtr = ip;
    ip = ip.wrapping_add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    /* check */
    if ip > iend.wrapping_sub(3) {
        /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* sequences */
    {
        /* assumption : MaxML >= MaxLL and MaxOff */
        let mut norm: [S16; MaxML + 1] = [0; MaxML + 1];
        let mut headerSize: usize;

        /* Build DTables */
        match LLtype as blockType_t {
            bt_rle => {
                LLlog = 0;
                FSE_buildDTable_rle(DTableLL, *ip);
                ip = ip.wrapping_add(1);
            }
            bt_raw => {
                LLlog = LLbits;
                FSE_buildDTable_raw(DTableLL, LLbits);
            }
            _ => {
                let mut max: U32 = MaxLL as U32;
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
                if LLlog > LLFSELog as U32 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.wrapping_add(headerSize);
                FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        match Offtype as blockType_t {
            bt_rle => {
                Offlog = 0;
                if ip > iend.wrapping_sub(2) {
                    /* min : "raw", hence no header, but at least xxLog bits */
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                FSE_buildDTable_rle(DTableOffb, *ip);
                ip = ip.wrapping_add(1);
            }
            bt_raw => {
                Offlog = Offbits;
                FSE_buildDTable_raw(DTableOffb, Offbits);
            }
            _ => {
                let mut max: U32 = MaxOff as U32;
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
                if Offlog > OffFSELog as U32 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.wrapping_add(headerSize);
                FSE_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
            }
        }

        match MLtype as blockType_t {
            bt_rle => {
                MLlog = 0;
                if ip > iend.wrapping_sub(2) {
                    /* min : "raw", hence no header, but at least xxLog bits */
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                FSE_buildDTable_rle(DTableML, *ip);
                ip = ip.wrapping_add(1);
            }
            bt_raw => {
                MLlog = MLbits;
                FSE_buildDTable_raw(DTableML, MLbits);
            }
            _ => {
                let mut max: U32 = MaxML as U32;
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
                if MLlog > MLFSELog as U32 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.wrapping_add(headerSize);
                FSE_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
            }
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
    DStream: FSE_DStream_t,
    stateLL: FSE_DState_t,
    stateOffb: FSE_DState_t,
    stateML: FSE_DState_t,
    prevOffset: usize,
    dumps: *const BYTE,
    dumpsEnd: *const BYTE,
}

unsafe fn ZSTD_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let mut litLength: usize;
    let prevOffset: usize;
    let mut offset: usize;
    let mut matchLength: usize;
    let mut dumps: *const BYTE = (*seqState).dumps;
    let de: *const BYTE = (*seqState).dumpsEnd;

    /* Literal length */
    litLength = FSE_decodeSymbol(
        &mut (*seqState).stateLL,
        &mut (*seqState).DStream,
    ) as usize;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    (*seqState).prevOffset = (*seq).offset;
    if litLength == MaxLL {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.wrapping_add(1);
            v
        } else {
            0
        };
        if add < 255 {
            litLength = litLength.wrapping_add(add as usize);
        } else {
            if dumps <= de.wrapping_sub(3) {
                litLength = ZSTD_readLE24(dumps as *const c_void) as usize;
                dumps = dumps.wrapping_add(3);
            }
        }
    }

    /* Offset */
    {
        let offsetCode: U32;
        let mut nbBits: U32;
        offsetCode = FSE_decodeSymbol(
            &mut (*seqState).stateOffb,
            &mut (*seqState).DStream,
        ) as U32;
        if ZSTD_32bits() != 0 {
            FSE_reloadDStream(&mut (*seqState).DStream);
        }
        nbBits = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            /* cmove */
            nbBits = 0;
        }
        offset = (1usize << (nbBits & ((core::mem::size_of::<usize>() * 8) - 1) as U32))
            .wrapping_add(FSE_readBits(&mut (*seqState).DStream, nbBits));
        if ZSTD_32bits() != 0 {
            FSE_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset;
        }
    }

    /* MatchLength */
    matchLength = FSE_decodeSymbol(
        &mut (*seqState).stateML,
        &mut (*seqState).DStream,
    ) as usize;
    if matchLength == MaxML {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.wrapping_add(1);
            v
        } else {
            0
        };
        if add < 255 {
            matchLength = matchLength.wrapping_add(add as usize);
        } else {
            if dumps <= de.wrapping_sub(3) {
                matchLength = ZSTD_readLE24(dumps as *const c_void) as usize;
                dumps = dumps.wrapping_add(3);
            }
        }
    }
    matchLength = matchLength.wrapping_add(MINMATCH);

    /* save result */
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> usize {
    /* added */
    static dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
    /* subtracted */
    static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
    let ostart: *const BYTE = op;
    let oLitEnd: *mut BYTE = op.wrapping_add(sequence.litLength);
    let litLength: usize = sequence.litLength;
    /* risk : address space overflow (32-bits) */
    let endMatch: *mut BYTE = op
        .wrapping_add(litLength)
        .wrapping_add(sequence.matchLength);
    let litEnd: *const BYTE = (*litPtr).wrapping_add(litLength);

    /* checks */
    let seqLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > (oend as usize).wrapping_sub(op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize).wrapping_sub(*litPtr as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Now we know there are no overflow in literal nor match lengths, can use pointer checks */
    if sequence.offset
        > ((oLitEnd as usize).wrapping_sub(base as usize) as U32) as usize
    {
        return ERROR(ZSTD_error_corruption_detected);
    }

    if endMatch > oend {
        /* overwrite beyond dst buffer */
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if litEnd > litLimit {
        /* overRead beyond lit buffer */
        return ERROR(ZSTD_error_corruption_detected);
    }
    if sequence.matchLength > (*litPtr as usize).wrapping_sub(op as usize) {
        /* overwrite literal segment */
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    /* copy Literals */
    /* note : v0.1 seems to allow scenarios where output or input are close to end of buffer */
    ZSTD_memmove(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength,
    );

    op = op.wrapping_add(litLength);
    /* update for next sequence */
    *litPtr = litEnd;

    /* check : last match must be at a minimum distance of 8 from end of dest buffer */
    if ((oend as isize).wrapping_sub(op as isize)) < 8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    /* copy Match */
    {
        let overlapRisk: U32 =
            (((litEnd as usize).wrapping_sub(endMatch as usize)) < 12) as U32;
        /* possible underflow at op - offset ? */
        let mut match_: *const BYTE = op.wrapping_sub(sequence.offset);
        let mut qutt: usize = 12;
        let mut saved: [U64; 2] = [0; 2];

        /* check */
        if match_ < base {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if sequence.offset > base as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* save beginning of literal sequence, in case of write overlap */
        if overlapRisk != 0 {
            if endMatch.wrapping_add(qutt) > oend {
                qutt = (oend as usize).wrapping_sub(endMatch as usize);
            }
            memcpy(
                saved.as_mut_ptr() as *mut c_void,
                endMatch as *const c_void,
                qutt,
            );
        }

        if sequence.offset < 8 {
            let dec64: c_int = dec64table[sequence.offset];
            *op.add(0) = *match_.add(0);
            *op.add(1) = *match_.add(1);
            *op.add(2) = *match_.add(2);
            *op.add(3) = *match_.add(3);
            match_ = match_.wrapping_offset(dec32table[sequence.offset] as isize);
            ZSTD_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
            match_ = match_.wrapping_offset(-(dec64 as isize));
        } else {
            ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
        }
        op = op.wrapping_add(8);
        match_ = match_.wrapping_add(8);

        if endMatch > oend.wrapping_sub(16 - MINMATCH) {
            if op < oend.wrapping_sub(8) {
                let diff: isize =
                    (oend.wrapping_sub(8) as isize).wrapping_sub(op as isize);
                ZSTD_wildcopy(op as *mut c_void, match_ as *const c_void, diff);
                match_ = match_.wrapping_offset(diff);
                op = oend.wrapping_sub(8);
            }
            while op < endMatch {
                *op = *match_;
                op = op.wrapping_add(1);
                match_ = match_.wrapping_add(1);
            }
        } else {
            /* works even if matchLength < 8 */
            ZSTD_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                (sequence.matchLength as isize).wrapping_sub(8),
            );
        }

        /* restore, in case of overlap */
        if overlapRisk != 0 {
            memcpy(
                endMatch as *mut c_void,
                saved.as_ptr() as *const c_void,
                qutt,
            );
        }
    }

    (endMatch as usize).wrapping_sub(ostart as usize)
}

/* typedef struct ZSTDv01_Dctx_s { ... } dctx_t; */
#[repr(C)]
pub struct dctx_t {
    LLTable: [U32; FSE_DTABLE_SIZE_U32(LLFSELog)],
    OffTable: [U32; FSE_DTABLE_SIZE_U32(OffFSELog)],
    MLTable: [U32; FSE_DTABLE_SIZE_U32(MLFSELog)],
    previousDstEnd: *mut c_void,
    base: *mut c_void,
    expected: usize,
    bType: blockType_t,
    phase: U32,
}

pub type ZSTDv01_Dctx = dctx_t;

unsafe fn ZSTD_decompressSequences(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    litStart: *const BYTE,
    litSize: usize,
) -> usize {
    let dctx = ctx as *mut dctx_t;
    let mut ip = seqStart as *const BYTE;
    let iend = ip.wrapping_add(seqSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.wrapping_add(maxDstSize);
    let mut errorCode: usize;
    let mut dumpsLength: usize = 0;
    let mut litPtr: *const BYTE = litStart;
    let litEnd: *const BYTE = litStart.wrapping_add(litSize);
    let mut nbSeq: c_int = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL: *mut U32 = (*dctx).LLTable.as_mut_ptr();
    let DTableML: *mut U32 = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb: *mut U32 = (*dctx).OffTable.as_mut_ptr();
    let base: *mut BYTE = (*dctx).base as *mut BYTE;

    /* Build Decoding Tables */
    errorCode = ZSTDv01_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        (iend as usize).wrapping_sub(ip as usize),
    );
    if ZSTDv01_isError(errorCode) != 0 {
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
        let mut seqState = seqState_t {
            DStream: FSE_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            },
            stateLL: FSE_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateOffb: FSE_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateML: FSE_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            prevOffset: 0,
            dumps: core::ptr::null(),
            dumpsEnd: core::ptr::null(),
        };

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.wrapping_add(dumpsLength);
        seqState.prevOffset = 1;
        errorCode = FSE_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
        );
        if FSE_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSE_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSE_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSE_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (FSE_reloadDStream(&mut seqState.DStream) <= FSE_DStream_completed)
            && (nbSeq > 0)
        {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize =
                ZSTD_execSequence(op, sequence, &mut litPtr, litEnd, base, oend);
            if ZSTDv01_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.wrapping_add(oneSeqSize);
        }

        /* check if reached exact end */
        if FSE_endOfDStream(&seqState.DStream) == 0 {
            /* requested too much : data is corrupted */
            return ERROR(ZSTD_error_corruption_detected);
        }
        if nbSeq < 0 {
            /* requested too many sequences : data is corrupted */
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* last literal segment */
        {
            let lastLLSize: usize = (litEnd as usize).wrapping_sub(litPtr as usize);
            if op.wrapping_add(lastLLSize) > oend {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            if lastLLSize > 0 {
                if op as *const BYTE != litPtr {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.wrapping_add(lastLLSize);
            }
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    /* blockType == blockCompressed, srcSize is trusted */
    let mut ip = src as *const BYTE;
    let mut litPtr: *const BYTE = core::ptr::null();
    let mut litSize: usize = 0;
    let errorCode: usize;

    /* Decode literals sub-block */
    errorCode = ZSTDv01_decodeLiteralsBlock(
        ctx,
        dst,
        maxDstSize,
        &mut litPtr,
        &mut litSize,
        src,
        srcSize,
    );
    if ZSTDv01_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.wrapping_add(errorCode);
    srcSize = srcSize.wrapping_sub(errorCode);

    ZSTD_decompressSequences(
        ctx,
        dst,
        maxDstSize,
        ip as *const c_void,
        srcSize,
        litPtr,
        litSize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompressDCtx(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend = ip.wrapping_add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.wrapping_add(maxDstSize);
    let mut remainingSize = srcSize;
    let magicNumber: U32;
    let mut errorCode: usize = 0;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    magicNumber = ZSTD_readBE32(src);
    if magicNumber != ZSTD_magicNumber {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    ip = ip.wrapping_add(ZSTD_frameHeaderSize);
    remainingSize = remainingSize.wrapping_sub(ZSTD_frameHeaderSize);

    /* Loop on each block */
    loop {
        let blockSize = ZSTDv01_getcBlockSize(
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
            &mut blockProperties,
        );
        if ZSTDv01_isError(blockSize) != 0 {
            return blockSize;
        }

        ip = ip.wrapping_add(ZSTD_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTD_blockHeaderSize);
        if blockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        match blockProperties.blockType {
            bt_compressed => {
                errorCode = ZSTD_decompressBlock(
                    ctx,
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    blockSize,
                );
            }
            bt_raw => {
                errorCode = ZSTD_copyUncompressedBlock(
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    blockSize,
                );
            }
            bt_rle => {
                /* not yet supported */
                return ERROR(ZSTD_error_GENERIC);
            }
            bt_end => {
                /* end of frame */
                if remainingSize != 0 {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
        if blockSize == 0 {
            /* bt_end */
            break;
        }

        if ZSTDv01_isError(errorCode) != 0 {
            return errorCode;
        }
        op = op.wrapping_add(errorCode);
        ip = ip.wrapping_add(blockSize);
        remainingSize = remainingSize.wrapping_sub(blockSize);
    }

    (op as usize).wrapping_sub(ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* dctx_t ctx; ctx.base = dst; (rest left uninitialized, as in C) */
    let mut ctx_storage = core::mem::MaybeUninit::<dctx_t>::uninit();
    let ctx = ctx_storage.as_mut_ptr();
    (*ctx).base = dst;
    ZSTDv01_decompressDCtx(ctx as *mut c_void, dst, maxDstSize, src, srcSize)
}

/* ZSTD_errorFrameSizeInfoLegacy() :
assumes `cSize` and `dBound` are _not_ NULL */
unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
    ret: usize,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR as c_ulonglong;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;
    let magicNumber: U32;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    magicNumber = ZSTD_readBE32(src);
    if magicNumber != ZSTD_magicNumber {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
        return;
    }
    ip = ip.wrapping_add(ZSTD_frameHeaderSize);
    remainingSize = remainingSize.wrapping_sub(ZSTD_frameHeaderSize);

    /* Loop on each block */
    loop {
        let blockSize =
            ZSTDv01_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv01_isError(blockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, blockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTD_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTD_blockHeaderSize);
        if blockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if blockSize == 0 {
            /* bt_end */
            break;
        }

        ip = ip.wrapping_add(blockSize);
        remainingSize = remainingSize.wrapping_sub(blockSize);
        nbBlocks = nbBlocks.wrapping_add(1);
    }

    *cSize = (ip as usize).wrapping_sub(src as *const BYTE as usize);
    *dBound = nbBlocks.wrapping_mul(BLOCKSIZE) as c_ulonglong;
}

/* ******************************
 *  Streaming Decompression API
 *******************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_resetDCtx(dctx: *mut ZSTDv01_Dctx) -> usize {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0;
    (*dctx).previousDstEnd = core::ptr::null_mut();
    (*dctx).base = core::ptr::null_mut();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_createDCtx() -> *mut ZSTDv01_Dctx {
    let dctx = malloc(core::mem::size_of::<ZSTDv01_Dctx>()) as *mut ZSTDv01_Dctx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDv01_resetDCtx(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_freeDCtx(dctx: *mut ZSTDv01_Dctx) -> usize {
    free(dctx as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_nextSrcSizeToDecompress(dctx: *mut ZSTDv01_Dctx) -> usize {
    (*(dctx as *mut dctx_t)).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompressContinue(
    dctx: *mut ZSTDv01_Dctx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ctx = dctx as *mut dctx_t;

    /* Sanity check */
    if srcSize != (*ctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dst != (*ctx).previousDstEnd {
        /* not contiguous */
        (*ctx).base = dst;
    }

    /* Decompress : frame header */
    if (*ctx).phase == 0 {
        /* Check frame magic header */
        let magicNumber: U32 = ZSTD_readBE32(src);
        if magicNumber != ZSTD_magicNumber {
            return ERROR(ZSTD_error_prefix_unknown);
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        return 0;
    }

    /* Decompress : block header */
    if (*ctx).phase == 1 {
        let mut bp = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let blockSize = ZSTDv01_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
        if ZSTDv01_isError(blockSize) != 0 {
            return blockSize;
        }
        if bp.blockType == bt_end {
            (*ctx).expected = 0;
            (*ctx).phase = 0;
        } else {
            (*ctx).expected = blockSize;
            (*ctx).bType = bp.blockType;
            (*ctx).phase = 2;
        }

        return 0;
    }

    /* Decompress : block content */
    {
        let rSize: usize;
        match (*ctx).bType {
            bt_compressed => {
                rSize =
                    ZSTD_decompressBlock(ctx as *mut c_void, dst, maxDstSize, src, srcSize);
            }
            bt_raw => {
                rSize = ZSTD_copyUncompressedBlock(dst, maxDstSize, src, srcSize);
            }
            bt_rle => {
                /* not yet handled */
                return ERROR(ZSTD_error_GENERIC);
            }
            bt_end => {
                /* should never happen (filtered at phase 1) */
                rSize = 0;
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTDv01_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = ((dst as *mut c_char).wrapping_add(rSize)) as *mut c_void;
        rSize
    }
}
