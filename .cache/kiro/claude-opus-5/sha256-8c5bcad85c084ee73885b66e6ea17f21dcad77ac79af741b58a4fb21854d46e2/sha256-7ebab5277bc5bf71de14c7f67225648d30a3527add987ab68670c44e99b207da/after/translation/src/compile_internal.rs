//! Translation of the shared definitions in `c_src/src/pcre2_compile.h`.

#![allow(dead_code, non_upper_case_globals, non_camel_case_types)]

use core::ffi::c_int;

use crate::internal::{class_bits_storage, PCRE2_SIZE, PCRE2_UCHAR, MAX_UTF_CODE_POINT,
    COMPILE_ERROR_BASE, BOOL};

/* Compile time error code numbers, 100 less than the public PCRE2_ERROR_* values. */
pub const ERR0: c_int = COMPILE_ERROR_BASE + 0;
pub const ERR1: c_int = COMPILE_ERROR_BASE + 1;
pub const ERR2: c_int = COMPILE_ERROR_BASE + 2;
pub const ERR3: c_int = COMPILE_ERROR_BASE + 3;
pub const ERR4: c_int = COMPILE_ERROR_BASE + 4;
pub const ERR5: c_int = COMPILE_ERROR_BASE + 5;
pub const ERR6: c_int = COMPILE_ERROR_BASE + 6;
pub const ERR7: c_int = COMPILE_ERROR_BASE + 7;
pub const ERR8: c_int = COMPILE_ERROR_BASE + 8;
pub const ERR9: c_int = COMPILE_ERROR_BASE + 9;
pub const ERR10: c_int = COMPILE_ERROR_BASE + 10;
pub const ERR11: c_int = COMPILE_ERROR_BASE + 11;
pub const ERR12: c_int = COMPILE_ERROR_BASE + 12;
pub const ERR13: c_int = COMPILE_ERROR_BASE + 13;
pub const ERR14: c_int = COMPILE_ERROR_BASE + 14;
pub const ERR15: c_int = COMPILE_ERROR_BASE + 15;
pub const ERR16: c_int = COMPILE_ERROR_BASE + 16;
pub const ERR17: c_int = COMPILE_ERROR_BASE + 17;
pub const ERR18: c_int = COMPILE_ERROR_BASE + 18;
pub const ERR19: c_int = COMPILE_ERROR_BASE + 19;
pub const ERR20: c_int = COMPILE_ERROR_BASE + 20;
pub const ERR21: c_int = COMPILE_ERROR_BASE + 21;
pub const ERR22: c_int = COMPILE_ERROR_BASE + 22;
pub const ERR23: c_int = COMPILE_ERROR_BASE + 23;
pub const ERR24: c_int = COMPILE_ERROR_BASE + 24;
pub const ERR25: c_int = COMPILE_ERROR_BASE + 25;
pub const ERR26: c_int = COMPILE_ERROR_BASE + 26;
pub const ERR27: c_int = COMPILE_ERROR_BASE + 27;
pub const ERR28: c_int = COMPILE_ERROR_BASE + 28;
pub const ERR29: c_int = COMPILE_ERROR_BASE + 29;
pub const ERR30: c_int = COMPILE_ERROR_BASE + 30;
pub const ERR31: c_int = COMPILE_ERROR_BASE + 31;
pub const ERR32: c_int = COMPILE_ERROR_BASE + 32;
pub const ERR33: c_int = COMPILE_ERROR_BASE + 33;
pub const ERR34: c_int = COMPILE_ERROR_BASE + 34;
pub const ERR35: c_int = COMPILE_ERROR_BASE + 35;
pub const ERR36: c_int = COMPILE_ERROR_BASE + 36;
pub const ERR37: c_int = COMPILE_ERROR_BASE + 37;
pub const ERR38: c_int = COMPILE_ERROR_BASE + 38;
pub const ERR39: c_int = COMPILE_ERROR_BASE + 39;
pub const ERR40: c_int = COMPILE_ERROR_BASE + 40;
pub const ERR41: c_int = COMPILE_ERROR_BASE + 41;
pub const ERR42: c_int = COMPILE_ERROR_BASE + 42;
pub const ERR43: c_int = COMPILE_ERROR_BASE + 43;
pub const ERR44: c_int = COMPILE_ERROR_BASE + 44;
pub const ERR45: c_int = COMPILE_ERROR_BASE + 45;
pub const ERR46: c_int = COMPILE_ERROR_BASE + 46;
pub const ERR47: c_int = COMPILE_ERROR_BASE + 47;
pub const ERR48: c_int = COMPILE_ERROR_BASE + 48;
pub const ERR49: c_int = COMPILE_ERROR_BASE + 49;
pub const ERR50: c_int = COMPILE_ERROR_BASE + 50;
pub const ERR51: c_int = COMPILE_ERROR_BASE + 51;
pub const ERR52: c_int = COMPILE_ERROR_BASE + 52;
pub const ERR53: c_int = COMPILE_ERROR_BASE + 53;
pub const ERR54: c_int = COMPILE_ERROR_BASE + 54;
pub const ERR55: c_int = COMPILE_ERROR_BASE + 55;
pub const ERR56: c_int = COMPILE_ERROR_BASE + 56;
pub const ERR57: c_int = COMPILE_ERROR_BASE + 57;
pub const ERR58: c_int = COMPILE_ERROR_BASE + 58;
pub const ERR59: c_int = COMPILE_ERROR_BASE + 59;
pub const ERR60: c_int = COMPILE_ERROR_BASE + 60;
pub const ERR61: c_int = COMPILE_ERROR_BASE + 61;
pub const ERR62: c_int = COMPILE_ERROR_BASE + 62;
pub const ERR63: c_int = COMPILE_ERROR_BASE + 63;
pub const ERR64: c_int = COMPILE_ERROR_BASE + 64;
pub const ERR65: c_int = COMPILE_ERROR_BASE + 65;
pub const ERR66: c_int = COMPILE_ERROR_BASE + 66;
pub const ERR67: c_int = COMPILE_ERROR_BASE + 67;
pub const ERR68: c_int = COMPILE_ERROR_BASE + 68;
pub const ERR69: c_int = COMPILE_ERROR_BASE + 69;
pub const ERR70: c_int = COMPILE_ERROR_BASE + 70;
pub const ERR71: c_int = COMPILE_ERROR_BASE + 71;
pub const ERR72: c_int = COMPILE_ERROR_BASE + 72;
pub const ERR73: c_int = COMPILE_ERROR_BASE + 73;
pub const ERR74: c_int = COMPILE_ERROR_BASE + 74;
pub const ERR75: c_int = COMPILE_ERROR_BASE + 75;
pub const ERR76: c_int = COMPILE_ERROR_BASE + 76;
pub const ERR77: c_int = COMPILE_ERROR_BASE + 77;
pub const ERR78: c_int = COMPILE_ERROR_BASE + 78;
pub const ERR79: c_int = COMPILE_ERROR_BASE + 79;
pub const ERR80: c_int = COMPILE_ERROR_BASE + 80;
pub const ERR81: c_int = COMPILE_ERROR_BASE + 81;
pub const ERR82: c_int = COMPILE_ERROR_BASE + 82;
pub const ERR83: c_int = COMPILE_ERROR_BASE + 83;
pub const ERR84: c_int = COMPILE_ERROR_BASE + 84;
pub const ERR85: c_int = COMPILE_ERROR_BASE + 85;
pub const ERR86: c_int = COMPILE_ERROR_BASE + 86;
pub const ERR87: c_int = COMPILE_ERROR_BASE + 87;
pub const ERR88: c_int = COMPILE_ERROR_BASE + 88;
pub const ERR89: c_int = COMPILE_ERROR_BASE + 89;
pub const ERR90: c_int = COMPILE_ERROR_BASE + 90;
pub const ERR91: c_int = COMPILE_ERROR_BASE + 91;
pub const ERR92: c_int = COMPILE_ERROR_BASE + 92;
pub const ERR93: c_int = COMPILE_ERROR_BASE + 93;
pub const ERR94: c_int = COMPILE_ERROR_BASE + 94;
pub const ERR95: c_int = COMPILE_ERROR_BASE + 95;
pub const ERR96: c_int = COMPILE_ERROR_BASE + 96;
pub const ERR97: c_int = COMPILE_ERROR_BASE + 97;
pub const ERR98: c_int = COMPILE_ERROR_BASE + 98;
pub const ERR99: c_int = COMPILE_ERROR_BASE + 99;
pub const ERR100: c_int = COMPILE_ERROR_BASE + 100;
pub const ERR101: c_int = COMPILE_ERROR_BASE + 101;
pub const ERR102: c_int = COMPILE_ERROR_BASE + 102;
pub const ERR103: c_int = COMPILE_ERROR_BASE + 103;
pub const ERR104: c_int = COMPILE_ERROR_BASE + 104;
pub const ERR105: c_int = COMPILE_ERROR_BASE + 105;
pub const ERR106: c_int = COMPILE_ERROR_BASE + 106;
pub const ERR107: c_int = COMPILE_ERROR_BASE + 107;
pub const ERR108: c_int = COMPILE_ERROR_BASE + 108;
pub const ERR109: c_int = COMPILE_ERROR_BASE + 109;
pub const ERR110: c_int = COMPILE_ERROR_BASE + 110;
pub const ERR111: c_int = COMPILE_ERROR_BASE + 111;
pub const ERR112: c_int = COMPILE_ERROR_BASE + 112;
pub const ERR113: c_int = COMPILE_ERROR_BASE + 113;
pub const ERR114: c_int = COMPILE_ERROR_BASE + 114;
pub const ERR115: c_int = COMPILE_ERROR_BASE + 115;
pub const ERR116: c_int = COMPILE_ERROR_BASE + 116;
pub const ERR117: c_int = COMPILE_ERROR_BASE + 117;
pub const ERR118: c_int = COMPILE_ERROR_BASE + 118;
pub const ERR119: c_int = COMPILE_ERROR_BASE + 119;
pub const ERR120: c_int = COMPILE_ERROR_BASE + 120;

/* Codes for parsed patterns. */
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

/* `META_CODE(x)` */
#[inline]
pub const fn meta_code(x: u32) -> u32 {
    x & 0xffff0000
}

/* `META_DATA(x)` */
#[inline]
pub const fn meta_data(x: u32) -> u32 {
    x & 0x0000ffff
}

/* `META_DIFF(x, y)` */
#[inline]
pub const fn meta_diff(x: u32, y: u32) -> u32 {
    (x.wrapping_sub(y)) >> 16
}

/* PCRE2_SIZE does not fit in a uint32_t, so offsets occupy two elements.
`PUTOFFSET(s, p)` */
#[inline]
pub unsafe fn putoffset(s: PCRE2_SIZE, p: &mut *mut u32) {
    unsafe {
        **p = (s >> 32) as u32;
        *p = p.add(1);
        **p = (s & 0xffffffff) as u32;
        *p = p.add(1);
    }
}

/* `GETOFFSET(s, p)` */
#[inline]
pub unsafe fn getoffset(p: &mut *mut u32) -> PCRE2_SIZE {
    unsafe {
        let s = ((*p.add(0) as PCRE2_SIZE) << 32) | (*p.add(1) as PCRE2_SIZE);
        *p = p.add(2);
        s
    }
}

/* `GETPLUSOFFSET(s, p)` */
#[inline]
pub unsafe fn getplusoffset(p: &mut *mut u32) -> PCRE2_SIZE {
    unsafe {
        let s = ((*p.add(1) as PCRE2_SIZE) << 32) | (*p.add(2) as PCRE2_SIZE);
        *p = p.add(2);
        s
    }
}

/* `READPLUSOFFSET(s, p)` */
#[inline]
pub unsafe fn readplusoffset(p: *const u32) -> PCRE2_SIZE {
    unsafe { ((*p.add(1) as PCRE2_SIZE) << 32) | (*p.add(2) as PCRE2_SIZE) }
}

/* `SKIPOFFSET(p)` */
#[inline]
pub unsafe fn skipoffset(p: &mut *mut u32) {
    unsafe { *p = p.add(2) };
}

pub const SIZEOFFSET: usize = 2;

/* Extended class management flags. */
pub const CLASS_IS_ECLASS: u32 = 0x1;

/* Highest character value in 8-bit mode. */
pub const MAX_UCHAR_VALUE: u32 = 0xff;

/* `GET_MAX_CHAR_VALUE(utf)` */
#[inline]
pub const fn get_max_char_value(utf: bool) -> u32 {
    if utf { MAX_UTF_CODE_POINT } else { MAX_UCHAR_VALUE }
}

/* `SETBIT(a, b)` */
#[inline]
pub unsafe fn setbit(a: *mut u8, b: u32) {
    unsafe { *a.add((b >> 3) as usize) |= (1u32 << (b & 0x7)) as u8 };
}

/* `SELECT_VALUE8(value8, value)` -- 8-bit mode selects the first. */
#[inline]
pub const fn select_value8<T: Copy>(value8: T, _value: T) -> T {
    value8
}

/* `CLIST_ALIGN_TO(base, align)` */
#[inline]
pub const fn clist_align_to(base: usize, align: usize) -> usize {
    (base + (align - 1)) & !(align - 1)
}

/* Indices of the POSIX classes in posix_names, posix_name_lengths,
posix_class_maps and posix_substitutes. */
pub const PC_DIGIT: usize = 7;
pub const PC_GRAPH: usize = 8;
pub const PC_PRINT: usize = 9;
pub const PC_PUNCT: usize = 10;
pub const PC_XDIGIT: usize = 13;

/* Flags for the hash_dup member of the named_group structure. */
pub const NAMED_GROUP_HASH_MASK: u16 = 0x7fff;
pub const NAMED_GROUP_IS_DUPNAME: u16 = 0x8000;

/* `NAMED_GROUP_GET_HASH(ng)` */
#[inline]
pub unsafe fn named_group_get_hash(ng: *const crate::internal::named_group) -> u16 {
    unsafe { (*ng).hash_dup & NAMED_GROUP_HASH_MASK }
}

/* Information about an OP_ECLASS internal operand. */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct eclass_op_info {
    /* The position of the operand, or NULL if lengthptr != NULL. */
    pub code_start: *mut PCRE2_UCHAR,
    pub length: PCRE2_SIZE,
    /* The operand's type if it is a single code (ECL_XCLASS, ECL_ANY, ECL_NONE);
    otherwise zero if the operand is not atomic. */
    pub op_single_type: u8,
    /* The constant-folded bitmap for code points < 256. */
    pub bits: class_bits_storage,
}

/* Silence an unused-import warning when BOOL is not otherwise referenced. */
const _: Option<BOOL> = None;
