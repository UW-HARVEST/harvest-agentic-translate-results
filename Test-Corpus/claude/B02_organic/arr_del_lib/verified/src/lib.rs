//! Rust translation of the C library in `c_src/` (an stb_ds-style implementation
//! plus the `arr_del`/`strkey` test helpers).
//!
//! The translation is intentionally literal: allocation sizes, memory layouts,
//! integer wrap-around, sign-extension quirks and the exact order of operations
//! of the original C code are all preserved so that the resulting shared object
//! is behaviourally (byte-for-byte) identical to the C one.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;

mod arena;
mod array;
mod hash;
mod hashmap;
mod tests;

#[allow(unused_imports)]
pub(crate) use arena::*;
#[allow(unused_imports)]
pub(crate) use array::*;
#[allow(unused_imports)]
pub(crate) use hash::*;
#[allow(unused_imports)]
pub(crate) use hashmap::*;

// ---------------------------------------------------------------------------
// libc bindings.  The C code uses realloc()/free()/mem*()/str*() directly; we
// call the very same functions so that ownership of blocks handed out by (or
// handed back to) this library stays 100% compatible with the C version.
// ---------------------------------------------------------------------------
extern "C" {
    pub(crate) fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub(crate) fn free(p: *mut c_void);
    pub(crate) fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub(crate) fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub(crate) fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub(crate) fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub(crate) fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub(crate) fn strlen(s: *const c_char) -> usize;
}

// ---------------------------------------------------------------------------
// STBDS_REALLOC / STBDS_FREE
// ---------------------------------------------------------------------------
#[inline]
pub(crate) unsafe fn STBDS_REALLOC(p: *mut c_void, s: usize) -> *mut c_void {
    realloc(p, s)
}

#[inline]
pub(crate) unsafe fn STBDS_FREE(p: *mut c_void) {
    free(p)
}

// ---------------------------------------------------------------------------
// Structures (all layout-compatible with the C originals)
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct {
///   size_t length; size_t capacity; void *hash_table; ptrdiff_t temp;
/// } stbds_array_header;
/// ```
#[repr(C)]
pub(crate) struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

/// ```c
/// typedef struct stbds_string_block {
///   struct stbds_string_block *next; char storage[8];
/// } stbds_string_block;
/// ```
#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

/// ```c
/// struct stbds_string_arena {
///   stbds_string_block *storage; size_t remaining;
///   unsigned char block; unsigned char mode;
/// };
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

/// ```c
/// typedef struct {
///   size_t hash[STBDS_BUCKET_LENGTH]; ptrdiff_t index[STBDS_BUCKET_LENGTH];
/// } stbds_hash_bucket;
/// ```
#[repr(C)]
pub(crate) struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

/// ```c
/// typedef struct {
///   char *temp_key;
///   size_t slot_count, used_count, used_count_threshold,
///          used_count_shrink_threshold, tombstone_count,
///          tombstone_count_threshold, seed, slot_count_log2;
///   stbds_string_arena string;
///   stbds_hash_bucket *storage;
/// } stbds_hash_index;
/// ```
#[repr(C)]
pub(crate) struct stbds_hash_index {
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
pub(crate) const STBDS_HM_BINARY: c_int = 0;
pub(crate) const STBDS_HM_STRING: c_int = 1;

pub(crate) const STBDS_SH_NONE: u8 = 0;
pub(crate) const STBDS_SH_DEFAULT: u8 = 1;
pub(crate) const STBDS_SH_STRDUP: u8 = 2;
pub(crate) const STBDS_SH_ARENA: u8 = 3;

pub(crate) const STBDS_BUCKET_LENGTH: usize = 8;
pub(crate) const STBDS_BUCKET_SHIFT: usize = 3;
pub(crate) const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
pub(crate) const STBDS_CACHE_LINE_SIZE: usize = 64;

pub(crate) const STBDS_INDEX_EMPTY: isize = -1;
pub(crate) const STBDS_INDEX_DELETED: isize = -2;

pub(crate) const STBDS_HASH_EMPTY: usize = 0;
pub(crate) const STBDS_HASH_DELETED: usize = 1;

pub(crate) const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() as u32) * 8;

#[inline]
pub(crate) fn STBDS_INDEX_IN_USE(x: isize) -> bool {
    x >= 0
}

#[inline]
pub(crate) fn STBDS_ALIGN_FWD(n: usize, a: usize) -> usize {
    n.wrapping_add(a).wrapping_sub(1) & !(a.wrapping_sub(1))
}

// ---------------------------------------------------------------------------
// Pointer helpers mirroring the C macros
// ---------------------------------------------------------------------------

/// `#define stbds_header(t) ((stbds_array_header *) (t) - 1)`
#[inline]
pub(crate) fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut u8).wrapping_sub(size_of::<stbds_array_header>()) as *mut stbds_array_header
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char *) (x) - (elemsize))`
#[inline]
pub(crate) fn STBDS_HASH_TO_ARR(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char *) (x) + (elemsize))`
#[inline]
pub(crate) fn STBDS_ARR_TO_HASH(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// byte offset from a `void *`
#[inline]
pub(crate) fn byte_off(p: *mut c_void, n: usize) -> *mut u8 {
    (p as *mut u8).wrapping_add(n)
}

/// `#define stbds_temp(t) stbds_header(t)->temp`
#[inline]
pub(crate) unsafe fn stbds_temp(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

#[inline]
pub(crate) unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline]
pub(crate) unsafe fn stbds_set_temp_key(t: *mut c_void, v: *mut c_char) {
    *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
}

/// `#define stbds_hash_table(a) ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
pub(crate) unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `#define stbds_arrlen(a) ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
pub(crate) unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// `#define stbds_arrcap(a) ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
pub(crate) unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// `STBDS_ASSERT` -- `assert()` from <assert.h>.  Aborts the process just like a
/// failing C assertion would (message text of a failing assert is not part of
/// the library's normal output).
#[inline]
pub(crate) fn STBDS_ASSERT(cond: bool) {
    if !cond {
        std::process::abort();
    }
}
