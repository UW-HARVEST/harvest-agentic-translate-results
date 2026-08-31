//! Translation of `zstd_errors.h`, `common/error_private.{h,c}` and the error /
//! version part of `common/zstd_common.c`.
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_uint};

/* ===== ZSTD_ErrorCode (zstd_errors.h) ===== */
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

/// `ERROR(name)` / `ZSTD_ERROR(name)` == `(size_t)-code`
#[inline(always)]
pub const fn err_code(code: ZSTD_ErrorCode) -> usize {
    (0isize - code as isize) as usize
}

/// `ERR_isError()`
#[inline(always)]
pub fn err_is_error(code: usize) -> bool {
    code > err_code(ZSTD_error_maxCode)
}

/// `ERR_getErrorCode()`
#[inline(always)]
pub fn err_get_error_code(code: usize) -> ZSTD_ErrorCode {
    if !err_is_error(code) {
        return 0;
    }
    (0usize.wrapping_sub(code)) as ZSTD_ErrorCode
}

/// The strings from `ERR_getErrorString()`, kept NUL-terminated so the exact
/// same bytes can be handed back to C callers.
fn error_string_bytes(code: ZSTD_ErrorCode) -> &'static [u8] {
    const NOT_ERROR_CODE: &[u8] = b"Unspecified error code\0";
    match code {
        ZSTD_error_no_error => b"No error detected\0",
        ZSTD_error_GENERIC => b"Error (generic)\0",
        ZSTD_error_prefix_unknown => b"Unknown frame descriptor\0",
        ZSTD_error_version_unsupported => b"Version not supported\0",
        ZSTD_error_frameParameter_unsupported => b"Unsupported frame parameter\0",
        ZSTD_error_frameParameter_windowTooLarge => {
            b"Frame requires too much memory for decoding\0"
        }
        ZSTD_error_corruption_detected => b"Data corruption detected\0",
        ZSTD_error_checksum_wrong => b"Restored data doesn't match checksum\0",
        ZSTD_error_literals_headerWrong => {
            b"Header of Literals' block doesn't respect format specification\0"
        }
        ZSTD_error_parameter_unsupported => b"Unsupported parameter\0",
        ZSTD_error_parameter_combination_unsupported => b"Unsupported combination of parameters\0",
        ZSTD_error_parameter_outOfBound => b"Parameter is out of bound\0",
        ZSTD_error_init_missing => b"Context should be init first\0",
        ZSTD_error_memory_allocation => b"Allocation error : not enough memory\0",
        ZSTD_error_workSpace_tooSmall => b"workSpace buffer is not large enough\0",
        ZSTD_error_stage_wrong => b"Operation not authorized at current processing stage\0",
        ZSTD_error_tableLog_tooLarge => b"tableLog requires too much memory : unsupported\0",
        ZSTD_error_maxSymbolValue_tooLarge => b"Unsupported max Symbol Value : too large\0",
        ZSTD_error_maxSymbolValue_tooSmall => b"Specified maxSymbolValue is too small\0",
        ZSTD_error_cannotProduce_uncompressedBlock => {
            b"This mode cannot generate an uncompressed block\0"
        }
        ZSTD_error_stabilityCondition_notRespected => {
            b"pledged buffer stability condition is not respected\0"
        }
        ZSTD_error_dictionary_corrupted => b"Dictionary is corrupted\0",
        ZSTD_error_dictionary_wrong => b"Dictionary mismatch\0",
        ZSTD_error_dictionaryCreation_failed => b"Cannot create Dictionary from provided samples\0",
        ZSTD_error_dstSize_tooSmall => b"Destination buffer is too small\0",
        ZSTD_error_srcSize_wrong => b"Src size is incorrect\0",
        ZSTD_error_dstBuffer_null => b"Operation on NULL destination buffer\0",
        ZSTD_error_noForwardProgress_destFull => {
            b"Operation made no progress over multiple calls, due to output buffer being full\0"
        }
        ZSTD_error_noForwardProgress_inputEmpty => {
            b"Operation made no progress over multiple calls, due to input being empty\0"
        }
        ZSTD_error_frameIndex_tooLarge => b"Frame index is too large\0",
        ZSTD_error_seekableIO => b"An I/O error occurred when reading/seeking\0",
        ZSTD_error_dstBuffer_wrong => b"Destination buffer is wrong\0",
        ZSTD_error_srcBuffer_wrong => b"Source buffer is wrong\0",
        ZSTD_error_sequenceProducer_failed => {
            b"Block-level external sequence producer returned an error code\0"
        }
        ZSTD_error_externalSequences_invalid => b"External sequences are not valid\0",
        _ => NOT_ERROR_CODE,
    }
}

/// `ERR_getErrorString()` — exported by the C build.
#[unsafe(no_mangle)]
pub extern "C" fn ERR_getErrorString(code: ZSTD_ErrorCode) -> *const c_char {
    error_string_bytes(code).as_ptr() as *const c_char
}

/// `ERR_getErrorName()`
#[inline(always)]
pub fn err_get_error_name(code: usize) -> *const c_char {
    ERR_getErrorString(err_get_error_code(code))
}

/* ===== public API from zstd_common.c ===== */

pub const ZSTD_VERSION_MAJOR: u32 = 1;
pub const ZSTD_VERSION_MINOR: u32 = 5;
pub const ZSTD_VERSION_RELEASE: u32 = 7;
pub const ZSTD_VERSION_NUMBER: u32 =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;
pub const ZSTD_VERSION_STRING: &[u8] = b"1.5.7\0";

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionNumber() -> c_uint {
    ZSTD_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_versionString() -> *const c_char {
    ZSTD_VERSION_STRING.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_isError(code: usize) -> c_uint {
    err_is_error(code) as c_uint
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorName(code: usize) -> *const c_char {
    err_get_error_name(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorCode(code: usize) -> ZSTD_ErrorCode {
    err_get_error_code(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_getErrorString(code: ZSTD_ErrorCode) -> *const c_char {
    ERR_getErrorString(code)
}
