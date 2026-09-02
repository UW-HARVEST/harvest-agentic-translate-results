//! Literal, semantics-preserving transliteration of `legacy/zstd_v06.c`.
//!
//! This file is a self-contained translation unit: it bundles its own
//! mem/endian helpers, error codes, bitstream reader, FSEv06 decoder, HUFv06
//! decoder, block/frame decoders, DCtx and the ZBUFFv06 streaming state
//! machine. Everything is translated literally; only the 37 documented
//! `FSEv06_*` / `HUFv06_*` / `ZSTDv06_*` / `ZBUFFv06_*` entry points are
//! exported (`#[unsafe(no_mangle)]`).

use core::ffi::{c_char, c_void};

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
// zstd_v06.c `#include "../common/error_private.h"` FIRST, which sets the
// ERROR_H_MODULE guard; the file has no local error enum. Hence ERROR(name),
// ERR_isError, ERR_getErrorName all resolve to the MODERN zstd error codes
// from zstd_errors.h. We reproduce those numeric values and string table
// exactly here to stay self-contained (verified against the C library).
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
// zstd_internal constants (v0.6)
// ============================================================================
const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526;
const ZSTDv06_DICT_MAGIC: U32 = 0xEC30A436;

const ZSTDv06_REP_NUM: U32 = 3;
const ZSTDv06_REP_INIT: usize = ZSTDv06_REP_NUM as usize;
const ZSTDv06_REP_MOVE: U32 = ZSTDv06_REP_NUM - 1;

const ZSTDv06_WINDOWLOG_ABSOLUTEMIN: U32 = 12;

const ZSTDv06_BLOCKHEADERSIZE: size_t = 3;
const ZSTDv06_blockHeaderSize: size_t = ZSTDv06_BLOCKHEADERSIZE;

const MIN_SEQUENCES_SIZE: size_t = 1;
const MIN_CBLOCK_SIZE: size_t = 1 + 1 + MIN_SEQUENCES_SIZE;

const ZSTD_HUFFDTABLE_CAPACITY_LOG: U32 = 12;

const IS_HUF: U32 = 0;
const IS_PCH: U32 = 1;
const IS_RAW: U32 = 2;
const IS_RLE: U32 = 3;

const LONGNBSEQ: i32 = 0x7F00;

const MINMATCH: size_t = 3;
const REPCODE_STARTVALUE: size_t = 1;

const Litbits: U32 = 8;
const MaxLit: U32 = (1 << Litbits) - 1;
const MaxML: U32 = 52;
const MaxLL: U32 = 35;
const MaxOff: U32 = 28;
const MaxSeq: U32 = if MaxLL > MaxML { MaxLL } else { MaxML };
const MLFSELog: U32 = 9;
const LLFSELog: U32 = 9;
const OffFSELog: U32 = 8;

const FSEv06_ENCODING_RAW: U32 = 0;
const FSEv06_ENCODING_RLE: U32 = 1;
const FSEv06_ENCODING_STATIC: U32 = 2;
const FSEv06_ENCODING_DYNAMIC: U32 = 3;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

const WILDCOPY_OVERLENGTH: usize = 8;

const ZSTDv06_FRAMEHEADERSIZE_MAX: usize = 13;
const ZSTDv06_frameHeaderSize_min: size_t = 5;
const ZSTDv06_frameHeaderSize_max: size_t = ZSTDv06_FRAMEHEADERSIZE_MAX;
const ZSTDv06_BLOCKSIZE_MAX: usize = 128 * 1024;

// static const size_t ZSTDv06_fcs_fieldSize[4] = { 0, 1, 2, 8 };
static ZSTDv06_fcs_fieldSize: [size_t; 4] = [0, 1, 2, 8];

// blockType_t { bt_compressed, bt_raw, bt_rle, bt_end }
pub type blockType_t = u32;
const bt_compressed: blockType_t = 0;
const bt_raw: blockType_t = 1;
const bt_rle: blockType_t = 2;
const bt_end: blockType_t = 3;

// --- sequence symbol tables (v0.6) ---
static LL_bits: [U32; (MaxLL + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
static LL_defaultNorm: [S16; (MaxLL + 1) as usize] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
const LL_defaultNormLog: U32 = 6;

static ML_bits: [U32; (MaxML + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
static ML_defaultNorm: [S16; (MaxML + 1) as usize] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
const ML_defaultNormLog: U32 = 6;

static OF_defaultNorm: [S16; (MaxOff + 1) as usize] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
const OF_defaultNormLog: U32 = 5;

// static void ZSTDv06_copy8(void* dst, const void* src) { memcpy(dst, src, 8); }
#[inline]
unsafe fn ZSTDv06_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}

// MEM_STATIC void ZSTDv06_wildcopy(void* dst, const void* src, ptrdiff_t length)
#[inline]
unsafe fn ZSTDv06_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.wrapping_offset(length);
    loop {
        ZSTDv06_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
        if !(op < oend) {
            break;
        }
    }
}

// ============================================================================
// bitstream (BITv06_DStream)
// ============================================================================
#[repr(C)]
pub struct BITv06_DStream_t {
    pub bitContainer: size_t,
    pub bitsConsumed: u32,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

// BITv06_DStream_status
pub type BITv06_DStream_status = u32;
const BITv06_DStream_unfinished: BITv06_DStream_status = 0;
const BITv06_DStream_endOfBuffer: BITv06_DStream_status = 1;
const BITv06_DStream_completed: BITv06_DStream_status = 2;
const BITv06_DStream_overflow: BITv06_DStream_status = 3;

#[inline]
pub unsafe fn BITv06_highbit32(val: U32) -> u32 {
    // __builtin_clz(val) ^ 31  == 31 - leading_zeros
    31u32.wrapping_sub(val.leading_zeros())
}

#[inline]
pub unsafe fn BITv06_initDStream(
    bitD: *mut BITv06_DStream_t,
    srcBuffer: *const c_void,
    srcSize: size_t,
) -> size_t {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BITv06_DStream_t>());
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    let bc_size = core::mem::size_of::<size_t>();
    if srcSize >= bc_size {
        /* normal case */
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char).wrapping_add(srcSize - bc_size);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        {
            let lastByte: BYTE = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1);
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC); /* endMark not present */
            }
            (*bitD).bitsConsumed = 8 - BITv06_highbit32(lastByte as U32);
        }
    } else {
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as size_t;
        let sb = srcBuffer as *const BYTE;
        // switch with fall-through
        if srcSize == 7 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*sb.wrapping_add(6) as size_t) << (bc_size * 8 - 16),
            );
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*sb.wrapping_add(5) as size_t) << (bc_size * 8 - 24),
            );
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*sb.wrapping_add(4) as size_t) << (bc_size * 8 - 32),
            );
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.wrapping_add(3) as size_t) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.wrapping_add(2) as size_t) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*sb.wrapping_add(1) as size_t) << 8);
        }
        {
            let lastByte: BYTE = *(srcBuffer as *const BYTE).wrapping_add(srcSize - 1);
            if lastByte == 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            (*bitD).bitsConsumed = 8 - BITv06_highbit32(lastByte as U32);
        }
        (*bitD).bitsConsumed += ((bc_size - srcSize) * 8) as U32;
    }

    srcSize
}

#[inline]
pub unsafe fn BITv06_lookBits(bitD: *const BITv06_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() * 8 - 1) as U32;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask - nbBits) & bitMask)
}

#[inline]
pub unsafe fn BITv06_lookBitsFast(bitD: *const BITv06_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() * 8 - 1) as U32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> (((bitMask + 1) - nbBits) & bitMask)
}

#[inline]
pub unsafe fn BITv06_skipBits(bitD: *mut BITv06_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
pub unsafe fn BITv06_readBits(bitD: *mut BITv06_DStream_t, nbBits: U32) -> size_t {
    let value = BITv06_lookBits(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

#[inline]
pub unsafe fn BITv06_readBitsFast(bitD: *mut BITv06_DStream_t, nbBits: U32) -> size_t {
    let value = BITv06_lookBitsFast(bitD, nbBits);
    BITv06_skipBits(bitD, nbBits);
    value
}

#[inline]
pub unsafe fn BITv06_reloadDStream(bitD: *mut BITv06_DStream_t) -> BITv06_DStream_status {
    let bc_size = core::mem::size_of::<size_t>();
    if (*bitD).bitsConsumed > (bc_size * 8) as U32 {
        /* should never happen */
        return BITv06_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(bc_size) {
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BITv06_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (bc_size * 8) as U32 {
            return BITv06_DStream_endOfBuffer;
        }
        return BITv06_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv06_DStream_status = BITv06_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32; /* ptr > start */
            result = BITv06_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline]
pub unsafe fn BITv06_endOfDStream(DStream: *const BITv06_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<size_t>() * 8) as U32)) as u32
}

// ============================================================================
// FSEv06
// ============================================================================
pub type FSEv06_DTable = u32;

const FSEv06_MAX_MEMORY_USAGE: u32 = 14;
const FSEv06_MAX_SYMBOL_VALUE: u32 = 255;
const FSEv06_MAX_TABLELOG: u32 = FSEv06_MAX_MEMORY_USAGE - 2;
const FSEv06_MIN_TABLELOG: u32 = 5;
const FSEv06_TABLELOG_ABSOLUTE_MAX: u32 = 15;

#[inline]
const fn FSEv06_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

#[inline]
unsafe fn FSEv06_TABLESTEP(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv06_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
} /* sizeof U32 */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv06_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
} /* size == U32 */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv06_DState_t {
    pub state: size_t,
    pub table: *const c_void,
}

#[inline]
unsafe fn FSEv06_initDState(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
    dt: *const FSEv06_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv06_DTableHeader;
    (*DStatePtr).state = BITv06_readBits(bitD, (*DTableH).tableLog as u32);
    BITv06_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

#[inline]
unsafe fn FSEv06_peekSymbol(DStatePtr: *const FSEv06_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).wrapping_add((*DStatePtr).state);
    DInfo.symbol
}

#[inline]
unsafe fn FSEv06_updateState(DStatePtr: *mut FSEv06_DState_t, bitD: *mut BITv06_DStream_t) {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let lowBits = BITv06_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
}

#[inline]
unsafe fn FSEv06_decodeSymbol(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv06_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

#[inline]
unsafe fn FSEv06_decodeSymbolFast(
    DStatePtr: *mut FSEv06_DState_t,
    bitD: *mut BITv06_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv06_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv06_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

// unsigned FSEv06_isError(size_t code) { return ERR_isError(code); }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

#[inline]
unsafe fn HUFv06_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

// static short FSEv06_abs(short a) { return a<0 ? -a : a; }
#[inline]
unsafe fn FSEv06_abs(a: i16) -> i16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_readNCount(
    normalizedCounter: *mut i16,
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
    nbBits = ((bitStream & 0xF) + FSEv06_MIN_TABLELOG) as i32; /* extract tableLog */
    if nbBits > FSEv06_TABLELOG_ABSOLUTE_MAX as i32 {
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
                if ip < iend.wrapping_sub(5) {
                    ip = ip.wrapping_add(2);
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
                *normalizedCounter.wrapping_add(charnum as usize) = 0;
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
            let max: i16 = ((2 * threshold - 1) - remaining) as i16;
            let mut count: i16;

            if (bitStream & (threshold - 1) as U32) < max as U32 {
                count = (bitStream & (threshold - 1) as U32) as i16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold - 1) as U32) as i16;
                if count >= threshold as i16 {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1; /* extra accuracy */
            remaining -= FSEv06_abs(count) as i32;
            *normalizedCounter.wrapping_add(charnum as usize) = count;
            charnum += 1;
            previous0 = (count == 0) as i32;
            while remaining < threshold {
                nbBits -= 1;
                threshold >>= 1;
            }

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_createDTable(mut tableLog: u32) -> *mut FSEv06_DTable {
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
    maxSymbolValue: u32,
    tableLog: u32,
) -> size_t {
    let tdPtr = dt.wrapping_add(1) as *mut c_void; /* because *dt is unsigned, 32-bits aligned on 32-bits */
    let tableDecode = tdPtr as *mut FSEv06_decode_t;
    let mut symbolNext = [0u16; (FSEv06_MAX_SYMBOL_VALUE + 1) as usize];

    let maxSV1: U32 = maxSymbolValue + 1;
    let tableSize: U32 = 1 << tableLog;
    let mut highThreshold: U32 = tableSize - 1;

    /* Sanity Checks */
    if maxSymbolValue > FSEv06_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSEv06_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    {
        let mut DTableH = FSEv06_DTableHeader {
            tableLog: tableLog as U16,
            fastMode: 1,
        };
        {
            let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let nc = *normalizedCounter.wrapping_add(s as usize);
                if nc == -1 {
                    (*tableDecode.wrapping_add(highThreshold as usize)).symbol = s as BYTE;
                    highThreshold = highThreshold.wrapping_sub(1);
                    symbolNext[s as usize] = 1;
                } else {
                    if nc >= largeLimit {
                        DTableH.fastMode = 0;
                    }
                    symbolNext[s as usize] = nc as u16;
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

    /* Spread symbols */
    {
        let tableMask: U32 = tableSize - 1;
        let step: U32 = FSEv06_TABLESTEP(tableSize);
        let mut position: U32 = 0;
        let mut s: U32 = 0;
        while s < maxSV1 {
            let nc = *normalizedCounter.wrapping_add(s as usize);
            let mut i: i32 = 0;
            while i < nc as i32 {
                (*tableDecode.wrapping_add(position as usize)).symbol = s as BYTE;
                position = (position + step) & tableMask;
                while position > highThreshold {
                    position = (position + step) & tableMask; /* lowprob area */
                }
                i += 1;
            }
            s += 1;
        }

        if position != 0 {
            return ERROR(ZSTD_error_GENERIC); /* position must reach all cells once */
        }
    }

    /* Build Decoding table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol: BYTE = (*tableDecode.wrapping_add(u as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = nextState.wrapping_add(1);
            (*tableDecode.wrapping_add(u as usize)).nbBits =
                (tableLog - BITv06_highbit32(nextState as U32)) as BYTE;
            (*tableDecode.wrapping_add(u as usize)).newState =
                (((nextState as U32) << (*tableDecode.wrapping_add(u as usize)).nbBits as U32)
                    .wrapping_sub(tableSize)) as U16;
            u += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_buildDTable_rle(dt: *mut FSEv06_DTable, symbolValue: BYTE) -> size_t {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv06_DTableHeader;
    let dPtr = dt.wrapping_add(1) as *mut c_void;
    let cell = dPtr as *mut FSEv06_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_buildDTable_raw(dt: *mut FSEv06_DTable, nbBits: u32) -> size_t {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv06_DTableHeader;
    let dPtr = dt.wrapping_add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv06_decode_t;
    let tableSize: u32 = 1 << nbBits;
    let tableMask: u32 = tableSize - 1;
    let maxSV1: u32 = tableMask + 1;

    /* Sanity checks */
    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC); /* min size */
    }

    /* Build Decoding Table */
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    let mut s: u32 = 0;
    while s < maxSV1 {
        (*dinfo.wrapping_add(s as usize)).newState = 0;
        (*dinfo.wrapping_add(s as usize)).symbol = s as BYTE;
        (*dinfo.wrapping_add(s as usize)).nbBits = nbBits as BYTE;
        s += 1;
    }

    0
}

unsafe fn FSEv06_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSEv06_DTable,
    fast: u32,
) -> size_t {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.wrapping_add(maxDstSize);
    let olimit = omax.wrapping_sub(3);

    let mut bitD = BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };
    let mut state1 = FSEv06_DState_t {
        state: 0,
        table: core::ptr::null(),
    };
    let mut state2 = FSEv06_DState_t {
        state: 0,
        table: core::ptr::null(),
    };

    /* Init */
    {
        let errorCode = BITv06_initDStream(&mut bitD, cSrc, cSrcSize);
        if FSEv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv06_initDState(&mut state1, &mut bitD, dt);
    FSEv06_initDState(&mut state2, &mut bitD, dt);

    macro_rules! GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSEv06_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSEv06_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    let bc_bits = core::mem::size_of::<size_t>() * 8;

    /* 4 symbols per loop */
    while (BITv06_reloadDStream(&mut bitD) == BITv06_DStream_unfinished) && (op < olimit) {
        *op.wrapping_add(0) = GETSYMBOL!(&mut state1);

        if (FSEv06_MAX_TABLELOG * 2 + 7) as usize > bc_bits {
            BITv06_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(1) = GETSYMBOL!(&mut state2);

        if (FSEv06_MAX_TABLELOG * 4 + 7) as usize > bc_bits {
            if BITv06_reloadDStream(&mut bitD) > BITv06_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.wrapping_add(2) = GETSYMBOL!(&mut state1);

        if (FSEv06_MAX_TABLELOG * 2 + 7) as usize > bc_bits {
            BITv06_reloadDStream(&mut bitD);
        }

        *op.wrapping_add(3) = GETSYMBOL!(&mut state2);

        op = op.wrapping_add(4);
    }

    /* tail */
    loop {
        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = GETSYMBOL!(&mut state1);
        op = op.wrapping_add(1);

        if BITv06_reloadDStream(&mut bitD) == BITv06_DStream_overflow {
            *op = GETSYMBOL!(&mut state2);
            op = op.wrapping_add(1);
            break;
        }

        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        *op = GETSYMBOL!(&mut state2);
        op = op.wrapping_add(1);

        if BITv06_reloadDStream(&mut bitD) == BITv06_DStream_overflow {
            *op = GETSYMBOL!(&mut state1);
            op = op.wrapping_add(1);
            break;
        }
    }

    op.offset_from(ostart) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSEv06_DTable,
) -> size_t {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv06_DTableHeader;
    let fastMode: U32 = (*DTableH).fastMode as U32;

    if fastMode != 0 {
        return FSEv06_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSEv06_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

// DTable_max_t
type DTable_max_t = [U32; FSEv06_DTABLE_SIZE_U32(FSEv06_MAX_TABLELOG)];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv06_decompress(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting = [0i16; (FSEv06_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt: DTable_max_t = [0u32; FSEv06_DTABLE_SIZE_U32(FSEv06_MAX_TABLELOG)];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSEv06_MAX_SYMBOL_VALUE;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong); /* too small input size */
    }

    /* normal FSE decoding mode */
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
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        ip = ip.wrapping_add(NCountLength);
        cSrcSize -= NCountLength;
    }

    {
        let errorCode =
            FSEv06_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
        if FSEv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    FSEv06_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

// ============================================================================
// HUFv06
// ============================================================================
const HUFv06_ABSOLUTEMAX_TABLELOG: u32 = 16;
const HUFv06_MAX_TABLELOG: u32 = 12;
const HUFv06_MAX_SYMBOL_VALUE: u32 = 255;

#[inline]
const fn HUFv06_DTABLE_SIZE(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

// MEM_STATIC size_t HUFv06_readStats(...)
unsafe fn HUFv06_readStats(
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
    let mut oSize: size_t;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.wrapping_add(0) as size_t;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            static L: [U32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
            oSize = L[iSize - 242] as size_t;
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
                while (n as size_t) < oSize {
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
        while (n as size_t) < oSize {
            let w = *huffWeight.wrapping_add(n as usize);
            if w as u32 >= HUFv06_ABSOLUTEMAX_TABLELOG {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *rankStats.wrapping_add(w as usize) += 1;
            weightTotal += (1u32 << w) >> 1;
            n += 1;
        }
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    {
        let tableLog: U32 = BITv06_highbit32(weightTotal) + 1;
        if tableLog > HUFv06_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *tableLogPtr = tableLog;
        {
            let total: U32 = 1 << tableLog;
            let rest: U32 = total - weightTotal;
            let verif: U32 = 1 << BITv06_highbit32(rest);
            let lastWeight: U32 = BITv06_highbit32(rest) + 1;
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected); /* last value must be a clean power of 2 */
            }
            *huffWeight.wrapping_add(oSize) = lastWeight as BYTE;
            *rankStats.wrapping_add(lastWeight as usize) += 1;
        }
    }

    /* check tree construction validity */
    if (*rankStats.wrapping_add(1) < 2) || (*rankStats.wrapping_add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

// typedef struct { BYTE byte; BYTE nbBits; } HUFv06_DEltX2;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUFv06_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
}

// typedef struct { U16 sequence; BYTE nbBits; BYTE length; } HUFv06_DEltX4;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUFv06_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
}

// typedef struct { BYTE symbol; BYTE weight; } sortedSymbol_t;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
    pub weight: BYTE,
}

// ---- single-symbol decoding (X2) ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_readDTableX2(
    DTable: *mut U16,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut huffWeight = [0u8; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal = [0u32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut tableLog: U32 = 0;
    let iSize: size_t;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.wrapping_add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv06_DEltX2;

    iSize = HUFv06_readStats(
        huffWeight.as_mut_ptr(),
        (HUFv06_MAX_SYMBOL_VALUE + 1) as size_t,
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
    *DTable.wrapping_add(0) = tableLog as U16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n < tableLog + 1 {
        let current = nextRankStart;
        nextRankStart += rankVal[n as usize] << (n - 1);
        rankVal[n as usize] = current;
        n += 1;
    }

    /* fill DTable */
    n = 0;
    while n < nbSymbols {
        let w = huffWeight[n as usize] as U32;
        let length = (1u32 << w) >> 1;
        let mut i: U32;
        let mut d = HUFv06_DEltX2 { byte: 0, nbBits: 0 };
        d.byte = n as BYTE;
        d.nbBits = (tableLog + 1 - w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize] + length {
            *dt.wrapping_add(i as usize) = d;
            i += 1;
        }
        rankVal[w as usize] += length;
        n += 1;
    }

    iSize
}

unsafe fn HUFv06_decodeSymbolX2(
    Dstream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BITv06_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c = (*dt.wrapping_add(val)).byte;
    BITv06_skipBits(Dstream, (*dt.wrapping_add(val)).nbBits as U32);
    c
}

macro_rules! HUF_DECODE_SYMBOLX2_0 {
    ($ptr:expr, $dstream:expr, $dt:expr, $dtlog:expr) => {{
        *$ptr = HUFv06_decodeSymbolX2($dstream, $dt, $dtlog);
        $ptr = $ptr.wrapping_add(1);
    }};
}
macro_rules! HUF_DECODE_SYMBOLX2_1 {
    ($ptr:expr, $dstream:expr, $dt:expr, $dtlog:expr) => {{
        if MEM_64bits() != 0 || (HUFv06_MAX_TABLELOG <= 12) {
            HUF_DECODE_SYMBOLX2_0!($ptr, $dstream, $dt, $dtlog);
        }
    }};
}
macro_rules! HUF_DECODE_SYMBOLX2_2 {
    ($ptr:expr, $dstream:expr, $dt:expr, $dtlog:expr) => {{
        if MEM_64bits() != 0 {
            HUF_DECODE_SYMBOLX2_0!($ptr, $dstream, $dt, $dtlog);
        }
    }};
}

unsafe fn HUFv06_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart = p;

    /* up to 4 symbols at a time */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(4))
    {
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished) && (p < pEnd) {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
    }

    pEnd.offset_from(pStart) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X2_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const U16,
) -> size_t {
    let op = dst as *mut BYTE;
    let oend = op.wrapping_add(dstSize);
    let dtLog: U32 = *DTable.wrapping_add(0) as U32;
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv06_DEltX2).wrapping_add(1);
    let mut bitD = BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    };

    {
        let errorCode = BITv06_initDStream(&mut bitD, cSrc, cSrcSize);
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
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable = [0u16; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG as u16;
    let mut ip = cSrc as *const BYTE;

    let errorCode = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    HUFv06_decompress1X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X2_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const U16,
) -> size_t {
    /* Check */
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv06_DEltX2).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0) as U32;
        let mut errorCode: size_t;

        let mut bitD1 = new_dstream();
        let mut bitD2 = new_dstream();
        let mut bitD3 = new_dstream();
        let mut bitD4 = new_dstream();
        let length1 = MEM_readLE16(istart as *const c_void) as size_t;
        let length2 = MEM_readLE16(istart.wrapping_add(2) as *const c_void) as size_t;
        let length3 = MEM_readLE16(istart.wrapping_add(4) as *const c_void) as size_t;
        let length4: size_t;
        let istart1 = istart.wrapping_add(6); /* jumpTable */
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

        length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
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

        /* 16-32 symbols per loop */
        endSignal = BITv06_reloadDStream(&mut bitD1)
            | BITv06_reloadDStream(&mut bitD2)
            | BITv06_reloadDStream(&mut bitD3)
            | BITv06_reloadDStream(&mut bitD4);
        while (endSignal == BITv06_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
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

        dstSize
    }
}

#[inline]
fn new_dstream() -> BITv06_DStream_t {
    BITv06_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X2(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable = [0u16; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG as u16;
    let mut ip = cSrc as *const BYTE;

    let errorCode = HUFv06_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    HUFv06_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// ---- double-symbols decoding (X4) ----
unsafe fn HUFv06_fillDTableX4Level2(
    DTable: *mut HUFv06_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: i32,
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
    let mut rankVal = [0u32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];

    /* get pre-calculated rankVal */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[u32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let skipSize = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1;
        let mut i: U32 = 0;
        while i < skipSize {
            *DTable.wrapping_add(i as usize) = DElt;
            i += 1;
        }
    }

    /* fill DTable */
    {
        let mut s: U32 = 0;
        while s < sortedListSize {
            let symbol: U32 = (*sortedSymbols.wrapping_add(s as usize)).symbol as U32;
            let weight: U32 = (*sortedSymbols.wrapping_add(s as usize)).weight as U32;
            let nbBits: U32 = nbBitsBaseline - weight;
            let length: U32 = 1 << (sizeLog - nbBits);
            let start: U32 = rankVal[weight as usize];
            let mut i: U32 = start;
            let end: U32 = start + length;

            MEM_writeLE16(
                &mut DElt.sequence as *mut U16 as *mut c_void,
                (baseSeq as U32 + (symbol << 8)) as U16,
            );
            DElt.nbBits = (nbBits + consumed) as BYTE;
            DElt.length = 2;
            loop {
                *DTable.wrapping_add(i as usize) = DElt;
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

// typedef U32 rankVal_t[HUFv06_ABSOLUTEMAX_TABLELOG][HUFv06_ABSOLUTEMAX_TABLELOG + 1];
type rankVal_t = [[U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];
    HUFv06_ABSOLUTEMAX_TABLELOG as usize];

unsafe fn HUFv06_fillDTableX4(
    DTable: *mut HUFv06_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const rankVal_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal = [0u32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: i32 = (nbBitsBaseline as i32) - (targetLog as i32); /* scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline - maxWeight;
    let mut s: U32;

    // rankValOrigin here corresponds to rankVal[0] in the caller (a *const [U32;N])
    let rankValOrigin0 = rankValOrigin as *const [U32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin0 as *const c_void,
        core::mem::size_of::<[u32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.wrapping_add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.wrapping_add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline - weight;
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1 << (targetLog - nbBits);

        if targetLog - nbBits >= minBits {
            /* enough room for a second symbol */
            let sortedRank: U32;
            let mut minWeight: i32 = nbBits as i32 + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.wrapping_add(minWeight as usize);
            HUFv06_fillDTableX4Level2(
                DTable.wrapping_add(start as usize),
                targetLog - nbBits,
                nbBits,
                (*rankValOrigin0.wrapping_add(nbBits as usize)).as_ptr(),
                minWeight,
                sortedList.wrapping_add(sortedRank as usize),
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
                let mut u: U32 = start;
                let end: U32 = start + length;
                while u < end {
                    *DTable.wrapping_add(u as usize) = DElt;
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
    DTable: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut weightList = [0u8; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol = [sortedSymbol_t {
        symbol: 0,
        weight: 0,
    }; (HUFv06_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats = [0u32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0 = [0u32; (HUFv06_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let mut rankVal: rankVal_t = [[0u32; (HUFv06_ABSOLUTEMAX_TABLELOG + 1) as usize];
        HUFv06_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.wrapping_add(0);
    let iSize: size_t;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUFv06_DEltX4).wrapping_add(1);

    if memLog > HUFv06_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUFv06_readStats(
        weightList.as_mut_ptr(),
        (HUFv06_MAX_SYMBOL_VALUE + 1) as size_t,
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

    // rankStart = rankStart0 + 1
    // We access rankStart[w] == rankStart0[w+1]; rankStart[0] == rankStart0[1].
    macro_rules! rankStart {
        ($i:expr) => {
            rankStart0[($i) as usize + 1]
        };
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW -= 1;
    }

    /* Get start index of each weight */
    {
        let mut w: U32 = 1;
        let mut nextRankStart: U32 = 0;
        while w < maxW + 1 {
            let current = nextRankStart;
            nextRankStart += rankStats[w as usize];
            rankStart!(w) = current;
            w += 1;
        }
        rankStart!(0) = nextRankStart; /* put all 0w symbols at the end of sorted list */
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = rankStart!(w);
            rankStart!(w) = r + 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        rankStart!(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        {
            let rescale: i32 = (memLog as i32 - tableLog as i32) - 1; /* tableLog <= memLog */
            let mut nextRankVal: U32 = 0;
            let mut w: U32 = 1;
            while w < maxW + 1 {
                let current = nextRankVal;
                nextRankVal += rankStats[w as usize] << ((w as i32 + rescale) as U32);
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
        &rankVal as *const rankVal_t,
        maxW,
        tableLog + 1,
    );

    iSize
}

unsafe fn HUFv06_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv06_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 2);
    BITv06_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    (*dt.wrapping_add(val)).length as U32
}

unsafe fn HUFv06_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv06_DStream_t,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv06_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 1);
    if (*dt.wrapping_add(val)).length == 1 {
        BITv06_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    } else {
        let bc_bits = (core::mem::size_of::<size_t>() * 8) as U32;
        if (*DStream).bitsConsumed < bc_bits {
            BITv06_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > bc_bits {
                (*DStream).bitsConsumed = bc_bits; /* ugly hack; works only because it's the last symbol */
            }
        }
    }
    1
}

macro_rules! HUF_DECODE_SYMBOLX4_0 {
    ($ptr:expr, $dstream:expr, $dt:expr, $dtlog:expr) => {{
        $ptr = $ptr.wrapping_add(HUFv06_decodeSymbolX4(
            $ptr as *mut c_void,
            $dstream,
            $dt,
            $dtlog,
        ) as usize);
    }};
}
macro_rules! HUF_DECODE_SYMBOLX4_1 {
    ($ptr:expr, $dstream:expr, $dt:expr, $dtlog:expr) => {{
        if MEM_64bits() != 0 || (HUFv06_MAX_TABLELOG <= 12) {
            HUF_DECODE_SYMBOLX4_0!($ptr, $dstream, $dt, $dtlog);
        }
    }};
}
macro_rules! HUF_DECODE_SYMBOLX4_2 {
    ($ptr:expr, $dstream:expr, $dt:expr, $dtlog:expr) => {{
        if MEM_64bits() != 0 {
            HUF_DECODE_SYMBOLX4_0!($ptr, $dstream, $dt, $dtlog);
        }
    }};
}

unsafe fn HUFv06_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv06_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv06_DEltX4,
    dtLog: U32,
) -> size_t {
    let pStart = p;

    /* up to 8 symbols at a time */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p < pEnd.wrapping_sub(7))
    {
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_1!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_2!(p, bitDPtr, dt, dtLog);
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    /* closer to the end */
    while (BITv06_reloadDStream(bitDPtr) == BITv06_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(2))
    {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog);
    }

    while p <= pEnd.wrapping_sub(2) {
        HUF_DECODE_SYMBOLX4_0!(p, bitDPtr, dt, dtLog); /* no need to reload */
    }

    if p < pEnd {
        p = p.wrapping_add(HUFv06_decodeLastSymbolX4(
            p as *mut c_void,
            bitDPtr,
            dt,
            dtLog,
        ) as usize);
    }

    p.offset_from(pStart) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X4_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const U32,
) -> size_t {
    let istart = cSrc as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.wrapping_add(dstSize);

    let dtLog: U32 = *DTable.wrapping_add(0);
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv06_DEltX4).wrapping_add(1);

    let mut bitD = new_dstream();
    {
        let errorCode = BITv06_initDStream(&mut bitD, istart as *const c_void, cSrcSize);
        if HUFv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv06_decodeStreamX4(ostart, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv06_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress1X4(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable = [0u32; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv06_decompress1X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X4_usingDTable(
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
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv06_DEltX4).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0);
        let mut errorCode: size_t;

        let mut bitD1 = new_dstream();
        let mut bitD2 = new_dstream();
        let mut bitD3 = new_dstream();
        let mut bitD4 = new_dstream();
        let length1 = MEM_readLE16(istart as *const c_void) as size_t;
        let length2 = MEM_readLE16(istart.wrapping_add(2) as *const c_void) as size_t;
        let length3 = MEM_readLE16(istart.wrapping_add(4) as *const c_void) as size_t;
        let length4: size_t;
        let istart1 = istart.wrapping_add(6); /* jumpTable */
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

        length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
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

        /* 16-32 symbols per loop */
        endSignal = BITv06_reloadDStream(&mut bitD1)
            | BITv06_reloadDStream(&mut bitD2)
            | BITv06_reloadDStream(&mut bitD3)
            | BITv06_reloadDStream(&mut bitD4);
        while (endSignal == BITv06_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
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

        dstSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress4X4(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable = [0u32; HUFv06_DTABLE_SIZE(HUFv06_MAX_TABLELOG)];
    DTable[0] = HUFv06_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv06_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv06_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv06_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// ---- Generic decompression selector ----
#[repr(C)]
#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
}

static algoTime: [[algo_time_t; 3]; 16] = [
    algo_row(0, 0, 1, 1, 2, 2),
    algo_row(0, 0, 1, 1, 2, 2),
    algo_row(38, 130, 1313, 74, 2151, 38),
    algo_row(448, 128, 1353, 74, 2238, 41),
    algo_row(556, 128, 1353, 74, 2238, 47),
    algo_row(714, 128, 1418, 74, 2436, 53),
    algo_row(883, 128, 1437, 74, 2464, 61),
    algo_row(897, 128, 1515, 75, 2622, 68),
    algo_row(926, 128, 1613, 75, 2730, 75),
    algo_row(947, 128, 1729, 77, 3359, 77),
    algo_row(1107, 128, 2083, 81, 4006, 84),
    algo_row(1177, 128, 2379, 87, 4785, 88),
    algo_row(1242, 128, 2415, 93, 5155, 84),
    algo_row(1349, 128, 2644, 106, 5260, 106),
    algo_row(1455, 128, 2422, 124, 4174, 124),
    algo_row(722, 128, 1891, 145, 1936, 146),
];

const fn algo_row(a: U32, b: U32, c: U32, d: U32, e: U32, f: U32) -> [algo_time_t; 3] {
    [
        algo_time_t {
            tableTime: a,
            decode256Time: b,
        },
        algo_time_t {
            tableTime: c,
            decode256Time: d,
        },
        algo_time_t {
            tableTime: e,
            decode256Time: f,
        },
    ]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv06_decompress(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    let mut Dtime = [0u32; 3]; /* decompression time estimation */

    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected); /* invalid */
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize; /* not compressed */
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as i32, dstSize);
        return dstSize; /* RLE */
    }

    /* decoder timing evaluation */
    {
        let Q: U32 = (cSrcSize * 16 / dstSize) as U32; /* Q < 16 */
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
        if algoNb == 0 {
            HUFv06_decompress4X2(dst, dstSize, cSrc, cSrcSize)
        } else {
            HUFv06_decompress4X4(dst, dstSize, cSrc, cSrcSize)
        }
    }
}

// ============================================================================
// ZSTDv06 error management
// ============================================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_isError(errorCode: size_t) -> u32 {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_getErrorName(errorCode: size_t) -> *const c_char {
    ERR_getErrorName(errorCode)
}

// static void ZSTDv06_copy4(void* dst, const void* src) { memcpy(dst, src, 4); }
#[inline]
unsafe fn ZSTDv06_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

// ============================================================================
// ZSTDv06 frameParams and DCtx
// ============================================================================
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTDv06_frameParams {
    pub frameContentSize: u64,
    pub windowLog: u32,
}

// ZSTDds stages
pub type ZSTDv06_dStage = u32;
const ZSTDds_getFrameHeaderSize: ZSTDv06_dStage = 0;
const ZSTDds_decodeFrameHeader: ZSTDv06_dStage = 1;
const ZSTDds_decodeBlockHeader: ZSTDv06_dStage = 2;
const ZSTDds_decompressBlock: ZSTDv06_dStage = 3;

#[repr(C)]
pub struct ZSTDv06_DCtx {
    pub LLTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(LLFSELog)],
    pub OffTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(OffFSELog)],
    pub MLTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(MLFSELog)],
    pub hufTableX4: [u32; HUFv06_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: size_t,
    pub headerSize: size_t,
    pub fParams: ZSTDv06_frameParams,
    pub bType: blockType_t,
    pub stage: ZSTDv06_dStage,
    pub flagRepeatTable: U32,
    pub litPtr: *const BYTE,
    pub litSize: size_t,
    pub litBuffer: [BYTE; ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_sizeofDCtx() -> size_t {
    core::mem::size_of::<ZSTDv06_DCtx>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBegin(dctx: *mut ZSTDv06_DCtx) -> size_t {
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
pub unsafe extern "C" fn ZSTDv06_freeDCtx(dctx: *mut ZSTDv06_DCtx) -> size_t {
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

// ---- frame header ----
unsafe fn ZSTDv06_frameHeaderSize(src: *const c_void, srcSize: size_t) -> size_t {
    if srcSize < ZSTDv06_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    let fcsId: U32 = (*(src as *const BYTE).wrapping_add(4) >> 6) as U32;
    ZSTDv06_frameHeaderSize_min + ZSTDv06_fcs_fieldSize[fcsId as usize]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_getFrameParams(
    fparamsPtr: *mut ZSTDv06_frameParams,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let ip = src as *const BYTE;

    if srcSize < ZSTDv06_frameHeaderSize_min {
        return ZSTDv06_frameHeaderSize_min;
    }
    if MEM_readLE32(src) != ZSTDv06_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }

    /* ensure there is enough srcSize to fully read/decode frame header */
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
        let frameDesc: BYTE = *ip.wrapping_add(4);
        (*fparamsPtr).windowLog = (frameDesc as U32 & 0xF) + ZSTDv06_WINDOWLOG_ABSOLUTEMIN;
        if (frameDesc & 0x20) != 0 {
            return ERROR(ZSTD_error_frameParameter_unsupported); /* reserved 1 bit */
        }
        match frameDesc >> 6 {
            /* fcsId */
            1 => (*fparamsPtr).frameContentSize = *ip.wrapping_add(5) as u64,
            2 => {
                (*fparamsPtr).frameContentSize =
                    MEM_readLE16(ip.wrapping_add(5) as *const c_void) as u64 + 256
            }
            3 => {
                (*fparamsPtr).frameContentSize =
                    MEM_readLE64(ip.wrapping_add(5) as *const c_void)
            }
            _ => (*fparamsPtr).frameContentSize = 0, /* case 0 / default */
        }
    }
    0
}

unsafe fn ZSTDv06_decodeFrameHeader(
    zc: *mut ZSTDv06_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let result = ZSTDv06_getFrameParams(&mut (*zc).fParams, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).fParams.windowLog > 25) {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    result
}

#[repr(C)]
#[derive(Clone, Copy)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

unsafe fn ZSTDv06_getcBlockSize(
    src: *const c_void,
    srcSize: size_t,
    bpPtr: *mut blockProperties_t,
) -> size_t {
    let in_: *const BYTE = src as *const BYTE;
    let cSize: U32;

    if srcSize < ZSTDv06_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (*bpPtr).blockType = (*in_ >> 6) as blockType_t;
    cSize = *in_.wrapping_add(2) as U32
        + ((*in_.wrapping_add(1) as U32) << 8)
        + (((*in_.wrapping_add(0) as U32) & 7) << 16);
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
    cSize as size_t
}

unsafe fn ZSTDv06_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    if dst.is_null() {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    memcpy(dst, src, srcSize);
    srcSize
}

// ---- literals block ----
unsafe fn ZSTDv06_decodeLiteralsBlock(
    dctx: *mut ZSTDv06_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart.wrapping_add(0) >> 6) as U32 {
        x if x == IS_HUF => {
            let litSize: size_t;
            let litCSize: size_t;
            let mut singleStream: size_t = 0;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) >> 4) & 3) as U32;
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            match lhSize {
                2 => {
                    /* 2 - 2 - 14 - 14 */
                    lhSize = 4;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 10)
                        + ((*istart.wrapping_add(1) as size_t) << 2)
                        + ((*istart.wrapping_add(2) as size_t) >> 6);
                    litCSize = (((*istart.wrapping_add(2) as size_t) & 63) << 8)
                        + *istart.wrapping_add(3) as size_t;
                }
                3 => {
                    /* 2 - 2 - 18 - 18 */
                    lhSize = 5;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 14)
                        + ((*istart.wrapping_add(1) as size_t) << 6)
                        + ((*istart.wrapping_add(2) as size_t) >> 2);
                    litCSize = (((*istart.wrapping_add(2) as size_t) & 3) << 16)
                        + ((*istart.wrapping_add(3) as size_t) << 8)
                        + *istart.wrapping_add(4) as size_t;
                }
                _ => {
                    /* case 0, 1, default : 2 - 2 - 10 - 10 */
                    lhSize = 3;
                    singleStream = (*istart.wrapping_add(0) & 16) as size_t;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 6)
                        + ((*istart.wrapping_add(1) as size_t) >> 2);
                    litCSize = (((*istart.wrapping_add(1) as size_t) & 3) << 8)
                        + *istart.wrapping_add(2) as size_t;
                }
            }
            if litSize > ZSTDv06_BLOCKSIZE_MAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if litCSize + lhSize as size_t > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            let hufResult = if singleStream != 0 {
                HUFv06_decompress1X2(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            } else {
                HUFv06_decompress(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            };
            if HUFv06_isError(hufResult) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            litCSize + lhSize as size_t
        }
        x if x == IS_PCH => {
            let litSize: size_t;
            let litCSize: size_t;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) >> 4) & 3) as U32;
            if lhSize != 1 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if (*dctx).flagRepeatTable == 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }

            /* 2 - 2 - 10 - 10 */
            lhSize = 3;
            litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 6)
                + ((*istart.wrapping_add(1) as size_t) >> 2);
            litCSize = (((*istart.wrapping_add(1) as size_t) & 3) << 8)
                + *istart.wrapping_add(2) as size_t;
            if litCSize + lhSize as size_t > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            {
                let errorCode = HUFv06_decompress1X4_usingDTable(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                    (*dctx).hufTableX4.as_ptr(),
                );
                if HUFv06_isError(errorCode) != 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            litCSize + lhSize as size_t
        }
        x if x == IS_RAW => {
            let litSize: size_t;
            let lhSize: U32 = ((*istart.wrapping_add(0) >> 4) & 3) as U32;
            let mut lhSizeV = lhSize;
            match lhSize {
                2 => {
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 8)
                        + *istart.wrapping_add(1) as size_t;
                }
                3 => {
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 16)
                        + ((*istart.wrapping_add(1) as size_t) << 8)
                        + *istart.wrapping_add(2) as size_t;
                }
                _ => {
                    /* case 0, 1, default */
                    lhSizeV = 1;
                    litSize = (*istart.wrapping_add(0) as size_t) & 31;
                }
            }
            let lhSize = lhSizeV;

            if lhSize as size_t + litSize + WILDCOPY_OVERLENGTH > srcSize {
                /* risk reading beyond src buffer with wildcopy */
                if litSize + lhSize as size_t > srcSize {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                memcpy(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litSize,
                );
                (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
                (*dctx).litSize = litSize;
                memset(
                    (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                    0,
                    WILDCOPY_OVERLENGTH,
                );
                return lhSize as size_t + litSize;
            }
            /* direct reference into compressed stream */
            (*dctx).litPtr = istart.wrapping_add(lhSize as usize);
            (*dctx).litSize = litSize;
            lhSize as size_t + litSize
        }
        x if x == IS_RLE => {
            let litSize: size_t;
            let lhSize: U32 = ((*istart.wrapping_add(0) >> 4) & 3) as U32;
            let mut lhSizeV = lhSize;
            match lhSize {
                2 => {
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 8)
                        + *istart.wrapping_add(1) as size_t;
                }
                3 => {
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 16)
                        + ((*istart.wrapping_add(1) as size_t) << 8)
                        + *istart.wrapping_add(2) as size_t;
                    if srcSize < 4 {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                }
                _ => {
                    /* case 0, 1, default */
                    lhSizeV = 1;
                    litSize = (*istart.wrapping_add(0) as size_t) & 31;
                }
            }
            let lhSize = lhSizeV;
            if litSize > ZSTDv06_BLOCKSIZE_MAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.wrapping_add(lhSize as usize) as i32,
                litSize + WILDCOPY_OVERLENGTH,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            lhSize as size_t + 1
        }
        _ => ERROR(ZSTD_error_corruption_detected), /* impossible */
    }
}

// ---- sequences ----
unsafe fn ZSTDv06_buildSeqTable(
    DTable: *mut FSEv06_DTable,
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
        x if x == FSEv06_ENCODING_RLE => {
            if srcSize == 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if (*(src as *const BYTE)) as U32 > max {
                return ERROR(ZSTD_error_corruption_detected);
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
                return ERROR(ZSTD_error_corruption_detected);
            }
            0
        }
        _ => {
            /* FSEv06_ENCODING_DYNAMIC and impossible default */
            let mut tableLog: U32 = 0;
            let mut norm = [0i16; (MaxSeq + 1) as usize];
            let mut maxV = max;
            let headerSize = FSEv06_readNCount(
                norm.as_mut_ptr(),
                &mut maxV,
                &mut tableLog,
                src,
                srcSize,
            );
            if FSEv06_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if tableLog > maxLog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            FSEv06_buildDTable(DTable, norm.as_ptr(), maxV, tableLog);
            headerSize
        }
    }
}

unsafe fn ZSTDv06_decodeSeqHeaders(
    nbSeqPtr: *mut i32,
    DTableLL: *mut FSEv06_DTable,
    DTableML: *mut FSEv06_DTable,
    DTableOffb: *mut FSEv06_DTable,
    flagRepeatTable: U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let istart = src as *const BYTE;
    let iend = istart.wrapping_add(srcSize);
    let mut ip = istart;

    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    {
        let mut nbSeq: i32 = *ip as i32;
        ip = ip.wrapping_add(1);
        if nbSeq == 0 {
            *nbSeqPtr = 0;
            return 1;
        }
        if nbSeq > 0x7F {
            if nbSeq == 0xFF {
                if ip.wrapping_add(2) > iend {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = MEM_readLE16(ip as *const c_void) as i32 + LONGNBSEQ;
                ip = ip.wrapping_add(2);
            } else {
                if ip >= iend {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = ((nbSeq - 0x80) << 8) + *ip as i32;
                ip = ip.wrapping_add(1);
            }
        }
        *nbSeqPtr = nbSeq;
    }

    /* FSE table descriptors */
    if ip.wrapping_add(4) > iend {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let LLtype: U32 = (*ip >> 6) as U32;
        let Offtype: U32 = ((*ip >> 4) & 3) as U32;
        let MLtype: U32 = ((*ip >> 2) & 3) as U32;
        ip = ip.wrapping_add(1);

        {
            let bhSize = ZSTDv06_buildSeqTable(
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
            if ZSTDv06_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
        {
            let bhSize = ZSTDv06_buildSeqTable(
                DTableOffb,
                Offtype,
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                flagRepeatTable,
            );
            if ZSTDv06_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
        {
            let bhSize = ZSTDv06_buildSeqTable(
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
            if ZSTDv06_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
    }

    ip.offset_from(istart) as size_t
}

#[repr(C)]
#[derive(Clone, Copy)]
struct seq_t {
    litLength: size_t,
    matchLength: size_t,
    offset: size_t,
}

#[repr(C)]
struct seqState_t {
    DStream: BITv06_DStream_t,
    stateLL: FSEv06_DState_t,
    stateOffb: FSEv06_DState_t,
    stateML: FSEv06_DState_t,
    prevOffset: [size_t; ZSTDv06_REP_INIT],
}

static LL_base: [U32; (MaxLL + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];
static ML_base: [U32; (MaxML + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 34, 36, 38, 40, 44, 48, 56, 64, 80, 96, 0x80, 0x100, 0x200, 0x400,
    0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];
static OF_base: [U32; (MaxOff + 1) as usize] = [
    0, 1, 3, 7, 0xF, 0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF, 0x3FFF, 0x7FFF,
    0xFFFF, 0x1FFFF, 0x3FFFF, 0x7FFFF, 0xFFFFF, 0x1FFFFF, 0x3FFFFF, 0x7FFFFF, 0xFFFFFF, 0x1FFFFFF,
    0x3FFFFFF, /*fake*/ 1, 1,
];

unsafe fn ZSTDv06_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    /* Literal length */
    let llCode: U32 = FSEv06_peekSymbol(&(*seqState).stateLL) as U32;
    let mlCode: U32 = FSEv06_peekSymbol(&(*seqState).stateML) as U32;
    let ofCode: U32 = FSEv06_peekSymbol(&(*seqState).stateOffb) as U32; /* <= maxOff */

    let llBits: U32 = LL_bits[llCode as usize];
    let mlBits: U32 = ML_bits[mlCode as usize];
    let ofBits: U32 = ofCode;
    let totalBits: U32 = llBits + mlBits + ofBits;

    /* sequence */
    {
        let mut offset: size_t;
        if ofCode == 0 {
            offset = 0;
        } else {
            offset = OF_base[ofCode as usize] as size_t
                + BITv06_readBits(&mut (*seqState).DStream, ofBits); /* <= 26 bits */
            if MEM_32bits() != 0 {
                BITv06_reloadDStream(&mut (*seqState).DStream);
            }
        }

        if offset < ZSTDv06_REP_NUM as size_t {
            if llCode == 0 && offset <= 1 {
                offset = 1 - offset;
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
            offset -= ZSTDv06_REP_MOVE as size_t;
            (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
            (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
            (*seqState).prevOffset[0] = offset;
        }
        (*seq).offset = offset;
    }

    (*seq).matchLength = ML_base[mlCode as usize] as size_t
        + MINMATCH
        + (if mlCode > 31 {
            BITv06_readBits(&mut (*seqState).DStream, mlBits)
        } else {
            0
        }); /* <= 16 bits */
    if MEM_32bits() != 0 && (mlBits + llBits > 24) {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }

    (*seq).litLength = LL_base[llCode as usize] as size_t
        + (if llCode > 15 {
            BITv06_readBits(&mut (*seqState).DStream, llBits)
        } else {
            0
        }); /* <= 16 bits */
    if MEM_32bits() != 0
        || (totalBits > 64 - 7 - (LLFSELog + MLFSELog + OffFSELog))
    {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }

    /* ANS state update */
    FSEv06_updateState(&mut (*seqState).stateLL, &mut (*seqState).DStream); /* <= 9 bits */
    FSEv06_updateState(&mut (*seqState).stateML, &mut (*seqState).DStream); /* <= 9 bits */
    if MEM_32bits() != 0 {
        BITv06_reloadDStream(&mut (*seqState).DStream); /* <= 18 bits */
    }
    FSEv06_updateState(&mut (*seqState).stateOffb, &mut (*seqState).DStream); /* <= 8 bits */
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
) -> size_t {
    let oLitEnd = op.wrapping_add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.wrapping_add(sequenceLength); /* risk : address space overflow (32-bits) */
    let oend_8 = oend.wrapping_sub(8);
    let iLitEnd = (*litPtr).wrapping_add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* checks */
    let seqLength = sequence.litLength + sequence.matchLength;

    if seqLength > oend.offset_from(op) as size_t {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as size_t {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if oLitEnd > oend_8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    if oMatchEnd > oend {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* overwrite beyond dst buffer */
    }
    if iLitEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected); /* overRead beyond lit buffer */
    }

    /* copy Literals */
    ZSTDv06_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* copy Match */
    if sequence.offset > oLitEnd.offset_from(base) as size_t {
        /* offset beyond prefix */
        if sequence.offset > oLitEnd.offset_from(vBase) as size_t {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_offset(-(base.offset_from(match_)));
        if match_.wrapping_add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = dictEnd.offset_from(match_) as size_t;
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.wrapping_add(length1);
            sequence.matchLength -= length1;
            match_ = base;
            if op > oend_8 || sequence.matchLength < MINMATCH {
                while op < oMatchEnd {
                    *op = *match_;
                    op = op.wrapping_add(1);
                    match_ = match_.wrapping_add(1);
                }
                return sequenceLength;
            }
        }
    }
    /* Requirement: op <= oend_8 */

    /* match within prefix */
    if sequence.offset < 8 {
        /* close range match, overlap */
        static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
        static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */
        let sub2: i32 = dec64table[sequence.offset];
        *op.wrapping_add(0) = *match_.wrapping_add(0);
        *op.wrapping_add(1) = *match_.wrapping_add(1);
        *op.wrapping_add(2) = *match_.wrapping_add(2);
        *op.wrapping_add(3) = *match_.wrapping_add(3);
        match_ = match_.wrapping_add(dec32table[sequence.offset] as usize);
        ZSTDv06_copy4(op.wrapping_add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.wrapping_offset(-(sub2 as isize));
    } else {
        ZSTDv06_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.wrapping_add(8);
    match_ = match_.wrapping_add(8);

    if oMatchEnd > oend.wrapping_sub(16 - MINMATCH) {
        if op < oend_8 {
            ZSTDv06_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                oend_8.offset_from(op),
            );
            match_ = match_.wrapping_offset(oend_8.offset_from(op));
            op = oend_8;
        }
        while op < oMatchEnd {
            *op = *match_;
            op = op.wrapping_add(1);
            match_ = match_.wrapping_add(1);
        }
    } else {
        ZSTDv06_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize - 8,
        ); /* works even if matchLength < 8 */
    }
    sequenceLength
}

unsafe fn ZSTDv06_decompressSequences(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
) -> size_t {
    let mut ip = seqStart as *const BYTE;
    let iend = ip.wrapping_add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = ostart.wrapping_add(maxDstSize);
    let mut op = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd = litPtr.wrapping_add((*dctx).litSize);
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: i32 = 0;

    /* Build Decoding Tables */
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
        ip = ip.wrapping_add(seqHSize);
        (*dctx).flagRepeatTable = 0;
    }

    /* Regen sequences */
    if nbSeq != 0 {
        let mut sequence = seq_t {
            litLength: 0,
            matchLength: 0,
            offset: 0,
        };
        let mut seqState = seqState_t {
            DStream: new_dstream(),
            stateLL: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateOffb: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateML: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            prevOffset: [0; ZSTDv06_REP_INIT],
        };

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
                iend.offset_from(ip) as size_t,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
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
                op = op.wrapping_add(oneSeqSize);
            }
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
    }

    /* last literal segment */
    {
        let lastLLSize = litEnd.offset_from(litPtr) as size_t;
        if litPtr > litEnd {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op.wrapping_add(lastLLSize) > oend {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if lastLLSize > 0 {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
    }

    op.offset_from(ostart) as size_t
}

unsafe fn ZSTDv06_checkContinuity(dctx: *mut ZSTDv06_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char).wrapping_offset(
            -((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char),
        ) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

unsafe fn ZSTDv06_decompressBlock_internal(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip = src as *const BYTE;

    if srcSize >= ZSTDv06_BLOCKSIZE_MAX {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals sub-block */
    {
        let litCSize = ZSTDv06_decodeLiteralsBlock(dctx, src, srcSize);
        if ZSTDv06_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.wrapping_add(litCSize);
        srcSize -= litCSize;
    }
    ZSTDv06_decompressSequences(dctx, dst, dstCapacity, ip as *const c_void, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBlock(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTDv06_checkContinuity(dctx, dst);
    ZSTDv06_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize)
}

// ---- frame decompression ----
unsafe fn ZSTDv06_decompressFrame(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut ip = src as *const BYTE;
    let iend = ip.wrapping_add(srcSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.wrapping_add(dstCapacity);
    let mut remainingSize = srcSize;
    let mut blockProperties = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };

    if srcSize < ZSTDv06_frameHeaderSize_min + ZSTDv06_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Frame Header */
    {
        let frameHeaderSize = ZSTDv06_frameHeaderSize(src, ZSTDv06_frameHeaderSize_min);
        if ZSTDv06_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if ZSTDv06_decodeFrameHeader(dctx, src, frameHeaderSize) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    /* Loop on each block */
    loop {
        let mut decodedSize: size_t = 0;
        let cBlockSize = ZSTDv06_getcBlockSize(
            ip as *const c_void,
            iend.offset_from(ip) as size_t,
            &mut blockProperties,
        );
        if ZSTDv06_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.wrapping_add(ZSTDv06_blockHeaderSize);
        remainingSize -= ZSTDv06_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                decodedSize = ZSTDv06_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_raw => {
                decodedSize = ZSTDv06_copyRawBlock(
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_rle => {
                return ERROR(ZSTD_error_GENERIC); /* not yet supported */
            }
            x if x == bt_end => {
                /* end of frame */
                if remainingSize != 0 {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC); /* impossible */
            }
        }
        if cBlockSize == 0 {
            break; /* bt_end */
        }

        if ZSTDv06_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.wrapping_add(decodedSize);
        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op.offset_from(ostart) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv06_DCtx,
    refDCtx: *const ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTDv06_copyDCtx(dctx, refDCtx);
    ZSTDv06_checkContinuity(dctx, dst);
    ZSTDv06_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompress_usingDict(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    ZSTDv06_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv06_checkContinuity(dctx, dst);
    ZSTDv06_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressDCtx(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTDv06_decompress_usingDict(dctx, dst, dstCapacity, src, srcSize, core::ptr::null(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompress(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    // ZSTDv06_HEAPMODE == 1
    let regenSize: size_t;
    let dctx = ZSTDv06_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv06_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv06_freeDCtx(dctx);
    regenSize
}

unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut size_t, dBound: *mut u64, ret: size_t) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: size_t,
    cSize: *mut size_t,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: size_t = 0;
    let mut blockProperties = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };

    /* Frame Header */
    {
        let frameHeaderSize = ZSTDv06_frameHeaderSize(src, srcSize);
        if ZSTDv06_isError(frameHeaderSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, frameHeaderSize);
            return;
        }
        if MEM_readLE32(src) != ZSTDv06_MAGICNUMBER {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
            return;
        }
        if srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
    }

    /* Loop on each block */
    loop {
        let cBlockSize =
            ZSTDv06_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv06_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTDv06_blockHeaderSize);
        remainingSize -= ZSTDv06_blockHeaderSize;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break; /* bt_end */
        }

        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = ip.offset_from(src as *const BYTE) as size_t;
    *dBound = (nbBlocks * ZSTDv06_BLOCKSIZE_MAX) as u64;
}

// ---- streaming decompression ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_nextSrcSizeToDecompress(dctx: *mut ZSTDv06_DCtx) -> size_t {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressContinue(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    /* Sanity check */
    if srcSize != (*dctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dstCapacity != 0 {
        ZSTDv06_checkContinuity(dctx, dst);
    }

    // switch with fall-through from getFrameHeaderSize into decodeFrameHeader
    if (*dctx).stage == ZSTDds_getFrameHeaderSize {
        if srcSize != ZSTDv06_frameHeaderSize_min {
            return ERROR(ZSTD_error_srcSize_wrong); /* impossible */
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
        (*dctx).expected = 0; /* not necessary to copy more */
        /* fall-through into ZSTDds_decodeFrameHeader */
        (*dctx).stage = ZSTDds_decodeFrameHeader;
    }

    match (*dctx).stage {
        x if x == ZSTDds_decodeFrameHeader => {
            let result: size_t;
            memcpy(
                (*dctx)
                    .headerBuffer
                    .as_mut_ptr()
                    .wrapping_add(ZSTDv06_frameHeaderSize_min) as *mut c_void,
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
        x if x == ZSTDds_decodeBlockHeader => {
            let mut bp = blockProperties_t {
                blockType: bt_compressed,
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
            let mut rSize: size_t;
            match (*dctx).bType {
                y if y == bt_compressed => {
                    rSize = ZSTDv06_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
                }
                y if y == bt_raw => {
                    rSize = ZSTDv06_copyRawBlock(dst, dstCapacity, src, srcSize);
                }
                y if y == bt_rle => {
                    return ERROR(ZSTD_error_GENERIC); /* not yet handled */
                }
                y if y == bt_end => {
                    rSize = 0;
                }
                _ => {
                    return ERROR(ZSTD_error_GENERIC); /* impossible */
                }
            }
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            (*dctx).expected = ZSTDv06_blockHeaderSize;
            if ZSTDv06_isError(rSize) != 0 {
                return rSize;
            }
            (*dctx).previousDstEnd = (dst as *const c_char).wrapping_add(rSize) as *const c_void;
            rSize
        }
        _ => ERROR(ZSTD_error_GENERIC), /* impossible */
    }
}

unsafe fn ZSTDv06_refDictContent(dctx: *mut ZSTDv06_DCtx, dict: *const c_void, dictSize: size_t) {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const c_char).wrapping_offset(
        -((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char),
    ) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const c_char).wrapping_add(dictSize) as *const c_void;
}

unsafe fn ZSTDv06_loadEntropy(
    dctx: *mut ZSTDv06_DCtx,
    mut dict: *const c_void,
    mut dictSize: size_t,
) -> size_t {
    let hSize: size_t;
    let offcodeHeaderSize: size_t;
    let matchlengthHeaderSize: size_t;
    let litlengthHeaderSize: size_t;

    hSize = HUFv06_readDTableX4((*dctx).hufTableX4.as_mut_ptr(), dict, dictSize);
    if HUFv06_isError(hSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(hSize) as *const c_void;
    dictSize -= hSize;

    {
        let mut offcodeNCount = [0i16; (MaxOff + 1) as usize];
        let mut offcodeMaxValue: U32 = MaxOff;
        let mut offcodeLog: U32 = 0;
        offcodeHeaderSize = FSEv06_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dict,
            dictSize,
        );
        if FSEv06_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).OffTable.as_mut_ptr(),
                offcodeNCount.as_ptr(),
                offcodeMaxValue,
                offcodeLog,
            );
            if FSEv06_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dict = (dict as *const c_char).wrapping_add(offcodeHeaderSize) as *const c_void;
        dictSize -= offcodeHeaderSize;
    }

    {
        let mut matchlengthNCount = [0i16; (MaxML + 1) as usize];
        let mut matchlengthMaxValue: U32 = MaxML;
        let mut matchlengthLog: U32 = 0;
        matchlengthHeaderSize = FSEv06_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dict,
            dictSize,
        );
        if FSEv06_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).MLTable.as_mut_ptr(),
                matchlengthNCount.as_ptr(),
                matchlengthMaxValue,
                matchlengthLog,
            );
            if FSEv06_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dict = (dict as *const c_char).wrapping_add(matchlengthHeaderSize) as *const c_void;
        dictSize -= matchlengthHeaderSize;
    }

    {
        let mut litlengthNCount = [0i16; (MaxLL + 1) as usize];
        let mut litlengthMaxValue: U32 = MaxLL;
        let mut litlengthLog: U32 = 0;
        litlengthHeaderSize = FSEv06_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dict,
            dictSize,
        );
        if FSEv06_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode = FSEv06_buildDTable(
                (*dctx).LLTable.as_mut_ptr(),
                litlengthNCount.as_ptr(),
                litlengthMaxValue,
                litlengthLog,
            );
            if FSEv06_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
    }

    (*dctx).flagRepeatTable = 1;
    hSize + offcodeHeaderSize + matchlengthHeaderSize + litlengthHeaderSize
}

unsafe fn ZSTDv06_decompress_insertDictionary(
    dctx: *mut ZSTDv06_DCtx,
    mut dict: *const c_void,
    mut dictSize: size_t,
) -> size_t {
    let eSize: size_t;
    let magic: U32 = MEM_readLE32(dict);
    if magic != ZSTDv06_DICT_MAGIC {
        /* pure content mode */
        ZSTDv06_refDictContent(dctx, dict, dictSize);
        return 0;
    }
    /* load entropy tables */
    dict = (dict as *const c_char).wrapping_add(4) as *const c_void;
    dictSize -= 4;
    eSize = ZSTDv06_loadEntropy(dctx, dict, dictSize);
    if ZSTDv06_isError(eSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    /* reference dictionary content */
    dict = (dict as *const c_char).wrapping_add(eSize) as *const c_void;
    dictSize -= eSize;
    ZSTDv06_refDictContent(dctx, dict, dictSize);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBegin_usingDict(
    dctx: *mut ZSTDv06_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    {
        let errorCode = ZSTDv06_decompressBegin(dctx);
        if ZSTDv06_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    if !dict.is_null() && dictSize != 0 {
        let errorCode = ZSTDv06_decompress_insertDictionary(dctx, dict, dictSize);
        if ZSTDv06_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }

    0
}

// ============================================================================
// ZBUFFv06 streaming
// ============================================================================
pub type ZBUFFv06_dStage = u32;
const ZBUFFds_init: ZBUFFv06_dStage = 0;
const ZBUFFds_loadHeader: ZBUFFv06_dStage = 1;
const ZBUFFds_read: ZBUFFv06_dStage = 2;
const ZBUFFds_load: ZBUFFv06_dStage = 3;
const ZBUFFds_flush: ZBUFFv06_dStage = 4;

#[repr(C)]
pub struct ZBUFFv06_DCtx {
    pub zd: *mut ZSTDv06_DCtx,
    pub fParams: ZSTDv06_frameParams,
    pub stage: ZBUFFv06_dStage,
    pub inBuff: *mut c_char,
    pub inBuffSize: size_t,
    pub inPos: size_t,
    pub outBuff: *mut c_char,
    pub outBuffSize: size_t,
    pub outStart: size_t,
    pub outEnd: size_t,
    pub blockSize: size_t,
    pub headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
    pub lhSize: size_t,
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
        ZBUFFv06_freeDCtx(zbd); /* avoid leaking the context */
        return core::ptr::null_mut();
    }
    (*zbd).stage = ZBUFFds_init;
    zbd
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_freeDCtx(zbd: *mut ZBUFFv06_DCtx) -> size_t {
    if zbd.is_null() {
        return 0; /* support free on null */
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
    dictSize: size_t,
) -> size_t {
    (*zbd).stage = ZBUFFds_loadHeader;
    (*zbd).lhSize = 0;
    (*zbd).inPos = 0;
    (*zbd).outStart = 0;
    (*zbd).outEnd = 0;
    ZSTDv06_decompressBegin_usingDict((*zbd).zd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressInit(zbd: *mut ZBUFFv06_DCtx) -> size_t {
    ZBUFFv06_decompressInitDictionary(zbd, core::ptr::null(), 0)
}

unsafe fn ZBUFFv06_limitCopy(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
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

// Faithful translation of the C `while (notDone) { switch(stage) {...} }` state
// machine, preserving `break` (continue the while loop) and `/* fall-through */`
// semantics. Fall-throughs are modelled with an inner `loop` labelled per the
// original switch: we set `stage` and `continue` the switch to emulate a
// fall-through when the C code lacks a `break`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressContinue(
    zbd: *mut ZBUFFv06_DCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut size_t,
    src: *const c_void,
    srcSizePtr: *mut size_t,
) -> size_t {
    let istart = src as *const c_char;
    let iend = istart.wrapping_add(*srcSizePtr);
    let mut ip = istart;
    let ostart = dst as *mut c_char;
    let oend = ostart.wrapping_add(*dstCapacityPtr);
    let mut op = ostart;
    let mut notDone: U32 = 1;

    while notDone != 0 {
        // `curStage` drives the switch; a fall-through updates it and re-loops.
        let mut curStage = (*zbd).stage;
        'sw: loop {
            match curStage {
                x if x == ZBUFFds_init => {
                    return ERROR(ZSTD_error_init_missing);
                }
                x if x == ZBUFFds_loadHeader => {
                    {
                        let hSize = ZSTDv06_getFrameParams(
                            &mut (*zbd).fParams,
                            (*zbd).headerBuffer.as_ptr() as *const c_void,
                            (*zbd).lhSize,
                        );
                        if hSize != 0 {
                            let toLoad = hSize - (*zbd).lhSize; /* hSize > zbd->lhSize */
                            if ZSTDv06_isError(hSize) != 0 {
                                return hSize;
                            }
                            if toLoad > iend.offset_from(ip) as size_t {
                                /* not enough input to load full header */
                                if !ip.is_null() {
                                    memcpy(
                                        (*zbd)
                                            .headerBuffer
                                            .as_mut_ptr()
                                            .wrapping_add((*zbd).lhSize)
                                            as *mut c_void,
                                        ip as *const c_void,
                                        iend.offset_from(ip) as size_t,
                                    );
                                }
                                (*zbd).lhSize += iend.offset_from(ip) as size_t;
                                *dstCapacityPtr = 0;
                                return (hSize - (*zbd).lhSize) + ZSTDv06_blockHeaderSize;
                            }
                            memcpy(
                                (*zbd).headerBuffer.as_mut_ptr().wrapping_add((*zbd).lhSize)
                                    as *mut c_void,
                                ip as *const c_void,
                                toLoad,
                            );
                            (*zbd).lhSize = hSize;
                            ip = ip.wrapping_add(toLoad);
                            break 'sw; /* C `break;` : leave switch, re-enter while */
                        }
                    }

                    /* Consume header */
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
                            /* long header */
                            let h2Size = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                            let h2Result = ZSTDv06_decompressContinue(
                                (*zbd).zd,
                                core::ptr::null_mut(),
                                0,
                                (*zbd).headerBuffer.as_ptr().wrapping_add(h1Size) as *const c_void,
                                h2Size,
                            );
                            if ZSTDv06_isError(h2Result) != 0 {
                                return h2Result;
                            }
                        }
                    }

                    /* Frame header instruct buffer sizes */
                    {
                        let blockSize = core::cmp::min(
                            1usize << (*zbd).fParams.windowLog,
                            ZSTDv06_BLOCKSIZE_MAX,
                        );
                        (*zbd).blockSize = blockSize;
                        if (*zbd).inBuffSize < blockSize {
                            free((*zbd).inBuff as *mut c_void);
                            (*zbd).inBuffSize = blockSize;
                            (*zbd).inBuff = malloc(blockSize) as *mut c_char;
                            if (*zbd).inBuff.is_null() {
                                return ERROR(ZSTD_error_memory_allocation);
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
                                    return ERROR(ZSTD_error_memory_allocation);
                                }
                            }
                        }
                    }
                    (*zbd).stage = ZBUFFds_read;
                    curStage = ZBUFFds_read;
                    continue 'sw; /* fall-through */
                }
                x if x == ZBUFFds_read => {
                    let neededInSize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    if neededInSize == 0 {
                        /* end of frame */
                        (*zbd).stage = ZBUFFds_init;
                        notDone = 0;
                        break 'sw;
                    }
                    if iend.offset_from(ip) as size_t >= neededInSize {
                        /* decode directly from src */
                        let decodedSize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.wrapping_add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize - (*zbd).outStart,
                            ip as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv06_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        ip = ip.wrapping_add(neededInSize);
                        if decodedSize == 0 {
                            break 'sw; /* this was just a header */
                        }
                        (*zbd).outEnd = (*zbd).outStart + decodedSize;
                        (*zbd).stage = ZBUFFds_flush;
                        break 'sw;
                    }
                    if ip == iend {
                        notDone = 0;
                        break 'sw;
                    } /* no more input */
                    (*zbd).stage = ZBUFFds_load;
                    curStage = ZBUFFds_load;
                    continue 'sw; /* fall-through */
                }
                x if x == ZBUFFds_load => {
                    let neededInSize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    let toLoad = neededInSize - (*zbd).inPos;
                    let loadedSize: size_t;
                    if toLoad > (*zbd).inBuffSize - (*zbd).inPos {
                        return ERROR(ZSTD_error_corruption_detected); /* should never happen */
                    }
                    loadedSize = ZBUFFv06_limitCopy(
                        (*zbd).inBuff.wrapping_add((*zbd).inPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        iend.offset_from(ip) as size_t,
                    );
                    ip = ip.wrapping_add(loadedSize);
                    (*zbd).inPos += loadedSize;
                    if loadedSize < toLoad {
                        notDone = 0;
                        break 'sw;
                    } /* not enough input, wait for more */

                    /* decode loaded input */
                    {
                        let decodedSize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.wrapping_add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize - (*zbd).outStart,
                            (*zbd).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv06_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        (*zbd).inPos = 0; /* input is consumed */
                        if decodedSize == 0 {
                            (*zbd).stage = ZBUFFds_read;
                            break 'sw;
                        }
                        (*zbd).outEnd = (*zbd).outStart + decodedSize;
                        (*zbd).stage = ZBUFFds_flush;
                    }
                    curStage = ZBUFFds_flush;
                    continue 'sw; /* fall-through */
                }
                x if x == ZBUFFds_flush => {
                    let toFlushSize = (*zbd).outEnd - (*zbd).outStart;
                    let flushedSize = ZBUFFv06_limitCopy(
                        op as *mut c_void,
                        oend.offset_from(op) as size_t,
                        (*zbd).outBuff.wrapping_add((*zbd).outStart) as *const c_void,
                        toFlushSize,
                    );
                    op = op.wrapping_add(flushedSize);
                    (*zbd).outStart += flushedSize;
                    if flushedSize == toFlushSize {
                        (*zbd).stage = ZBUFFds_read;
                        if (*zbd).outStart + (*zbd).blockSize > (*zbd).outBuffSize {
                            (*zbd).outStart = 0;
                            (*zbd).outEnd = 0;
                        }
                        break 'sw;
                    }
                    /* cannot flush everything */
                    notDone = 0;
                    break 'sw;
                }
                _ => return ERROR(ZSTD_error_GENERIC), /* impossible */
            }
        }
    }

    /* result */
    *srcSizePtr = ip.offset_from(istart) as size_t;
    *dstCapacityPtr = op.offset_from(ostart) as size_t;
    {
        let mut nextSrcSizeHint = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
        if nextSrcSizeHint > ZSTDv06_blockHeaderSize {
            nextSrcSizeHint += ZSTDv06_blockHeaderSize;
        }
        nextSrcSizeHint -= (*zbd).inPos; /* already loaded */
        nextSrcSizeHint
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_recommendedDInSize() -> size_t {
    ZSTDv06_BLOCKSIZE_MAX + ZSTDv06_blockHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_recommendedDOutSize() -> size_t {
    ZSTDv06_BLOCKSIZE_MAX
}
