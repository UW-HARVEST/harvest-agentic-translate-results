//! Raw C runtime bindings plus the shared data layout of the C library.
//!
//! The C code uses `realloc`/`free` from <stdlib.h> for every allocation
//! (`STBDS_REALLOC(c,p,s) realloc(p,s)` / `STBDS_FREE(c,p) free(p)`), so the
//! Rust translation must use the very same libc allocator: memory handed out by
//! e.g. `stbds_arrgrowf` may be released by `stbds_arrfreef` or by the caller,
//! and the allocation/free *sequence* has to stay identical.

use core::ffi::{c_char, c_int, c_uchar, c_uint, c_void};

extern "C" {
    pub fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_uint,
        function: *const c_char,
    ) -> !;
}

/// `assert()` from <assert.h>: `STBDS_ASSERT` is `#define`d to `assert` and
/// `NDEBUG` is never defined, so the assertions are live in the C build.
macro_rules! stbds_assert {
    ($cond:expr, $text:expr, $line:expr, $func:expr) => {
        if !($cond) {
            $crate::ffi::__assert_fail(
                $text.as_ptr() as *const core::ffi::c_char,
                $crate::ffi::FILE_NAME.as_ptr() as *const core::ffi::c_char,
                $line,
                $func.as_ptr() as *const core::ffi::c_char,
            );
        }
    };
}
pub(crate) use stbds_assert;

pub static FILE_NAME: &[u8] = b"src/lib.c\0";

// ---------------------------------------------------------------------------
// Layout of the structures shared with the C code.  These are part of the
// observable ABI (callers embed `stbds_string_arena` and read the array header
// through the stb_ds macros), so the layouts must match exactly.
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
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

/// ```c
/// typedef struct stbds_string_block {
///   struct stbds_string_block *next;
///   char storage[8];
/// } stbds_string_block;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
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
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: c_uchar,
    pub mode: c_uchar,
}

/// ```c
/// typedef struct {
///    size_t    hash [STBDS_BUCKET_LENGTH];
///    ptrdiff_t index[STBDS_BUCKET_LENGTH];
/// } stbds_hash_bucket;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_hash_bucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
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

// Compile time layout checks mirroring the C struct sizes on LP64.
const _: () = {
    assert!(core::mem::size_of::<stbds_array_header>() == 32);
    assert!(core::mem::size_of::<stbds_string_block>() == 16);
    assert!(core::mem::size_of::<stbds_string_arena>() == 24);
    assert!(core::mem::size_of::<stbds_hash_bucket>() == 128);
    assert!(core::mem::size_of::<stbds_hash_index>() == 104);
    assert!(core::mem::size_of::<usize>() == 8);
};
