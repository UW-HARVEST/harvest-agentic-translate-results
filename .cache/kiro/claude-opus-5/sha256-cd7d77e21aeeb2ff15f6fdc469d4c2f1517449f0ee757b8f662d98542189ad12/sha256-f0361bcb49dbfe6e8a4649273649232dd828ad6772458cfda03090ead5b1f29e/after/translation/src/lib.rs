//! Rust translation of the C library in `c_src/` (stb_ds.h implementation unit).
//!
//! The C source is a single translation unit built from `stb_ds.h`'s
//! `STB_DS_IMPLEMENTATION` body plus a small test driver (`strkey`, `hm_geti`).
//! Behaviour, including quirks and implementation-defined arithmetic, is
//! reproduced exactly; no bugs are fixed.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};

pub(crate) mod arr;
pub(crate) mod hash;
pub(crate) mod hashmap;
pub(crate) mod strings;
pub(crate) mod unit_tests;

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free directly through
// STBDS_REALLOC / STBDS_FREE, plus str*/mem* helpers)
// ---------------------------------------------------------------------------
extern "C" {
    pub(crate) fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub(crate) fn free(p: *mut c_void);
    pub(crate) fn abort() -> !;
    pub(crate) fn strlen(s: *const c_char) -> usize;
    pub(crate) fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub(crate) fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
}

/// `STBDS_ASSERT` maps to `assert`, which aborts when the condition fails.
macro_rules! stbds_assert {
    ($cond:expr) => {
        if !($cond) {
            unsafe { $crate::abort() }
        }
    };
}
pub(crate) use stbds_assert;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
pub struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct stbds_hash_index {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: stbds_string_arena,
    pub storage: *mut stbds_hash_bucket,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub(crate) const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();

pub(crate) const STBDS_HM_BINARY: c_int = 0;
pub(crate) const STBDS_HM_STRING: c_int = 1;

#[allow(dead_code)]
pub(crate) const STBDS_SH_NONE: c_int = 0;
pub(crate) const STBDS_SH_DEFAULT: c_int = 1;
pub(crate) const STBDS_SH_STRDUP: c_int = 2;
pub(crate) const STBDS_SH_ARENA: c_int = 3;

pub(crate) const STBDS_BUCKET_LENGTH: usize = 8;
pub(crate) const STBDS_BUCKET_SHIFT: usize = 3;
pub(crate) const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
pub(crate) const STBDS_CACHE_LINE_SIZE: usize = 64;

pub(crate) const STBDS_INDEX_EMPTY: isize = -1;
pub(crate) const STBDS_INDEX_DELETED: isize = -2;

pub(crate) const STBDS_HASH_EMPTY: usize = 0;
pub(crate) const STBDS_HASH_DELETED: usize = 1;

pub(crate) const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

/// `stbds_siphash_bytes` is only valid for 64-bit `size_t`; the C source
/// enforces this with a negative-array-size typedef.
const _: () = assert!(core::mem::size_of::<usize>() == 8);

// ---------------------------------------------------------------------------
// Header / array accessors (macros in the C source)
// ---------------------------------------------------------------------------

/// `stbds_header(t)` == `((stbds_array_header *)(t) - 1)`
#[inline(always)]
pub(crate) fn header(a: *mut c_void) -> *mut stbds_array_header {
    (a as *mut u8).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
}

/// `stbds_arrlen(a)`
#[inline(always)]
pub(crate) unsafe fn arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*header(a)).length as isize
    }
}

/// `stbds_arrcap(a)`
#[inline(always)]
pub(crate) unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

/// `stbds_hash_table(a)`
#[inline(always)]
pub(crate) unsafe fn hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*header(a)).hash_table as *mut stbds_hash_index
}

/// `STBDS_ARR_TO_HASH(x, elemsize)`
#[inline(always)]
pub(crate) fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `STBDS_HASH_TO_ARR(x, elemsize)`
#[inline(always)]
pub(crate) fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `stbds_temp(t)` read
#[inline(always)]
pub(crate) unsafe fn temp_get(a: *mut c_void) -> isize {
    (*header(a)).temp
}

/// `stbds_temp(t) = v`
#[inline(always)]
pub(crate) unsafe fn temp_set(a: *mut c_void, v: isize) {
    (*header(a)).temp = v;
}

/// `stbds_temp_key(t) = v`, i.e. `*(char **) stbds_header(t)->hash_table = v`
#[inline(always)]
pub(crate) unsafe fn temp_key_set(a: *mut c_void, v: *mut c_char) {
    *((*header(a)).hash_table as *mut *mut c_char) = v;
}

/// `STBDS_ALIGN_FWD(n, a)`
#[inline(always)]
pub(crate) fn align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a).wrapping_sub(1)) & !(a.wrapping_sub(1))
}
