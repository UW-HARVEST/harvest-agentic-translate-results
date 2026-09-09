//! Literal Rust transliteration of `c_src/src/legacy/zstd_v05.c`.
//!
//! This file is a self-contained translation unit: it bundles its own
//! mem/endian helpers, error codes, bitstream reader, FSEv05 decoder, HUFv05
//! decoder, block/frame decoders, DCtx and the ZBUFFv05 streaming state
//! machine. Everything is translated literally; only the 50 documented
//! `FSEv05_*` / `HUFv05_*` / `ZSTDv05_*` / `ZBUFFv05_*` entry points are
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
pub unsafe fn MEM_write32(memPtr: *mut c_void, value: U32) {
    (memPtr as *mut U32).write_unaligned(value)
}
#[inline]
pub unsafe fn MEM_write64(memPtr: *mut c_void, value: U64) {
    (memPtr as *mut U64).write_unaligned(value)
}

#[inline]
pub unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    // target is little-endian
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
// zstd_v05.c `#include "../common/error_private.h"` FIRST, which sets the
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
// zstd_internal constants
// ============================================================================
const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525;
const ZSTDv05_DICT_MAGIC: U32 = 0xEC30A435;
const ZSTDv05_WINDOWLOG_ABSOLUTEMIN: U32 = 11;

const BLOCKSIZE: usize = 128 * 1024; /* 128 KB */

const ZSTDv05_blockHeaderSize: size_t = 3;
const ZSTDv05_frameHeaderSize_min: size_t = 5;
const ZSTDv05_frameHeaderSize_max: size_t = 5;

const IS_HUFv05: U32 = 0;
const IS_PCH: U32 = 1;
const IS_RAW: U32 = 2;
const IS_RLE: U32 = 3;

const MINMATCH: size_t = 4;
const REPCODE_STARTVALUE: size_t = 1;

const MLbits: U32 = 7;
const LLbits: U32 = 6;
const Offbits: U32 = 5;
const MaxLL: U32 = (1 << LLbits) - 1;
const MaxML: U32 = (1 << MLbits) - 1;
const MaxOff: U32 = (1 << Offbits) - 1;
const MLFSEv05Log: U32 = 10;
const LLFSEv05Log: U32 = 10;
const OffFSEv05Log: U32 = 9;

const FSEv05_ENCODING_RAW: U32 = 0;
const FSEv05_ENCODING_RLE: U32 = 1;
const FSEv05_ENCODING_STATIC: U32 = 2;
const FSEv05_ENCODING_DYNAMIC: U32 = 3;

const ZSTD_HUFFDTABLE_CAPACITY_LOG: U32 = 12;

const MIN_SEQUENCES_SIZE: size_t = 1;
const MIN_CBLOCK_SIZE: size_t = 1 + 1 + MIN_SEQUENCES_SIZE;

const WILDCOPY_OVERLENGTH: usize = 8;

const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

// blockType_t { bt_compressed, bt_raw, bt_rle, bt_end }
pub type blockType_t = u32;
const bt_compressed: blockType_t = 0;
const bt_raw: blockType_t = 1;
const bt_rle: blockType_t = 2;
const bt_end: blockType_t = 3;

#[inline]
unsafe fn ZSTDv05_copy8(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 8);
}
#[inline]
unsafe fn ZSTDv05_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

// MEM_STATIC void ZSTDv05_wildcopy(void* dst, const void* src, ptrdiff_t length)
#[inline]
unsafe fn ZSTDv05_wildcopy(dst: *mut c_void, src: *const c_void, length: isize) {
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.wrapping_offset(length);
    loop {
        // COPY8(op, ip)
        ZSTDv05_copy8(op as *mut c_void, ip as *const c_void);
        op = op.wrapping_add(8);
        ip = ip.wrapping_add(8);
        if !(op < oend) {
            break;
        }
    }
}

// ============================================================================
// bitstream (BITv05_DStream)
// ============================================================================
#[repr(C)]
pub struct BITv05_DStream_t {
    pub bitContainer: size_t,
    pub bitsConsumed: u32,
    pub ptr: *const c_char,
    pub start: *const c_char,
}

pub type BITv05_DStream_status = u32;
const BITv05_DStream_unfinished: BITv05_DStream_status = 0;
const BITv05_DStream_endOfBuffer: BITv05_DStream_status = 1;
const BITv05_DStream_completed: BITv05_DStream_status = 2;
const BITv05_DStream_overflow: BITv05_DStream_status = 3;

#[inline]
unsafe fn BITv05_highbit32(val: U32) -> u32 {
    // __builtin_clz(val) ^ 31  (val != 0 by contract)
    31u32.wrapping_sub(val.leading_zeros())
}

unsafe fn BITv05_initDStream(
    bitD: *mut BITv05_DStream_t,
    srcBuffer: *const c_void,
    srcSize: size_t,
) -> size_t {
    if srcSize < 1 {
        memset(bitD as *mut c_void, 0, core::mem::size_of::<BITv05_DStream_t>());
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    let sizeofST = core::mem::size_of::<size_t>();
    if srcSize >= sizeofST {
        /* normal case */
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (srcBuffer as *const c_char).wrapping_add(srcSize - sizeofST);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        contain32 = *((srcBuffer as *const BYTE).wrapping_add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BITv05_highbit32(contain32);
    } else {
        let contain32: U32;
        (*bitD).start = srcBuffer as *const c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as size_t;
        let base = (*bitD).start as *const BYTE;
        // switch with fall-through
        if srcSize == 7 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*(base.wrapping_add(6)) as size_t) << (sizeofST * 8 - 16),
            );
        }
        if srcSize >= 6 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*(base.wrapping_add(5)) as size_t) << (sizeofST * 8 - 24),
            );
        }
        if srcSize >= 5 {
            (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                (*(base.wrapping_add(4)) as size_t) << (sizeofST * 8 - 32),
            );
        }
        if srcSize >= 4 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*(base.wrapping_add(3)) as size_t) << 24);
        }
        if srcSize >= 3 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*(base.wrapping_add(2)) as size_t) << 16);
        }
        if srcSize >= 2 {
            (*bitD).bitContainer = (*bitD)
                .bitContainer
                .wrapping_add((*(base.wrapping_add(1)) as size_t) << 8);
        }
        contain32 = *((srcBuffer as *const BYTE).wrapping_add(srcSize - 1)) as U32;
        if contain32 == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
        (*bitD).bitsConsumed = 8 - BITv05_highbit32(contain32);
        (*bitD).bitsConsumed += ((sizeofST - srcSize) as U32).wrapping_mul(8);
    }

    srcSize
}

#[inline]
unsafe fn BITv05_lookBits(bitD: *const BITv05_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() * 8) as U32 - 1;
    (((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask)) >> 1)
        >> ((bitMask.wrapping_sub(nbBits)) & bitMask)
}

#[inline]
unsafe fn BITv05_lookBitsFast(bitD: *const BITv05_DStream_t, nbBits: U32) -> size_t {
    let bitMask: U32 = (core::mem::size_of::<size_t>() * 8) as U32 - 1;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & bitMask))
        >> (((bitMask + 1).wrapping_sub(nbBits)) & bitMask)
}

#[inline]
unsafe fn BITv05_skipBits(bitD: *mut BITv05_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

#[inline]
unsafe fn BITv05_readBits(bitD: *mut BITv05_DStream_t, nbBits: u32) -> size_t {
    let value = BITv05_lookBits(bitD, nbBits);
    BITv05_skipBits(bitD, nbBits);
    value
}

#[inline]
unsafe fn BITv05_readBitsFast(bitD: *mut BITv05_DStream_t, nbBits: u32) -> size_t {
    let value = BITv05_lookBitsFast(bitD, nbBits);
    BITv05_skipBits(bitD, nbBits);
    value
}

unsafe fn BITv05_reloadDStream(bitD: *mut BITv05_DStream_t) -> BITv05_DStream_status {
    let sizeofBC = core::mem::size_of::<size_t>();
    if (*bitD).bitsConsumed > (sizeofBC * 8) as u32 {
        return BITv05_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).start.wrapping_add(sizeofBC) {
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(((*bitD).bitsConsumed >> 3) as usize);
        (*bitD).bitsConsumed &= 7;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        return BITv05_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < (sizeofBC * 8) as u32 {
            return BITv05_DStream_endOfBuffer;
        }
        return BITv05_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BITv05_DStream_status = BITv05_DStream_unfinished;
        if (*bitD).ptr.wrapping_sub(nbBytes as usize) < (*bitD).start {
            nbBytes = (*bitD).ptr.offset_from((*bitD).start) as U32; /* ptr > start */
            result = BITv05_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.wrapping_sub(nbBytes as usize);
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_sub(nbBytes.wrapping_mul(8));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline]
unsafe fn BITv05_endOfDStream(DStream: *const BITv05_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start)
        && ((*DStream).bitsConsumed == (core::mem::size_of::<size_t>() * 8) as u32)) as u32
}

// ============================================================================
// FSEv05
// ============================================================================
pub type FSEv05_DTable = u32;

const FSEv05_MAX_MEMORY_USAGE: u32 = 14;
const FSEv05_MAX_SYMBOL_VALUE: u32 = 255;
const FSEv05_MAX_TABLELOG: u32 = FSEv05_MAX_MEMORY_USAGE - 2;
const FSEv05_MAX_TABLESIZE: u32 = 1u32 << FSEv05_MAX_TABLELOG;
const FSEv05_MIN_TABLELOG: u32 = 5;
const FSEv05_TABLELOG_ABSOLUTE_MAX: u32 = 15;

// #define FSEv05_DTABLE_SIZE_U32(maxTableLog) (1 + (1<<maxTableLog))
#[inline]
const fn FSEv05_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv05_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
} /* sizeof U32 */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv05_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
} /* size == U32 */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSEv05_DState_t {
    pub state: size_t,
    pub table: *const c_void,
}

#[inline]
unsafe fn FSEv05_initDState(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
    dt: *const FSEv05_DTable,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv05_DTableHeader;
    (*DStatePtr).state = BITv05_readBits(bitD, (*DTableH).tableLog as u32);
    BITv05_reloadDStream(bitD);
    (*DStatePtr).table = dt.wrapping_add(1) as *const c_void;
}

#[inline]
unsafe fn FSEv05_peakSymbol(DStatePtr: *mut FSEv05_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).wrapping_add((*DStatePtr).state);
    DInfo.symbol
}

#[inline]
unsafe fn FSEv05_decodeSymbol(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv05_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

#[inline]
unsafe fn FSEv05_decodeSymbolFast(
    DStatePtr: *mut FSEv05_DState_t,
    bitD: *mut BITv05_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSEv05_decode_t).wrapping_add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BITv05_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    symbol
}

#[inline]
unsafe fn FSEv05_endOfDState(DStatePtr: *const FSEv05_DState_t) -> u32 {
    ((*DStatePtr).state == 0) as u32
}

#[inline]
unsafe fn FSEv05_tableStep(tableSize: U32) -> U32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_createDTable(mut tableLog: u32) -> *mut FSEv05_DTable {
    if tableLog > FSEv05_TABLELOG_ABSOLUTE_MAX {
        tableLog = FSEv05_TABLELOG_ABSOLUTE_MAX;
    }
    malloc(FSEv05_DTABLE_SIZE_U32(tableLog) * core::mem::size_of::<U32>()) as *mut FSEv05_DTable
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_freeDTable(dt: *mut FSEv05_DTable) {
    free(dt as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable(
    dt: *mut FSEv05_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
) -> size_t {
    let mut DTableH = FSEv05_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tdPtr = dt.wrapping_add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSEv05_decode_t;
    let tableSize: U32 = 1 << tableLog;
    let tableMask: U32 = tableSize - 1;
    let step: U32 = FSEv05_tableStep(tableSize);
    let mut symbolNext = [0u16; (FSEv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut position: U32 = 0;
    let mut highThreshold: U32 = tableSize - 1;
    let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
    let mut noLarge: U32 = 1;
    let mut s: U32;

    /* Sanity Checks */
    if maxSymbolValue > FSEv05_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSEv05_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    memset(
        tableDecode as *mut c_void,
        0,
        core::mem::size_of::<BYTE>() * (maxSymbolValue as usize + 1),
    );
    DTableH.tableLog = tableLog as U16;
    s = 0;
    while s <= maxSymbolValue {
        let nc = *normalizedCounter.wrapping_add(s as usize);
        if nc == -1 {
            (*tableDecode.wrapping_add(highThreshold as usize)).symbol = s as BYTE;
            highThreshold = highThreshold.wrapping_sub(1);
            symbolNext[s as usize] = 1;
        } else {
            if nc >= largeLimit {
                noLarge = 0;
            }
            symbolNext[s as usize] = nc as u16;
        }
        s += 1;
    }

    /* Spread symbols */
    s = 0;
    while s <= maxSymbolValue {
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
        return ERROR(ZSTD_error_GENERIC);
    }

    /* Build Decoding table */
    {
        let mut i: U32 = 0;
        while i < tableSize {
            let symbol: BYTE = (*tableDecode.wrapping_add(i as usize)).symbol;
            let nextState: U16 = symbolNext[symbol as usize];
            symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
            (*tableDecode.wrapping_add(i as usize)).nbBits =
                (tableLog.wrapping_sub(BITv05_highbit32(nextState as U32))) as BYTE;
            (*tableDecode.wrapping_add(i as usize)).newState =
                (((nextState as U32) << (*tableDecode.wrapping_add(i as usize)).nbBits)
                    .wrapping_sub(tableSize)) as U16;
            i += 1;
        }
    }

    DTableH.fastMode = noLarge as U16;
    memcpy(
        dt as *mut c_void,
        &DTableH as *const FSEv05_DTableHeader as *const c_void,
        core::mem::size_of::<FSEv05_DTableHeader>(),
    );
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

#[inline]
unsafe fn FSEv05_abs(a: i16) -> i16 {
    if a < 0 {
        -a
    } else {
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_readNCount(
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
    nbBits = ((bitStream & 0xF) + FSEv05_MIN_TABLELOG) as i32; /* extract tableLog */
    if nbBits as u32 > FSEv05_TABLELOG_ABSOLUTE_MAX {
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

            if (bitStream & (threshold as u32 - 1)) < (max as U32) {
                count = (bitStream & (threshold as u32 - 1)) as i16;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold as u32 - 1)) as i16;
                if count >= threshold as i16 {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1; /* extra accuracy */
            remaining -= FSEv05_abs(count) as i32;
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
pub unsafe extern "C" fn FSEv05_buildDTable_rle(dt: *mut FSEv05_DTable, symbolValue: u8) -> size_t {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv05_DTableHeader;
    let dPtr = dt.wrapping_add(1) as *mut c_void;
    let cell = dPtr as *mut FSEv05_decode_t;

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).newState = 0;
    (*cell).symbol = symbolValue;
    (*cell).nbBits = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_buildDTable_raw(dt: *mut FSEv05_DTable, nbBits: u32) -> size_t {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut FSEv05_DTableHeader;
    let dPtr = dt.wrapping_add(1) as *mut c_void;
    let dinfo = dPtr as *mut FSEv05_decode_t;
    let tableSize: u32 = 1 << nbBits;
    let tableMask: u32 = tableSize - 1;
    let maxSymbolValue: u32 = tableMask;
    let mut s: u32;

    /* Sanity checks */
    if nbBits < 1 {
        return ERROR(ZSTD_error_GENERIC);
    }

    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1;
    s = 0;
    while s <= maxSymbolValue {
        (*dinfo.wrapping_add(s as usize)).newState = 0;
        (*dinfo.wrapping_add(s as usize)).symbol = s as BYTE;
        (*dinfo.wrapping_add(s as usize)).nbBits = nbBits as BYTE;
        s += 1;
    }

    0
}

unsafe fn FSEv05_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSEv05_DTable,
    fast: u32,
) -> size_t {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.wrapping_add(maxDstSize);
    let olimit = omax.wrapping_sub(3);

    let mut bitD: BITv05_DStream_t = core::mem::zeroed();
    let mut state1: FSEv05_DState_t = core::mem::zeroed();
    let mut state2: FSEv05_DState_t = core::mem::zeroed();
    let mut errorCode: size_t;

    /* Init */
    errorCode = BITv05_initDStream(&mut bitD, cSrc, cSrcSize);
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }

    FSEv05_initDState(&mut state1, &mut bitD, dt);
    FSEv05_initDState(&mut state2, &mut bitD, dt);

    macro_rules! FSEv05_GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSEv05_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSEv05_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    // static test: FSEv05_MAX_TABLELOG*2+7 > sizeof(size_t)*8 ?
    let sizeofBits = (core::mem::size_of::<size_t>() * 8) as u32;
    let test2 = FSEv05_MAX_TABLELOG * 2 + 7 > sizeofBits;
    let test4 = FSEv05_MAX_TABLELOG * 4 + 7 > sizeofBits;

    /* 4 symbols per loop */
    while (BITv05_reloadDStream(&mut bitD) == BITv05_DStream_unfinished) && (op < olimit) {
        *op.wrapping_add(0) = FSEv05_GETSYMBOL!(&mut state1);
        if test2 {
            BITv05_reloadDStream(&mut bitD);
        }
        *op.wrapping_add(1) = FSEv05_GETSYMBOL!(&mut state2);
        if test4 {
            if BITv05_reloadDStream(&mut bitD) > BITv05_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }
        *op.wrapping_add(2) = FSEv05_GETSYMBOL!(&mut state1);
        if test2 {
            BITv05_reloadDStream(&mut bitD);
        }
        *op.wrapping_add(3) = FSEv05_GETSYMBOL!(&mut state2);
        op = op.wrapping_add(4);
    }

    /* tail */
    loop {
        if (BITv05_reloadDStream(&mut bitD) > BITv05_DStream_completed)
            || (op == omax)
            || (BITv05_endOfDStream(&bitD) != 0
                && (fast != 0 || FSEv05_endOfDState(&state1) != 0))
        {
            break;
        }
        *op = FSEv05_GETSYMBOL!(&mut state1);
        op = op.wrapping_add(1);

        if (BITv05_reloadDStream(&mut bitD) > BITv05_DStream_completed)
            || (op == omax)
            || (BITv05_endOfDStream(&bitD) != 0
                && (fast != 0 || FSEv05_endOfDState(&state2) != 0))
        {
            break;
        }
        *op = FSEv05_GETSYMBOL!(&mut state2);
        op = op.wrapping_add(1);
    }

    /* end ? */
    if BITv05_endOfDStream(&bitD) != 0
        && FSEv05_endOfDState(&state1) != 0
        && FSEv05_endOfDState(&state2) != 0
    {
        return op.offset_from(ostart) as size_t;
    }

    if op == omax {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    ERROR(ZSTD_error_corruption_detected)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_decompress_usingDTable(
    dst: *mut c_void,
    originalSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSEv05_DTable,
) -> size_t {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const FSEv05_DTableHeader;
    let fastMode: U32 = (*DTableH).fastMode as U32;

    if fastMode != 0 {
        return FSEv05_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 1);
    }
    FSEv05_decompress_usingDTable_generic(dst, originalSize, cSrc, cSrcSize, dt, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv05_decompress(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut counting = [0i16; (FSEv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut dt = [0u32; FSEv05_DTABLE_SIZE_U32(FSEv05_MAX_TABLELOG)];
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSEv05_MAX_SYMBOL_VALUE;
    let mut errorCode: size_t;

    if cSrcSize < 2 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* normal FSEv05 decoding mode */
    errorCode = FSEv05_readNCount(
        counting.as_mut_ptr(),
        &mut maxSymbolValue,
        &mut tableLog,
        istart as *const c_void,
        cSrcSize,
    );
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    errorCode = FSEv05_buildDTable(dt.as_mut_ptr(), counting.as_ptr(), maxSymbolValue, tableLog);
    if FSEv05_isError(errorCode) != 0 {
        return errorCode;
    }

    /* always return, even if it is an error code */
    FSEv05_decompress_usingDTable(dst, maxDstSize, ip as *const c_void, cSrcSize, dt.as_ptr())
}

// ============================================================================
// HUFv05
// ============================================================================
const HUFv05_ABSOLUTEMAX_TABLELOG: u32 = 16;
const HUFv05_MAX_TABLELOG: u32 = 12;
const HUFv05_MAX_SYMBOL_VALUE: u32 = 255;

#[inline]
const fn HUFv05_DTABLE_SIZE(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUFv05_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
} /* single-symbol decoding */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUFv05_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
} /* double-symbols decoding */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
    pub weight: BYTE,
}

unsafe fn HUFv05_readStats(
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
    iSize = *ip.wrapping_add(0) as size_t;

    if iSize >= 128 {
        /* special header */
        if iSize >= 242 {
            /* RLE */
            static L: [i32; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128];
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
            n = 0;
            while (n as size_t) < oSize {
                *huffWeight.wrapping_add(n as usize) = *ip.wrapping_add((n / 2) as usize) >> 4;
                *huffWeight.wrapping_add((n + 1) as usize) =
                    *ip.wrapping_add((n / 2) as usize) & 15;
                n += 2;
            }
        }
    } else {
        /* header compressed with FSEv05 (normal case) */
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        oSize = FSEv05_decompress(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.wrapping_add(1) as *const c_void,
            iSize,
        ); /* max (hwSize-1) values decoded, as last one is implied */
        if FSEv05_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    memset(
        rankStats as *mut c_void,
        0,
        (HUFv05_ABSOLUTEMAX_TABLELOG as usize + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    n = 0;
    while (n as size_t) < oSize {
        let w = *huffWeight.wrapping_add(n as usize);
        if w as U32 >= HUFv05_ABSOLUTEMAX_TABLELOG {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *rankStats.wrapping_add(w as usize) = (*rankStats.wrapping_add(w as usize)).wrapping_add(1);
        weightTotal = weightTotal.wrapping_add((1u32 << w) >> 1);
        n += 1;
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    tableLog = BITv05_highbit32(weightTotal) + 1;
    if tableLog > HUFv05_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_corruption_detected);
    }
    {
        let total: U32 = 1u32 << tableLog;
        let rest: U32 = total - weightTotal;
        let verif: U32 = 1u32 << BITv05_highbit32(rest);
        let lastWeight: U32 = BITv05_highbit32(rest) + 1;
        if verif != rest {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *huffWeight.wrapping_add(oSize) = lastWeight as BYTE;
        *rankStats.wrapping_add(lastWeight as usize) =
            (*rankStats.wrapping_add(lastWeight as usize)).wrapping_add(1);
    }

    /* check tree construction validity */
    if (*rankStats.wrapping_add(1) < 2) || (*rankStats.wrapping_add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    *tableLogPtr = tableLog;
    iSize + 1
}

// -------- single-symbol decoding --------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_readDTableX2(
    DTable: *mut U16,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut huffWeight = [0u8; (HUFv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankVal = [0u32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut tableLog: U32 = 0;
    let iSize: size_t;
    let mut nbSymbols: U32 = 0;
    let mut n: U32;
    let mut nextRankStart: U32;
    let dtPtr = DTable.wrapping_add(1) as *mut c_void;
    let dt = dtPtr as *mut HUFv05_DEltX2;

    iSize = HUFv05_readStats(
        huffWeight.as_mut_ptr(),
        (HUFv05_MAX_SYMBOL_VALUE + 1) as size_t,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv05_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > *DTable.wrapping_add(0) as U32 {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    *DTable.wrapping_add(0) = tableLog as U16;

    /* Prepare ranks */
    nextRankStart = 0;
    n = 1;
    while n <= tableLog {
        let current = nextRankStart;
        nextRankStart += rankVal[n as usize] << (n - 1);
        rankVal[n as usize] = current;
        n += 1;
    }

    /* fill DTable */
    n = 0;
    while n < nbSymbols {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = (1u32 << w) >> 1;
        let mut i: U32;
        let mut D = HUFv05_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = (tableLog + 1 - w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize] + length {
            *dt.wrapping_add(i as usize) = D;
            i += 1;
        }
        rankVal[w as usize] += length;
        n += 1;
    }

    iSize
}

#[inline]
unsafe fn HUFv05_decodeSymbolX2(
    Dstream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val = BITv05_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c = (*dt.wrapping_add(val)).byte;
    BITv05_skipBits(Dstream, (*dt.wrapping_add(val)).nbBits as U32);
    c
}

unsafe fn HUFv05_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv05_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv05_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart = p;

    // macros translated (MEM_64bits() == 1 and HUFv05_MAX_TABLELOG<=12, both true here)
    macro_rules! DECODE0 {
        () => {{
            *p = HUFv05_decodeSymbolX2(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }};
    }
    macro_rules! DECODE1 {
        () => {
            // if (MEM_64bits() || (HUFv05_MAX_TABLELOG<=12))
            DECODE0!()
        };
    }
    macro_rules! DECODE2 {
        () => {
            // if (MEM_64bits())
            if MEM_64bits() != 0 {
                DECODE0!()
            }
        };
    }

    /* up to 4 symbols at a time */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(4))
    {
        DECODE2!();
        DECODE1!();
        DECODE2!();
        DECODE0!();
    }

    /* closer to the end */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished) && (p < pEnd) {
        DECODE0!();
    }

    /* no more data to retrieve from bitstream, hence no need to reload */
    while p < pEnd {
        DECODE0!();
    }

    p.offset_from(pStart) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X2_usingDTable(
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
    let dt = (dtPtr as *const HUFv05_DEltX2).wrapping_add(1);
    let mut bitD: BITv05_DStream_t = core::mem::zeroed();

    if dstSize <= cSrcSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let errorCode = BITv05_initDStream(&mut bitD, cSrc, cSrcSize);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    HUFv05_decodeStreamX2(op, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv05_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X2(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable = [0u16; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG as u16;
    let mut ip = cSrc as *const BYTE;
    let errorCode: size_t;

    errorCode = HUFv05_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    HUFv05_decompress1X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X2_usingDTable(
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
        let dt = (dtPtr as *const HUFv05_DEltX2).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0) as U32;
        let mut errorCode: size_t;

        let mut bitD1: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv05_DStream_t = core::mem::zeroed();
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
            return ERROR(ZSTD_error_corruption_detected);
        }
        errorCode = BITv05_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }

        macro_rules! DECODE0 {
            ($op:expr, $bd:expr) => {{
                *$op = HUFv05_decodeSymbolX2($bd, dt, dtLog);
                $op = $op.wrapping_add(1);
            }};
        }
        macro_rules! DECODE1 {
            ($op:expr, $bd:expr) => {
                DECODE0!($op, $bd)
            };
        }
        macro_rules! DECODE2 {
            ($op:expr, $bd:expr) => {
                if MEM_64bits() != 0 {
                    DECODE0!($op, $bd)
                }
            };
        }

        /* 16-32 symbols per loop */
        endSignal = BITv05_reloadDStream(&mut bitD1)
            | BITv05_reloadDStream(&mut bitD2)
            | BITv05_reloadDStream(&mut bitD3)
            | BITv05_reloadDStream(&mut bitD4);
        while (endSignal == BITv05_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            DECODE2!(op1, &mut bitD1);
            DECODE2!(op2, &mut bitD2);
            DECODE2!(op3, &mut bitD3);
            DECODE2!(op4, &mut bitD4);
            DECODE1!(op1, &mut bitD1);
            DECODE1!(op2, &mut bitD2);
            DECODE1!(op3, &mut bitD3);
            DECODE1!(op4, &mut bitD4);
            DECODE2!(op1, &mut bitD1);
            DECODE2!(op2, &mut bitD2);
            DECODE2!(op3, &mut bitD3);
            DECODE2!(op4, &mut bitD4);
            DECODE0!(op1, &mut bitD1);
            DECODE0!(op2, &mut bitD2);
            DECODE0!(op3, &mut bitD3);
            DECODE0!(op4, &mut bitD4);
            endSignal = BITv05_reloadDStream(&mut bitD1)
                | BITv05_reloadDStream(&mut bitD2)
                | BITv05_reloadDStream(&mut bitD3)
                | BITv05_reloadDStream(&mut bitD4);
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

        /* finish bitStreams one by one */
        HUFv05_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv05_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv05_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv05_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BITv05_endOfDStream(&bitD1)
            & BITv05_endOfDStream(&bitD2)
            & BITv05_endOfDStream(&bitD3)
            & BITv05_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        dstSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X2(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable = [0u16; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG as u16;
    let mut ip = cSrc as *const BYTE;
    let errorCode: size_t;

    errorCode = HUFv05_readDTableX2(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(errorCode);
    cSrcSize -= errorCode;

    HUFv05_decompress4X2_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// -------- double-symbols decoding --------

unsafe fn HUFv05_fillDTableX4Level2(
    DTable: *mut HUFv05_DEltX4,
    sizeLog: U32,
    consumed: U32,
    rankValOrigin: *const U32,
    minWeight: i32,
    sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    let mut DElt = HUFv05_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal = [0u32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut s: U32;

    /* get pre-calculated rankVal */
    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        rankValOrigin as *const c_void,
        core::mem::size_of::<[u32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill skipped values */
    if minWeight > 1 {
        let mut i: U32;
        let skipSize: U32 = rankVal[minWeight as usize];
        MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1;
        i = 0;
        while i < skipSize {
            *DTable.wrapping_add(i as usize) = DElt;
            i += 1;
        }
    }

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol: U32 = (*sortedSymbols.wrapping_add(s as usize)).symbol as U32;
        let weight: U32 = (*sortedSymbols.wrapping_add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let length: U32 = 1u32 << (sizeLog - nbBits);
        let start: U32 = rankVal[weight as usize];
        let mut i: U32 = start;
        let end: U32 = start + length;

        MEM_writeLE16(
            &mut DElt.sequence as *mut U16 as *mut c_void,
            (baseSeq as U32).wrapping_add(symbol << 8) as U16,
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

// typedef U32 rankVal_t[HUFv05_ABSOLUTEMAX_TABLELOG][HUFv05_ABSOLUTEMAX_TABLELOG + 1];
type rankVal_t = [[U32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize]; HUFv05_ABSOLUTEMAX_TABLELOG as usize];

unsafe fn HUFv05_fillDTableX4(
    DTable: *mut HUFv05_DEltX4,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    rankStart: *const U32,
    rankValOrigin: *const rankVal_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal = [0u32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let scaleLog: i32 = nbBitsBaseline as i32 - targetLog as i32; /* scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32;

    memcpy(
        rankVal.as_mut_ptr() as *mut c_void,
        (*rankValOrigin)[0].as_ptr() as *const c_void,
        core::mem::size_of::<[u32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize]>(),
    );

    /* fill DTable */
    s = 0;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.wrapping_add(s as usize)).symbol as U16;
        let weight: U32 = (*sortedList.wrapping_add(s as usize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let start: U32 = rankVal[weight as usize];
        let length: U32 = 1u32 << (targetLog - nbBits);

        if targetLog.wrapping_sub(nbBits) >= minBits {
            /* enough room for a second symbol */
            let sortedRank: U32;
            let mut minWeight: i32 = nbBits as i32 + scaleLog;
            if minWeight < 1 {
                minWeight = 1;
            }
            sortedRank = *rankStart.wrapping_add(minWeight as usize);
            HUFv05_fillDTableX4Level2(
                DTable.wrapping_add(start as usize),
                targetLog - nbBits,
                nbBits,
                (*rankValOrigin)[nbBits as usize].as_ptr(),
                minWeight,
                sortedList.wrapping_add(sortedRank as usize),
                sortedListSize - sortedRank,
                nbBitsBaseline,
                symbol,
            );
        } else {
            let mut i: U32;
            let end: U32 = start + length;
            let mut DElt = HUFv05_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };

            MEM_writeLE16(&mut DElt.sequence as *mut U16 as *mut c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1;
            i = start;
            while i < end {
                *DTable.wrapping_add(i as usize) = DElt;
                i += 1;
            }
        }
        rankVal[weight as usize] += length;
        s += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_readDTableX4(
    DTable: *mut u32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut weightList = [0u8; (HUFv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut sortedSymbol = [sortedSymbol_t { symbol: 0, weight: 0 };
        (HUFv05_MAX_SYMBOL_VALUE + 1) as usize];
    let mut rankStats = [0u32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize];
    let mut rankStart0 = [0u32; (HUFv05_ABSOLUTEMAX_TABLELOG + 2) as usize];
    let mut rankVal: rankVal_t =
        [[0u32; (HUFv05_ABSOLUTEMAX_TABLELOG + 1) as usize]; HUFv05_ABSOLUTEMAX_TABLELOG as usize];
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let sizeOfSort: U32;
    let mut nbSymbols: U32 = 0;
    let memLog: U32 = *DTable.wrapping_add(0);
    let iSize: size_t;
    let dtPtr = DTable as *mut c_void;
    let dt = (dtPtr as *mut HUFv05_DEltX4).wrapping_add(1);

    // rankStart = rankStart0 + 1
    let rankStart = rankStart0.as_mut_ptr().wrapping_add(1);

    if memLog > HUFv05_ABSOLUTEMAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUFv05_readStats(
        weightList.as_mut_ptr(),
        (HUFv05_MAX_SYMBOL_VALUE + 1) as size_t,
        rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if HUFv05_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > memLog {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* find maxWeight */
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 {
        maxW = maxW.wrapping_sub(1);
    }

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w <= maxW {
            let current = nextRankStart;
            nextRankStart += rankStats[w as usize];
            *rankStart.wrapping_add(w as usize) = current;
            w += 1;
        }
        *rankStart.wrapping_add(0) = nextRankStart; /* put all 0w symbols at the end of sorted list*/
        sizeOfSort = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32;
        s = 0;
        while s < nbSymbols {
            let w: U32 = weightList[s as usize] as U32;
            let r: U32 = *rankStart.wrapping_add(w as usize);
            *rankStart.wrapping_add(w as usize) = (*rankStart.wrapping_add(w as usize)) + 1;
            sortedSymbol[r as usize].symbol = s as BYTE;
            sortedSymbol[r as usize].weight = w as BYTE;
            s += 1;
        }
        *rankStart.wrapping_add(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let minBits: U32 = tableLog + 1 - maxW;
        let mut nextRankVal: U32 = 0;
        let mut w: U32;
        let mut consumed: U32;
        let rescale: i32 = (memLog as i32 - tableLog as i32) - 1; /* tableLog <= memLog */
        // U32* rankVal0 = rankVal[0];
        w = 1;
        while w <= maxW {
            let current = nextRankVal;
            nextRankVal += rankStats[w as usize] << ((w as i32 + rescale) as u32);
            rankVal[0][w as usize] = current;
            w += 1;
        }
        consumed = minBits;
        while consumed <= memLog - minBits {
            // U32* rankValPtr = rankVal[consumed];
            w = 1;
            while w <= maxW {
                rankVal[consumed as usize][w as usize] = rankVal[0][w as usize] >> consumed;
                w += 1;
            }
            consumed += 1;
        }
    }

    HUFv05_fillDTableX4(
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

#[inline]
unsafe fn HUFv05_decodeSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv05_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 2);
    BITv05_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    (*dt.wrapping_add(val)).length as U32
}

#[inline]
unsafe fn HUFv05_decodeLastSymbolX4(
    op: *mut c_void,
    DStream: *mut BITv05_DStream_t,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> U32 {
    let val = BITv05_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    memcpy(op, dt.wrapping_add(val) as *const c_void, 1);
    if (*dt.wrapping_add(val)).length == 1 {
        BITv05_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (core::mem::size_of::<size_t>() * 8) as u32 {
            BITv05_skipBits(DStream, (*dt.wrapping_add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (core::mem::size_of::<size_t>() * 8) as u32 {
                (*DStream).bitsConsumed = (core::mem::size_of::<size_t>() * 8) as u32;
            }
        }
    }
    1
}

unsafe fn HUFv05_decodeStreamX4(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv05_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv05_DEltX4,
    dtLog: U32,
) -> size_t {
    let pStart = p;

    macro_rules! DECODE0 {
        () => {{
            p = p.wrapping_add(HUFv05_decodeSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
        }};
    }
    macro_rules! DECODE1 {
        () => {
            // if (MEM_64bits() || (HUFv05_MAX_TABLELOG<=12))
            DECODE0!()
        };
    }
    macro_rules! DECODE2 {
        () => {
            if MEM_64bits() != 0 {
                DECODE0!()
            }
        };
    }

    /* up to 8 symbols at a time */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && (p < pEnd.wrapping_sub(7))
    {
        DECODE2!();
        DECODE1!();
        DECODE2!();
        DECODE0!();
    }

    /* closer to the end */
    while (BITv05_reloadDStream(bitDPtr) == BITv05_DStream_unfinished)
        && (p <= pEnd.wrapping_sub(2))
    {
        DECODE0!();
    }

    while p <= pEnd.wrapping_sub(2) {
        DECODE0!(); /* no need to reload : reached the end of DStream */
    }

    if p < pEnd {
        p = p.wrapping_add(HUFv05_decodeLastSymbolX4(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    p.offset_from(pStart) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X4_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const u32,
) -> size_t {
    let istart = cSrc as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.wrapping_add(dstSize);

    let dtLog: U32 = *DTable.wrapping_add(0);
    let dtPtr = DTable as *const c_void;
    let dt = (dtPtr as *const HUFv05_DEltX4).wrapping_add(1);
    let errorCode: size_t;

    let mut bitD: BITv05_DStream_t = core::mem::zeroed();
    errorCode = BITv05_initDStream(&mut bitD, istart as *const c_void, cSrcSize);
    if HUFv05_isError(errorCode) != 0 {
        return errorCode;
    }

    HUFv05_decodeStreamX4(ostart, &mut bitD, oend, dt, dtLog);

    /* check */
    if BITv05_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress1X4(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable = [0u32; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv05_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv05_decompress1X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X4_usingDTable(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const u32,
) -> size_t {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);
        let dtPtr = DTable as *const c_void;
        let dt = (dtPtr as *const HUFv05_DEltX4).wrapping_add(1);
        let dtLog: U32 = *DTable.wrapping_add(0);
        let mut errorCode: size_t;

        let mut bitD1: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD2: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD3: BITv05_DStream_t = core::mem::zeroed();
        let mut bitD4: BITv05_DStream_t = core::mem::zeroed();
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
            return ERROR(ZSTD_error_corruption_detected);
        }
        errorCode = BITv05_initDStream(&mut bitD1, istart1 as *const c_void, length1);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD2, istart2 as *const c_void, length2);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD3, istart3 as *const c_void, length3);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }
        errorCode = BITv05_initDStream(&mut bitD4, istart4 as *const c_void, length4);
        if HUFv05_isError(errorCode) != 0 {
            return errorCode;
        }

        macro_rules! DECODE0 {
            ($op:expr, $bd:expr) => {{
                $op = $op
                    .wrapping_add(HUFv05_decodeSymbolX4($op as *mut c_void, $bd, dt, dtLog) as usize);
            }};
        }
        macro_rules! DECODE1 {
            ($op:expr, $bd:expr) => {
                DECODE0!($op, $bd)
            };
        }
        macro_rules! DECODE2 {
            ($op:expr, $bd:expr) => {
                if MEM_64bits() != 0 {
                    DECODE0!($op, $bd)
                }
            };
        }

        /* 16-32 symbols per loop */
        endSignal = BITv05_reloadDStream(&mut bitD1)
            | BITv05_reloadDStream(&mut bitD2)
            | BITv05_reloadDStream(&mut bitD3)
            | BITv05_reloadDStream(&mut bitD4);
        while (endSignal == BITv05_DStream_unfinished) && (op4 < oend.wrapping_sub(7)) {
            DECODE2!(op1, &mut bitD1);
            DECODE2!(op2, &mut bitD2);
            DECODE2!(op3, &mut bitD3);
            DECODE2!(op4, &mut bitD4);
            DECODE1!(op1, &mut bitD1);
            DECODE1!(op2, &mut bitD2);
            DECODE1!(op3, &mut bitD3);
            DECODE1!(op4, &mut bitD4);
            DECODE2!(op1, &mut bitD1);
            DECODE2!(op2, &mut bitD2);
            DECODE2!(op3, &mut bitD3);
            DECODE2!(op4, &mut bitD4);
            DECODE0!(op1, &mut bitD1);
            DECODE0!(op2, &mut bitD2);
            DECODE0!(op3, &mut bitD3);
            DECODE0!(op4, &mut bitD4);

            endSignal = BITv05_reloadDStream(&mut bitD1)
                | BITv05_reloadDStream(&mut bitD2)
                | BITv05_reloadDStream(&mut bitD3)
                | BITv05_reloadDStream(&mut bitD4);
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

        /* finish bitStreams one by one */
        HUFv05_decodeStreamX4(op1, &mut bitD1, opStart2, dt, dtLog);
        HUFv05_decodeStreamX4(op2, &mut bitD2, opStart3, dt, dtLog);
        HUFv05_decodeStreamX4(op3, &mut bitD3, opStart4, dt, dtLog);
        HUFv05_decodeStreamX4(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        endSignal = BITv05_endOfDStream(&bitD1)
            & BITv05_endOfDStream(&bitD2)
            & BITv05_endOfDStream(&bitD3)
            & BITv05_endOfDStream(&bitD4);
        if endSignal == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        dstSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress4X4(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable = [0u32; HUFv05_DTABLE_SIZE(HUFv05_MAX_TABLELOG)];
    DTable[0] = HUFv05_MAX_TABLELOG;
    let mut ip = cSrc as *const BYTE;

    let hSize = HUFv05_readDTableX4(DTable.as_mut_ptr(), cSrc, cSrcSize);
    if HUFv05_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUFv05_decompress4X4_usingDTable(dst, dstSize, ip as *const c_void, cSrcSize, DTable.as_ptr())
}

// -------- Generic decompression selector --------

#[repr(C)]
#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
}

static algoTime: [[algo_time_t; 3]; 16] = [
    [at(0, 0), at(1, 1), at(2, 2)],
    [at(0, 0), at(1, 1), at(2, 2)],
    [at(38, 130), at(1313, 74), at(2151, 38)],
    [at(448, 128), at(1353, 74), at(2238, 41)],
    [at(556, 128), at(1353, 74), at(2238, 47)],
    [at(714, 128), at(1418, 74), at(2436, 53)],
    [at(883, 128), at(1437, 74), at(2464, 61)],
    [at(897, 128), at(1515, 75), at(2622, 68)],
    [at(926, 128), at(1613, 75), at(2730, 75)],
    [at(947, 128), at(1729, 77), at(3359, 77)],
    [at(1107, 128), at(2083, 81), at(4006, 84)],
    [at(1177, 128), at(2379, 87), at(4785, 88)],
    [at(1242, 128), at(2415, 93), at(5155, 84)],
    [at(1349, 128), at(2644, 106), at(5260, 106)],
    [at(1455, 128), at(2422, 124), at(4174, 124)],
    [at(722, 128), at(1891, 145), at(1936, 146)],
];

const fn at(t: U32, d: U32) -> algo_time_t {
    algo_time_t {
        tableTime: t,
        decode256Time: d,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv05_decompress(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
) -> size_t {
    // static const decompressionAlgo decompress[3] = { HUFv05_decompress4X2, HUFv05_decompress4X4, NULL };
    let mut Q: U32;
    let D256: U32 = (dstSize >> 8) as U32;
    let mut Dtime = [0u32; 3];
    let mut algoNb: U32 = 0;
    let mut n: i32;

    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize >= dstSize {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if cSrcSize == 1 {
        memset(dst, *(cSrc as *const BYTE) as i32, dstSize);
        return dstSize;
    } /* RLE */

    /* decoder timing evaluation */
    Q = (cSrcSize * 16 / dstSize) as U32; /* Q < 16 since dstSize > cSrcSize */
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
        HUFv05_decompress4X2(dst, dstSize, cSrc, cSrcSize)
    } else {
        HUFv05_decompress4X4(dst, dstSize, cSrc, cSrcSize)
    }
}

// ============================================================================
// ZSTDv05 decompression
// ============================================================================

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTDv05_parameters {
    pub srcSize: U64,
    pub windowLog: U32,
    pub contentLog: U32,
    pub hashLog: U32,
    pub searchLog: U32,
    pub searchLength: U32,
    pub targetLength: U32,
    pub strategy: U32, // ZSTDv05_strategy enum
}

#[repr(C)]
#[derive(Clone, Copy)]
struct blockProperties_t {
    blockType: blockType_t,
    origSize: U32,
}

// ZSTDv05_dStage
type ZSTDv05_dStage = u32;
const ZSTDv05ds_getFrameHeaderSize: ZSTDv05_dStage = 0;
const ZSTDv05ds_decodeFrameHeader: ZSTDv05_dStage = 1;
const ZSTDv05ds_decodeBlockHeader: ZSTDv05_dStage = 2;
const ZSTDv05ds_decompressBlock: ZSTDv05_dStage = 3;

#[repr(C)]
pub struct ZSTDv05_DCtx {
    LLTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(LLFSEv05Log)],
    OffTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(OffFSEv05Log)],
    MLTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(MLFSEv05Log)],
    hufTableX4: [u32; HUFv05_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    previousDstEnd: *const c_void,
    base: *const c_void,
    vBase: *const c_void,
    dictEnd: *const c_void,
    expected: size_t,
    headerSize: size_t,
    params: ZSTDv05_parameters,
    bType: blockType_t,
    stage: ZSTDv05_dStage,
    flagStaticTables: U32,
    litPtr: *const BYTE,
    litSize: size_t,
    litBuffer: [BYTE; BLOCKSIZE + WILDCOPY_OVERLENGTH],
    headerBuffer: [BYTE; ZSTDv05_frameHeaderSize_max],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_isError(code: size_t) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_sizeofDCtx() -> size_t {
    core::mem::size_of::<ZSTDv05_DCtx>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBegin(dctx: *mut ZSTDv05_DCtx) -> size_t {
    (*dctx).expected = ZSTDv05_frameHeaderSize_min;
    (*dctx).stage = ZSTDv05ds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).hufTableX4[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG;
    (*dctx).flagStaticTables = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_createDCtx() -> *mut ZSTDv05_DCtx {
    let dctx = malloc(core::mem::size_of::<ZSTDv05_DCtx>()) as *mut ZSTDv05_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDv05_decompressBegin(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_freeDCtx(dctx: *mut ZSTDv05_DCtx) -> size_t {
    free(dctx as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_copyDCtx(
    dstDCtx: *mut ZSTDv05_DCtx,
    srcDCtx: *const ZSTDv05_DCtx,
) {
    memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv05_DCtx>()
            - (BLOCKSIZE + WILDCOPY_OVERLENGTH + ZSTDv05_frameHeaderSize_max),
    );
}

// -------- Frame/Block decoding --------

unsafe fn ZSTDv05_decodeFrameHeader_Part1(
    zc: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let magicNumber: U32;
    if srcSize != ZSTDv05_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTDv05_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    (*zc).headerSize = ZSTDv05_frameHeaderSize_min;
    (*zc).headerSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_getFrameParams(
    params: *mut ZSTDv05_parameters,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let magicNumber: U32;
    if srcSize < ZSTDv05_frameHeaderSize_min {
        return ZSTDv05_frameHeaderSize_max;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTDv05_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    memset(params as *mut c_void, 0, core::mem::size_of::<ZSTDv05_parameters>());
    (*params).windowLog =
        (*(src as *const BYTE).wrapping_add(4) as U32 & 15) + ZSTDv05_WINDOWLOG_ABSOLUTEMIN;
    if (*(src as *const BYTE).wrapping_add(4) as U32 >> 4) != 0 {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    0
}

unsafe fn ZSTDv05_decodeFrameHeader_Part2(
    zc: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let result: size_t;
    if srcSize != (*zc).headerSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    result = ZSTDv05_getFrameParams(&mut (*zc).params, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).params.windowLog > 25) {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    result
}

unsafe fn ZSTDv05_getcBlockSize(
    src: *const c_void,
    srcSize: size_t,
    bpPtr: *mut blockProperties_t,
) -> size_t {
    let in_ = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    headerFlags = *in_;
    cSize = *in_.wrapping_add(2) as U32
        + ((*in_.wrapping_add(1) as U32) << 8)
        + (((*in_.wrapping_add(0) as U32) & 7) << 16);

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

unsafe fn ZSTDv05_copyRawBlock(
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    if dst.is_null() {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    memcpy(dst, src, srcSize);
    srcSize
}

unsafe fn ZSTDv05_decodeLiteralsBlock(
    dctx: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let istart = src as *const BYTE;

    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart.wrapping_add(0) as U32) >> 6 {
        x if x == IS_HUFv05 => {
            let litSize: size_t;
            let litCSize: size_t;
            let mut singleStream: size_t = 0;
            let lhSize0: U32 = ((*istart.wrapping_add(0) as U32) >> 4) & 3;
            let lhSize: U32;
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            match lhSize0 {
                2 => {
                    lhSize = 4;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 10)
                        + ((*istart.wrapping_add(1) as size_t) << 2)
                        + ((*istart.wrapping_add(2) as size_t) >> 6);
                    litCSize = (((*istart.wrapping_add(2) as size_t) & 63) << 8)
                        + *istart.wrapping_add(3) as size_t;
                }
                3 => {
                    lhSize = 5;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 14)
                        + ((*istart.wrapping_add(1) as size_t) << 6)
                        + ((*istart.wrapping_add(2) as size_t) >> 2);
                    litCSize = (((*istart.wrapping_add(2) as size_t) & 3) << 16)
                        + ((*istart.wrapping_add(3) as size_t) << 8)
                        + *istart.wrapping_add(4) as size_t;
                }
                _ => {
                    // case 0, case 1, default
                    lhSize = 3;
                    singleStream = (*istart.wrapping_add(0) as size_t) & 16;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 6)
                        + ((*istart.wrapping_add(1) as size_t) >> 2);
                    litCSize = (((*istart.wrapping_add(1) as size_t) & 3) << 8)
                        + *istart.wrapping_add(2) as size_t;
                }
            }
            if litSize > BLOCKSIZE {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if litCSize + lhSize as size_t > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            let hufResult = if singleStream != 0 {
                HUFv05_decompress1X2(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            } else {
                HUFv05_decompress(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                )
            };
            if HUFv05_isError(hufResult) != 0 {
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
            let errorCode: size_t;
            let litSize: size_t;
            let litCSize: size_t;
            let lhSize0: U32 = ((*istart.wrapping_add(0) as U32) >> 4) & 3;
            let lhSize: U32;
            if lhSize0 != 1 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if (*dctx).flagStaticTables == 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }

            lhSize = 3;
            litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 6)
                + ((*istart.wrapping_add(1) as size_t) >> 2);
            litCSize =
                (((*istart.wrapping_add(1) as size_t) & 3) << 8) + *istart.wrapping_add(2) as size_t;
            if litCSize + lhSize as size_t > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            errorCode = HUFv05_decompress1X4_usingDTable(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                litSize,
                istart.wrapping_add(lhSize as usize) as *const c_void,
                litCSize,
                (*dctx).hufTableX4.as_ptr(),
            );
            if HUFv05_isError(errorCode) != 0 {
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
        x if x == IS_RAW => {
            let litSize: size_t;
            let lhSize0: U32 = ((*istart.wrapping_add(0) as U32) >> 4) & 3;
            let lhSize: U32;
            match lhSize0 {
                2 => {
                    lhSize = 2;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 8)
                        + *istart.wrapping_add(1) as size_t;
                }
                3 => {
                    lhSize = 3;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 16)
                        + ((*istart.wrapping_add(1) as size_t) << 8)
                        + *istart.wrapping_add(2) as size_t;
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.wrapping_add(0) as size_t) & 31;
                }
            }

            if lhSize as size_t + litSize + WILDCOPY_OVERLENGTH > srcSize {
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
            let lhSize0: U32 = ((*istart.wrapping_add(0) as U32) >> 4) & 3;
            let lhSize: U32;
            match lhSize0 {
                2 => {
                    // lhSize retains its parsed value (2) in this case
                    lhSize = lhSize0;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 8)
                        + *istart.wrapping_add(1) as size_t;
                }
                3 => {
                    // lhSize retains its parsed value (3) in this case
                    lhSize = lhSize0;
                    litSize = (((*istart.wrapping_add(0) as size_t) & 15) << 16)
                        + ((*istart.wrapping_add(1) as size_t) << 8)
                        + *istart.wrapping_add(2) as size_t;
                    if srcSize < 4 {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                }
                _ => {
                    lhSize = 1;
                    litSize = (*istart.wrapping_add(0) as size_t) & 31;
                }
            }
            if litSize > BLOCKSIZE {
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
        _ => ERROR(ZSTD_error_corruption_detected),
    }
}

unsafe fn ZSTDv05_decodeSeqHeaders(
    nbSeq: *mut i32,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut size_t,
    DTableLL: *mut FSEv05_DTable,
    DTableML: *mut FSEv05_DTable,
    DTableOffb: *mut FSEv05_DTable,
    src: *const c_void,
    srcSize: size_t,
    flagStaticTable: U32,
) -> size_t {
    let istart = src as *const BYTE;
    let mut ip = istart;
    let iend = istart.wrapping_add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: u32 = 0;
    let mut Offlog: u32 = 0;
    let mut MLlog: u32 = 0;
    let mut dumpsLength: size_t;

    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    *nbSeq = *ip as i32;
    ip = ip.wrapping_add(1);
    if *nbSeq == 0 {
        return 1;
    }
    if *nbSeq >= 128 {
        if ip >= iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        *nbSeq = ((*nbSeq - 128) << 8) + *ip as i32;
        ip = ip.wrapping_add(1);
    }

    if ip >= iend {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    LLtype = (*ip as U32) >> 6;
    Offtype = ((*ip as U32) >> 4) & 3;
    MLtype = ((*ip as U32) >> 2) & 3;
    if *ip & 2 != 0 {
        if ip.wrapping_add(3) > iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        dumpsLength = *ip.wrapping_add(2) as size_t;
        dumpsLength += (*ip.wrapping_add(1) as size_t) << 8;
        ip = ip.wrapping_add(3);
    } else {
        if ip.wrapping_add(2) > iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        dumpsLength = *ip.wrapping_add(1) as size_t;
        dumpsLength += ((*ip.wrapping_add(0) as size_t) & 1) << 8;
        ip = ip.wrapping_add(2);
    }
    *dumpsPtr = ip;
    ip = ip.wrapping_add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    /* check */
    if ip > iend.wrapping_sub(3) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* sequences */
    {
        let mut norm = [0i16; (MaxML + 1) as usize];
        let mut headerSize: size_t;

        /* Build DTables */
        match LLtype {
            x if x == FSEv05_ENCODING_RLE => {
                LLlog = 0;
                FSEv05_buildDTable_rle(DTableLL, *ip);
                ip = ip.wrapping_add(1);
            }
            x if x == FSEv05_ENCODING_RAW => {
                LLlog = LLbits;
                FSEv05_buildDTable_raw(DTableLL, LLbits);
            }
            x if x == FSEv05_ENCODING_STATIC => {
                if flagStaticTable == 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            _ => {
                // FSEv05_ENCODING_DYNAMIC / default
                let mut max: u32 = MaxLL;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut LLlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as size_t,
                );
                if FSEv05_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if LLlog > LLFSEv05Log {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.wrapping_add(headerSize);
                FSEv05_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
            }
        }

        match Offtype {
            x if x == FSEv05_ENCODING_RLE => {
                Offlog = 0;
                if ip > iend.wrapping_sub(2) {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                FSEv05_buildDTable_rle(DTableOffb, *ip & MaxOff as BYTE);
                ip = ip.wrapping_add(1);
            }
            x if x == FSEv05_ENCODING_RAW => {
                Offlog = Offbits;
                FSEv05_buildDTable_raw(DTableOffb, Offbits);
            }
            x if x == FSEv05_ENCODING_STATIC => {
                if flagStaticTable == 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            _ => {
                let mut max: u32 = MaxOff;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut Offlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as size_t,
                );
                if FSEv05_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if Offlog > OffFSEv05Log {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.wrapping_add(headerSize);
                FSEv05_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
            }
        }

        match MLtype {
            x if x == FSEv05_ENCODING_RLE => {
                MLlog = 0;
                if ip > iend.wrapping_sub(2) {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                FSEv05_buildDTable_rle(DTableML, *ip);
                ip = ip.wrapping_add(1);
            }
            x if x == FSEv05_ENCODING_RAW => {
                MLlog = MLbits;
                FSEv05_buildDTable_raw(DTableML, MLbits);
            }
            x if x == FSEv05_ENCODING_STATIC => {
                if flagStaticTable == 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            _ => {
                let mut max: u32 = MaxML;
                headerSize = FSEv05_readNCount(
                    norm.as_mut_ptr(),
                    &mut max,
                    &mut MLlog,
                    ip as *const c_void,
                    iend.offset_from(ip) as size_t,
                );
                if FSEv05_isError(headerSize) != 0 {
                    return ERROR(ZSTD_error_GENERIC);
                }
                if MLlog > MLFSEv05Log {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                ip = ip.wrapping_add(headerSize);
                FSEv05_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
            }
        }
        let _ = (LLlog, Offlog, MLlog);
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
    DStream: BITv05_DStream_t,
    stateLL: FSEv05_DState_t,
    stateOffb: FSEv05_DState_t,
    stateML: FSEv05_DState_t,
    prevOffset: size_t,
    dumps: *const BYTE,
    dumpsEnd: *const BYTE,
}

unsafe fn ZSTDv05_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let mut litLength: size_t;
    let prevOffset: size_t;
    let mut offset: size_t;
    let mut matchLength: size_t;
    let mut dumps: *const BYTE = (*seqState).dumps;
    let de: *const BYTE = (*seqState).dumpsEnd;

    /* Literal length */
    litLength = FSEv05_peakSymbol(&mut (*seqState).stateLL) as size_t;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    if litLength == MaxLL as size_t {
        let add: U32 = *dumps as U32;
        dumps = dumps.wrapping_add(1);
        if add < 255 {
            litLength += add as size_t;
        } else if dumps.wrapping_add(2) <= de {
            litLength = MEM_readLE16(dumps as *const c_void) as size_t;
            dumps = dumps.wrapping_add(2);
            if (litLength & 1) != 0 && dumps < de {
                litLength += (*dumps as size_t) << 16;
                dumps = dumps.wrapping_add(1);
            }
            litLength >>= 1;
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1);
        }
    }

    /* Offset */
    {
        static offsetPrefix: [U32; (MaxOff + 1) as usize] = [
            1, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
            131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, 1, 1,
            1, 1, 1,
        ];
        let offsetCode: U32 = FSEv05_peakSymbol(&mut (*seqState).stateOffb) as U32;
        let mut nbBits: U32 = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0;
        }
        offset = (offsetPrefix[offsetCode as usize] as size_t)
            .wrapping_add(BITv05_readBits(&mut (*seqState).DStream, nbBits));
        if MEM_32bits() != 0 {
            BITv05_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset;
        }
        if (offsetCode | (litLength == 0) as U32) != 0 {
            (*seqState).prevOffset = (*seq).offset;
        }
        FSEv05_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream);
    }

    /* Literal length update */
    FSEv05_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream);
    if MEM_32bits() != 0 {
        BITv05_reloadDStream(&mut (*seqState).DStream);
    }

    /* MatchLength */
    matchLength = FSEv05_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as size_t;
    if matchLength == MaxML as size_t {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.wrapping_add(1);
            v
        } else {
            0
        };
        if add < 255 {
            matchLength += add as size_t;
        } else if dumps.wrapping_add(2) <= de {
            matchLength = MEM_readLE16(dumps as *const c_void) as size_t;
            dumps = dumps.wrapping_add(2);
            if (matchLength & 1) != 0 && dumps < de {
                matchLength += (*dumps as size_t) << 16;
                dumps = dumps.wrapping_add(1);
            }
            matchLength >>= 1;
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1);
        }
    }
    matchLength += MINMATCH;

    /* save result */
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

unsafe fn ZSTDv05_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    static dec32table: [i32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
    static dec64table: [i32; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
    let oLitEnd = op.wrapping_add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.wrapping_add(sequenceLength);
    let oend_8 = oend.wrapping_sub(8);
    let litEnd = (*litPtr).wrapping_add(sequence.litLength);
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
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if litEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* copy Literals */
    ZSTDv05_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    );
    op = oLitEnd;
    *litPtr = litEnd; /* update for next sequence */

    /* copy Match */
    if sequence.offset > oLitEnd.offset_from(base) as size_t {
        /* offset beyond prefix */
        if sequence.offset > oLitEnd.offset_from(vBase) as size_t {
            return ERROR(ZSTD_error_corruption_detected);
        }
        // match = dictEnd - (base - match)
        match_ = dictEnd.wrapping_sub(base.offset_from(match_) as usize);
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

    /* match within prefix */
    if sequence.offset < 8 {
        /* close range match, overlap */
        let sub2: i32 = dec64table[sequence.offset];
        *op.wrapping_add(0) = *match_.wrapping_add(0);
        *op.wrapping_add(1) = *match_.wrapping_add(1);
        *op.wrapping_add(2) = *match_.wrapping_add(2);
        *op.wrapping_add(3) = *match_.wrapping_add(3);
        match_ = match_.wrapping_offset(dec32table[sequence.offset] as isize);
        ZSTDv05_copy4(op.wrapping_add(4) as *mut c_void, match_ as *const c_void);
        match_ = match_.wrapping_offset(-(sub2 as isize));
    } else {
        ZSTDv05_copy8(op as *mut c_void, match_ as *const c_void);
    }
    op = op.wrapping_add(8);
    match_ = match_.wrapping_add(8);

    if oMatchEnd > oend.wrapping_sub(16 - MINMATCH) {
        if op < oend_8 {
            ZSTDv05_wildcopy(
                op as *mut c_void,
                match_ as *const c_void,
                oend_8.offset_from(op) as isize,
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
        ZSTDv05_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize - 8,
        );
    }
    sequenceLength
}

unsafe fn ZSTDv05_decompressSequences(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
) -> size_t {
    let mut ip = seqStart as *const BYTE;
    let iend = ip.wrapping_add(seqSize);
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let oend = ostart.wrapping_add(maxDstSize);
    let mut errorCode: size_t;
    let mut dumpsLength: size_t = 0;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd = litPtr.wrapping_add((*dctx).litSize);
    let mut nbSeq: i32 = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL = (*dctx).LLTable.as_mut_ptr();
    let DTableML = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb = (*dctx).OffTable.as_mut_ptr();
    let base = (*dctx).base as *const BYTE;
    let vBase = (*dctx).vBase as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;

    /* Build Decoding Tables */
    errorCode = ZSTDv05_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        seqSize,
        (*dctx).flagStaticTables,
    );
    if ZSTDv05_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.wrapping_add(errorCode);

    /* Regen sequences */
    if nbSeq != 0 {
        let mut sequence: seq_t = core::mem::zeroed();
        let mut seqState: seqState_t = core::mem::zeroed();

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        sequence.offset = REPCODE_STARTVALUE;
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.wrapping_add(dumpsLength);
        seqState.prevOffset = REPCODE_STARTVALUE;
        errorCode = BITv05_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            iend.offset_from(ip) as size_t,
        );
        if ERR_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSEv05_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSEv05_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSEv05_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BITv05_reloadDStream(&mut seqState.DStream) <= BITv05_DStream_completed) && nbSeq != 0
        {
            let oneSeqSize: size_t;
            nbSeq -= 1;
            ZSTDv05_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTDv05_execSequence(
                op, oend, sequence, &mut litPtr, litEnd, base, vBase, dictEnd,
            );
            if ZSTDv05_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.wrapping_add(oneSeqSize);
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

unsafe fn ZSTDv05_checkContinuity(dctx: *mut ZSTDv05_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char).wrapping_offset(
            -(((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char)),
        ) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

unsafe fn ZSTDv05_decompressBlock_internal(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip = src as *const BYTE;
    let litCSize: size_t;

    if srcSize >= BLOCKSIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals sub-block */
    litCSize = ZSTDv05_decodeLiteralsBlock(dctx, src, srcSize);
    if ZSTDv05_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.wrapping_add(litCSize);
    srcSize -= litCSize;

    ZSTDv05_decompressSequences(dctx, dst, dstCapacity, ip as *const c_void, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBlock(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTDv05_checkContinuity(dctx, dst);
    ZSTDv05_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize)
}

unsafe fn ZSTDv05_decompress_continueDCtx(
    dctx: *mut ZSTDv05_DCtx,
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
    let mut blockProperties: blockProperties_t = core::mem::zeroed();
    memset(
        &mut blockProperties as *mut blockProperties_t as *mut c_void,
        0,
        core::mem::size_of::<blockProperties_t>(),
    );

    /* Frame Header */
    {
        let mut frameHeaderSize: size_t;
        if srcSize < ZSTDv05_frameHeaderSize_min + ZSTDv05_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        frameHeaderSize =
            ZSTDv05_decodeFrameHeader_Part1(dctx, src, ZSTDv05_frameHeaderSize_min);
        if ZSTDv05_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize + ZSTDv05_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize -= frameHeaderSize;
        frameHeaderSize = ZSTDv05_decodeFrameHeader_Part2(dctx, src, frameHeaderSize);
        if ZSTDv05_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
    }

    /* Loop on each block */
    loop {
        let mut decodedSize: size_t = 0;
        let cBlockSize =
            ZSTDv05_getcBlockSize(ip as *const c_void, iend.offset_from(ip) as size_t, &mut blockProperties);
        if ZSTDv05_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.wrapping_add(ZSTDv05_blockHeaderSize);
        remainingSize -= ZSTDv05_blockHeaderSize;
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        match blockProperties.blockType {
            x if x == bt_compressed => {
                decodedSize = ZSTDv05_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_raw => {
                decodedSize = ZSTDv05_copyRawBlock(
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            x if x == bt_rle => {
                return ERROR(ZSTD_error_GENERIC);
            }
            x if x == bt_end => {
                /* end of frame */
                if remainingSize != 0 {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC);
            }
        }
        if cBlockSize == 0 {
            break;
        } /* bt_end */

        if ZSTDv05_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.wrapping_add(decodedSize);
        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
    }

    op.offset_from(ostart) as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv05_DCtx,
    refDCtx: *const ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTDv05_copyDCtx(dctx, refDCtx);
    ZSTDv05_checkContinuity(dctx, dst);
    ZSTDv05_decompress_continueDCtx(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompress_usingDict(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    ZSTDv05_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv05_checkContinuity(dctx, dst);
    ZSTDv05_decompress_continueDCtx(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressDCtx(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTDv05_decompress_usingDict(dctx, dst, maxDstSize, src, srcSize, core::ptr::null(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompress(
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    // ZSTDv05_HEAPMODE == 1
    let regenSize: size_t;
    let dctx = ZSTDv05_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv05_decompressDCtx(dctx, dst, maxDstSize, src, srcSize);
    ZSTDv05_freeDCtx(dctx);
    regenSize
}

unsafe fn ZSTD_errorFrameSizeInfoLegacy(cSize: *mut size_t, dBound: *mut u64, ret: size_t) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: size_t,
    cSize: *mut size_t,
    dBound: *mut u64,
) {
    let mut ip = src as *const BYTE;
    let mut remainingSize = srcSize;
    let mut nbBlocks: size_t = 0;
    let mut blockProperties: blockProperties_t = core::mem::zeroed();

    /* Frame Header */
    if srcSize < ZSTDv05_frameHeaderSize_min {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    if MEM_readLE32(src) != ZSTDv05_MAGICNUMBER {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
        return;
    }
    ip = ip.wrapping_add(ZSTDv05_frameHeaderSize_min);
    remainingSize -= ZSTDv05_frameHeaderSize_min;

    /* Loop on each block */
    loop {
        let cBlockSize =
            ZSTDv05_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv05_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTDv05_blockHeaderSize);
        remainingSize -= ZSTDv05_blockHeaderSize;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break;
        } /* bt_end */

        ip = ip.wrapping_add(cBlockSize);
        remainingSize -= cBlockSize;
        nbBlocks += 1;
    }

    *cSize = ip.offset_from(src as *const BYTE) as size_t;
    *dBound = (nbBlocks as u64).wrapping_mul(BLOCKSIZE as u64);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_nextSrcSizeToDecompress(dctx: *mut ZSTDv05_DCtx) -> size_t {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressContinue(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    /* Sanity check */
    if srcSize != (*dctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ZSTDv05_checkContinuity(dctx, dst);

    match (*dctx).stage {
        x if x == ZSTDv05ds_getFrameHeaderSize => {
            if srcSize != ZSTDv05_frameHeaderSize_min {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            (*dctx).headerSize =
                ZSTDv05_decodeFrameHeader_Part1(dctx, src, ZSTDv05_frameHeaderSize_min);
            if ZSTDv05_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            memcpy(
                (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTDv05_frameHeaderSize_min,
            );
            if (*dctx).headerSize > ZSTDv05_frameHeaderSize_min {
                return ERROR(ZSTD_error_GENERIC);
            }
            (*dctx).expected = 0;
            /* fallthrough to decodeFrameHeader */
            let result =
                ZSTDv05_decodeFrameHeader_Part2(dctx, (*dctx).headerBuffer.as_ptr() as *const c_void, (*dctx).headerSize);
            if ZSTDv05_isError(result) != 0 {
                return result;
            }
            (*dctx).expected = ZSTDv05_blockHeaderSize;
            (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
            0
        }
        x if x == ZSTDv05ds_decodeFrameHeader => {
            let result = ZSTDv05_decodeFrameHeader_Part2(
                dctx,
                (*dctx).headerBuffer.as_ptr() as *const c_void,
                (*dctx).headerSize,
            );
            if ZSTDv05_isError(result) != 0 {
                return result;
            }
            (*dctx).expected = ZSTDv05_blockHeaderSize;
            (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
            0
        }
        x if x == ZSTDv05ds_decodeBlockHeader => {
            let mut bp: blockProperties_t = core::mem::zeroed();
            let blockSize = ZSTDv05_getcBlockSize(src, ZSTDv05_blockHeaderSize, &mut bp);
            if ZSTDv05_isError(blockSize) != 0 {
                return blockSize;
            }
            if bp.blockType == bt_end {
                (*dctx).expected = 0;
                (*dctx).stage = ZSTDv05ds_getFrameHeaderSize;
            } else {
                (*dctx).expected = blockSize;
                (*dctx).bType = bp.blockType;
                (*dctx).stage = ZSTDv05ds_decompressBlock;
            }
            0
        }
        x if x == ZSTDv05ds_decompressBlock => {
            let rSize: size_t;
            match (*dctx).bType {
                y if y == bt_compressed => {
                    rSize = ZSTDv05_decompressBlock_internal(dctx, dst, maxDstSize, src, srcSize);
                }
                y if y == bt_raw => {
                    rSize = ZSTDv05_copyRawBlock(dst, maxDstSize, src, srcSize);
                }
                y if y == bt_rle => {
                    return ERROR(ZSTD_error_GENERIC);
                }
                y if y == bt_end => {
                    rSize = 0;
                }
                _ => {
                    return ERROR(ZSTD_error_GENERIC);
                }
            }
            (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
            (*dctx).expected = ZSTDv05_blockHeaderSize;
            if ZSTDv05_isError(rSize) != 0 {
                return rSize;
            }
            (*dctx).previousDstEnd = (dst as *mut c_char).wrapping_add(rSize) as *const c_void;
            rSize
        }
        _ => ERROR(ZSTD_error_GENERIC),
    }
}

unsafe fn ZSTDv05_refDictContent(dctx: *mut ZSTDv05_DCtx, dict: *const c_void, dictSize: size_t) {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const c_char).wrapping_offset(
        -(((*dctx).previousDstEnd as *const c_char).offset_from((*dctx).base as *const c_char)),
    ) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const c_char).wrapping_add(dictSize) as *const c_void;
}

unsafe fn ZSTDv05_loadEntropy(
    dctx: *mut ZSTDv05_DCtx,
    mut dict: *const c_void,
    mut dictSize: size_t,
) -> size_t {
    let hSize: size_t;
    let offcodeHeaderSize: size_t;
    let matchlengthHeaderSize: size_t;
    let mut errorCode: size_t;
    let litlengthHeaderSize: size_t;
    let mut offcodeNCount = [0i16; (MaxOff + 1) as usize];
    let mut offcodeMaxValue: u32 = MaxOff;
    let mut offcodeLog: u32 = 0;
    let mut matchlengthNCount = [0i16; (MaxML + 1) as usize];
    let mut matchlengthMaxValue: u32 = MaxML;
    let mut matchlengthLog: u32 = 0;
    let mut litlengthNCount = [0i16; (MaxLL + 1) as usize];
    let mut litlengthMaxValue: u32 = MaxLL;
    let mut litlengthLog: u32 = 0;

    hSize = HUFv05_readDTableX4((*dctx).hufTableX4.as_mut_ptr(), dict, dictSize);
    if HUFv05_isError(hSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(hSize) as *const c_void;
    dictSize -= hSize;

    offcodeHeaderSize = FSEv05_readNCount(
        offcodeNCount.as_mut_ptr(),
        &mut offcodeMaxValue,
        &mut offcodeLog,
        dict,
        dictSize,
    );
    if FSEv05_isError(offcodeHeaderSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if offcodeLog > OffFSEv05Log {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).OffTable.as_mut_ptr(),
        offcodeNCount.as_ptr(),
        offcodeMaxValue,
        offcodeLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(offcodeHeaderSize) as *const c_void;
    dictSize -= offcodeHeaderSize;

    matchlengthHeaderSize = FSEv05_readNCount(
        matchlengthNCount.as_mut_ptr(),
        &mut matchlengthMaxValue,
        &mut matchlengthLog,
        dict,
        dictSize,
    );
    if FSEv05_isError(matchlengthHeaderSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if matchlengthLog > MLFSEv05Log {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).MLTable.as_mut_ptr(),
        matchlengthNCount.as_ptr(),
        matchlengthMaxValue,
        matchlengthLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(matchlengthHeaderSize) as *const c_void;
    dictSize -= matchlengthHeaderSize;

    litlengthHeaderSize = FSEv05_readNCount(
        litlengthNCount.as_mut_ptr(),
        &mut litlengthMaxValue,
        &mut litlengthLog,
        dict,
        dictSize,
    );
    if litlengthLog > LLFSEv05Log {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if FSEv05_isError(litlengthHeaderSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).LLTable.as_mut_ptr(),
        litlengthNCount.as_ptr(),
        litlengthMaxValue,
        litlengthLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    (*dctx).flagStaticTables = 1;
    hSize + offcodeHeaderSize + matchlengthHeaderSize + litlengthHeaderSize
}

unsafe fn ZSTDv05_decompress_insertDictionary(
    dctx: *mut ZSTDv05_DCtx,
    mut dict: *const c_void,
    mut dictSize: size_t,
) -> size_t {
    let eSize: size_t;
    let magic: U32 = MEM_readLE32(dict);
    if magic != ZSTDv05_DICT_MAGIC {
        /* pure content mode */
        ZSTDv05_refDictContent(dctx, dict, dictSize);
        return 0;
    }
    /* load entropy tables */
    dict = (dict as *const c_char).wrapping_add(4) as *const c_void;
    dictSize -= 4;
    eSize = ZSTDv05_loadEntropy(dctx, dict, dictSize);
    if ZSTDv05_isError(eSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    /* reference dictionary content */
    dict = (dict as *const c_char).wrapping_add(eSize) as *const c_void;
    dictSize -= eSize;
    ZSTDv05_refDictContent(dctx, dict, dictSize);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBegin_usingDict(
    dctx: *mut ZSTDv05_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    let mut errorCode: size_t;
    errorCode = ZSTDv05_decompressBegin(dctx);
    if ZSTDv05_isError(errorCode) != 0 {
        return errorCode;
    }

    if !dict.is_null() && dictSize != 0 {
        errorCode = ZSTDv05_decompress_insertDictionary(dctx, dict, dictSize);
        if ZSTDv05_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }

    0
}

// ============================================================================
// ZBUFFv05 streaming API
// ============================================================================
const ZBUFFv05_blockHeaderSize: size_t = 3;

unsafe fn ZBUFFv05_limitCopy(
    dst: *mut c_void,
    maxDstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let length = if maxDstSize < srcSize {
        maxDstSize
    } else {
        srcSize
    };
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

// ZBUFFv05_dStage
type ZBUFFv05_dStage = u32;
const ZBUFFv05ds_init: ZBUFFv05_dStage = 0;
const ZBUFFv05ds_readHeader: ZBUFFv05_dStage = 1;
const ZBUFFv05ds_loadHeader: ZBUFFv05_dStage = 2;
const ZBUFFv05ds_decodeHeader: ZBUFFv05_dStage = 3;
const ZBUFFv05ds_read: ZBUFFv05_dStage = 4;
const ZBUFFv05ds_load: ZBUFFv05_dStage = 5;
const ZBUFFv05ds_flush: ZBUFFv05_dStage = 6;

#[repr(C)]
pub struct ZBUFFv05_DCtx {
    zc: *mut ZSTDv05_DCtx,
    params: ZSTDv05_parameters,
    inBuff: *mut c_char,
    inBuffSize: size_t,
    inPos: size_t,
    outBuff: *mut c_char,
    outBuffSize: size_t,
    outStart: size_t,
    outEnd: size_t,
    hPos: size_t,
    stage: ZBUFFv05_dStage,
    headerBuffer: [u8; ZSTDv05_frameHeaderSize_max],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_createDCtx() -> *mut ZBUFFv05_DCtx {
    let zbc = malloc(core::mem::size_of::<ZBUFFv05_DCtx>()) as *mut ZBUFFv05_DCtx;
    if zbc.is_null() {
        return core::ptr::null_mut();
    }
    memset(zbc as *mut c_void, 0, core::mem::size_of::<ZBUFFv05_DCtx>());
    (*zbc).zc = ZSTDv05_createDCtx();
    (*zbc).stage = ZBUFFv05ds_init;
    zbc
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_freeDCtx(zbc: *mut ZBUFFv05_DCtx) -> size_t {
    if zbc.is_null() {
        return 0;
    }
    ZSTDv05_freeDCtx((*zbc).zc);
    free((*zbc).inBuff as *mut c_void);
    free((*zbc).outBuff as *mut c_void);
    free(zbc as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressInitDictionary(
    zbc: *mut ZBUFFv05_DCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    (*zbc).stage = ZBUFFv05ds_readHeader;
    (*zbc).hPos = 0;
    (*zbc).inPos = 0;
    (*zbc).outStart = 0;
    (*zbc).outEnd = 0;
    ZSTDv05_decompressBegin_usingDict((*zbc).zc, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressInit(zbc: *mut ZBUFFv05_DCtx) -> size_t {
    ZBUFFv05_decompressInitDictionary(zbc, core::ptr::null(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressContinue(
    zbc: *mut ZBUFFv05_DCtx,
    dst: *mut c_void,
    maxDstSizePtr: *mut size_t,
    src: *const c_void,
    srcSizePtr: *mut size_t,
) -> size_t {
    let istart = src as *const c_char;
    let mut ip = istart;
    let iend = istart.wrapping_add(*srcSizePtr);
    let ostart = dst as *mut c_char;
    let mut op = ostart;
    let oend = ostart.wrapping_add(*maxDstSizePtr);
    let mut notDone: U32 = 1;

    // The C code is a `while (notDone) switch(zbc->stage) { ... }` with several
    // fall-through cases. We reproduce it with a `'restart` loop; a C `break`
    // becomes `continue 'restart`, and a fall-through simply proceeds to the
    // next block within the same iteration (guarded by the current stage).
    'restart: while notDone != 0 {
        // --- ZBUFFv05ds_init ---
        if (*zbc).stage == ZBUFFv05ds_init {
            return ERROR(ZSTD_error_init_missing);
        }

        // --- ZBUFFv05ds_readHeader --- (ends with `break;`)
        if (*zbc).stage == ZBUFFv05ds_readHeader {
            let headerSize = ZSTDv05_getFrameParams(&mut (*zbc).params, src, *srcSizePtr);
            if ZSTDv05_isError(headerSize) != 0 {
                return headerSize;
            }
            if headerSize != 0 {
                memcpy(
                    (*zbc).headerBuffer.as_mut_ptr().wrapping_add((*zbc).hPos) as *mut c_void,
                    src,
                    *srcSizePtr,
                );
                (*zbc).hPos += *srcSizePtr;
                *maxDstSizePtr = 0;
                (*zbc).stage = ZBUFFv05ds_loadHeader;
                return headerSize - (*zbc).hPos;
            }
            (*zbc).stage = ZBUFFv05ds_decodeHeader;
            continue 'restart; /* break; */
        }

        // --- ZBUFFv05ds_loadHeader --- (falls through to decodeHeader)
        if (*zbc).stage == ZBUFFv05ds_loadHeader {
            let mut headerSize = ZBUFFv05_limitCopy(
                (*zbc).headerBuffer.as_mut_ptr().wrapping_add((*zbc).hPos) as *mut c_void,
                ZSTDv05_frameHeaderSize_max - (*zbc).hPos,
                src,
                *srcSizePtr,
            );
            (*zbc).hPos += headerSize;
            ip = ip.wrapping_add(headerSize);
            headerSize = ZSTDv05_getFrameParams(
                &mut (*zbc).params,
                (*zbc).headerBuffer.as_ptr() as *const c_void,
                (*zbc).hPos,
            );
            if ZSTDv05_isError(headerSize) != 0 {
                return headerSize;
            }
            if headerSize != 0 {
                *maxDstSizePtr = 0;
                return headerSize - (*zbc).hPos;
            }
            /* fall-through to decodeHeader (stage still loadHeader, guard below matches) */
        }

        // --- ZBUFFv05ds_decodeHeader --- (falls through to read, or `break;` to load)
        if (*zbc).stage == ZBUFFv05ds_decodeHeader || (*zbc).stage == ZBUFFv05ds_loadHeader {
            /* apply header to create / resize buffers */
            {
                let neededOutSize: size_t = 1usize << (*zbc).params.windowLog;
                let neededInSize: size_t = BLOCKSIZE;
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
            if (*zbc).hPos != 0 {
                /* some data already loaded into headerBuffer : transfer into inBuff */
                memcpy(
                    (*zbc).inBuff as *mut c_void,
                    (*zbc).headerBuffer.as_ptr() as *const c_void,
                    (*zbc).hPos,
                );
                (*zbc).inPos = (*zbc).hPos;
                (*zbc).hPos = 0;
                (*zbc).stage = ZBUFFv05ds_load;
                continue 'restart; /* break; */
            }
            (*zbc).stage = ZBUFFv05ds_read;
            /* fall-through to read */
        }

        // --- ZBUFFv05ds_read --- (falls through to load, or `break;` various)
        if (*zbc).stage == ZBUFFv05ds_read {
            let neededInSize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
            if neededInSize == 0 {
                /* end of frame */
                (*zbc).stage = ZBUFFv05ds_init;
                notDone = 0;
                continue 'restart;
            }
            if (iend.offset_from(ip) as size_t) >= neededInSize {
                /* directly decode from src */
                let decodedSize = ZSTDv05_decompressContinue(
                    (*zbc).zc,
                    (*zbc).outBuff.wrapping_add((*zbc).outStart) as *mut c_void,
                    (*zbc).outBuffSize - (*zbc).outStart,
                    ip as *const c_void,
                    neededInSize,
                );
                if ZSTDv05_isError(decodedSize) != 0 {
                    return decodedSize;
                }
                ip = ip.wrapping_add(neededInSize);
                if decodedSize == 0 {
                    continue 'restart;
                } /* this was just a header */
                (*zbc).outEnd = (*zbc).outStart + decodedSize;
                (*zbc).stage = ZBUFFv05ds_flush;
                continue 'restart;
            }
            if ip == iend {
                notDone = 0;
                continue 'restart;
            } /* no more input */
            (*zbc).stage = ZBUFFv05ds_load;
            /* fall-through to load */
        }

        // --- ZBUFFv05ds_load --- (falls through to flush)
        if (*zbc).stage == ZBUFFv05ds_load {
            let neededInSize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
            let toLoad = neededInSize - (*zbc).inPos;
            let loadedSize: size_t;
            if toLoad > (*zbc).inBuffSize - (*zbc).inPos {
                return ERROR(ZSTD_error_corruption_detected);
            }
            loadedSize = ZBUFFv05_limitCopy(
                (*zbc).inBuff.wrapping_add((*zbc).inPos) as *mut c_void,
                toLoad,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
            );
            ip = ip.wrapping_add(loadedSize);
            (*zbc).inPos += loadedSize;
            if loadedSize < toLoad {
                notDone = 0;
                continue 'restart;
            } /* not enough input, wait for more */
            {
                let decodedSize = ZSTDv05_decompressContinue(
                    (*zbc).zc,
                    (*zbc).outBuff.wrapping_add((*zbc).outStart) as *mut c_void,
                    (*zbc).outBuffSize - (*zbc).outStart,
                    (*zbc).inBuff as *const c_void,
                    neededInSize,
                );
                if ZSTDv05_isError(decodedSize) != 0 {
                    return decodedSize;
                }
                (*zbc).inPos = 0; /* input is consumed */
                if decodedSize == 0 {
                    (*zbc).stage = ZBUFFv05ds_read;
                    continue 'restart;
                } /* this was just a header */
                (*zbc).outEnd = (*zbc).outStart + decodedSize;
                (*zbc).stage = ZBUFFv05ds_flush;
                /* fall-through to flush */
            }
        }

        // --- ZBUFFv05ds_flush ---
        if (*zbc).stage == ZBUFFv05ds_flush {
            let toFlushSize = (*zbc).outEnd - (*zbc).outStart;
            let flushedSize = ZBUFFv05_limitCopy(
                op as *mut c_void,
                oend.offset_from(op) as size_t,
                (*zbc).outBuff.wrapping_add((*zbc).outStart) as *const c_void,
                toFlushSize,
            );
            op = op.wrapping_add(flushedSize);
            (*zbc).outStart += flushedSize;
            if flushedSize == toFlushSize {
                (*zbc).stage = ZBUFFv05ds_read;
                if (*zbc).outStart + BLOCKSIZE > (*zbc).outBuffSize {
                    (*zbc).outStart = 0;
                    (*zbc).outEnd = 0;
                }
                continue 'restart;
            }
            /* cannot flush everything */
            notDone = 0;
            continue 'restart;
        }

        // default: impossible
        return ERROR(ZSTD_error_GENERIC);
    }

    *srcSizePtr = ip.offset_from(istart) as size_t;
    *maxDstSizePtr = op.offset_from(ostart) as size_t;

    {
        let mut nextSrcSizeHint = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
        if nextSrcSizeHint > ZBUFFv05_blockHeaderSize {
            nextSrcSizeHint += ZBUFFv05_blockHeaderSize;
        }
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbc).inPos);
        nextSrcSizeHint
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_isError(errorCode: size_t) -> u32 {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_getErrorName(errorCode: size_t) -> *const c_char {
    ERR_getErrorName(errorCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_recommendedDInSize() -> size_t {
    BLOCKSIZE + ZBUFFv05_blockHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_recommendedDOutSize() -> size_t {
    BLOCKSIZE
}
