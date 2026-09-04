//! Shared definitions from `pcre2_compile.h`, used by `compile.rs`,
//! `compile_class.rs` and `compile_cgroup.rs`.

use crate::internal::*;
use core::ffi::c_int;

// ---------------------------------------------------------------------------
// Compile time error code numbers (enum { ERR0 = COMPILE_ERROR_BASE, ERR1, ... })
// ---------------------------------------------------------------------------

/// `ERRn` == `COMPILE_ERROR_BASE + n` == `100 + n`.
#[inline(always)]
pub const fn ERR(n: c_int) -> c_int {
    COMPILE_ERROR_BASE as c_int + n
}

macro_rules! errs {
    ($($name:ident = $n:expr),* $(,)?) => { $(pub const $name: c_int = ERR($n);)* };
}

errs! {
    ERR0 = 0, ERR1 = 1, ERR2 = 2, ERR3 = 3, ERR4 = 4, ERR5 = 5, ERR6 = 6,
    ERR7 = 7, ERR8 = 8, ERR9 = 9, ERR10 = 10, ERR11 = 11, ERR12 = 12,
    ERR13 = 13, ERR14 = 14, ERR15 = 15, ERR16 = 16, ERR17 = 17, ERR18 = 18,
    ERR19 = 19, ERR20 = 20, ERR21 = 21, ERR22 = 22, ERR23 = 23, ERR24 = 24,
    ERR25 = 25, ERR26 = 26, ERR27 = 27, ERR28 = 28, ERR29 = 29, ERR30 = 30,
    ERR31 = 31, ERR32 = 32, ERR33 = 33, ERR34 = 34, ERR35 = 35, ERR36 = 36,
    ERR37 = 37, ERR38 = 38, ERR39 = 39, ERR40 = 40, ERR41 = 41, ERR42 = 42,
    ERR43 = 43, ERR44 = 44, ERR45 = 45, ERR46 = 46, ERR47 = 47, ERR48 = 48,
    ERR49 = 49, ERR50 = 50, ERR51 = 51, ERR52 = 52, ERR53 = 53, ERR54 = 54,
    ERR55 = 55, ERR56 = 56, ERR57 = 57, ERR58 = 58, ERR59 = 59, ERR60 = 60,
    ERR61 = 61, ERR62 = 62, ERR63 = 63, ERR64 = 64, ERR65 = 65, ERR66 = 66,
    ERR67 = 67, ERR68 = 68, ERR69 = 69, ERR70 = 70, ERR71 = 71, ERR72 = 72,
    ERR73 = 73, ERR74 = 74, ERR75 = 75, ERR76 = 76, ERR77 = 77, ERR78 = 78,
    ERR79 = 79, ERR80 = 80, ERR81 = 81, ERR82 = 82, ERR83 = 83, ERR84 = 84,
    ERR85 = 85, ERR86 = 86, ERR87 = 87, ERR88 = 88, ERR89 = 89, ERR90 = 90,
    ERR91 = 91, ERR92 = 92, ERR93 = 93, ERR94 = 94, ERR95 = 95, ERR96 = 96,
    ERR97 = 97, ERR98 = 98, ERR99 = 99, ERR100 = 100, ERR101 = 101,
    ERR102 = 102, ERR103 = 103, ERR104 = 104, ERR105 = 105, ERR106 = 106,
    ERR107 = 107, ERR108 = 108, ERR109 = 109, ERR110 = 110, ERR111 = 111,
    ERR112 = 112, ERR113 = 113, ERR114 = 114, ERR115 = 115, ERR116 = 116,
    ERR117 = 117, ERR118 = 118, ERR119 = 119, ERR120 = 120,
}

// ---------------------------------------------------------------------------
// Macros for manipulating elements of the parsed pattern vector
// ---------------------------------------------------------------------------

/// `META_CODE(x)`.
#[inline(always)]
pub const fn META_CODE(x: u32) -> u32 {
    x & 0xffff0000
}

/// `META_DATA(x)`.
#[inline(always)]
pub const fn META_DATA(x: u32) -> u32 {
    x & 0x0000ffff
}

/// `META_DIFF(x, y)`.
#[inline(always)]
pub const fn META_DIFF(x: u32, y: u32) -> u32 {
    x.wrapping_sub(y) >> 16
}

// ---------------------------------------------------------------------------
// PCRE2_SIZE storage in the uint32_t parsed pattern (64-bit world: SIZEOFFSET 2)
// ---------------------------------------------------------------------------

/// `PUTOFFSET(s, p)` — store a `PCRE2_SIZE` as two `uint32_t`, advancing `p`.
#[inline(always)]
pub unsafe fn PUTOFFSET(s: PCRE2_SIZE, p: &mut *mut u32) {
    unsafe {
        **p = (s >> 32) as u32;
        *p = p.add(1);
        **p = (s & 0xffffffff) as u32;
        *p = p.add(1);
    }
}

/// `GETOFFSET(s, p)` — read a `PCRE2_SIZE` from two `uint32_t`, advancing `p`.
#[inline(always)]
pub unsafe fn GETOFFSET(p: &mut *const u32) -> PCRE2_SIZE {
    unsafe {
        let s = ((*p.add(0) as PCRE2_SIZE) << 32) | (*p.add(1) as PCRE2_SIZE);
        *p = p.add(2);
        s
    }
}

/// `GETOFFSET(s, p)` for a mutable pointer.
#[inline(always)]
pub unsafe fn GETOFFSET_MUT(p: &mut *mut u32) -> PCRE2_SIZE {
    unsafe {
        let s = ((*p.add(0) as PCRE2_SIZE) << 32) | (*p.add(1) as PCRE2_SIZE);
        *p = p.add(2);
        s
    }
}

/// `GETPLUSOFFSET(s, p)` — read from `p[1]`/`p[2]`, advancing `p` by 2.
#[inline(always)]
pub unsafe fn GETPLUSOFFSET(p: &mut *mut u32) -> PCRE2_SIZE {
    unsafe {
        let s = ((*p.add(1) as PCRE2_SIZE) << 32) | (*p.add(2) as PCRE2_SIZE);
        *p = p.add(2);
        s
    }
}

/// `READPLUSOFFSET(s, p)` — read from `p[1]`/`p[2]` without advancing.
#[inline(always)]
pub unsafe fn READPLUSOFFSET(p: *const u32) -> PCRE2_SIZE {
    unsafe { ((*p.add(1) as PCRE2_SIZE) << 32) | (*p.add(2) as PCRE2_SIZE) }
}

/// `SKIPOFFSET(p)` — advance `p` past a stored offset.
#[inline(always)]
pub unsafe fn SKIPOFFSET(p: &mut *mut u32) {
    unsafe { *p = p.add(2) }
}

/// `SKIPOFFSET(p)` for a const pointer.
#[inline(always)]
pub unsafe fn SKIPOFFSET_CONST(p: &mut *const u32) {
    unsafe { *p = p.add(2) }
}

pub const SIZEOFFSET_U: usize = 2;

// ---------------------------------------------------------------------------
// Misc macros
// ---------------------------------------------------------------------------

pub const MAX_UCHAR_VALUE_U: u32 = 0xff;

/// `GET_MAX_CHAR_VALUE(utf)`.
#[inline(always)]
pub const fn GET_MAX_CHAR_VALUE(utf: bool) -> u32 {
    if utf { MAX_UTF_CODE_POINT_U } else { MAX_UCHAR_VALUE_U }
}

/// `SETBIT(a, b)` — set bit `b` in a byte-array bitmap.
#[inline(always)]
pub unsafe fn SETBIT(a: *mut u8, b: u32) {
    unsafe { *a.add((b >> 3) as usize) |= 1u8 << (b & 0x7) }
}

/// `SELECT_VALUE8(value8, value)` — in 8-bit mode always the first argument.
#[inline(always)]
pub const fn SELECT_VALUE8<T: Copy>(value8: T, _value: T) -> T {
    value8
}

/// `CLIST_ALIGN_TO(base, align)`.
#[inline(always)]
pub const fn CLIST_ALIGN_TO(base: usize, align: usize) -> usize {
    (base + (align - 1)) & !(align - 1)
}

// ---------------------------------------------------------------------------
// eclass_op_info
// ---------------------------------------------------------------------------

/// Information about an `OP_ECLASS` internal operand.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct eclass_op_info {
    /// The position of the operand, or NULL if `lengthptr != NULL`.
    pub code_start: *mut PCRE2_UCHAR,
    pub length: PCRE2_SIZE,
    /// The operand's type if it is a single code (`ECL_XCLASS`, `ECL_ANY`,
    /// `ECL_NONE`); otherwise zero if the operand is not atomic.
    pub op_single_type: u8,
    /// The constant-folded bitmap for code points < 256.
    pub bits: class_bits_storage,
}

// ---------------------------------------------------------------------------
// named_group hash helpers
// ---------------------------------------------------------------------------

pub const NAMED_GROUP_HASH_MASK_U: u16 = 0x7fff;
pub const NAMED_GROUP_IS_DUPNAME_U: u16 = 0x8000;

/// `NAMED_GROUP_GET_HASH(ng)`.
#[inline(always)]
pub unsafe fn NAMED_GROUP_GET_HASH(ng: *const named_group) -> u16 {
    unsafe { (*ng).hash_dup & NAMED_GROUP_HASH_MASK_U }
}

// ---------------------------------------------------------------------------
// PRIV(posix_class_maps) — defined in pcre2_compile.c
// ---------------------------------------------------------------------------

/// `PRIV(posix_class_maps)` — base map offset, second map offset (or -1), and a
/// tweak code for each POSIX class, in the order of `posix_names`.
#[unsafe(no_mangle)]
pub static _pcre2_posix_class_maps8: [c_int; 42] = [
    cbit_word as c_int,   cbit_digit as c_int, -2, // alpha
    cbit_lower as c_int,  -1,                   0, // lower
    cbit_upper as c_int,  -1,                   0, // upper
    cbit_word as c_int,   -1,                   2, // alnum - word without underscore
    cbit_print as c_int,  cbit_cntrl as c_int,  0, // ascii
    cbit_space as c_int,  -1,                   1, // blank - a GNU extension
    cbit_cntrl as c_int,  -1,                   0, // cntrl
    cbit_digit as c_int,  -1,                   0, // digit
    cbit_graph as c_int,  -1,                   0, // graph
    cbit_print as c_int,  -1,                   0, // print
    cbit_punct as c_int,  -1,                   0, // punct
    cbit_space as c_int,  -1,                   0, // space
    cbit_word as c_int,   -1,                   0, // word - a Perl extension
    cbit_xdigit as c_int, -1,                   0, // xdigit
];
