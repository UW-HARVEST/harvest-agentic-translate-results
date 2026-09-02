//! Literal Rust transliteration of `c_src/src/legacy/zstd_v03.c`.
//!
//! This file is a self-contained translation unit: it bundles its own
//! mem/endian helpers, error enum, bitstream reader, FSE decoder, Huff0
//! decoder, block/frame decoders, DCtx and streaming state machine.
//! Everything is translated literally; only the 8 `ZSTDv03_*` wrapper
//! functions are exported (`#[unsafe(no_mangle)]`).

use core::ffi::c_void;

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
// IMPORTANT: zstd_v03.c #includes "../common/error_private.h" *before* its own
// inline error enum, which defines ERROR_H_MODULE. That guards out the file's
// own tiny enum, so at compile time ERROR()/ERR_isError()/PREFIX() all resolve
// to the MODERN common definitions using the ZSTD_error_* values from
// zstd_errors.h. Hence we must use those numeric values (verified by comparing
// return codes against the reference libzstd.so).
// ============================================================================
pub const ZSTD_error_No_Error: U32 = 0;
pub const ZSTD_error_GENERIC: U32 = 1;
pub const ZSTD_error_prefix_unknown: U32 = 10;
pub const ZSTD_error_corruption_detected: U32 = 20;
pub const ZSTD_error_tableLog_tooLarge: U32 = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: U32 = 46;
pub const ZSTD_error_maxSymbolValue_tooSmall: U32 = 48;
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

// ============================================================================
// Bitstream decompression API (read backward)
// ============================================================================
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BIT_DStream_t {
    pub bitContainer: size_t,
    pub bitsConsumed: u32,
    pub ptr: *const i8,
    pub start: *const i8,
}

pub type BIT_DStream_status = u32;
pub const BIT_DStream_unfinished: BIT_DStream_status = 0;
pub const BIT_DStream_endOfBuffer: BIT_DStream_status = 1;
pub const BIT_DStream_completed: BIT_DStream_status = 2;
pub const BIT_DStream_overflow: BIT_DStream_status = 3;

#[inline]
pub unsafe fn BIT_highbit32(val: U32) -> u32 {
    // __builtin_clz(val) ^ 31
    (val.leading_zeros()) ^ 31
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

    let sz = core::mem::size_of::<size_t>();

    if srcSize >= sz {
        // normal case
        let contain32: U32;
        (*bitD).start = srcBuffer as *const i8;
        (*bitD).ptr = (srcBuffer as *const i8).wrapping_add(srcSize).wrapping_sub(sz);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const i8;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as size_t;
        let base = (*bitD).start as *const BYTE;
        // switch with fallthrough
        let shift_bits = (sz * 8) as u32;
        if srcSize == 7 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*base.add(6) as size_t) << (shift_bits - 16));
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*base.add(5) as size_t) << (shift_bits - 24));
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*base.add(4) as size_t) << (shift_bits - 32));
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*base.add(3) as size_t) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*base.add(2) as size_t) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD).bitContainer
                .wrapping_add((*base.add(1) as size_t) << 8);
        }
        contain32 = *((srcBuffer as *const BYTE).add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
        (*bitD).bitsConsumed = 8u32.wrapping_sub(BIT_highbit32(contain32));
        (*bitD).bitsConsumed = (*bitD).bitsConsumed
            .wrapping_add(((sz - srcSize) as u32).wrapping_mul(8));
    }

    srcSize
}

#[inline]
pub unsafe fn BIT_lookBits(bitD: *const BIT_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() as U32) * 8 - 1;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> (((bitMask - nbBits) & bitMask) as size_t)
}

#[inline]
pub unsafe fn BIT_lookBitsFast(bitD: *const BIT_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() as U32) * 8 - 1;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> ((((bitMask + 1) - nbBits) & bitMask) as size_t)
}

#[inline]
pub unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
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
    let containerBits = (core::mem::size_of::<size_t>() * 8) as u32;

    if (*bitD).bitsConsumed > containerBits {
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(core::mem::size_of::<size_t>()) {
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < containerBits {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BIT_DStream_status = BIT_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32; // ptr > start
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline]
pub unsafe fn BIT_endOfDStream(DStream: *const BIT_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<size_t>() * 8) as u32)) as u32
}

// ============================================================================
// FSE_CTable / FSE_DTable primitive types
// ============================================================================
pub type FSE_CTable = u32;
pub type FSE_DTable = u32;

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

pub unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
    dt: *const FSE_DTable,
) {
    let mut DTableH: FSE_DTableHeader = core::mem::zeroed();
    memcpy(
        &mut DTableH as *mut _ as *mut c_void,
        dt as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );
    (*DStatePtr).state = BIT_readBits(bitD, DTableH.tableLog as U32);
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const c_void;
}

pub unsafe fn FSE_decodeSymbol(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSE_decodeSymbolFast(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

pub unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

// ============================================================================
// FSE constants
// ============================================================================
pub const FSE_MAX_MEMORY_USAGE: u32 = 14;
pub const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSE_MAX_SYMBOL_VALUE: u32 = 255;

pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2; // 12
pub const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
pub const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
pub const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
pub const FSE_MIN_TABLELOG: u32 = 5;
pub const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

// FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG) = 1 + (1<<12) = 4097
pub const DTABLE_MAX_SIZE_U32: usize = (1 + (1usize << FSE_MAX_TABLELOG)) as usize;

pub const FSE_DECODE_TYPE_IS_BYTE: bool = true; // FSE_FUNCTION_TYPE == BYTE

#[inline]
pub unsafe fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1).wrapping_add(tableSize >> 3).wrapping_add(3)
}

pub unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> size_t {
    let ptr = dt.add(1) as *mut c_void;
    let mut DTableH: FSE_DTableHeader = core::mem::zeroed();
    let tableDecode = ptr as *mut FSE_decode_t;
    let tableSize: U32 = 1u32 << tableLog;
    let tableMask: U32 = tableSize - 1;
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize - 1;
    let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

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
        s += 1;
    }

    // Spread symbols
    s = 0;
    while s <= maxSymbolValue {
        let mut i: i32 = 0;
        while i < *normalizedCounter.add(s as usize) as i32 {
            (*tableDecode.add(position as usize)).symbol = s as BYTE;
            position = (position.wrapping_add(step)) & tableMask;
            while position > highThreshold {
                position = (position.wrapping_add(step)) & tableMask;
            }
            i += 1;
        }
        s += 1;
    }

    if position != 0 {
        return ERROR(ZSTD_error_GENERIC);
    }

    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.add(i as usize)).nbBits =
                (tableLog.wrapping_sub(BIT_highbit32(nextState as U32))) as BYTE;
            (*tableDecode.add(i as usize)).newState =
                (((nextState as U32) << (*tableDecode.add(i as usize)).nbBits)
                    .wrapping_sub(tableSize)) as U16;
            i += 1;
        }
    }

    DTableH.fastMode = noLarge as U16;
    memcpy(
        dt as *mut c_void,
        &DTableH as *const _ as *const c_void,
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
    let iend = istart.wrapping_add(hbSize);
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
    nbBits = ((bitStream & 0xF) + FSE_MIN_TABLELOG) as i32;
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
            let mut n0: u32 = charnum;
            while (bitStream & 0xFFFF) == 0xFFFF {
                n0 = n0.wrapping_add(24);
                if ip < iend.wrapping_sub(5) {
                    ip = ip.wrapping_add(2);
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
                charnum += 1;
            }
            if (ip <= iend.wrapping_sub(7))
                || (ip.wrapping_add((bitCount >> 3) as usize) <= iend.wrapping_sub(4))
            {
                ip = ip.wrapping_add((bitCount >> 3) as usize);
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
                count = (bitStream & ((2 * threshold - 1) as U32)) as S16;
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
                if (ip <= iend.wrapping_sub(7))
                    || (ip.wrapping_add((bitCount >> 3) as usize) <= iend.wrapping_sub(4))
                {
                    ip = ip.wrapping_add((bitCount >> 3) as usize);
                    bitCount &= 7;
                } else {
                    bitCount -= (8 * (iend.offset_from(ip) - 4)) as i32;
                    ip = iend.wrapping_sub(4);
                }
                bitStream = MEM_readLE32(ip as *const c_void) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return ERROR(ZSTD_error_GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.wrapping_add(((bitCount + 7) >> 3) as usize);
    if (ip.offset_from(istart) as size_t) > hbSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip.offset_from(istart) as size_t
}

// ----------------------------------------------------------------------------
// Decompression (Byte symbols)
// ----------------------------------------------------------------------------
pub unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> size_t {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let cell = (ptr as *mut FSE_decode_t).add(1);

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

pub unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: u32) -> size_t {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let dinfo = (ptr as *mut FSE_decode_t).add(1);
    let tableSize: u32 = 1u32 << nbBits;
    let tableMask: u32 = tableSize - 1;
    let maxSymbolValue: u32 = tableMask;
    let mut s: u32;

    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC);
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
    let omax = op.wrapping_add(maxDstSize);
    let olimit = omax.wrapping_sub(3);

    let mut bitD: BIT_DStream_t = core::mem::zeroed();
    let mut state1: FSE_DState_t = core::mem::zeroed();
    let mut state2: FSE_DState_t = core::mem::zeroed();
    let errorCode: size_t;

    errorCode = BIT_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    macro_rules! GETSYMBOL {
        ($sp:expr) => {
            if fast != 0 {
                FSE_decodeSymbolFast($sp, &mut bitD)
            } else {
                FSE_decodeSymbol($sp, &mut bitD)
            }
        };
    }

    // static test: FSE_MAX_TABLELOG*2+7 > sizeof(size_t)*8 ; on 64-bit = 31 > 64 false
    let containerBits = (core::mem::size_of::<size_t>() * 8) as u32;
    let test2 = FSE_MAX_TABLELOG * 2 + 7 > containerBits;
    let test4 = FSE_MAX_TABLELOG * 4 + 7 > containerBits;

    // 4 symbols per loop
    while (BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit) {
        *op.add(0) = GETSYMBOL!(&mut state1);
        if test2 {
            BIT_reloadDStream(&mut bitD);
        }
        *op.add(1) = GETSYMBOL!(&mut state2);
        if test4 {
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }
        *op.add(2) = GETSYMBOL!(&mut state1);
        if test2 {
            BIT_reloadDStream(&mut bitD);
        }
        *op.add(3) = GETSYMBOL!(&mut state2);
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
        *op = GETSYMBOL!(&mut state1);
        op = op.add(1);

        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0
                && (fast != 0 || FSE_endOfDState(&state2) != 0))
        {
            break;
        }
        *op = GETSYMBOL!(&mut state2);
        op = op.add(1);
    }

    if BIT_endOfDStream(&bitD) != 0
        && FSE_endOfDState(&state1) != 0
        && FSE_endOfDState(&state2) != 0
    {
        return op.offset_from(ostart) as size_t;
    }

    if op == omax {
        return ERROR(ZSTD_error_dstSize_tooSmall);
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
    memcpy(
        &mut DTableH as *mut _ as *mut c_void,
        dt as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );

    if DTableH.fastMode != 0 {
        return FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

pub unsafe fn FSE_decompress(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [S16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [U32; DTABLE_MAX_SIZE_U32] = [0; DTABLE_MAX_SIZE_U32];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE;
    let mut errorCode: size_t;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong);
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
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSE_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

// ============================================================================
// Huff0 : Huffman block decompression
// ============================================================================
#[inline]
pub unsafe fn HUF_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

pub const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16;
pub const HUF_MAX_TABLELOG: u32 = 12;
pub const HUF_DEFAULT_TABLELOG: u32 = HUF_MAX_TABLELOG;
pub const HUF_MAX_SYMBOL_VALUE: u32 = 255;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
}

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
        ((HUF_ABSOLUTEMAX_TABLELOG + 1) as usize) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as size_t) < oSize {
        if (*huffWeight.add(n as usize) as U32) >= HUF_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *rankStats.add(*huffWeight.add(n as usize) as usize) =
            (*rankStats.add(*huffWeight.add(n as usize) as usize)).wrapping_add(1);
        weightTotal =
            weightTotal.wrapping_add((1u32 << *huffWeight.add(n as usize)) >> 1);
        n += 1;
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    // get last non-null symbol weight
    tableLog = BIT_highbit32(weightTotal) + 1;
    if tableLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_corruption_detected);
    }
    {
        let total: U32 = 1u32 << tableLog;
        let rest: U32 = total - weightTotal;
        let verif: U32 = 1u32 << BIT_highbit32(rest);
        let lastWeight: U32 = BIT_highbit32(rest) + 1;
        if verif != rest {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *huffWeight.add(oSize) = lastWeight as BYTE;
        *rankStats.add(lastWeight as usize) =
            (*rankStats.add(lastWeight as usize)).wrapping_add(1);
    }

    // check tree construction validity
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    *nbSymbolsPtr = (oSize + 1) as U32;
    *tableLogPtr = tableLog;
    (iSize + 1) as size_t
}

// --------------------------------------------------------------------------
// single-symbol decoding
// --------------------------------------------------------------------------
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
    let ptr = DTable.add(1) as *mut c_void;
    let dt = ptr as *mut HUF_DEltX2;

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

    if tableLog > *DTable.add(0) as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    *DTable.add(0) = tableLog as U16;

    // Prepare ranks
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current: U32 = nextRankStart;
        nextRankStart = nextRankStart.wrapping_add(rankVal[n as usize] << (n - 1));
        rankVal[n as usize] = current;
        n += 1;
    }

    // fill DTable
    n = 0;
    while n < nbSymbols {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32 << w) >> 1;
        let mut i: U32;
        let mut D: HUF_DEltX2 = core::mem::zeroed();
        D.byte = n as BYTE;
        D.nbBits = (tableLog + 1 - w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize] + length {
            *dt.add(i as usize) = D;
            i += 1;
        }
        rankVal[w as usize] = rankVal[w as usize].wrapping_add(length);
        n += 1;
    }

    iSize
}

pub unsafe fn HUF_decodeSymbolX2(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BIT_lookBitsFast(Dstream, dtLog);
    let c: BYTE = (*dt.add(val)).byte;
    BIT_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

pub unsafe fn HUF_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart = p;

    // up to 4 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(4))
    {
        // X2_2: if MEM_64bits()
        if MEM_64bits() != 0 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.add(1);
        }
        // X2_1: if MEM_64bits() || HUF_MAX_TABLELOG<=12
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.add(1);
        }
        // X2_2
        if MEM_64bits() != 0 {
            *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.add(1);
        }
        // X2_0
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    // closer to the end
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd) {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    // no more data to retrieve from bitstream
    while p < pEnd {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
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
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DEltX2).add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
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
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize = (dstSize + 3) / 4;
        let opStart2 = ostart.wrapping_add(segmentSize);
        let opStart3 = opStart2.wrapping_add(segmentSize);
        let opStart4 = opStart3.wrapping_add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: U32;

        length4 = cSrcSize.wrapping_sub(length1.wrapping_add(length2).wrapping_add(length3).wrapping_add(6));
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

        macro_rules! X2_0 {
            ($op:expr, $bd:expr) => {{
                *$op = HUF_decodeSymbolX2($bd, dt, dtLog);
                $op = $op.add(1);
            }};
        }
        macro_rules! X2_1 {
            ($op:expr, $bd:expr) => {{
                if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                    *$op = HUF_decodeSymbolX2($bd, dt, dtLog);
                    $op = $op.add(1);
                }
            }};
        }
        macro_rules! X2_2 {
            ($op:expr, $bd:expr) => {{
                if MEM_64bits() != 0 {
                    *$op = HUF_decodeSymbolX2($bd, dt, dtLog);
                    $op = $op.add(1);
                }
            }};
        }

        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            X2_2!(op1, &mut bitD1);
            X2_2!(op2, &mut bitD2);
            X2_2!(op3, &mut bitD3);
            X2_2!(op4, &mut bitD4);
            X2_1!(op1, &mut bitD1);
            X2_1!(op2, &mut bitD2);
            X2_1!(op3, &mut bitD3);
            X2_1!(op4, &mut bitD4);
            X2_2!(op1, &mut bitD1);
            X2_2!(op2, &mut bitD2);
            X2_2!(op3, &mut bitD3);
            X2_2!(op4, &mut bitD4);
            X2_0!(op1, &mut bitD1);
            X2_0!(op2, &mut bitD2);
            X2_0!(op3, &mut bitD3);
            X2_0!(op4, &mut bitD4);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        HUF_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

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

pub unsafe fn HUF_decompress4X2(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    // HUF_CREATE_STATIC_DTABLEX2(DTable, HUF_MAX_TABLELOG) = u16[HUF_DTABLE_SIZE(12)] = {12}
    let mut DTable: [U16; (1 + (1u32 << HUF_MAX_TABLELOG)) as usize] =
        [0; (1 + (1u32 << HUF_MAX_TABLELOG)) as usize];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let errorCode: size_t;

    errorCode = HUF_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    HUF_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// --------------------------------------------------------------------------
// double-symbols decoding
// --------------------------------------------------------------------------
// rankVal_t = U32[HUF_ABSOLUTEMAX_TABLELOG][HUF_ABSOLUTEMAX_TABLELOG + 1]
//           = U32[16][17]
pub const RANKVAL_DIM0: usize = HUF_ABSOLUTEMAX_TABLELOG as usize; // 16
pub const RANKVAL_DIM1: usize = (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize; // 17

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

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    // fill skipped values
    if minWeight > 1 {
        let mut i: U32;
        let skipSize: U32 = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut _ as *mut c_void, baseSeq);
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
        let symbol: U32 = (*sortedSymbols.add(s as usize)).symbol as U32;
        let weight: U32 = (*sortedSymbols.add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let length: U32 = 1u32 << (sizeLog - nbBits);
        let start: U32 = rankVal[weight as usize];
        let mut i: U32 = start;
        let end: U32 = start + length;

        MEM_writeLE16(
            &mut DElt.sequence as *mut _ as *mut c_void,
            (baseSeq as U32).wrapping_add(symbol << 8) as U16,
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

        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
        s += 1;
    }
}

pub unsafe fn HUF_fillDTableX4(
    DTable: *mut HUF_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const [U32; RANKVAL_DIM1], // rankVal_t
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] =
        [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: i32 = nbBitsBaseline as i32 - targetLog as i32;
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32 << (targetLog - nbBits);

        if targetLog.wrapping_sub(nbBits) >= minBits {
            // enough room for a second symbol
            let sortedRank: U32;
            let mut minWeight: i32 = nbBits as i32 + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUF_fillDTableX4Level2(
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
            let mut DElt: HUF_DEltX4 = core::mem::zeroed();

            MEM_writeLE16(&mut DElt.sequence as *mut _ as *mut c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1;
            i = start;
            while i < end {
                *DTable.add(i as usize) = DElt;
                i += 1;
            }
        }
        rankVal[weight as usize] = rankVal[weight as usize].wrapping_add(length);
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
    let rankStart = rankStart0.as_mut_ptr().add(1);
    let mut rankVal: [[U32; RANKVAL_DIM1]; RANKVAL_DIM0] = [[0; RANKVAL_DIM1]; RANKVAL_DIM0];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.add(0);
    let ip = src as *const BYTE;
    let mut iSize: size_t = *ip.add(0) as size_t;
    let ptr = DTable as *mut c_void;
    let dt = (ptr as *mut HUF_DEltX4).add(1);

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

    if tableLog > memLog {
        return ERROR(ZSTD_error_tableLog_tooLarge);
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
            let current: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankStats[w as usize]);
            *rankStart.add(w as usize) = current;
            w += 1;
        }
        *rankStart.add(0) = nextRankStart;
        sizeOfSort = nextRankStart;
    }

    // sort symbols by weight
    {
        let mut s: U32;
        s = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = *rankStart.add(w as usize);
            *rankStart.add(w as usize) = (*rankStart.add(w as usize)).wrapping_add(1);
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        *rankStart.add(0) = 0;
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
            let current: U32 = nextRankVal;
            nextRankVal =
                nextRankVal.wrapping_add(rankStats[w as usize] << (w as i32 + rescale));
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

    HUF_fillDTableX4(
        dt,
        memLog,
        sortedSymbol.as_ptr(),
        sizeOfSort,
        rankStart0.as_ptr(),
        rankVal.as_ptr(),
        maxW,
        tableLog + 1,
    );

    iSize
}

pub unsafe fn HUF_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 2);
    BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

pub unsafe fn HUF_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        let containerBits = (core::mem::size_of::<size_t>() * 8) as u32;
        if (*DStream).bitsConsumed < containerBits {
            BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > containerBits {
                (*DStream).bitsConsumed = containerBits;
            }
        }
    }
    1
}

pub unsafe fn HUF_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> size_t {
    let pStart = p;

    // up to 8 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished)
        && (p < pEnd.wrapping_sub(7))
    {
        // X4_2
        if MEM_64bits() != 0 {
            p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        }
        // X4_1
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
            p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        }
        // X4_2
        if MEM_64bits() != 0 {
            p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        }
        // X4_0
        p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    // closer to the end
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(2))
    {
        p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    while p <= pEnd.wrapping_sub(2) {
        p = p.wrapping_add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    if p < pEnd {
        p = p.wrapping_add(HUF_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
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
        let oend = ostart.wrapping_add(dstSize);

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DEltX4).add(1);
        let dtLog: U32 = *DTable.add(0);
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
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize = (dstSize + 3) / 4;
        let opStart2 = ostart.wrapping_add(segmentSize);
        let opStart3 = opStart2.wrapping_add(segmentSize);
        let opStart4 = opStart3.wrapping_add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: U32;

        length4 = cSrcSize.wrapping_sub(length1.wrapping_add(length2).wrapping_add(length3).wrapping_add(6));
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

        macro_rules! X4_0 {
            ($op:expr, $bd:expr) => {{
                $op = $op.wrapping_add(
                    HUF_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize,
                );
            }};
        }
        macro_rules! X4_1 {
            ($op:expr, $bd:expr) => {{
                if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 {
                    $op = $op.wrapping_add(
                        HUF_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize,
                    );
                }
            }};
        }
        macro_rules! X4_2 {
            ($op:expr, $bd:expr) => {{
                if MEM_64bits() != 0 {
                    $op = $op.wrapping_add(
                        HUF_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize,
                    );
                }
            }};
        }

        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            X4_2!(op1, &mut bitD1);
            X4_2!(op2, &mut bitD2);
            X4_2!(op3, &mut bitD3);
            X4_2!(op4, &mut bitD4);
            X4_1!(op1, &mut bitD1);
            X4_1!(op2, &mut bitD2);
            X4_1!(op3, &mut bitD3);
            X4_1!(op4, &mut bitD4);
            X4_2!(op1, &mut bitD1);
            X4_2!(op2, &mut bitD2);
            X4_2!(op3, &mut bitD3);
            X4_2!(op4, &mut bitD4);
            X4_0!(op1, &mut bitD1);
            X4_0!(op2, &mut bitD2);
            X4_0!(op3, &mut bitD3);
            X4_0!(op4, &mut bitD4);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        HUF_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

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
    mut cSrcSize: size_t,
) -> size_t {
    // HUF_CREATE_STATIC_DTABLEX4(DTable, HUF_MAX_TABLELOG) = u32[HUF_DTABLE_SIZE(12)] = {12}
    let mut DTable: [U32; (1 + (1u32 << HUF_MAX_TABLELOG)) as usize] =
        [0; (1 + (1u32 << HUF_MAX_TABLELOG)) as usize];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUF_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// --------------------------------------------------------------------------
// Generic decompression selector
// --------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}

static algoTime: [[algo_time_t; 3]; 16] = [
    [algo_time_t { tableTime: 0, decode256Time: 0 }, algo_time_t { tableTime: 1, decode256Time: 1 }, algo_time_t { tableTime: 2, decode256Time: 2 }],
    [algo_time_t { tableTime: 0, decode256Time: 0 }, algo_time_t { tableTime: 1, decode256Time: 1 }, algo_time_t { tableTime: 2, decode256Time: 2 }],
    [algo_time_t { tableTime: 38, decode256Time: 130 }, algo_time_t { tableTime: 1313, decode256Time: 74 }, algo_time_t { tableTime: 2151, decode256Time: 38 }],
    [algo_time_t { tableTime: 448, decode256Time: 128 }, algo_time_t { tableTime: 1353, decode256Time: 74 }, algo_time_t { tableTime: 2238, decode256Time: 41 }],
    [algo_time_t { tableTime: 556, decode256Time: 128 }, algo_time_t { tableTime: 1353, decode256Time: 74 }, algo_time_t { tableTime: 2238, decode256Time: 47 }],
    [algo_time_t { tableTime: 714, decode256Time: 128 }, algo_time_t { tableTime: 1418, decode256Time: 74 }, algo_time_t { tableTime: 2436, decode256Time: 53 }],
    [algo_time_t { tableTime: 883, decode256Time: 128 }, algo_time_t { tableTime: 1437, decode256Time: 74 }, algo_time_t { tableTime: 2464, decode256Time: 61 }],
    [algo_time_t { tableTime: 897, decode256Time: 128 }, algo_time_t { tableTime: 1515, decode256Time: 75 }, algo_time_t { tableTime: 2622, decode256Time: 68 }],
    [algo_time_t { tableTime: 926, decode256Time: 128 }, algo_time_t { tableTime: 1613, decode256Time: 75 }, algo_time_t { tableTime: 2730, decode256Time: 75 }],
    [algo_time_t { tableTime: 947, decode256Time: 128 }, algo_time_t { tableTime: 1729, decode256Time: 77 }, algo_time_t { tableTime: 3359, decode256Time: 77 }],
    [algo_time_t { tableTime: 1107, decode256Time: 128 }, algo_time_t { tableTime: 2083, decode256Time: 81 }, algo_time_t { tableTime: 4006, decode256Time: 84 }],
    [algo_time_t { tableTime: 1177, decode256Time: 128 }, algo_time_t { tableTime: 2379, decode256Time: 87 }, algo_time_t { tableTime: 4785, decode256Time: 88 }],
    [algo_time_t { tableTime: 1242, decode256Time: 128 }, algo_time_t { tableTime: 2415, decode256Time: 93 }, algo_time_t { tableTime: 5155, decode256Time: 84 }],
    [algo_time_t { tableTime: 1349, decode256Time: 128 }, algo_time_t { tableTime: 2644, decode256Time: 106 }, algo_time_t { tableTime: 5260, decode256Time: 106 }],
    [algo_time_t { tableTime: 1455, decode256Time: 128 }, algo_time_t { tableTime: 2422, decode256Time: 124 }, algo_time_t { tableTime: 4174, decode256Time: 124 }],
    [algo_time_t { tableTime: 722, decode256Time: 128 }, algo_time_t { tableTime: 1891, decode256Time: 145 }, algo_time_t { tableTime: 1936, decode256Time: 146 }],
];

pub unsafe fn HUF_decompress(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    // decompress[3] = { HUF_decompress4X2, HUF_decompress4X4, NULL }
    let Q: U32;
    let D256: U32 = (dstSize >> 8) as U32;
    let mut Dtime: [U32; 3] = [0; 3];
    let mut algoNb: U32 = 0;
    let mut n: i32;

    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as i32, dstSize);
        return dstSize;
    }

    Q = (cSrcSize.wrapping_mul(16) / dstSize) as U32;
    n = 0;
    while n < 3 {
        Dtime[n as usize] = algoTime[Q as usize][n as usize]
            .tableTime
            .wrapping_add(algoTime[Q as usize][n as usize].decode256Time.wrapping_mul(D256));
        n += 1;
    }

    Dtime[1] = Dtime[1].wrapping_add(Dtime[1] >> 4);
    Dtime[2] = Dtime[2].wrapping_add(Dtime[2] >> 3);

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
// zstd decompression section
// ============================================================================
pub const ZSTD_MEMORY_USAGE: u32 = 17;

pub const BIT7: u32 = 128;
pub const BIT6: u32 = 64;
pub const BIT5: u32 = 32;
pub const BIT4: u32 = 16;
pub const BIT1: u32 = 2;
pub const BIT0: u32 = 1;

pub const BLOCKSIZE: usize = 128 * 1024;
pub const MIN_SEQUENCES_SIZE: usize = 2 + 2 + 3 + 1;
pub const MIN_CBLOCK_SIZE: usize = 3 + MIN_SEQUENCES_SIZE;
pub const IS_RAW: u8 = BIT0 as u8;
pub const IS_RLE: u8 = BIT1 as u8;

pub const MINMATCH: usize = 4;
pub const MLbits: u32 = 7;
pub const LLbits: u32 = 6;
pub const Offbits: u32 = 5;
pub const MaxML: u32 = (1u32 << MLbits) - 1; // 127
pub const MaxLL: u32 = (1u32 << LLbits) - 1; // 63
pub const MaxOff: u32 = 31;
pub const LitFSELog: u32 = 11;
pub const MLFSELog: u32 = 10;
pub const LLFSELog: u32 = 10;
pub const OffFSELog: u32 = 9;
pub const MaxSeq: u32 = if MaxLL < MaxML { MaxML } else { MaxLL }; // 127

pub const LITERAL_NOENTROPY: u32 = 63;
pub const COMMAND_NOENTROPY: u32 = 7;

pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

pub const ZSTD_blockHeaderSize: size_t = 3;
pub const ZSTD_frameHeaderSize: size_t = 4;

pub const ZSTD_magicNumber: U32 = 0xFD2FB523;

// FSE_DTABLE_SIZE_U32
pub const LLTABLE_SIZE_U32: usize = (1 + (1usize << LLFSELog)) as usize; // 1025
pub const OFFTABLE_SIZE_U32: usize = (1 + (1usize << OffFSELog)) as usize; // 513
pub const MLTABLE_SIZE_U32: usize = (1 + (1usize << MLFSELog)) as usize; // 1025

#[inline]
pub unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}
#[inline]
pub unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

pub unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.wrapping_offset(length);
    // do { COPY8 } while (op < oend);
    loop {
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if !(op < oend) {
            break;
        }
    }
}

// blockType_t: bt_compressed, bt_raw, bt_rle, bt_end
pub type blockType_t = u32;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

#[inline]
pub unsafe fn ZSTD_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

// ----------------------------------------------------------------------------
// Decompression context
// ----------------------------------------------------------------------------
#[repr(C)]
pub struct ZSTD_DCtx {
    pub LLTable: [U32; LLTABLE_SIZE_U32],
    pub OffTable: [U32; OFFTABLE_SIZE_U32],
    pub MLTable: [U32; MLTABLE_SIZE_U32],
    pub previousDstEnd: *mut c_void,
    pub base: *mut c_void,
    pub expected: size_t,
    pub bType: blockType_t,
    pub phase: U32,
    pub litPtr: *const BYTE,
    pub litSize: size_t,
    pub litBuffer: [BYTE; BLOCKSIZE + 8],
}

// ZSTDv03_Dctx is the same struct
pub type ZSTDv03_Dctx = ZSTD_DCtx;

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
        .wrapping_add((*inp.add(1) as U32) << 8)
        .wrapping_add(((*inp.add(0) as U32) & 7) << 16);

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

pub unsafe fn ZSTD_copyUncompressedBlock(
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

pub unsafe fn ZSTD_decompressLiterals(
    dst: *mut c_void,
    maxDstSizePtr: *mut size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let ip = src as *const BYTE;

    let litSize: size_t = ((MEM_readLE32(src) & 0x1FFFFF) >> 2) as size_t;
    let litCSize: size_t = ((MEM_readLE32(ip.add(2) as *const c_void) & 0xFFFFFF) >> 5) as size_t;

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

pub unsafe fn ZSTD_decodeLiteralsBlock(
    ctx: *mut c_void,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let dctx = ctx as *mut ZSTD_DCtx;
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart) & 3 {
        // default & case 0
        1 => {
            // IS_RAW
            let litSize: size_t = ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as size_t;
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
            (*dctx).litPtr = istart.add(3);
            (*dctx).litSize = litSize;
            litSize + 3
        }
        2 => {
            // IS_RLE
            let litSize: size_t = ((MEM_readLE32(istart as *const c_void) & 0xFFFFFF) >> 2) as size_t;
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
        _ => {
            // default & case 0
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
            readSize
        }
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
    let iend = istart.wrapping_add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: U32 = 0;
    let mut Offlog: U32 = 0;
    let mut MLlog: U32 = 0;
    let dumpsLength: size_t;

    if srcSize < 5 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    *nbSeq = MEM_readLE16(ip as *const c_void) as i32;
    ip = ip.add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = (*ip.add(2) as size_t).wrapping_add((*ip.add(1) as size_t) << 8);
        ip = ip.add(3);
    } else {
        dumpsLength = (*ip.add(1) as size_t).wrapping_add(((*ip.add(0) as size_t) & 1) << 8);
        ip = ip.add(2);
    }
    *dumpsPtr = ip;
    ip = ip.wrapping_add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    if ip > iend.wrapping_sub(3) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    {
        let mut norm: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut headerSize: size_t;

        // LLtype
        if LLtype == bt_rle {
            LLlog = 0;
            FSE_buildDTable_rle(DTableLL, *ip);
            ip = ip.add(1);
        } else if LLtype == bt_raw {
            LLlog = LLbits;
            FSE_buildDTable_raw(DTableLL, LLbits);
        } else {
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
            ip = ip.wrapping_add(headerSize);
            FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
        }

        // Offtype
        if Offtype == bt_rle {
            Offlog = 0;
            if ip > iend.wrapping_sub(2) {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            FSE_buildDTable_rle(DTableOffb, *ip & MaxOff as BYTE);
            ip = ip.add(1);
        } else if Offtype == bt_raw {
            Offlog = Offbits;
            FSE_buildDTable_raw(DTableOffb, Offbits);
        } else {
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
            ip = ip.wrapping_add(headerSize);
            FSE_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
        }

        // MLtype
        if MLtype == bt_rle {
            MLlog = 0;
            if ip > iend.wrapping_sub(2) {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            FSE_buildDTable_rle(DTableML, *ip);
            ip = ip.add(1);
        } else if MLtype == bt_raw {
            MLlog = MLbits;
            FSE_buildDTable_raw(DTableML, MLbits);
        } else {
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
            ip = ip.wrapping_add(headerSize);
            FSE_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
        }
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
    (*seqState).prevOffset = (*seq).offset;
    if litLength == MaxLL as size_t {
        let add: U32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.add(1);
            v as U32
        } else {
            0
        };
        if add < 255 {
            litLength = litLength.wrapping_add(add as size_t);
        } else if dumps.wrapping_add(3) <= de {
            litLength = MEM_readLE24(dumps as *const c_void) as size_t;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1);
        }
    }

    // Offset
    {
        static offsetPrefix: [size_t; (MaxOff + 1) as usize] = [
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
            nbBits = 0;
        }
        offset = offsetPrefix[offsetCode as usize]
            .wrapping_add(BIT_readBits(&mut (*seqState).DStream, nbBits));
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset;
        }
    }

    // MatchLength
    matchLength = FSE_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as size_t;
    if matchLength == MaxML as size_t {
        let add: U32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.add(1);
            v as U32
        } else {
            0
        };
        if add < 255 {
            matchLength = matchLength.wrapping_add(add as size_t);
        } else if dumps.wrapping_add(3) <= de {
            matchLength = MEM_readLE24(dumps as *const c_void) as size_t;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1);
        }
    }
    matchLength = matchLength.wrapping_add(MINMATCH);

    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

pub unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> size_t {
    static dec32table: [i32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
    static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
    let ostart = op;
    let oLitEnd = op.wrapping_add(sequence.litLength);
    let oMatchEnd = op.wrapping_add(sequence.litLength).wrapping_add(sequence.matchLength);
    let oend_8 = oend.wrapping_sub(8);
    let litEnd = (*litPtr).wrapping_add(sequence.litLength);

    let seqLength: size_t = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > (oend.offset_from(op) as size_t) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit.offset_from(*litPtr) as size_t) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if oLitEnd > oend_8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.offset > (oLitEnd.offset_from(base) as U32 as size_t) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    if oMatchEnd > oend {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if litEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected);
    }

    // copy Literals
    ZSTD_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = litEnd;

    // copy Match
    {
        let mut match_: *const BYTE = op.wrapping_sub(sequence.offset);

        if sequence.offset > (op as size_t) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if match_ < base as *const BYTE {
            return ERROR(ZSTD_error_corruption_detected);
        }

        if sequence.offset < 8 {
            let dec64: i32 = dec64table[sequence.offset];
            *op.add(0) = *match_.add(0);
            *op.add(1) = *match_.add(1);
            *op.add(2) = *match_.add(2);
            *op.add(3) = *match_.add(3);
            match_ = match_.wrapping_add(dec32table[sequence.offset] as usize);
            ZSTD_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
            match_ = match_.wrapping_sub(dec64 as usize);
        } else {
            ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
        }
        op = op.add(8);
        match_ = match_.add(8);

        if oMatchEnd > oend.wrapping_sub(16 - MINMATCH) {
            if op < oend_8 {
                ZSTD_wildcopy(
                    op as *mut c_void,
                    match_ as *const c_void,
                    oend_8.offset_from(op) as isize,
                );
                match_ = match_.wrapping_add(oend_8.offset_from(op) as usize);
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
                (sequence.matchLength as isize) - 8,
            );
        }
    }

    oMatchEnd.offset_from(ostart) as size_t
}

pub unsafe fn ZSTD_decompressSequences(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
) -> size_t {
    let dctx = ctx as *mut ZSTD_DCtx;
    let mut ip = seqStart as *const BYTE;
    let iend = ip.wrapping_add(seqSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.wrapping_add(maxDstSize);
    let mut errorCode: size_t;
    let mut dumpsLength: size_t = 0;
    let mut litPtr = (*dctx).litPtr;
    let litEnd = litPtr.wrapping_add((*dctx).litSize);
    let mut nbSeq: i32 = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *mut BYTE;

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
    ip = ip.wrapping_add(errorCode);

    {
        let mut sequence: seq_t = core::mem::zeroed();
        let mut seqState: seqState_t = core::mem::zeroed();

        memset(&mut sequence as *mut _ as *mut c_void, 0, core::mem::size_of::<seq_t>());
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.wrapping_add(dumpsLength);
        sequence.offset = 4;
        seqState.prevOffset = sequence.offset;
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

        while (BIT_reloadDStream(&mut seqState.DStream) <= BIT_DStream_completed) && (nbSeq > 0) {
            let oneSeqSize: size_t;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(op, sequence, &mut litPtr, litEnd, base, oend);
            if ZSTD_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.wrapping_add(oneSeqSize);
        }

        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if nbSeq < 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        // last literal segment
        {
            let lastLLSize: size_t = litEnd.offset_from(litPtr) as size_t;
            if litPtr > litEnd {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if op.wrapping_add(lastLLSize) > oend {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            if lastLLSize > 0 {
                if op != litPtr as *mut BYTE {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.wrapping_add(lastLLSize);
            }
        }
    }

    op.offset_from(ostart) as size_t
}

pub unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip = src as *const BYTE;

    let litCSize = ZSTD_decodeLiteralsBlock(ctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.wrapping_add(litCSize);
    srcSize -= litCSize;

    ZSTD_decompressSequences(ctx, dst, maxDstSize, ip as *const c_void, srcSize)
}

pub unsafe fn ZSTD_decompressDCtx(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut ip = src as *const BYTE;
    let iend = ip.wrapping_add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.wrapping_add(maxDstSize);
    let mut remainingSize = srcSize;
    let magicNumber: U32;
    let mut blockProperties: blockProperties_t = core::mem::zeroed();

    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_magicNumber {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    ip = ip.wrapping_add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

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

        ip = ip.wrapping_add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        if blockProperties.blockType == bt_compressed {
            decodedSize = ZSTD_decompressBlock(
                ctx,
                op as *mut c_void,
                oend.offset_from(op) as size_t,
                ip as *const c_void,
                cBlockSize,
            );
        } else if blockProperties.blockType == bt_raw {
            decodedSize = ZSTD_copyUncompressedBlock(
                op as *mut c_void,
                oend.offset_from(op) as size_t,
                ip as *const c_void,
                cBlockSize,
            );
        } else if blockProperties.blockType == bt_rle {
            return ERROR(ZSTD_error_GENERIC);
        } else if blockProperties.blockType == bt_end {
            if remainingSize != 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
        } else {
            return ERROR(ZSTD_error_GENERIC);
        }
        if cBlockSize == 0 {
            break;
        }

        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.wrapping_add(decodedSize);
        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op.offset_from(ostart) as size_t
}

pub unsafe fn ZSTD_decompress(
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut ctx: ZSTD_DCtx = core::mem::zeroed();
    ctx.base = dst;
    ZSTD_decompressDCtx(
        &mut ctx as *mut _ as *mut c_void,
        dst,
        maxDstSize,
        src,
        srcSize,
    )
}

pub unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut size_t,
    dBound: *mut u64,
    ret: size_t,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

// ----------------------------------------------------------------------------
// Streaming Decompression API (internal)
// ----------------------------------------------------------------------------
pub unsafe fn ZSTD_resetDCtx(dctx: *mut ZSTD_DCtx) -> size_t {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0;
    (*dctx).previousDstEnd = core::ptr::null_mut();
    (*dctx).base = core::ptr::null_mut();
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
    if srcSize != (*ctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dst != (*ctx).previousDstEnd {
        (*ctx).base = dst;
    }

    // frame header
    if (*ctx).phase == 0 {
        let magicNumber: U32 = MEM_readLE32(src);
        if magicNumber != ZSTD_magicNumber {
            return ERROR(ZSTD_error_prefix_unknown);
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        return 0;
    }

    // block header
    if (*ctx).phase == 1 {
        let mut bp: blockProperties_t = core::mem::zeroed();
        let blockSize = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &mut bp);
        if ZSTD_isError(blockSize) != 0 {
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

    // block content
    {
        let rSize: size_t;
        if (*ctx).bType == bt_compressed {
            rSize = ZSTD_decompressBlock(ctx as *mut c_void, dst, maxDstSize, src, srcSize);
        } else if (*ctx).bType == bt_raw {
            rSize = ZSTD_copyUncompressedBlock(dst, maxDstSize, src, srcSize);
        } else if (*ctx).bType == bt_rle {
            return ERROR(ZSTD_error_GENERIC);
        } else if (*ctx).bType == bt_end {
            rSize = 0;
        } else {
            return ERROR(ZSTD_error_GENERIC);
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTD_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = ((dst as *mut i8).wrapping_add(rSize)) as *mut c_void;
        rSize
    }
}

// ============================================================================
// Wrapper layer (the 8 exported linker symbols)
// ============================================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_isError(code: size_t) -> u32 {
    ZSTD_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_decompress(
    dst: *mut c_void,
    maxOriginalSize: size_t,
    src: *const c_void,
    compressedSize: size_t,
) -> size_t {
    ZSTD_decompress(dst, maxOriginalSize, src, compressedSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: size_t,
    cSize: *mut size_t,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: size_t = 0;
    let magicNumber: U32;
    let mut blockProperties: blockProperties_t = core::mem::zeroed();

    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_magicNumber {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
        return;
    }
    ip = ip.wrapping_add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    loop {
        let cBlockSize = ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break;
        }

        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = ip.offset_from(src as *const BYTE) as size_t;
    *dBound = (nbBlocks as u64).wrapping_mul(BLOCKSIZE as u64);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_createDCtx() -> *mut ZSTDv03_Dctx {
    ZSTD_createDCtx() as *mut ZSTDv03_Dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_freeDCtx(dctx: *mut ZSTDv03_Dctx) -> size_t {
    ZSTD_freeDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_resetDCtx(dctx: *mut ZSTDv03_Dctx) -> size_t {
    ZSTD_resetDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_nextSrcSizeToDecompress(dctx: *mut ZSTDv03_Dctx) -> size_t {
    ZSTD_nextSrcSizeToDecompress(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_decompressContinue(
    dctx: *mut ZSTDv03_Dctx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_decompressContinue(dctx as *mut ZSTD_DCtx, dst, maxDstSize, src, srcSize)
}
