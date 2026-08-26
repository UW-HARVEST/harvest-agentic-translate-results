//! Translation of `common/error_private.h` + `common/error_private.c`
#![allow(dead_code)]

use core::ffi::c_char;

/* ZSTD_ErrorCode values, from zstd_errors.h */
pub const ZSTD_error_no_error: i32 = 0;
pub const ZSTD_error_GENERIC: i32 = 1;
pub const ZSTD_error_prefix_unknown: i32 = 10;
pub const ZSTD_error_version_unsupported: i32 = 12;
pub const ZSTD_error_frameParameter_unsupported: i32 = 14;
pub const ZSTD_error_frameParameter_windowTooLarge: i32 = 16;
pub const ZSTD_error_corruption_detected: i32 = 20;
pub const ZSTD_error_checksum_wrong: i32 = 22;
pub const ZSTD_error_literals_headerWrong: i32 = 24;
pub const ZSTD_error_dictionary_corrupted: i32 = 30;
pub const ZSTD_error_dictionary_wrong: i32 = 32;
pub const ZSTD_error_dictionaryCreation_failed: i32 = 34;
pub const ZSTD_error_parameter_unsupported: i32 = 40;
pub const ZSTD_error_parameter_combination_unsupported: i32 = 41;
pub const ZSTD_error_parameter_outOfBound: i32 = 42;
pub const ZSTD_error_tableLog_tooLarge: i32 = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: i32 = 46;
pub const ZSTD_error_maxSymbolValue_tooSmall: i32 = 48;
pub const ZSTD_error_cannotProduce_uncompressedBlock: i32 = 49;
pub const ZSTD_error_stabilityCondition_notRespected: i32 = 50;
pub const ZSTD_error_stage_wrong: i32 = 60;
pub const ZSTD_error_init_missing: i32 = 62;
pub const ZSTD_error_memory_allocation: i32 = 64;
pub const ZSTD_error_workSpace_tooSmall: i32 = 66;
pub const ZSTD_error_dstSize_tooSmall: i32 = 70;
pub const ZSTD_error_srcSize_wrong: i32 = 72;
pub const ZSTD_error_dstBuffer_null: i32 = 74;
pub const ZSTD_error_noForwardProgress_destFull: i32 = 80;
pub const ZSTD_error_noForwardProgress_inputEmpty: i32 = 82;
pub const ZSTD_error_frameIndex_tooLarge: i32 = 100;
pub const ZSTD_error_seekableIO: i32 = 102;
pub const ZSTD_error_dstBuffer_wrong: i32 = 104;
pub const ZSTD_error_srcBuffer_wrong: i32 = 105;
pub const ZSTD_error_sequenceProducer_failed: i32 = 106;
pub const ZSTD_error_externalSequences_invalid: i32 = 107;
pub const ZSTD_error_maxCode: i32 = 120;

/// `ERROR(name)` / `ZSTD_ERROR(name)` : `((size_t)-PREFIX(name))`
#[inline(always)]
pub const fn ERROR(code: i32) -> usize {
    (-(code as isize)) as usize
}

#[inline(always)]
pub const fn ERR_isError(code: usize) -> u32 {
    (code > ERROR(ZSTD_error_maxCode)) as u32
}

#[inline(always)]
pub const fn ERR_getErrorCode(code: usize) -> i32 {
    if ERR_isError(code) == 0 {
        return 0;
    }
    (0usize.wrapping_sub(code)) as i32
}

#[inline(always)]
pub unsafe fn ERR_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorString(ERR_getErrorCode(code))
}

macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

/// `const char* ERR_getErrorString(ERR_enum code)`
#[unsafe(no_mangle)]
pub extern "C" fn ERR_getErrorString(code: i32) -> *const c_char {
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
