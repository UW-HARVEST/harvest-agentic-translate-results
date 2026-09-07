//! Data layout and constants of `stb_ds` as embedded in `c_src/src/lib.c`.
//!
//! Every struct is `#[repr(C)]` with the exact field order of the C original so
//! that all of the pointer arithmetic the library performs on the "header in
//! front of the payload" layout stays bit-compatible.

use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Compile-time configuration (`#define`s of the original)
// ---------------------------------------------------------------------------

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 ? 3 : 2
pub const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
pub const STBDS_CACHE_LINE_SIZE: usize = 64;

pub const STBDS_INDEX_EMPTY: isize = -1;
pub const STBDS_INDEX_DELETED: isize = -2;

pub const STBDS_HASH_EMPTY: usize = 0;
pub const STBDS_HASH_DELETED: usize = 1;

// `STBDS_HM_BINARY` / `STBDS_SH_NONE` are the zero-valued members of the
// original's mode enumerations; the C code spells the zero literally, so the
// named constants are kept for documentation only.
#[allow(dead_code)]
pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

#[allow(dead_code)]
pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;
/// `STBDS_SIZE_T_BITS` — `sizeof(size_t) * 8`.
pub const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

pub const STBDS_SIPHASH_C_ROUNDS: usize = 2;
pub const STBDS_SIPHASH_D_ROUNDS: usize = 4;

pub const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
pub const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// The C file contains `typedef int
/// STBDS_SIPHASH_2_4_can_only_be_used_in_64_bit_builds[sizeof(size_t)==8?1:-1];`
/// which fails to compile on non 64-bit targets.  Mirror that restriction.
const _: () = assert!(core::mem::size_of::<usize>() == 8);

// ---------------------------------------------------------------------------
// Structures
// ---------------------------------------------------------------------------

/// `stbds_array_header` — stored immediately *before* the user array data.
#[repr(C)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

/// `stbds_string_block`
#[repr(C)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

/// `struct stbds_string_arena`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

/// `stbds_hash_bucket`
#[repr(C)]
pub struct HashBucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

/// `stbds_hash_index`
#[repr(C)]
pub struct HashIndex {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: StringArena,
    pub storage: *mut HashBucket,
}

// ---------------------------------------------------------------------------
// Mutable process-wide state
// ---------------------------------------------------------------------------

/// Wrapper granting the same "single threaded, no synchronisation" access to a
/// mutable global that plain C file-scope variables have.
#[repr(transparent)]
pub struct CGlobal<T>(UnsafeCell<T>);

unsafe impl<T> Sync for CGlobal<T> {}

impl<T> CGlobal<T> {
    pub const fn new(v: T) -> Self {
        CGlobal(UnsafeCell::new(v))
    }
    #[inline]
    pub fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// `static size_t stbds_hash_seed = 0x31415926;`
pub static STBDS_HASH_SEED: CGlobal<usize> = CGlobal::new(0x31415926);

// ---------------------------------------------------------------------------
// Pointer helpers (the macro zoo of the original)
// ---------------------------------------------------------------------------

/// `stbds_header(t)` — `((stbds_array_header *) (t) - 1)`
#[inline]
pub unsafe fn stbds_header(t: *mut c_void) -> *mut ArrayHeader {
    (t as *mut ArrayHeader).wrapping_sub(1)
}

/// `STBDS_HASH_TO_ARR(x, elemsize)` — `((char *) (x) - (elemsize))`
#[inline]
pub fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x, elemsize)` — `((char *) (x) + (elemsize))`
#[inline]
pub fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `(char *) a + elemsize * i + keyoffset` with C's wrapping semantics.
#[inline]
pub fn elem_at(a: *mut c_void, elemsize: usize, i: usize, extra: usize) -> *mut u8 {
    (a as *mut u8)
        .wrapping_add(elemsize.wrapping_mul(i))
        .wrapping_add(extra)
}

/// `stbds_arrlen(a)` — `((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
pub unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if !a.is_null() {
        (*stbds_header(a)).length as isize
    } else {
        0
    }
}

/// `stbds_arrcap(a)` — `((a) ? stbds_header(a)->capacity : 0)`
#[inline]
pub unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if !a.is_null() {
        (*stbds_header(a)).capacity
    } else {
        0
    }
}

/// `stbds_temp(t)` — `stbds_header(t)->temp`, as an lvalue-ish pointer.
#[inline]
pub unsafe fn stbds_temp_ptr(t: *mut c_void) -> *mut isize {
    &raw mut (*stbds_header(t)).temp
}

/// `stbds_temp_key(t)` — `(*(char **) stbds_header(t)->hash_table)`
#[inline]
pub unsafe fn stbds_temp_key_ptr(t: *mut c_void) -> *mut *mut c_char {
    (*stbds_header(t)).hash_table as *mut *mut c_char
}

/// `stbds_hash_table(a)` — `((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
pub unsafe fn stbds_hash_table(a: *mut c_void) -> *mut HashIndex {
    (*stbds_header(a)).hash_table as *mut HashIndex
}

/// `STBDS_ALIGN_FWD(n, a)` — `(((n) + (a) - 1) & ~((a) - 1))`
#[inline]
pub fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

/// `STBDS_INDEX_IN_USE(x)` — `((x) >= 0)`
#[inline]
pub fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

/// `STBDS_ROTATE_LEFT(val, n)`
#[inline]
pub fn rotl(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `STBDS_ROTATE_RIGHT(val, n)`
#[inline]
pub fn rotr(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

/// Widening of an `int`-typed C sub-expression to `size_t`: the value is sign
/// extended, which the original relies on (accidentally) in
/// `stbds_siphash_bytes`.
#[inline]
pub fn sx(v: i32) -> usize {
    v as i64 as u64 as usize
}
