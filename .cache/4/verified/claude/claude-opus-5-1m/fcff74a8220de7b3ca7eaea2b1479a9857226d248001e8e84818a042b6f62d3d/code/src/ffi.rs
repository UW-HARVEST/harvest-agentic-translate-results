//! C runtime declarations and the C data structures used by the library.
//!
//! Translated from `c_src/src/lib.c` (an inlined copy of `stb_ds.h`).
//! Layouts must match the C structs byte for byte, so everything is `#[repr(C)]`.

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc: the C code uses realloc/free (via STBDS_REALLOC/STBDS_FREE) plus the
// usual string/IO routines. Use the very same libc entry points so behaviour
// (allocation, stdio buffering, formatting) is byte-identical.
// ---------------------------------------------------------------------------
extern "C" {
    pub fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

/// `STBDS_ASSERT` -- the C library builds with `assert`, which is compiled out
/// in the shipped (NDEBUG) configuration. Kept as a no-op so no extra output or
/// aborts are ever produced.
macro_rules! stbds_assert {
    ($cond:expr) => {
        let _ = &$cond;
    };
}
pub(crate) use stbds_assert;

// ---------------------------------------------------------------------------
// Structures
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct {
///   size_t      length;
///   size_t      capacity;
///   void      * hash_table;
///   ptrdiff_t   temp;
/// } stbds_array_header;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StbdsArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

/// ```c
/// typedef struct stbds_string_block { struct stbds_string_block *next; char storage[8]; } stbds_string_block;
/// ```
#[repr(C)]
pub struct StbdsStringBlock {
    pub next: *mut StbdsStringBlock,
    pub storage: [c_char; 8],
}

/// ```c
/// struct stbds_string_arena {
///   stbds_string_block *storage;
///   size_t remaining;
///   unsigned char block;
///   unsigned char mode;
/// };
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StbdsStringArena {
    pub storage: *mut StbdsStringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_BUCKET_SHIFT: usize = 3;
pub const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
pub const STBDS_CACHE_LINE_SIZE: usize = 64;

/// ```c
/// typedef struct { size_t hash[8]; ptrdiff_t index[8]; } stbds_hash_bucket;
/// ```
#[repr(C)]
pub struct StbdsHashBucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

/// ```c
/// typedef struct {
///   char * temp_key;
///   size_t slot_count;
///   size_t used_count;
///   size_t used_count_threshold;
///   size_t used_count_shrink_threshold;
///   size_t tombstone_count;
///   size_t tombstone_count_threshold;
///   size_t seed;
///   size_t slot_count_log2;
///   stbds_string_arena string;
///   stbds_hash_bucket *storage;
/// } stbds_hash_index;
/// ```
#[repr(C)]
pub struct StbdsHashIndex {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: StbdsStringArena,
    pub storage: *mut StbdsHashBucket,
}

pub const STBDS_INDEX_EMPTY: isize = -1;
pub const STBDS_INDEX_DELETED: isize = -2;

#[inline]
pub fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

pub const STBDS_HASH_EMPTY: usize = 0;
pub const STBDS_HASH_DELETED: usize = 1;

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

// enum { STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA };
pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

pub const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

// ---------------------------------------------------------------------------
// Array header helpers (the `stbds_header`/`stbds_arrlen`/... macros)
// ---------------------------------------------------------------------------

/// `#define stbds_header(t)  ((stbds_array_header *) (t) - 1)`
#[inline]
pub fn stbds_header(t: *mut c_void) -> *mut StbdsArrayHeader {
    (t as *mut StbdsArrayHeader).wrapping_sub(1)
}

/// `#define stbds_arrcap(a) ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
pub unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// `#define stbds_arrlen(a) ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
pub unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// `#define stbds_temp(t) stbds_header(t)->temp`
#[inline]
pub unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline]
pub unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    let table = (*stbds_header(t)).hash_table as *mut *mut c_char;
    *table = v;
}

/// `#define stbds_hash_table(a) ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
pub unsafe fn stbds_hash_table(a: *mut c_void) -> *mut StbdsHashIndex {
    (*stbds_header(a)).hash_table as *mut StbdsHashIndex
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char *) (x) - (elemsize))`
#[inline]
pub fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char *) (x) + (elemsize))`
#[inline]
pub fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}
