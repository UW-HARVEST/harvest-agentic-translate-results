//! File-scope `#define`s, enums and structures of `pcre2_compile.c` that are
//! shared between the Rust modules into which that file has been split.

use crate::internal::*;
use core::ffi::c_int;

// --- Limits and sizes ------------------------------------------------------

pub const MAX_GROUP_NUMBER: u32 = 65535;
pub const MAX_REPEAT_COUNT: u32 = 65535;
pub const REPEAT_UNLIMITED: u32 = MAX_REPEAT_COUNT + 1;

/// `COMPILE_WORK_SIZE` — size of stack workspace, in code units.
pub const COMPILE_WORK_SIZE: usize = 3000 * LINK_SIZE_U;

/// `C16_WORK_SIZE` — number of elements in the 16-bit workspace vector.
pub const C16_WORK_SIZE: usize =
    (COMPILE_WORK_SIZE * core::mem::size_of::<PCRE2_UCHAR>()) / core::mem::size_of::<u16>();

pub const GROUPINFO_DEFAULT_SIZE: usize = 256;
pub const WORK_SIZE_SAFETY_MARGIN: usize = 100;
pub const NAMED_GROUP_LIST_SIZE: usize = 20;
pub const PARSED_PATTERN_DEFAULT_SIZE: usize = 1024;

/// `OFLOW_MAX` — `INT_MAX - 20`.
pub const OFLOW_MAX: c_int = c_int::MAX - 20;

// --- parsed_skip types -----------------------------------------------------

pub const PSKIP_ALT: u32 = 0;
pub const PSKIP_CLASS: u32 = 1;
pub const PSKIP_KET: u32 = 2;

// --- "Required code unit" flags --------------------------------------------

pub const REQ_UNSET: u32 = 0xffffffff;
pub const REQ_NONE: u32 = 0xfffffffe;
pub const REQ_CASELESS: u32 = 0x00000001;
pub const REQ_VARY: u32 = 0x00000002;

// --- Group information flags ----------------------------------------------

pub const GI_SET_FIXED_LENGTH: u32 = 0x80000000;
pub const GI_NOT_FIXED_LENGTH: u32 = 0x40000000;
pub const GI_FIXED_LENGTH_MASK: u32 = 0x0000ffff;

// --- Nesting save block ----------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct nest_save {
    pub nest_depth: u16,
    pub reset_group: u16,
    pub max_group: u16,
    pub flags: u16,
    pub options: u32,
    pub xoptions: u32,
}

pub const NSF_RESET: u32 = 0x0001;
pub const NSF_CONDASSERT: u32 = 0x0002;
pub const NSF_ATOMICSR: u32 = 0x0004;

/// Options that are changeable within the pattern and so must be tracked
/// during parsing.
pub const PARSE_TRACKED_OPTIONS: u32 = (PCRE2_CASELESS
    | PCRE2_DOTALL
    | PCRE2_DUPNAMES
    | PCRE2_EXTENDED
    | PCRE2_EXTENDED_MORE
    | PCRE2_MULTILINE
    | PCRE2_NO_AUTO_CAPTURE
    | PCRE2_UNGREEDY) as u32;

pub const PARSE_TRACKED_EXTRA_OPTIONS: u32 = (PCRE2_EXTRA_CASELESS_RESTRICT
    | PCRE2_EXTRA_ASCII_BSD
    | PCRE2_EXTRA_ASCII_BSS
    | PCRE2_EXTRA_ASCII_BSW
    | PCRE2_EXTRA_ASCII_DIGIT
    | PCRE2_EXTRA_ASCII_POSIX) as u32;

// --- Character-class range analysis states (the two OK values must be last) -

pub const RANGE_NO: u32 = 0;
pub const RANGE_STARTED: u32 = 1;
pub const RANGE_FORBID_NO: u32 = 2;
pub const RANGE_FORBID_STARTED: u32 = 3;
pub const RANGE_OK_ESCAPED: u32 = 4;
pub const RANGE_OK_LITERAL: u32 = 5;

// --- Extended character class operator/operand states ----------------------

pub const CLASS_OP_EMPTY: u32 = 0;
pub const CLASS_OP_OPERAND: u32 = 1;
pub const CLASS_OP_OPERATOR: u32 = 2;

// --- Character class parse modes (the two PERL_EXT values must be last) ----

pub const CLASS_MODE_NORMAL: u32 = 0;
pub const CLASS_MODE_ALT_EXT: u32 = 1;
pub const CLASS_MODE_PERL_EXT: u32 = 2;
pub const CLASS_MODE_PERL_EXT_LEAF: u32 = 3;

// --- Table of extra lengths for each of the meta codes ---------------------

/// `meta_extra_lengths` — emitted from the C source.
pub static META_EXTRA_LENGTHS: [u8; 73] = [
    0, 0, 0, 0, 3, 1, 3, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 3, 3, 3, 3, 2, 0, 1, 1, 0, 0, 0, 0, 0, 2, 1, 1, 0, 0, 2, 3, 0, 0, 0, 2, 2, 0, 2, 1, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 0, 0, 0, 0, 0,
];

// --- Helpers ---------------------------------------------------------------

/// `UPPER_CASE(c)` for the non-EBCDIC escapes table.
#[inline(always)]
pub const fn UPPER_CASE(c: u32) -> u32 {
    c.wrapping_sub(32)
}
