// Constants translated from pcre2_compile.h
#![allow(non_upper_case_globals, dead_code)]
use core::ffi::c_int;

pub const META_END: u32 = 0x80000000;
pub const META_ALT: u32 = 0x80010000;
pub const META_ATOMIC: u32 = 0x80020000;
pub const META_BACKREF: u32 = 0x80030000;
pub const META_BACKREF_BYNAME: u32 = 0x80040000;
pub const META_BIGVALUE: u32 = 0x80050000;
pub const META_CALLOUT_NUMBER: u32 = 0x80060000;
pub const META_CALLOUT_STRING: u32 = 0x80070000;
pub const META_CAPTURE: u32 = 0x80080000;
pub const META_CIRCUMFLEX: u32 = 0x80090000;
pub const META_CLASS: u32 = 0x800a0000;
pub const META_CLASS_EMPTY: u32 = 0x800b0000;
pub const META_CLASS_EMPTY_NOT: u32 = 0x800c0000;
pub const META_CLASS_END: u32 = 0x800d0000;
pub const META_CLASS_NOT: u32 = 0x800e0000;
pub const META_COND_ASSERT: u32 = 0x800f0000;
pub const META_COND_DEFINE: u32 = 0x80100000;
pub const META_COND_NAME: u32 = 0x80110000;
pub const META_COND_NUMBER: u32 = 0x80120000;
pub const META_COND_RNAME: u32 = 0x80130000;
pub const META_COND_RNUMBER: u32 = 0x80140000;
pub const META_COND_VERSION: u32 = 0x80150000;
pub const META_OFFSET: u32 = 0x80160000;
pub const META_SCS: u32 = 0x80170000;
pub const META_CAPTURE_NAME: u32 = 0x80180000;
pub const META_CAPTURE_NUMBER: u32 = 0x80190000;
pub const META_DOLLAR: u32 = 0x801a0000;
pub const META_DOT: u32 = 0x801b0000;
pub const META_ESCAPE: u32 = 0x801c0000;
pub const META_KET: u32 = 0x801d0000;
pub const META_NOCAPTURE: u32 = 0x801e0000;
pub const META_OPTIONS: u32 = 0x801f0000;
pub const META_POSIX: u32 = 0x80200000;
pub const META_POSIX_NEG: u32 = 0x80210000;
pub const META_RANGE_ESCAPED: u32 = 0x80220000;
pub const META_RANGE_LITERAL: u32 = 0x80230000;
pub const META_RECURSE: u32 = 0x80240000;
pub const META_RECURSE_BYNAME: u32 = 0x80250000;
pub const META_SCRIPT_RUN: u32 = 0x80260000;
pub const META_LOOKAHEAD: u32 = 0x80270000;
pub const META_LOOKAHEADNOT: u32 = 0x80280000;
pub const META_LOOKBEHIND: u32 = 0x80290000;
pub const META_LOOKBEHINDNOT: u32 = 0x802a0000;
pub const META_LOOKAHEAD_NA: u32 = 0x802b0000;
pub const META_LOOKBEHIND_NA: u32 = 0x802c0000;
pub const META_MARK: u32 = 0x802d0000;
pub const META_ACCEPT: u32 = 0x802e0000;
pub const META_FAIL: u32 = 0x802f0000;
pub const META_COMMIT: u32 = 0x80300000;
pub const META_COMMIT_ARG: u32 = 0x80310000;
pub const META_PRUNE: u32 = 0x80320000;
pub const META_PRUNE_ARG: u32 = 0x80330000;
pub const META_SKIP: u32 = 0x80340000;
pub const META_SKIP_ARG: u32 = 0x80350000;
pub const META_THEN: u32 = 0x80360000;
pub const META_THEN_ARG: u32 = 0x80370000;
pub const META_ASTERISK: u32 = 0x80380000;
pub const META_ASTERISK_PLUS: u32 = 0x80390000;
pub const META_ASTERISK_QUERY: u32 = 0x803a0000;
pub const META_PLUS: u32 = 0x803b0000;
pub const META_PLUS_PLUS: u32 = 0x803c0000;
pub const META_PLUS_QUERY: u32 = 0x803d0000;
pub const META_QUERY: u32 = 0x803e0000;
pub const META_QUERY_PLUS: u32 = 0x803f0000;
pub const META_QUERY_QUERY: u32 = 0x80400000;
pub const META_MINMAX: u32 = 0x80410000;
pub const META_MINMAX_PLUS: u32 = 0x80420000;
pub const META_MINMAX_QUERY: u32 = 0x80430000;
pub const META_ECLASS_AND: u32 = 0x80440000;
pub const META_ECLASS_OR: u32 = 0x80450000;
pub const META_ECLASS_SUB: u32 = 0x80460000;
pub const META_ECLASS_XOR: u32 = 0x80470000;
pub const META_ECLASS_NOT: u32 = 0x80480000;
pub const META_ATOMIC_SCRIPT_RUN: u32 = 0x8fff0000;
pub const META_FIRST_QUANTIFIER: u32 = META_ASTERISK;
pub const META_LAST_QUANTIFIER: u32 = META_MINMAX_QUERY;

#[inline(always)] pub fn META_CODE(x: u32) -> u32 { x & 0xffff0000u32 }
#[inline(always)] pub fn META_DATA(x: u32) -> u32 { x & 0x0000ffffu32 }
#[inline(always)] pub fn META_DIFF(x: u32, y: u32) -> u32 { (x.wrapping_sub(y)) >> 16 }

pub const SIZEOFFSET: usize = 2;
pub const CLASS_IS_ECLASS: u32 = 0x1;
pub const MAX_UCHAR_VALUE: u32 = 0xffu32;
#[inline(always)] pub fn GET_MAX_CHAR_VALUE(utf: bool) -> u32 { if utf { crate::internal::MAX_UTF_CODE_POINT } else { MAX_UCHAR_VALUE } }

pub const PC_DIGIT: usize = 7;
pub const PC_GRAPH: usize = 8;
pub const PC_PRINT: usize = 9;
pub const PC_PUNCT: usize = 10;
pub const PC_XDIGIT: usize = 13;

pub const NAMED_GROUP_HASH_MASK: u16 = 0x7fff;
pub const NAMED_GROUP_IS_DUPNAME: u16 = 0x8000u16;

// Compile-time error code numbers: ERRn == COMPILE_ERROR_BASE + n
pub const COMPILE_ERROR_BASE: c_int = 100;
#[inline(always)] pub const fn ERR(n: c_int) -> c_int { COMPILE_ERROR_BASE + n }

