//! Translation of common/error_private.{h,c} and zstd_errors.h enum.
#![allow(dead_code)]

use core::ffi::{c_char, c_uint};

// ZSTD_ErrorCode values (from include/zstd_errors.h)
pub mod code {
    pub const NO_ERROR: i32 = 0;
    pub const GENERIC: i32 = 1;
    pub const PREFIX_UNKNOWN: i32 = 10;
    pub const VERSION_UNSUPPORTED: i32 = 12;
    pub const FRAMEPARAMETER_UNSUPPORTED: i32 = 14;
    pub const FRAMEPARAMETER_WINDOWTOOLARGE: i32 = 16;
    pub const CORRUPTION_DETECTED: i32 = 20;
    pub const CHECKSUM_WRONG: i32 = 22;
    pub const LITERALS_HEADERWRONG: i32 = 24;
    pub const DICTIONARY_CORRUPTED: i32 = 30;
    pub const DICTIONARY_WRONG: i32 = 32;
    pub const DICTIONARYCREATION_FAILED: i32 = 34;
    pub const PARAMETER_UNSUPPORTED: i32 = 40;
    pub const PARAMETER_COMBINATION_UNSUPPORTED: i32 = 41;
    pub const PARAMETER_OUTOFBOUND: i32 = 42;
    pub const TABLELOG_TOOLARGE: i32 = 44;
    pub const MAXSYMBOLVALUE_TOOLARGE: i32 = 46;
    pub const MAXSYMBOLVALUE_TOOSMALL: i32 = 48;
    pub const CANNOTPRODUCE_UNCOMPRESSEDBLOCK: i32 = 49;
    pub const STABILITYCONDITION_NOTRESPECTED: i32 = 50;
    pub const STAGE_WRONG: i32 = 60;
    pub const INIT_MISSING: i32 = 62;
    pub const MEMORY_ALLOCATION: i32 = 64;
    pub const WORKSPACE_TOOSMALL: i32 = 66;
    pub const DSTSIZE_TOOSMALL: i32 = 70;
    pub const SRCSIZE_WRONG: i32 = 72;
    pub const DSTBUFFER_NULL: i32 = 74;
    pub const NOFORWARDPROGRESS_DESTFULL: i32 = 80;
    pub const NOFORWARDPROGRESS_INPUTEMPTY: i32 = 82;
    pub const FRAMEINDEX_TOOLARGE: i32 = 100;
    pub const SEEKABLEIO: i32 = 102;
    pub const DSTBUFFER_WRONG: i32 = 104;
    pub const SRCBUFFER_WRONG: i32 = 105;
    pub const SEQUENCEPRODUCER_FAILED: i32 = 106;
    pub const EXTERNALSEQUENCES_INVALID: i32 = 107;
    pub const MAXCODE: i32 = 120;
}

/// ERROR(name) = (size_t)-code
#[inline]
pub fn error(code: i32) -> usize {
    (-(code as isize)) as usize
}

#[inline]
pub fn err_is_error(code: usize) -> c_uint {
    (code > error(self::code::MAXCODE)) as c_uint
}

#[inline]
pub fn err_get_error_code(code: usize) -> i32 {
    if err_is_error(code) == 0 {
        0
    } else {
        (0usize.wrapping_sub(code)) as i32
    }
}

/// Translation of ERR_getErrorString (error_private.c)
#[unsafe(no_mangle)]
pub extern "C" fn ERR_getErrorString(code: i32) -> *const c_char {
    use self::code as C;
    let s: &[u8] = match code {
        C::NO_ERROR => b"No error detected\0",
        C::GENERIC => b"Error (generic)\0",
        C::PREFIX_UNKNOWN => b"Unknown frame descriptor\0",
        C::VERSION_UNSUPPORTED => b"Version not supported\0",
        C::FRAMEPARAMETER_UNSUPPORTED => b"Unsupported frame parameter\0",
        C::FRAMEPARAMETER_WINDOWTOOLARGE => b"Frame requires too much memory for decoding\0",
        C::CORRUPTION_DETECTED => b"Data corruption detected\0",
        C::CHECKSUM_WRONG => b"Restored data doesn't match checksum\0",
        C::LITERALS_HEADERWRONG => {
            b"Header of Literals' block doesn't respect format specification\0"
        }
        C::PARAMETER_UNSUPPORTED => b"Unsupported parameter\0",
        C::PARAMETER_COMBINATION_UNSUPPORTED => b"Unsupported combination of parameters\0",
        C::PARAMETER_OUTOFBOUND => b"Parameter is out of bound\0",
        C::INIT_MISSING => b"Context should be init first\0",
        C::MEMORY_ALLOCATION => b"Allocation error : not enough memory\0",
        C::WORKSPACE_TOOSMALL => b"workSpace buffer is not large enough\0",
        C::STAGE_WRONG => b"Operation not authorized at current processing stage\0",
        C::TABLELOG_TOOLARGE => b"tableLog requires too much memory : unsupported\0",
        C::MAXSYMBOLVALUE_TOOLARGE => b"Unsupported max Symbol Value : too large\0",
        C::MAXSYMBOLVALUE_TOOSMALL => b"Specified maxSymbolValue is too small\0",
        C::CANNOTPRODUCE_UNCOMPRESSEDBLOCK => {
            b"This mode cannot generate an uncompressed block\0"
        }
        C::STABILITYCONDITION_NOTRESPECTED => {
            b"pledged buffer stability condition is not respected\0"
        }
        C::DICTIONARY_CORRUPTED => b"Dictionary is corrupted\0",
        C::DICTIONARY_WRONG => b"Dictionary mismatch\0",
        C::DICTIONARYCREATION_FAILED => b"Cannot create Dictionary from provided samples\0",
        C::DSTSIZE_TOOSMALL => b"Destination buffer is too small\0",
        C::SRCSIZE_WRONG => b"Src size is incorrect\0",
        C::DSTBUFFER_NULL => b"Operation on NULL destination buffer\0",
        C::NOFORWARDPROGRESS_DESTFULL => {
            b"Operation made no progress over multiple calls, due to output buffer being full\0"
        }
        C::NOFORWARDPROGRESS_INPUTEMPTY => {
            b"Operation made no progress over multiple calls, due to input being empty\0"
        }
        C::FRAMEINDEX_TOOLARGE => b"Frame index is too large\0",
        C::SEEKABLEIO => b"An I/O error occurred when reading/seeking\0",
        C::DSTBUFFER_WRONG => b"Destination buffer is wrong\0",
        C::SRCBUFFER_WRONG => b"Source buffer is wrong\0",
        C::SEQUENCEPRODUCER_FAILED => b"Block-level external sequence producer returned an error code\0",
        C::EXTERNALSEQUENCES_INVALID => b"External sequences are not valid\0",
        _ => b"Unspecified error code\0",
    };
    s.as_ptr() as *const c_char
}

#[inline]
pub fn err_get_error_name(code: usize) -> *const c_char {
    ERR_getErrorString(err_get_error_code(code))
}
