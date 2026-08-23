//! Translation of legacy/zstd_v01.c + legacy/zstd_v01.h
//!
//! This is a self-contained legacy decoder: it defines its own private copies of
//! the FSE / bitstream / mem helpers, exactly as the C file does.
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens,
    unused_imports
)]

use crate::error_private::*;
use crate::mem::{calloc, free, malloc, memcpy, memmove, memset, qsort};

use core::ffi::{c_char, c_int, c_uint, c_void};

/******************************************
*  Basic Types
******************************************/
pub type BYTE = u8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

/******************************************
*  Static allocation
******************************************/
/* You can statically allocate FSE CTable/DTable as a table of unsigned using below macro */
pub const fn FSE_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

/* You can statically allocate Huff0 DTable as a table of unsigned short using below macro */
pub const fn HUF_DTABLE_SIZE_U16(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

/******************************************
*  Error Management
******************************************/
pub type FSE_errorCodes = c_int;
pub const FSE_OK_NoError: FSE_errorCodes = 0;
pub const FSE_ERROR_GENERIC: FSE_errorCodes = 1;
pub const FSE_ERROR_tableLog_tooLarge: FSE_errorCodes = 2;
pub const FSE_ERROR_maxSymbolValue_tooLarge: FSE_errorCodes = 3;
pub const FSE_ERROR_maxSymbolValue_tooSmall: FSE_errorCodes = 4;
pub const FSE_ERROR_dstSize_tooSmall: FSE_errorCodes = 5;
pub const FSE_ERROR_srcSize_wrong: FSE_errorCodes = 6;
pub const FSE_ERROR_corruptionDetected: FSE_errorCodes = 7;
pub const FSE_ERROR_maxCode: FSE_errorCodes = 8;

/* `(size_t)-FSE_ERROR_xxx` */
macro_rules! FSE_ERR {
    ($e:expr) => {
        (0usize).wrapping_sub($e as usize)
    };
}

/******************************************
*  FSE symbol compression API
******************************************/
pub type FSE_CTable = c_uint;
pub type FSE_DTable = c_uint;

#[repr(C)]
pub struct FSE_CStream_t {
    pub bitContainer: usize,
    pub bitPos: c_int,
    pub startPtr: *mut c_char,
    pub ptr: *mut c_char,
    pub endPtr: *mut c_char,
}

#[repr(C)]
pub struct FSE_CState_t {
    pub value: isize,
    pub stateTable: *const c_void,
    pub symbolTT: *const c_void,
    pub stateLog: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSE_DStream_t {
    pub bitContainer: usize,
    pub bitsConsumed: c_uint,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSE_DState_t {
    pub state: usize,
    pub table: *const c_void, /* precise table may vary, depending on U16 */
}

/* result of FSE_reloadDStream() */
pub const FSE_DStream_unfinished: c_uint = 0;
pub const FSE_DStream_endOfBuffer: c_uint = 1;
pub const FSE_DStream_completed: c_uint = 2;
pub const FSE_DStream_tooFar: c_uint = 3;

/****************************************************************
*  Tuning parameters
****************************************************************/
pub const FSE_MAX_MEMORY_USAGE: u32 = 14;
pub const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;

/* FSE_MAX_SYMBOL_VALUE : Maximum symbol value authorized. */
pub const FSE_MAX_SYMBOL_VALUE: u32 = 255;

/****************************************************************
*  Byte symbol type
****************************************************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSE_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
} /* size == U32 */

/****************************************************************
*  Memory I/O
*****************************************************************/
pub fn FSE_32bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 4) as c_uint
}

pub fn FSE_isLittleEndian() -> c_uint {
    if cfg!(target_endian = "little") {
        1
    } else {
        0
    }
}

pub unsafe fn FSE_read16(memPtr: *const c_void) -> U16 {
    core::ptr::read_unaligned(memPtr as *const U16)
}

pub unsafe fn FSE_read32(memPtr: *const c_void) -> U32 {
    core::ptr::read_unaligned(memPtr as *const U32)
}

pub unsafe fn FSE_read64(memPtr: *const c_void) -> U64 {
    core::ptr::read_unaligned(memPtr as *const U64)
}

pub unsafe fn FSE_readLE16(memPtr: *const c_void) -> U16 {
    if FSE_isLittleEndian() != 0 {
        FSE_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        ((*p.add(0) as c_int) + ((*p.add(1) as c_int) << 8)) as U16
    }
}

pub unsafe fn FSE_readLE32(memPtr: *const c_void) -> U32 {
    if FSE_isLittleEndian() != 0 {
        FSE_read32(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U32)
            + ((*p.add(1) as U32) << 8)
            + ((*p.add(2) as U32) << 16)
            + ((*p.add(3) as U32) << 24)
    }
}

pub unsafe fn FSE_readLE64(memPtr: *const c_void) -> U64 {
    if FSE_isLittleEndian() != 0 {
        FSE_read64(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U64)
            + ((*p.add(1) as U64) << 8)
            + ((*p.add(2) as U64) << 16)
            + ((*p.add(3) as U64) << 24)
            + ((*p.add(4) as U64) << 32)
            + ((*p.add(5) as U64) << 40)
            + ((*p.add(6) as U64) << 48)
            + ((*p.add(7) as U64) << 56)
    }
}

pub unsafe fn FSE_readLEST(memPtr: *const c_void) -> usize {
    if FSE_32bits() != 0 {
        FSE_readLE32(memPtr) as usize
    } else {
        FSE_readLE64(memPtr) as usize
    }
}

/****************************************************************
*  Constants
*****************************************************************/
pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
pub const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
pub const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
pub const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
pub const FSE_MIN_TABLELOG: u32 = 5;

pub const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

/****************************************************************
*  Complex types
****************************************************************/
#[repr(C)]
pub struct FSE_symbolCompressionTransform {
    pub deltaFindState: c_int,
    pub deltaNbBits: U32,
} /* total 8 bytes */

pub type DTable_max_t = [U32; FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)];

/****************************************************************
*  Internal functions
****************************************************************/
pub fn FSE_highbit32(val: U32) -> c_uint {
    /* __builtin_clz (val) ^ 31 */
    val.leading_zeros() ^ 31
}

/****************************************************************
*  Templates
****************************************************************/
pub fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSE_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
} /* sizeof U32 */

pub unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    /* because dt is unsigned, 32-bits aligned on 32-bits */
    let tableDecode: *mut FSE_decode_t = (ptr as *mut FSE_decode_t).wrapping_add(1);
    let tableSize: U32 = 1u32.wrapping_shl(tableLog);
    let tableMask: U32 = tableSize.wrapping_sub(1);
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; FSE_MAX_SYMBOL_VALUE as usize + 1] =
        [0; FSE_MAX_SYMBOL_VALUE as usize + 1];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);
    let largeLimit: S16 = (1i32.wrapping_shl(tableLog.wrapping_sub(1))) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    /* Sanity Checks */
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return FSE_ERR!(FSE_ERROR_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return FSE_ERR!(FSE_ERROR_tableLog_tooLarge);
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
                position = position.wrapping_add(step) & tableMask; /* lowprob area */
            }
            i += 1;
        }
        s = s.wrapping_add(1);
    }

    if position != 0 {
        return FSE_ERR!(FSE_ERROR_GENERIC);
    }

    /* Build Decoding table */
    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.add(i as usize)).nbBits =
                tableLog.wrapping_sub(FSE_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(i as usize)).newState = (nextState as U32)
                .wrapping_shl((*tableDecode.add(i as usize)).nbBits as U32)
                .wrapping_sub(tableSize) as U16;
            i = i.wrapping_add(1);
        }
    }

    (*DTableH).fastMode = noLarge as U16;
    return 0;
}

/******************************************
*  FSE byte symbol
******************************************/
pub fn FSE_isError(code: usize) -> c_uint {
    (code > (0usize).wrapping_sub(FSE_ERROR_maxCode as usize)) as c_uint
}

pub fn FSE_abs(a: S16) -> S16 {
    if a < 0 {
        (-(a as c_int)) as S16
    } else {
        a
    }
}

/****************************************************************
*  Header bitstream management
****************************************************************/
pub unsafe fn FSE_readNCount(
    normalizedCounter: *mut S16,
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
        return FSE_ERR!(FSE_ERROR_srcSize_wrong);
    }
    bitStream = FSE_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) as c_int) + FSE_MIN_TABLELOG as c_int; /* extract tableLog */
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as c_int {
        return FSE_ERR!(FSE_ERROR_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1i32.wrapping_shl(nbBits as u32)).wrapping_add(1);
    threshold = 1i32.wrapping_shl(nbBits as u32);
    nbBits += 1;

    while (remaining > 1) && (charnum <= *maxSVPtr) {
        if previous0 != 0 {
            let mut n0: c_uint = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 = n0.wrapping_add(24);
                if ip < iend.wrapping_sub(5) {
                    ip = ip.wrapping_add(2);
                    bitStream = FSE_readLE32(ip as *const c_void).wrapping_shr(bitCount as u32);
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
                return FSE_ERR!(FSE_ERROR_maxSymbolValue_tooSmall);
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
                bitStream = FSE_readLE32(ip as *const c_void).wrapping_shr(bitCount as u32);
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
                if count as c_int >= threshold {
                    count = (count as c_int).wrapping_sub(max as c_int) as S16;
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining = remaining.wrapping_sub(FSE_abs(count) as c_int);
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
                    bitCount = bitCount.wrapping_sub(
                        (8isize.wrapping_mul(
                            (iend.wrapping_sub(4) as isize).wrapping_sub(ip as isize),
                        )) as c_int,
                    );
                    ip = iend.wrapping_sub(4);
                }
                bitStream = FSE_readLE32(ip as *const c_void).wrapping_shr((bitCount & 31) as u32);
            }
        }
    }
    if remaining != 1 {
        return FSE_ERR!(FSE_ERROR_GENERIC);
    }
    *maxSVPtr = charnum.wrapping_sub(1);

    ip = ip.wrapping_offset(((bitCount + 7) >> 3) as isize);
    if ((ip as usize).wrapping_sub(istart as usize)) > hbSize {
        return FSE_ERR!(FSE_ERROR_srcSize_wrong);
    }
    return (ip as usize).wrapping_sub(istart as usize);
}

/*********************************************************
*  Decompression (Byte symbols)
*********************************************************/
pub unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let cell: *mut FSE_decode_t = (ptr as *mut FSE_decode_t).wrapping_add(1);

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    return 0;
}

pub unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: c_uint) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let dinfo: *mut FSE_decode_t = (ptr as *mut FSE_decode_t).wrapping_add(1);
    let tableSize: c_uint = 1u32.wrapping_shl(nbBits);
    let tableMask: c_uint = tableSize.wrapping_sub(1);
    let maxSymbolValue: c_uint = tableMask;
    let mut s: c_uint;

    /* Sanity checks */
    if nbBits < 1 {
        return FSE_ERR!(FSE_ERROR_GENERIC); /* min size */
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

    return 0;
}

/* FSE_initDStream
 * Initialize a FSE_DStream_t.
 * srcBuffer must point at the beginning of an FSE block.
 * The function result is the size of the FSE_block (== srcSize).
 * If srcSize is too small, the function will return an errorCode;
 */
pub unsafe fn FSE_initDStream(
    bitD: *mut FSE_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        return FSE_ERR!(FSE_ERROR_srcSize_wrong);
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
            return FSE_ERR!(FSE_ERROR_GENERIC); /* stop bit not present */
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(FSE_highbit32(contain32));
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let st = (*bitD).start as *const BYTE;
        /* switch(srcSize) with fallthrough from case 7 down to case 2 */
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*st.wrapping_add(6) as usize) << (core::mem::size_of::<usize>() * 8 - 16),
            );
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*st.wrapping_add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24),
            );
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*st.wrapping_add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32),
            );
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*st.wrapping_add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*st.wrapping_add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*st.wrapping_add(1) as usize) << 8);
        }
        contain32 = *(srcBuffer as *const BYTE).wrapping_add(srcSize.wrapping_sub(1)) as U32;
        if contain32 == 0 {
            return FSE_ERR!(FSE_ERROR_GENERIC); /* stop bit not present */
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(FSE_highbit32(contain32));
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(
            ((core::mem::size_of::<usize>().wrapping_sub(srcSize)) as U32).wrapping_mul(8),
        );
    }

    return srcSize;
}

/*FSE_lookBits
 * Provides next n bits from the bitContainer.
 */
pub unsafe fn FSE_lookBits(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> (bitMask.wrapping_sub(nbBits) & bitMask)
}

pub unsafe fn FSE_lookBitsFast(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    /* only if nbBits >= 1 !! */
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((bitMask.wrapping_add(1).wrapping_sub(nbBits)) & bitMask)
}

pub unsafe fn FSE_skipBits(bitD: *mut FSE_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

/*FSE_readBits
 * Read next n bits from the bitContainer.
 */
pub unsafe fn FSE_readBits(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    let value = FSE_lookBits(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    value
}

pub unsafe fn FSE_readBitsFast(bitD: *mut FSE_DStream_t, nbBits: U32) -> usize {
    /* only if nbBits >= 1 !! */
    let value = FSE_lookBitsFast(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    value
}

pub unsafe fn FSE_reloadDStream(bitD: *mut FSE_DStream_t) -> c_uint {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as c_uint {
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
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as c_uint {
            return FSE_DStream_endOfBuffer;
        }
        return FSE_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: U32 = FSE_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = ((*bitD).ptr as usize).wrapping_sub((*bitD).start as usize) as U32; /* ptr > start */
            result = FSE_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const c_void);
        return result;
    }
}

pub unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut FSE_DStream_t,
    dt: *const FSE_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH: *const FSE_DTableHeader = ptr as *const FSE_DTableHeader;
    (*DStatePtr).state = FSE_readBits(bitD, (*DTableH).tableLog as U32);
    FSE_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

pub unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut FSE_DStream_t) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = FSE_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut FSE_DStream_t) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits: usize = FSE_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* FSE_endOfDStream
Tells if bitD has reached end of bitStream or not */
pub unsafe fn FSE_endOfDStream(bitD: *const FSE_DStream_t) -> c_uint {
    (((*bitD).ptr == (*bitD).start)
        && ((*bitD).bitsConsumed == (core::mem::size_of::<usize>() * 8) as c_uint))
        as c_uint
}

pub unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> c_uint {
    ((*DStatePtr).state == 0) as c_uint
}

pub unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: c_uint,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.wrapping_add(maxDstSize);
    let olimit: *mut BYTE = omax.wrapping_sub(3);

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
    errorCode = FSE_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    /* 4 symbols per loop */
    while (FSE_reloadDStream(&mut bitD) == FSE_DStream_unfinished) && (op < olimit) {
        *op.wrapping_add(0) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };

        if (FSE_MAX_TABLELOG as usize) * 2 + 7 > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            FSE_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(1) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state2, &mut bitD)
        };

        if (FSE_MAX_TABLELOG as usize) * 4 + 7 > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            if FSE_reloadDStream(&mut bitD) > FSE_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.wrapping_add(2) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };

        if (FSE_MAX_TABLELOG as usize) * 2 + 7 > core::mem::size_of::<usize>() * 8 {
            /* This test must be static */
            FSE_reloadDStream(&mut bitD);
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
        if (FSE_reloadDStream(&mut bitD) > FSE_DStream_completed)
            || (op == omax)
            || (FSE_endOfDStream(&bitD) != 0 && (fast != 0 || FSE_endOfDState(&state1) != 0))
        {
            break;
        }

        *op = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };
        op = op.wrapping_add(1);

        if (FSE_reloadDStream(&mut bitD) > FSE_DStream_completed)
            || (op == omax)
            || (FSE_endOfDStream(&bitD) != 0 && (fast != 0 || FSE_endOfDState(&state2) != 0))
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
    if FSE_endOfDStream(&bitD) != 0
        && FSE_endOfDState(&state1) != 0
        && FSE_endOfDState(&state2) != 0
    {
        return (op as usize).wrapping_sub(ostart as usize);
    }

    if op == omax {
        return FSE_ERR!(FSE_ERROR_dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */
    }

    return FSE_ERR!(FSE_ERROR_corruptionDetected);
}

pub unsafe fn FSE_decompress_usingDTable(
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

pub unsafe fn FSE_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut cSrcSize = cSrcSize;
    let mut counting: [S16; FSE_MAX_SYMBOL_VALUE as usize + 1] =
        [0; FSE_MAX_SYMBOL_VALUE as usize + 1];
    let mut dt: DTable_max_t = [0; FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSE_MAX_SYMBOL_VALUE;
    let mut errorCode: usize;

    if cSrcSize < 2 {
        return FSE_ERR!(FSE_ERROR_srcSize_wrong); /* too small input size */
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
        return FSE_ERR!(FSE_ERROR_srcSize_wrong); /* too small input size */
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
    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

/* *******************************************************
*  Huff0 : Huffman block compression
*********************************************************/
pub const HUF_MAX_SYMBOL_VALUE: u32 = 255;
pub const HUF_DEFAULT_TABLELOG: u32 = 12; /* used by default, when not specified */
pub const HUF_MAX_TABLELOG: u32 = 12; /* max possible tableLog; for allocation purpose */
pub const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16; /* absolute limit of HUF_MAX_TABLELOG */

#[repr(C)]
pub struct HUF_CElt {
    pub val: U16,
    pub nbBits: BYTE,
}

#[repr(C)]
pub struct nodeElt {
    pub count: U32,
    pub parent: U16,
    pub byte: BYTE,
    pub nbBits: BYTE,
}

/* *******************************************************
*  Huff0 : Huffman block decompression
*********************************************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_DElt {
    pub byte: BYTE,
    pub nbBits: BYTE,
}

static HUF_readDTable_l: [c_int; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];

pub unsafe fn HUF_readDTable(DTable: *mut U16, src: *const c_void, srcSize: usize) -> usize {
    let mut huffWeight: [BYTE; HUF_MAX_SYMBOL_VALUE as usize + 1] =
        [0; HUF_MAX_SYMBOL_VALUE as usize + 1];
    /* large enough for values from 0 to 16 */
    let mut rankVal: [U32; HUF_ABSOLUTEMAX_TABLELOG as usize + 1] =
        [0; HUF_ABSOLUTEMAX_TABLELOG as usize + 1];
    let mut weightTotal: U32;
    let mut maxBits: U32;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: usize;
    let mut oSize: usize;
    let mut n: U32;
    let mut nextRankStart: U32;
    let ptr = DTable.wrapping_add(1) as *mut c_void;
    let dt: *mut HUF_DElt = ptr as *mut HUF_DElt;

    if srcSize == 0 {
        return FSE_ERR!(FSE_ERROR_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            oSize = HUF_readDTable_l[iSize.wrapping_sub(242)] as usize;
            memset(
                huffWeight.as_mut_ptr() as *mut c_void,
                1,
                core::mem::size_of::<[BYTE; HUF_MAX_SYMBOL_VALUE as usize + 1]>(),
            );
            iSize = 0;
        } else {
            /* Incompressible */
            oSize = iSize.wrapping_sub(127);
            iSize = (oSize.wrapping_add(1)) / 2;
            if iSize.wrapping_add(1) > srcSize {
                return FSE_ERR!(FSE_ERROR_srcSize_wrong);
            }
            ip = ip.wrapping_add(1);
            n = 0;
            while (n as usize) < oSize {
                huffWeight[n as usize] = *ip.wrapping_add((n / 2) as usize) >> 4;
                huffWeight[n as usize + 1] = *ip.wrapping_add((n / 2) as usize) & 15;
                n = n.wrapping_add(2);
            }
        }
    } else {
        /* header compressed with FSE (normal case) */
        if iSize.wrapping_add(1) > srcSize {
            return FSE_ERR!(FSE_ERROR_srcSize_wrong);
        }
        /* max 255 values decoded, last one is implied */
        oSize = FSE_decompress(
            huffWeight.as_mut_ptr() as *mut c_void,
            HUF_MAX_SYMBOL_VALUE as usize,
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
        core::mem::size_of::<[U32; HUF_ABSOLUTEMAX_TABLELOG as usize + 1]>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if huffWeight[n as usize] as U32 >= HUF_ABSOLUTEMAX_TABLELOG {
            return FSE_ERR!(FSE_ERROR_corruptionDetected);
        }
        rankVal[huffWeight[n as usize] as usize] =
            rankVal[huffWeight[n as usize] as usize].wrapping_add(1);
        weightTotal =
            weightTotal.wrapping_add((1u32.wrapping_shl(huffWeight[n as usize] as U32)) >> 1);
        n = n.wrapping_add(1);
    }
    if weightTotal == 0 {
        return FSE_ERR!(FSE_ERROR_corruptionDetected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    maxBits = FSE_highbit32(weightTotal).wrapping_add(1);
    if maxBits > *DTable.add(0) as U32 {
        return FSE_ERR!(FSE_ERROR_tableLog_tooLarge); /* DTable is too small */
    }
    *DTable.add(0) = maxBits as U16;
    {
        let total: U32 = 1u32.wrapping_shl(maxBits);
        let rest: U32 = total.wrapping_sub(weightTotal);
        let verif: U32 = 1u32.wrapping_shl(FSE_highbit32(rest));
        let lastWeight: U32 = FSE_highbit32(rest).wrapping_add(1);
        if verif != rest {
            return FSE_ERR!(FSE_ERROR_corruptionDetected); /* last value must be a clean power of 2 */
        }
        huffWeight[oSize] = lastWeight as BYTE;
        rankVal[lastWeight as usize] = rankVal[lastWeight as usize].wrapping_add(1);
    }

    /* check tree construction validity */
    if (rankVal[1] < 2) || (rankVal[1] & 1) != 0 {
        return FSE_ERR!(FSE_ERROR_corruptionDetected);
    }

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= maxBits {
        let current: U32 = nextRankStart;
        nextRankStart =
            nextRankStart.wrapping_add(rankVal[n as usize].wrapping_shl(n.wrapping_sub(1)));
        rankVal[n as usize] = current;
        n = n.wrapping_add(1);
    }

    /* fill DTable */
    n = 0;
    while (n as usize) <= oSize {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32.wrapping_shl(w)) >> 1;
        let mut i: U32;
        let mut D = HUF_DElt { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = (maxBits.wrapping_add(1).wrapping_sub(w)) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize].wrapping_add(length) {
            *dt.wrapping_add(i as usize) = D;
            i = i.wrapping_add(1);
        }
        rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
        n = n.wrapping_add(1);
    }

    return iSize.wrapping_add(1);
}

pub unsafe fn HUF_decodeSymbol(
    Dstream: *mut FSE_DStream_t,
    dt: *const HUF_DElt,
    dtLog: U32,
) -> BYTE {
    let val: usize = FSE_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.wrapping_add(val)).byte;
    FSE_skipBits(Dstream, (*dt.wrapping_add(val)).nbBits as U32);
    c
}

pub unsafe fn HUF_decompress_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    if cSrcSize < 6 {
        return FSE_ERR!(FSE_ERROR_srcSize_wrong);
    }
    {
        let ostart: *mut BYTE = dst as *mut BYTE;
        let mut op: *mut BYTE = ostart;
        let omax: *mut BYTE = op.wrapping_add(maxDstSize);
        let olimit: *mut BYTE = if maxDstSize < 15 {
            op
        } else {
            omax.wrapping_sub(15)
        };

        let ptr = DTable as *const c_void;
        let dt: *const HUF_DElt = (ptr as *const HUF_DElt).wrapping_add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;
        let mut reloadStatus: U32;

        /* Init */
        let jumpTable: *const U16 = cSrc as *const U16;
        let length1: usize = FSE_readLE16(jumpTable as *const c_void) as usize;
        let length2: usize = FSE_readLE16(jumpTable.wrapping_add(1) as *const c_void) as usize;
        let length3: usize = FSE_readLE16(jumpTable.wrapping_add(2) as *const c_void) as usize;
        /* check coherency !! */
        let length4: usize = cSrcSize
            .wrapping_sub(6)
            .wrapping_sub(length1)
            .wrapping_sub(length2)
            .wrapping_sub(length3);
        let start1: *const c_char = (cSrc as *const c_char).wrapping_add(6);
        let start2: *const c_char = start1.wrapping_add(length1);
        let start3: *const c_char = start2.wrapping_add(length2);
        let start4: *const c_char = start3.wrapping_add(length3);
        let mut bitD1 = FSE_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
        };
        let mut bitD2 = bitD1;
        let mut bitD3 = bitD1;
        let mut bitD4 = bitD1;

        if length1
            .wrapping_add(length2)
            .wrapping_add(length3)
            .wrapping_add(6)
            >= cSrcSize
        {
            return FSE_ERR!(FSE_ERROR_srcSize_wrong);
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
            /* HUF_DECODE_SYMBOL_1( 0, bitD1); */
            *op.wrapping_add(0) = HUF_decodeSymbol(&mut bitD1, dt, dtLog);
            if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                FSE_reloadDStream(&mut bitD1);
            }
            /* HUF_DECODE_SYMBOL_1( 1, bitD2); */
            *op.wrapping_add(1) = HUF_decodeSymbol(&mut bitD2, dt, dtLog);
            if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                FSE_reloadDStream(&mut bitD2);
            }
            /* HUF_DECODE_SYMBOL_1( 2, bitD3); */
            *op.wrapping_add(2) = HUF_decodeSymbol(&mut bitD3, dt, dtLog);
            if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                FSE_reloadDStream(&mut bitD3);
            }
            /* HUF_DECODE_SYMBOL_1( 3, bitD4); */
            *op.wrapping_add(3) = HUF_decodeSymbol(&mut bitD4, dt, dtLog);
            if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                FSE_reloadDStream(&mut bitD4);
            }
            /* HUF_DECODE_SYMBOL_2( 4, bitD1); */
            *op.wrapping_add(4) = HUF_decodeSymbol(&mut bitD1, dt, dtLog);
            if FSE_32bits() != 0 {
                FSE_reloadDStream(&mut bitD1);
            }
            /* HUF_DECODE_SYMBOL_2( 5, bitD2); */
            *op.wrapping_add(5) = HUF_decodeSymbol(&mut bitD2, dt, dtLog);
            if FSE_32bits() != 0 {
                FSE_reloadDStream(&mut bitD2);
            }
            /* HUF_DECODE_SYMBOL_2( 6, bitD3); */
            *op.wrapping_add(6) = HUF_decodeSymbol(&mut bitD3, dt, dtLog);
            if FSE_32bits() != 0 {
                FSE_reloadDStream(&mut bitD3);
            }
            /* HUF_DECODE_SYMBOL_2( 7, bitD4); */
            *op.wrapping_add(7) = HUF_decodeSymbol(&mut bitD4, dt, dtLog);
            if FSE_32bits() != 0 {
                FSE_reloadDStream(&mut bitD4);
            }
            /* HUF_DECODE_SYMBOL_1( 8, bitD1); */
            *op.wrapping_add(8) = HUF_decodeSymbol(&mut bitD1, dt, dtLog);
            if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                FSE_reloadDStream(&mut bitD1);
            }
            /* HUF_DECODE_SYMBOL_1( 9, bitD2); */
            *op.wrapping_add(9) = HUF_decodeSymbol(&mut bitD2, dt, dtLog);
            if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                FSE_reloadDStream(&mut bitD2);
            }
            /* HUF_DECODE_SYMBOL_1(10, bitD3); */
            *op.wrapping_add(10) = HUF_decodeSymbol(&mut bitD3, dt, dtLog);
            if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                FSE_reloadDStream(&mut bitD3);
            }
            /* HUF_DECODE_SYMBOL_1(11, bitD4); */
            *op.wrapping_add(11) = HUF_decodeSymbol(&mut bitD4, dt, dtLog);
            if FSE_32bits() != 0 && (HUF_MAX_TABLELOG > 12) {
                FSE_reloadDStream(&mut bitD4);
            }
            /* HUF_DECODE_SYMBOL_0(12..15) */
            *op.wrapping_add(12) = HUF_decodeSymbol(&mut bitD1, dt, dtLog);
            *op.wrapping_add(13) = HUF_decodeSymbol(&mut bitD2, dt, dtLog);
            *op.wrapping_add(14) = HUF_decodeSymbol(&mut bitD3, dt, dtLog);
            *op.wrapping_add(15) = HUF_decodeSymbol(&mut bitD4, dt, dtLog);

            op = op.wrapping_add(16);
            reloadStatus = FSE_reloadDStream(&mut bitD2)
                | FSE_reloadDStream(&mut bitD3)
                | FSE_reloadDStream(&mut bitD4);
            FSE_reloadDStream(&mut bitD1);
        }

        if reloadStatus != FSE_DStream_completed {
            /* not complete : some bitStream might be FSE_DStream_unfinished */
            return FSE_ERR!(FSE_ERROR_corruptionDetected);
        }

        /* tail */
        {
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
                *op.wrapping_add(0) = HUF_decodeSymbol(&mut bitTail, dt, dtLog);
                op = op.wrapping_add(1);
            }

            if FSE_endOfDStream(&bitTail) != 0 {
                return (op as usize).wrapping_sub(ostart as usize);
            }
        }

        if op == omax {
            return FSE_ERR!(FSE_ERROR_dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */
        }

        return FSE_ERR!(FSE_ERROR_corruptionDetected);
    }
}

pub unsafe fn HUF_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* HUF_CREATE_STATIC_DTABLE(DTable, HUF_MAX_TABLELOG) */
    let mut DTable: [U16; HUF_DTABLE_SIZE_U16(HUF_MAX_TABLELOG)] =
        [0; HUF_DTABLE_SIZE_U16(HUF_MAX_TABLELOG)];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;
    let mut errorCode: usize;

    errorCode = HUF_readDTable(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return FSE_ERR!(FSE_ERROR_srcSize_wrong);
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

/****************************************************************
*  zstd - standard compression library
*  Tuning parameters
*****************************************************************/
pub const ZSTD_MEMORY_USAGE: u32 = 17;

/**************************************
   CPU Feature Detection
**************************************/
pub const ZSTD_UNALIGNED_ACCESS: u32 = 1;

/********************************************************
*  Constants
*********************************************************/
pub static ZSTD_magicNumber: U32 = 0xFD2FB51E; /* 3rd version : seqNb header */

pub const HASH_LOG: u32 = ZSTD_MEMORY_USAGE - 2;
pub const HASH_TABLESIZE: usize = 1usize << HASH_LOG;
pub const HASH_MASK: usize = HASH_TABLESIZE - 1;

pub const KNUTH: U32 = 2654435761;

pub const BIT7: u32 = 128;
pub const BIT6: u32 = 64;
pub const BIT5: u32 = 32;
pub const BIT4: u32 = 16;

pub const BLOCKSIZE: usize = 128 * (1 << 10); /* define, for static allocation */

pub const WORKPLACESIZE: usize = BLOCKSIZE * 3;
pub const MINMATCH: usize = 4;
pub const MLbits: u32 = 7;
pub const LLbits: u32 = 6;
pub const Offbits: u32 = 5;
pub const MaxML: u32 = (1u32 << MLbits) - 1;
pub const MaxLL: u32 = (1u32 << LLbits) - 1;
pub const MaxOff: u32 = (1u32 << Offbits) - 1;
pub const LitFSELog: u32 = 11;
pub const MLFSELog: u32 = 10;
pub const LLFSELog: u32 = 10;
pub const OffFSELog: u32 = 9;
pub const MaxSeq: u32 = if MaxLL < MaxML { MaxML } else { MaxLL };

pub const LITERAL_NOENTROPY: u32 = 63;
pub const COMMAND_NOENTROPY: u32 = 7; /* to remove */

pub const ZSTD_CONTENTSIZE_ERROR: u64 = (0u64).wrapping_sub(2);

pub static ZSTD_blockHeaderSize: usize = 3;
pub static ZSTD_frameHeaderSize: usize = 4;

/* from zstd_v01.h : Prefix - version detection */
pub const ZSTDv01_magicNumber: U32 = 0xFD2FB51E; /* Big Endian version */
pub const ZSTDv01_magicNumberLE: U32 = 0x1EB52FFD; /* Little Endian version */

/********************************************************
*  Memory operations
*********************************************************/
pub fn ZSTD_32bits() -> c_uint {
    (core::mem::size_of::<*const c_void>() == 4) as c_uint
}

pub fn ZSTD_isLittleEndian() -> c_uint {
    if cfg!(target_endian = "little") {
        1
    } else {
        0
    }
}

pub unsafe fn ZSTD_read16(p: *const c_void) -> U16 {
    core::ptr::read_unaligned(p as *const U16)
}

pub unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

pub unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

pub unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_offset(length);
    while op < oend {
        /* COPY8(op, ip) */
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
    }
}

pub unsafe fn ZSTD_readLE16(memPtr: *const c_void) -> U16 {
    if ZSTD_isLittleEndian() != 0 {
        ZSTD_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        ((*p.add(0) as U16).wrapping_add((*p.add(1) as U16) << 8)) as U16
    }
}

pub unsafe fn ZSTD_readLE24(memPtr: *const c_void) -> U32 {
    (ZSTD_readLE16(memPtr) as U32)
        .wrapping_add(((*(memPtr as *const BYTE).add(2) as c_int) << 16) as U32)
}

pub unsafe fn ZSTD_readBE32(memPtr: *const c_void) -> U32 {
    let p = memPtr as *const BYTE;
    ((*p.add(0) as U32) << 24)
        .wrapping_add((*p.add(1) as U32) << 16)
        .wrapping_add((*p.add(2) as U32) << 8)
        .wrapping_add((*p.add(3) as U32) << 0)
}

/**************************************
*  Local structures
***************************************/
pub type blockType_t = c_int;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

#[repr(C)]
pub struct SeqStore_t {
    pub buffer: *mut c_void,
    pub offsetStart: *mut U32,
    pub offset: *mut U32,
    pub offCodeStart: *mut BYTE,
    pub offCode: *mut BYTE,
    pub litStart: *mut BYTE,
    pub lit: *mut BYTE,
    pub litLengthStart: *mut BYTE,
    pub litLength: *mut BYTE,
    pub matchLengthStart: *mut BYTE,
    pub matchLength: *mut BYTE,
    pub dumpsStart: *mut BYTE,
    pub dumps: *mut BYTE,
}

#[repr(C)]
pub struct cctxi_t {
    pub base: *const BYTE,
    pub current: U32,
    pub nextUpdate: U32,
    pub seqStore: SeqStore_t,
    pub hashTable: [U32; HASH_TABLESIZE],
    pub buffer: [BYTE; WORKPLACESIZE],
}
pub type ZSTD_Cctx = cctxi_t;

/**************************************
*  Error Management
**************************************/
/* published entry point */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv01_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/**************************************
*  Tool functions
**************************************/
pub const ZSTD_VERSION_MAJOR: u32 = 0;
pub const ZSTD_VERSION_MINOR: u32 = 1;
pub const ZSTD_VERSION_RELEASE: u32 = 3;
pub const ZSTD_VERSION_NUMBER: u32 =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;

/**************************************************************
*   Decompression code
**************************************************************/
pub unsafe fn ZSTDv01_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let inp: *const BYTE = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    headerFlags = *inp;
    cSize = ((*inp.add(2) as c_int)
        .wrapping_add((*inp.add(1) as c_int) << 8)
        .wrapping_add(((*inp.add(0) as c_int) & 7) << 16)) as U32;

    (*bpPtr).blockType = (headerFlags >> 6) as blockType_t;
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    return cSize as usize;
}

pub unsafe fn ZSTD_copyUncompressedBlock(
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
    return srcSize;
}

pub unsafe fn ZSTD_decompressLiterals(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.wrapping_add(maxDstSize);
    let ip: *const BYTE = src as *const BYTE;
    let errorCode: usize;
    let mut litSize: usize;

    /* check : minimum 2, for litSize, +1, for content */
    if srcSize <= 3 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    litSize = ((*ip.add(1) as c_int).wrapping_add((*ip.add(0) as c_int) << 8)) as usize;
    /* mmmmh.... */
    litSize = litSize
        .wrapping_add(((((*ip.wrapping_offset(-3) as c_int) >> 3) & 7) << 16) as usize);
    op = oend.wrapping_sub(litSize);

    /* (void)ctx; */
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
    return litSize;
}

pub unsafe fn ZSTDv01_decodeLiteralsBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    litStart: *mut *const BYTE,
    litSize: *mut usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
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
            let decodedLitSize =
                ZSTD_decompressLiterals(ctx, dst, maxDstSize, ip as *const c_void, litcSize);
            if ZSTDv01_isError(decodedLitSize) != 0 {
                return decodedLitSize;
            }
            *litStart = oend.wrapping_sub(decodedLitSize);
            *litSize = decodedLitSize;
            ip = ip.wrapping_add(litcSize);
        }
        /* case bt_end: default: */
        _ => {
            return ERROR(ZSTD_error_GENERIC);
        }
    }

    return (ip as usize).wrapping_sub(istart as usize);
}

pub unsafe fn ZSTDv01_decodeSeqHeaders(
    nbSeq: *mut c_int,
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
    *nbSeq = ZSTD_readLE16(ip as *const c_void) as c_int;
    ip = ip.wrapping_add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = (*ip.add(2) as usize).wrapping_add((*ip.add(1) as usize) << 8);
        ip = ip.wrapping_add(3);
    } else {
        dumpsLength = (*ip.add(1) as usize).wrapping_add(((*ip.add(0) as usize) & 1) << 8);
        ip = ip.wrapping_add(2);
    }
    *dumpsPtr = ip;
    ip = ip.wrapping_add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    /* check */
    /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */
    if ip > iend.wrapping_sub(3) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* sequences */
    {
        /* assumption : MaxML >= MaxLL and MaxOff */
        let mut norm: [S16; MaxML as usize + 1] = [0; MaxML as usize + 1];
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
        }

        match Offtype as blockType_t {
            bt_rle => {
                Offlog = 0;
                /* min : "raw", hence no header, but at least xxLog bits */
                if ip > iend.wrapping_sub(2) {
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
        }

        match MLtype as blockType_t {
            bt_rle => {
                MLlog = 0;
                /* min : "raw", hence no header, but at least xxLog bits */
                if ip > iend.wrapping_sub(2) {
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
    }

    return (ip as usize).wrapping_sub(istart as usize);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct seq_t {
    pub litLength: usize,
    pub offset: usize,
    pub matchLength: usize,
}

#[repr(C)]
pub struct seqState_t {
    pub DStream: FSE_DStream_t,
    pub stateLL: FSE_DState_t,
    pub stateOffb: FSE_DState_t,
    pub stateML: FSE_DState_t,
    pub prevOffset: usize,
    pub dumps: *const BYTE,
    pub dumpsEnd: *const BYTE,
}

pub unsafe fn ZSTD_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
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
    (*seqState).prevOffset = (*seq).offset;
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
        offsetCode =
            FSE_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream) as U32;
        if ZSTD_32bits() != 0 {
            FSE_reloadDStream(&mut (*seqState).DStream);
        }
        nbBits = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0; /* cmove */
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

static dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */

pub unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> usize {
    let ostart: *const BYTE = op;
    let oLitEnd: *mut BYTE = op.wrapping_add(sequence.litLength);
    let litLength: usize = sequence.litLength;
    /* risk : address space overflow (32-bits) */
    let endMatch: *mut BYTE = op.wrapping_add(litLength).wrapping_add(sequence.matchLength);
    let litEnd: *const BYTE = (*litPtr).wrapping_add(litLength);

    /* checks */
    let seqLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > ((oend as usize).wrapping_sub(op as usize)) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > ((litLimit as usize).wrapping_sub(*litPtr as usize)) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Now we know there are no overflow in literal nor match lengths, can use pointer checks */
    if sequence.offset
        > ((((oLitEnd as isize).wrapping_sub(base as isize)) as U32) as usize)
    {
        return ERROR(ZSTD_error_corruption_detected);
    }

    if endMatch > oend {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* overwrite beyond dst buffer */
    }
    if litEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected); /* overRead beyond lit buffer */
    }
    if sequence.matchLength > ((*litPtr as usize).wrapping_sub(op as usize)) {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* overwrite literal segment */
    }

    /* copy Literals */
    /* note : v0.1 seems to allow scenarios where output or input are close to end of buffer */
    memmove(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength,
    );

    op = op.wrapping_add(litLength);
    *litPtr = litEnd; /* update for next sequence */

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
        if match_ < (base as *const BYTE) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if sequence.offset > (base as usize) {
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
            *op.wrapping_add(0) = *match_.wrapping_add(0);
            *op.wrapping_add(1) = *match_.wrapping_add(1);
            *op.wrapping_add(2) = *match_.wrapping_add(2);
            *op.wrapping_add(3) = *match_.wrapping_add(3);
            match_ = match_.wrapping_offset(dec32table[sequence.offset] as isize);
            ZSTD_copy4(op.wrapping_add(4) as *mut c_void, match_ as *const c_void);
            match_ = match_.wrapping_offset(-(dec64 as isize));
        } else {
            ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
        }
        op = op.wrapping_add(8);
        match_ = match_.wrapping_add(8);

        if endMatch > oend.wrapping_sub(16 - MINMATCH) {
            if op < oend.wrapping_sub(8) {
                ZSTD_wildcopy(
                    op as *mut c_void,
                    match_ as *const c_void,
                    (oend.wrapping_sub(8) as isize).wrapping_sub(op as isize),
                );
                match_ = match_
                    .wrapping_offset((oend.wrapping_sub(8) as isize).wrapping_sub(op as isize));
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

    return (endMatch as usize).wrapping_sub(ostart as usize);
}

#[repr(C)]
pub struct ZSTDv01_Dctx {
    pub LLTable: [U32; FSE_DTABLE_SIZE_U32(LLFSELog)],
    pub OffTable: [U32; FSE_DTABLE_SIZE_U32(OffFSELog)],
    pub MLTable: [U32; FSE_DTABLE_SIZE_U32(MLFSELog)],
    pub previousDstEnd: *mut c_void,
    pub base: *mut c_void,
    pub expected: usize,
    pub bType: blockType_t,
    pub phase: U32,
}
pub type dctx_t = ZSTDv01_Dctx;

pub unsafe fn ZSTD_decompressSequences(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    litStart: *const BYTE,
    litSize: usize,
) -> usize {
    let dctx: *mut dctx_t = ctx as *mut dctx_t;
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(seqSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
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

        /* memset(&sequence, 0, sizeof(sequence)); */
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

        while (FSE_reloadDStream(&mut seqState.DStream) <= FSE_DStream_completed) && (nbSeq > 0)
        {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(op, sequence, &mut litPtr, litEnd, base, oend);
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
                if (op as *const BYTE) != litPtr {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.wrapping_add(lastLLSize);
            }
        }
    }

    return (op as usize).wrapping_sub(ostart as usize);
}

pub unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* blockType == blockCompressed, srcSize is trusted */
    let mut srcSize = srcSize;
    let mut ip: *const BYTE = src as *const BYTE;
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
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut remainingSize: usize = srcSize;
    let magicNumber: U32;
    let mut errorCode: usize = 0;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize.wrapping_add(ZSTD_blockHeaderSize) {
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
                return ERROR(ZSTD_error_GENERIC); /* not yet supported */
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
            break; /* bt_end */
        }

        if ZSTDv01_isError(errorCode) != 0 {
            return errorCode;
        }
        op = op.wrapping_add(errorCode);
        ip = ip.wrapping_add(blockSize);
        remainingSize = remainingSize.wrapping_sub(blockSize);
    }

    return (op as usize).wrapping_sub(ostart as usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ctx: core::mem::MaybeUninit<dctx_t> = core::mem::MaybeUninit::uninit();
    let ctxp: *mut dctx_t = ctx.as_mut_ptr();
    (*ctxp).base = dst;
    ZSTDv01_decompressDCtx(ctxp as *mut c_void, dst, maxDstSize, src, srcSize)
}

/* ZSTD_errorFrameSizeInfoLegacy() :
assumes `cSize` and `dBound` are _not_ NULL */
pub unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut u64, ret: usize) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;
    let magicNumber: U32;
    let mut blockProperties = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTD_frameHeaderSize.wrapping_add(ZSTD_blockHeaderSize) {
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
            break; /* bt_end */
        }

        ip = ip.wrapping_add(blockSize);
        remainingSize = remainingSize.wrapping_sub(blockSize);
        nbBlocks = nbBlocks.wrapping_add(1);
    }

    *cSize = (ip as usize).wrapping_sub(src as usize);
    *dBound = (nbBlocks.wrapping_mul(BLOCKSIZE)) as u64;
}

/*******************************
*  Streaming Decompression API
*******************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_resetDCtx(dctx: *mut ZSTDv01_Dctx) -> usize {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0;
    (*dctx).previousDstEnd = core::ptr::null_mut();
    (*dctx).base = core::ptr::null_mut();
    return 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_createDCtx() -> *mut ZSTDv01_Dctx {
    let dctx: *mut ZSTDv01_Dctx =
        malloc(core::mem::size_of::<ZSTDv01_Dctx>()) as *mut ZSTDv01_Dctx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDv01_resetDCtx(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_freeDCtx(dctx: *mut ZSTDv01_Dctx) -> usize {
    free(dctx as *mut c_void);
    return 0;
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
    let ctx: *mut dctx_t = dctx as *mut dctx_t;

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
                rSize = ZSTD_decompressBlock(ctx as *mut c_void, dst, maxDstSize, src, srcSize);
            }
            bt_raw => {
                rSize = ZSTD_copyUncompressedBlock(dst, maxDstSize, src, srcSize);
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
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTDv01_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = ((dst as *mut c_char).wrapping_add(rSize)) as *mut c_void;
        return rSize;
    }
}
