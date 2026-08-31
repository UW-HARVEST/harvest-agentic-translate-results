#!/usr/bin/env python3
"""Generate Rust definitions from c_src/src/pcre2_compile.h."""
import re

H = '$HARVEST_WORKDIR/c_src/src/pcre2_compile.h'
OUT = '$HARVEST_WORKDIR/translation/src/compile_internal.rs'

src = open(H).read()
nc = re.sub(r'/\*.*?\*/', ' ', src, flags=re.S)

out = [
    "//! Translation of the shared definitions in `c_src/src/pcre2_compile.h`.",
    "",
    "#![allow(dead_code, non_upper_case_globals, non_camel_case_types)]",
    "",
    "use core::ffi::c_int;",
    "",
    "use crate::internal::{class_bits_storage, PCRE2_SIZE, PCRE2_UCHAR, MAX_UTF_CODE_POINT,",
    "    COMPILE_ERROR_BASE, BOOL};",
    "",
    "/* Compile time error code numbers, 100 less than the public PCRE2_ERROR_* values. */",
]

# ERR0..ERR120
m = re.search(r'enum\s*\{\s*ERR0\s*=\s*COMPILE_ERROR_BASE\s*,(.*?)\};', nc, flags=re.S)
names = ['ERR0'] + [t.strip() for t in m.group(1).split(',') if t.strip()]
for i, n in enumerate(names):
    out.append(f"pub const {n}: c_int = COMPILE_ERROR_BASE + {i};")
out.append("")

# META_* constants
out.append("/* Codes for parsed patterns. */")
for m in re.finditer(r'#define\s+(META_\w+)\s+(0x[0-9a-fA-F]+)u', nc):
    out.append(f"pub const {m.group(1)}: u32 = {m.group(2)};")
out.append("pub const META_FIRST_QUANTIFIER: u32 = META_ASTERISK;")
out.append("pub const META_LAST_QUANTIFIER: u32 = META_MINMAX_QUERY;")
out.append("")

out.append("""/* `META_CODE(x)` */
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
const _: Option<BOOL> = None;""")

open(OUT, 'w').write('\n'.join(out) + '\n')
print('errs:', len(names), 'metas ok')
