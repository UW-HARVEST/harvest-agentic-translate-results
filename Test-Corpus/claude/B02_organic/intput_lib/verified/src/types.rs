//! Data layout + macro helpers translated from `c_src/src/lib.c`.
//!
//! Every struct here is `#[repr(C)]` and must stay bit-compatible with the C
//! originals because the same heap blocks are shared across the ABI boundary.

use core::ffi::{c_char, c_void};

/// ```c
/// typedef struct { size_t length; size_t capacity; void *hash_table; ptrdiff_t temp; } stbds_array_header;
/// ```
#[repr(C)]
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
/// struct stbds_string_arena { stbds_string_block *storage; size_t remaining; unsigned char block; unsigned char mode; };
/// ```
#[repr(C)]
pub struct StbdsStringArena {
    pub storage: *mut StbdsStringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

/// ```c
/// typedef struct { size_t hash[8]; ptrdiff_t index[8]; } stbds_hash_bucket;
/// ```
#[repr(C)]
pub struct StbdsHashBucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

/// ```c
/// typedef struct { char *temp_key; size_t slot_count; ... stbds_string_arena string; stbds_hash_bucket *storage; } stbds_hash_index;
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

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 ? 3 : 2
pub const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
pub const STBDS_CACHE_LINE_SIZE: usize = 64;

pub const STBDS_INDEX_EMPTY: isize = -1;
pub const STBDS_INDEX_DELETED: isize = -2;

pub const STBDS_HASH_EMPTY: usize = 0;
pub const STBDS_HASH_DELETED: usize = 1;

pub const STBDS_HM_BINARY: i32 = 0;
pub const STBDS_HM_STRING: i32 = 1;

// enum { STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA };
pub const STBDS_SH_NONE: i32 = 0;
pub const STBDS_SH_DEFAULT: i32 = 1;
pub const STBDS_SH_STRDUP: i32 = 2;
pub const STBDS_SH_ARENA: i32 = 3;

pub const HEADER_SIZE: usize = core::mem::size_of::<StbdsArrayHeader>();

/// `#define STBDS_INDEX_IN_USE(x) ((x) >= 0)`
#[inline]
pub fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
#[inline]
pub fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a).wrapping_sub(1) & !a.wrapping_sub(1)
}

/// `#define stbds_header(t) ((stbds_array_header *) (t) - 1)`
///
/// Uses `wrapping_sub` rather than `offset(-1)`: C computes this address with
/// plain (wrapping) pointer arithmetic even when `t` is NULL or wild — see
/// `stbds_arrfreef(NULL)`, which the C turns into `free((void *) -32)`. A Rust
/// `offset` would be an overflowing-address-calculation UB check instead.
#[inline]
pub unsafe fn stbds_header(t: *mut c_void) -> *mut StbdsArrayHeader {
    (t as *mut u8).wrapping_sub(HEADER_SIZE) as *mut StbdsArrayHeader
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

/// `#define stbds_arrcap(a) ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
pub unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// `#define stbds_temp(t) stbds_header(t)->temp`
#[inline]
pub unsafe fn stbds_temp(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

#[inline]
pub unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline]
pub unsafe fn stbds_set_temp_key(t: *mut c_void, v: *mut c_char) {
    *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
}

/// `#define stbds_hash_table(a) ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
pub unsafe fn stbds_hash_table(a: *mut c_void) -> *mut StbdsHashIndex {
    (*stbds_header(a)).hash_table as *mut StbdsHashIndex
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char*) (x) - (elemsize))`
#[inline]
pub unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut c_char).wrapping_sub(elemsize) as *mut c_void
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char*) (x) + (elemsize))`
#[inline]
pub unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut c_char).wrapping_add(elemsize) as *mut c_void
}
