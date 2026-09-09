//! Translation of `zstd_errors.h`, `common/error_private.h` and `common/error_private.c`
#![allow(dead_code)]

use core::ffi::{c_char, c_uint};

pub type ZSTD_ErrorCode = c_uint;

pub const ZSTD_error_no_error: ZSTD_ErrorCode = 0;
pub const ZSTD_error_GENERIC: ZSTD_ErrorCode = 1;
pub const ZSTD_error_prefix_unknown: ZSTD_ErrorCode = 10;
pub const ZSTD_error_version_unsupported: ZSTD_ErrorCode = 12;
pub const ZSTD_error_frameParameter_unsupported: ZSTD_ErrorCode = 14;
pub const ZSTD_error_frameParameter_windowTooLarge: ZSTD_ErrorCode = 16;
pub const ZSTD_error_corruption_detected: ZSTD_ErrorCode = 20;
pub const ZSTD_error_checksum_wrong: ZSTD_ErrorCode = 22;
pub const ZSTD_error_literals_headerWrong: ZSTD_ErrorCode = 24;
pub const ZSTD_error_dictionary_corrupted: ZSTD_ErrorCode = 30;
pub const ZSTD_error_dictionary_wrong: ZSTD_ErrorCode = 32;
pub const ZSTD_error_dictionaryCreation_failed: ZSTD_ErrorCode = 34;
pub const ZSTD_error_parameter_unsupported: ZSTD_ErrorCode = 40;
pub const ZSTD_error_parameter_combination_unsupported: ZSTD_ErrorCode = 41;
pub const ZSTD_error_parameter_outOfBound: ZSTD_ErrorCode = 42;
pub const ZSTD_error_tableLog_tooLarge: ZSTD_ErrorCode = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: ZSTD_ErrorCode = 46;
pub const ZSTD_error_maxSymbolValue_tooSmall: ZSTD_ErrorCode = 48;
pub const ZSTD_error_cannotProduce_uncompressedBlock: ZSTD_ErrorCode = 49;
pub const ZSTD_error_stabilityCondition_notRespected: ZSTD_ErrorCode = 50;
pub const ZSTD_error_stage_wrong: ZSTD_ErrorCode = 60;
pub const ZSTD_error_init_missing: ZSTD_ErrorCode = 62;
pub const ZSTD_error_memory_allocation: ZSTD_ErrorCode = 64;
pub const ZSTD_error_workSpace_tooSmall: ZSTD_ErrorCode = 66;
pub const ZSTD_error_dstSize_tooSmall: ZSTD_ErrorCode = 70;
pub const ZSTD_error_srcSize_wrong: ZSTD_ErrorCode = 72;
pub const ZSTD_error_dstBuffer_null: ZSTD_ErrorCode = 74;
pub const ZSTD_error_noForwardProgress_destFull: ZSTD_ErrorCode = 80;
pub const ZSTD_error_noForwardProgress_inputEmpty: ZSTD_ErrorCode = 82;
pub const ZSTD_error_frameIndex_tooLarge: ZSTD_ErrorCode = 100;
pub const ZSTD_error_seekableIO: ZSTD_ErrorCode = 102;
pub const ZSTD_error_dstBuffer_wrong: ZSTD_ErrorCode = 104;
pub const ZSTD_error_srcBuffer_wrong: ZSTD_ErrorCode = 105;
pub const ZSTD_error_sequenceProducer_failed: ZSTD_ErrorCode = 106;
pub const ZSTD_error_externalSequences_invalid: ZSTD_ErrorCode = 107;
pub const ZSTD_error_maxCode: ZSTD_ErrorCode = 120;

/// `ERROR(name)` == `(size_t)-ZSTD_error_##name`
#[inline(always)]
pub const fn ERROR(code: ZSTD_ErrorCode) -> usize {
    (0usize).wrapping_sub(code as usize)
}

#[inline(always)]
pub fn ERR_isError(code: usize) -> c_uint {
    (code > ERROR(ZSTD_error_maxCode)) as c_uint
}

#[inline(always)]
pub fn ERR_getErrorCode(code: usize) -> ZSTD_ErrorCode {
    if ERR_isError(code) == 0 {
        return 0;
    }
    (0usize).wrapping_sub(code) as ZSTD_ErrorCode
}

#[inline(always)]
pub fn ERR_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorString_impl(ERR_getErrorCode(code))
}

macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

pub fn ERR_getErrorString_impl(code: ZSTD_ErrorCode) -> *const c_char {
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
        ZSTD_error_noForwardProgress_destFull => cstr!(
            "Operation made no progress over multiple calls, due to output buffer being full"
        ),
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
        _ => cstr!("Unspecified error code"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ERR_getErrorString(code: ZSTD_ErrorCode) -> *const c_char {
    ERR_getErrorString_impl(code)
}
