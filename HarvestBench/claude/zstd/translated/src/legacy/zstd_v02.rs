//! Translation of legacy/zstd_v02.c (zstd v0.2 decompressor).
//! Self-contained: local mem/BIT/FSE/HUF/ZSTD implementations, LE 64-bit.
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens,
    unused_variables
)]

use core::ffi::c_void;

use crate::common::allocations::{free, malloc, memcpy, memmove, memset};

// ------------------------------------------------------------------
// Basic types
// ------------------------------------------------------------------
type BYTE = u8;
type U16 = u16;
type S16 = i16;
type U32 = u32;
type S32 = i32;
type U64 = u64;
type S64 = i64;

// ------------------------------------------------------------------
// Error management (local enum ZSTD_error_* : values 0..9)
// ------------------------------------------------------------------
// NOTE: In the C, legacy files include ../common/error_private.h FIRST, which
// defines ERROR_H_MODULE and guards out the file-local ERR_codes enum. So
// ERROR(name)/ERR_isError use the MODERN zstd_errors.h values, not the
// position-based local enum. Reproduce the modern values here.
mod err {
    pub const No_Error: i32 = 0;
    pub const GENERIC: i32 = 1;
    pub const dstSize_tooSmall: i32 = 70;
    pub const srcSize_wrong: i32 = 72;
    pub const prefix_unknown: i32 = 10;
    pub const corruption_detected: i32 = 20;
    pub const tableLog_tooLarge: i32 = 44;
    pub const maxSymbolValue_tooLarge: i32 = 46;
    pub const maxSymbolValue_tooSmall: i32 = 48;
    pub const maxCode: i32 = 120;
}

#[inline]
fn ERROR(code: i32) -> usize {
    (-(code as isize)) as usize
}

#[inline]
fn ERR_isError(code: usize) -> u32 {
    (code > ERROR(err::maxCode)) as u32
}

// ------------------------------------------------------------------
// Memory I/O  (little-endian, 64-bit)
// ------------------------------------------------------------------
#[inline]
fn MEM_32bits() -> u32 {
    0
}
#[inline]
fn MEM_64bits() -> u32 {
    1
}
#[inline]
fn MEM_isLittleEndian() -> u32 {
    1
}

#[inline]
unsafe fn MEM_read16(memPtr: *const u8) -> U16 {
    (memPtr as *const U16).read_unaligned()
}
#[inline]
unsafe fn MEM_read32(memPtr: *const u8) -> U32 {
    (memPtr as *const U32).read_unaligned()
}
#[inline]
unsafe fn MEM_read64(memPtr: *const u8) -> U64 {
    (memPtr as *const U64).read_unaligned()
}
#[inline]
unsafe fn MEM_write16(memPtr: *mut u8, value: U16) {
    (memPtr as *mut U16).write_unaligned(value);
}
#[inline]
unsafe fn MEM_readLE16(memPtr: *const u8) -> U16 {
    MEM_read16(memPtr)
}
#[inline]
unsafe fn MEM_writeLE16(memPtr: *mut u8, val: U16) {
    MEM_write16(memPtr, val);
}
#[inline]
unsafe fn MEM_readLE24(memPtr: *const u8) -> U32 {
    (MEM_readLE16(memPtr) as U32) + ((*memPtr.add(2) as U32) << 16)
}
#[inline]
unsafe fn MEM_readLE32(memPtr: *const u8) -> U32 {
    MEM_read32(memPtr)
}
#[inline]
unsafe fn MEM_readLE64(memPtr: *const u8) -> U64 {
    MEM_read64(memPtr)
}
#[inline]
unsafe fn MEM_readLEST(memPtr: *const u8) -> usize {
    MEM_readLE64(memPtr) as usize
}

// ------------------------------------------------------------------
// bitstream helpers
// ------------------------------------------------------------------
#[inline]
fn BIT_highbit32(val: U32) -> u32 {
    val.leading_zeros() ^ 31
}

#[repr(C)]
#[derive(Clone, Copy)]
struct BIT_DStream_t {
    bitContainer: usize,
    bitsConsumed: u32,
    ptr: *const i8,
    start: *const i8,
}

const BIT_DStream_unfinished: u32 = 0;
const BIT_DStream_endOfBuffer: u32 = 1;
const BIT_DStream_completed: u32 = 2;
const BIT_DStream_overflow: u32 = 3;
type BIT_DStream_status = u32;

unsafe fn BIT_initDStream(bitD: *mut BIT_DStream_t, srcBuffer: *const c_void, srcSize: usize) -> usize {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BIT_DStream_t>());
        return ERROR(err::srcSize_wrong);
    }

    if srcSize >= core::mem::size_of::<usize>() {
        // normal case
        let contain32: U32;
        (*bitD).start = srcBuffer as *const i8;
        (*bitD).ptr = (srcBuffer as *const i8).add(srcSize - core::mem::size_of::<usize>());
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const u8);
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(err::GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BIT_highbit32(contain32);
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const i8;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as usize;
        let start = (*bitD).start as *const BYTE;
        match srcSize {
            7 => {
                (*bitD).bitContainer += (*start.add(6) as usize) << (core::mem::size_of::<usize>() * 8 - 16);
                (*bitD).bitContainer += (*start.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24);
                (*bitD).bitContainer += (*start.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
                (*bitD).bitContainer += (*start.add(3) as usize) << 24;
                (*bitD).bitContainer += (*start.add(2) as usize) << 16;
                (*bitD).bitContainer += (*start.add(1) as usize) << 8;
            }
            6 => {
                (*bitD).bitContainer += (*start.add(5) as usize) << (core::mem::size_of::<usize>() * 8 - 24);
                (*bitD).bitContainer += (*start.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
                (*bitD).bitContainer += (*start.add(3) as usize) << 24;
                (*bitD).bitContainer += (*start.add(2) as usize) << 16;
                (*bitD).bitContainer += (*start.add(1) as usize) << 8;
            }
            5 => {
                (*bitD).bitContainer += (*start.add(4) as usize) << (core::mem::size_of::<usize>() * 8 - 32);
                (*bitD).bitContainer += (*start.add(3) as usize) << 24;
                (*bitD).bitContainer += (*start.add(2) as usize) << 16;
                (*bitD).bitContainer += (*start.add(1) as usize) << 8;
            }
            4 => {
                (*bitD).bitContainer += (*start.add(3) as usize) << 24;
                (*bitD).bitContainer += (*start.add(2) as usize) << 16;
                (*bitD).bitContainer += (*start.add(1) as usize) << 8;
            }
            3 => {
                (*bitD).bitContainer += (*start.add(2) as usize) << 16;
                (*bitD).bitContainer += (*start.add(1) as usize) << 8;
            }
            2 => {
                (*bitD).bitContainer += (*start.add(1) as usize) << 8;
            }
            _ => {}
        }
        contain32 = *(srcBuffer as *const BYTE).add(srcSize - 1) as U32;
        if contain32 == 0 {
            return ERROR(err::GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BIT_highbit32(contain32);
        (*bitD).bitsConsumed += ((core::mem::size_of::<usize>() - srcSize) as U32) * 8;
    }

    srcSize
}

#[inline]
unsafe fn BIT_lookBits(bitD: *const BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1) >> ((bitMask - nbBits) & bitMask)
}

#[inline]
unsafe fn BIT_lookBitsFast(bitD: *const BIT_DStream_t, nbBits: U32) -> usize {
    let bitMask: U32 = (core::mem::size_of::<usize>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> (((bitMask + 1) - nbBits) & bitMask)
}

#[inline]
unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
unsafe fn BIT_readBits(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

#[inline]
unsafe fn BIT_readBitsFast(bitD: *mut BIT_DStream_t, nbBits: U32) -> usize {
    let value = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

unsafe fn BIT_reloadDStream(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).bitsConsumed > (core::mem::size_of::<usize>() * 8) as U32 {
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.add(core::mem::size_of::<usize>()) {
        (*bitD).ptr = (*bitD).ptr.offset(-(((*bitD).bitsConsumed >> 3) as isize));
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const u8);
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (core::mem::size_of::<usize>() * 8) as U32 {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BIT_DStream_status = BIT_DStream_unfinished;
        if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32;
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const u8);
        result
    }
}

#[inline]
unsafe fn BIT_endOfDStream(DStream: *const BIT_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<usize>() * 8) as U32)) as u32
}

// ------------------------------------------------------------------
// FSE : Finite State Entropy decoder
// ------------------------------------------------------------------
type FSE_CTable = u32;
type FSE_DTable = u32;

const FSE_MAX_MEMORY_USAGE: u32 = 14;
const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;

const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
const FSE_MAX_TABLESIZE: u32 = 1u32 << FSE_MAX_TABLELOG;
const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

const FSE_DTABLE_SIZE_U32_MACRO_MAXTABLELOG: usize = (1 + (1 << FSE_MAX_TABLELOG)) as usize;
// DTable_max_t = U32[FSE_DTABLE_SIZE_U32(FSE_MAX_TABLELOG)]
type DTable_max_t = [U32; FSE_DTABLE_SIZE_U32_MACRO_MAXTABLELOG];

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DState_t {
    state: usize,
    table: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_DTableHeader {
    tableLog: U16,
    fastMode: U16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FSE_decode_t {
    newState: u16,
    symbol: u8,
    nbBits: u8,
}

#[inline]
unsafe fn FSE_initDState(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t, dt: *const FSE_DTable) {
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
unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBits(bitD, nbBits);

    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
unsafe fn FSE_decodeSymbolFast(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBitsFast(bitD, nbBits);

    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline]
unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

#[inline]
fn FSE_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

unsafe fn FSE_buildDTable(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    let ptr = dt.add(1) as *mut c_void;
    let tableDecode = ptr as *mut FSE_decode_t;
    let mut DTableH: FSE_DTableHeader = core::mem::zeroed();
    let tableSize: U32 = 1u32 << tableLog;
    let tableMask: U32 = tableSize - 1;
    let step: U32 = FSE_tableStep(tableSize);
    let mut symbolNext: [U16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] = [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize - 1;
    let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return ERROR(err::maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(err::tableLog_tooLarge);
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

    s = 0;
    while s <= maxSymbolValue {
        let mut i: i32 = 0;
        while i < *normalizedCounter.add(s as usize) as i32 {
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
        return ERROR(err::GENERIC);
    }

    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] += 1;
            (*tableDecode.add(i as usize)).nbBits = (tableLog - BIT_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(i as usize)).newState =
                (((nextState as U32) << (*tableDecode.add(i as usize)).nbBits).wrapping_sub(tableSize)) as U16;
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
fn FSE_isError(code: usize) -> u32 {
    ERR_isError(code)
}

#[inline]
fn FSE_abs(a: S16) -> S16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

unsafe fn FSE_readNCount(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
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
        return ERROR(err::srcSize_wrong);
    }
    bitStream = MEM_readLE32(ip);
    nbBits = ((bitStream & 0xF) + FSE_MIN_TABLELOG) as i32;
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as i32 {
        return ERROR(err::tableLog_tooLarge);
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
                return ERROR(err::maxSymbolValue_tooSmall);
            }
            while charnum < n0 {
                *normalizedCounter.add(charnum as usize) = 0;
                charnum += 1;
            }
            if (ip <= iend.offset(-7)) || (ip.add((bitCount >> 3) as usize) <= iend.offset(-4)) {
                ip = ip.add((bitCount >> 3) as usize);
                bitCount &= 7;
                bitStream = MEM_readLE32(ip) >> bitCount;
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
                if count >= threshold as S16 {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1;
            remaining -= FSE_abs(count) as i32;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as i32;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

            {
                if (ip <= iend.offset(-7)) || (ip.add((bitCount >> 3) as usize) <= iend.offset(-4)) {
                    ip = ip.add((bitCount >> 3) as usize);
                    bitCount &= 7;
                } else {
                    bitCount -= (8 * (iend.offset(-4).offset_from(ip))) as i32;
                    ip = iend.offset(-4);
                }
                bitStream = MEM_readLE32(ip) >> (bitCount & 31);
            }
        }
    }
    if remaining != 1 {
        return ERROR(err::GENERIC);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.add(((bitCount + 7) >> 3) as usize);
    if (ip.offset_from(istart) as usize) > hbSize {
        return ERROR(err::srcSize_wrong);
    }
    ip.offset_from(istart) as usize
}

unsafe fn FSE_buildDTable_rle(dt: *mut FSE_DTable, symbolValue: BYTE) -> usize {
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

unsafe fn FSE_buildDTable_raw(dt: *mut FSE_DTable, nbBits: u32) -> usize {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSE_DTableHeader;
    let dinfo = (ptr as *mut FSE_decode_t).add(1);
    let tableSize: u32 = 1u32 << nbBits;
    let tableMask: u32 = tableSize - 1;
    let maxSymbolValue: u32 = tableMask;
    let mut s: u32;

    if nbBits < 1 {
        return ERROR(err::GENERIC);
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
    let omax = op.add(maxDstSize);
    let olimit = omax.offset(-3);

    let mut bitD: BIT_DStream_t = core::mem::zeroed();
    let mut state1: FSE_DState_t = core::mem::zeroed();
    let mut state2: FSE_DState_t = core::mem::zeroed();
    let mut errorCode: usize;

    errorCode = BIT_initDStream(&mut bitD, cSrc, cSrcSize);
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

    // 4 symbols per loop
    while (BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) && (op < olimit) {
        *op.add(0) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(1) = FSE_GETSYMBOL!(&mut state2);

        if FSE_MAX_TABLELOG * 4 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<usize>() * 8) as u32 {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(3) = FSE_GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    // tail
    loop {
        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0 && (fast != 0 || FSE_endOfDState(&state1) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL!(&mut state1);
        op = op.add(1);

        if (BIT_reloadDStream(&mut bitD) > BIT_DStream_completed)
            || (op == omax)
            || (BIT_endOfDStream(&bitD) != 0 && (fast != 0 || FSE_endOfDState(&state2) != 0))
        {
            break;
        }

        *op = FSE_GETSYMBOL!(&mut state2);
        op = op.add(1);
    }

    if BIT_endOfDStream(&bitD) != 0 && FSE_endOfDState(&state1) != 0 && FSE_endOfDState(&state2) != 0 {
        return op.offset_from(ostart) as usize;
    }

    if op == omax {
        return ERROR(err::dstSize_tooSmall);
    }

    ERROR(err::corruption_detected)
}

unsafe fn FSE_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
) -> usize {
    let mut DTableH: FSE_DTableHeader = core::mem::zeroed();
    memcpy(
        &mut DTableH as *mut FSE_DTableHeader as *mut c_void,
        dt as *const c_void,
        core::mem::size_of::<FSE_DTableHeader>(),
    );

    if DTableH.fastMode != 0 {
        return FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSE_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

unsafe fn FSE_decompress(dst: *mut c_void, maxDstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [S16; (FSE_MAX_SYMBOL_VALUE + 1) as usize] = [0; (FSE_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: DTable_max_t = [0; FSE_DTABLE_SIZE_U32_MACRO_MAXTABLELOG];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE;
    let mut errorCode: usize;
    let mut cSrcSize = cSrcSize;

    if cSrcSize < 2 {
        return ERROR(err::srcSize_wrong);
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
        return ERROR(err::srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSE_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }

    FSE_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

// ------------------------------------------------------------------
// Huff0 : Huffman decoder
// ------------------------------------------------------------------
#[inline]
fn HUF_isError(code: usize) -> u32 {
    ERR_isError(code)
}

const HUF_ABSOLUTEMAX_TABLELOG: u32 = 16;
const HUF_MAX_TABLELOG: u32 = 12;
const HUF_DEFAULT_TABLELOG: u32 = HUF_MAX_TABLELOG;
const HUF_MAX_SYMBOL_VALUE: u32 = 255;

// HUF_DTABLE_SIZE(maxTableLog) = 1 + (1<<maxTableLog)
const HUF_DTABLE_SIZE_MAX: usize = (1 + (1u32 << HUF_MAX_TABLELOG)) as usize;

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX2 {
    byte: BYTE,
    nbBits: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX4 {
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

unsafe fn HUF_readStats(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut weightTotal: U32;
    let mut tableLog: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: usize;
    let mut oSize: usize;
    let mut n: U32;

    if srcSize == 0 {
        return ERROR(err::srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        // special header
        if iSize >= 242 {
            // RLE
            static l: [i32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = l[iSize - 242] as usize;
            memset(huffWeight as *mut c_void, 1, hwSize);
            iSize = 0;
        } else {
            // Incompressible
            oSize = iSize - 127;
            iSize = (oSize + 1) / 2;
            if iSize + 1 > srcSize {
                return ERROR(err::srcSize_wrong);
            }
            if oSize >= hwSize {
                return ERROR(err::corruption_detected);
            }
            ip = ip.add(1);
            n = 0;
            while (n as usize) < oSize {
                *huffWeight.add(n as usize) = *ip.add((n / 2) as usize) >> 4;
                *huffWeight.add(n as usize + 1) = *ip.add((n / 2) as usize) & 15;
                n += 2;
            }
        }
    } else {
        // header compressed with FSE (normal case)
        if iSize + 1 > srcSize {
            return ERROR(err::srcSize_wrong);
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
        (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as usize) < oSize {
        if (*huffWeight.add(n as usize) as U32) >= HUF_ABSOLUTEMAX_TABLELOG {
            return ERROR(err::corruption_detected);
        }
        *rankStats.add(*huffWeight.add(n as usize) as usize) += 1;
        weightTotal += (1u32 << *huffWeight.add(n as usize)) >> 1;
        n += 1;
    }
    if weightTotal == 0 {
        return ERROR(err::corruption_detected);
    }

    // get last non-null symbol weight (implied, total must be 2^n)
    tableLog = BIT_highbit32(weightTotal) + 1;
    if tableLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(err::corruption_detected);
    }
    {
        let total: U32 = 1u32 << tableLog;
        let rest: U32 = total - weightTotal;
        let verif: U32 = 1u32 << BIT_highbit32(rest);
        let lastWeight: U32 = BIT_highbit32(rest) + 1;
        if verif != rest {
            return ERROR(err::corruption_detected);
        }
        *huffWeight.add(oSize) = lastWeight as BYTE;
        *rankStats.add(lastWeight as usize) += 1;
    }

    // check tree construction validity
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(err::corruption_detected);
    }

    // results
    *nbSymbolsPtr = (oSize + 1) as U32;
    *tableLogPtr = tableLog;
    iSize + 1
}

// -------- single-symbol decoding (X2) --------

unsafe fn HUF_readDTableX2(DTable: *mut U16, src: *const c_void, srcSize: usize) -> usize {
    let mut huffWeight: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] = [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] = [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut tableLog: U32 = 0;
    let ip = src as *const BYTE;
    let mut iSize: usize = *ip.add(0) as usize;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let ptr = DTable.add(1) as *mut c_void;
    let dt = ptr as *mut HUF_DEltX2;

    iSize = HUF_readStats(
        huffWeight.as_mut_ptr(),
        (HUF_MAX_SYMBOL_VALUE + 1) as usize,
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
        return ERROR(err::tableLog_tooLarge);
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
        rankVal[w as usize] += length;
        n += 1;
    }

    iSize
}

#[inline]
unsafe fn HUF_decodeSymbolX2(Dstream: *mut BIT_DStream_t, dt: *const HUF_DEltX2, dtLog: U32) -> BYTE {
    let val = BIT_lookBitsFast(Dstream, dtLog);
    let c = (*dt.add(val)).byte;
    BIT_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

unsafe fn HUF_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;

    // up to 4 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.offset(-4)) {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    // closer to the end
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd) {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    // no more data to retrieve from bitstream, hence no need to reload
    while p < pEnd {
        *p = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        p = p.add(1);
    }

    pEnd.offset_from(pStart) as usize
}

unsafe fn HUF_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U16,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(err::corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DEltX2).add(1);
        let dtLog: U32 = *DTable.add(0) as U32;
        let mut errorCode: usize;

        let mut bitD1: BIT_DStream_t = core::mem::zeroed();
        let mut bitD2: BIT_DStream_t = core::mem::zeroed();
        let mut bitD3: BIT_DStream_t = core::mem::zeroed();
        let mut bitD4: BIT_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart) as usize;
        let length2 = MEM_readLE16(istart.add(2)) as usize;
        let length3 = MEM_readLE16(istart.add(4)) as usize;
        let mut length4: usize;
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
            return ERROR(err::corruption_detected);
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

        macro_rules! DEC {
            ($op:expr, $bd:expr) => {{
                *$op = HUF_decodeSymbolX2($bd, dt, dtLog);
                $op = $op.add(1);
            }};
        }

        // 16-32 symbols per loop
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.offset(-7)) {
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        if op1 > opStart2 {
            return ERROR(err::corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(err::corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(err::corruption_detected);
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
            return ERROR(err::corruption_detected);
        }

        dstSize
    }
}

unsafe fn HUF_decompress4X2(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    let mut DTable: [U16; HUF_DTABLE_SIZE_MAX] = [0; HUF_DTABLE_SIZE_MAX];
    DTable[0] = HUF_MAX_TABLELOG as U16;
    let mut ip = cSrc as *const BYTE;
    let mut errorCode: usize;
    let mut cSrcSize = cSrcSize;

    errorCode = HUF_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(err::srcSize_wrong);
    }
    ip = ip.add(errorCode);
    cSrcSize -= errorCode;

    HUF_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// -------- double-symbols decoding (X4) --------

// rankVal_t = U32[HUF_ABSOLUTEMAX_TABLELOG][HUF_ABSOLUTEMAX_TABLELOG + 1]
type rankVal_t = [[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]; HUF_ABSOLUTEMAX_TABLELOG as usize];

unsafe fn HUF_fillDTableX4Level2(
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
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] = [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
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
        MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut u8, baseSeq);
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
        let nbBits: U32 = nbBitsBaseline - weight;
        let length: U32 = 1u32 << (sizeLog - nbBits);
        let start: U32 = rankVal[weight as usize];
        let mut i: U32 = start;
        let end: U32 = start + length;

        MEM_writeLE16(
            &mut DElt.sequence as *mut U16 as *mut u8,
            (baseSeq as U32).wrapping_add(symbol << 8) as U16,
        );
        DElt.nbBits = (nbBits + consumed) as BYTE;
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

unsafe fn HUF_fillDTableX4(
    DTable: *mut HUF_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *mut rankVal_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] = [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: i32 = (nbBitsBaseline - targetLog) as i32;
    let minBits: U32 = nbBitsBaseline - maxWeight;
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        (*rankValOrigin).as_ptr() as *const c_void,
        core::mem::size_of::<[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    // fill DTable
    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline - weight;
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32 << (targetLog - nbBits);

        if targetLog - nbBits >= minBits {
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
                (*rankValOrigin)[nbBits as usize].as_ptr(),
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

            MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut u8, symbol);
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

unsafe fn HUF_readDTableX4(DTable: *mut U32, src: *const c_void, srcSize: usize) -> usize {
    let mut weightList: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] = [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] = [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize] = [0; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let mut rankVal: rankVal_t = [[0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]; HUF_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let mut sizeOfSort: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.add(0);
    let ip = src as *const BYTE;
    let mut iSize: usize = *ip.add(0) as usize;
    let ptr = DTable as *mut c_void;
    let dt = (ptr as *mut HUF_DEltX4).add(1);

    // rankStart = rankStart0+1
    macro_rules! rankStart {
        ($i:expr) => {
            rankStart0[($i as usize) + 1]
        };
    }

    if memLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(err::tableLog_tooLarge);
    }

    iSize = HUF_readStats(
        weightList.as_mut_ptr(),
        (HUF_MAX_SYMBOL_VALUE + 1) as usize,
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
        return ERROR(err::tableLog_tooLarge);
    }

    // find maxWeight
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        if maxW == 0 {
            return ERROR(err::GENERIC);
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
            rankStart!(w) = current;
            w += 1;
        }
        rankStart!(0) = nextRankStart;
        sizeOfSort = nextRankStart;
    }

    // sort symbols by weight
    {
        let mut s: U32;
        s = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = rankStart!(w);
            rankStart!(w) += 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        rankStart!(0) = 0;
    }

    // Build rankVal
    {
        let minBits: U32 = tableLog + 1 - maxW;
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: i32 = (memLog - tableLog) as i32 - 1;
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
        &mut rankVal,
        maxW,
        tableLog + 1,
    );

    iSize
}

#[inline]
unsafe fn HUF_decodeSymbolX4(op: *mut c_void, DStream: *mut BIT_DStream_t, dt: *const HUF_DEltX4, dtLog: U32) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 2);
    BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

#[inline]
unsafe fn HUF_decodeLastSymbolX4(op: *mut c_void, DStream: *mut BIT_DStream_t, dt: *const HUF_DEltX4, dtLog: U32) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (core::mem::size_of::<usize>() * 8) as U32 {
            BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (core::mem::size_of::<usize>() * 8) as U32 {
                (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as U32;
            }
        }
    }
    1
}

unsafe fn HUF_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> usize {
    let pStart = p;

    // up to 8 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p < pEnd.offset(-7)) {
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    // closer to the end
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.offset(-2)) {
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    while p <= pEnd.offset(-2) {
        p = p.add(HUF_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    if p < pEnd {
        p = p.add(HUF_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    p.offset_from(pStart) as usize
}

unsafe fn HUF_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U32,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(err::corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);

        let ptr = DTable as *const c_void;
        let dt = (ptr as *const HUF_DEltX4).add(1);
        let dtLog: U32 = *DTable.add(0);
        let mut errorCode: usize;

        let mut bitD1: BIT_DStream_t = core::mem::zeroed();
        let mut bitD2: BIT_DStream_t = core::mem::zeroed();
        let mut bitD3: BIT_DStream_t = core::mem::zeroed();
        let mut bitD4: BIT_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart) as usize;
        let length2 = MEM_readLE16(istart.add(2)) as usize;
        let length3 = MEM_readLE16(istart.add(4)) as usize;
        let mut length4: usize;
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
            return ERROR(err::corruption_detected);
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

        macro_rules! DEC {
            ($op:expr, $bd:expr) => {{
                $op = $op.add(HUF_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize);
            }};
        }

        // 16-32 symbols per loop
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (endSignal == BIT_DStream_unfinished) && (op4 < oend.offset(-7)) {
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        if op1 > opStart2 {
            return ERROR(err::corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(err::corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(err::corruption_detected);
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
            return ERROR(err::corruption_detected);
        }

        dstSize
    }
}

unsafe fn HUF_decompress4X4(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    let mut DTable: [U32; HUF_DTABLE_SIZE_MAX] = [0; HUF_DTABLE_SIZE_MAX];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let hSize = HUF_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(err::srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// -------- quad-symbol decoding (X6) --------

#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DDescX6 {
    nbBits: BYTE,
    nbBytes: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
union HUF_DSeqX6 {
    byte: [BYTE; 4],
    sequence: U32,
}

// HUF_CREATE_STATIC_DTABLEX6 : U32[HUF_DTABLE_SIZE(maxTableLog) * 3 / 2]
const HUF_DTABLE_SIZE_X6_MAX: usize = HUF_DTABLE_SIZE_MAX * 3 / 2;

unsafe fn HUF_fillDTableX6LevelN(
    DDescription: *mut HUF_DDescX6,
    DSequence: *mut HUF_DSeqX6,
    sizeLog: i32,
    rankValOrigin: *const rankVal_t,
    consumed: U32,
    minWeight: i32,
    maxWeight: U32,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    nbBitsBaseline: U32,
    mut baseSeq: HUF_DSeqX6,
    mut DDesc: HUF_DDescX6,
) {
    let scaleLog: i32 = nbBitsBaseline as i32 - sizeLog;
    let minBits: i32 = nbBitsBaseline as i32 - maxWeight as i32;
    let level: U32 = DDesc.nbBytes as U32;
    let mut rankVal: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] = [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let symbolStartPos;
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        (*rankValOrigin)[consumed as usize].as_ptr() as *const c_void,
        core::mem::size_of::<[U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    // fill skipped values
    if minWeight > 1 {
        let mut i: U32;
        let skipSize: U32 = rankVal[minWeight as usize];
        i = 0;
        while i < skipSize {
            *DSequence.add(i as usize) = baseSeq;
            *DDescription.add(i as usize) = DDesc;
            i += 1;
        }
    }

    // fill DTable
    DDesc.nbBytes += 1;
    symbolStartPos = *rankStart.add(minWeight as usize);
    s = symbolStartPos;
    while s < sortedListSize {
        let symbol: BYTE = (*sortedSymbols.add(s as usize)).symbol;
        let weight: U32 = (*sortedSymbols.add(s as usize)).weight as U32;
        let nbBits: i32 = nbBitsBaseline as i32 - weight as i32;
        let totalBits: i32 = consumed as i32 + nbBits;
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32 << (sizeLog - nbBits);
        baseSeq.byte[level as usize] = symbol;
        DDesc.nbBits = totalBits as BYTE;

        if (level < 3) && (sizeLog - totalBits >= minBits) {
            let mut nextMinWeight: i32 = totalBits + scaleLog;
            if nextMinWeight < 1 {
                nextMinWeight = 1;
            }
            HUF_fillDTableX6LevelN(
                DDescription.add(start as usize),
                DSequence.add(start as usize),
                sizeLog - nbBits,
                rankValOrigin,
                totalBits as U32,
                nextMinWeight,
                maxWeight,
                sortedSymbols,
                sortedListSize,
                rankStart,
                nbBitsBaseline,
                baseSeq,
                DDesc,
            );
        } else {
            let mut i: U32;
            let end: U32 = start + length;
            i = start;
            while i < end {
                *DDescription.add(i as usize) = DDesc;
                *DSequence.add(i as usize) = baseSeq;
                i += 1;
            }
        }
        rankVal[weight as usize] += length;
        s += 1;
    }
}

unsafe fn HUF_readDTableX6(DTable: *mut U32, src: *const c_void, srcSize: usize) -> usize {
    let mut weightList: [BYTE; (HUF_MAX_SYMBOL_VALUE + 1) as usize] = [0; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUF_MAX_SYMBOL_VALUE + 1) as usize] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; (HUF_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize] = [0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0: [U32; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize] = [0; (HUF_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let mut sizeOfSort: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let mut rankVal: rankVal_t = [[0; (HUF_ABSOLUTEMAX_TABLELOG + 1) as usize]; HUF_ABSOLUTEMAX_TABLELOG as usize];
    let memLog: U32 = *DTable.add(0);
    let ip = src as *const BYTE;
    let mut iSize: usize = *ip.add(0) as usize;

    macro_rules! rankStart {
        ($i:expr) => {
            rankStart0[($i as usize) + 1]
        };
    }

    if memLog > HUF_ABSOLUTEMAX_TABLELOG {
        return ERROR(err::tableLog_tooLarge);
    }

    iSize = HUF_readStats(
        weightList.as_mut_ptr(),
        (HUF_MAX_SYMBOL_VALUE + 1) as usize,
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
        return ERROR(err::tableLog_tooLarge);
    }

    // find maxWeight
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        if maxW == 0 {
            return ERROR(err::GENERIC);
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
            rankStart!(w) = current;
            w += 1;
        }
        rankStart!(0) = nextRankStart;
        sizeOfSort = nextRankStart;
    }

    // sort symbols by weight
    {
        let mut s: U32;
        s = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = rankStart!(w);
            rankStart!(w) += 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        rankStart!(0) = 0;
    }

    // Build rankVal
    {
        let minBits: U32 = tableLog + 1 - maxW;
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: i32 = (memLog - tableLog) as i32 - 1;
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
                rankVal[consumed as usize][w as usize] = rankVal[0][w as usize] >> consumed;
                w += 1;
            }
            consumed += 1;
        }
    }

    // fill tables
    {
        let ptr = DTable.add(1) as *mut c_void;
        let DDescription = ptr as *mut HUF_DDescX6;
        let dSeqStart = DTable.add(1 + (1usize << (memLog - 1))) as *mut c_void;
        let DSequence = dSeqStart as *mut HUF_DSeqX6;
        let mut DSeq: HUF_DSeqX6 = HUF_DSeqX6 { sequence: 0 };
        let mut DDesc: HUF_DDescX6 = HUF_DDescX6 { nbBits: 0, nbBytes: 0 };
        DSeq.sequence = 0;
        DDesc.nbBits = 0;
        DDesc.nbBytes = 0;
        HUF_fillDTableX6LevelN(
            DDescription,
            DSequence,
            memLog as i32,
            &rankVal as *const rankVal_t,
            0,
            1,
            maxW,
            sortedSymbol.as_ptr(),
            sizeOfSort,
            rankStart0.as_ptr(),
            tableLog + 1,
            DSeq,
            DDesc,
        );
    }

    iSize
}

#[inline]
unsafe fn HUF_decodeSymbolX6(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dd: *const HUF_DDescX6,
    ds: *const HUF_DSeqX6,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog);
    memcpy(op, ds.add(val) as *const c_void, core::mem::size_of::<HUF_DSeqX6>());
    BIT_skipBits(DStream, (*dd.add(val)).nbBits as U32);
    (*dd.add(val)).nbBytes as U32
}

#[inline]
unsafe fn HUF_decodeLastSymbolsX6(
    op: *mut c_void,
    maxL: U32,
    DStream: *mut BIT_DStream_t,
    dd: *const HUF_DDescX6,
    ds: *const HUF_DSeqX6,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog);
    let length: U32 = (*dd.add(val)).nbBytes as U32;
    if length <= maxL {
        memcpy(op, ds.add(val) as *const c_void, length as usize);
        BIT_skipBits(DStream, (*dd.add(val)).nbBits as U32);
        return length;
    }
    memcpy(op, ds.add(val) as *const c_void, maxL as usize);
    if (*DStream).bitsConsumed < (core::mem::size_of::<usize>() * 8) as U32 {
        BIT_skipBits(DStream, (*dd.add(val)).nbBits as U32);
        if (*DStream).bitsConsumed > (core::mem::size_of::<usize>() * 8) as U32 {
            (*DStream).bitsConsumed = (core::mem::size_of::<usize>() * 8) as U32;
        }
    }
    maxL
}

unsafe fn HUF_decodeStreamX6(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    DTable: *const U32,
    dtLog: U32,
) -> usize {
    let ddPtr = DTable.add(1) as *const c_void;
    let dd = ddPtr as *const HUF_DDescX6;
    let dsPtr = DTable.add(1 + (1usize << (dtLog - 1))) as *const c_void;
    let ds = dsPtr as *const HUF_DSeqX6;
    let pStart = p;

    // up to 16 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.offset(-16)) {
        p = p.add(HUF_decodeSymbolX6(p as *mut c_void, bitDPtr, dd, ds, dtLog) as usize);
        p = p.add(HUF_decodeSymbolX6(p as *mut c_void, bitDPtr, dd, ds, dtLog) as usize);
        p = p.add(HUF_decodeSymbolX6(p as *mut c_void, bitDPtr, dd, ds, dtLog) as usize);
        p = p.add(HUF_decodeSymbolX6(p as *mut c_void, bitDPtr, dd, ds, dtLog) as usize);
    }

    // closer to the end, up to 4 symbols at a time
    while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) && (p <= pEnd.offset(-4)) {
        p = p.add(HUF_decodeSymbolX6(p as *mut c_void, bitDPtr, dd, ds, dtLog) as usize);
    }

    while p <= pEnd.offset(-4) {
        p = p.add(HUF_decodeSymbolX6(p as *mut c_void, bitDPtr, dd, ds, dtLog) as usize);
    }

    while p < pEnd {
        p = p.add(HUF_decodeLastSymbolsX6(
            p as *mut c_void,
            pEnd.offset_from(p) as U32,
            bitDPtr,
            dd,
            ds,
            dtLog,
        ) as usize);
    }

    p.offset_from(pStart) as usize
}

unsafe fn HUF_decompress4X6_usingDTable(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const U32,
) -> usize {
    if cSrcSize < 10 {
        return ERROR(err::corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);

        let dtLog: U32 = *DTable.add(0);
        let ddPtr = DTable.add(1) as *const c_void;
        let dd = ddPtr as *const HUF_DDescX6;
        let dsPtr = DTable.add(1 + (1usize << (dtLog - 1))) as *const c_void;
        let ds = dsPtr as *const HUF_DSeqX6;
        let mut errorCode: usize;

        let mut bitD1: BIT_DStream_t = core::mem::zeroed();
        let mut bitD2: BIT_DStream_t = core::mem::zeroed();
        let mut bitD3: BIT_DStream_t = core::mem::zeroed();
        let mut bitD4: BIT_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart) as usize;
        let length2 = MEM_readLE16(istart.add(2)) as usize;
        let length3 = MEM_readLE16(istart.add(4)) as usize;
        let mut length4: usize;
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
            return ERROR(err::corruption_detected);
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

        macro_rules! DEC {
            ($op:expr, $bd:expr) => {{
                $op = $op.add(HUF_decodeSymbolX6($op as *mut c_void, $bd, dd, ds, dtLog) as usize);
            }};
        }

        // 16-64 symbols per loop
        endSignal = BIT_reloadDStream(&mut bitD1)
            | BIT_reloadDStream(&mut bitD2)
            | BIT_reloadDStream(&mut bitD3)
            | BIT_reloadDStream(&mut bitD4);
        while (op3 <= opStart4) && (endSignal == BIT_DStream_unfinished) && (op4 <= oend.offset(-16)) {
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);
            DEC!(op1, &mut bitD1);
            DEC!(op2, &mut bitD2);
            DEC!(op3, &mut bitD3);
            DEC!(op4, &mut bitD4);

            endSignal = BIT_reloadDStream(&mut bitD1)
                | BIT_reloadDStream(&mut bitD2)
                | BIT_reloadDStream(&mut bitD3)
                | BIT_reloadDStream(&mut bitD4);
        }

        if op1 > opStart2 {
            return ERROR(err::corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(err::corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(err::corruption_detected);
        }

        HUF_decodeStreamX6(op1, &mut bitD1, opStart2, DTable, dtLog);
        HUF_decodeStreamX6(op2, &mut bitD2, opStart3, DTable, dtLog);
        HUF_decodeStreamX6(op3, &mut bitD3, opStart4, DTable, dtLog);
        HUF_decodeStreamX6(op4, &mut bitD4, oend, DTable, dtLog);

        endSignal = BIT_endOfDStream(&bitD1)
            & BIT_endOfDStream(&bitD2)
            & BIT_endOfDStream(&bitD3)
            & BIT_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(err::corruption_detected);
        }

        dstSize
    }
}

unsafe fn HUF_decompress4X6(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    let mut DTable: [U32; HUF_DTABLE_SIZE_X6_MAX] = [0; HUF_DTABLE_SIZE_X6_MAX];
    DTable[0] = HUF_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;
    let mut cSrcSize = cSrcSize;

    let hSize = HUF_readDTableX6(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(err::srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X6_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// -------- Generic decompression selector --------

#[repr(C)]
#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
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

type decompressionAlgo = unsafe fn(*mut c_void, usize, *const c_void, usize) -> usize;

unsafe fn HUF_decompress(dst: *mut c_void, dstSize: usize, cSrc: *const c_void, cSrcSize: usize) -> usize {
    static decompress: [decompressionAlgo; 3] = [HUF_decompress4X2, HUF_decompress4X4, HUF_decompress4X6];
    let Q: U32;
    let D256: U32 = (dstSize >> 8) as U32;
    let mut Dtime: [U32; 3] = [0; 3];
    let mut algoNb: U32 = 0;
    let mut n: i32;

    if dstSize == 0 {
        return ERROR(err::dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(err::corruption_detected);
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as i32, dstSize);
        return dstSize;
    }

    Q = (cSrcSize * 16 / dstSize) as U32;
    n = 0;
    while n < 3 {
        Dtime[n as usize] =
            algoTime[Q as usize][n as usize].tableTime + (algoTime[Q as usize][n as usize].decode256Time * D256);
        n += 1;
    }

    Dtime[1] += Dtime[1] >> 4;
    Dtime[2] += Dtime[2] >> 3;

    if Dtime[1] < Dtime[0] {
        algoNb = 1;
    }
    if Dtime[2] < Dtime[algoNb as usize] {
        algoNb = 2;
    }

    decompress[algoNb as usize](dst, dstSize, cSrc, cSrcSize)
}

// ------------------------------------------------------------------
// ZSTD decompression
// ------------------------------------------------------------------
const ZSTD_MEMORY_USAGE: u32 = 17;
const HASH_LOG: u32 = ZSTD_MEMORY_USAGE - 2;
const HASH_TABLESIZE: u32 = 1u32 << HASH_LOG;
const HASH_MASK: u32 = HASH_TABLESIZE - 1;

const BLOCKSIZE: usize = 128 * (1 << 10);
const MIN_SEQUENCES_SIZE: usize = 2 + 2 + 3 + 1;
const MIN_CBLOCK_SIZE: usize = 3 + MIN_SEQUENCES_SIZE;
const IS_RAW: u32 = 1; // BIT0
const IS_RLE: u32 = 2; // BIT1

const MINMATCH: usize = 4;
const MLbits: u32 = 7;
const LLbits: u32 = 6;
const Offbits: u32 = 5;
const MaxML: u32 = (1 << MLbits) - 1;
const MaxLL: u32 = (1 << LLbits) - 1;
const MaxOff: u32 = 31;
const LitFSELog: u32 = 11;
const MLFSELog: u32 = 10;
const LLFSELog: u32 = 10;
const OffFSELog: u32 = 9;

const LITERAL_NOENTROPY: u32 = 63;
const COMMAND_NOENTROPY: u32 = 7;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

const ZSTD_magicNumber: U32 = 0xFD2FB522;

static ZSTD_blockHeaderSize: usize = 3;
static ZSTD_frameHeaderSize: usize = 4;

// FSE_DTABLE_SIZE_U32(maxTableLog) = 1 + (1<<maxTableLog)
const LL_DTABLE_SIZE_U32: usize = (1 + (1u32 << LLFSELog)) as usize;
const Off_DTABLE_SIZE_U32: usize = (1 + (1u32 << OffFSELog)) as usize;
const ML_DTABLE_SIZE_U32: usize = (1 + (1u32 << MLFSELog)) as usize;

#[inline]
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}
#[inline]
unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

unsafe fn ZSTD_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.offset(length);
    loop {
        ZSTD_copy8(op as *mut c_void, ip as *const c_void);
        op = op.add(8);
        ip = ip.add(8);
        if op >= oend {
            break;
        }
    }
}

const bt_compressed: u32 = 0;
const bt_raw: u32 = 1;
const bt_rle: u32 = 2;
const bt_end: u32 = 3;
type blockType_t = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

#[inline]
fn ZSTD_isError(code: usize) -> u32 {
    ERR_isError(code)
}

#[repr(C)]
pub struct ZSTDv02_Dctx_s {
    LLTable: [U32; LL_DTABLE_SIZE_U32],
    OffTable: [U32; Off_DTABLE_SIZE_U32],
    MLTable: [U32; ML_DTABLE_SIZE_U32],
    previousDstEnd: *mut c_void,
    base: *mut c_void,
    expected: usize,
    bType: blockType_t,
    phase: U32,
    litPtr: *const BYTE,
    litSize: usize,
    litBuffer: [BYTE; BLOCKSIZE + 8],
}

type ZSTD_DCtx = ZSTDv02_Dctx_s;

unsafe fn ZSTD_getcBlockSize(src: *const c_void, srcSize: usize, bpPtr: *mut blockProperties_t) -> usize {
    let inp = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR(err::srcSize_wrong);
    }

    headerFlags = *inp;
    cSize = *inp.add(2) as U32 + ((*inp.add(1) as U32) << 8) + (((*inp.add(0) as U32) & 7) << 16);

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

unsafe fn ZSTD_copyUncompressedBlock(dst: *mut c_void, maxDstSize: usize, src: *const c_void, srcSize: usize) -> usize {
    if srcSize > maxDstSize {
        return ERROR(err::dstSize_tooSmall);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

unsafe fn ZSTD_decompressLiterals(
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip = src as *const BYTE;

    let litSize: usize = ((MEM_readLE32(src as *const BYTE) & 0x1FFFFF) >> 2) as usize;
    let litCSize: usize = ((MEM_readLE32(ip.add(2)) & 0xFFFFFF) >> 5) as usize;

    if litSize > *maxDstSizePtr {
        return ERROR(err::corruption_detected);
    }
    if litCSize + 5 > srcSize {
        return ERROR(err::corruption_detected);
    }

    if HUF_isError(HUF_decompress(dst, litSize, ip.add(5) as *const c_void, litCSize)) != 0 {
        return ERROR(err::corruption_detected);
    }

    *maxDstSizePtr = litSize;
    litCSize + 5
}

unsafe fn ZSTD_decodeLiteralsBlock(ctx: *mut c_void, src: *const c_void, srcSize: usize) -> usize {
    let dctx = ctx as *mut ZSTD_DCtx;
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(err::corruption_detected);
    }

    match *istart & 3 {
        x if x == IS_RAW as BYTE => {
            let litSize: usize = ((MEM_readLE32(istart) & 0xFFFFFF) >> 2) as usize;
            if litSize > srcSize - 11 {
                if litSize > BLOCKSIZE {
                    return ERROR(err::corruption_detected);
                }
                if litSize > srcSize - 3 {
                    return ERROR(err::corruption_detected);
                }
                memcpy((*dctx).litBuffer.as_mut_ptr() as *mut c_void, istart as *const c_void, litSize);
                (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
                (*dctx).litSize = litSize;
                memset((*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void, 0, 8);
                return litSize + 3;
            }
            (*dctx).litPtr = istart.add(3);
            (*dctx).litSize = litSize;
            litSize + 3
        }
        x if x == IS_RLE as BYTE => {
            let litSize: usize = ((MEM_readLE32(istart) & 0xFFFFFF) >> 2) as usize;
            if litSize > BLOCKSIZE {
                return ERROR(err::corruption_detected);
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
            // default / case 0
            let mut litSize: usize = BLOCKSIZE;
            let readSize = ZSTD_decompressLiterals((*dctx).litBuffer.as_mut_ptr() as *mut c_void, &mut litSize, src, srcSize);
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset((*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void, 0, 8);
            readSize
        }
    }
}

unsafe fn ZSTD_decodeSeqHeaders(
    nbSeq: *mut i32,
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
    let iend = istart.add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: U32 = 0;
    let mut Offlog: U32 = 0;
    let mut MLlog: U32 = 0;
    let mut dumpsLength: usize;

    if srcSize < 5 {
        return ERROR(err::srcSize_wrong);
    }

    *nbSeq = MEM_readLE16(ip) as i32;
    ip = ip.add(2);
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        dumpsLength = *ip.add(2) as usize;
        dumpsLength += (*ip.add(1) as usize) << 8;
        ip = ip.add(3);
    } else {
        dumpsLength = *ip.add(1) as usize;
        dumpsLength += ((*ip.add(0) as usize) & 1) << 8;
        ip = ip.add(2);
    }
    *dumpsPtr = ip;
    ip = ip.add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    if ip > iend.offset(-3) {
        return ERROR(err::srcSize_wrong);
    }

    {
        let mut norm: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut headerSize: usize;

        // LL
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
                headerSize = FSE_readNCount(norm.as_mut_ptr(), &mut max, &mut LLlog, ip as *const c_void, iend.offset_from(ip) as usize);
                if FSE_isError(headerSize) != 0 {
                    return ERROR(err::GENERIC);
                }
                if LLlog > LLFSELog {
                    return ERROR(err::corruption_detected);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        // Off
        match Offtype {
            x if x == bt_rle => {
                Offlog = 0;
                if ip > iend.offset(-2) {
                    return ERROR(err::srcSize_wrong);
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
                headerSize = FSE_readNCount(norm.as_mut_ptr(), &mut max, &mut Offlog, ip as *const c_void, iend.offset_from(ip) as usize);
                if FSE_isError(headerSize) != 0 {
                    return ERROR(err::GENERIC);
                }
                if Offlog > OffFSELog {
                    return ERROR(err::corruption_detected);
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
                    return ERROR(err::srcSize_wrong);
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
                headerSize = FSE_readNCount(norm.as_mut_ptr(), &mut max, &mut MLlog, ip as *const c_void, iend.offset_from(ip) as usize);
                if FSE_isError(headerSize) != 0 {
                    return ERROR(err::GENERIC);
                }
                if MLlog > MLFSELog {
                    return ERROR(err::corruption_detected);
                }
                ip = ip.add(headerSize);
                FSE_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
            }
        }
    }

    ip.offset_from(istart) as usize
}

#[repr(C)]
#[derive(Clone, Copy)]
struct seq_t {
    litLength: usize,
    offset: usize,
    matchLength: usize,
}

#[repr(C)]
struct seqState_t {
    DStream: BIT_DStream_t,
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

    // Literal length
    litLength = FSE_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream) as usize;
    prevOffset = if litLength != 0 { (*seq).offset } else { (*seqState).prevOffset };
    (*seqState).prevOffset = (*seq).offset;
    if litLength == MaxLL as usize {
        let add: U32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.add(1);
            v as U32
        } else {
            0
        };
        if add < 255 {
            litLength += add as usize;
        } else if dumps.add(3) <= de {
            litLength = MEM_readLE24(dumps) as usize;
            dumps = dumps.add(3);
        }
        if dumps >= de {
            dumps = de.offset(-1);
        }
    }

    // Offset
    {
        static offsetPrefix: [usize; (MaxOff + 1) as usize] = [
            1, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144,
            524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, 1, 1, 1, 1, 1,
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
        offset = offsetPrefix[offsetCode as usize] + BIT_readBits(&mut (*seqState).DStream, nbBits);
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset;
        }
    }

    // MatchLength
    matchLength = FSE_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as usize;
    if matchLength == MaxML as usize {
        let add: U32 = if dumps < de {
            let v = *dumps;
            dumps = dumps.add(1);
            v as U32
        } else {
            0
        };
        if add < 255 {
            matchLength += add as usize;
        } else if dumps.add(3) <= de {
            matchLength = MEM_readLE24(dumps) as usize;
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

unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> usize {
    static dec32table: [i32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
    static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
    let ostart = op;
    let oLitEnd = op.add(sequence.litLength);
    let oMatchEnd = op.add(sequence.litLength + sequence.matchLength);
    let oend_8 = oend.offset(-8);
    let litEnd = (*litPtr).add(sequence.litLength);

    let seqLength: usize = sequence.litLength + sequence.matchLength;

    if seqLength > oend.offset_from(op) as usize {
        return ERROR(err::dstSize_tooSmall);
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as usize {
        return ERROR(err::corruption_detected);
    }
    if oLitEnd > oend_8 {
        return ERROR(err::dstSize_tooSmall);
    }
    if sequence.offset > (oLitEnd.offset_from(base) as U32) as usize {
        return ERROR(err::corruption_detected);
    }

    if oMatchEnd > oend {
        return ERROR(err::dstSize_tooSmall);
    }
    if litEnd > litLimit {
        return ERROR(err::corruption_detected);
    }

    // copy Literals
    ZSTD_wildcopy(op as *mut c_void, *litPtr as *const c_void, sequence.litLength as isize);
    op = oLitEnd;
    *litPtr = litEnd;

    // copy Match
    {
        let mut match_: *const BYTE = op.offset(-(sequence.offset as isize));

        if sequence.offset > op as usize {
            return ERROR(err::corruption_detected);
        }
        if match_ < base as *const BYTE {
            return ERROR(err::corruption_detected);
        }

        if sequence.offset < 8 {
            let dec64 = dec64table[sequence.offset];
            *op.add(0) = *match_.add(0);
            *op.add(1) = *match_.add(1);
            *op.add(2) = *match_.add(2);
            *op.add(3) = *match_.add(3);
            match_ = match_.add(dec32table[sequence.offset] as usize);
            ZSTD_copy4(op.add(4) as *mut c_void, match_ as *const c_void);
            match_ = match_.offset(-(dec64 as isize));
        } else {
            ZSTD_copy8(op as *mut c_void, match_ as *const c_void);
        }
        op = op.add(8);
        match_ = match_.add(8);

        if oMatchEnd > oend.offset(-(16 - MINMATCH as isize)) {
            if op < oend_8 {
                ZSTD_wildcopy(op as *mut c_void, match_ as *const c_void, oend_8.offset_from(op));
                match_ = match_.add(oend_8.offset_from(op) as usize);
                op = oend_8;
            }
            while op < oMatchEnd {
                *op = *match_;
                op = op.add(1);
                match_ = match_.add(1);
            }
        } else {
            ZSTD_wildcopy(op as *mut c_void, match_ as *const c_void, sequence.matchLength as isize - 8);
        }
    }

    oMatchEnd.offset_from(ostart) as usize
}

unsafe fn ZSTD_decompressSequences(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
) -> usize {
    let dctx = ctx as *mut ZSTD_DCtx;
    let mut ip = seqStart as *const BYTE;
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut errorCode: usize;
    let mut dumpsLength: usize = 0;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd = litPtr.add((*dctx).litSize);
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
        iend.offset_from(ip) as usize,
    );
    if ZSTD_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.add(errorCode);

    // Regen sequences
    {
        let mut sequence: seq_t = core::mem::zeroed();
        let mut seqState: seqState_t = core::mem::zeroed();

        memset(&mut sequence as *mut seq_t as *mut c_void, 0, core::mem::size_of::<seq_t>());
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.add(dumpsLength);
        seqState.prevOffset = 1;
        errorCode = BIT_initDStream(&mut seqState.DStream, ip as *const c_void, iend.offset_from(ip) as usize);
        if ERR_isError(errorCode) != 0 {
            return ERROR(err::corruption_detected);
        }
        FSE_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSE_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSE_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BIT_reloadDStream(&mut seqState.DStream) <= BIT_DStream_completed) && (nbSeq > 0) {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTD_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTD_execSequence(op, sequence, &mut litPtr, litEnd, base, oend);
            if ZSTD_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
        }

        // check if reached exact end
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(err::corruption_detected);
        }
        if nbSeq < 0 {
            return ERROR(err::corruption_detected);
        }

        // last literal segment
        {
            let lastLLSize: usize = litEnd.offset_from(litPtr) as usize;
            if litPtr > litEnd {
                return ERROR(err::corruption_detected);
            }
            if op.add(lastLLSize) > oend {
                return ERROR(err::dstSize_tooSmall);
            }
            if lastLLSize > 0 {
                if op != litPtr as *mut BYTE {
                    memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
                }
                op = op.add(lastLLSize);
            }
        }
    }

    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_decompressBlock(
    ctx: *mut c_void,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    let mut ip = src as *const BYTE;

    let litCSize = ZSTD_decodeLiteralsBlock(ctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.add(litCSize);
    srcSize -= litCSize;

    ZSTD_decompressSequences(ctx, dst, maxDstSize, ip as *const c_void, srcSize)
}

unsafe fn ZSTD_decompressDCtx(ctx: *mut c_void, dst: *mut c_void, maxDstSize: usize, src: *const c_void, srcSize: usize) -> usize {
    let mut ip = src as *const BYTE;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.add(maxDstSize);
    let mut remainingSize = srcSize;
    let magicNumber: U32;
    let mut blockProperties: blockProperties_t = core::mem::zeroed();

    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        return ERROR(err::srcSize_wrong);
    }
    magicNumber = MEM_readLE32(src as *const BYTE);
    if magicNumber != ZSTD_magicNumber {
        return ERROR(err::prefix_unknown);
    }
    ip = ip.add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    loop {
        let mut decodedSize: usize = 0;
        let cBlockSize = ZSTD_getcBlockSize(ip as *const c_void, iend.offset_from(ip) as usize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(err::srcSize_wrong);
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                decodedSize = ZSTD_decompressBlock(ctx, op as *mut c_void, oend.offset_from(op) as usize, ip as *const c_void, cBlockSize);
            }
            x if x == bt_raw => {
                decodedSize = ZSTD_copyUncompressedBlock(op as *mut c_void, oend.offset_from(op) as usize, ip as *const c_void, cBlockSize);
            }
            x if x == bt_rle => {
                return ERROR(err::GENERIC);
            }
            x if x == bt_end => {
                if remainingSize != 0 {
                    return ERROR(err::srcSize_wrong);
                }
            }
            _ => {
                return ERROR(err::GENERIC);
            }
        }
        if cBlockSize == 0 {
            break;
        }

        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_decompress(dst: *mut c_void, maxDstSize: usize, src: *const c_void, srcSize: usize) -> usize {
    let mut ctx: ZSTD_DCtx = core::mem::zeroed();
    ctx.base = dst;
    ZSTD_decompressDCtx(&mut ctx as *mut ZSTD_DCtx as *mut c_void, dst, maxDstSize, src, srcSize)
}

unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut usize, dBound: *mut u64, ret: usize) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: usize = 0;
    let magicNumber: U32;
    let mut blockProperties: blockProperties_t = core::mem::zeroed();

    if srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(err::srcSize_wrong));
        return;
    }
    magicNumber = MEM_readLE32(src as *const BYTE);
    if magicNumber != ZSTD_magicNumber {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(err::prefix_unknown));
        return;
    }
    ip = ip.add(ZSTD_frameHeaderSize);
    remainingSize -= ZSTD_frameHeaderSize;

    loop {
        let cBlockSize = ZSTD_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTD_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.add(ZSTD_blockHeaderSize);
        remainingSize -= ZSTD_blockHeaderSize;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(err::srcSize_wrong));
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
    *dBound = (nbBlocks * BLOCKSIZE) as u64;
}

// -------- Streaming Decompression API --------

unsafe fn ZSTD_resetDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0;
    (*dctx).previousDstEnd = core::ptr::null_mut();
    (*dctx).base = core::ptr::null_mut();
    0
}

unsafe fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    let dctx = malloc(core::mem::size_of::<ZSTD_DCtx>()) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_resetDCtx(dctx);
    dctx
}

unsafe fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    free(dctx as *mut c_void);
    0
}

unsafe fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> usize {
    (*dctx).expected
}

unsafe fn ZSTD_decompressContinue(
    ctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize != (*ctx).expected {
        return ERROR(err::srcSize_wrong);
    }
    if dst != (*ctx).previousDstEnd {
        (*ctx).base = dst;
    }

    // Decompress : frame header
    if (*ctx).phase == 0 {
        let magicNumber: U32 = MEM_readLE32(src as *const BYTE);
        if magicNumber != ZSTD_magicNumber {
            return ERROR(err::prefix_unknown);
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        return 0;
    }

    // Decompress : block header
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

    // Decompress : block content
    {
        let rSize: usize;
        match (*ctx).bType {
            x if x == bt_compressed => {
                rSize = ZSTD_decompressBlock(ctx as *mut c_void, dst, maxDstSize, src, srcSize);
            }
            x if x == bt_raw => {
                rSize = ZSTD_copyUncompressedBlock(dst, maxDstSize, src, srcSize);
            }
            x if x == bt_rle => {
                return ERROR(err::GENERIC);
            }
            x if x == bt_end => {
                rSize = 0;
            }
            _ => {
                return ERROR(err::GENERIC);
            }
        }
        (*ctx).phase = 1;
        (*ctx).expected = ZSTD_blockHeaderSize;
        if ZSTD_isError(rSize) != 0 {
            return rSize;
        }
        (*ctx).previousDstEnd = (dst as *mut i8).add(rSize) as *mut c_void;
        rSize
    }
}

// -------- wrapper layer --------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_isError(code: usize) -> u32 {
    ZSTD_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_decompress(
    dst: *mut c_void,
    maxOriginalSize: usize,
    src: *const c_void,
    compressedSize: usize,
) -> usize {
    ZSTD_decompress(dst, maxOriginalSize, src, compressedSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_createDCtx() -> *mut ZSTDv02_Dctx_s {
    ZSTD_createDCtx() as *mut ZSTDv02_Dctx_s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_freeDCtx(dctx: *mut ZSTDv02_Dctx_s) -> usize {
    ZSTD_freeDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_resetDCtx(dctx: *mut ZSTDv02_Dctx_s) -> usize {
    ZSTD_resetDCtx(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_nextSrcSizeToDecompress(dctx: *mut ZSTDv02_Dctx_s) -> usize {
    ZSTD_nextSrcSizeToDecompress(dctx as *mut ZSTD_DCtx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv02_decompressContinue(
    dctx: *mut ZSTDv02_Dctx_s,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompressContinue(dctx as *mut ZSTD_DCtx, dst, maxDstSize, src, srcSize)
}

