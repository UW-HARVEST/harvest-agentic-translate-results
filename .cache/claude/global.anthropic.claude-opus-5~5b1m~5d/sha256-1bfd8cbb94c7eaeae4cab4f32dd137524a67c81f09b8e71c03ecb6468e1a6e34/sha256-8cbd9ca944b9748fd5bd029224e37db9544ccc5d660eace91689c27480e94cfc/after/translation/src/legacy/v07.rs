//! Translation of `legacy/zstd_v07.c` (and `legacy/zstd_v07.h`)
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::cmem::{
    MEM_32bits, MEM_64bits, MEM_readLE16, MEM_readLE32, MEM_readLE64, MEM_readLEST, MEM_writeLE16,
    ZSTD_memcpy, ZSTD_memmove, ZSTD_memset, BYTE, S16, U16, U32, U64,
};
use crate::cmem::{free, malloc};
use crate::error_private::{
    ERROR, ERR_getErrorName, ERR_isError, ZSTD_error_GENERIC, ZSTD_error_checksum_wrong,
    ZSTD_error_corruption_detected, ZSTD_error_dictionary_corrupted, ZSTD_error_dictionary_wrong,
    ZSTD_error_dstSize_tooSmall, ZSTD_error_frameParameter_unsupported, ZSTD_error_init_missing,
    ZSTD_error_maxSymbolValue_tooLarge, ZSTD_error_maxSymbolValue_tooSmall,
    ZSTD_error_memory_allocation, ZSTD_error_prefix_unknown, ZSTD_error_srcSize_wrong,
    ZSTD_error_tableLog_tooLarge,
};
use crate::xxhash::{XXH64_state_t, ZSTD_XXH64_digest, ZSTD_XXH64_reset, ZSTD_XXH64_update};

/* ===================================================================
 * zstd_v07.h  /  ZSTDv07_STATIC_LINKING_ONLY constants
 * =================================================================== */

pub const ZSTDv07_MAGICNUMBER: U32 = 0xFD2FB527;
pub const ZSTDv07_MAGIC_SKIPPABLE_START: U32 = 0x184D2A50;

pub const ZSTDv07_WINDOWLOG_MAX_32: c_uint = 25;
pub const ZSTDv07_WINDOWLOG_MAX_64: c_uint = 27;
#[inline(always)]
fn ZSTDv07_WINDOWLOG_MAX() -> U32 {
    if MEM_32bits() != 0 {
        ZSTDv07_WINDOWLOG_MAX_32
    } else {
        ZSTDv07_WINDOWLOG_MAX_64
    }
}

pub const ZSTDv07_FRAMEHEADERSIZE_MAX: usize = 18;
const ZSTDv07_frameHeaderSize_min: usize = 5;
const ZSTDv07_frameHeaderSize_max: usize = ZSTDv07_FRAMEHEADERSIZE_MAX;
const ZSTDv07_skippableHeaderSize: usize = 8;

pub const ZSTDv07_BLOCKSIZE_ABSOLUTEMAX: usize = 128 * 1024;

/* custom memory allocation functions */
pub type ZSTDv07_allocFunction = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type ZSTDv07_freeFunction = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTDv07_customMem {
    pub customAlloc: ZSTDv07_allocFunction,
    pub customFree: ZSTDv07_freeFunction,
    pub opaque: *mut c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTDv07_frameParams {
    pub frameContentSize: u64,
    pub windowSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
}

/* ===================================================================
 * bitstream.h  (BITv07_*)
 * =================================================================== */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BITv07_DStream_t {
    pub bitContainer: usize,
    pub bitsConsumed: c_uint,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

pub type BITv07_DStream_status = c_uint;
pub const BITv07_DStream_unfinished: BITv07_DStream_status = 0;
pub const BITv07_DStream_endOfBuffer: BITv07_DStream_status = 1;
pub const BITv07_DStream_completed: BITv07_DStream_status = 2;
pub const BITv07_DStream_overflow: BITv07_DStream_status = 3;

#[inline(always)]
unsafe fn BITv07_highbit32(val: U32) -> c_uint {
    val.leading_zeros() ^ 31
}

unsafe fn BITv07_initDStream(
    bitD: *mut BITv07_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        ZSTD_memset(
            bitD as *mut c_void,
            0,
            core::mem::size_of::<BITv07_DStream_t>(),
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
            (*bitD).bitsConsumed = if lastByte != 0 {
                8 - BITv07_highbit32(lastByte as U32)
            } else {
                0
            };
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        /* switch with fall-through, cases 7..2 */
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).add(6) as usize) << (64 - 16));
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).add(5) as usize) << (64 - 24));
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).add(4) as usize) << (64 - 32));
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).add(3) as usize) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).add(2) as usize) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*(srcBuffer as *const BYTE).add(1) as usize) << 8);
        }
        {
            let lastByte: BYTE = *(srcBuffer as *const BYTE).add(srcSize - 1);
            (*bitD).bitsConsumed = if lastByte != 0 {
                8 - BITv07_highbit32(lastByte as U32)
            } else {
                0
            };
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
        (*bitD).bitsConsumed = (*bitD).bitsConsumed
            .wrapping_add(((core::mem::size_of::<usize>() - srcSize) * 8) as U32);
    }

    srcSize
}

#[inline(always)]
unsafe fn BITv07_lookBits(bitD: *const BITv07_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask.wrapping_sub(nbBits)) & bitMask)
}

#[inline(always)]
unsafe fn BITv07_lookBitsFast(bitD: *const BITv07_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1).wrapping_sub(nbBits)) & bitMask)
}

#[inline(always)]
unsafe fn BITv07_skipBits(bitD: *mut BITv07_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

#[inline(always)]
unsafe fn BITv07_readBits(bitD: *mut BITv07_DStream_t, nbBits: U32) -> usize {
    let value = BITv07_lookBits(bitD, nbBits);
    BITv07_skipBits(bitD, nbBits);
    value
}

#[inline(always)]
unsafe fn BITv07_readBitsFast(bitD: *mut BITv07_DStream_t, nbBits: U32) -> usize {
    let value = BITv07_lookBitsFast(bitD, nbBits);
    BITv07_skipBits(bitD, nbBits);
    value
}

unsafe fn BITv07_reloadDStream(bitD: *mut BITv07_DStream_t) -> BITv07_DStream_status {
    if (*bitD).bitsConsumed as usize > core::mem::size_of::<usize>() * 8 {
        return BITv07_DStream_overflow;
    }

    if (*bitD).ptr as usize >= ((*bitD).start as usize) + core::mem::size_of::<usize>() {
        (*bitD).ptr = ((*bitD).ptr as usize).wrapping_sub(((*bitD).bitsConsumed >> 3) as usize)
            as *const c_char;
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BITv07_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize) < core::mem::size_of::<usize>() * 8 {
            return BITv07_DStream_endOfBuffer;
        }
        return BITv07_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv07_DStream_status = BITv07_DStream_unfinished;
        if ((*bitD).ptr as usize).wrapping_sub(nbBytes as usize) < (*bitD).start as usize {
            nbBytes = ((*bitD).ptr as usize - (*bitD).start as usize) as U32;
            result = BITv07_DStream_endOfBuffer;
        }
        (*bitD).ptr = ((*bitD).ptr as usize).wrapping_sub(nbBytes as usize) as *const c_char;
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes * 8);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline(always)]
unsafe fn BITv07_endOfDStream(DStream: *const BITv07_DStream_t) -> c_uint {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed as usize == core::mem::size_of::<usize>() * 8)) as c_uint
}

/* ===================================================================
 * fse.h  (FSEv07_*)
 * =================================================================== */

pub type FSEv07_DTable = c_uint;

pub const FSEv07_NCOUNTBOUND: usize = 512;

pub const FSEv07_MAX_MEMORY_USAGE: c_uint = 14;
pub const FSEv07_DEFAULT_MEMORY_USAGE: c_uint = 13;
pub const FSEv07_MAX_SYMBOL_VALUE: c_uint = 255;

pub const FSEv07_MAX_TABLELOG: c_uint = FSEv07_MAX_MEMORY_USAGE - 2;
pub const FSEv07_MAX_TABLESIZE: c_uint = 1u32 << FSEv07_MAX_TABLELOG;
pub const FSEv07_MAXTABLESIZE_MASK: c_uint = FSEv07_MAX_TABLESIZE - 1;
pub const FSEv07_DEFAULT_TABLELOG: c_uint = FSEv07_DEFAULT_MEMORY_USAGE - 2;
pub const FSEv07_MIN_TABLELOG: c_uint = 5;
pub const FSEv07_TABLELOG_ABSOLUTE_MAX: c_uint = 15;

#[inline(always)]
const fn FSEv07_DTABLE_SIZE_U32(maxTableLog: usize) -> usize {
    1 + (1usize << maxTableLog)
}

#[inline(always)]
const fn FSEv07_TABLESTEP(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv07_DState_t {
    pub state: usize,
    pub table: *const c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv07_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSEv07_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
}

#[inline(always)]
unsafe fn FSEv07_initDState(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
    dt: *const FSEv07_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv07_DTableHeader;
    (*DStatePtr).state = BITv07_readBits(bitD, (*DTableH).tableLog as U32);
    BITv07_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

#[inline(always)]
unsafe fn FSEv07_peekSymbol(DStatePtr: *const FSEv07_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

#[inline(always)]
unsafe fn FSEv07_updateState(DStatePtr: *mut FSEv07_DState_t, bitD: *mut BITv07_DStream_t) {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let lowBits = BITv07_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
}

#[inline(always)]
unsafe fn FSEv07_decodeSymbol(DStatePtr: *mut FSEv07_DState_t, bitD: *mut BITv07_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv07_readBits(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

#[inline(always)]
unsafe fn FSEv07_decodeSymbolFast(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv07_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = (DInfo.newState as usize).wrapping_add(lowBits);
    symbol
}

/* ===================================================================
 * huf.h  (HUFv07_*)
 * =================================================================== */

pub const HUFv07_BLOCKSIZE_MAX: usize = 128 * 1024;
pub const HUFv07_TABLELOG_ABSOLUTEMAX: usize = 16;
pub const HUFv07_TABLELOG_MAX: usize = 12;
pub const HUFv07_TABLELOG_DEFAULT: usize = 11;
pub const HUFv07_SYMBOLVALUE_MAX: usize = 255;

pub type HUFv07_DTable = U32;

#[inline(always)]
const fn HUFv07_DTABLE_SIZE(maxTableLog: usize) -> usize {
    1 + (1usize << maxTableLog)
}

/* ===================================================================
 * entropy_common.c
 * =================================================================== */

/*-****************************************
*  FSE Error Management
******************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
*  HUF Error Management
****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/*-**************************************************************
*  FSE NCount encoding-decoding
****************************************************************/
unsafe fn FSEv07_abs(a: i16) -> i16 {
    if a < 0 {
        (a as i32).wrapping_neg() as i16
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
    nbBits = ((bitStream & 0xF) + FSEv07_MIN_TABLELOG) as c_int; /* extract tableLog */
    if nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX as c_int {
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
                *normalizedCounter.add(charnum as usize) = 0;
                charnum = charnum.wrapping_add(1);
            }
            if ((ip as usize) <= (iend as usize).wrapping_sub(7))
                || ((ip as isize).wrapping_add((bitCount >> 3) as isize) as usize
                    <= (iend as usize).wrapping_sub(4))
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr(bitCount as u32);
            } else {
                bitStream >>= 2;
            }
        }
        {
            let max: i16 = ((2 * threshold - 1) - remaining) as i16;
            let mut count: i16;

            if (bitStream & ((threshold as U32).wrapping_sub(1))) < (max as U32) {
                count = (bitStream & ((threshold as U32).wrapping_sub(1))) as i16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & ((2 * threshold - 1) as U32)) as i16;
                if count as c_int >= threshold {
                    count = count.wrapping_sub(max);
                }
                bitCount += nbBits;
            }

            count = count.wrapping_sub(1); /* extra accuracy */
            remaining -= FSEv07_abs(count) as c_int;
            *normalizedCounter.add(charnum as usize) = count;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as c_int;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            if ((ip as usize) <= (iend as usize).wrapping_sub(7))
                || ((ip as isize).wrapping_add((bitCount >> 3) as isize) as usize
                    <= (iend as usize).wrapping_sub(4))
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * ((iend as isize) - 4 - (ip as isize))) as c_int;
                ip = iend.sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void).wrapping_shr((bitCount & 31) as u32);
        }
    } /* while ((remaining>1) && (charnum<=*maxSVPtr)) */
    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    *maxSVPtr = charnum.wrapping_sub(1);

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    if (ip as usize - istart as usize) > hbSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip as usize - istart as usize
}

static HUFv07_readStats_l: [U32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readStats(
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
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            oSize = HUFv07_readStats_l[iSize - 242] as usize;
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

    /* collect weight stats */
    ZSTD_memset(
        rankStats as *mut c_void,
        0,
        (HUFv07_TABLELOG_ABSOLUTEMAX + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    {
        let mut n: U32 = 0;
        while (n as usize) < oSize {
            if *huffWeight.add(n as usize) as usize >= HUFv07_TABLELOG_ABSOLUTEMAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *rankStats.add(*huffWeight.add(n as usize) as usize) += 1;
            weightTotal = weightTotal.wrapping_add((1u32 << *huffWeight.add(n as usize)) >> 1);
            n += 1;
        }
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    {
        let tableLog: U32 = BITv07_highbit32(weightTotal) + 1;
        if tableLog as usize > HUFv07_TABLELOG_ABSOLUTEMAX {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *tableLogPtr = tableLog;
        /* determine last weight */
        {
            let total: U32 = 1u32 << tableLog;
            let rest: U32 = total.wrapping_sub(weightTotal);
            let verif: U32 = 1u32 << BITv07_highbit32(rest);
            let lastWeight: U32 = BITv07_highbit32(rest) + 1;
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *huffWeight.add(oSize) = lastWeight as BYTE;
            *rankStats.add(lastWeight as usize) += 1;
        }
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

/* ===================================================================
 * fse_decompress.c
 * =================================================================== */

/* Function templates */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_createDTable(mut tableLog: c_uint) -> *mut FSEv07_DTable {
    if tableLog > FSEv07_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv07_TABLELOG_ABSOLUTE_MAX;
    }
    malloc(FSEv07_DTABLE_SIZE_U32(tableLog as usize) * core::mem::size_of::<U32>())
        as *mut FSEv07_DTable
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
    let mut symbolNext: [U16; FSEv07_MAX_SYMBOL_VALUE as usize + 1] =
        [0; FSEv07_MAX_SYMBOL_VALUE as usize + 1];

    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);
    let tableSize: U32 = 1u32 << tableLog;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);

    /* Sanity Checks */
    if maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSEv07_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    {
        let mut DTableH = FSEv07_DTableHeader {
            tableLog: 0,
            fastMode: 0,
        };
        DTableH.tableLog = tableLog as U16;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
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
                s += 1;
            }
        }
        ZSTD_memcpy(
            dt as *mut c_void,
            &DTableH as *const FSEv07_DTableHeader as *const c_void,
            core::mem::size_of::<FSEv07_DTableHeader>(),
        );
    }

    /* Spread symbols */
    {
        let tableMask: U32 = tableSize.wrapping_sub(1);
        let step: U32 = FSEv07_TABLESTEP(tableSize);
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
            s += 1;
        }

        if position != 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
    }

    /* Build Decoding table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol: BYTE = (*tableDecode.add(u as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.add(u as usize)).nbBits =
                (tableLog.wrapping_sub(BITv07_highbit32(nextState as U32))) as BYTE;
            (*tableDecode.add(u as usize)).newState = (((nextState as U32)
                << (*tableDecode.add(u as usize)).nbBits)
            .wrapping_sub(tableSize)) as U16;
            u += 1;
        }
    }

    0
}

/*-*******************************************************
*  Decompression (Byte symbols)
*********************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable_rle(dt: *mut FSEv07_DTable, symbolValue: BYTE) -> usize {
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
    let tableSize: c_uint = 1u32 << nbBits;
    let tableMask: c_uint = tableSize.wrapping_sub(1);
    let maxSV1: c_uint = tableMask.wrapping_add(1);

    /* Sanity checks */
    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC);
    }

    /* Build Decoding Table */
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    {
        let mut s: c_uint = 0;
        while s < maxSV1 {
            (*dinfo.add(s as usize)).newState = 0;
            (*dinfo.add(s as usize)).symbol = s as BYTE;
            (*dinfo.add(s as usize)).nbBits = nbBits as BYTE;
            s += 1;
        }
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
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = (ostart as usize).wrapping_add(maxDstSize) as *mut BYTE;
    let olimit = (omax as usize).wrapping_sub(3) as *mut BYTE;

    let mut bitD = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1 = FSEv07_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2 = FSEv07_DState_t {
        state: 0,
        table: core::ptr::null(),
    };

    /* Init */
    {
        let errorCode = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if FSEv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv07_initDState(&mut state1, &mut bitD, dt);
    FSEv07_initDState(&mut state2, &mut bitD, dt);

    /* 4 symbols per loop.
     * The `FSEv07_MAX_TABLELOG*2+7 > sizeof(size_t)*8` and
     * `FSEv07_MAX_TABLELOG*4+7 > sizeof(size_t)*8` static tests are both false
     * on a 64-bit target, so the intermediate reloads are compiled out. */
    while (BITv07_reloadDStream(&mut bitD) == BITv07_DStream_unfinished)
        && ((op as usize) < olimit as usize)
    {
        *op.add(0) = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state1, &mut bitD)
        };
        *op.add(1) = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state2, &mut bitD)
        };
        *op.add(2) = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state1, &mut bitD)
        };
        *op.add(3) = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state2, &mut bitD)
        };
        op = op.add(4);
    }

    /* tail */
    loop {
        if op as usize > (omax as usize).wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state1, &mut bitD)
        };
        op = op.add(1);

        if BITv07_reloadDStream(&mut bitD) == BITv07_DStream_overflow {
            *op = if fast != 0 {
                FSEv07_decodeSymbolFast(&mut state2, &mut bitD)
            } else {
                FSEv07_decodeSymbol(&mut state2, &mut bitD)
            };
            op = op.add(1);
            break;
        }

        if op as usize > (omax as usize).wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = if fast != 0 {
            FSEv07_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSEv07_decodeSymbol(&mut state2, &mut bitD)
        };
        op = op.add(1);

        if BITv07_reloadDStream(&mut bitD) == BITv07_DStream_overflow {
            *op = if fast != 0 {
                FSEv07_decodeSymbolFast(&mut state1, &mut bitD)
            } else {
                FSEv07_decodeSymbol(&mut state1, &mut bitD)
            };
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
    let fastMode: U32 = (*DTableH).fastMode as U32;

    /* select fast mode (static) */
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
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [i16; FSEv07_MAX_SYMBOL_VALUE as usize + 1] =
        [0; FSEv07_MAX_SYMBOL_VALUE as usize + 1];
    let mut dt: [U32; FSEv07_DTABLE_SIZE_U32(FSEv07_MAX_TABLELOG as usize)] =
        [0; FSEv07_DTABLE_SIZE_U32(FSEv07_MAX_TABLELOG as usize)];
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSEv07_MAX_SYMBOL_VALUE;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* normal FSE decoding mode */
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
            return ERROR(ZSTD_error_srcSize_wrong);
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

    FSEv07_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const c_void,
        cSrcSize,
        dt.as_ptr(),
    )
}

/* ===================================================================
 * huf_decompress.c
 * =================================================================== */

/*-***************************/
/*  generic DTableDesc       */
/*-***************************/

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DTableDesc {
    pub maxTableLog: BYTE,
    pub tableType: BYTE,
    pub tableLog: BYTE,
    pub reserved: BYTE,
}

unsafe fn HUFv07_getDTableDesc(table: *const HUFv07_DTable) -> DTableDesc {
    let mut dtd = DTableDesc {
        maxTableLog: 0,
        tableType: 0,
        tableLog: 0,
        reserved: 0,
    };
    ZSTD_memcpy(
        &mut dtd as *mut DTableDesc as *mut c_void,
        table as *const c_void,
        core::mem::size_of::<DTableDesc>(),
    );
    dtd
}

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUFv07_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX2(
    DTable: *mut HUFv07_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut huffWeight: [BYTE; HUFv07_SYMBOLVALUE_MAX + 1] = [0; HUFv07_SYMBOLVALUE_MAX + 1];
    let mut rankVal: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1] = [0; HUFv07_TABLELOG_ABSOLUTEMAX + 1];
    let mut tableLog: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let iSize: usize;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv07_DEltX2;

    iSize = HUFv07_readStats(
        huffWeight.as_mut_ptr(),
        HUFv07_SYMBOLVALUE_MAX + 1,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }

    /* Table header */
    {
        let mut dtd = HUFv07_getDTableDesc(DTable);
        if tableLog > (dtd.maxTableLog as U32 + 1) {
            return ERROR(ZSTD_error_tableLog_tooLarge);
        }
        dtd.tableType = 0;
        dtd.tableLog = tableLog as BYTE;
        ZSTD_memcpy(
            DTable as *mut c_void,
            &dtd as *const DTableDesc as *const c_void,
            core::mem::size_of::<DTableDesc>(),
        );
    }

    /* Prepare ranks */
    {
        let mut n: U32 = 1;
        let mut nextRankStart: U32 = 0;
        while n < tableLog + 1 {
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankVal[n as usize] << (n - 1));
            rankVal[n as usize] = current;
            n += 1;
        }
    }

    /* fill DTable */
    {
        let mut n: U32 = 0;
        while n < nbSymbols {
            let w: U32 = huffWeight[n as usize] as U32;
            let length: U32 = (1u32 << w) >> 1;
            let mut i: U32;
            let mut D = HUFv07_DEltX2 { byte: 0, nbBits: 0 };
            D.byte = n as BYTE;
            D.nbBits = (tableLog + 1 - w) as BYTE;
            i = rankVal[w as usize];
            while i < rankVal[w as usize].wrapping_add(length) {
                *dt.add(i as usize) = D;
                i += 1;
            }
            rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
            n += 1;
        }
    }

    iSize
}

unsafe fn HUFv07_decodeSymbolX2(
    Dstream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BITv07_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.add(val)).byte;
    BITv07_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

/* HUFv07_DECODE_SYMBOLX2_0 / _1 / _2 : on a 64-bit target all three expand to
 * an unconditional decode. */
#[inline(always)]
unsafe fn HUFv07_DECODE_SYMBOLX2_0(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) {
    **p = HUFv07_decodeSymbolX2(DStreamPtr, dt, dtLog);
    *p = (*p).add(1);
}

#[inline(always)]
unsafe fn HUFv07_DECODE_SYMBOLX2_1(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) {
    if MEM_64bits() != 0 || (HUFv07_TABLELOG_MAX <= 12) {
        HUFv07_DECODE_SYMBOLX2_0(p, DStreamPtr, dt, dtLog);
    }
}

#[inline(always)]
unsafe fn HUFv07_DECODE_SYMBOLX2_2(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) {
    if MEM_64bits() != 0 {
        HUFv07_DECODE_SYMBOLX2_0(p, DStreamPtr, dt, dtLog);
    }
}

unsafe fn HUFv07_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 4 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished)
        && (p as usize <= (pEnd as usize).wrapping_sub(4))
    {
        HUFv07_DECODE_SYMBOLX2_2(&mut p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX2_1(&mut p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX2_2(&mut p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished)
        && ((p as usize) < pEnd as usize)
    {
        HUFv07_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while (p as usize) < pEnd as usize {
        HUFv07_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
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
    let op = dst as *mut BYTE;
    let oend = (op as usize).wrapping_add(dstSize) as *mut BYTE;
    let dtPtr = DTable.add(1) as *const c_void;
    let dt = dtPtr as *const HUFv07_DEltX2;
    let mut bitD = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let dtd = HUFv07_getDTableDesc(DTable);
    let dtLog: U32 = dtd.tableLog as U32;

    {
        let errorCode = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv07_decodeStreamX2(op, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv07_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
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
        return ERROR(ZSTD_error_GENERIC);
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
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv07_readDTableX2(DCtx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
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
    /* HUFv07_CREATE_STATIC_DTABLEX2(DTable, HUFv07_TABLELOG_MAX) */
    let mut DTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)] =
        [0; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)];
    DTable[0] = ((HUFv07_TABLELOG_MAX - 1) as U32).wrapping_mul(0x1000001);
    HUFv07_decompress1X2_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

unsafe fn HUFv07_decompress4X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUFv07_DTable,
) -> usize {
    /* Check */
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = (ostart as usize).wrapping_add(dstSize) as *mut BYTE;
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX2;

        /* Init */
        let mut bitD1 = BITv07_DStream_t {
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
        let length4: usize =
            cSrcSize.wrapping_sub(length1.wrapping_add(length2).wrapping_add(length3) + 6);
        let istart1 = istart.add(6); /* jumpTable */
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = (ostart as usize).wrapping_add(segmentSize) as *mut BYTE;
        let opStart3 = (opStart2 as usize).wrapping_add(segmentSize) as *mut BYTE;
        let opStart4 = (opStart3 as usize).wrapping_add(segmentSize) as *mut BYTE;
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: U32;
        let dtd = HUFv07_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        {
            let errorCode = BITv07_initDStream(&mut bitD1, istart1 as *const c_void, length1);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(&mut bitD2, istart2 as *const c_void, length2);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(&mut bitD3, istart3 as *const c_void, length3);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(&mut bitD4, istart4 as *const c_void, length4);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv07_reloadDStream(&mut bitD1)
            | BITv07_reloadDStream(&mut bitD2)
            | BITv07_reloadDStream(&mut bitD3)
            | BITv07_reloadDStream(&mut bitD4);
        while (endSignal == BITv07_DStream_unfinished)
            && ((op4 as usize) < (oend as usize).wrapping_sub(7))
        {
            HUFv07_DECODE_SYMBOLX2_2(&mut op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2(&mut op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2(&mut op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2(&mut op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_1(&mut op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_1(&mut op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_1(&mut op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_1(&mut op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2(&mut op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2(&mut op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2(&mut op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_2(&mut op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_0(&mut op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_0(&mut op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_0(&mut op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX2_0(&mut op4, &mut bitD4, dt, dtLog);
            endSignal = BITv07_reloadDStream(&mut bitD1)
                | BITv07_reloadDStream(&mut bitD2)
                | BITv07_reloadDStream(&mut bitD3)
                | BITv07_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 as usize > opStart2 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 as usize > opStart3 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 as usize > opStart4 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUFv07_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv07_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv07_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv07_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BITv07_endOfDStream(&bitD1)
            & BITv07_endOfDStream(&bitD2)
            & BITv07_endOfDStream(&bitD3)
            & BITv07_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* decoded size */
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
        return ERROR(ZSTD_error_GENERIC);
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
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv07_readDTableX2(dctx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
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
    let mut DTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)] =
        [0; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX - 1)];
    DTable[0] = ((HUFv07_TABLELOG_MAX - 1) as U32).wrapping_mul(0x1000001);
    HUFv07_decompress4X2_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

/* *************************/
/* double-symbols decoding */
/* *************************/

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUFv07_DEltX4 {
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

unsafe fn HUFv07_fillDTableX4Level2(
    DTable: *mut HUFv07_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: c_int,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    let mut DElt = HUFv07_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1] = [0; HUFv07_TABLELOG_ABSOLUTEMAX + 1];

    /* get pre-calculated rankVal */
    ZSTD_memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: U32 = 0;
        let skipSize: U32 = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1;
        while i < skipSize {
            *DTable.add(i as usize) = DElt;
            i += 1;
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
            let length: U32 = 1u32 << (sizeLog.wrapping_sub(nbBits));
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
            } /* since length >= 1 */

            rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
            s += 1;
        }
    }
}

pub type rankVal_t = [[U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1]; HUFv07_TABLELOG_ABSOLUTEMAX];

unsafe fn HUFv07_fillDTableX4(
    DTable: *mut HUFv07_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1] = [0; HUFv07_TABLELOG_ABSOLUTEMAX + 1];
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int; /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    ZSTD_memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1]>(),
    );

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32 << (targetLog.wrapping_sub(nbBits));

        if targetLog.wrapping_sub(nbBits) >= minBits {
            /* enough room for a second symbol */
            let sortedRank: U32;
            let mut minWeight: c_int = nbBits.wrapping_add(scaleLog as U32) as c_int;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUFv07_fillDTableX4Level2(
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
            let mut DElt = HUFv07_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };
            MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1;
            {
                let mut u: U32 = start;
                let end: U32 = start.wrapping_add(length);
                while u < end {
                    *DTable.add(u as usize) = DElt;
                    u += 1;
                }
            }
        }
        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX4(
    DTable: *mut HUFv07_DTable,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightList: [BYTE; HUFv07_SYMBOLVALUE_MAX + 1] = [0; HUFv07_SYMBOLVALUE_MAX + 1];
    let mut sortedSymbol: [sortedSymbol_t; HUFv07_SYMBOLVALUE_MAX + 1] =
        [sortedSymbol_t {
            symbol: 0,
            weight: 0,
        }; HUFv07_SYMBOLVALUE_MAX + 1];
    let mut rankStats: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 1] =
        [0; HUFv07_TABLELOG_ABSOLUTEMAX + 1];
    let mut rankStart0: [U32; HUFv07_TABLELOG_ABSOLUTEMAX + 2] =
        [0; HUFv07_TABLELOG_ABSOLUTEMAX + 2];
    let rankStart = rankStart0.as_mut_ptr().add(1);
    let mut rankVal: rankVal_t = [[0; HUFv07_TABLELOG_ABSOLUTEMAX + 1]; HUFv07_TABLELOG_ABSOLUTEMAX];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let mut dtd = HUFv07_getDTableDesc(DTable);
    let maxTableLog: U32 = dtd.maxTableLog as U32;
    let iSize: usize;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv07_DEltX4;

    if maxTableLog as usize > HUFv07_TABLELOG_ABSOLUTEMAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUFv07_readStats(
        weightList.as_mut_ptr(),
        HUFv07_SYMBOLVALUE_MAX + 1,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > maxTableLog {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW = maxW.wrapping_sub(1);
    }

    /* Get start index of each weight */
    {
        let mut w: U32 = 1;
        let mut nextRankStart: U32 = 0;
        while w < maxW + 1 {
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankStats[w as usize]);
            *rankStart.add(w as usize) = current;
            w += 1;
        }
        *rankStart.add(0) = nextRankStart; /* put all 0w symbols at the end of sorted list*/
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
            s += 1;
        }
        *rankStart.add(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let rankVal0: *mut U32 = rankVal.as_mut_ptr() as *mut U32; /* rankVal[0] */
        {
            let rescale: c_int = (maxTableLog.wrapping_sub(tableLog)) as c_int - 1; /* tableLog <= maxTableLog */
            let mut nextRankVal: U32 = 0;
            let mut w: U32 = 1;
            while w < maxW + 1 {
                let current: U32 = nextRankVal;
                nextRankVal = nextRankVal.wrapping_add(
                    rankStats[w as usize].wrapping_shl((w as c_int + rescale) as u32),
                );
                *rankVal0.add(w as usize) = current;
                w += 1;
            }
        }
        {
            let minBits: U32 = tableLog + 1 - maxW;
            let mut consumed: U32 = minBits;
            while consumed < maxTableLog.wrapping_sub(minBits).wrapping_add(1) {
                let rankValPtr: *mut U32 =
                    rankVal0.add(consumed as usize * (HUFv07_TABLELOG_ABSOLUTEMAX + 1));
                let mut w: U32 = 1;
                while w < maxW + 1 {
                    *rankValPtr.add(w as usize) = *rankVal0.add(w as usize) >> consumed;
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
        rankVal.as_ptr(),
        maxW,
        tableLog + 1,
    );

    dtd.tableLog = maxTableLog as BYTE;
    dtd.tableType = 1;
    ZSTD_memcpy(
        DTable as *mut c_void,
        &dtd as *const DTableDesc as *const c_void,
        core::mem::size_of::<DTableDesc>(),
    );
    iSize
}

unsafe fn HUFv07_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv07_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    ZSTD_memcpy(op, dt.add(val) as *const c_void, 2);
    BITv07_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

unsafe fn HUFv07_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv07_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    ZSTD_memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BITv07_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if ((*DStream).bitsConsumed as usize) < core::mem::size_of::<usize>() * 8 {
            BITv07_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed as usize > core::mem::size_of::<usize>() * 8 {
                /* ugly hack; works only because it's the last symbol. */
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as c_uint;
            }
        }
    }
    1
}

#[inline(always)]
unsafe fn HUFv07_DECODE_SYMBOLX4_0(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) {
    let n = HUFv07_decodeSymbolX4(*p as *mut c_void, DStreamPtr, dt, dtLog);
    *p = (*p).add(n as usize);
}

#[inline(always)]
unsafe fn HUFv07_DECODE_SYMBOLX4_1(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) {
    if MEM_64bits() != 0 || (HUFv07_TABLELOG_MAX <= 12) {
        HUFv07_DECODE_SYMBOLX4_0(p, DStreamPtr, dt, dtLog);
    }
}

#[inline(always)]
unsafe fn HUFv07_DECODE_SYMBOLX4_2(
    p: &mut *mut BYTE,
    DStreamPtr: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) {
    if MEM_64bits() != 0 {
        HUFv07_DECODE_SYMBOLX4_0(p, DStreamPtr, dt, dtLog);
    }
}

unsafe fn HUFv07_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished)
        && ((p as usize) < (pEnd as usize).wrapping_sub(7))
    {
        HUFv07_DECODE_SYMBOLX4_2(&mut p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX4_1(&mut p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX4_2(&mut p, bitDPtr, dt, dtLog);
        HUFv07_DECODE_SYMBOLX4_0(&mut p, bitDPtr, dt, dtLog);
    }

    /* closer to end : up to 2 symbols at a time */
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished)
        && (p as usize <= (pEnd as usize).wrapping_sub(2))
    {
        HUFv07_DECODE_SYMBOLX4_0(&mut p, bitDPtr, dt, dtLog);
    }

    while p as usize <= (pEnd as usize).wrapping_sub(2) {
        HUFv07_DECODE_SYMBOLX4_0(&mut p, bitDPtr, dt, dtLog); /* no need to reload : reached the end of DStream */
    }

    if (p as usize) < pEnd as usize {
        let n = HUFv07_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog);
        p = p.add(n as usize);
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
    let mut bitD = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };

    /* Init */
    {
        let errorCode = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    /* decode */
    {
        let ostart = dst as *mut BYTE;
        let oend = (ostart as usize).wrapping_add(dstSize) as *mut BYTE;
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX4;
        let dtd = HUFv07_getDTableDesc(DTable);
        HUFv07_decodeStreamX4(ostart, &mut bitD, oend, dt, dtd.tableLog as U32);
    }

    /* check */
    if BITv07_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* decoded size */
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
        return ERROR(ZSTD_error_GENERIC);
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
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv07_readDTableX4(DCtx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
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
    /* HUFv07_CREATE_STATIC_DTABLEX4(DTable, HUFv07_TABLELOG_MAX) */
    let mut DTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)] =
        [0; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)];
    DTable[0] = (HUFv07_TABLELOG_MAX as U32).wrapping_mul(0x1000001);
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
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = (ostart as usize).wrapping_add(dstSize) as *mut BYTE;
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX4;

        /* Init */
        let mut bitD1 = BITv07_DStream_t {
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
        let length4: usize =
            cSrcSize.wrapping_sub(length1.wrapping_add(length2).wrapping_add(length3) + 6);
        let istart1 = istart.add(6); /* jumpTable */
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = (ostart as usize).wrapping_add(segmentSize) as *mut BYTE;
        let opStart3 = (opStart2 as usize).wrapping_add(segmentSize) as *mut BYTE;
        let opStart4 = (opStart3 as usize).wrapping_add(segmentSize) as *mut BYTE;
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: U32;
        let dtd = HUFv07_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        {
            let errorCode = BITv07_initDStream(&mut bitD1, istart1 as *const c_void, length1);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(&mut bitD2, istart2 as *const c_void, length2);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(&mut bitD3, istart3 as *const c_void, length3);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }
        {
            let errorCode = BITv07_initDStream(&mut bitD4, istart4 as *const c_void, length4);
            if HUFv07_isError(errorCode) != 0 {
                return errorCode;
            }
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        endSignal = BITv07_reloadDStream(&mut bitD1)
            | BITv07_reloadDStream(&mut bitD2)
            | BITv07_reloadDStream(&mut bitD3)
            | BITv07_reloadDStream(&mut bitD4);
        while (endSignal == BITv07_DStream_unfinished)
            && ((op4 as usize) < (oend as usize).wrapping_sub(7))
        {
            HUFv07_DECODE_SYMBOLX4_2(&mut op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2(&mut op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2(&mut op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2(&mut op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_1(&mut op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_1(&mut op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_1(&mut op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_1(&mut op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2(&mut op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2(&mut op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2(&mut op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_2(&mut op4, &mut bitD4, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_0(&mut op1, &mut bitD1, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_0(&mut op2, &mut bitD2, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_0(&mut op3, &mut bitD3, dt, dtLog);
            HUFv07_DECODE_SYMBOLX4_0(&mut op4, &mut bitD4, dt, dtLog);

            endSignal = BITv07_reloadDStream(&mut bitD1)
                | BITv07_reloadDStream(&mut bitD2)
                | BITv07_reloadDStream(&mut bitD3)
                | BITv07_reloadDStream(&mut bitD4);
        }

        /* check corruption */
        if op1 as usize > opStart2 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 as usize > opStart3 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 as usize > opStart4 as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUFv07_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv07_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv07_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv07_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        {
            let endCheck: U32 = BITv07_endOfDStream(&bitD1)
                & BITv07_endOfDStream(&bitD2)
                & BITv07_endOfDStream(&bitD3)
                & BITv07_endOfDStream(&bitD4);
            if endCheck == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }

        /* decoded size */
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
        return ERROR(ZSTD_error_GENERIC);
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
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv07_readDTableX4(dctx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
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
    let mut DTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)] =
        [0; HUFv07_DTABLE_SIZE(HUFv07_TABLELOG_MAX)];
    DTable[0] = (HUFv07_TABLELOG_MAX as U32).wrapping_mul(0x1000001);
    HUFv07_decompress4X4_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

/* ********************************/
/* Generic decompression selector */
/* ********************************/

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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
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
    [at!(0, 0), at!(1, 1), at!(2, 2)],           /* Q==0 : impossible */
    [at!(0, 0), at!(1, 1), at!(2, 2)],           /* Q==1 : impossible */
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_selectDecoder(dstSize: usize, cSrcSize: usize) -> U32 {
    /* decoder timing evaluation */
    let Q: U32 = (cSrcSize * 16 / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
    let D256: U32 = (dstSize >> 8) as U32;
    let at0: *const algo_time_t = (algoTime.as_ptr() as *const algo_time_t)
        .add(Q as usize * 3);
    let DTime0: U32 = (*at0.add(0))
        .tableTime
        .wrapping_add((*at0.add(0)).decode256Time.wrapping_mul(D256));
    let mut DTime1: U32 = (*at0.add(1))
        .tableTime
        .wrapping_add((*at0.add(1)).decode256Time.wrapping_mul(D256));
    DTime1 = DTime1.wrapping_add(DTime1 >> 3); /* advantage to algorithm using less memory, for cache eviction */

    (DTime1 < DTime0) as U32
}

pub type decompressionAlgo =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;

static HUFv07_decompress_algos: [decompressionAlgo; 2] =
    [HUFv07_decompress4X2, HUFv07_decompress4X4];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
) -> usize {
    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if cSrcSize == dstSize {
        ZSTD_memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        ZSTD_memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    }

    {
        let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize);
        HUFv07_decompress_algos[algoNb as usize](dst, dstSize, cSrc, cSrcSize)
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
    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if cSrcSize == dstSize {
        ZSTD_memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        ZSTD_memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    }

    {
        let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize);
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
    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if (cSrcSize >= dstSize) || (cSrcSize <= 1) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize);
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
    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if cSrcSize == dstSize {
        ZSTD_memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        ZSTD_memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    }

    {
        let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUFv07_decompress1X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv07_decompress1X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
        }
    }
}

/* ===================================================================
 * zstd_common.c
 * =================================================================== */

/*-****************************************
*  ZSTD Error Management
******************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
*  ZBUFF Error Management
****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

unsafe extern "C" fn ZSTDv07_defaultAllocFunction(
    _opaque: *mut c_void,
    size: usize,
) -> *mut c_void {
    let address = malloc(size);
    address
}

unsafe extern "C" fn ZSTDv07_defaultFreeFunction(_opaque: *mut c_void, address: *mut c_void) {
    free(address);
}

/* ===================================================================
 * zstd_internal.h
 * =================================================================== */

pub const ZSTDv07_OPT_NUM: usize = 1 << 12;
pub const ZSTDv07_DICT_MAGIC: U32 = 0xEC30A437;

pub const ZSTDv07_REP_NUM: usize = 3;
pub const ZSTDv07_REP_INIT: usize = ZSTDv07_REP_NUM;
pub const ZSTDv07_REP_MOVE: usize = ZSTDv07_REP_NUM - 1;
static repStartValue: [U32; ZSTDv07_REP_NUM] = [1, 4, 8];

pub const ZSTDv07_WINDOWLOG_ABSOLUTEMIN: c_uint = 10;
static ZSTDv07_fcs_fieldSize: [usize; 4] = [0, 2, 4, 8];
static ZSTDv07_did_fieldSize: [usize; 4] = [0, 1, 2, 4];

pub const ZSTDv07_BLOCKHEADERSIZE: usize = 3;
const ZSTDv07_blockHeaderSize: usize = ZSTDv07_BLOCKHEADERSIZE;

pub type blockType_t = c_uint;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

pub const MIN_SEQUENCES_SIZE: usize = 1;
pub const MIN_CBLOCK_SIZE: usize = 1 + 1 + MIN_SEQUENCES_SIZE;

pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: usize = 12;

pub type litBlockType_t = c_uint;
pub const lbt_huffman: litBlockType_t = 0;
pub const lbt_repeat: litBlockType_t = 1;
pub const lbt_raw: litBlockType_t = 2;
pub const lbt_rle: litBlockType_t = 3;

pub const LONGNBSEQ: c_int = 0x7F00;

pub const MINMATCH: usize = 3;
pub const EQUAL_READ32: usize = 4;

pub const Litbits: usize = 8;
pub const MaxLit: usize = (1 << Litbits) - 1;
pub const MaxML: usize = 52;
pub const MaxLL: usize = 35;
pub const MaxOff: usize = 28;
pub const MaxSeq: usize = if MaxLL > MaxML { MaxLL } else { MaxML };
pub const MLFSELog: c_uint = 9;
pub const LLFSELog: c_uint = 9;
pub const OffFSELog: c_uint = 8;

pub const FSEv07_ENCODING_RAW: U32 = 0;
pub const FSEv07_ENCODING_RLE: U32 = 1;
pub const FSEv07_ENCODING_STATIC: U32 = 2;
pub const FSEv07_ENCODING_DYNAMIC: U32 = 3;

pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

static LL_bits: [U32; MaxLL + 1] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
static LL_defaultNorm: [S16; MaxLL + 1] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
static LL_defaultNormLog: U32 = 6;

static ML_bits: [U32; MaxML + 1] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
static ML_defaultNorm: [S16; MaxML + 1] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
static ML_defaultNormLog: U32 = 6;

static OF_defaultNorm: [S16; MaxOff + 1] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
static OF_defaultNormLog: U32 = 5;

/*-*******************************************
*  Shared functions to include for inlining
*********************************************/
#[inline(always)]
unsafe fn ZSTDv07_copy8(dst: *mut c_void, src: *const c_void) {
    let v = (src as *const u64).read_unaligned();
    (dst as *mut u64).write_unaligned(v);
}

pub const WILDCOPY_OVERLENGTH: usize = 8;

/* ZSTDv07_wildcopy() :
 * custom version of memcpy(), can copy up to 7 bytes too many (8 bytes if length==0) */
unsafe fn ZSTDv07_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = (op as isize).wrapping_add(length) as *mut BYTE;
    loop {
        ZSTDv07_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if !((op as usize) < oend as usize) {
            break;
        }
    }
}

/* custom memory allocation functions */
const defaultCustomMem: ZSTDv07_customMem = ZSTDv07_customMem {
    customAlloc: Some(ZSTDv07_defaultAllocFunction),
    customFree: Some(ZSTDv07_defaultFreeFunction),
    opaque: core::ptr::null_mut(),
};

/* ===================================================================
 * zstd_decompress.c
 * =================================================================== */

#[inline(always)]
unsafe fn ZSTDv07_copy4(dst: *mut c_void, src: *const c_void) {
    let v = (src as *const u32).read_unaligned();
    (dst as *mut u32).write_unaligned(v);
}

/*-*************************************************************
*   Context management
***************************************************************/
pub type ZSTDv07_dStage = c_uint;
pub const ZSTDds_getFrameHeaderSize: ZSTDv07_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTDv07_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTDv07_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTDv07_dStage = 3;
pub const ZSTDds_decodeSkippableHeader: ZSTDv07_dStage = 4;
pub const ZSTDds_skipFrame: ZSTDv07_dStage = 5;

#[repr(C)]
pub struct ZSTDv07_DCtx {
    pub LLTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(LLFSELog as usize)],
    pub OffTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(OffFSELog as usize)],
    pub MLTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32(MLFSELog as usize)],
    pub hufTable: [HUFv07_DTable; HUFv07_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: usize,
    pub rep: [U32; 3],
    pub fParams: ZSTDv07_frameParams,
    pub bType: blockType_t,
    pub stage: ZSTDv07_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: usize,
    pub dictID: U32,
    pub litPtr: *const BYTE,
    pub customMem: ZSTDv07_customMem,
    pub litSize: usize,
    pub litBuffer: [BYTE; ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [BYTE; ZSTDv07_FRAMEHEADERSIZE_MAX],
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
    (*dctx).hufTable[0] = (ZSTD_HUFFDTABLE_CAPACITY_LOG as U32).wrapping_mul(0x1000001);
    (*dctx).fseEntropy = 0;
    (*dctx).litEntropy = (*dctx).fseEntropy;
    (*dctx).dictID = 0;
    {
        let mut i: c_int = 0;
        while i < ZSTDv07_REP_NUM as c_int {
            (*dctx).rep[i as usize] = repStartValue[i as usize];
            i += 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDCtx_advanced(
    mut customMem: ZSTDv07_customMem,
) -> *mut ZSTDv07_DCtx {
    let dctx: *mut ZSTDv07_DCtx;

    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem;
    }

    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    dctx = (customMem.customAlloc.unwrap())(
        customMem.opaque,
        core::mem::size_of::<ZSTDv07_DCtx>(),
    ) as *mut ZSTDv07_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_memcpy(
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
pub unsafe extern "C" fn ZSTDv07_copyDCtx(
    dstDCtx: *mut ZSTDv07_DCtx,
    srcDCtx: *const ZSTDv07_DCtx,
) {
    ZSTD_memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv07_DCtx>()
            - (ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + WILDCOPY_OVERLENGTH + ZSTDv07_frameHeaderSize_max),
    );
}

/* ZSTDv07_frameHeaderSize() :
 * srcSize must be >= ZSTDv07_frameHeaderSize_min.
 * @return : size of the Frame Header */
unsafe fn ZSTDv07_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let fhd: BYTE = *(src as *const BYTE).add(4);
        let dictID: U32 = (fhd & 3) as U32;
        let directMode: U32 = ((fhd >> 5) & 1) as U32;
        let fcsId: U32 = (fhd >> 6) as U32;
        ZSTDv07_frameHeaderSize_min
            + ((directMode == 0) as usize)
            + ZSTDv07_did_fieldSize[dictID as usize]
            + ZSTDv07_fcs_fieldSize[fcsId as usize]
            + ((directMode != 0 && ZSTDv07_fcs_fieldSize[fcsId as usize] == 0) as usize)
    }
}

/* ZSTDv07_getFrameParams() :
 * decode Frame Header, or require larger `srcSize`. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getFrameParams(
    fparamsPtr: *mut ZSTDv07_frameParams,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip = src as *const BYTE;

    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ZSTDv07_frameHeaderSize_min;
    }
    ZSTD_memset(
        fparamsPtr as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv07_frameParams>(),
    );
    if MEM_readLE32(src) != ZSTDv07_MAGICNUMBER {
        if (MEM_readLE32(src) & 0xFFFFFFF0u32) == ZSTDv07_MAGIC_SKIPPABLE_START {
            if srcSize < ZSTDv07_skippableHeaderSize {
                return ZSTDv07_skippableHeaderSize;
            }
            (*fparamsPtr).frameContentSize =
                MEM_readLE32((src as *const c_char).add(4) as *const c_void) as u64;
            (*fparamsPtr).windowSize = 0; /* windowSize==0 means a frame is skippable */
            return 0;
        }
        return ERROR(ZSTD_error_prefix_unknown);
    }

    /* ensure there is enough `srcSize` to fully read/decode frame header */
    {
        let fhsize = ZSTDv07_frameHeaderSize(src, srcSize);
        if srcSize < fhsize {
            return fhsize;
        }
    }

    {
        let fhdByte: BYTE = *ip.add(4);
        let mut pos: usize = 5;
        let dictIDSizeCode: U32 = (fhdByte & 3) as U32;
        let checksumFlag: U32 = ((fhdByte >> 2) & 1) as U32;
        let directMode: U32 = ((fhdByte >> 5) & 1) as U32;
        let fcsID: U32 = (fhdByte >> 6) as U32;
        let windowSizeMax: U32 = 1u32 << ZSTDv07_WINDOWLOG_MAX();
        let mut windowSize: U32 = 0;
        let mut dictID: U32 = 0;
        let mut frameContentSize: U64 = 0;
        if (fhdByte & 0x08) != 0 {
            /* reserved bits, which must be zero */
            return ERROR(ZSTD_error_frameParameter_unsupported);
        }
        if directMode == 0 {
            let wlByte: BYTE = *ip.add(pos);
            pos += 1;
            let windowLog: U32 = ((wlByte >> 3) as U32) + ZSTDv07_WINDOWLOG_ABSOLUTEMIN;
            if windowLog > ZSTDv07_WINDOWLOG_MAX() {
                return ERROR(ZSTD_error_frameParameter_unsupported);
            }
            windowSize = 1u32 << windowLog;
            windowSize = windowSize.wrapping_add((windowSize >> 3) * ((wlByte & 7) as U32));
        }

        match dictIDSizeCode {
            1 => {
                dictID = *ip.add(pos) as U32;
                pos += 1;
            }
            2 => {
                dictID = MEM_readLE16(ip.add(pos) as *const c_void) as U32;
                pos += 2;
            }
            3 => {
                dictID = MEM_readLE32(ip.add(pos) as *const c_void);
                pos += 4;
            }
            _ => {}
        }
        match fcsID {
            1 => {
                frameContentSize = MEM_readLE16(ip.add(pos) as *const c_void) as U64 + 256;
            }
            2 => {
                frameContentSize = MEM_readLE32(ip.add(pos) as *const c_void) as U64;
            }
            3 => {
                frameContentSize = MEM_readLE64(ip.add(pos) as *const c_void);
            }
            _ => {
                if directMode != 0 {
                    frameContentSize = *ip.add(pos) as U64;
                }
            }
        }
        if windowSize == 0 {
            windowSize = frameContentSize as U32;
        }
        if windowSize > windowSizeMax {
            return ERROR(ZSTD_error_frameParameter_unsupported);
        }
        (*fparamsPtr).frameContentSize = frameContentSize;
        (*fparamsPtr).windowSize = windowSize;
        (*fparamsPtr).dictID = dictID;
        (*fparamsPtr).checksumFlag = checksumFlag;
    }
    0
}

/* ZSTDv07_getDecompressedSize() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getDecompressedSize(src: *const c_void, srcSize: usize) -> u64 {
    let mut fparams = ZSTDv07_frameParams {
        frameContentSize: 0,
        windowSize: 0,
        dictID: 0,
        checksumFlag: 0,
    };
    let frResult = ZSTDv07_getFrameParams(&mut fparams, src, srcSize);
    if frResult != 0 {
        return 0;
    }
    fparams.frameContentSize
}

/* ZSTDv07_decodeFrameHeader() */
unsafe fn ZSTDv07_decodeFrameHeader(
    dctx: *mut ZSTDv07_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result = ZSTDv07_getFrameParams(&mut (*dctx).fParams, src, srcSize);
    if (*dctx).fParams.dictID != 0 && ((*dctx).dictID != (*dctx).fParams.dictID) {
        return ERROR(ZSTD_error_dictionary_wrong);
    }
    if (*dctx).fParams.checksumFlag != 0 {
        ZSTD_XXH64_reset(&mut (*dctx).xxhState, 0);
    }
    result
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

/* ZSTDv07_getcBlockSize() :
 * Provides the size of compressed block from block header `src` */
unsafe fn ZSTDv07_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_: *const BYTE = src as *const BYTE;
    let cSize: U32;

    if srcSize < ZSTDv07_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (*bpPtr).blockType = ((*in_) >> 6) as blockType_t;
    cSize = (*in_.add(2) as U32)
        .wrapping_add((*in_.add(1) as U32) << 8)
        .wrapping_add(((*in_.add(0) & 7) as U32) << 16);
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
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0 {
        ZSTD_memcpy(dst, src, srcSize);
    }
    srcSize
}

/* ZSTDv07_decodeLiteralsBlock() :
 * @return : nb of bytes read from src (< srcSize ) */
unsafe fn ZSTDv07_decodeLiteralsBlock(
    dctx: *mut ZSTDv07_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart.add(0) >> 6) as litBlockType_t {
        lbt_huffman => {
            let litSize: usize;
            let litCSize: usize;
            let mut singleStream: usize = 0;
            let mut lhSize: U32 = ((*istart.add(0) >> 4) & 3) as U32;
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            match lhSize {
                2 => {
                    /* 2 - 2 - 14 - 14 */
                    lhSize = 4;
                    litSize = (((*istart.add(0) & 15) as usize) << 10)
                        + ((*istart.add(1) as usize) << 2)
                        + ((*istart.add(2) >> 6) as usize);
                    litCSize =
                        (((*istart.add(2) & 63) as usize) << 8) + (*istart.add(3) as usize);
                }
                3 => {
                    /* 2 - 2 - 18 - 18 */
                    lhSize = 5;
                    litSize = (((*istart.add(0) & 15) as usize) << 14)
                        + ((*istart.add(1) as usize) << 6)
                        + ((*istart.add(2) >> 2) as usize);
                    litCSize = (((*istart.add(2) & 3) as usize) << 16)
                        + ((*istart.add(3) as usize) << 8)
                        + (*istart.add(4) as usize);
                }
                _ => {
                    /* 2 - 2 - 10 - 10 */
                    lhSize = 3;
                    singleStream = (*istart.add(0) & 16) as usize;
                    litSize = (((*istart.add(0) & 15) as usize) << 6)
                        + ((*istart.add(1) >> 2) as usize);
                    litCSize = (((*istart.add(1) & 3) as usize) << 8) + (*istart.add(2) as usize);
                }
            }
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            if HUFv07_isError(if singleStream != 0 {
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
            }) != 0
            {
                return ERROR(ZSTD_error_corruption_detected);
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            (*dctx).litEntropy = 1;
            ZSTD_memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            litCSize + lhSize as usize
        }
        lbt_repeat => {
            let litSize: usize;
            let litCSize: usize;
            let mut lhSize: U32 = ((*istart.add(0) >> 4) & 3) as U32;
            if lhSize != 1 {
                /* only case supported for now : small litSize, single stream */
                return ERROR(ZSTD_error_corruption_detected);
            }
            if (*dctx).litEntropy == 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }

            /* 2 - 2 - 10 - 10 */
            lhSize = 3;
            litSize =
                (((*istart.add(0) & 15) as usize) << 6) + ((*istart.add(1) >> 2) as usize);
            litCSize = (((*istart.add(1) & 3) as usize) << 8) + (*istart.add(2) as usize);
            if litCSize + lhSize as usize > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
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
        lbt_raw => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.add(0) >> 4) & 3) as U32;
            match lhSize {
                2 => {
                    litSize =
                        (((*istart.add(0) & 15) as usize) << 8) + (*istart.add(1) as usize);
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
        lbt_rle => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.add(0) >> 4) & 3) as U32;
            match lhSize {
                2 => {
                    litSize =
                        (((*istart.add(0) & 15) as usize) << 8) + (*istart.add(1) as usize);
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
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
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
        _ => ERROR(ZSTD_error_corruption_detected),
    }
}

/* ZSTDv07_buildSeqTable() */
unsafe fn ZSTDv07_buildSeqTable(
    DTable: *mut FSEv07_DTable,
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
        FSEv07_ENCODING_RLE => {
            if srcSize == 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if (*(src as *const BYTE)) as U32 > max {
                return ERROR(ZSTD_error_corruption_detected);
            }
            FSEv07_buildDTable_rle(DTable, *(src as *const BYTE));
            1
        }
        FSEv07_ENCODING_RAW => {
            FSEv07_buildDTable(DTable, defaultNorm, max, defaultLog);
            0
        }
        FSEv07_ENCODING_STATIC => {
            if flagRepeatTable == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            0
        }
        _ => {
            let mut tableLog: U32 = 0;
            let mut norm: [S16; MaxSeq + 1] = [0; MaxSeq + 1];
            let headerSize = FSEv07_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut tableLog,
                src,
                srcSize,
            );
            if FSEv07_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if tableLog > maxLog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            FSEv07_buildDTable(DTable, norm.as_ptr(), max, tableLog);
            headerSize
        }
    }
}

unsafe fn ZSTDv07_decodeSeqHeaders(
    nbSeqPtr: *mut c_int,
    DTableLL: *mut FSEv07_DTable,
    DTableML: *mut FSEv07_DTable,
    DTableOffb: *mut FSEv07_DTable,
    flagRepeatTable: U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;
    let iend = istart.add(srcSize);
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
                if (ip.add(2) as usize) > iend as usize {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = MEM_readLE16(ip as *const c_void) as c_int + LONGNBSEQ;
                ip = ip.add(2);
            } else {
                if ip as usize >= iend as usize {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = ((nbSeq - 0x80) << 8) + *ip as c_int;
                ip = ip.add(1);
            }
        }
        *nbSeqPtr = nbSeq;
    }

    /* FSE table descriptors */
    if (ip.add(4) as usize) > iend as usize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let LLtype: U32 = (*ip >> 6) as U32;
        let OFtype: U32 = ((*ip >> 4) & 3) as U32;
        let MLtype: U32 = ((*ip >> 2) & 3) as U32;
        ip = ip.add(1);

        /* Build DTables */
        {
            let llhSize = ZSTDv07_buildSeqTable(
                DTableLL,
                LLtype,
                MaxLL as U32,
                LLFSELog,
                ip as *const c_void,
                iend as usize - ip as usize,
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(llhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(llhSize);
        }
        {
            let ofhSize = ZSTDv07_buildSeqTable(
                DTableOffb,
                OFtype,
                MaxOff as U32,
                OffFSELog,
                ip as *const c_void,
                iend as usize - ip as usize,
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(ofhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(ofhSize);
        }
        {
            let mlhSize = ZSTDv07_buildSeqTable(
                DTableML,
                MLtype,
                MaxML as U32,
                MLFSELog,
                ip as *const c_void,
                iend as usize - ip as usize,
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv07_isError(mlhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(mlhSize);
        }
    }

    ip as usize - istart as usize
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
    pub DStream: BITv07_DStream_t,
    pub stateLL: FSEv07_DState_t,
    pub stateOffb: FSEv07_DState_t,
    pub stateML: FSEv07_DState_t,
    pub prevOffset: [usize; ZSTDv07_REP_INIT],
}

static LL_base: [U32; MaxLL + 1] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

static ML_base: [U32; MaxML + 1] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
    0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003,
];

static OF_base: [U32; MaxOff + 1] = [
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

    let llCode: U32 = FSEv07_peekSymbol(&(*seqState).stateLL) as U32;
    let mlCode: U32 = FSEv07_peekSymbol(&(*seqState).stateML) as U32;
    let ofCode: U32 = FSEv07_peekSymbol(&(*seqState).stateOffb) as U32; /* <= maxOff, by table construction */

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
                .wrapping_add(BITv07_readBits(&mut (*seqState).DStream, ofBits)); /* <= (ZSTDv07_WINDOWLOG_MAX-1) bits */
            if MEM_32bits() != 0 {
                BITv07_reloadDStream(&mut (*seqState).DStream);
            }
        }

        if ofCode <= 1 {
            if (((llCode == 0) as U32) & ((offset <= 1) as U32)) != 0 {
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
            (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
            (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
            (*seqState).prevOffset[0] = offset;
        }
        seq.offset = offset;
    }

    seq.matchLength = (ML_base[mlCode as usize] as usize).wrapping_add(if mlCode > 31 {
        BITv07_readBits(&mut (*seqState).DStream, mlBits)
    } else {
        0
    }); /* <= 16 bits */
    if MEM_32bits() != 0 && (mlBits.wrapping_add(llBits) > 24) {
        BITv07_reloadDStream(&mut (*seqState).DStream);
    }

    seq.litLength = (LL_base[llCode as usize] as usize).wrapping_add(if llCode > 15 {
        BITv07_readBits(&mut (*seqState).DStream, llBits)
    } else {
        0
    }); /* <= 16 bits */
    if MEM_32bits() != 0
        || (totalBits > 64 - 7 - (LLFSELog + MLFSELog + OffFSELog))
    {
        BITv07_reloadDStream(&mut (*seqState).DStream);
    }

    /* ANS state update */
    FSEv07_updateState(&mut (*seqState).stateLL, &mut (*seqState).DStream); /* <= 9 bits */
    FSEv07_updateState(&mut (*seqState).stateML, &mut (*seqState).DStream); /* <= 9 bits */
    if MEM_32bits() != 0 {
        BITv07_reloadDStream(&mut (*seqState).DStream); /* <= 18 bits */
    }
    FSEv07_updateState(&mut (*seqState).stateOffb, &mut (*seqState).DStream); /* <= 8 bits */

    seq
}

static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */

unsafe fn ZSTDv07_execSequence(
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
    let oMatchEnd = (op as usize).wrapping_add(sequenceLength) as *mut BYTE; /* risk : address space overflow (32-bits) */
    let oend_w = (oend as usize).wrapping_sub(WILDCOPY_OVERLENGTH) as *mut BYTE;
    let iLitEnd = ((*litPtr) as usize).wrapping_add(sequence.litLength) as *const BYTE;
    let mut match_ = (oLitEnd as usize).wrapping_sub(sequence.offset) as *const BYTE;

    /* check */
    if sequence.litLength + WILDCOPY_OVERLENGTH > (oend as usize - op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequenceLength > (oend as usize - op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize - (*litPtr) as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* copy Literals */
    ZSTDv07_wildcopy(
        op as *mut c_void,
        (*litPtr) as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* copy Match */
    if sequence.offset > (oLitEnd as usize - base as usize) {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as usize - vBase as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = (dictEnd as usize)
            .wrapping_sub((base as usize).wrapping_sub(match_ as usize)) as *const BYTE;
        if (match_ as usize).wrapping_add(sequence.matchLength) <= dictEnd as usize {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = dictEnd as usize - match_ as usize;
            ZSTD_memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.add(length1);
            sequence.matchLength -= length1;
            match_ = base;
            if op as usize > oend_w as usize || sequence.matchLength < MINMATCH {
                while (op as usize) < oMatchEnd as usize {
                    *op = *match_;
                    op = op.add(1);
                    match_ = match_.add(1);
                }
                return sequenceLength;
            }
        }
    }
    /* Requirement: op <= oend_w */

    /* match within prefix */
    if sequence.offset < 8 {
        /* close range match, overlap */
        let sub2: c_int = dec64table[sequence.offset];
        *op.add(0) = *match_.add(0);
        *op.add(1) = *match_.add(1);
        *op.add(2) = *match_.add(2);
        *op.add(3) = *match_.add(3);
        match_ = match_.add(dec32table[sequence.offset] as usize);
        ZSTDv07_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
        match_ = (match_ as usize).wrapping_sub(sub2 as usize) as *const BYTE;
    } else {
        ZSTDv07_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.add(8);
    match_ = match_.add(8);

    if oMatchEnd as usize > (oend as usize).wrapping_sub(16 - MINMATCH) {
        if (op as usize) < oend_w as usize {
            ZSTDv07_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                (oend_w as isize) - (op as isize),
            );
            match_ = (match_ as usize)
                .wrapping_add(oend_w as usize - op as usize) as *const BYTE;
            op = oend_w;
        }
        while (op as usize) < oMatchEnd as usize {
            *op = *match_;
            op = op.add(1);
            match_ = match_.add(1);
        }
    } else {
        ZSTDv07_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            (sequence.matchLength as isize) - 8,
        ); /* works even if matchLength < 8 */
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
    let mut ip = seqStart as *const BYTE;
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = (ostart as usize).wrapping_add(maxDstSize) as *mut BYTE;
    let mut op = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd = (litPtr as usize).wrapping_add((*dctx).litSize) as *const BYTE;
    let DTableLL: *mut FSEv07_DTable = (*dctx).LLTable.as_mut_ptr();
    let DTableML: *mut FSEv07_DTable = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb: *mut FSEv07_DTable = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: c_int = 0;

    /* Build Decoding Tables */
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

    /* Regen sequences */
    if nbSeq != 0 {
        let mut seqState = seqState_t {
            DStream: BITv07_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            },
            stateLL: FSEv07_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateOffb: FSEv07_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateML: FSEv07_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            prevOffset: [0; ZSTDv07_REP_INIT],
        };
        (*dctx).fseEntropy = 1;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTDv07_REP_INIT {
                seqState.prevOffset[i as usize] = (*dctx).rep[i as usize] as usize;
                i += 1;
            }
        }
        {
            let errorCode = BITv07_initDStream(
                &mut seqState.DStream,
                ip as *const c_void,
                iend as usize - ip as usize,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }
        FSEv07_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSEv07_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSEv07_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BITv07_reloadDStream(&mut seqState.DStream) <= BITv07_DStream_completed)
            && nbSeq != 0
        {
            nbSeq -= 1;
            {
                let sequence = ZSTDv07_decodeSequence(&mut seqState);
                let oneSeqSize = ZSTDv07_execSequence(
                    op,
                    oend,
                    sequence,
                    &mut litPtr,
                    litEnd,
                    base,
                    vBase,
                    dictEnd,
                );
                if ZSTDv07_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.add(oneSeqSize);
            }
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* save reps for next block */
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTDv07_REP_INIT {
                (*dctx).rep[i as usize] = seqState.prevOffset[i as usize] as U32;
                i += 1;
            }
        }
    }

    /* last literal segment */
    {
        let lastLLSize: usize = litEnd as usize - litPtr as usize;
        if lastLLSize > (oend as usize - op as usize) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if lastLLSize > 0 {
            ZSTD_memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    op as usize - ostart as usize
}

unsafe fn ZSTDv07_checkContinuity(dctx: *mut ZSTDv07_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = ((dst as *const c_char) as usize).wrapping_sub(
            ((*dctx).previousDstEnd as *const c_char as usize)
                .wrapping_sub((*dctx).base as *const c_char as usize),
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
    /* blockType == blockCompressed */
    let mut ip = src as *const BYTE;

    if srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals sub-block */
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
    ZSTDv07_checkContinuity(dctx, dst as *const c_void);
    dSize = ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
    (*dctx).previousDstEnd = (dst as *mut c_char).add(dSize) as *const c_void;
    dSize
}

/* ZSTDv07_insertBlock() :
 * insert `src` block into `dctx` history. Useful to track uncompressed blocks. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_insertBlock(
    dctx: *mut ZSTDv07_DCtx,
    blockStart: *const c_void,
    blockSize: usize,
) -> usize {
    ZSTDv07_checkContinuity(dctx, blockStart);
    (*dctx).previousDstEnd = (blockStart as *const c_char).add(blockSize) as *const c_void;
    blockSize
}

unsafe fn ZSTDv07_generateNxBytes(
    dst: *mut c_void,
    dstCapacity: usize,
    byte: BYTE,
    length: usize,
) -> usize {
    if length > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if length > 0 {
        ZSTD_memset(dst, byte as c_int, length);
    }
    length
}

/* ZSTDv07_decompressFrame() :
 * `dctx` must be properly initialized */
unsafe fn ZSTDv07_decompressFrame(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut BYTE;
    let oend = (ostart as usize).wrapping_add(dstCapacity) as *mut BYTE;
    let mut op = ostart;
    let mut remainingSize: usize = srcSize;

    /* check */
    if srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Frame Header */
    {
        let frameHeaderSize = ZSTDv07_frameHeaderSize(src, ZSTDv07_frameHeaderSize_min);
        if ZSTDv07_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv07_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if ZSTDv07_decodeFrameHeader(dctx, src, frameHeaderSize) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    /* Loop on each block */
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
            return ERROR(ZSTD_error_srcSize_wrong);
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
                /* end of frame */
                if remainingSize != 0 {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                decodedSize = 0;
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
        if blockProperties.blockType == bt_end {
            break;
        }

        if ZSTDv07_isError(decodedSize) != 0 {
            return decodedSize;
        }
        if (*dctx).fParams.checksumFlag != 0 {
            ZSTD_XXH64_update(&mut (*dctx).xxhState, op as *const c_void, decodedSize);
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op as usize - ostart as usize
}

/* ZSTDv07_decompress_usingPreparedDCtx() */
unsafe fn ZSTDv07_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv07_DCtx,
    refDCtx: *const ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv07_copyDCtx(dctx, refDCtx);
    ZSTDv07_checkContinuity(dctx, dst as *const c_void);
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
    ZSTDv07_checkContinuity(dctx, dst as *const c_void);
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
    ZSTDv07_decompress_usingDict(
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
pub unsafe extern "C" fn ZSTDv07_decompress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* ZSTDv07_HEAPMODE == 1 */
    let regenSize: usize;
    let dctx = ZSTDv07_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv07_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv07_freeDCtx(dctx);
    regenSize
}

/* ZSTD_errorFrameSizeInfoLegacy() :
 * assumes `cSize` and `dBound` are _not_ NULL */
unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut u64, ret: usize) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;

    /* check */
    if srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }

    /* Frame Header */
    {
        let frameHeaderSize = ZSTDv07_frameHeaderSize(src, srcSize);
        if ZSTDv07_isError(frameHeaderSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, frameHeaderSize);
            return;
        }
        if MEM_readLE32(src) != ZSTDv07_MAGICNUMBER {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
            return;
        }
        if srcSize < frameHeaderSize + ZSTDv07_blockHeaderSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }
        ip = ip.add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    /* Loop on each block */
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
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = ip as usize - src as usize;
    *dBound = (nbBlocks * ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) as u64;
}

/*_******************************
*  Streaming Decompression API
********************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_nextSrcSizeToDecompress(dctx: *mut ZSTDv07_DCtx) -> usize {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isSkipFrame(dctx: *mut ZSTDv07_DCtx) -> c_int {
    ((*dctx).stage == ZSTDds_skipFrame) as c_int
}

/* ZSTDv07_decompressContinue() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressContinue(
    dctx: *mut ZSTDv07_DCtx,
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
        ZSTDv07_checkContinuity(dctx, dst as *const c_void);
    }

    match (*dctx).stage {
        ZSTDds_getFrameHeaderSize => {
            if srcSize != ZSTDv07_frameHeaderSize_min {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if (MEM_readLE32(src) & 0xFFFFFFF0u32) == ZSTDv07_MAGIC_SKIPPABLE_START {
                ZSTD_memcpy(
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
            ZSTD_memcpy(
                (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTDv07_frameHeaderSize_min,
            );
            if (*dctx).headerSize > ZSTDv07_frameHeaderSize_min {
                (*dctx).expected = (*dctx).headerSize - ZSTDv07_frameHeaderSize_min;
                (*dctx).stage = ZSTDds_decodeFrameHeader;
                return 0;
            }
            (*dctx).expected = 0; /* not necessary to copy more */
            /* fall-through into ZSTDds_decodeFrameHeader */
            {
                let result: usize;
                ZSTD_memcpy(
                    (*dctx)
                        .headerBuffer
                        .as_mut_ptr()
                        .add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
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
            ZSTD_memcpy(
                (*dctx)
                    .headerBuffer
                    .as_mut_ptr()
                    .add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
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
                    let h64: U64 = ZSTD_XXH64_digest(&(*dctx).xxhState);
                    let h32: U32 = ((h64 >> 11) as U32) & ((1u32 << 22) - 1);
                    let ip = src as *const BYTE;
                    let check32: U32 = (*ip.add(2) as U32)
                        .wrapping_add((*ip.add(1) as U32) << 8)
                        .wrapping_add(((*ip.add(0) & 0x3F) as U32) << 16);
                    if check32 != h32 {
                        return ERROR(ZSTD_error_checksum_wrong);
                    }
                }
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
                        ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
                }
                bt_raw => {
                    rSize = ZSTDv07_copyRawBlock(dst, dstCapacity, src, srcSize);
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
            (*dctx).expected = ZSTDv07_blockHeaderSize;
            if ZSTDv07_isError(rSize) != 0 {
                return rSize;
            }
            (*dctx).previousDstEnd = (dst as *mut c_char).add(rSize) as *const c_void;
            if (*dctx).fParams.checksumFlag != 0 {
                ZSTD_XXH64_update(&mut (*dctx).xxhState, dst as *const c_void, rSize);
            }
            return rSize;
        }
        ZSTDds_decodeSkippableHeader => {
            ZSTD_memcpy(
                (*dctx)
                    .headerBuffer
                    .as_mut_ptr()
                    .add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
                src,
                (*dctx).expected,
            );
            (*dctx).expected =
                MEM_readLE32((*dctx).headerBuffer.as_ptr().add(4) as *const c_void) as usize;
            (*dctx).stage = ZSTDds_skipFrame;
            return 0;
        }
        ZSTDds_skipFrame => {
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            return 0;
        }
        _ => {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
    }
}

unsafe fn ZSTDv07_refDictContent(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = ((dict as *const c_char) as usize).wrapping_sub(
        ((*dctx).previousDstEnd as *const c_char as usize)
            .wrapping_sub((*dctx).base as *const c_char as usize),
    ) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const c_char).add(dictSize) as *const c_void;
    0
}

unsafe fn ZSTDv07_loadEntropy(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut dictPtr = dict as *const BYTE;
    let dictEnd = dictPtr.add(dictSize);

    {
        let hSize = HUFv07_readDTableX4((*dctx).hufTable.as_mut_ptr(), dict, dictSize);
        if HUFv07_isError(hSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dictPtr = dictPtr.add(hSize);
    }

    {
        let mut offcodeNCount: [i16; MaxOff + 1] = [0; MaxOff + 1];
        let mut offcodeMaxValue: c_uint = MaxOff as c_uint;
        let mut offcodeLog: c_uint = 0;
        let offcodeHeaderSize = FSEv07_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            dictEnd as usize - dictPtr as usize,
        );
        if FSEv07_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv07_buildDTable(
                (*dctx).OffTable.as_mut_ptr(),
                offcodeNCount.as_ptr(),
                offcodeMaxValue,
                offcodeLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dictPtr = dictPtr.add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount: [i16; MaxML + 1] = [0; MaxML + 1];
        let mut matchlengthMaxValue: c_uint = MaxML as c_uint;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize = FSEv07_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            dictEnd as usize - dictPtr as usize,
        );
        if FSEv07_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv07_buildDTable(
                (*dctx).MLTable.as_mut_ptr(),
                matchlengthNCount.as_ptr(),
                matchlengthMaxValue,
                matchlengthLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dictPtr = dictPtr.add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount: [i16; MaxLL + 1] = [0; MaxLL + 1];
        let mut litlengthMaxValue: c_uint = MaxLL as c_uint;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize = FSEv07_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            dictEnd as usize - dictPtr as usize,
        );
        if FSEv07_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv07_buildDTable(
                (*dctx).LLTable.as_mut_ptr(),
                litlengthNCount.as_ptr(),
                litlengthMaxValue,
                litlengthLog,
            );
            if FSEv07_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dictPtr = dictPtr.add(litlengthHeaderSize);
    }

    if (dictPtr.add(12) as usize) > dictEnd as usize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[0] = MEM_readLE32(dictPtr.add(0) as *const c_void);
    if (*dctx).rep[0] == 0 || (*dctx).rep[0] as usize >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[1] = MEM_readLE32(dictPtr.add(4) as *const c_void);
    if (*dctx).rep[1] == 0 || (*dctx).rep[1] as usize >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[2] = MEM_readLE32(dictPtr.add(8) as *const c_void);
    if (*dctx).rep[2] == 0 || (*dctx).rep[2] as usize >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dictPtr = dictPtr.add(12);

    (*dctx).fseEntropy = 1;
    (*dctx).litEntropy = (*dctx).fseEntropy;
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
        let magic: U32 = MEM_readLE32(dict);
        if magic != ZSTDv07_DICT_MAGIC {
            return ZSTDv07_refDictContent(dctx, dict, dictSize); /* pure content mode */
        }
    }
    (*dctx).dictID = MEM_readLE32((dict as *const c_char).add(4) as *const c_void);

    /* load entropy tables */
    dict = (dict as *const c_char).add(8) as *const c_void;
    dictSize -= 8;
    {
        let eSize = ZSTDv07_loadEntropy(dctx, dict, dictSize);
        if ZSTDv07_isError(eSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dict = (dict as *const c_char).add(eSize) as *const c_void;
        dictSize -= eSize;
    }

    /* reference dictionary content */
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
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }

    0
}

#[repr(C)]
pub struct ZSTDv07_DDict {
    pub dict: *mut c_void,
    pub dictSize: usize,
    pub refContext: *mut ZSTDv07_DCtx,
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

        ZSTD_memcpy(dictContent, dict, dictSize);
        {
            let errorCode =
                ZSTDv07_decompressBegin_usingDict(dctx, dictContent as *const c_void, dictSize);
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

/* ZSTDv07_createDDict() */
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
    let cFree: ZSTDv07_freeFunction = (*(*ddict).refContext).customMem.customFree;
    let opaque: *mut c_void = (*(*ddict).refContext).customMem.opaque;
    ZSTDv07_freeDCtx((*ddict).refContext);
    (cFree.unwrap())(opaque, (*ddict).dict);
    (cFree.unwrap())(opaque, ddict as *mut c_void);
    0
}

/* ZSTDv07_decompress_usingDDict() */
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

/* ===================================================================
 * zbuff_decompress.c
 * =================================================================== */

pub type ZBUFFv07_dStage = c_uint;
pub const ZBUFFds_init: ZBUFFv07_dStage = 0;
pub const ZBUFFds_loadHeader: ZBUFFv07_dStage = 1;
pub const ZBUFFds_read: ZBUFFv07_dStage = 2;
pub const ZBUFFds_load: ZBUFFv07_dStage = 3;
pub const ZBUFFds_flush: ZBUFFv07_dStage = 4;

/* *** Resource management *** */
#[repr(C)]
pub struct ZBUFFv07_DCtx {
    pub zd: *mut ZSTDv07_DCtx,
    pub fParams: ZSTDv07_frameParams,
    pub stage: ZBUFFv07_dStage,
    pub inBuff: *mut c_char,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub outBuff: *mut c_char,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub blockSize: usize,
    pub headerBuffer: [BYTE; ZSTDv07_FRAMEHEADERSIZE_MAX],
    pub lhSize: usize,
    pub customMem: ZSTDv07_customMem,
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
    ZSTD_memset(
        zbd as *mut c_void,
        0,
        core::mem::size_of::<ZBUFFv07_DCtx>(),
    );
    ZSTD_memcpy(
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
        return 0; /* support free on null */
    }
    ZSTDv07_freeDCtx((*zbd).zd);
    if !(*zbd).inBuff.is_null() {
        ((*zbd).customMem.customFree.unwrap())(
            (*zbd).customMem.opaque,
            (*zbd).inBuff as *mut c_void,
        );
    }
    if !(*zbd).outBuff.is_null() {
        ((*zbd).customMem.customFree.unwrap())(
            (*zbd).customMem.opaque,
            (*zbd).outBuff as *mut c_void,
        );
    }
    ((*zbd).customMem.customFree.unwrap())((*zbd).customMem.opaque, zbd as *mut c_void);
    0
}

/* *** Initialization *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInitDictionary(
    zbd: *mut ZBUFFv07_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*zbd).stage = ZBUFFds_loadHeader;
    (*zbd).outEnd = 0;
    (*zbd).outStart = (*zbd).outEnd;
    (*zbd).inPos = (*zbd).outStart;
    (*zbd).lhSize = (*zbd).inPos;
    ZSTDv07_decompressBegin_usingDict((*zbd).zd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInit(zbd: *mut ZBUFFv07_DCtx) -> usize {
    ZBUFFv07_decompressInitDictionary(zbd, core::ptr::null(), 0)
}

/* internal util function */
#[inline(always)]
unsafe fn ZBUFFv07_limitCopy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length: usize = if dstCapacity < srcSize {
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
pub unsafe extern "C" fn ZBUFFv07_decompressContinue(
    zbd: *mut ZBUFFv07_DCtx,
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
        let mut cur: ZBUFFv07_dStage = (*zbd).stage;
        'dispatch: loop {
            match cur {
                ZBUFFds_init => {
                    return ERROR(ZSTD_error_init_missing);
                }

                ZBUFFds_loadHeader => {
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
                            let toLoad: usize = hSize - (*zbd).lhSize; /* if hSize!=0, hSize > zbd->lhSize */
                            if toLoad > (iend as usize - ip as usize) {
                                /* not enough input to load full header */
                                if !ip.is_null() {
                                    ZSTD_memcpy(
                                        (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize)
                                            as *mut c_void,
                                        ip as *const c_void,
                                        iend as usize - ip as usize,
                                    );
                                }
                                (*zbd).lhSize += iend as usize - ip as usize;
                                *dstCapacityPtr = 0;
                                return (hSize - (*zbd).lhSize) + ZSTDv07_blockHeaderSize;
                            }
                            ZSTD_memcpy(
                                (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize)
                                    as *mut c_void,
                                ip as *const c_void,
                                toLoad,
                            );
                            (*zbd).lhSize = hSize;
                            ip = ip.add(toLoad);
                            break 'dispatch;
                        }
                    }

                    /* Consume header */
                    {
                        let h1Size = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd); /* == ZSTDv07_frameHeaderSize_min */
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
                            /* long header */
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

                    (*zbd).fParams.windowSize = if (*zbd).fParams.windowSize
                        > (1u32 << ZSTDv07_WINDOWLOG_ABSOLUTEMIN)
                    {
                        (*zbd).fParams.windowSize
                    } else {
                        1u32 << ZSTDv07_WINDOWLOG_ABSOLUTEMIN
                    };

                    /* Frame header instruct buffer sizes */
                    {
                        let blockSize: usize =
                            (if (*zbd).fParams.windowSize < ZSTDv07_BLOCKSIZE_ABSOLUTEMAX as U32 {
                                (*zbd).fParams.windowSize
                            } else {
                                ZSTDv07_BLOCKSIZE_ABSOLUTEMAX as U32
                            }) as usize;
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
                                return ERROR(ZSTD_error_memory_allocation);
                            }
                        }
                        {
                            let neededOutSize: usize = (*zbd).fParams.windowSize as usize
                                + blockSize
                                + WILDCOPY_OVERLENGTH * 2;
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
                                    return ERROR(ZSTD_error_memory_allocation);
                                }
                            }
                        }
                    }
                    (*zbd).stage = ZBUFFds_read;
                    /* fall-through */
                    cur = ZBUFFds_read;
                    continue 'dispatch;
                }

                ZBUFFds_read => {
                    {
                        let neededInSize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                        if neededInSize == 0 {
                            /* end of frame */
                            (*zbd).stage = ZBUFFds_init;
                            notDone = 0;
                            break 'dispatch;
                        }
                        if (iend as usize - ip as usize) >= neededInSize {
                            /* decode directly from src */
                            let isSkipFrame: c_int = ZSTDv07_isSkipFrame((*zbd).zd);
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
                                break 'dispatch; /* this was just a header */
                            }
                            (*zbd).outEnd = (*zbd).outStart + decodedSize;
                            (*zbd).stage = ZBUFFds_flush;
                            break 'dispatch;
                        }
                        if ip == iend {
                            notDone = 0;
                            break 'dispatch;
                        } /* no more input */
                        (*zbd).stage = ZBUFFds_load;
                    }
                    /* fall-through */
                    cur = ZBUFFds_load;
                    continue 'dispatch;
                }

                ZBUFFds_load => {
                    {
                        let neededInSize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                        let toLoad: usize = neededInSize - (*zbd).inPos; /* should always be <= remaining space within inBuff */
                        let loadedSize: usize;
                        if toLoad > (*zbd).inBuffSize - (*zbd).inPos {
                            return ERROR(ZSTD_error_corruption_detected);
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
                            break 'dispatch;
                        } /* not enough input, wait for more */

                        /* decode loaded input */
                        {
                            let isSkipFrame: c_int = ZSTDv07_isSkipFrame((*zbd).zd);
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
                            (*zbd).inPos = 0; /* input is consumed */
                            if decodedSize == 0 && isSkipFrame == 0 {
                                (*zbd).stage = ZBUFFds_read;
                                break 'dispatch;
                            } /* this was just a header */
                            (*zbd).outEnd = (*zbd).outStart + decodedSize;
                            (*zbd).stage = ZBUFFds_flush;
                            /* pass-through */
                        }
                    }
                    /* fall-through */
                    cur = ZBUFFds_flush;
                    continue 'dispatch;
                }

                ZBUFFds_flush => {
                    {
                        let toFlushSize: usize = (*zbd).outEnd - (*zbd).outStart;
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
                                (*zbd).outEnd = 0;
                                (*zbd).outStart = (*zbd).outEnd;
                            }
                            break 'dispatch;
                        }
                        /* cannot flush everything */
                        notDone = 0;
                        break 'dispatch;
                    }
                }
                _ => {
                    return ERROR(ZSTD_error_GENERIC); /* impossible */
                }
            }
        }
    }

    /* result */
    *srcSizePtr = ip as usize - istart as usize;
    *dstCapacityPtr = op as usize - ostart as usize;
    {
        let mut nextSrcSizeHint = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbd).inPos); /* already loaded*/
        nextSrcSizeHint
    }
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_recommendedDInSize() -> usize {
    ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + ZSTDv07_blockHeaderSize /* block header size*/
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_recommendedDOutSize() -> usize {
    ZSTDv07_BLOCKSIZE_ABSOLUTEMAX
}
