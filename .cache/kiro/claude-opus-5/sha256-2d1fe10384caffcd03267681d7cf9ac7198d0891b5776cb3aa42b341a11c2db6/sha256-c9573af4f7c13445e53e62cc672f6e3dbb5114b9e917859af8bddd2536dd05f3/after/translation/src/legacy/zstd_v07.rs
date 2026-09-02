//! Literal Rust transliteration of `c_src/src/legacy/zstd_v07.c`.
//!
//! This file is a self-contained translation unit: it bundles its own
//! mem/endian helpers, error codes, bitstream reader, FSEv07 decoder, HUFv07
//! decoder, block/frame decoders, DCtx / DDict and the ZBUFFv07 streaming
//! state machine. Everything is translated literally; only the documented
//! `FSEv07_*` / `HUFv07_*` / `ZSTDv07_*` / `ZBUFFv07_*` entry points are
//! exported (`#[unsafe(no_mangle)]`).
//!
//! The XXH64 checksum path uses the modern common (namespaced `ZSTD_XXH64_*`)
//! implementation, exactly as the C build does (`XXH_NAMESPACE=ZSTD_`), so no
//! duplicate/colliding XXH symbols are introduced.

use core::ffi::{c_char, c_void};

use crate::common::xxhash::{
    XXH64_state_t, ZSTD_XXH64_digest as XXH64_digest, ZSTD_XXH64_reset as XXH64_reset,
    ZSTD_XXH64_update as XXH64_update,
};

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
    MEM_read16(memPtr)
}
#[inline]
pub unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    MEM_write16(memPtr, val)
}
#[inline]
pub unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    MEM_read32(memPtr)
}
#[inline]
pub unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    MEM_read64(memPtr)
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
// Error codes
//
// zstd_v07.c `#include "../common/error_private.h"` which defines the MODERN
// zstd error codes from zstd_errors.h. `ERROR(name)`, `ERR_isError` and
// `ERR_getErrorName` resolve to those codes/strings. We reproduce the numeric
// values and the string table exactly here to stay self-contained.
// ============================================================================
pub const ZSTD_error_no_error: U32 = 0;
pub const ZSTD_error_GENERIC: U32 = 1;
pub const ZSTD_error_prefix_unknown: U32 = 10;
pub const ZSTD_error_version_unsupported: U32 = 12;
pub const ZSTD_error_frameParameter_unsupported: U32 = 14;
pub const ZSTD_error_frameParameter_windowTooLarge: U32 = 16;
pub const ZSTD_error_corruption_detected: U32 = 20;
pub const ZSTD_error_checksum_wrong: U32 = 22;
pub const ZSTD_error_literals_headerWrong: U32 = 24;
pub const ZSTD_error_dictionary_corrupted: U32 = 30;
pub const ZSTD_error_dictionary_wrong: U32 = 32;
pub const ZSTD_error_dictionaryCreation_failed: U32 = 34;
pub const ZSTD_error_parameter_unsupported: U32 = 40;
pub const ZSTD_error_parameter_combination_unsupported: U32 = 41;
pub const ZSTD_error_parameter_outOfBound: U32 = 42;
pub const ZSTD_error_tableLog_tooLarge: U32 = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: U32 = 46;
pub const ZSTD_error_maxSymbolValue_tooSmall: U32 = 48;
pub const ZSTD_error_cannotProduce_uncompressedBlock: U32 = 49;
pub const ZSTD_error_stabilityCondition_notRespected: U32 = 50;
pub const ZSTD_error_stage_wrong: U32 = 60;
pub const ZSTD_error_init_missing: U32 = 62;
pub const ZSTD_error_memory_allocation: U32 = 64;
pub const ZSTD_error_workSpace_tooSmall: U32 = 66;
pub const ZSTD_error_dstSize_tooSmall: U32 = 70;
pub const ZSTD_error_srcSize_wrong: U32 = 72;
pub const ZSTD_error_dstBuffer_null: U32 = 74;
pub const ZSTD_error_noForwardProgress_destFull: U32 = 80;
pub const ZSTD_error_noForwardProgress_inputEmpty: U32 = 82;
pub const ZSTD_error_frameIndex_tooLarge: U32 = 100;
pub const ZSTD_error_seekableIO: U32 = 102;
pub const ZSTD_error_dstBuffer_wrong: U32 = 104;
pub const ZSTD_error_srcBuffer_wrong: U32 = 105;
pub const ZSTD_error_sequenceProducer_failed: U32 = 106;
pub const ZSTD_error_externalSequences_invalid: U32 = 107;
pub const ZSTD_error_maxCode: U32 = 120;

// #define ERROR(name) ((size_t)-PREFIX(name))
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
pub unsafe fn ERR_getErrorCode(code: size_t) -> U32 {
    if ERR_isError(code) == 0 {
        return 0;
    }
    (0usize.wrapping_sub(code)) as U32
}

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

pub unsafe fn ERR_getErrorString(code: U32) -> *const c_char {
    let notErrorCode = cstr!("Unspecified error code");
    match code {
        ZSTD_error_no_error => cstr!("No error detected"),
        ZSTD_error_GENERIC => cstr!("Error (generic)"),
        ZSTD_error_prefix_unknown => cstr!("Unknown frame descriptor"),
        ZSTD_error_version_unsupported => cstr!("Version not supported"),
        ZSTD_error_frameParameter_unsupported => cstr!("Unsupported frame parameter"),
        ZSTD_error_frameParameter_windowTooLarge => {
            cstr!("Frame requires too much memory for decoding")
        }
        ZSTD_error_corruption_detected => cstr!("Data corruption detected"),
        ZSTD_error_checksum_wrong => cstr!("Restored data doesn't match checksum"),
        ZSTD_error_literals_headerWrong => {
            cstr!("Header of Literals' block doesn't respect format specification")
        }
        ZSTD_error_parameter_unsupported => cstr!("Unsupported parameter"),
        ZSTD_error_parameter_combination_unsupported => {
            cstr!("Unsupported combination of parameters")
        }
        ZSTD_error_parameter_outOfBound => cstr!("Parameter is out of bound"),
        ZSTD_error_init_missing => cstr!("Context should be init first"),
        ZSTD_error_memory_allocation => cstr!("Allocation error : not enough memory"),
        ZSTD_error_workSpace_tooSmall => cstr!("workSpace buffer is not large enough"),
        ZSTD_error_stage_wrong => cstr!("Operation not authorized at current processing stage"),
        ZSTD_error_tableLog_tooLarge => cstr!("tableLog requires too much memory : unsupported"),
        ZSTD_error_maxSymbolValue_tooLarge => cstr!("Unsupported max Symbol Value : too large"),
        ZSTD_error_maxSymbolValue_tooSmall => cstr!("Specified maxSymbolValue is too small"),
        ZSTD_error_cannotProduce_uncompressedBlock => {
            cstr!("This mode cannot generate an uncompressed block")
        }
        ZSTD_error_stabilityCondition_notRespected => {
            cstr!("pledged buffer stability condition is not respected")
        }
        ZSTD_error_dictionary_corrupted => cstr!("Dictionary is corrupted"),
        ZSTD_error_dictionary_wrong => cstr!("Dictionary mismatch"),
        ZSTD_error_dictionaryCreation_failed => {
            cstr!("Cannot create Dictionary from provided samples")
        }
        ZSTD_error_dstSize_tooSmall => cstr!("Destination buffer is too small"),
        ZSTD_error_srcSize_wrong => cstr!("Src size is incorrect"),
        ZSTD_error_dstBuffer_null => cstr!("Operation on NULL destination buffer"),
        ZSTD_error_noForwardProgress_destFull => {
            cstr!("Operation made no progress over multiple calls, due to output buffer being full")
        }
        ZSTD_error_noForwardProgress_inputEmpty => {
            cstr!("Operation made no progress over multiple calls, due to input being empty")
        }
        ZSTD_error_frameIndex_tooLarge => cstr!("Frame index is too large"),
        ZSTD_error_seekableIO => cstr!("An I/O error occurred when reading/seeking"),
        ZSTD_error_dstBuffer_wrong => cstr!("Destination buffer is wrong"),
        ZSTD_error_srcBuffer_wrong => cstr!("Source buffer is wrong"),
        ZSTD_error_sequenceProducer_failed => {
            cstr!("Block-level external sequence producer returned an error code")
        }
        ZSTD_error_externalSequences_invalid => cstr!("External sequences are not valid"),
        _ => notErrorCode,
    }
}

#[inline]
pub unsafe fn ERR_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorString(ERR_getErrorCode(code))
}

// ============================================================================
// bitstream (read backward)
// ============================================================================
#[repr(C)]
pub struct BITv07_DStream_t {
    pub bitContainer: size_t,
    pub bitsConsumed: u32,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

pub type BITv07_DStream_status = u32;
pub const BITv07_DStream_unfinished: BITv07_DStream_status = 0;
pub const BITv07_DStream_endOfBuffer: BITv07_DStream_status = 1;
pub const BITv07_DStream_completed: BITv07_DStream_status = 2;
pub const BITv07_DStream_overflow: BITv07_DStream_status = 3;

#[inline]
pub unsafe fn BITv07_highbit32(val: U32) -> u32 {
    // __builtin_clz(val) ^ 31   (val != 0 assumed by callers)
    31u32 ^ val.leading_zeros()
}

pub unsafe fn BITv07_initDStream(
    bitD: *mut BITv07_DStream_t,
    srcBuffer: *const c_void,
    srcSize: size_t,
) -> size_t {
    if srcSize < 1 {
        memset(
            bitD as *mut c_void,
            0,
            core::mem::size_of::<BITv07_DStream_t>() as size_t,
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    let bc_size = core::mem::size_of::<size_t>();

    if srcSize >= bc_size {
        // normal case
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char).add(srcSize - bc_size);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        let lastByte = *(srcBuffer as *const BYTE).add(srcSize - 1);
        (*bitD).bitsConsumed = if lastByte != 0 {
            8u32.wrapping_sub(BITv07_highbit32(lastByte as U32))
        } else {
            0
        };
        if lastByte == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as size_t;
        // switch(srcSize) with fall-through
        let sb = srcBuffer as *const BYTE;
        let cb_bits = bc_size * 8;
        if srcSize >= 7 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(6) as size_t) << (cb_bits - 16));
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(5) as size_t) << (cb_bits - 24));
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.add(4) as size_t) << (cb_bits - 32));
        }
        if srcSize >= 4 {
            (*bitD).bitContainer =
                (*bitD).bitContainer.wrapping_add((*sb.add(3) as size_t) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer =
                (*bitD).bitContainer.wrapping_add((*sb.add(2) as size_t) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer =
                (*bitD).bitContainer.wrapping_add((*sb.add(1) as size_t) << 8);
        }
        let lastByte = *(srcBuffer as *const BYTE).add(srcSize - 1);
        (*bitD).bitsConsumed = if lastByte != 0 {
            8u32.wrapping_sub(BITv07_highbit32(lastByte as U32))
        } else {
            0
        };
        if lastByte == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
        (*bitD).bitsConsumed = (*bitD)
            .bitsConsumed
            .wrapping_add(((bc_size - srcSize) * 8) as u32);
    }

    srcSize
}

#[inline]
pub unsafe fn BITv07_lookBits(bitD: *const BITv07_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask.wrapping_sub(nbBits)) & bitMask)
}

#[inline]
pub unsafe fn BITv07_lookBitsFast(bitD: *const BITv07_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1).wrapping_sub(nbBits)) & bitMask)
}

#[inline]
pub unsafe fn BITv07_skipBits(bitD: *mut BITv07_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

#[inline]
pub unsafe fn BITv07_readBits(bitD: *mut BITv07_DStream_t, nbBits: U32) -> size_t {
    let value = BITv07_lookBits(bitD, nbBits);
    BITv07_skipBits(bitD, nbBits);
    value
}

#[inline]
pub unsafe fn BITv07_readBitsFast(bitD: *mut BITv07_DStream_t, nbBits: U32) -> size_t {
    let value = BITv07_lookBitsFast(bitD, nbBits);
    BITv07_skipBits(bitD, nbBits);
    value
}

pub unsafe fn BITv07_reloadDStream(bitD: *mut BITv07_DStream_t) -> BITv07_DStream_status {
    let bc_size = core::mem::size_of::<size_t>();
    if (*bitD).bitsConsumed > (bc_size * 8) as u32 {
        return BITv07_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.add(bc_size) {
        (*bitD).ptr = (*bitD).ptr.offset(-(((*bitD).bitsConsumed >> 3) as isize));
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BITv07_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (bc_size * 8) as u32 {
            return BITv07_DStream_endOfBuffer;
        }
        return BITv07_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv07_DStream_status = BITv07_DStream_unfinished;
        if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32;
            result = BITv07_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes * 8);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline]
pub unsafe fn BITv07_endOfDStream(dStream: *const BITv07_DStream_t) -> u32 {
    (((*dStream).ptr == (*dStream).start)
        && ((*dStream).bitsConsumed == (core::mem::size_of::<size_t>() * 8) as u32)) as u32
}

// ============================================================================
// FSEv07
// ============================================================================
pub type FSEv07_DTable = u32;

pub const FSEv07_MAX_MEMORY_USAGE: u32 = 14;
pub const FSEv07_MAX_SYMBOL_VALUE: u32 = 255;
pub const FSEv07_MAX_TABLELOG: u32 = FSEv07_MAX_MEMORY_USAGE - 2; // 12
pub const FSEv07_MIN_TABLELOG: u32 = 5;
pub const FSEv07_TABLELOG_ABSOLUTE_MAX: u32 = 15;

#[inline]
fn FSEv07_TABLESTEP(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

pub const FSEv07_DTABLE_SIZE_U32_MAX: usize = (1 + (1usize << FSEv07_MAX_TABLELOG));

#[repr(C)]
pub struct FSEv07_DState_t {
    pub state: size_t,
    pub table: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv07_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv07_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
}

#[inline]
pub unsafe fn FSEv07_initDState(
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

#[inline]
pub unsafe fn FSEv07_peekSymbol(DStatePtr: *const FSEv07_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

#[inline]
pub unsafe fn FSEv07_updateState(DStatePtr: *mut FSEv07_DState_t, bitD: *mut BITv07_DStream_t) {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let lowBits = BITv07_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
}

#[inline]
pub unsafe fn FSEv07_decodeSymbol(DStatePtr: *mut FSEv07_DState_t, bitD: *mut BITv07_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BITv07_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

#[inline]
pub unsafe fn FSEv07_decodeSymbolFast(
    DStatePtr: *mut FSEv07_DState_t,
    bitD: *mut BITv07_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv07_decode_t).add((*DStatePtr).state);
    let nbBits = DInfo.nbBits as U32;
    let symbol = DInfo.symbol;
    let lowBits = BITv07_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

// ---- FSE / HUF error management (exported) ---------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_isError(code: size_t) -> u32 {
    ERR_isError(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_isError(code: size_t) -> u32 {
    ERR_isError(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

// ---- FSE NCount reader -----------------------------------------------------
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
    nbBits = ((bitStream & 0xF) + FSEv07_MIN_TABLELOG) as i32; // extract tableLog
    if nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX as i32 {
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
            let max: i16 = ((2 * threshold - 1) - remaining) as i16;
            let count: i16;

            if (bitStream & (threshold as U32 - 1)) < (max as U32) {
                count = (bitStream & (threshold as U32 - 1)) as i16;
                bitCount += nbBits - 1;
            } else {
                let mut c = (bitStream & (2 * threshold as U32 - 1)) as i16;
                if c >= threshold as i16 {
                    c -= max;
                }
                count = c;
                bitCount += nbBits;
            }

            let count = count - 1; // extra accuracy
            remaining -= FSEv07_abs(count) as i32;
            *normalizedCounter.add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as i32;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

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

// ---- HUF readStats (uses FSEv07_decompress) --------------------------------
pub const HUFv07_TABLELOG_ABSOLUTEMAX: u32 = 16;
pub const HUFv07_TABLELOG_MAX: u32 = 12;
pub const HUFv07_SYMBOLVALUE_MAX: u32 = 255;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readStats(
    huffWeight: *mut BYTE,
    hwSize: size_t,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut weightTotal: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: size_t;
    let oSize: size_t;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.add(0) as size_t;

    if iSize >= 128 {
        // special header
        if iSize >= 242 {
            // RLE
            static L: [U32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = L[iSize - 242] as size_t;
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
            {
                let mut n: U32 = 0;
                while (n as size_t) < oSize {
                    *huffWeight.add(n as usize) = *ip.add((n / 2) as usize) >> 4;
                    *huffWeight.add((n + 1) as usize) = *ip.add((n / 2) as usize) & 15;
                    n += 2;
                }
            }
        }
    } else {
        // header compressed with FSE (normal case)
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

    // collect weight stats
    memset(
        rankStats as *mut c_void,
        0,
        ((HUFv07_TABLELOG_ABSOLUTEMAX + 1) as size_t) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    {
        let mut n: U32 = 0;
        while (n as size_t) < oSize {
            let w = *huffWeight.add(n as usize);
            if (w as u32) >= HUFv07_TABLELOG_ABSOLUTEMAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *rankStats.add(w as usize) += 1;
            weightTotal = weightTotal.wrapping_add((1u32 << w) >> 1);
            n += 1;
        }
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    // get last non-null symbol weight (implied, total must be 2^n)
    {
        let tableLog = BITv07_highbit32(weightTotal) + 1;
        if tableLog > HUFv07_TABLELOG_ABSOLUTEMAX {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *tableLogPtr = tableLog;
        {
            let total = 1u32 << tableLog;
            let rest = total - weightTotal;
            let verif = 1u32 << BITv07_highbit32(rest);
            let lastWeight = BITv07_highbit32(rest) + 1;
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *huffWeight.add(oSize) = lastWeight as BYTE;
            *rankStats.add(lastWeight as usize) += 1;
        }
    }

    // check tree construction validity
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    // results
    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

// ---- FSE decode tables & decompress ----------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_createDTable(mut tableLog: u32) -> *mut FSEv07_DTable {
    if tableLog > FSEv07_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv07_TABLELOG_ABSOLUTE_MAX;
    }
    malloc(((1 + (1usize << tableLog)) * core::mem::size_of::<U32>()) as size_t)
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
    maxSymbolValue: u32,
    tableLog: u32,
) -> size_t {
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSEv07_decode_t;
    let mut symbolNext: [U16; (FSEv07_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSEv07_MAX_SYMBOL_VALUE + 1) as usize];

    let maxSV1 = maxSymbolValue + 1;
    let tableSize: U32 = 1 << tableLog;
    let mut highThreshold = tableSize - 1;

    if maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSEv07_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    // Init, lay down lowprob symbols
    {
        let mut DTableH = FSEv07_DTableHeader {
            tableLog: tableLog as U16,
            fastMode: 1,
        };
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
        memcpy(
            dt as *mut c_void,
            &DTableH as *const FSEv07_DTableHeader as *const c_void,
            core::mem::size_of::<FSEv07_DTableHeader>() as size_t,
        );
    }

    // Spread symbols
    {
        let tableMask = tableSize - 1;
        let step = FSEv07_TABLESTEP(tableSize);
        let mut position: U32 = 0;
        let mut s: U32 = 0;
        while s < maxSV1 {
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
            return ERROR(ZSTD_error_GENERIC);
        }
    }

    // Build Decoding table
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol = (*tableDecode.add(u as usize)).symbol;
            let nextState = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.add(u as usize)).nbBits =
                (tableLog - BITv07_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.add(u as usize)).newState =
                (((nextState as U32) << (*tableDecode.add(u as usize)).nbBits)
                    .wrapping_sub(tableSize)) as U16;
            u += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable_rle(dt: *mut FSEv07_DTable, symbolValue: BYTE) -> size_t {
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
pub unsafe extern "C" fn FSEv07_buildDTable_raw(dt: *mut FSEv07_DTable, nbBits: u32) -> size_t {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv07_DTableHeader;
    let dPtr = dt.add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv07_decode_t;
    let tableSize: u32 = 1 << nbBits;
    let tableMask = tableSize - 1;
    let maxSV1 = tableMask + 1;

    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC);
    }

    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    let mut s: u32 = 0;
    while s < maxSV1 {
        (*dinfo.add(s as usize)).newState = 0;
        (*dinfo.add(s as usize)).symbol = s as BYTE;
        (*dinfo.add(s as usize)).nbBits = nbBits as BYTE;
        s += 1;
    }

    0
}

#[inline]
unsafe fn FSEv07_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSEv07_DTable,
    fast: u32,
) -> size_t {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.add(maxDstSize);
    let olimit = omax.offset(-3);

    let mut bitD: BITv07_DStream_t = core::mem::zeroed();
    let mut state1: FSEv07_DState_t = core::mem::zeroed();
    let mut state2: FSEv07_DState_t = core::mem::zeroed();

    {
        let errorCode = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if FSEv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv07_initDState(&mut state1, &mut bitD, dt);
    FSEv07_initDState(&mut state2, &mut bitD, dt);

    macro_rules! GETSYMBOL {
        ($sp:expr) => {
            if fast != 0 {
                FSEv07_decodeSymbolFast($sp, &mut bitD)
            } else {
                FSEv07_decodeSymbol($sp, &mut bitD)
            }
        };
    }

    // 4 symbols per loop
    let bc_bits = (core::mem::size_of::<size_t>() * 8) as u32;
    while (BITv07_reloadDStream(&mut bitD) == BITv07_DStream_unfinished) && (op < olimit) {
        *op.add(0) = GETSYMBOL!(&mut state1);

        if FSEv07_MAX_TABLELOG * 2 + 7 > bc_bits {
            BITv07_reloadDStream(&mut bitD);
        }

        *op.add(1) = GETSYMBOL!(&mut state2);

        if FSEv07_MAX_TABLELOG * 4 + 7 > bc_bits {
            if BITv07_reloadDStream(&mut bitD) > BITv07_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = GETSYMBOL!(&mut state1);

        if FSEv07_MAX_TABLELOG * 2 + 7 > bc_bits {
            BITv07_reloadDStream(&mut bitD);
        }

        *op.add(3) = GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    // tail
    loop {
        if op > omax.offset(-2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = GETSYMBOL!(&mut state1);
        op = op.add(1);
        if BITv07_reloadDStream(&mut bitD) == BITv07_DStream_overflow {
            *op = GETSYMBOL!(&mut state2);
            op = op.add(1);
            break;
        }

        if op > omax.offset(-2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = GETSYMBOL!(&mut state2);
        op = op.add(1);
        if BITv07_reloadDStream(&mut bitD) == BITv07_DStream_overflow {
            *op = GETSYMBOL!(&mut state1);
            op = op.add(1);
            break;
        }
    }

    op.offset_from(ostart) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSEv07_DTable,
) -> size_t {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv07_DTableHeader;
    let fastMode = (*DTableH).fastMode as U32;

    if fastMode != 0 {
        return FSEv07_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSEv07_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_decompress(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting: [i16; (FSEv07_MAX_SYMBOL_VALUE + 1) as usize] =
        [0; (FSEv07_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: [U32; FSEv07_DTABLE_SIZE_U32_MAX] = [0; FSEv07_DTABLE_SIZE_U32_MAX];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSEv07_MAX_SYMBOL_VALUE;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong);
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

// ============================================================================
// HUFv07
// ============================================================================
pub type HUFv07_DTable = U32;

#[inline]
fn HUFv07_DTABLE_SIZE(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DTableDesc {
    pub maxTableLog: BYTE,
    pub tableType: BYTE,
    pub tableLog: BYTE,
    pub reserved: BYTE,
}

#[inline]
unsafe fn HUFv07_getDTableDesc(table: *const HUFv07_DTable) -> DTableDesc {
    let mut dtd: DTableDesc = core::mem::zeroed();
    memcpy(
        &mut dtd as *mut DTableDesc as *mut c_void,
        table as *const c_void,
        core::mem::size_of::<DTableDesc>() as size_t,
    );
    dtd
}

// ---- single-symbol decoding (X2) -------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUFv07_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX2(
    DTable: *mut HUFv07_DTable,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut huffWeight: [BYTE; (HUFv07_SYMBOLVALUE_MAX + 1) as usize] =
        [0; (HUFv07_SYMBOLVALUE_MAX + 1) as usize];
    let mut rankVal: [U32; (HUFv07_TABLELOG_ABSOLUTEMAX + 1) as usize] =
        [0; (HUFv07_TABLELOG_ABSOLUTEMAX + 1) as usize];
    let mut tableLog: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let iSize: size_t;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv07_DEltX2;

    iSize = HUFv07_readStats(
        huffWeight.as_mut_ptr(),
        (HUFv07_SYMBOLVALUE_MAX + 1) as size_t,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }

    // Table header
    {
        let mut dtd = HUFv07_getDTableDesc(DTable);
        if tableLog > (dtd.maxTableLog as U32 + 1) {
            return ERROR(ZSTD_error_tableLog_tooLarge);
        }
        dtd.tableType = 0;
        dtd.tableLog = tableLog as BYTE;
        memcpy(
            DTable as *mut c_void,
            &dtd as *const DTableDesc as *const c_void,
            core::mem::size_of::<DTableDesc>() as size_t,
        );
    }

    // Prepare ranks
    {
        let mut nextRankStart: U32 = 0;
        let mut n: U32 = 1;
        while n < tableLog + 1 {
            let current = nextRankStart;
            nextRankStart += rankVal[n as usize] << (n - 1);
            rankVal[n as usize] = current;
            n += 1;
        }
    }

    // fill DTable
    {
        let mut n: U32 = 0;
        while n < nbSymbols {
            let w = huffWeight[n as usize] as U32;
            let length = (1u32 << w) >> 1;
            let mut i: U32;
            let mut D = HUFv07_DEltX2 {
                byte: n as BYTE,
                nbBits: (tableLog + 1 - w) as BYTE,
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
    dtLog: U32,
) -> BYTE {
    let val = BITv07_lookBitsFast(Dstream, dtLog);
    let c = (*dt.add(val)).byte;
    BITv07_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

macro_rules! HUF_DECODE_SYMBOLX2_0 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUFv07_decodeSymbolX2($ds, $dt, $dtLog);
        $ptr = $ptr.add(1);
    }};
}
macro_rules! HUF_DECODE_SYMBOLX2_1 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUFv07_TABLELOG_MAX <= 12) {
            HUF_DECODE_SYMBOLX2_0!($ptr, $ds, $dt, $dtLog);
        }
    }};
}
macro_rules! HUF_DECODE_SYMBOLX2_2 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            HUF_DECODE_SYMBOLX2_0!($ptr, $ds, $dt, $dtLog);
        }
    }};
}

#[inline]
unsafe fn HUFv07_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart = p;

    // up to 4 symbols at a time
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished) && (p <= pEnd.offset(-4)) {
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    // closer to the end
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished) && (p < pEnd) {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    // no more data to retrieve from bitstream, hence no need to reload
    while p < pEnd {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    pEnd.offset_from(pStart) as size_t
}

unsafe fn HUFv07_decompress1X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
    let op = dst as *mut BYTE;
    let oend = op.add(dstSize);
    let dtPtr = DTable.add(1) as *const c_void;
    let dt = dtPtr as *const HUFv07_DEltX2;
    let mut bitD: BITv07_DStream_t = core::mem::zeroed();
    let dtd = HUFv07_getDTableDesc(DTable);
    let dtLog = dtd.tableLog as U32;

    {
        let errorCode = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv07_decodeStreamX2(op, &mut bitD, oend, dt, dtLog);

    if BITv07_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X2_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    // HUFv07_CREATE_STATIC_DTABLEX2(DTable, HUFv07_TABLELOG_MAX)
    let mut DTable: [HUFv07_DTable; 1 + (1 << (HUFv07_TABLELOG_MAX - 1))] =
        [0; 1 + (1 << (HUFv07_TABLELOG_MAX - 1))];
    DTable[0] = (HUFv07_TABLELOG_MAX - 1).wrapping_mul(0x1000001);
    HUFv07_decompress1X2_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

unsafe fn HUFv07_decompress4X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX2;

        let mut bitD1: BITv07_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv07_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv07_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv07_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart as *const c_void) as size_t;
        let length2 = MEM_readLE16(istart.add(2) as *const c_void) as size_t;
        let length3 = MEM_readLE16(istart.add(4) as *const c_void) as size_t;
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
        let mut endSignal: U32;
        let dtd = HUFv07_getDTableDesc(DTable);
        let dtLog = dtd.tableLog as U32;

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

        endSignal = BITv07_reloadDStream(&mut bitD1)
            | BITv07_reloadDStream(&mut bitD2)
            | BITv07_reloadDStream(&mut bitD3)
            | BITv07_reloadDStream(&mut bitD4);
        while (endSignal == BITv07_DStream_unfinished) && (op4 < oend.offset(-7)) {
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
            endSignal = BITv07_reloadDStream(&mut bitD1)
                | BITv07_reloadDStream(&mut bitD2)
                | BITv07_reloadDStream(&mut bitD3)
                | BITv07_reloadDStream(&mut bitD4);
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

        HUFv07_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv07_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv07_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv07_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        endSignal = BITv07_endOfDStream(&bitD1)
            & BITv07_endOfDStream(&bitD2)
            & BITv07_endOfDStream(&bitD3)
            & BITv07_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        dstSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    let mut DTable: [HUFv07_DTable; 1 + (1 << (HUFv07_TABLELOG_MAX - 1))] =
        [0; 1 + (1 << (HUFv07_TABLELOG_MAX - 1))];
    DTable[0] = (HUFv07_TABLELOG_MAX - 1).wrapping_mul(0x1000001);
    HUFv07_decompress4X2_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

// ---- double-symbols decoding (X4) ------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUFv07_DEltX4 {
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

// rankVal_t == U32[ABSMAX][ABSMAX+1]
const RANKVAL_ROWS: usize = HUFv07_TABLELOG_ABSOLUTEMAX as usize;
const RANKVAL_COLS: usize = (HUFv07_TABLELOG_ABSOLUTEMAX + 1) as usize;

unsafe fn HUFv07_fillDTableX4Level2(
    DTable: *mut HUFv07_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: i32,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    let mut DElt: HUFv07_DEltX4 = core::mem::zeroed();
    let mut rankVal: [U32; RANKVAL_COLS] = [0; RANKVAL_COLS];

    // get pre-calculated rankVal
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; RANKVAL_COLS]>() as size_t,
    );

    // fill skipped values
    if minWeight > 1 {
        let skipSize = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1;
        let mut i: U32 = 0;
        while i < skipSize {
            *DTable.add(i as usize) = DElt;
            i += 1;
        }
    }

    // fill DTable
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
                if i >= end {
                    break;
                }
            }

            rankVal[weight as usize] += length;
            s += 1;
        }
    }
}

unsafe fn HUFv07_fillDTableX4(
    DTable: *mut HUFv07_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const [U32; RANKVAL_COLS],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; RANKVAL_COLS] = [0; RANKVAL_COLS];
    let scaleLog = nbBitsBaseline as i32 - targetLog as i32;
    let minBits = nbBitsBaseline - maxWeight;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[U32; RANKVAL_COLS]>() as size_t,
    );

    let mut s: U32 = 0;
    while s < sortedListSize {
        let symbol = (*sortedList.add(s as usize)).symbol as U16;
        let weight = (*sortedList.add(s as usize)).weight as U32;
        let nbBits = nbBitsBaseline - weight;
        let start = rankVal[weight as usize];
        let length = 1u32 << (targetLog - nbBits);

        if targetLog - nbBits >= minBits {
            let sortedRank;
            let mut minWeight = nbBits as i32 + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.add(minWeight as usize);
            HUFv07_fillDTableX4Level2(
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
            let mut DElt: HUFv07_DEltX4 = core::mem::zeroed();
            MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
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
    srcSize: size_t,
) -> size_t {
    let mut weightList: [BYTE; (HUFv07_SYMBOLVALUE_MAX + 1) as usize] =
        [0; (HUFv07_SYMBOLVALUE_MAX + 1) as usize];
    let mut sortedSymbol: [sortedSymbol_t; (HUFv07_SYMBOLVALUE_MAX + 1) as usize] =
        [sortedSymbol_t { symbol: 0, weight: 0 }; (HUFv07_SYMBOLVALUE_MAX + 1) as usize];
    let mut rankStats: [U32; (HUFv07_TABLELOG_ABSOLUTEMAX + 1) as usize] =
        [0; (HUFv07_TABLELOG_ABSOLUTEMAX + 1) as usize];
    let mut rankStart0: [U32; (HUFv07_TABLELOG_ABSOLUTEMAX + 2) as usize] =
        [0; (HUFv07_TABLELOG_ABSOLUTEMAX + 2) as usize];
    let mut rankVal: [[U32; RANKVAL_COLS]; RANKVAL_ROWS] = [[0; RANKVAL_COLS]; RANKVAL_ROWS];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let mut dtd = HUFv07_getDTableDesc(DTable);
    let maxTableLog = dtd.maxTableLog as U32;
    let iSize: size_t;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv07_DEltX4;

    // rankStart = rankStart0 + 1
    if maxTableLog > HUFv07_TABLELOG_ABSOLUTEMAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUFv07_readStats(
        weightList.as_mut_ptr(),
        (HUFv07_SYMBOLVALUE_MAX + 1) as size_t,
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
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    // find maxWeight
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW -= 1;
    }

    // Get start index of each weight (rankStart[w] = rankStart0[w+1])
    {
        let mut nextRankStart: U32 = 0;
        let mut w: U32 = 1;
        while w < maxW + 1 {
            let current = nextRankStart;
            nextRankStart += rankStats[w as usize];
            rankStart0[(w + 1) as usize] = current; // rankStart[w]
            w += 1;
        }
        rankStart0[1] = nextRankStart; // rankStart[0]
        sizeOfSort = nextRankStart;
    }

    // sort symbols by weight
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w = weightList[s as usize] as U32;
            let r = rankStart0[(w + 1) as usize]; // rankStart[w]++
            rankStart0[(w + 1) as usize] += 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        rankStart0[1] = 0; // rankStart[0] = 0
    }

    // Build rankVal
    {
        // rankVal0 == rankVal[0]
        {
            let rescale = (maxTableLog as i32 - tableLog as i32) - 1;
            let mut nextRankVal: U32 = 0;
            let mut w: U32 = 1;
            while w < maxW + 1 {
                let current = nextRankVal;
                nextRankVal += rankStats[w as usize] << ((w as i32 + rescale) as u32);
                rankVal[0][w as usize] = current;
                w += 1;
            }
        }
        {
            let minBits = tableLog + 1 - maxW;
            let mut consumed = minBits;
            while consumed < maxTableLog - minBits + 1 {
                let mut w: U32 = 1;
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
        rankVal.as_ptr(),
        maxW,
        tableLog + 1,
    );

    dtd.tableLog = maxTableLog as BYTE;
    dtd.tableType = 1;
    memcpy(
        DTable as *mut c_void,
        &dtd as *const DTableDesc as *const c_void,
        core::mem::size_of::<DTableDesc>() as size_t,
    );
    iSize
}

#[inline]
unsafe fn HUFv07_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv07_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 2);
    BITv07_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

#[inline]
unsafe fn HUFv07_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv07_DStream_t,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv07_lookBitsFast(DStream, dtLog);
    memcpy(op, dt.add(val) as *const c_void, 1);
    if (*dt.add(val)).length == 1 {
        BITv07_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        let bc_bits = (core::mem::size_of::<size_t>() * 8) as u32;
        if (*DStream).bitsConsumed < bc_bits {
            BITv07_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > bc_bits {
                (*DStream).bitsConsumed = bc_bits;
            }
        }
    }
    1
}

macro_rules! HUF_DECODE_SYMBOLX4_0 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.add(HUFv07_decodeSymbolX4($ptr as *mut c_void, $ds, $dt, $dtLog) as usize);
    }};
}
macro_rules! HUF_DECODE_SYMBOLX4_1 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUFv07_TABLELOG_MAX <= 12) {
            $ptr = $ptr.add(HUFv07_decodeSymbolX4($ptr as *mut c_void, $ds, $dt, $dtLog) as usize);
        }
    }};
}
macro_rules! HUF_DECODE_SYMBOLX4_2 {
    ($ptr:expr, $ds:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            $ptr = $ptr.add(HUFv07_decodeSymbolX4($ptr as *mut c_void, $ds, $dt, $dtLog) as usize);
        }
    }};
}

#[inline]
unsafe fn HUFv07_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> size_t {
    let pStart = p;

    // up to 8 symbols at a time
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished) && (p < pEnd.offset(-7)) {
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    // closer to end : up to 2 symbols at a time
    while (BITv07_reloadDStream(bitDPtr) == BITv07_DStream_unfinished) && (p <= pEnd.offset(-2)) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p <= pEnd.offset(-2) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    if p < pEnd {
        p = p.add(HUFv07_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    p.offset_from(pStart) as size_t
}

unsafe fn HUFv07_decompress1X4_usingDTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
    let mut bitD: BITv07_DStream_t = core::mem::zeroed();

    {
        let errorCode = BITv07_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv07_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    {
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX4;
        let dtd = HUFv07_getDTableDesc(DTable);
        HUFv07_decodeStreamX4(ostart, &mut bitD, oend, dt, dtd.tableLog as U32);
    }

    if BITv07_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X4_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    // HUFv07_CREATE_STATIC_DTABLEX4(DTable, HUFv07_TABLELOG_MAX)
    let mut DTable: [HUFv07_DTable; 1 + (1 << HUFv07_TABLELOG_MAX)] =
        [0; 1 + (1 << HUFv07_TABLELOG_MAX)];
    DTable[0] = HUFv07_TABLELOG_MAX.wrapping_mul(0x1000001);
    HUFv07_decompress1X4_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

unsafe fn HUFv07_decompress4X4_usingDTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dstSize);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUFv07_DEltX4;

        let mut bitD1: BITv07_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv07_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv07_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv07_DStream_t = core::mem::zeroed();
        let length1 = MEM_readLE16(istart as *const c_void) as size_t;
        let length2 = MEM_readLE16(istart.add(2) as *const c_void) as size_t;
        let length3 = MEM_readLE16(istart.add(4) as *const c_void) as size_t;
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
        let mut endSignal: U32;
        let dtd = HUFv07_getDTableDesc(DTable);
        let dtLog = dtd.tableLog as U32;

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

        endSignal = BITv07_reloadDStream(&mut bitD1)
            | BITv07_reloadDStream(&mut bitD2)
            | BITv07_reloadDStream(&mut bitD3)
            | BITv07_reloadDStream(&mut bitD4);
        while (endSignal == BITv07_DStream_unfinished) && (op4 < oend.offset(-7)) {
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

            endSignal = BITv07_reloadDStream(&mut bitD1)
                | BITv07_reloadDStream(&mut bitD2)
                | BITv07_reloadDStream(&mut bitD3)
                | BITv07_reloadDStream(&mut bitD4);
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

        HUFv07_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv07_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv07_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv07_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        {
            let endCheck = BITv07_endOfDStream(&bitD1)
                & BITv07_endOfDStream(&bitD2)
                & BITv07_endOfDStream(&bitD3)
                & BITv07_endOfDStream(&bitD4);
            if endCheck == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }

        dstSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    let mut DTable: [HUFv07_DTable; 1 + (1 << HUFv07_TABLELOG_MAX)] =
        [0; 1 + (1 << HUFv07_TABLELOG_MAX)];
    DTable[0] = HUFv07_TABLELOG_MAX.wrapping_mul(0x1000001);
    HUFv07_decompress4X4_DCtx(DTable.as_mut_ptr(), dst, dstSize, cSrc, cSrcSize)
}

// ---- Generic decompression selector ----------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X_usingDTable(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
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
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUFv07_DTable,
) -> size_t {
    let dtd = HUFv07_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        HUFv07_decompress4X4_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    } else {
        HUFv07_decompress4X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_selectDecoder(dstSize: size_t, cSrcSize: size_t) -> U32 {
    let Q = (cSrcSize * 16 / dstSize) as U32;
    let D256 = (dstSize >> 8) as U32;
    let DTime0 = algoTime[Q as usize][0].tableTime
        + (algoTime[Q as usize][0].decode256Time.wrapping_mul(D256));
    let mut DTime1 = algoTime[Q as usize][1].tableTime
        + (algoTime[Q as usize][1].decode256Time.wrapping_mul(D256));
    DTime1 += DTime1 >> 3;

    (DTime1 < DTime0) as U32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    // validation checks
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

    let algoNb = HUFv07_selectDecoder(dstSize, cSrcSize);
    if algoNb != 0 {
        HUFv07_decompress4X4(dst, dstSize, cSrc, cSrcSize)
    } else {
        HUFv07_decompress4X2(dst, dstSize, cSrc, cSrcSize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X_DCtx(
    dctx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
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

    let algoNb = HUFv07_selectDecoder(dstSize, cSrcSize);
    if algoNb != 0 {
        HUFv07_decompress4X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    } else {
        HUFv07_decompress4X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X_hufOnly(
    dctx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if (cSrcSize >= dstSize) || (cSrcSize <= 1) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    let algoNb = HUFv07_selectDecoder(dstSize, cSrcSize);
    if algoNb != 0 {
        HUFv07_decompress4X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    } else {
        HUFv07_decompress4X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X_DCtx(
    dctx: *mut HUFv07_DTable,
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
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

    let algoNb = HUFv07_selectDecoder(dstSize, cSrcSize);
    if algoNb != 0 {
        HUFv07_decompress1X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    } else {
        HUFv07_decompress1X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    }
}

// ============================================================================
// ZSTDv07 / ZBUFFv07
// ============================================================================

pub const ZSTDv07_MAGICNUMBER: U32 = 0xFD2FB527;
pub const ZSTDv07_MAGIC_SKIPPABLE_START: U32 = 0x184D2A50;
pub const ZSTDv07_DICT_MAGIC: U32 = 0xEC30A437;

pub const ZSTDv07_WINDOWLOG_MAX_32: u32 = 25;
pub const ZSTDv07_WINDOWLOG_MAX_64: u32 = 27;
#[inline]
unsafe fn ZSTDv07_WINDOWLOG_MAX() -> U32 {
    if MEM_32bits() != 0 {
        ZSTDv07_WINDOWLOG_MAX_32
    } else {
        ZSTDv07_WINDOWLOG_MAX_64
    }
}

pub const ZSTDv07_FRAMEHEADERSIZE_MAX: usize = 18;
pub const ZSTDv07_frameHeaderSize_min: size_t = 5;
pub const ZSTDv07_frameHeaderSize_max: size_t = ZSTDv07_FRAMEHEADERSIZE_MAX;
pub const ZSTDv07_skippableHeaderSize: size_t = 8;

pub const ZSTDv07_BLOCKSIZE_ABSOLUTEMAX: usize = 128 * 1024;
pub const ZSTDv07_WINDOWLOG_ABSOLUTEMIN: u32 = 10;
pub const ZSTDv07_REP_NUM: usize = 3;
pub const ZSTDv07_REP_INIT: usize = ZSTDv07_REP_NUM;

pub const WILDCOPY_OVERLENGTH: usize = 8;
pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;
pub const ZSTDv07_BLOCKHEADERSIZE: size_t = 3;
pub const ZSTDv07_blockHeaderSize: size_t = ZSTDv07_BLOCKHEADERSIZE;

pub const MIN_SEQUENCES_SIZE: size_t = 1;
pub const MIN_CBLOCK_SIZE: size_t = 1 + 1 + MIN_SEQUENCES_SIZE;
pub const LONGNBSEQ: i32 = 0x7F00;
pub const MINMATCH: usize = 3;

pub const MaxLL: u32 = 35;
pub const MaxML: u32 = 52;
pub const MaxOff: u32 = 28;
pub const MLFSELog: u32 = 9;
pub const LLFSELog: u32 = 9;
pub const OffFSELog: u32 = 8;

pub const FSEv07_ENCODING_RAW: U32 = 0;
pub const FSEv07_ENCODING_RLE: U32 = 1;
pub const FSEv07_ENCODING_STATIC: U32 = 2;
pub const FSEv07_ENCODING_DYNAMIC: U32 = 3;

pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

const FSEv07_DTABLE_SIZE_U32_LL: usize = 1 + (1 << LLFSELog);
const FSEv07_DTABLE_SIZE_U32_OFF: usize = 1 + (1 << OffFSELog);
const FSEv07_DTABLE_SIZE_U32_ML: usize = 1 + (1 << MLFSELog);
const HUF_DTABLE_SIZE_CAP: usize = 1 + (1 << ZSTD_HUFFDTABLE_CAPACITY_LOG);

static repStartValue: [U32; ZSTDv07_REP_NUM] = [1, 4, 8];

static ZSTDv07_fcs_fieldSize: [size_t; 4] = [0, 2, 4, 8];
static ZSTDv07_did_fieldSize: [size_t; 4] = [0, 1, 2, 4];

static LL_bits: [U32; (MaxLL + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
static LL_defaultNorm: [S16; (MaxLL + 1) as usize] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1,
    1, -1, -1, -1, -1,
];
static LL_defaultNormLog: U32 = 6;

static ML_bits: [U32; (MaxML + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
static ML_defaultNorm: [S16; (MaxML + 1) as usize] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
static ML_defaultNormLog: U32 = 6;

static OF_defaultNorm: [S16; (MaxOff + 1) as usize] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
static OF_defaultNormLog: U32 = 5;

#[inline]
fn MIN_usize(a: size_t, b: size_t) -> size_t {
    if a < b {
        a
    } else {
        b
    }
}
#[inline]
fn MAX_u32(a: U32, b: U32) -> U32 {
    if a > b {
        a
    } else {
        b
    }
}

// blockType_t : { bt_compressed, bt_raw, bt_rle, bt_end }
pub type blockType_t = u32;
pub const bt_compressed: blockType_t = 0;
pub const bt_raw: blockType_t = 1;
pub const bt_rle: blockType_t = 2;
pub const bt_end: blockType_t = 3;

// litBlockType_t : { lbt_huffman, lbt_repeat, lbt_raw, lbt_rle }
pub const lbt_huffman: u32 = 0;
pub const lbt_repeat: u32 = 1;
pub const lbt_raw: u32 = 2;
pub const lbt_rle: u32 = 3;

// ---- custom memory management ----------------------------------------------
pub type ZSTDv07_allocFunction = Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>;
pub type ZSTDv07_freeFunction = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTDv07_customMem {
    pub customAlloc: ZSTDv07_allocFunction,
    pub customFree: ZSTDv07_freeFunction,
    pub opaque: *mut c_void,
}

unsafe extern "C" fn ZSTDv07_defaultAllocFunction(_opaque: *mut c_void, size: size_t) -> *mut c_void {
    malloc(size)
}

unsafe extern "C" fn ZSTDv07_defaultFreeFunction(_opaque: *mut c_void, address: *mut c_void) {
    free(address);
}

#[inline]
fn defaultCustomMem() -> ZSTDv07_customMem {
    ZSTDv07_customMem {
        customAlloc: Some(ZSTDv07_defaultAllocFunction),
        customFree: Some(ZSTDv07_defaultFreeFunction),
        opaque: core::ptr::null_mut(),
    }
}

// ---- frame params ----------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTDv07_frameParams {
    pub frameContentSize: u64,
    pub windowSize: u32,
    pub dictID: u32,
    pub checksumFlag: u32,
}

// ---- DCtx ------------------------------------------------------------------
pub type ZSTDv07_dStage = u32;
pub const ZSTDds_getFrameHeaderSize: ZSTDv07_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTDv07_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTDv07_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTDv07_dStage = 3;
pub const ZSTDds_decodeSkippableHeader: ZSTDv07_dStage = 4;
pub const ZSTDds_skipFrame: ZSTDv07_dStage = 5;

#[repr(C)]
pub struct ZSTDv07_DCtx {
    pub LLTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32_LL],
    pub OffTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32_OFF],
    pub MLTable: [FSEv07_DTable; FSEv07_DTABLE_SIZE_U32_ML],
    pub hufTable: [HUFv07_DTable; HUF_DTABLE_SIZE_CAP],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: size_t,
    pub rep: [U32; 3],
    pub fParams: ZSTDv07_frameParams,
    pub bType: blockType_t,
    pub stage: ZSTDv07_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: size_t,
    pub dictID: U32,
    pub litPtr: *const BYTE,
    pub customMem: ZSTDv07_customMem,
    pub litSize: size_t,
    pub litBuffer: [BYTE; ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [BYTE; ZSTDv07_FRAMEHEADERSIZE_MAX],
}

// ---- ZSTD/ZBUFF error management (exported) --------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isError(code: size_t) -> u32 {
    ERR_isError(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_isError(errorCode: size_t) -> u32 {
    ERR_isError(errorCode)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_getErrorName(errorCode: size_t) -> *const c_char {
    ERR_getErrorName(errorCode)
}

// ---- shared inline copy helpers --------------------------------------------
#[inline]
unsafe fn ZSTDv07_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}
#[inline]
unsafe fn ZSTDv07_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

#[inline]
pub unsafe fn ZSTDv07_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
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

// ---- DCtx management -------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_sizeofDCtx(_dctx: *const ZSTDv07_DCtx) -> size_t {
    core::mem::size_of::<ZSTDv07_DCtx>() as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_estimateDCtxSize() -> size_t {
    core::mem::size_of::<ZSTDv07_DCtx>() as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBegin(dctx: *mut ZSTDv07_DCtx) -> size_t {
    (*dctx).expected = ZSTDv07_frameHeaderSize_min;
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).hufTable[0] = (ZSTD_HUFFDTABLE_CAPACITY_LOG).wrapping_mul(0x1000001) as HUFv07_DTable;
    (*dctx).litEntropy = 0;
    (*dctx).fseEntropy = 0;
    (*dctx).dictID = 0;
    {
        let mut i = 0usize;
        while i < ZSTDv07_REP_NUM {
            (*dctx).rep[i] = repStartValue[i];
            i += 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDCtx_advanced(mut customMem: ZSTDv07_customMem) -> *mut ZSTDv07_DCtx {
    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem();
    }
    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    let dctx = (customMem.customAlloc.unwrap())(
        customMem.opaque,
        core::mem::size_of::<ZSTDv07_DCtx>() as size_t,
    ) as *mut ZSTDv07_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    memcpy(
        &mut (*dctx).customMem as *mut ZSTDv07_customMem as *mut c_void,
        &customMem as *const ZSTDv07_customMem as *const c_void,
        core::mem::size_of::<ZSTDv07_customMem>() as size_t,
    );
    ZSTDv07_decompressBegin(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDCtx() -> *mut ZSTDv07_DCtx {
    ZSTDv07_createDCtx_advanced(defaultCustomMem())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_freeDCtx(dctx: *mut ZSTDv07_DCtx) -> size_t {
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
        (core::mem::size_of::<ZSTDv07_DCtx>()
            - (ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + WILDCOPY_OVERLENGTH + ZSTDv07_frameHeaderSize_max))
            as size_t,
    );
}

// ---- frame header ----------------------------------------------------------
unsafe fn ZSTDv07_frameHeaderSize(src: *const c_void, srcSize: size_t) -> size_t {
    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    let fhd = *(src as *const BYTE).add(4);
    let dictID = (fhd & 3) as U32;
    let directMode = ((fhd >> 5) & 1) as U32;
    let fcsId = (fhd >> 6) as U32;
    ZSTDv07_frameHeaderSize_min
        + (directMode == 0) as size_t
        + ZSTDv07_did_fieldSize[dictID as usize]
        + ZSTDv07_fcs_fieldSize[fcsId as usize]
        + (directMode != 0 && ZSTDv07_fcs_fieldSize[fcsId as usize] == 0) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getFrameParams(
    fparamsPtr: *mut ZSTDv07_frameParams,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let ip = src as *const BYTE;

    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ZSTDv07_frameHeaderSize_min;
    }
    memset(
        fparamsPtr as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv07_frameParams>() as size_t,
    );
    if MEM_readLE32(src) != ZSTDv07_MAGICNUMBER {
        if (MEM_readLE32(src) & 0xFFFFFFF0u32) == ZSTDv07_MAGIC_SKIPPABLE_START {
            if srcSize < ZSTDv07_skippableHeaderSize {
                return ZSTDv07_skippableHeaderSize;
            }
            (*fparamsPtr).frameContentSize =
                MEM_readLE32((src as *const c_char).add(4) as *const c_void) as u64;
            (*fparamsPtr).windowSize = 0;
            return 0;
        }
        return ERROR(ZSTD_error_prefix_unknown);
    }

    {
        let fhsize = ZSTDv07_frameHeaderSize(src, srcSize);
        if srcSize < fhsize {
            return fhsize;
        }
    }

    {
        let fhdByte = *ip.add(4);
        let mut pos: size_t = 5;
        let dictIDSizeCode = (fhdByte & 3) as U32;
        let checksumFlag = ((fhdByte >> 2) & 1) as U32;
        let directMode = ((fhdByte >> 5) & 1) as U32;
        let fcsID = (fhdByte >> 6) as U32;
        let windowSizeMax: U32 = 1u32 << ZSTDv07_WINDOWLOG_MAX();
        let mut windowSize: U32 = 0;
        let mut dictID: U32 = 0;
        let mut frameContentSize: U64 = 0;
        if (fhdByte & 0x08) != 0 {
            return ERROR(ZSTD_error_frameParameter_unsupported);
        }
        if directMode == 0 {
            let wlByte = *ip.add(pos);
            pos += 1;
            let windowLog = (wlByte >> 3) as U32 + ZSTDv07_WINDOWLOG_ABSOLUTEMIN;
            if windowLog > ZSTDv07_WINDOWLOG_MAX() {
                return ERROR(ZSTD_error_frameParameter_unsupported);
            }
            windowSize = 1u32 << windowLog;
            windowSize = windowSize.wrapping_add((windowSize >> 3) * (wlByte & 7) as U32);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getDecompressedSize(src: *const c_void, srcSize: size_t) -> u64 {
    let mut fparams: ZSTDv07_frameParams = core::mem::zeroed();
    let frResult = ZSTDv07_getFrameParams(&mut fparams, src, srcSize);
    if frResult != 0 {
        return 0;
    }
    fparams.frameContentSize
}

unsafe fn ZSTDv07_decodeFrameHeader(
    dctx: *mut ZSTDv07_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let result = ZSTDv07_getFrameParams(&mut (*dctx).fParams, src, srcSize);
    if (*dctx).fParams.dictID != 0 && ((*dctx).dictID != (*dctx).fParams.dictID) {
        return ERROR(ZSTD_error_dictionary_wrong);
    }
    if (*dctx).fParams.checksumFlag != 0 {
        XXH64_reset(&mut (*dctx).xxhState, 0);
    }
    result
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

unsafe fn ZSTDv07_getcBlockSize(
    src: *const c_void,
    srcSize: size_t,
    bpPtr: *mut blockProperties_t,
) -> size_t {
    let inp = src as *const BYTE;
    let cSize: U32;

    if srcSize < ZSTDv07_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (*bpPtr).blockType = ((*inp) >> 6) as blockType_t;
    cSize = *inp.add(2) as U32 + ((*inp.add(1) as U32) << 8) + (((*inp.add(0) & 7) as U32) << 16);
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as size_t
}

unsafe fn ZSTDv07_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    if srcSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0 {
        memcpy(dst, src, srcSize);
    }
    srcSize
}

// ---- literals block --------------------------------------------------------
unsafe fn ZSTDv07_decodeLiteralsBlock(
    dctx: *mut ZSTDv07_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart.add(0) >> 6) as u32 {
        lbt_huffman => {
            let litSize: size_t;
            let litCSize: size_t;
            let mut singleStream: size_t = 0;
            let mut lhSize = ((*istart.add(0) >> 4) & 3) as U32;
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            match lhSize {
                2 => {
                    lhSize = 4;
                    litSize = (((*istart.add(0) & 15) as size_t) << 10)
                        + ((*istart.add(1) as size_t) << 2)
                        + ((*istart.add(2) >> 6) as size_t);
                    litCSize = (((*istart.add(2) & 63) as size_t) << 8) + *istart.add(3) as size_t;
                }
                3 => {
                    lhSize = 5;
                    litSize = (((*istart.add(0) & 15) as size_t) << 14)
                        + ((*istart.add(1) as size_t) << 6)
                        + ((*istart.add(2) >> 2) as size_t);
                    litCSize = (((*istart.add(2) & 3) as size_t) << 16)
                        + ((*istart.add(3) as size_t) << 8)
                        + *istart.add(4) as size_t;
                }
                _ => {
                    // case 0, 1, default
                    lhSize = 3;
                    singleStream = (*istart.add(0) & 16) as size_t;
                    litSize = (((*istart.add(0) & 15) as size_t) << 6)
                        + ((*istart.add(1) >> 2) as size_t);
                    litCSize = (((*istart.add(1) & 3) as size_t) << 8) + *istart.add(2) as size_t;
                }
            }
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if litCSize + lhSize as size_t > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
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
                return ERROR(ZSTD_error_corruption_detected);
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            (*dctx).litEntropy = 1;
            memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH as size_t,
            );
            litCSize + lhSize as size_t
        }
        lbt_repeat => {
            let litSize: size_t;
            let litCSize: size_t;
            let mut lhSize = ((*istart.add(0) >> 4) & 3) as U32;
            if lhSize != 1 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if (*dctx).litEntropy == 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }

            lhSize = 3;
            litSize =
                (((*istart.add(0) & 15) as size_t) << 6) + ((*istart.add(1) >> 2) as size_t);
            litCSize = (((*istart.add(1) & 3) as size_t) << 8) + *istart.add(2) as size_t;
            if litCSize + lhSize as size_t > srcSize {
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
            memset(
                (*dctx).litBuffer.as_mut_ptr().add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH as size_t,
            );
            litCSize + lhSize as size_t
        }
        lbt_raw => {
            let litSize: size_t;
            let mut lhSize = ((*istart.add(0) >> 4) & 3) as U32;
            match lhSize {
                2 => {
                    litSize = (((*istart.add(0) & 15) as size_t) << 8) + *istart.add(1) as size_t;
                }
                3 => {
                    litSize = (((*istart.add(0) & 15) as size_t) << 16)
                        + ((*istart.add(1) as size_t) << 8)
                        + *istart.add(2) as size_t;
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) & 31) as size_t;
                }
            }

            if lhSize as size_t + litSize + WILDCOPY_OVERLENGTH > srcSize {
                if litSize + lhSize as size_t > srcSize {
                    return ERROR(ZSTD_error_corruption_detected);
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
                    WILDCOPY_OVERLENGTH as size_t,
                );
                return lhSize as size_t + litSize;
            }
            (*dctx).litPtr = istart.add(lhSize as usize);
            (*dctx).litSize = litSize;
            lhSize as size_t + litSize
        }
        lbt_rle => {
            let litSize: size_t;
            let mut lhSize = ((*istart.add(0) >> 4) & 3) as U32;
            match lhSize {
                2 => {
                    litSize = (((*istart.add(0) & 15) as size_t) << 8) + *istart.add(1) as size_t;
                }
                3 => {
                    litSize = (((*istart.add(0) & 15) as size_t) << 16)
                        + ((*istart.add(1) as size_t) << 8)
                        + *istart.add(2) as size_t;
                    if srcSize < 4 {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.add(0) & 31) as size_t;
                }
            }
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.add(lhSize as usize) as i32,
                litSize + WILDCOPY_OVERLENGTH,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            lhSize as size_t + 1
        }
        _ => ERROR(ZSTD_error_corruption_detected),
    }
}

// ---- sequence tables -------------------------------------------------------
unsafe fn ZSTDv07_buildSeqTable(
    DTable: *mut FSEv07_DTable,
    type_: U32,
    max: U32,
    maxLog: U32,
    src: *const c_void,
    srcSize: size_t,
    defaultNorm: *const S16,
    defaultLog: U32,
    flagRepeatTable: U32,
) -> size_t {
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
            // FSEv07_ENCODING_DYNAMIC (and default)
            let mut tableLog: U32 = 0;
            let mut norm: [S16; (MaxSeq() + 1) as usize] = [0; (MaxSeq() + 1) as usize];
            let mut maxv = max;
            let headerSize = FSEv07_readNCount(norm.as_mut_ptr(), &mut maxv, &mut tableLog, src, srcSize);
            if FSEv07_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if tableLog > maxLog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            FSEv07_buildDTable(DTable, norm.as_ptr(), maxv, tableLog);
            headerSize
        }
    }
}

#[inline]
const fn MaxSeq() -> u32 {
    // MAX(MaxLL, MaxML) = MAX(35, 52) = 52
    if MaxLL > MaxML {
        MaxLL
    } else {
        MaxML
    }
}

unsafe fn ZSTDv07_decodeSeqHeaders(
    nbSeqPtr: *mut i32,
    DTableLL: *mut FSEv07_DTable,
    DTableML: *mut FSEv07_DTable,
    DTableOffb: *mut FSEv07_DTable,
    flagRepeatTable: U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let istart = src as *const BYTE;
    let iend = istart.add(srcSize);
    let mut ip = istart;

    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    // SeqHead
    {
        let mut nbSeq: i32 = *ip as i32;
        ip = ip.add(1);
        if nbSeq == 0 {
            *nbSeqPtr = 0;
            return 1;
        }
        if nbSeq > 0x7F {
            if nbSeq == 0xFF {
                if ip.add(2) > iend {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = MEM_readLE16(ip as *const c_void) as i32 + LONGNBSEQ;
                ip = ip.add(2);
            } else {
                if ip >= iend {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = ((nbSeq - 0x80) << 8) + *ip as i32;
                ip = ip.add(1);
            }
        }
        *nbSeqPtr = nbSeq;
    }

    // FSE table descriptors
    if ip.add(4) > iend {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let LLtype = (*ip >> 6) as U32;
        let OFtype = ((*ip >> 4) & 3) as U32;
        let MLtype = ((*ip >> 2) & 3) as U32;
        ip = ip.add(1);

        {
            let llhSize = ZSTDv07_buildSeqTable(
                DTableLL,
                LLtype,
                MaxLL,
                LLFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
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
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
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
                MaxML,
                MLFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
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

    ip.offset_from(istart) as size_t
}

// ---- sequence decoding -----------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct seq_t {
    pub litLength: size_t,
    pub matchLength: size_t,
    pub offset: size_t,
}

#[repr(C)]
pub struct seqState_t {
    pub DStream: BITv07_DStream_t,
    pub stateLL: FSEv07_DState_t,
    pub stateOffb: FSEv07_DState_t,
    pub stateML: FSEv07_DState_t,
    pub prevOffset: [size_t; ZSTDv07_REP_INIT],
}

static LL_base: [U32; (MaxLL + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];
static ML_base: [U32; (MaxML + 1) as usize] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
    0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003,
];
static OF_base: [U32; (MaxOff + 1) as usize] = [
    0, 1, 1, 5, 0xD, 0x1D, 0x3D, 0x7D, 0xFD, 0x1FD, 0x3FD, 0x7FD, 0xFFD, 0x1FFD, 0x3FFD, 0x7FFD,
    0xFFFD, 0x1FFFD, 0x3FFFD, 0x7FFFD, 0xFFFFD, 0x1FFFFD, 0x3FFFFD, 0x7FFFFD, 0xFFFFFD, 0x1FFFFFD,
    0x3FFFFFD, 0x7FFFFFD, 0xFFFFFFD,
];

unsafe fn ZSTDv07_decodeSequence(seqState: *mut seqState_t) -> seq_t {
    let mut seq: seq_t = core::mem::zeroed();

    let llCode = FSEv07_peekSymbol(&(*seqState).stateLL) as U32;
    let mlCode = FSEv07_peekSymbol(&(*seqState).stateML) as U32;
    let ofCode = FSEv07_peekSymbol(&(*seqState).stateOffb) as U32;

    let llBits = LL_bits[llCode as usize];
    let mlBits = ML_bits[mlCode as usize];
    let ofBits = ofCode;
    let totalBits = llBits + mlBits + ofBits;

    // sequence offset
    {
        let mut offset: size_t;
        if ofCode == 0 {
            offset = 0;
        } else {
            offset = OF_base[ofCode as usize] as size_t
                + BITv07_readBits(&mut (*seqState).DStream, ofBits);
            if MEM_32bits() != 0 {
                BITv07_reloadDStream(&mut (*seqState).DStream);
            }
        }

        if ofCode <= 1 {
            if (llCode == 0) && (offset <= 1) {
                offset = (1 as size_t).wrapping_sub(offset);
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

    seq.matchLength = ML_base[mlCode as usize] as size_t
        + if mlCode > 31 {
            BITv07_readBits(&mut (*seqState).DStream, mlBits)
        } else {
            0
        };
    if MEM_32bits() != 0 && (mlBits + llBits > 24) {
        BITv07_reloadDStream(&mut (*seqState).DStream);
    }

    seq.litLength = LL_base[llCode as usize] as size_t
        + if llCode > 15 {
            BITv07_readBits(&mut (*seqState).DStream, llBits)
        } else {
            0
        };
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
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.add(sequenceLength);
    let oend_w = oend.offset(-(WILDCOPY_OVERLENGTH as isize));
    let iLitEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.offset(-(sequence.offset as isize)) as *const BYTE;

    // check
    if sequence.litLength + WILDCOPY_OVERLENGTH > (oend.offset_from(op) as size_t) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequenceLength > (oend.offset_from(op) as size_t) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > ((litLimit.offset_from(*litPtr)) as size_t) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    // copy Literals
    ZSTDv07_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = iLitEnd;

    // copy Match
    if sequence.offset > (oLitEnd.offset_from(base) as size_t) {
        if sequence.offset > (oLitEnd.offset_from(vBase) as size_t) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        // match = dictEnd - (base - match)
        match_ = dictEnd.offset(-(base.offset_from(match_) as isize));
        if match_.add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        {
            let length1 = dictEnd.offset_from(match_) as size_t;
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

    // match within prefix
    if sequence.offset < 8 {
        static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
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
                oend_w.offset_from(op),
            );
            match_ = match_.offset(oend_w.offset_from(op));
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
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
) -> size_t {
    let mut ip = seqStart as *const BYTE;
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(maxDstSize);
    let mut op = ostart;
    let mut litPtr = (*dctx).litPtr;
    let litEnd = litPtr.add((*dctx).litSize);
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: i32 = 0;

    // Build Decoding Tables
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

    // Regen sequences
    if nbSeq != 0 {
        let mut seqState: seqState_t = core::mem::zeroed();
        (*dctx).fseEntropy = 1;
        {
            let mut i = 0usize;
            while i < ZSTDv07_REP_INIT {
                seqState.prevOffset[i] = (*dctx).rep[i] as size_t;
                i += 1;
            }
        }
        {
            let errorCode = BITv07_initDStream(
                &mut seqState.DStream,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }
        FSEv07_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSEv07_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSEv07_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BITv07_reloadDStream(&mut seqState.DStream) <= BITv07_DStream_completed) && nbSeq != 0
        {
            nbSeq -= 1;
            let sequence = ZSTDv07_decodeSequence(&mut seqState);
            let oneSeqSize = ZSTDv07_execSequence(
                op, oend, sequence, &mut litPtr, litEnd, base, vBase, dictEnd,
            );
            if ZSTDv07_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
        }

        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        {
            let mut i = 0usize;
            while i < ZSTDv07_REP_INIT {
                (*dctx).rep[i] = seqState.prevOffset[i] as U32;
                i += 1;
            }
        }
    }

    // last literal segment
    {
        let lastLLSize = litEnd.offset_from(litPtr) as size_t;
        if lastLLSize > (oend.offset_from(op) as size_t) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if lastLLSize > 0 {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    op.offset_from(ostart) as size_t
}

unsafe fn ZSTDv07_checkContinuity(dctx: *mut ZSTDv07_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char).offset(
            -(((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char)),
        ) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

unsafe fn ZSTDv07_decompressBlock_internal(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip = src as *const BYTE;

    if srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX {
        return ERROR(ZSTD_error_srcSize_wrong);
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
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let dSize: size_t;
    ZSTDv07_checkContinuity(dctx, dst);
    dSize = ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
    (*dctx).previousDstEnd = (dst as *mut c_char).add(dSize) as *const c_void;
    dSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_insertBlock(
    dctx: *mut ZSTDv07_DCtx,
    blockStart: *const c_void,
    blockSize: size_t,
) -> size_t {
    ZSTDv07_checkContinuity(dctx, blockStart);
    (*dctx).previousDstEnd = (blockStart as *const c_char).add(blockSize) as *const c_void;
    blockSize
}

unsafe fn ZSTDv07_generateNxBytes(
    dst: *mut c_void,
    dstCapacity: size_t,
    byte: BYTE,
    length: size_t,
) -> size_t {
    if length > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if length > 0 {
        memset(dst, byte as i32, length);
    }
    length
}

// ---- frame decompress ------------------------------------------------------
unsafe fn ZSTDv07_decompressFrame(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut ip = src as *const BYTE;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dstCapacity);
    let mut op = ostart;
    let mut remainingSize = srcSize;

    if srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    // Frame Header
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

    // Loop on each block
    loop {
        let mut decodedSize: size_t;
        let mut blockProperties: blockProperties_t = core::mem::zeroed();
        let cBlockSize =
            ZSTDv07_getcBlockSize(ip as *const c_void, iend.offset_from(ip) as size_t, &mut blockProperties);
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
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_raw => {
                decodedSize = ZSTDv07_copyRawBlock(
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_rle => {
                decodedSize = ZSTDv07_generateNxBytes(
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    *ip,
                    blockProperties.origSize as size_t,
                );
            }
            bt_end => {
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
            XXH64_update(&mut (*dctx).xxhState, op as *const c_void, decodedSize);
        }
        op = op.add(decodedSize);
        ip = ip.add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op.offset_from(ostart) as size_t
}

unsafe fn ZSTDv07_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv07_DCtx,
    refDCtx: *const ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTDv07_copyDCtx(dctx, refDCtx);
    ZSTDv07_checkContinuity(dctx, dst);
    ZSTDv07_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompress_usingDict(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    ZSTDv07_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv07_checkContinuity(dctx, dst);
    ZSTDv07_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressDCtx(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
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
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    // ZSTDv07_HEAPMODE == 1
    let regenSize: size_t;
    let dctx = ZSTDv07_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv07_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv07_freeDCtx(dctx);
    regenSize
}

unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut size_t, dBound: *mut u64, ret: size_t) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: size_t,
    cSize: *mut size_t,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: size_t = 0;

    if srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }

    // Frame Header
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

    // Loop on each block
    loop {
        let mut blockProperties: blockProperties_t = core::mem::zeroed();
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

    *cSize = ip.offset_from(src as *const BYTE) as size_t;
    *dBound = (nbBlocks * ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) as u64;
}

// ---- streaming (bufferless) ------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_nextSrcSizeToDecompress(dctx: *mut ZSTDv07_DCtx) -> size_t {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isSkipFrame(dctx: *mut ZSTDv07_DCtx) -> i32 {
    ((*dctx).stage == ZSTDds_skipFrame) as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressContinue(
    dctx: *mut ZSTDv07_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    // Sanity check
    if srcSize != (*dctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dstCapacity != 0 {
        ZSTDv07_checkContinuity(dctx, dst);
    }

    // The C uses switch with a fall-through from ZSTDds_getFrameHeaderSize into
    // ZSTDds_decodeFrameHeader. We reproduce it by a loop-once with explicit
    // stage variable so the fall-through happens.
    let mut stage = (*dctx).stage;
    loop {
        match stage {
            ZSTDds_getFrameHeaderSize => {
                if srcSize != ZSTDv07_frameHeaderSize_min {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                if (MEM_readLE32(src) & 0xFFFFFFF0u32) == ZSTDv07_MAGIC_SKIPPABLE_START {
                    memcpy(
                        (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                        src,
                        ZSTDv07_frameHeaderSize_min,
                    );
                    (*dctx).expected =
                        ZSTDv07_skippableHeaderSize - ZSTDv07_frameHeaderSize_min;
                    (*dctx).stage = ZSTDds_decodeSkippableHeader;
                    return 0;
                }
                (*dctx).headerSize =
                    ZSTDv07_frameHeaderSize(src, ZSTDv07_frameHeaderSize_min);
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
                (*dctx).expected = 0; // not necessary to copy more
                // fall-through
                stage = ZSTDds_decodeFrameHeader;
                continue;
            }
            ZSTDds_decodeFrameHeader => {
                let result: size_t;
                memcpy(
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
                let mut bp: blockProperties_t = core::mem::zeroed();
                let cBlockSize =
                    ZSTDv07_getcBlockSize(src, ZSTDv07_blockHeaderSize, &mut bp);
                if ZSTDv07_isError(cBlockSize) != 0 {
                    return cBlockSize;
                }
                if bp.blockType == bt_end {
                    if (*dctx).fParams.checksumFlag != 0 {
                        let h64 = XXH64_digest(&(*dctx).xxhState);
                        let h32 = ((h64 >> 11) as U32) & ((1u32 << 22) - 1);
                        let ip = src as *const BYTE;
                        let check32 = *ip.add(2) as U32
                            + ((*ip.add(1) as U32) << 8)
                            + (((*ip.add(0) & 0x3F) as U32) << 16);
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
                let rSize: size_t;
                match (*dctx).bType {
                    bt_compressed => {
                        rSize = ZSTDv07_decompressBlock_internal(
                            dctx, dst, dstCapacity, src, srcSize,
                        );
                    }
                    bt_raw => {
                        rSize = ZSTDv07_copyRawBlock(dst, dstCapacity, src, srcSize);
                    }
                    bt_rle => {
                        return ERROR(ZSTD_error_GENERIC);
                    }
                    bt_end => {
                        rSize = 0;
                    }
                    _ => {
                        return ERROR(ZSTD_error_GENERIC);
                    }
                }
                (*dctx).stage = ZSTDds_decodeBlockHeader;
                (*dctx).expected = ZSTDv07_blockHeaderSize;
                if ZSTDv07_isError(rSize) != 0 {
                    return rSize;
                }
                (*dctx).previousDstEnd = (dst as *mut c_char).add(rSize) as *const c_void;
                if (*dctx).fParams.checksumFlag != 0 {
                    XXH64_update(&mut (*dctx).xxhState, dst as *const c_void, rSize);
                }
                return rSize;
            }
            ZSTDds_decodeSkippableHeader => {
                memcpy(
                    (*dctx)
                        .headerBuffer
                        .as_mut_ptr()
                        .add(ZSTDv07_frameHeaderSize_min) as *mut c_void,
                    src,
                    (*dctx).expected,
                );
                (*dctx).expected =
                    MEM_readLE32((*dctx).headerBuffer.as_ptr().add(4) as *const c_void) as size_t;
                (*dctx).stage = ZSTDds_skipFrame;
                return 0;
            }
            ZSTDds_skipFrame => {
                (*dctx).expected = 0;
                (*dctx).stage = ZSTDds_getFrameHeaderSize;
                return 0;
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
    }
}

// ---- dictionary ------------------------------------------------------------
unsafe fn ZSTDv07_refDictContent(dctx: *mut ZSTDv07_DCtx, dict: *const c_void, dictSize: size_t) -> size_t {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const c_char).offset(
        -(((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char)),
    ) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const c_char).add(dictSize) as *const c_void;
    0
}

unsafe fn ZSTDv07_loadEntropy(dctx: *mut ZSTDv07_DCtx, dict: *const c_void, dictSize: size_t) -> size_t {
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
        let mut offcodeNCount: [i16; (MaxOff + 1) as usize] = [0; (MaxOff + 1) as usize];
        let mut offcodeMaxValue: U32 = MaxOff;
        let mut offcodeLog: U32 = 0;
        let offcodeHeaderSize = FSEv07_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
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
        let mut matchlengthNCount: [i16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut matchlengthMaxValue: U32 = MaxML;
        let mut matchlengthLog: U32 = 0;
        let matchlengthHeaderSize = FSEv07_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
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
        let mut litlengthNCount: [i16; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
        let mut litlengthMaxValue: U32 = MaxLL;
        let mut litlengthLog: U32 = 0;
        let litlengthHeaderSize = FSEv07_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
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

    if dictPtr.add(12) > dictEnd {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[0] = MEM_readLE32(dictPtr.add(0) as *const c_void);
    if (*dctx).rep[0] == 0 || (*dctx).rep[0] as size_t >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[1] = MEM_readLE32(dictPtr.add(4) as *const c_void);
    if (*dctx).rep[1] == 0 || (*dctx).rep[1] as size_t >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*dctx).rep[2] = MEM_readLE32(dictPtr.add(8) as *const c_void);
    if (*dctx).rep[2] == 0 || (*dctx).rep[2] as size_t >= dictSize {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dictPtr = dictPtr.add(12);

    (*dctx).litEntropy = 1;
    (*dctx).fseEntropy = 1;
    dictPtr.offset_from(dict as *const BYTE) as size_t
}

unsafe fn ZSTDv07_decompress_insertDictionary(
    dctx: *mut ZSTDv07_DCtx,
    mut dict: *const c_void,
    mut dictSize: size_t,
) -> size_t {
    if dictSize < 8 {
        return ZSTDv07_refDictContent(dctx, dict, dictSize);
    }
    {
        let magic = MEM_readLE32(dict);
        if magic != ZSTDv07_DICT_MAGIC {
            return ZSTDv07_refDictContent(dctx, dict, dictSize);
        }
    }
    (*dctx).dictID = MEM_readLE32((dict as *const c_char).add(4) as *const c_void);

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

    ZSTDv07_refDictContent(dctx, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBegin_usingDict(
    dctx: *mut ZSTDv07_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
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

// ---- DDict -----------------------------------------------------------------
#[repr(C)]
pub struct ZSTDv07_DDict {
    pub dict: *mut c_void,
    pub dictSize: size_t,
    pub refContext: *mut ZSTDv07_DCtx,
}

unsafe fn ZSTDv07_createDDict_advanced(
    dict: *const c_void,
    dictSize: size_t,
    mut customMem: ZSTDv07_customMem,
) -> *mut ZSTDv07_DDict {
    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem();
    }
    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    {
        let ddict = (customMem.customAlloc.unwrap())(
            customMem.opaque,
            core::mem::size_of::<ZSTDv07_DDict>() as size_t,
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
pub unsafe extern "C" fn ZSTDv07_createDDict(dict: *const c_void, dictSize: size_t) -> *mut ZSTDv07_DDict {
    let allocator = ZSTDv07_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTDv07_createDDict_advanced(dict, dictSize, allocator)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_freeDDict(ddict: *mut ZSTDv07_DDict) -> size_t {
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
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    ddict: *const ZSTDv07_DDict,
) -> size_t {
    ZSTDv07_decompress_usingPreparedDCtx(
        dctx,
        (*ddict).refContext,
        dst,
        dstCapacity,
        src,
        srcSize,
    )
}

// ============================================================================
// ZBUFFv07 streaming
// ============================================================================
pub type ZBUFFv07_dStage = u32;
pub const ZBUFFds_init: ZBUFFv07_dStage = 0;
pub const ZBUFFds_loadHeader: ZBUFFv07_dStage = 1;
pub const ZBUFFds_read: ZBUFFv07_dStage = 2;
pub const ZBUFFds_load: ZBUFFv07_dStage = 3;
pub const ZBUFFds_flush: ZBUFFv07_dStage = 4;

#[repr(C)]
pub struct ZBUFFv07_DCtx {
    pub zd: *mut ZSTDv07_DCtx,
    pub fParams: ZSTDv07_frameParams,
    pub stage: ZBUFFv07_dStage,
    pub inBuff: *mut c_char,
    pub inBuffSize: size_t,
    pub inPos: size_t,
    pub outBuff: *mut c_char,
    pub outBuffSize: size_t,
    pub outStart: size_t,
    pub outEnd: size_t,
    pub blockSize: size_t,
    pub headerBuffer: [BYTE; ZSTDv07_FRAMEHEADERSIZE_MAX],
    pub lhSize: size_t,
    pub customMem: ZSTDv07_customMem,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_createDCtx() -> *mut ZBUFFv07_DCtx {
    ZBUFFv07_createDCtx_advanced(defaultCustomMem())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_createDCtx_advanced(mut customMem: ZSTDv07_customMem) -> *mut ZBUFFv07_DCtx {
    let zbd: *mut ZBUFFv07_DCtx;

    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem();
    }
    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return core::ptr::null_mut();
    }

    zbd = (customMem.customAlloc.unwrap())(
        customMem.opaque,
        core::mem::size_of::<ZBUFFv07_DCtx>() as size_t,
    ) as *mut ZBUFFv07_DCtx;
    if zbd.is_null() {
        return core::ptr::null_mut();
    }
    memset(
        zbd as *mut c_void,
        0,
        core::mem::size_of::<ZBUFFv07_DCtx>() as size_t,
    );
    memcpy(
        &mut (*zbd).customMem as *mut ZSTDv07_customMem as *mut c_void,
        &customMem as *const ZSTDv07_customMem as *const c_void,
        core::mem::size_of::<ZSTDv07_customMem>() as size_t,
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
pub unsafe extern "C" fn ZBUFFv07_freeDCtx(zbd: *mut ZBUFFv07_DCtx) -> size_t {
    if zbd.is_null() {
        return 0;
    }
    ZSTDv07_freeDCtx((*zbd).zd);
    if !(*zbd).inBuff.is_null() {
        ((*zbd).customMem.customFree.unwrap())((*zbd).customMem.opaque, (*zbd).inBuff as *mut c_void);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInitDictionary(
    zbd: *mut ZBUFFv07_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    (*zbd).stage = ZBUFFds_loadHeader;
    (*zbd).lhSize = 0;
    (*zbd).inPos = 0;
    (*zbd).outStart = 0;
    (*zbd).outEnd = 0;
    ZSTDv07_decompressBegin_usingDict((*zbd).zd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInit(zbd: *mut ZBUFFv07_DCtx) -> size_t {
    ZBUFFv07_decompressInitDictionary(zbd, core::ptr::null(), 0)
}

#[inline]
unsafe fn ZBUFFv07_limitCopy(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let length = MIN_usize(dstCapacity, srcSize);
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressContinue(
    zbd: *mut ZBUFFv07_DCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut size_t,
    src: *const c_void,
    srcSizePtr: *mut size_t,
) -> size_t {
    let istart = src as *const c_char;
    let iend = istart.add(*srcSizePtr);
    let mut ip = istart;
    let ostart = dst as *mut c_char;
    let oend = ostart.add(*dstCapacityPtr);
    let mut op = ostart;
    let mut notDone: U32 = 1;

    // The C switch has fall-through: loadHeader -> read -> load -> flush.
    // We use an inner `stage` variable driven loop; a C `break;` from a case
    // (return to the outer `while(notDone)` and re-read `zbd->stage`) is modeled
    // by `stage = (*zbd).stage; continue 'outer;` and explicit fall-through by
    // setting `stage` to the next case and falling into it via `continue 'inner`.
    'outer: while notDone != 0 {
        let mut stage = (*zbd).stage;
        'inner: loop {
            match stage {
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
                            let toLoad = hSize - (*zbd).lhSize;
                            if toLoad > (iend.offset_from(ip) as size_t) {
                                if !ip.is_null() {
                                    memcpy(
                                        (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize)
                                            as *mut c_void,
                                        ip as *const c_void,
                                        iend.offset_from(ip) as size_t,
                                    );
                                }
                                (*zbd).lhSize += iend.offset_from(ip) as size_t;
                                *dstCapacityPtr = 0;
                                return (hSize - (*zbd).lhSize) + ZSTDv07_blockHeaderSize;
                            }
                            memcpy(
                                (*zbd).headerBuffer.as_mut_ptr().add((*zbd).lhSize)
                                    as *mut c_void,
                                ip as *const c_void,
                                toLoad,
                            );
                            (*zbd).lhSize = hSize;
                            ip = ip.add(toLoad);
                            // C `break;` -> re-enter outer while (stage unchanged)
                            continue 'outer;
                        }
                    }

                    // Consume header
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

                    (*zbd).fParams.windowSize = MAX_u32(
                        (*zbd).fParams.windowSize,
                        1u32 << ZSTDv07_WINDOWLOG_ABSOLUTEMIN,
                    );

                    // Frame header instruct buffer sizes
                    {
                        let blockSize = MIN_usize(
                            (*zbd).fParams.windowSize as size_t,
                            ZSTDv07_BLOCKSIZE_ABSOLUTEMAX,
                        );
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
                            let neededOutSize = (*zbd).fParams.windowSize as size_t
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
                    // fall-through
                    stage = ZBUFFds_read;
                    continue 'inner;
                }

                ZBUFFds_read => {
                    {
                        let neededInSize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                        if neededInSize == 0 {
                            // end of frame
                            (*zbd).stage = ZBUFFds_init;
                            notDone = 0;
                            continue 'outer;
                        }
                        if (iend.offset_from(ip) as size_t) >= neededInSize {
                            // decode directly from src
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
                                continue 'outer; // just a header
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
                    // fall-through
                    stage = ZBUFFds_load;
                    continue 'inner;
                }

                ZBUFFds_load => {
                    {
                        let neededInSize = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
                        let toLoad = neededInSize - (*zbd).inPos;
                        let loadedSize;
                        if toLoad > (*zbd).inBuffSize - (*zbd).inPos {
                            return ERROR(ZSTD_error_corruption_detected);
                        }
                        loadedSize = ZBUFFv07_limitCopy(
                            (*zbd).inBuff.add((*zbd).inPos) as *mut c_void,
                            toLoad,
                            ip as *const c_void,
                            iend.offset_from(ip) as size_t,
                        );
                        ip = ip.add(loadedSize);
                        (*zbd).inPos += loadedSize;
                        if loadedSize < toLoad {
                            notDone = 0;
                            continue 'outer;
                        }

                        // decode loaded input
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
                            // pass-through
                        }
                    }
                    // fall-through
                    stage = ZBUFFds_flush;
                    continue 'inner;
                }

                ZBUFFds_flush => {
                    let toFlushSize = (*zbd).outEnd - (*zbd).outStart;
                    let flushedSize = ZBUFFv07_limitCopy(
                        op as *mut c_void,
                        oend.offset_from(op) as size_t,
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
                    // cannot flush everything
                    notDone = 0;
                    continue 'outer;
                }

                _ => {
                    return ERROR(ZSTD_error_GENERIC);
                }
            }
        }
    }

    *srcSizePtr = ip.offset_from(istart) as size_t;
    *dstCapacityPtr = op.offset_from(ostart) as size_t;
    {
        let mut nextSrcSizeHint = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbd).inPos);
        return nextSrcSizeHint;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_recommendedDInSize() -> size_t {
    ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + ZSTDv07_blockHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_recommendedDOutSize() -> size_t {
    ZSTDv07_BLOCKSIZE_ABSOLUTEMAX
}
